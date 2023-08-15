import { parse as parsePath, join as joinPath, fromFileUrl, toFileUrl } from "https://deno.land/std@0.198.0/path/mod.ts"

import { ensureDir } from "https://deno.land/std@0.198.0/fs/mod.ts"

import { doRun } from "./bootstrap.ts"
import { RuntimeError } from "./errors.ts"

export class Prompts {
    #source_url: URL
    #prompts?: PromptsModule
    #ready?: Promise<void>
    
    constructor(url?: string) {
        if (url === undefined) {
            url = Deno.mainModule
        }
        this.#source_url = new URL(url)
    }

    async loadPrompts(): Promise<PromptsModule> {
        const source_url = this.#source_url

        // TODO: don't use path stuff for URLs
        const parsed_path = parsePath(source_url.pathname)

        if (source_url.protocol !== "file:") {
            // if remote module, try to get `.prompts.js` for an early win
            const prompts_path = new URL(source_url)
            prompts_path.pathname = `${parsed_path.dir}/${parsed_path.name}.prompts.js`
            try {
                return await import(prompts_path.toString())
            }
            catch (e) {
                if (e instanceof TypeError && e.cause === "ERR_MODULE_NOT_FOUND") {
                    // absorb silently, we'll build below
                } else {
                    throw e
                }
            }
        }

        const local_build_path = joinPath(Deno.cwd(), ".trackway", "builds")

        await ensureDir(local_build_path)

        const proc = await doRun({
            urls: [source_url],
            output_path: local_build_path
        })

        if (!(await proc?.status)?.success) {
            throw new RuntimeError("tc exited unsuccessfully")
        }

        const output_path = toFileUrl(joinPath(local_build_path, `${parsed_path.name}.prompts.js`))

        return import(output_path.toString())
    }

    spawnBackgroundInit() {
        this.#ready = this.loadPrompts().then((prompts) => {
            this.#prompts = prompts
        })
    }

    async ensureReady() {
        if (this.#ready === undefined) {
            this.spawnBackgroundInit()
        }
        await this.#ready
    }

    newScope(): Scope {
        return new Scope(this.#prompts!)
    }
}

interface PromptsModule {
    ast: PromptNode[]
}

type PromptNode = {
    "type": "ts" | "plaintext"
    fmt: string
    id: string
    context?: string[]
}

export class Scope {
    #prompts: PromptsModule
    #current: Map<string, PromptNode>

    static child = Scope.ident("\\w+")

    static ident(pat: string): string {
        return `${pat}#\\d+`
    }

    constructor(prompts: PromptsModule) {
        this.#prompts = prompts
        this.#current = new Map()
    }

    iterFor(...pat: string[]): PromptNode[] {
        const regex_str = `^${pat.join("\\.")}$`
        const regex = new RegExp(regex_str)
        return this.#prompts.ast.filter((node) => regex.test(node.id))
    }

    add(...pat: string[]) {
        this.iterFor(...pat).forEach((node) => {
            this.#current.set(node.id, node)
            node.context?.forEach((node_id) => {
                if (!this.#current.has(node_id)) {
                    this.add(...node_id.split("."))
                }
            })
        })
    }
    
    current(): PromptNode[] {
        return Array.from(this.#current.values())
    }
}
