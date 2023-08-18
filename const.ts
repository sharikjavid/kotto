import { Scope } from "./prompts.ts"
import { FunctionCall } from "./mod.ts"
import {Feedback, RuntimeError} from "./errors.ts";

/**
 * Wrap around the string with md-style block quotes
 * @param s text to wrap
 * @param type the block quote format (e.g. ts, js, etc)
 */
function blockQuote(s: string, type?: string): string {
    return `\`\`\`${type || "TypeScript"}\n${s}\n\`\`\``
}

function parseResponseBlockQuote(resp: string): FunctionCall | undefined {
const lines = resp.split("\n")
    let start, end
    for (let i = 0; i < lines.length; i++) {
        if (BLOCK_QUOTE.test(lines[i])) {
            if (start === undefined) {
                start = i
            } else {
                end = i
                break
            }
        }
    }
    if (start !== undefined && end !== undefined) {
        return JSON.parse(lines.slice(start + 1, end).join("\n"))
    }
}

const BLOCK_QUOTE = /^```\w*$/

/**
 * A way to generate prompts from runtime objects
 */
export interface Template {
    renderContext: (scope: Scope) => string

    renderOutput: (value?: object) => string

    renderError: (err: Error) => string

    parseResponse: (resp: string) => FunctionCall
}

/**
 * The most naive prompts template
 *
 * Basically ask nicely if the LLM accepts to call functions and speak JSON
 */
export const Naive: Template = {
    renderContext: (scope: Scope) => {
        const flattened = scope
            .current()
            .filter((node) => node.type === "ts")
            .map((decl) => decl.fmt)
            .join("\n\n")

        return `You are the runtime of a JavaScript program, you decide which functions to call.

Here is the abbreviated code of the program:

${blockQuote(flattened)}

Each of your prompts must be of the following valid JSON form:

{
   "name": "the name of the function you want to call",
   "reasoning": "the reasoning that you've used to arrive to the conclusion you should use this function",
   "arguments": [
        "the", "arguments", "of", "the", "function", "you", "want", "to", "call"
   ]
}

You must make sure that the function you are calling accepts the arguments you give it.

Let's begin!`
    },

    renderOutput: (value?: object) => blockQuote(JSON.stringify(value), "json"),

    parseResponse: (resp: string) => {
        try {
            return JSON.parse(resp)
        } catch (err) {
            const res = parseResponseBlockQuote(resp)
            if (res === undefined) throw err
            else return res
        }
    },

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

Your answer must not include anything other than a valid JSON object.`
    }
}

