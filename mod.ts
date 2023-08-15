import { ChatCompletionRequestMessage, CreateChatCompletionRequest, Configuration, OpenAIApi } from "npm:openai@^3.3.0"

import * as colors from "https://deno.land/std@0.198.0/fmt/colors.ts"

import { RuntimeError, Interrupt } from "./errors.ts"
import { getLogLevel, setLogLevel, warn, debug } from "./bootstrap.ts"
import { Prompts, Scope } from "./prompts.ts"
import { Naive, Template } from "./const.ts"

export function thought(content?: string) {
    switch (getLogLevel()) {
        case "introspective":
        case "debug":
            console.log(colors.gray(`llm: ${content || "(no reasoning given)"}`))
            break;
    }
}

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
        debug({ text: `API response ${resp.content}`, color: colors.blue, prefix: "openai" })
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
        adder: (scope: Scope) => scope.add("method_decl", Scope.ident(target.constructor.name), Scope.ident(property_key))
    })
}

export class Agent {
    resolved?: any = undefined
    is_done = false

    resolve(value: any) {
        this.is_done = true
        this.resolved = value
    }

    then(onResolve, onReject) {
        return (new AgentController(this)).then(onResolve, onReject)
    }
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

interface AnnotatedAgent {
    prompts?: string

    template?: Template

    exports: ExportsMap

    is_done: boolean

    resolved?: any
}

export class AgentController {
    agent: AnnotatedAgent
    llm: LLM
    prompts: Prompts
    template: Template
    history: Action[] = []

    constructor(agent: AnnotatedAgent) {
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
            const output = await (this.agent[call_name])(...action.call.arguments)
            action.output = output
            this.history.push(action)
        } else {
            throw new TypeError(`${action.call.name} is not a function`)
        }
    }

    async doNext(prompt?: string, role?: "user" | "system"): Promise<any> {
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

        debug({ text: prompt, color: colors.yellow, prefix: "prompt" })

        const response = await this.llm.call([{
            "role": role,
            "content": prompt
        }])

        thought(response.reasoning)

        await this.doAction({ call: response })

        return this.agent.resolved
    }

    async runToCompletion(): any {
        await this.prompts.ensureReady()

        let resolved
        do {
            resolved = await this.doNext().catch((err) => {
                if (!(err instanceof Interrupt)) {
                    warn(`caught an exception: ${err}`)
                    warn("asking the llm to fix it")
                    const error_prompt = this.template.renderError(err)
                    return this.doNext(error_prompt)
                } else {
                    throw err
                }
            })
        } while (!this.agent.is_done)

        return resolved
    }

    then(onResolve, onReject) {
        return this.runToCompletion().then(onResolve, onReject)
    }
}

async function call(inner: (...args: any[]) => any) {
    if (inner.name === undefined) {
        throw new Error("`call` can only be used with top-level named functions")
    }

    // TODO: what if this gets minified?

    class CallAgent extends Agent {
        exports: ExportsMap = new ExportsMap()

        async call(...args: any[]) {
            this.resolve(await inner(...args))
        }
    }

    const agent = new CallAgent()

    agent.exports.insert(inner.name, {
        property_key: "call",
        adder: (scope: Scope) => scope.add("fn_decl", Scope.ident(inner.name))
    })

    return await agent
}

export default {
    Agent,
    AgentController,
    use: use_decorator,
    task: description_decorator,
    prompts: prompts_decorator,
    Interrupt,
    setLogLevel,
    getLogLevel,
    call
}
