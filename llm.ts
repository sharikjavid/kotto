import {
  ChatCompletionRequestMessage,
  Configuration,
  type CreateChatCompletionRequest,
  OpenAIApi,
} from "./deps.ts";

import * as errors from "./errors.ts";

export interface LLM {
  complete(messages: ChatCompletionRequestMessage[]): Promise<string>;
}

export class OpenAIChatCompletion {
  #openai: OpenAIApi;
  #messages: ChatCompletionRequestMessage[] = [];
  #base: CreateChatCompletionRequest;

  get messages(): ChatCompletionRequestMessage[] {
    return this.#messages;
  }

  constructor(apiKey: string) {
    const configuration = new Configuration({
      apiKey,
    });

    this.#openai = new OpenAIApi(configuration);

    this.#base = {
      "model": "gpt-3.5-turbo",
      "messages": [],
    };
  }

  async send(
    messages: ChatCompletionRequestMessage[],
  ): Promise<ChatCompletionRequestMessage> {
    const req: CreateChatCompletionRequest = {
      ...this.#base,
      "messages": this.#messages.concat(messages),
    };

    let resp;
    try {
      resp = await this.#openai.createChatCompletion(req);
    } catch (err) {
      const data = await err.json();
      throw new errors.RuntimeError(`openai: ${data.error.message}`);
    }

    const resp_msg = resp.data.choices[0].message;

    if (resp_msg === undefined) {
      throw new errors.RuntimeError("Didn't receive a completion");
    }

    this.#messages.push(...messages, resp_msg);

    return resp_msg;
  }

  async complete(messages: ChatCompletionRequestMessage[]): Promise<string> {
    const resp = await this.send(messages);

    if (resp.content === undefined) {
      throw new errors.RuntimeError("Completion has empty content");
    }

    return resp.content;
  }
}
