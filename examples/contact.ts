import ai from "../mod.ts"

import { green } from "https://deno.land/std@0.198.0/fmt/colors.ts"

type UserInformation = {
    email?: string
    username?: string
    order_number?: string
}

@ai.prompts(import.meta.url)
class GetOrderDetails extends ai.Agent {
    info: UserInformation = {}

    /**
     * Ask the user for more information.
     *
     * @param {string} request Explain what you're requesting from the user. Formulate it as a question.
     * @returns {string} What the user says.
     */
    @ai.use
    ask(question: string): string {
        return prompt(question)
    }

    /**
     * Update the user's stored information card.
     *
     * Only the fields which are set in `info` are updated. Any field which is not set is not updated.
     *
     * @param {UserInformation} info The user information to update
     * @returns {boolean} `true` if the update was successful, `false` otherwise
     */
    @ai.use
    updateDetail(info: UserInformation): boolean {
        this.info = {
            ...this.info,
            ...info
        }
        console.log(green(`llm: updated the state of user card to ${JSON.stringify(this.info, null, 2)}`))
        return true
    }

    /**
     * Ends the conversation.
     *
     * Once all the details have been collected, the conversation can be ended.
     *
     * If the user starts being rude or aggressive, the conversation can be ended early.
     *
     * @param {string} goodbye A goodbye message for the user.
     */
    @ai.use
    endConversation(goodbye: string) {
        this.resolve(goodbye)
    }
}

console.log(await new GetOrderDetails())