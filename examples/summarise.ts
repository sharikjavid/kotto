import * as ai from "../mod.ts"

import {cyan, bold} from "https://deno.land/std@0.198.0/fmt/colors.ts";
import {parse as parseFlags} from "https://deno.land/std@0.198.0/flags/mod.ts";

type Summary = {
    // What the project does
    what_does_it_do: string,
    // How to use the project
    how_to_use: string,
    // How to install the project
    how_to_install: string,
}

class Summarise {
    constructor(public readme: string) {
    }

    // Retrieve the content of the README.md of a GitHub repository
    @ai.use
    getReadme(): string {
        return this.readme
    }

    // Summarise the content of the README.md
    @ai.use
    setSummary({what_does_it_do, how_to_use, how_to_install}: Summary) {
        console.log(cyan(bold("# What does it do")))
        console.log(what_does_it_do)
        console.log()

        console.log(cyan(bold("# How to use")))
        console.log(how_to_use)
        console.log()

        console.log(cyan(bold("# How to install")))
        console.log(how_to_install)
        console.log()
        throw new ai.Exit()
    }
}

export default async ({argv}: ai.AgentOptions) => {
    const flags = parseFlags(argv, {
        string: ["branch"]
    })

    if (flags._[0] === undefined) {
        console.error(`a repo to summarise is required
        
to summarise a repo:

    trackway run summarise.ts -- brokad/trackway 
   
to summarise a specific branch:

    trackway run summarise.ts -- brokad/trackway --branch=master
        `)
        Deno.exit(1)
    }

    const repo = flags._[0]
    const branch = flags.branch || "main"

    const url = `https://raw.githubusercontent.com/${repo}/${branch}/README.md`
    const resp = await fetch(url)
    if (!resp.ok) {
        throw new Error(`failed to fetch ${url}`)
    }
    const readme = (await resp.text()).slice(0, 10000)

    return new Summarise(readme)
}