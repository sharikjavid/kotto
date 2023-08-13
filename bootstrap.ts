import * as colors from "https://deno.land/std@0.198.0/fmt/colors.ts"
import * as path from "https://deno.land/std@0.198.0/path/mod.ts"
import * as fs from "https://deno.land/std@0.198.0/fs/mod.ts"
import { grantOrThrow } from "https://deno.land/std@0.198.0/permissions/mod.ts"

export type LogLevel = "silent" | "quiet" | "introspective" | "debug"

let LOG_LEVEL: LogLevel = "introspective"

export function setLogLevel(level: LogLevel) {
    LOG_LEVEL = level
}

export function getLogLevel(): LogLevel {
    return LOG_LEVEL
}

export type LogEntry = {
    prefix?: string,
    color?: (_: string) => string,
    text: string,
    target?: "stdout" | "stderr"
}

export function getLogPrefix(prefix?: string, color?: (_: string) => string): string {
    prefix = prefix || "trackway"
    color = color || colors.cyan
    return color(`${prefix}:`)
}

export function renderLog({ prefix, color, text }: LogEntry): string {
    return `${getLogPrefix(prefix, color)} ${text}`
}

export function log({prefix, color, text, target}: LogEntry) {
    if (LOG_LEVEL == "silent") return
    const content = renderLog({ prefix, color, text })
    if (target || "stderr" == "stderr")
        console.error(content)
    else
        console.log(content)
}

export function info(text: string) {
    return log({ text })
}

export function error(text: string) {
    return log({ text, color: (s: string) => colors.red(colors.bold(s)) })
}

export function warn(text: string) {
    return log({ text, color: colors.yellow })
}

export function debug(entry: LogEntry) {
    if (getLogLevel() == "debug") log(entry)
}

export function ask(question: string): "yes" | "no" {
    const ans = prompt(renderLog({ text: `${question} [Y/n]`, color: colors.yellow }))
    if (!(ans === "y" || ans === "Y" || ans === null)) {
        return "no"
    } else {
        return "yes"
    }
}

type Platform = {
    machine: string,
    os: string
}

async function doUname(): Promise<Platform> {
    const child = doRun({
        exec: "uname",
        args: ["-sm"],
        stdout: "piped"
    })

    if ((await child.status).success) {
        const output = new TextDecoder().decode((await child.output()).stdout)
        const [os, machine] = output.toLowerCase().split(" ")
        return {
            machine,
            os
        }
    }

    throw new Error("unknown or unsupported platform, sorry")
}

async function resolveLatestRelease(): Promise<URL> {
    const { machine, os } = await doUname()
    return new URL(`https://github.com/brokad/trackway/releases/latest/download/tc-${os}-${machine}`)
}

async function doBootstrap() {
    if (ask("could not find tc, do you want to install it now?") != "yes")
        return
    
    info("this will download it from github and put it under ${HOME}/.local/bin/tc")
    info("this operation requires the following permissions:")
    info("  * read/write filesystem access")
    info("  * net access")
    info("  * env access (for locating your ${HOME})")

    await grantOrThrow({
        name: "net"
    }, {
        name: "read"
    }, {
        name: "write"
    }, {
        name: "env"
    })

    const github_bin_url = await resolveLatestRelease()

    const home = Deno.env.get("HOME")

    if (home === undefined) throw new Error("could not locate home")

    const bin_target_path = path.join(home, ".local/bin/tc")
    const bin_target_dir = path.dirname(bin_target_path)

    await fs.ensureDir(bin_target_dir)

    const resp = await fetch(new Request(github_bin_url, {
        redirect: "follow"
    }))

    if (resp.status != 200) {
        error(`could not download tc: ${resp.statusText}`)
        return
    }

    info(`downloading ${github_bin_url}`)

    const f = await Deno.open(bin_target_path, {
        write: true,
        create: true,
    })

    await resp.body?.pipeTo(f.writable)

    await Deno.chmod(bin_target_path, 0o755)

    if(await isAvailable()) {
        info("all done 🎉")
    } else {
        error("couldn't make sure tc is available, try running this in your terminal and try again:")
        error(`    export PATH=${bin_target_dir}:\${PATH}`)
    }
}

type RunParameters = {
    exec?: string
    args?: string[],
    stdout?: "piped" | "inherit" | "null",
    should_prompt?: boolean
}

export function doRun(params?: RunParameters): Deno.ChildProcess | undefined {
    try {
        const cmd = new Deno.Command(params?.exec || "tc", {
            args: params?.args,
            stdout: params?.stdout
        })
        return cmd.spawn()
    } catch (e) {
        if (e instanceof Deno.errors.NotFound) return undefined
        else throw e
    }
}

async function ensurePermissionsToRun(params?: RunParameters) {
    const run_permission: Deno.PermissionDescriptor = {
        name: "run"
    }

    const should_prompt = params?.should_prompt || true

    const permission_status = await Deno.permissions.query(run_permission)
    if (permission_status.state != "granted" && should_prompt) {
        info("we need to run tc (a native binary) in order to compile prompts, is that ok?")
        await Deno.permissions.request(run_permission)
    }
}

async function havePermissionsToRun(params?: RunParameters): Promise<boolean> {
    return ensurePermissionsToRun(params).then(() => true).catch(() => false)
}

async function isAvailable(params?: RunParameters): Promise<boolean> {
    const child = doRun({ exec: params?.exec })
    return child?.status?.then((status) => status.success) || false
}

if (await havePermissionsToRun() && !await isAvailable()) {
    await doBootstrap()
}
