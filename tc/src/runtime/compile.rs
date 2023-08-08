use std::cell::RefCell;
use std::collections::HashMap;
use std::io::stderr;
use std::rc::Rc;
use std::sync::Arc;
use deno_ast::{MediaType, ParseParams, SourceTextInfo};
use deno_ast::swc::visit::{VisitMut, Visit};
use deno_ast::swc::ast::{CallExpr, ClassDecl, Ident, Param, Str, TsTypeAliasDecl, TsTypeRef, Lit, Decorator, Class, ClassMethod, Module, Id};
use deno_ast::swc::common::{FilePathMapping, SourceFile, SourceMap};
use deno_core::{ModuleCode, ModuleLoader, ModuleSource, ModuleSourceFuture, ModuleSpecifier, ModuleType, futures::FutureExt, Resource};
use tonic::codegen::Body;
use tracing::event;
use crate::error::Error;
use crate::runtime::emit::CompileMessageEmitter;

use super::emit::{Emitter, Level};

const TASK_WARN_MIN_SIZE: usize = 12;
const HINT_WARN_MIN_SIZE: usize = 12;

#[derive(Debug)]
pub struct TypeAliasMap(HashMap<Id, TsTypeAliasDecl>);

impl TypeAliasMap {
    pub fn get_decl(&self, id: &Id) -> Option<&TsTypeAliasDecl> {
        self.0.get(id)
    }
}

#[derive(Debug)]
pub struct TypeAliasVisitor(HashMap<Id, TsTypeAliasDecl>);

impl TypeAliasVisitor {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn run(mut self, module: &Module) -> TypeAliasMap {
        self.visit_module(module);
        TypeAliasMap(self.0)
    }
}

impl Visit for TypeAliasVisitor {
    fn visit_ts_type_alias_decl(&mut self, n: &TsTypeAliasDecl) {
        self.0.insert(n.id.to_id(), n.clone());
    }
}

