import * as ai from "../mod.ts"

type Feedback = {
    /// An overall assessment of the feedback given
    overall: "positive" | "neutral" | "negative"

    /// A value between -1 (most negative) and +1 (most positive)
    sentiment: number

    /// The activity that the feedback given was about
    activity: string
}

class ExtractFeedback {
    constructor(public raw: string) {}

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
     * End the task by providing a structured {Feedback} derived from a raw feedback
     *
     * @param {Feedback} structured a structured feedback object
     */
    @ai.use
    end(structured: Feedback) {
        console.log(structured)
        throw new ai.Exit(structured)
    }
}

export default () => new ExtractFeedback("I really enjoyed watching Game of Thrones!")

// prints:
// {
//   overall: "positive",
//   sentiment: 0.8,
//   activity: "Game of Thrones"
// }
