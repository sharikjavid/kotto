use std::path::Path;
use std::io::Write;

use crate::{AnyError, anyhow, CanPush};
use crate::{ast, filter, codegen, emit, util};
use crate::prompts::{Prompt, Prompts, PromptsWriter, PromptType};

use codegen::Node;
use deno_ast::ModuleSpecifier;

use tracing::{event, Level};

const PROMPTS_EXT: &'static str = "prompts.js";

pub async fn parse_module(module_specifier: String, module_source: String) -> Result<deno_ast::ParsedSource, AnyError> {
    let media_type = deno_ast::MediaType::from_path(Path::new(&module_specifier));
    let parse_params = deno_ast::ParseParams {
        specifier: module_specifier.clone(),
        text_info: deno_ast::SourceTextInfo::from_string(module_source),
        media_type,
        capture_tokens: false,
        scope_analysis: true,
        maybe_syntax: None
    };

    let parsed_source = deno_ast::parse_module(parse_params)?;

    if parsed_source.is_module() {
        Ok(parsed_source)
    } else {
        Err(AnyError::msg(format!("not a module: {}", module_specifier)))
    }
}

pub async fn compile_prompts_for_module<'p, C>(
    parsed_module: &deno_ast::ParsedSource,
    prompt_writer: &mut PromptsWriter<'p, C>
) -> Result<(), AnyError>
where
    C: CanPush<Prompt>
{
    let comments = parsed_module.comments().as_single_threaded();
    let module = parsed_module.module();

    let filter_params = filter::FilterParams::default();
    let filtered_module = filter::run_filters(filter_params, module).await?;

    for type_alias_decl in filtered_module.type_alias_decls.values() {
        prompt_writer.set_id(format!("type_alias_decl.{}", type_alias_decl.id))?;
        prompt_writer.set_type(PromptType::TypeScript);

        let mut buf = Vec::new();
        let mut emitter = emit::Emitter::new(&mut buf)
            .with_comments(&comments);
        type_alias_decl.emit_with(&mut emitter)?;
        let source_text = String::from_utf8(buf)?;
        prompt_writer.set_fmt(source_text)?;

        prompt_writer.push()?;
    }

    for class_decl in &filtered_module.class_decls {
        let id = format!("class_decl.{}", class_decl.class_decl.ident);
        prompt_writer.set_id(id)?;
        prompt_writer.set_type(PromptType::TypeScript);

        let mut buf = Vec::new();
        let mut emitter = emit::Emitter::new(&mut buf)
            .with_comments(&comments);
        class_decl.class_decl.emit_with(&mut emitter)?;
        let source_text = String::from_utf8(buf)?;
        prompt_writer.set_fmt(source_text)?;

        prompt_writer.push()?;

        for (prop_name, class_method) in &class_decl.class_methods {
            let prop_ident = if let Some(prop_ident) = prop_name.clone().ident() {
                prop_ident
            } else {
                continue
            };

            let id = format!("method_decl.{}.{}", class_decl.class_decl.ident, prop_ident);
            prompt_writer.set_id(id)?;
            prompt_writer.set_type(PromptType::TypeScript);

            let closure = filtered_module
                .find_closure_of_type_refs(&class_method.type_refs)
                .into_iter()
                .map(|id| format!("type_alias_decl.{}", ast::Ident::from(id)));

            prompt_writer.add_to_context(closure)?;

            let mut buf = Vec::new();
            let mut emitter = emit::Emitter::new(&mut buf)
                .with_comments(&comments);
            class_method.emit_with(&mut emitter)?;
            let source_text = String::from_utf8(buf)?;
            prompt_writer.set_fmt(source_text)?;

            prompt_writer.push()?;
        }
    }

    Ok(())
}

pub async fn compile_prompts_for_specifiers<P>(
    specifiers: &[ModuleSpecifier],
    output: Option<P>
) -> Result<(), AnyError>
where
    P: AsRef<Path>
{
    for specifier in specifiers {
        let module_source = if specifier.scheme().contains("http") {
            let resp = reqwest::get(specifier.to_string()).await?;
            if resp.status() == 200 {
                resp.text().await?
            } else {
                return Err(anyhow!("could not retrieve {specifier}: {}", resp.status()))
            }
        } else {
            tokio::fs::read_to_string(specifier.path()).await?
        };

        let parsed_module = parse_module(specifier.to_string(), module_source).await?;

        let mut prompts: Vec<Prompt> = Vec::new();
        let mut prompt_writer = PromptsWriter::new(&mut prompts);

        event!(Level::INFO, "building for {}", specifier);

        compile_prompts_for_module(&parsed_module, &mut prompt_writer).await?;

        let prompts = Prompts(prompts);

        let mut writer: Box<dyn Write> = if let Some(base) = output.as_ref() {
            let prompts_path = util::add_extension_to_path(specifier.path(), PROMPTS_EXT);
            let output_path = base.as_ref().join(prompts_path.file_name().unwrap());
            Box::new(std::fs::File::create(output_path)?)
        } else {
            Box::new(std::io::stdout())
        };

        write!(&mut writer, "export const ast = ")?;
        serde_json::to_writer_pretty(writer, &prompts)?;
    }

    Ok(())
}