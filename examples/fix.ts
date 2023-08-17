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

import ai from "../mod.ts"

import * as colors from "https://deno.land/std@0.198.0/fmt/colors.ts"

type Command = {
    command: string,
    args: string[]
}

class Fix {
    cmd: Command = {
        command: Deno.args[0],
        args: Deno.args.slice(1)
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
    returnImprovedCommand(cmd: Command) {
        if (cmd === undefined)
            throw new ai.Feedback("you must provide a `cmd` argument")
        else if (cmd.command != this.cmd.command)
            throw new ai.Feedback("your command must be the same as the user's command")

        const flat = [cmd.command, ...cmd.args].join(" ")
        const res = prompt(colors.dim("llm thinks you want \`") + flat + colors.dim("\`, is that ok? (Y/n)"))
        if (res === null || res.toLowerCase() === "y") {
            throw new ai.Exit(cmd)
        } else {
            throw new ai.Feedback("the user is not happy with this command")
        }
    }
}

// Uncomment this to disable traces
//ai.setLogLevel("quiet")

// Ask for help on the command we passes in args
const fix = await ai.run(new Fix())
const cmd = new Deno.Command(fix.command, { args: fix.args })
await cmd.spawn().output()