pub struct MatchCallStrLit(&'static str, Option<Str>);

impl MatchCallStrLit {
    fn new(callee_ident: &'static str) -> Self {
        Self(callee_ident, None)
    }

    pub fn match_decorator(&self, decorator: &Decorator) -> Option<Str> {
        self.match_call_expr(decorator.expr.as_call()?)
    }

    pub fn first_match_decorator(&self, decorators: &[Decorator]) -> Option<Str> {
        decorators.iter().filter_map(|d| self.match_decorator(d)).next()
    }

    pub fn match_call_expr(&self, n: &CallExpr) -> Option<Str> {
        if n.callee.as_expr()?.as_ident()?.clone().without_loc().as_ref() == self.0 {
            match n.args.get(0)?.expr.as_lit()? {
                Lit::Str(s) => Some(s.to_owned()),
                _ => None
            }
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct Task {
    name: Ident,
    description: Str,
    output: TsTypeRef,
    methods: HashMap<Ident, TaskMethod>,
}

impl Task {
    fn get_type_param_as_ts_type_ref(n: &Class) -> Option<TsTypeRef> {
        n.super_type_params.as_ref()?.params.get(0)?.as_ts_type_ref().cloned()
    }

    fn match_class_decl(n: &ClassDecl) -> Option<Self> {
        let description = MatchCallStrLit::new("task").first_match_decorator(&n.class.decorators)?;

        let name = n.ident.clone();

        let output = Self::get_type_param_as_ts_type_ref(&n.class)
            .expect("`@task` decorated classes must extend `Task<O>`");

        let methods = n.class.body.iter().filter_map(|member| member.as_method())
            .filter_map(|method| {
                Some((method.key.as_ident()?.clone(), TaskMethod::from_method(method)?))
            })
            .collect();

        Some(Self {
            name,
            description,
            output,
            methods,
        })
    }
}

// TODO(brokad): this is unhygienic
#[derive(Debug)]
pub struct TaskMap(HashMap<String, Task>);

impl Default for TaskMap {
    fn default() -> Self {
        Self(HashMap::new())
    }
}

pub struct TaskMapResource(RefCell<TaskMap>);

impl TaskMapResource {
    pub fn new() -> Self {
        Self(RefCell::new(TaskMap::default()))
    }

    pub fn get_task(&self, task_id: &String) -> Option<Task> {
        self.0.borrow().0.get(task_id).cloned()
    }
}

impl Resource for TaskMapResource {}

pub struct TaskVisitor<'s, E> {
    type_alias: &'s TypeAliasMap,
    visited_tasks: HashMap<String, Task>,
    emitter: E,
}

impl<'s, E> TaskVisitor<'s, E> {
    pub fn new(type_alias: &'s TypeAliasMap, emitter: E) -> Self {
        Self {
            type_alias,
            visited_tasks: HashMap::new(),
            emitter,
        }
    }
}

impl<'s, E> TaskVisitor<'s, E>
    where
        E: Emitter
{
    pub fn run(mut self, n: &mut Module) -> TaskMap {
        self.visit_mut_module(n);
        TaskMap(self.visited_tasks)
    }
}

impl<'s, E> VisitMut for TaskVisitor<'s, E>
    where
        E: Emitter
{
    fn visit_mut_class_decl(&mut self, n: &mut ClassDecl) {
        if let Some(task) = Task::match_class_decl(n) {
            // short description for @task
            if task.description.value.split(" ").count() < TASK_WARN_MIN_SIZE {
                self.emitter.emit(&task.description.span, Level::WARN, "the `@task` description is short: try expanding on what the task does");
            }

            for (_, TaskMethod { hint, .. }) in &task.methods {
                // short description for @hint (method)
                if hint.value.split(" ").count() < HINT_WARN_MIN_SIZE {
                    self.emitter.emit(&hint.span, Level::WARN, "the `@hint` description is short: try expanding on how the method is used");
                }
            }

            if task.output.type_name.as_ident().map(Ident::to_id).and_then(|id| self.type_alias.get_decl(&id)).is_none() {
                self.emitter.emit(&task.output.span, Level::WARN, "unable to resolve the output type of this task: try simplifying the type hints");
            }

            // TODO(brokad): this is unhygienic
            self.visited_tasks.insert(n.ident.to_id().0.to_string(), task);
        }
    }
}

#[derive(Debug, Clone)]
pub struct TaskMethod {
    params: Vec<Param>,
    return_type: Option<TsTypeRef>,
    hint: Str,
}

impl TaskMethod {
    pub fn from_method(n: &ClassMethod) -> Option<Self> {
        let hint = MatchCallStrLit::new("hint").first_match_decorator(&n.function.decorators)?;

        let return_type = n.function.return_type.as_ref()
            .and_then(|t| t.type_ann.as_ts_type_ref())
            .cloned();

        Some(Self {
            params: vec![],  // TODO(brokad)
            return_type,
            hint,
        })
    }
}

pub struct Compiler {
    task_map: Rc<TaskMapResource>,
    source_map: Arc<SourceMap>,
    swc_compiler: swc::Compiler
}

pub struct RawModuleSource {
    module_specifier: ModuleSpecifier,
    media_type: MediaType,
    source_file: Arc<SourceFile>,
}

impl Compiler {
    pub fn into_module_loader(self) -> PassthruModuleLoader {
        PassthruModuleLoader(Rc::new(self))
    }

    pub fn new() -> Self {
        let source_map = Arc::new(SourceMap::new(FilePathMapping::empty()));
        Self {
            task_map: todo!(),
            swc_compiler: swc::Compiler::new(source_map.clone()),
            source_map
        }
    }

    #[tracing::instrument]
    pub fn fetch_source(&self, module_specifier: ModuleSpecifier) -> Result<RawModuleSource, Error> {
        let (media_type, source_file) = if module_specifier.scheme() == "file" {
            let path = module_specifier.to_file_path().unwrap();
            let media_type = MediaType::from_path(&path);
            let source_file = self.source_map.load_file(&path).unwrap();
            (media_type, source_file)
        } else {
            todo!("imports other than local files are not supported yet: ${module_specifier:?}")
        };

        Ok(RawModuleSource {
            module_specifier,
            media_type,
            source_file
        })
    }

    pub fn module_type_from_media_type(media_type: &MediaType) -> ModuleType {
        match media_type {
            MediaType::Json => ModuleType::Json,
            _ => ModuleType::JavaScript
        }
    }

    pub fn should_process_from_media_type(media_type: &MediaType) -> bool {
        match media_type {
            MediaType::TypeScript
            | MediaType::Mts
            | MediaType::Cts
            | MediaType::Dts
            | MediaType::Dmts
            | MediaType::Dcts
            | MediaType::Tsx
            | MediaType::Jsx => true,
            _ => false
        }
    }

    #[tracing::instrument]
    pub fn load_module(
        &self,
        RawModuleSource {
            module_specifier,
            media_type,
            source_file
        }: RawModuleSource) -> Result<ModuleSource, Error> {
        let module_type = Self::module_type_from_media_type(&media_type);

        if Self::should_process_from_media_type(&media_type) {
            let parsed = deno_ast::parse_module_with_post_process(ParseParams {
                specifier: module_specifier.to_string(),
                text_info: SourceTextInfo::new(source_file.src.into()),
                media_type,
                capture_tokens: false,
                scope_analysis: true,
                maybe_syntax: None,
            }, |mut module| {
                let type_alias_map = TypeAliasVisitor::new().run(&module);

                let emitter = CompileMessageEmitter::new(
                    module_specifier.as_str(),
                    source_text.as_str(),
                    stderr()
                ).unwrap();

                let visited_tasks = TaskVisitor::new(
                    &type_alias_map,
                    emitter,
                ).run(&mut module);

                // TODO(brokad): this is unhygienic
                self.task_map.0.borrow_mut().0.extend(visited_tasks.0.into_iter());

                module
            }).unwrap();

            source_text = parsed.transpile(&Default::default())?.text;
        }

        Ok(ModuleSource::new(
            module_type,
            ModuleCode::from(source_text),
            &module_specifier,
        ))
    }

    pub async fn load_module_impl(self: Rc<Self>, module_specifier: ModuleSpecifier) -> Result<ModuleSource, Error> {
        let source = self.fetch_source(module_specifier).await?;
        self.load_module(source).await
    }
}

pub struct PassthruModuleLoader(pub Rc<Compiler>);

impl ModuleLoader for PassthruModuleLoader {
    fn resolve(
        &self,
        specifier: &str,
        referrer: &str,
        _kind: deno_core::ResolutionKind,
    ) -> Result<ModuleSpecifier, deno_core::error::AnyError> {
        deno_core::resolve_import(specifier, referrer).map_err(|e| e.into())
    }

    fn load(
        &self,
        module_specifier: &ModuleSpecifier,
        _maybe_referrer: Option<&ModuleSpecifier>,
        _is_dyn_import: bool,
    ) -> std::pin::Pin<Box<ModuleSourceFuture>> {
        let module_specifier = module_specifier.clone();
        self.0.clone().load_module_impl(module_specifier).boxed_local()
    }
}