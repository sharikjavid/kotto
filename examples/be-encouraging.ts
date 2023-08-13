import ai from "trackway/mod.ts"

@ai.prompts(import.meta.url)
class BeEncouraging extends ai.Agent {
    @ai.use
    endImmediately(an_encouraging_statement: string) {
        this.resolve(an_encouraging_statement)
    }
}

console.log(await new BeEncouraging())
