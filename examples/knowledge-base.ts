import ai from "../mod.ts"

import { green } from "https://deno.land/std@0.198.0/fmt/colors.ts"

@ai.prompts(import.meta.url)
class KnowledgeBase extends ai.Agent {
    knowledge_base: Map<string, string[]> = new Map()

    /**
     * Ask the user if they have a question they want an answer to. This must only be used once.
     *
     * @param {string} query A message sent to the user
     * @returns {string | undefined} What the user replied (`null` if the user didn't reply anything)
     */
    @ai.use
    ask(query: string): string | null {
        return prompt(query)
    }

    /**
     * Search for a set of keywords in the knowledge base, attempting to find the answer among questions
     * that have been asked before. This is a cheap operation.
     *
     * A good number of keywords for a search is anywhere between 5 and 10.
     *
     * @param {string[]} keywords An array of keywords to search for
     * @returns {string[]} An array of answers found in the knowledge base. Empty if not answers were found.
     */
    @ai.use
    lookupFromKeywords(keywords: string[]): string[] {
        let output = []
        for (const keyword of keywords) {
            const hit = this.knowledge_base.get(keyword)
            if (hit !== undefined) output.push(hit)
        }
        return output
    }

    /**
     * If the answer to the user's question cannot be provided, the question must be asked to a researcher.
     *
     * *Caution* This is an expensive operation, and should not be done unless we have tried answering
     * the question using the existing knowledge base with `lookupFromKeywords`.
     *
     * @param {string} question The question that is asked to the researcher
     * @returns {string} The response provided by the researcher
     */
    @ai.use
    askResearcher(question: string): string {
        return prompt(green(`researcher: ${question}`))
    }

    /**
     * Save an answer in the knowledge base, so it can be cheaply recovered next time a user asks a
     * similar question.
     *
     * @param {string[]} keywords An array of keywords to associate the question with
     * @param {string} answer The answer that we should provide to the question
     */
    @ai.use
    saveAnswer(keywords: string[], answer: string) {
        for (const keyword of keywords) {
            if (this.knowledge_base.has(keyword)) {
                this.knowledge_base.get(keyword)?.push(answer)
            } else {
                this.knowledge_base.set(keyword, [answer])
            }
        }
    }

    /**
     * End the conversation, giving the answer to the user. Only give an answer that was recovered from
     * the knowledge base or otherwise was given directly by a researcher.
     *
     * @param {string} answer The answer to give the user.
     */
    @ai.use
    giveAnswer(answer: string) {
        this.resolve(answer)
    }
}

const knowledge_base = new KnowledgeBase()

// Run the agent forever, or until you get bored, whichever comes first.
while (true) {
    console.log(green(`llm: state of knowledge base: ${JSON.stringify(Array.from(knowledge_base.knowledge_base))}`))
    console.log(await knowledge_base)
}
