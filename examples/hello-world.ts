import ai from "../mod.ts"

class HelloWorld extends ai.Agent {
    /**
     * Prints an encouraging statement.
     * 
     * @param {string} message Should be encouraging.
     */
    @ai.use
    print(message: string) {
        console.log(message)
    }
}

await ai.run(new HelloWorld())
// prints: "Keep up the good work!"
