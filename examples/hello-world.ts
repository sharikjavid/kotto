import ai from "../mod.ts"

/**
 * Prints an encouraging statement.
 * 
 * @param {string} message Should be encouraging.
 */
function print(message: string) {
    console.log(message)
}

await ai.call(print)

// prints: "Keep up the good work!"
