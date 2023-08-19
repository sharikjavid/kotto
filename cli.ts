import { toml, flags, toFileUrl, resolve, ensureDir, join, dirname } from "./deps.ts"

import { Prompts } from "./prompts.ts";
import { RuntimeError } from "./errors.ts";
import { AgentController } from "./mod.ts"
import { OpenAIChatCompletion } from "./llm.ts"
import { runCargoInstall } from "./utils.ts"
import * as log from "./log.ts"

const SEMVER = "0.1.0"

const HELP = `Let your code chat with LLMs

Docs: https://trackway.ai/docs
Tutorial: https://github.com/brokad/trackway#tutorial
Bugs: https://github.com/brokad/trackway/issues

To run an agent:

  trackway run https://trackway.ai/agents/hello.ts
  
To run an agent in debug mode:

  trackway debug https://trackway.ai/agents/hello.ts
  
To set OpenAI key:

  trackway config openai.key KEY

Usage: trackway [OPTIONS] [COMMAND]

Commands:
  run          Run an agent
  debug        Run an agent in debug mode
  config       Set configuration options
  deploy       Deploy an agent (coming soon)
  upgrade      Upgrade trackway
  help         Show this help message
  
Options:
    --help       Print help
    --version    Print version
`

const HELP_RUN = `Run an agent

To run an agent:

    trackway run https://trackway.ai/agents/hello.ts

Usage: trackway run [OPTIONS] PATH

Arguments:
    PATH         Path to the agent to run

Options:
    --trace      Enable trace logging
    --allow-exit Allow the agent to decide when to exit
`

const HELP_DEBUG = `Run an agent in debug mode

To run an agent in debug mode:

    trackway debug https://trackway.ai/agents/hello.ts

Usage: trackway debug [OPTIONS] PATH

Arguments:
    PATH         Path to the agent to run
    
Options:
    --allow-exit Allow the agent to decide when to exit
`

const HELP_CONFIG = `Set configuration options

To set OpenAI key:

    trackway config openai.key KEY

Usage: trackway config ATTR VALUE

Arguments:
    ATTR         The configuration attribute to set (e.g. openai.key)
    VALUE        The value to set the configuration attribute to
`

const HELP_UPGRADE = `Upgrade trackway
    
To upgrade trackway to the latest version:

    trackway upgrade
    
Usage: trackway upgrade
`

const HELP_VERSION = `trackway ${SEMVER}`

function getUserArgs(): string[] {
    const user_args = Deno.args.findIndex(arg => arg == "--")
    if (user_args == -1) {
        return []
    } else {
        return Deno.args.slice(user_args + 1)
    }
}

function renderHelp(command?: string, includeDesc = true) {
    let help
    switch (command) {
        case "run":
            help = HELP_RUN
            break
        case "debug":
            help = HELP_DEBUG
            break
        case "config":
            help = HELP_CONFIG
            break
        case "upgrade":
            help = HELP_UPGRADE
            break
        default:
            help = HELP
    }
    if (!includeDesc) help = help.split("\n").slice(1).join("\n")
    return help
}

type RunFlags = {
    path: string,
    trace: boolean,
    allow_exit: boolean,
    is_debug: boolean,
    openai_key: string
}

export async function doRun(args: RunFlags): Promise<any> {
    log.setLogLevel(args.trace ? "trace" : "quiet")

    if (args.allow_exit === true) {
        log.warn("allow_exit is not yet implemented")
    }

    const config = await getConfig()

    const openai_key = config.openai?.key
    if (openai_key === undefined) {
        throw new RuntimeError(`openai.key is not set

try running:

    trackway config openai.key KEY`)
    }

    const llm = new OpenAIChatCompletion(openai_key)

    const temp_dir = await Deno.makeTempDir({
        prefix: "trackway-"
    })

    let url
    try {
        url = new URL(args.path)
    } catch (_) {
        url = toFileUrl(resolve(args.path))
    }

    const mod = await import(url.toString())

    if (mod.default === undefined) {
        throw new RuntimeError(`module does not have a default export

try adding:

  export default () => new MyAgent()`)
    }

    const agent = mod.default({ args: getUserArgs() })

    const prompts = await Prompts.fromUrl(url, {
        work_dir: temp_dir
    })

    const ctl = new AgentController(agent, prompts, llm)

    return await ctl.runToCompletion()
}

type Config = {
    openai?: {
        key?: string
    }
}

const configValidator = {
    openai: {
        key: (key: string) => {
            if (key.length === 0) {
                throw new RuntimeError("key cannot be empty")
            }
        }
    }
}

const getConfigPath = () => join(Deno.env.get("HOME")!, ".config", "trackway", "config.toml")

async function getConfig(): Promise<Config> {
    const config_path = getConfigPath()

    let config: Config = {}

    try {
        config = toml.parse(await Deno.readTextFile(config_path))
    } catch (err) {
        if (err instanceof Deno.errors.NotFound) {
            await ensureDir(dirname(config_path))
        } else {
            throw err
        }
    }

    return config
}

function setConfig(config: Config) {
    return Deno.writeTextFile(getConfigPath(), toml.stringify(config))
}

async function config(attr: string, value: string) {
    const config = await getConfig()

    if (attr === "openai.key") {
        configValidator.openai.key(value)
        config.openai = {
            key: value
        }
    } else {
        throw new RuntimeError(`unknown configuration attribute '${attr}'`)
    }

    await setConfig(config)
}

async function upgrade() {
    log.info("installing trackway...")
    // TODO upgrade this cli too
    await runCargoInstall()
    log.info("trackway is installed 🎉")
}

interface ErrorExt extends Error {
    code?: number,
}

function unwind(err: ErrorExt) {
    if (err instanceof RuntimeError) {
        log.error(err.message)
        Deno.exit(err.code || 1)
    } else {
        throw err
    }
}

async function main() {
    const args = flags.parse(Deno.args, {
        boolean: ["help", "trace", "version", "allow-exit"]
    })

    if (args.version) {
        console.log(HELP_VERSION)
        Deno.exit(0)
    }

    if (args._.length === 0) {
        console.log(HELP)
        Deno.exit(0)
    }

    const command = args._[0].toString()

    if (args.help) {
        console.log(renderHelp(command))
        Deno.exit(0)
    }

    switch (command) {
        case "run":
        case "debug":
            if (typeof args._[1] !== "string") {
                const help = renderHelp(command, false)
                throw new RuntimeError(`${command} requires a PATH argument\n${help}`)
            }
            await doRun({
                path: args._[1],
                trace: args.trace || command == "debug",
                allow_exit: args["allow-exit"],
                is_debug: command == "debug",
                openai_key: "" // TODO
            })
            break
        case "config":
            if (typeof args._[1] !== "string" || typeof args._[2] !== "string") {
                const help = renderHelp(command, false)
                throw new RuntimeError(`${command} requires an ATTR and VALUE argument\n${help}`)
            }
            await config(args._[1], args._[2])
            break
        case "upgrade":
            await upgrade()
            break
        case "deploy":
            log.info("deploy is coming soon! 🚀")
            break
        default:
            throw new RuntimeError(`unknown command '${command}'\n${renderHelp()}`)
    }
}

if (import.meta.main) {
    try {
        await main()
    } catch (err) {
        unwind(err)
    }
}