import { join as joinPath, toFileUrl } from "./deps.ts";

import { Internal } from "./errors.ts";
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

type PromptTy =
    "ts"
    | "plaintext";

type PromptAstTy =
    "method_decl"
    | "class_decl"
    | "type_alias_decl"
    | "fn_decl";

type PromptNode = {
  "type": PromptTy;
  ast_ty: PromptAstTy;
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
      throw new Internal(
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
      throw new Internal(`expected path to a .ts, got ${mod_url}`);
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
    const ast_ty_regex_str = `^${pat[0]}$`;
    const ast_ty_regex = new RegExp(ast_ty_regex_str);
    const id_regex_str = `^${pat.slice(1).join("\\.")}$`;
    const id_regex = new RegExp(id_regex_str);
    return this.#prompts.ast.filter((node) => ast_ty_regex.test(node.ast_ty) && id_regex.test(node.id));
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
