import { ChatCompletionRequestMessage, CreateChatCompletionRequest, Configuration, OpenAIApi } from "npm:openai@^3.3.0"

import * as errors from "./errors.ts"

export class OpenAIChatCompletion {
    #openai: OpenAIApi
    #messages: ChatCompletionRequestMessage[] = []
    #base: CreateChatCompletionRequest

    get messages(): ChatCompletionRequestMessage[] {
        return this.#messages
    }

    constructor() {
        const apiKey = Deno.env.get("OPENAI_KEY")

        if (apiKey === undefined) {
            throw new errors.RuntimeError("The `OPENAI_KEY` env variable must be set.")
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
            throw new errors.Interrupt(err)
        }

        const resp_msg = resp.data.choices[0].message

        if (resp_msg === undefined) {
            throw new errors.RuntimeError("Didn't receive a completion")
        }

        this.#messages.push(...messages, resp_msg)

        return resp_msg
    }

    async complete(messages: ChatCompletionRequestMessage[]): Promise<string> {
        const resp = await this.send(messages)

        if (resp.content === undefined) {
            throw new errors.RuntimeError("Completion has empty content")
        }

        return resp.content
    }
}