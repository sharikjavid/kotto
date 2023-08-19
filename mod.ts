import { Prompts, Scope } from "./prompts.ts"
import { Naive, Template } from "./const.ts"

import * as errors from "./errors.ts"
import * as log from "./log.ts"
import * as llm from "./llm.ts"

export { RuntimeError, Interrupt, Feedback, Exit } from "./errors.ts"

export type AgentOptions = {
    argv: string[]
}

/**
 * Set the log level.
 */
export const setLogLevel = log.setLogLevel

/**
 * Get the current log level.
 */
export const getLogLevel = log.getLogLevel

const logger = log.logger

type FunctionCall = {
    name: string,
    reasoning?: string,
    arguments: any[]
}

type ExportDescriptor = {
    property_key: string,
    adder: (scope: Scope) => void,
    description?: string
}

class ExportsMap {
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

const description = (task: string) => {
    return <T extends { new (...args: any[]): {} } >(constructor: T) => {
        constructor.prototype.task = task
    }
}

const prompts = (prompts: string) => {
    return <T extends { new (...args: any[]): {} } >(constructor: T) => {
        constructor.prototype.prompts = prompts
    }
}

export const use: MethodDecorator = (target: any, property_key: string, _descriptor?: PropertyDescriptor) => {
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

/**
 * An agent is a class that has at least one @use decorated method.
 *
 * You can run agents with [[run]] or [[runOnce]].
 */
export interface Agent {
    [functions: string]: any;
}

export type Exited = {
    output: any
}

export function isExited(pending: Exited | Pending): pending is Exited {
    return "output" in pending
}

export type Pending = {
    role: "user" | "system"
    prompt?: string
}

export function isPending(exited: Exited | Pending): exited is Pending {
    return "prompt" in exited
}

/**
 * An agent controller is a class that manages the execution of an agent.
 *
 * You can run agents with [[run]] or [[runOnce]].
 */
export class AgentController {
    agent: Agent

    prompts: Prompts

    exports: ExportsMap

    llm: llm.LLM

    template: Template

    history: Action[] = []

    constructor(agent: Agent, prompts: Prompts, llm: llm.LLM) {
        this.agent = agent

        this.prompts = prompts

        this.exports = agent.exports || new ExportsMap()

        this.llm = llm

        this.template = agent.template || Naive
    }

    renderContext(): string {
        const scope = this.prompts.newScope()

        this.exports.forEach(({ property_key, adder }) => {
            logger.trace(`adding '${property_key}' to scope`)
            adder(scope)
        })

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
        try {
            await this.complete(prompt, role)
            return {
                role: "user"
            }
        }
        catch (err) {
            // TODO backoff
            if (err instanceof errors.Feedback) {
                logger.feedback(err)
                return {
                    prompt: err.message,
                    role: "system"
                }
            } else if (err instanceof errors.Interrupt) {
                logger.interrupt(err)
                throw err.value
            } else if (err instanceof errors.RuntimeError) {
                throw err
            } else if (err instanceof errors.Exit) {
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
            if (this.history.length == 0) {
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
            throw new errors.Feedback(`could not extract JSON from your response: ${completion}`)
        }

        logger.thought(response.reasoning || "(no reasoning given)")

        await this.doAction({ call: response })
    }

    async runToCompletion(): Promise<any> {
        let tick = undefined
        while (true) {
            tick = await this.tick(tick)
            if (isExited(tick)) {
                return tick.output
            }
        }
    }
}