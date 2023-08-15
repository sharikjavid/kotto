import ai from "../mod.ts"

class HelloWorld extends ai.Agent {
    /**
     * Ask the user in what language they want to hear "Hello, world!"
     *
     * @param {string} query A message sent to the user
     * @returns {string} What the user replied
     */
    @ai.use
    ask(query: string): string {
        return prompt(query)!
    }

    /**
     * End the conversation with a "Hello, world!" in the desired language.
     *
     * @param {string} hello "Hello, world!" translated in the language the user requested.
     */
    @ai.use
    end(hello: string) {
        this.resolve(hello)
    }

    /**
     * End the conversation early.
     *
     * If we're unable to determine which language the user prefers, end the conversation early.
     *
     * @param {string} reason A reason why the conversation was ended early.
     */
    @ai.use
    unable(reason: string) {
        throw new ai.Interrupt(reason)
    }
}

console.log(await new HelloWorld())

