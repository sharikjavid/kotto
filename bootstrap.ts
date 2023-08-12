import * as colors from "https://deno.land/std@0.198.0/fmt/colors.ts"
import * as path from "https://deno.land/std@0.198.0/path/mod.ts"
import * as fs from "https://deno.land/std@0.198.0/fs/mod.ts"
import * as streams from "https://deno.land/std@0.198.0/streams/mod.ts"
import { grantOrThrow } from "https://deno.land/std@0.198.0/permissions/mod.ts"

type RunParameters = {
    exec?: string
    args?: string[],
    stdout?: "piped" | "inherit" | "null",
    should_prompt?: boolean
}

function getLogPrefix(color?: (_: string) => string): string {
    color = color || colors.cyan
    return color(colors.bold("trackway:"))
}

function log(msg: string, color?: (_: string) => string) {
    console.log(`${getLogPrefix(color)} ${msg}`)
}

function logLine() {
    console.error()
}

type Platform = {
    machine: string,
    os: string
}

async function doUname(): Promise<Platform> {
    const child = await doRun({
        exec: "uname",
        args: ["-sm"],
        stdout: "piped"
    })

    if ((await child.status).success) {
        const output_b = await streams.readAll(streams.readerFromStreamReader(child.stdout.getReader()))
        const output = new TextDecoder().decode(output_b)
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
    const ans = prompt(`${getLogPrefix(colors.yellow)} could not find tc, do you want to install it now? [Y/n]`)
    if (!(ans === "y" || ans === "Y" || ans === null)) {
        return
    }
    
    log("this will download it from github and put it under ${HOME}/.local/bin/tc")
    
    log("this operation requires the following permissions:")
    log("  * read/write filesystem access")
    log("  * net access")
    log("  * env access (for locating your ${HOME})")

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
        log(`could not download tc: ${resp.statusText}`, colors.red)
        return
    }

    log(`downloading ${github_bin_url}`)

    const f = await Deno.open(bin_target_path, {
        write: true,
        create: true,
    })

    await resp.body?.pipeTo(f.writable)

    await Deno.chmod(bin_target_path, 0o755)

    if(await isAvailable()) {
        log("all done 🎉")
    } else {
        log("we couldn't make sure tc is available, try running this in your terminal and try again:", colors.red)
        log(`    export PATH=${bin_target_dir}:\${PATH}`, colors.red)
    }
}

export async function doRun(params?: RunParameters): Promise<Deno.ChildProcess> {
    const cmd = new Deno.Command(params?.exec || "tc", {
        args: params?.args,
        stdout: params?.stdout
    })
    return await cmd.spawn()
}

async function ensurePermissionsToRun(params?: RunParameters) {
    const run_permission: Deno.PermissionDescriptor = {
        name: "run"
    }

    const should_prompt = params?.should_prompt || true

    const permission_status = await Deno.permissions.query(run_permission)
    if (permission_status.state != "granted" && should_prompt) {
        log("we need to run tc (a native binary) in order to compile prompts, is that ok?")
        await Deno.permissions.request(run_permission)
    }
}

async function havePermissionsToRun(params?: RunParameters): Promise<boolean> {
    return ensurePermissionsToRun(params).then(() => true).catch(() => false)
}

function isAvailable(params?: RunParameters): Promise<boolean> {
    return doRun({ exec: params?.exec })
        .then((child) => child.status)
        .then((status) => status.success)
        .catch((err) => {
            if (err instanceof Deno.errors.NotFound) return false
            else throw err
        })
}

if (await havePermissionsToRun() && !await isAvailable()) {
    await doBootstrap()
}
