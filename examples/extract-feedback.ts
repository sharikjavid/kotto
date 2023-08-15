import ai from "../mod.ts"

type Feedback = {
    /// An overall assessment of the feedback given
    overall: "positive" | "neutral" | "negative"

    /// A value between -1 (most negative) and +1 (most positive)
    sentiment: number

    /// The activity that the feedback given was about
    activity: string
}

function process(feedback: Feedback) {
    console.log(feedback)
}

await ai.call(process, {
    input: "I'm the biggest fan of Game of Thrones!!"
})

// prints:
// {
//   overall: "positive",
//   sentiment: 0.8,
//   activity: "Game of Thrones"
// }
