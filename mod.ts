import { ChatCompletionRequestMessage, CreateChatCompletionRequest, Configuration, OpenAIApi } from "npm:openai@^3.3.0"

import { RuntimeError, Interrupt, Feedback, Exit } from "./errors.ts"
import { Prompts, Scope } from "./prompts.ts"
import { Naive, Template } from "./const.ts"
import logger from "./log.ts"

export type FunctionCall = {
    name: string,
    reasoning?: string,
    arguments: any[]
}

class LLM {
    #openai: OpenAIApi
    #messages: ChatCompletionRequestMessage[] = []
    #base: CreateChatCompletionRequest

    get messages(): ChatCompletionRequestMessage[] {
        return this.#messages
    }

    constructor() {
        const configuration = new Configuration({
            apiKey: Deno.env.get("OPENAI_KEY")
        });

        this.#openai = new OpenAIApi(configuration)

        this.#base = {
            "model": "gpt-3.5-turbo",
            "messages": []
        }
    }

    async send(messages: ChatCompletionRequestMessage[]): Promise<ChatCompletionRequestMessage> {
        const req: CreateChatCompletionRequest = {
            ...this.#base,
            "messages": this.#messages.concat(messages)
        }

        const resp = await this.#openai.createChatCompletion(req)
        const resp_msg = resp.data.choices[0].message
        if (resp_msg === undefined) {
            throw new Error("TODO")
        } else {
            this.#messages.push(...messages, resp_msg)
            return resp_msg
        }
    }

    async call(messages: ChatCompletionRequestMessage[]): Promise<FunctionCall> {
        const resp = await this.send(messages)
        return JSON.parse(resp.content)
    }
}

export type ExportDescriptor = {
    property_key: string,
    adder: (scope: Scope) => void,
    description?: string
}

export class ExportsMap {
    #inner: Map<string, ExportDescriptor> = new Map()

    get(property_key: string): ExportDescriptor | undefined {
        return this.#inner.get(property_key)
    }

    insert(property_key: string, descriptor: ExportDescriptor) {
        this.#inner.set(property_key, descriptor)
    }

    forEach(fn: (_: ExportDescriptor) => void) {
        this.#inner.forEach(fn)
    }
}

type ConstructorDecorator = <T extends { new (...args: any[]): {} } >(constructor: T) => any

type MethodDecorator = (target: any, property_key: string, descriptor: PropertyDescriptor) => void

const description_decorator = (task: string) => {
    return <T extends { new (...args: any[]): {} } >(constructor: T) => {
        constructor.prototype.task = task
    }
}

const prompts_decorator = (prompts: string) => {
    return <T extends { new (...args: any[]): {} } >(constructor: T) => {
        constructor.prototype.prompts = prompts
    }
}

const use_decorator: MethodDecorator = (target: any, property_key: string, _descriptor?: PropertyDescriptor) => {
    if (target.exports === undefined) {
        target.exports = new Map()
    }
    target.exports.set(property_key, {
        property_key,
        adder: (scope: Scope) => scope.addFromId("method_decl", Scope.ident(target.constructor.name), Scope.ident(property_key))
    })
}

type PromptDescriptor = {
    ty: "plain_text" | "ts"
    fmt: string
    id: string
    context: string[]
}

type Action = {
    call: FunctionCall,
    output?: object
}

export interface Agent {
    prompts?: string

    template?: Template

    exports: ExportsMap
}

export class AgentController {
    agent: Agent
    llm: LLM
    prompts: Prompts
    template: Template
    history: Action[] = []

    constructor(agent: Agent) {
        agent.is_done = false
        agent.resolved = undefined
        this.llm = new LLM()
        this.prompts = new Prompts(agent.prompts)
        this.prompts.spawnBackgroundInit()
        this.template = agent.template || Naive
        this.agent = agent
    }

    renderContext(): string {
        const scope = this.prompts.newScope()
        this.agent.exports.forEach(({ adder }) => adder(scope))
        return this.template.renderContext(scope)
    }

    async doAction(action: Action) {
        const exports = this.agent.exports

        const export_descriptor = exports.get(action.call.name)

        if (export_descriptor !== undefined) {
            const call_name = export_descriptor.property_key
            const args = action.call.arguments

            logger.calls(call_name, args)
            const output = await (this.agent[call_name])(...args)
            logger.returns(output)

            action.output = output
            this.history.push(action)
        } else {
            throw new TypeError(`${action.call.name} is not a function`)
        }
    }

    async doNext(prompt?: string, role?: "user" | "system") {
        if (this.agent.is_done) {
            throw new Error("agent is done")
        }

        if (prompt === undefined) {
            if (this.llm.messages.length == 0) {
                prompt = this.renderContext()
            } else {
                const last = this.history[this.history.length - 1]
                prompt = this.template.renderOutput(last.output)
            }
        }

        if (role === undefined) {
            role = "system"
        }

        const response = await this.llm.call([{
            "role": role,
            "content": prompt
        }])

        logger.thought(response.reasoning)

        await this.doAction({ call: response })
    }

    async runToCompletion(): any {
        await this.prompts.ensureReady()

        let prompt: string
        while (true) {
            try {
                await this.doNext(prompt)
                prompt = undefined
            }
            catch (err) {
                // TODO backoff
                if (err instanceof Feedback) {
                    logger.feedback(err)
                    prompt = err.message
                } else if (err instanceof Interrupt) {
                    logger.interrupt(err)
                    throw err.value
                } else if (err instanceof Exit) {
                    logger.exit(err)
                    return err.value
                } else {
                    logger.error(err)
                    prompt = this.template.renderError(err)
                }
            }
        }
    }
}

function run(agent: any): Promise<any> {
    // TODO check agent has the interface
    return (new AgentController(agent)).runToCompletion()
}

export default {
    AgentController,
    use: use_decorator,
    task: description_decorator,
    prompts: prompts_decorator,
    Interrupt,
    Feedback,
    Exit,
    setLogLevel: logger.setLogLevel,
    getLogLevel: logger.getLogLevel,
    run
}
