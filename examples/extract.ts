import * as ai from "../mod.ts"

type Data = {
    first_name?: string,

    age?: number,

    location?: string,

    sentiment?: "positive" | "negative",

    url?: string
}

class Extract {
    constructor(public raw: string) {}

    /**
     * Obtain a raw string
     *
     * @returns {string} A raw string given by the user
     */
    @ai.use
    getRawString(): string {
        return this.raw
    }

    /**
     * End the task by extracting structured data from the raw string
     *
     * @param {Data} structured a structured Data object
     */
    @ai.use
    setData(structured: Data) {
        console.log(structured)
        throw new ai.Exit(structured)
    }
}

export default ({ argv }: ai.AgentOptions) => new Extract(argv[0])
