import {
  OpenAI,
} from "./deps.ts";

import * as errors from "./errors.ts";

export interface LLM {
  complete(messages: OpenAI.Chat.CreateChatCompletionRequestMessage[]): Promise<string>;
}

export class OpenAIChatCompletion {
  #openai: OpenAI;
  #messages: OpenAI.Chat.CreateChatCompletionRequestMessage[] = [];
  #base: OpenAI.Chat.CompletionCreateParamsNonStreaming;

  get messages(): OpenAI.Chat.CreateChatCompletionRequestMessage[] {
    return this.#messages;
  }

  constructor(apiKey: string) {
    this.#openai = new OpenAI({
      apiKey
    });

    this.#base = {
      "model": "gpt-3.5-turbo",
      "messages": [],
    };
  }

  async send(
    messages: OpenAI.Chat.CreateChatCompletionRequestMessage[],
  ): Promise<OpenAI.Chat.CreateChatCompletionRequestMessage> {
    const req: OpenAI.Chat.CompletionCreateParamsNonStreaming = {
      ...this.#base,
      "messages": this.#messages.concat(messages),
    };

    let resp;
    try {
      resp = await this.#openai.chat.completions.create(req);
    } catch (err) {
      throw new errors.Internal("openai error", {
        context: err
      });
    }

    const resp_msg = resp.choices[0].message;

    if (resp_msg === undefined) {
      throw new errors.Internal("Didn't receive a completion");
    }

    this.#messages.push(...messages, resp_msg);

    return resp_msg;
  }

  async complete(messages: OpenAI.Chat.CreateChatCompletionRequestMessage[]): Promise<string> {
    const resp = await this.send(messages);

    if (!resp.content) {
      throw new errors.Internal("Completion has empty content");
    }

    return resp.content;
  }
}
