use std::path::Path;

use crate::AnyError;
use crate::{filter, codegen, emit, util};
use crate::prompts::{Prompt, Prompts, PromptsWriter, PromptType};

use codegen::Node;

use tracing::{event, Level};

const PROMPTS_EXT: &'static str = ".prompts";

pub async fn parse_module<P>(module_path: P) -> Result<deno_ast::ParsedSource, AnyError>
where
    P: AsRef<Path>
{
    let module_path = module_path.as_ref();

    let module_source = tokio::fs::read_to_string(module_path).await?;

    let parse_params = deno_ast::ParseParams {
        specifier: module_path.to_string_lossy().to_string(),
        text_info: deno_ast::SourceTextInfo::from_string(module_source),
        media_type: deno_ast::MediaType::from_path(module_path),
        capture_tokens: false,
        scope_analysis: true,
        maybe_syntax: None
    };

    let parsed_source = deno_ast::parse_module(parse_params)?;

    if parsed_source.is_module() {
        Ok(parsed_source)
    } else {
        Err(AnyError::msg(format!("not a module: {}", module_path.display())))
    }
}

pub async fn compile_prompts<P>(paths: &[P]) -> Result<(), AnyError>
where
    P: AsRef<Path>
{
    for path in paths {
        let path = path.as_ref();

        let parsed_module = parse_module(&path).await?;
        let module = parsed_module.module();

        let filter_params = filter::FilterParams::default();
        let filtered_module = filter::run_filters(filter_params, module).await?;

        let mut prompts: Vec<Prompt> = Vec::new();
        let mut prompt_writer = PromptsWriter::new(&mut prompts);

        for type_alias_decl in filtered_module.type_alias_decls {
            prompt_writer.set_id(format!("type_alias_decl.{}", type_alias_decl.id))?;
            prompt_writer.set_type(PromptType::TypeScript);

            let mut buf = Vec::new();
            let mut emitter = emit::Emitter::new(&mut buf);
            type_alias_decl.emit_with(&mut emitter)?;
            let source_text = String::from_utf8(buf)?;
            prompt_writer.set_fmt(source_text)?;

            prompt_writer.push()?;
        }

        let prompts = Prompts(prompts);

        let prompts_path = util::add_extension_to_path(path, PROMPTS_EXT).unwrap();
        event!(Level::INFO, "writing {}", prompts_path.display());
        let prompts_file = std::fs::File::create(prompts_path)?;
        serde_json::to_writer_pretty(prompts_file, &prompts)?;
    }

    Ok(())
}