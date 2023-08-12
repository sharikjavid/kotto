import ai from "trackway/mod.ts"

type Sentiment = "bad" | "neutral" | "good"

@ai.prompts(import.meta.url)
class WhatLanguageIsThis extends ai.Agent {
    /**
     * Retrieve an input string from the user.
     */
    @ai.use
    getInput(): string {
        const ans = prompt("What do you want to say?")
        if (ans === null) throw new Error("You need to say something!")
        else return ans
    }

    /**
     * If the input is invalid or does not make sense, flag it.
     * @param {string} reason Why it was flagged.
     */
    @ai.use
    recordInvalidInput(reason: string) {
        throw new ai.Interrupt(reason)
    }

    /**
     * Record a sentiment and a one-word summary, associated to the input.
     * @param {Sentiment} sentiment The sentiment to associate to the input.
     * @param summary A one-word summary of the input.
     */
    @ai.use
    setSentiment(sentiment: Sentiment, summary: string) {
        this.resolve({
            sentiment,
            summary
        })
    }
}

console.log(await new WhatLanguageIsThis())
