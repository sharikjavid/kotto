import { ChatCompletionRequestMessage, CreateChatCompletionRequest, Configuration, OpenAIApi } from "npm:openai@^3.3.0"

import { parse as parsePath, join as joinPath } from "https://deno.land/std@0.198.0/path/mod.ts"

export type LLMFunctionCall = {
    function_name: string,
    function_parameters: any[]
}

class LLM {
    #openai: OpenAIApi
    #messages: ChatCompletionRequestMessage[] = []
    #base: CreateChatCompletionRequest

    constructor() {
        const configuration = new Configuration({
            apiKey: Deno.env.get("OPENAI_KEY")
        });

        this.#openai = new OpenAIApi(configuration)

        this.#base = {
            "model": "gpt-3.5-turbo",
            "messages": [],
            "functions": [{
                "name": "eval",
                "description": "call one of the functions (make sure the function parameters adhere to the type signature of the function)",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "function_name": {
                            "type": "string",
                            "description": "the name of the function to call (it must be one of the functions in the list)"
                        },
                        "function_parameters": {
                            "type": "array",
                            "description": "the parameters to call the function with, encoded as an array of JSON values (the parameters must adhere to the type signature of the function)",
                            "items": {
                                "type": ["number","string","boolean","object","array", "null"]
                            }
                        }
                    },
                },
            }],
            "function_call": {
                "name": "eval"
            }
        }
    }

    async send(messages: ChatCompletionRequestMessage[]): Promise<ChatCompletionRequestMessage> {
        let req: CreateChatCompletionRequest = {
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

    async next(messages: ChatCompletionRequestMessage[]): Promise<LLMFunctionCall> {
        const resp = await this.send(messages)

        if (resp.function_call?.arguments === undefined) throw new Error("TODO")

        return JSON.parse(resp.function_call.arguments)
    }
}

export type ExportDescriptor = {
    property_key: string,
    description?: string
}

export class ExportsMap {
    #inner: Map<string, ExportDescriptor> = new Map()

    get(property_key: string): ExportDescriptor | undefined {
        return this.#inner.get(propertyKey)
    }

    insert(property_key: string, descriptor: ExportDescriptor) {
        this.#inner.set(propertyKey, descriptor)
    }
}

type ConstructorDecorator = <T extends { new (...args: any[]): {} } >(constructor: T) => any

type MethodDecorator = (target: any, property_key: string, descriptor: PropertyDescriptor) => void

const description_decorator = (description: string) => {
    return <T extends { new (...args: any[]): {} } >(constructor: T) => {
        return class extends constructor {
            description: string = description
        }
    }
}

const prompts_decorator = (prompts: string) => {
    return <T extends { new (...args: any[]): {} } >(constructor: T) => {
        return class extends constructor {
            prompts: string = prompts
        }
    }
}

const use_decorator: () => MethodDecorator = () => {
    return (target: any, property_key: string, descriptor?: PropertyDescriptor) => {
        target.exports.insert(property_key, {
            property_key
        })
    }
}

export class Agent {
    is_done: boolean = false
    exports: Map<keyof Agent, ExportDescriptor>
    prompts: string
    description?: string

    end() {
        this.is_done = true
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

export class Prompts {
    #source_path: string
    #prompts: PromptDescriptor[]

    constructor(path: string) {
        let source_path = parsePath(path)
        this.#source_path = joinPath(source_path.dir, `${source_path.name}.prompts.js`)
    }

    async load() {
        this.#prompts = (await import(this.#source_path)).ast
    }


}

export class AgentController {
    agent: Agent
    llm: LLM
    prompts: Prompts
    do_init: Promise<void>

    constructor(agent: Agent) {
        this.agent = agent
        this.llm = new LLM()
        this.prompts = new Prompts(agent.prompts)
        this.do_init = new Promise((onResolve, onReject) => this.doInit().then(onResolve, onReject))
    }

    async doInit() {
        await this.prompts.load()
    }

    async doNext(): Promise<boolean> {
        await this.do_init

        if (this.agent.is_done) {
            throw new Error("agent is done")
        }

        // TODO

        return this.agent.is_done
    }

    async runToCompletion() {
        let is_done
        do {
            is_done = await doNext()
        } while (!is_done)
    }

    then(onResolve, onReject) {
        onResolve()
    }
}

export const ai = {
    Agent,
    use: use_decorator,
    task: description_decorator,
    prompts: prompts_decorator
}

export default ai