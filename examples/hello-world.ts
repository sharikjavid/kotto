import * as ai from "../mod.ts"

export class HelloWorld {
    /**
     * Cheer me up!
     * 
     * @param {string} message An encouraging message.
     */
    @ai.use
    cheer(message: string) {
        console.log(message)
        throw new ai.Exit(message)
    }
}

export default () => new HelloWorld()