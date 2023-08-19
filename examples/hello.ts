import * as ai from "../mod.ts"

export class Hello {
    /**
     * Call this function with a positive message for the user. Make it encouraging and fun!
     * 
     * @param {string} message A positive message
     */
    @ai.use
    positivity(message: string) {
        console.log(message)
        throw new ai.Exit(message)
    }
}

export default () => new Hello()