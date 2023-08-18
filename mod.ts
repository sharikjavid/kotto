import { ChatCompletionRequestMessage, CreateChatCompletionRequest, Configuration, OpenAIApi } from "npm:openai@^3.3.0"

import { RuntimeError, Interrupt, Feedback, Exit } from "./errors.ts"
import { Prompts, Scope } from "./prompts.ts"
import { Naive, Template } from "./const.ts"
import logger, { setLogLevel, getLogLevel } from "./log.ts"

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
        const apiKey = Deno.env.get("OPENAI_KEY")

        if (apiKey === undefined) {
            throw new RuntimeError("The `OPENAI_KEY` env variable must be set.")
        }

        const configuration = new Configuration({
            apiKey
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

        let resp 
        try {
            resp = await this.#openai.createChatCompletion(req)
        } catch (err) {
            throw new Interrupt(err)
        }

        const resp_msg = resp.data.choices[0].message

        if (resp_msg === undefined) {
            throw new RuntimeError("Didn't receive a completion")
        }

        this.#messages.push(...messages, resp_msg)

        return resp_msg
    }

    async complete(messages: ChatCompletionRequestMessage[]): Promise<string> {
        const resp = await this.send(messages)

        if (resp.content === undefined) {
            throw new RuntimeError("Completion has empty content")
        }
        
        return resp.content
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

type Action = {
    call: FunctionCall,
    output?: object
}

export interface Agent {
    [functions: string]: any;
}

type Exited = {
    output: any
}

function isExited(pending: Exited | Pending): pending is Exited {
    return "output" in pending
}

type Pending = {
    role: "user" | "system"
    prompt?: string
}

function isPending(exited: Exited | Pending): exited is Pending {
    return "prompt" in exited
}

export class AgentController {
    agent: Agent

    exports: ExportsMap

    llm: LLM

    prompts: Prompts

    template: Template

    history: Action[] = []
    prompt?: string
    role: "user" | "system" = "user"

    constructor(agent: Agent) {
        this.agent = agent

        this.exports = agent.exports || new ExportsMap()

        this.llm = new LLM()

        this.prompts = new Prompts(agent.prompts)

        this.prompts.spawnBackgroundInit()

        this.template = agent.template || Naive
    }

    renderContext(): string {
        const scope = this.prompts.newScope()

        this.exports.forEach(({ adder }) => adder(scope))

        return this.template.renderContext(scope)
    }

    async doAction(action: Action) {
        const exports = this.agent.exports

        const export_descriptor = exports.get(action.call.name)
        
        if (export_descriptor === undefined) {
            throw new TypeError(`${action.call.name} is not a function`)
        }

        const call_name = export_descriptor.property_key
        
        if (typeof this.agent[call_name] !== "function") {
            throw new TypeError(`${action.call.name} is not a function`)
        }

        const args = action.call.arguments

        logger.calls(call_name, args)
        const output = await (this.agent[call_name])(...args)
        logger.returns(output)

        action.output = output
        this.history.push(action)
        return
    }

    async tick({ prompt, role }: Pending = { role: "user" }): Promise<Exited | Pending> {
        await this.ensureReady()

        try {
            await this.complete(prompt, role)
            return {
                role: "user"
            }
        }
        catch (err) {
            // TODO backoff
            if (err instanceof Feedback) {
                logger.feedback(err)
                return {
                    prompt: err.message,
                    role: "system"
                }
            } else if (err instanceof Interrupt) {
                logger.interrupt(err)
                throw err.value
            } else if (err instanceof RuntimeError) {
                throw err
            } else if (err instanceof Exit) {
                logger.exit(err)
                return {
                    output: err.value
                }
            } else {
                logger.error(err)
                return {
                    role: "system",
                    prompt: this.template.renderError(err)
                }
            }
        }
    }

    async complete(prompt?: string, role: "user" | "system" = "system") {
        if (prompt === undefined) {
            if (this.llm.messages.length == 0) {
                prompt = this.renderContext()
            } else {
                const last = this.history[this.history.length - 1]
                prompt = this.template.renderOutput(last.output)
            }
        }

        const completion = await this.llm.complete([{
            "role": role,
            "content": prompt
        }])

        let response
        try {
            response = this.template.parseResponse(completion)
        } catch (_) {
            throw new Feedback(`could not extract JSON from your response: ${completion}`)
        }

        logger.thought(response.reasoning || "(no reasoning given)")

        await this.doAction({ call: response })
    }

    async ensureReady() {
        await this.prompts.ensureReady()
    }

    async runToCompletion(): Promise<any> {
        let tick = undefined
        while (true) {
            tick = await this.tick(tick)
            if (isExited(tick)) {
                return tick.output
            }
            logger.trace()
        }
    }
}

/**
 * Run an agent to completion.
 * @param agent
 */
function run(agent: any): Promise<any> {
    // TODO check agent has the interface
    return (new AgentController(agent)).runToCompletion()
}

/**
 * Run an agent once.
 * @param agent
 */
async function call(agent: any): Promise<Exited | Pending> {
    // TODO check agent has the interface
    const ctl = new AgentController(agent)
    return await ctl.tick()
}

export default {
    AgentController,
    use: use_decorator,
    task: description_decorator,
    prompts: prompts_decorator,
    Interrupt,
    Feedback,
    Exit,
    setLogLevel,
    getLogLevel,
    run,
    runOnce: call
}
