import { join as joinPath, toFileUrl } from "./deps.ts";

import { RuntimeError } from "./errors.ts";
import { logger } from "./log.ts";

export type RunParameters = {
  exec?: string;
  output_path?: string;
  urls?: URL[];
  should_prompt?: boolean;
};

export function runRust(params?: RunParameters): Deno.ChildProcess {
  const args = [];
  let stdout: "inherit" | "piped" = "inherit";

  if (params?.output_path !== undefined) {
    args.push(`-o=${params.output_path}`);
  } else {
    stdout = "piped";
  }

  if (params?.urls !== undefined) {
    args.push(...params.urls.map((url) => url.toString()));
  }

  const cmd = new Deno.Command(params?.exec || "kottoc", {
    args,
    stdout,
    stderr: "inherit",
  });

  return cmd.spawn();
}

type PromptOpts = {
  work_dir?: string;
};

interface PromptsModule {
  ast: PromptNode[];
}

type PromptNode = {
  "type": "ts" | "plaintext";
  fmt: string;
  id: string;
  context?: string[];
};

export class Prompts {
  readonly #mod: PromptsModule;

  constructor(mod: PromptsModule) {
    this.#mod = mod;
  }

  static fromModule(mod: any): Prompts {
    return new Prompts(mod);
  }

  static async fromBuiltUrl(import_url: URL): Promise<Prompts> {
    return Prompts.fromModule(await import(import_url.href));
  }

  static async build(source_url: URL, opts: PromptOpts): Promise<URL> {
    const output_path = opts.work_dir || Deno.cwd();

    const file_name = source_url.pathname.split("/").pop()!;

    const output_name = `${file_name.split(".")[0]}.prompts.js`;

    const proc = runRust({
      urls: [source_url],
      output_path,
    });

    if (!(await proc?.status)?.success) {
      throw new RuntimeError(
          `failed to generate prompts for ${source_url.toString()}`,
      );
    }

    const local_import_path = joinPath(output_path, output_name);
    logger.trace(
        "prompts",
        `generated for ${source_url.toString()}, output: ${local_import_path}`,
    );

    return toFileUrl(local_import_path)
  }

  static async fromDefault(mod_url: string): Promise<Prompts> {
    if (!mod_url.endsWith(".ts")) {
      throw new RuntimeError(`expected path to a .ts, got ${mod_url}`);
    }

    mod_url = `${mod_url.slice(0, -3)}.prompts.js`;

    return Prompts.fromModule(await import(mod_url));
  }

  newScope(): Scope {
    return new Scope(this.#mod);
  }
}

export class Scope {
  #prompts: PromptsModule;
  #current: Map<string, PromptNode>;

  static child = Scope.ident("\\w+");

  static ident(pat: string): string {
    return `${pat}#\\d+`;
  }

  constructor(prompts: PromptsModule) {
    this.#prompts = prompts;
    this.#current = new Map();
  }

  iterFor(...pat: string[]): PromptNode[] {
    const regex_str = `^${pat.join("\\.")}$`;
    const regex = new RegExp(regex_str);
    return this.#prompts.ast.filter((node) => regex.test(node.id));
  }

  addFromId(...pat: string[]) {
    this.iterFor(...pat).forEach((node) => {
      this.#current.set(node.id, node);
      node.context?.forEach((node_id) => {
        if (!this.#current.has(node_id)) {
          this.addFromId(...node_id.split("."));
        }
      });
    });
  }

  addNode(node: PromptNode) {
    this.#current.set(node.id, node);
  }

  current(): PromptNode[] {
    return Array.from(this.#current.values());
  }
}
