function blockQuote(s: string, type?: string): string {
    return `\`\`\`${type || "TypeScript"}\n${s}\n\`\`\``
}

export interface Template {
    renderContext: (scope: Scope) => string
    renderOutput: (value?: object) => string
    renderError: (err: Error) => string
    parseResponse: (resp: string) => FunctionCall
}

export const Naive: Template = {
    renderContext: (scope: Scope) => {
        const flattened = scope
            .current()
            .filter((node) => node.type === "ts")
            .map((decl) => decl.fmt)
            .join("\n\n")

        const output_template = `}`

        return `You are the runtime of a JavaScript program, you decide which functions to call.

Here is the abbreviated code of the program:

${blockQuote(flattened)}

I am going to feed this discussion to an API. So do not be verbose, just tell me which function you want 
to call, with what argument, and I will tell you what the returned value is. Each of your prompts must 
be of the following JSON form:

\`\`\`json
{
   "name": "the name of the function you want to call",
   "reasoning": "the reasoning that you've used to arrive to the conclusion you should use this function",
   "arguments": [
        // ... the arguments of the function you want to call
   ]
}
\`\`\`

You must make sure that the function you are calling accepts the arguments you give it. This includes
checking the arguments have the correct type for that function (refer to the types defined above, and the 
built-in type definitions that are part of JavaScript/TypeScript's specification).

Let's begin!`
    },

    renderOutput: (value: object) => blockQuote(JSON.stringify(value), "json"),

    parseResponse: (resp: string) => JSON.parse(resp),

    renderError: (err: Error) => {
        return `error: ${err}.

Remember, your answers must be valid JSON objects, conforming to the following format (excluding the block quote):

\`\`\`json
{
   "name": "the name of the function you want to call",
   "reasoning": "the reasoning that you've used to arrive to the conclusion you should use this function",
   "arguments": [
        // ... the arguments of the function you want to call
   ]
}
\`\`\`

Your answer must not include anything other than a valid JSON object.
`
    }
}

