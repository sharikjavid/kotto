import ai from "trackway/mod.ts"

type Language = "english" | "french" | "german"

type Output = {
    language: Language
}

/**
 * A class which, when awaited, returns the spoken language
 * in which the input string is written.
 */
@ai.prompts(import.meta.url)
class WhatLanguageIsThis extends ai.Agent {
    output?: Output

    /**
     * Retrieve the input string.
     */
    @ai.use
    getInput(): string {
        const ans = prompt("What do you want to say? ")
        if (ans === null) throw new Error("You need to say something!")
        else return ans
    }

    /**
     * Fulfil the task, resolving it to the given output.
     * @param output The output to which the task resolves.
     */
    @ai.use
    setOutput(output: Output) {
        this.output = output
        this.done()
    }

    /**
     * Reject the task.
     * @param reason Why the task was rejected.
     */
    @ai.use
    reject(reason: string) {
        throw new Error(reason)
    }
}

console.log(await new WhatLanguageIsThis())
