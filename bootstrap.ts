import * as colors from "https://deno.land/std@0.198.0/fmt/colors.ts"
import * as path from "https://deno.land/std@0.198.0/path/mod.ts"
import * as fs from "https://deno.land/std@0.198.0/fs/mod.ts"
import { grantOrThrow } from "https://deno.land/std@0.198.0/permissions/mod.ts"

export type LogLevel = "silent" | "quiet" | "introspective" | "debug"

let LOG_LEVEL: LogLevel = "introspective"

const TRACKWAY_REPO = "ssh://git@github.com/brokad/trackway"

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
    const cmd = Deno.Command("uname", {
        args: ["-sm"]
    })
    const stdout = (await cmd.output()).stdout
    const output = new TextDecoder().decode(stdout)
    const [os, machine] = output.toLowerCase().split(" ")
    return {
        machine,
        os
    }
}

async function resolveLatestRelease(): Promise<URL> {
    const { machine, os } = await doUname()
    return new URL(`https://github.com/brokad/trackway/releases/latest/download/tc-${os}-${machine}`)
}

async function donwloadLatestRelease() {
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
}

async function runCargoInstall() {
    try {
        const cmd = new Deno.Command("cargo", {
            args: ["install", "--bin=tc", `--git=${TRACKWAY_REPO}.git`]
        })
        if (!(await cmd.spawn().status).success) {
            warn("cargo seems to have exited unsuccessfully, things may not work as expected")
        }
    } catch (e) {
        if (e instanceof Deno.errors.NotFound) {
            error("couldn't find a Rust toolchain")
            error("read more here: https://www.rust-lang.org/tools/install")
        }
        throw e
    }
}

async function doBootstrap() {
    if (ask("could not find tc, do you want to install it now?") != "yes")
        return

    info("we'll build tc from source")
    await runCargoInstall()

    if(await isAvailable()) {
        info("all done 🎉")
    } else {
        error("couldn't make sure tc is available: is tc in your $PATH? then try again")
    }
}

type RunParameters = {
    exec?: string
    output_path?: string,
    urls?: URL[],
    should_prompt?: boolean
}

export function doRun(params?: RunParameters): Deno.ChildProcess {
    let args = []
    let stdout = "inherit"

    if (params?.output_path !== undefined) {
        args.push(`-o=${params.output_path}`)
    } else {
        stdout = "piped"
    }

    if (params?.urls !== undefined) {
        args.push(...params.urls)
    }

    const cmd = new Deno.Command(params?.exec || "tc", {
        args,
        stdout,
        stderr: "inherit"
    })

    return cmd.spawn()
}

async function ensurePermissionsToRun(params?: RunParameters) {
    const permissions = [{
        name: "run",
        command: "tc"
    }, {
        name: "net",
        host: "damien.sh:443"
    }, {
        name: "net",
        host: "api.openai.com"
    }, {
            name: "read",
            path: "./"
    }, {
        name: "write",
        path: "./"
    }, {
        name: "env",
        variable: "OPENAI_KEY"
    }]

    const should_prompt = params?.should_prompt || true

    let missing_perms = false
    for (const permission of permissions) {
        const permission_status = await Deno.permissions.query(permission)
        missing_perms |= permission_status.state != "granted"
    }

    if (missing_perms && should_prompt) {
        info("we need to run with the following permissions:")
        info("  *        run: to run tc, a companion native binary which generates prompt files")
        info("  * read/write: to manipulate prompt files generated by tc")
        info("  *        env: to read the value of the OPENAI_KEY environment variable")
        info("  *        net: to interact with OpenAI's API")
        warn("you will now be prompted to grant those permissions (rerun deno with `-A` to silent this)\n")
        await grantOrThrow(permissions)
        console.log()
    }
}

async function havePermissionsToRun(params?: RunParameters): Promise<boolean> {
    return ensurePermissionsToRun(params).then(() => true).catch(() => false)
}

async function isAvailable(params?: RunParameters): Promise<boolean> {
    let child

    try {
        child = doRun({ exec: params?.exec })
    } catch (e) {
        if (e instanceof Deno.errors.NotFound) {
            return false
        } else {
            throw e
        }
    }

    return child?.status?.then((status) => status.success) || false
}

if (await havePermissionsToRun() && !await isAvailable()) {
    await doBootstrap()
}
