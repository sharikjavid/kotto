import ai from "../mod.ts"

type Feedback = {
    /// An overall assessment of the feedback given
    overall: "positive" | "neutral" | "negative"

    /// A value between -1 (most negative) and +1 (most positive)
    sentiment: number

    /// The activity that the feedback given was about
    activity: string
}

class ExtractFeedback extends ai.Agent {
    constructor(public raw: string) { super() }

    /**
     * Obtain a raw feedback string
     *
     * @returns {string} A raw feedback string
     */
    @ai.use
    getRawFeedback(): string {
        return this.raw
    }

    /**
     * Print a structured {Feedback} derived from a raw feedback
     *
     * @param {Feedback} structured a structured feedback object
     */
    @ai.use
    printStructuredFeedback(structured: Feedback) {
        console.log(structured)
        this.resolve()
    }
}

ai.run(new ExtractFeedback("I love Game of Thrones!"))

// prints:
// {
//   overall: "positive",
//   sentiment: 0.8,
//   activity: "Game of Thrones"
// }
