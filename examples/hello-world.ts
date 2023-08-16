import ai from "../mod.ts"

class HelloWorld {
    /**
     * Cheer me up!
     * 
     * @param {string} message An encouraging message.
     */
    @ai.use
    cheer(message: string) {
        throw new ai.Exit(message)
    }
}

console.log(await ai.run(new HelloWorld()))
// prints: "Keep up the good work!"
