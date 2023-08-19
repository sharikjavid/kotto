/*
 * This example shows how to add functionality to an agent by wrapping around I/O.
 *
 * It is a small utility that takes a command you are trying to run, gives it to the agent, and
 * asks it for help with making it work.
 *
 * For example:
 *     `deno run -A https://damien.sh/trackway/examples/fix.ts tar MyFile.tar.gz`
 * or
 *     `deno run -A https://damien.sh/trackway/examples/fix.ts grep -e "???" MyFile.txt`
 * and when it asks, tell it something like:
 *     `grep all IP addresses in file`
 */

import * as ai from "../mod.ts"

import * as colors from "https://deno.land/std@0.198.0/fmt/colors.ts"

type Command = {
    command: string,
    args: string[]
}

class Fix {
    cmd: Command

    constructor(args: string[]) {
        this.cmd = {
            command: args[0],
            args: args.slice(1)
        }
    }

    // Get the command the user is trying to run.
    //
    // The user is facing an issue with this command. You should ask the user
    // what issue they're facing with the `ask` function.
    @ai.use
    getCommand(): Command {
        return this.cmd
    }

    // Ask the user a question.
    //
    // Ask the user a question to find out what problem the user is facing.
    // Keep the question short.
    //
    // Returns: the user's response.
    @ai.use
    ask(question: string): string {
        return prompt(colors.dim(question))!
    }

    // Return a completed command.
    //
    // Once the user's problem is solved, this can be called with the
    // improved command.
    @ai.use
    async returnImprovedCommand(cmd: Command) {
        if (cmd === undefined)
            throw new ai.Feedback("you must provide a `cmd` argument")
        else if (cmd.command != this.cmd.command)
            throw new ai.Feedback("your command must be the same as the user's command")

        const flat = [cmd.command, ...cmd.args].join(" ")

        const res = prompt(colors.dim("llm thinks you want \`")
            + flat
            + colors.dim("\`, is that ok? (Y/n)"))

        if (res === null || res.toLowerCase() === "y") {
            await new Deno.Command(cmd.command, { args: cmd.args }).spawn().status
            throw new ai.Exit()
        } else {
            throw new ai.Feedback("the user is not happy with this command")
        }
    }
}

export default ({ argv }: ai.AgentOptions) => {
    if (argv[0] === undefined) {
        console.error(`${colors.red("fix:")} you must call this with a command to fix 

For example:

    trackway run fix.ts -- egrep -e "???" MyFile.txt`)
        Deno.exit(1)
    } else {
        return new Fix(argv)
    }
}






