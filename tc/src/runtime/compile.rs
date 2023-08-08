use std::cell::{RefCell, RefMut};
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::rc::Rc;
use deno_ast::{MediaType, ParseParams, SourceTextInfo};
use deno_ast::swc::visit::{VisitMut, Visit};
use deno_ast::swc::ast::{self, CallExpr, ClassDecl, Ident, Str, TsTypeAliasDecl, Lit, Decorator, Class, ClassMethod, Module, Id, TsTypeAnn, Pat, TsType};
use deno_ast::swc::codegen::{Node, self};
use deno_ast::swc::codegen::text_writer::WriteJs;
use deno_ast::swc::common::{FilePathMapping, SourceFile, SourceMap, Span};
use deno_ast::swc::visit;
use deno_core::{ModuleCode, ModuleLoader, ModuleSource, ModuleSourceFuture, ModuleSpecifier, ModuleType, futures::FutureExt, Resource};
use deno_core::error::AnyError;
use tonic::codegen::Body;
use crate::error::Error;

use super::emit::Emitter;

const TASK_WARN_MIN_SIZE: usize = 12;
const HINT_WARN_MIN_SIZE: usize = 12;

#[derive(Debug)]
pub struct TypeAliasMap(HashMap<Id, TsTypeAliasDecl>);

impl Default for TypeAliasMap {
    fn default() -> Self {
        Self(HashMap::default())
    }
}

impl TypeAliasMap {
    pub(crate) fn ts_type_to_id(ty: &TsType) -> Option<Id> {
        match ty {
            TsType::TsTypeRef(ast::TsTypeRef { type_name, .. }) => {
                let ident = match type_name {
                    ast::TsEntityName::TsQualifiedName(qual) => &qual.right,
                    ast::TsEntityName::Ident(ident) => ident
                };
                Some(ident.to_id())
            }
            _ => None
        }
    }

    pub fn get_decl(&self, id: &Id) -> Option<&TsTypeAliasDecl> {
        self.0.get(id)
    }

    pub fn insert_decl(&mut self, id: Id, decl: TsTypeAliasDecl) -> Option<TsTypeAliasDecl> {
        self.0.insert(id, decl)
    }
}

#[derive(Debug)]
pub struct TypeAliasVisitor<'m>(&'m mut TypeAliasMap);

impl<'m> TypeAliasVisitor<'m> {
    pub fn new(map: &'m mut TypeAliasMap) -> Self {
        Self(map)
    }

    pub fn run(mut self, module: &Module) {
        self.visit_module(module);
    }
}

impl<'m> Visit for TypeAliasVisitor<'m> {
    fn visit_ts_type_alias_decl(&mut self, n: &TsTypeAliasDecl) {
        self.0.insert_decl(n.id.to_id(), n.clone());
    }
}

struct TsTypeClosureVisitor<'m>(&'m mut Vec<Id>);

impl<'m> Visit for TsTypeClosureVisitor<'m> {
    fn visit_ts_type(&mut self, ts_type: &TsType) {
        if let Some(id) = TypeAliasMap::ts_type_to_id(ts_type) {
            self.0.push(id);
        }
        visit::visit_ts_type(self, ts_type)
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

// TODO(brokad): Should `Ident` be `Id`?
#[derive(Debug, Clone)]
pub struct Task {
    name: Ident,
    description: Str,
    output: TsType,
    methods: HashMap<Ident, TaskMethod>,
}

impl Task {
    fn get_type_param_as_ts_type(n: &Class) -> Option<TsType> {
        n.super_type_params.as_ref()?.params.get(0).cloned().map(|t| *t)
    }

    fn match_class_decl(n: &ClassDecl) -> Option<Self> {
        let description = MatchCallStrLit::new("task").first_match_decorator(&n.class.decorators)?;

        let name = n.ident.clone();

        let output = Self::get_type_param_as_ts_type(&n.class)
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
        Self(HashMap::default())
    }
}

impl TaskMap {
    pub fn get(&self, task_name: &str) -> Option<&Task> {
        self.0.get(task_name)
    }

    pub fn insert(&mut self, task_name: String, task: Task) -> Option<Task> {
        self.0.insert(task_name, task)
    }
}

pub struct TaskVisitor<'m> {
    task_map: &'m mut TaskMap
}

impl<'m> TaskVisitor<'m> {
    pub fn new(task_map: &'m mut TaskMap) -> Self {
        Self {
            task_map
        }
    }
}

impl<'m> TaskVisitor<'m> {
    pub fn run(mut self, n: &Module) {
        self.visit_module(n);
    }
}

impl<'m> Visit for TaskVisitor<'m> {
    fn visit_class_decl(&mut self, n: &ClassDecl) {
        if let Some(task) = Task::match_class_decl(n) {
            /*
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
             */

            // TODO(brokad): this is unhygienic
            self.task_map.insert(n.ident.to_id().0.to_string(), task);
        }
    }
}

#[derive(Debug, Clone)]
pub struct TaskMethod {
    params: Vec<TsType>,
    return_type: Option<TsType>,
    hint: Str,
}

impl TaskMethod {
    pub fn type_ann_from_pat(pat: &Pat) -> Option<&TsType> {
        match pat {
            Pat::Ident(ast::BindingIdent { type_ann, .. }) |
            Pat::Object(ast::ObjectPat { type_ann, .. }) |
            Pat::Array(ast::ArrayPat { type_ann, .. }) |
            Pat::Assign(ast::AssignPat { type_ann, .. }) |
            Pat::Rest(ast::RestPat { type_ann, .. }) => {
                type_ann.as_ref().map(|d| d.type_ann.as_ref())
            }
            _ => None
        }
    }

    #[tracing::instrument]
    pub fn from_method(n: &ClassMethod) -> Option<Self> {
        let hint = MatchCallStrLit::new("hint").first_match_decorator(&n.function.decorators)?;

        let params = n.function.params.iter().map(|param| {
            if let Some(type_ann) = Self::type_ann_from_pat(&param.pat) {
                type_ann.clone()
            }
            else {
                todo!("unsupported function parameters: try simplifying the method's signature")
            }
        }).collect();

        let return_type = n.function.return_type.as_ref()
            .map(|t| t.type_ann.as_ref())
            .cloned();

        Some(Self {
            params,
            return_type,
            hint
        })
    }
}

pub struct Compiler {
    type_map: TypeAliasMap,
    task_map: TaskMap,
    source_map: Rc<SourceMap>
}

pub struct CompilerResource(RefCell<Compiler>);

impl CompilerResource {
    pub fn borrow_mut(&self) -> RefMut<'_, Compiler> {
        self.0.borrow_mut()
    }

    pub async fn load_module_impl(self: Rc<Self>, module_specifier: ModuleSpecifier) -> Result<ModuleSource, AnyError> {
        let mut locked = self.borrow_mut();
        let source = locked.fetch_source(module_specifier).unwrap();
        let module_source = locked.load_module(source).unwrap();
        Ok(module_source)
    }
}

impl Resource for CompilerResource {}

pub struct RawModuleSource {
    module_specifier: ModuleSpecifier,
    media_type: MediaType,
    source_file: Rc<SourceFile>,
}

impl Compiler {
    pub fn into_resource(self) -> CompilerResource {
        CompilerResource(RefCell::new(self))
    }

    pub fn new() -> Self {
        let source_map = Rc::new(SourceMap::new(FilePathMapping::empty()));
        Self {
            type_map: TypeAliasMap::default(),
            task_map: TaskMap::default(),
            source_map
        }
    }

    // TODO(brokad): Very hacky
    pub fn print_task_context<W: Write>(&self, task_name: &str, mut writer: W) -> Result<(), Error> {
        let task = self.task_map.get(&task_name).unwrap();

        let source_map = self.source_map.clone();
        let type_map = &self.type_map;

        let mut emitter = codegen::Emitter {
            cfg: codegen::Config::default(),
            cm: source_map.clone(),
            comments: None,
            wr: codegen::text_writer::JsWriter::new(source_map.clone(), "\n", writer, None)
        };

        let mut to_visit = Vec::new();

        to_visit.push(task.output.clone());

        for task_method in task.methods.values() {
            if let Some(return_type) = task_method.return_type.as_ref() {
                to_visit.push(return_type.clone());
            }

            for param in &task_method.params {
                to_visit.push(param.clone());
            }
        }

        let mut type_closure = HashSet::new();
        while let Some(next) = to_visit.pop() {
            let mut refs = Vec::new();
            TsTypeClosureVisitor(&mut refs)
                .visit_ts_type(&next);
            for type_ref in refs {
                if !type_closure.contains(&type_ref) {
                    if let Some(TsTypeAliasDecl { type_ann, .. }) = type_map.get_decl(&type_ref) {
                        to_visit.push(type_ann.as_ref().clone());
                    }
                    type_closure.insert(type_ref);
                }
            }
        }

        for type_ref in type_closure {
            if let Some(type_decl) = type_map.get_decl(&type_ref) {
                type_decl.emit_with(&mut emitter).unwrap();
                emitter.wr.write_line().unwrap();
            }
        }

        for (task_ident, task_method) in task.methods.iter() {
            emitter.wr.write_line().unwrap();
            emitter.wr.write_str(&format!("\n// hint: {}", task_method.hint.value)).unwrap();
            emitter.wr.write_line().unwrap();
            let param_ident = Ident::new("_".into(), Span::default());
            let return_type = task_method.return_type.as_ref().map(|rt| {
                Box::new(ast::TsTypeAnn {
                    span: Span::default(),
                    type_ann: Box::new(rt.clone())
                })
            });
            let params: Vec<_> = task_method.params.iter().map(|param_type| {
                let binding = ast::BindingIdent {
                    id: param_ident.clone(),
                    type_ann: Some(Box::new(ast::TsTypeAnn {
                        span: Span::default(),
                        type_ann: Box::new(param_type.clone())
                    }))
                };
                ast::Param {
                    span: Span::default(),
                    decorators: Vec::new(),
                    pat: binding.into()
                }
            }).collect();
            let function = ast::FnDecl {
                ident: task_ident.clone(),
                declare: false,
                function: Box::new(ast::Function {
                    params,
                    decorators: Vec::new(),
                    span: Span::default(),
                    body: None,
                    is_generator: false,
                    is_async: false,
                    type_params: None,
                    return_type
                })
            };
            function.emit_with(&mut emitter);
        }

        Ok(())
    }

    pub fn get_task_description(&self, task_name: &str) -> Result<String, Error> {
        Ok(self.task_map.get(task_name).unwrap().description.value.to_string())
    }

    #[tracing::instrument(skip(self))]
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

    #[tracing::instrument(skip(self))]
    pub fn load_module(
        &mut self,
        RawModuleSource {
            module_specifier,
            media_type,
            source_file
        }: RawModuleSource) -> Result<ModuleSource, Error> {
        let module_type = Self::module_type_from_media_type(&media_type);

        let source_text = if Self::should_process_from_media_type(&media_type) {
            let parsed = deno_ast::parse_module(ParseParams {
                specifier: module_specifier.to_string(),
                text_info: SourceTextInfo::from_string(source_file.src.to_string()),
                media_type,
                capture_tokens: false,
                scope_analysis: true,
                maybe_syntax: None,
            }).unwrap();

            let module = parsed.module();

            TypeAliasVisitor::new(&mut self.type_map)
                .run(module);

            TaskVisitor::new(&mut self.task_map)
                .run(module);

            parsed.transpile(&Default::default())?.text
        } else {
            source_file.src.to_string()
        };

        Ok(ModuleSource::new(
            module_type,
            ModuleCode::from(source_text),
            &module_specifier,
        ))
    }
}

pub struct PassthruModuleLoader(Rc<CompilerResource>);

impl PassthruModuleLoader {
    pub fn from_compiler_resource(compiler_resource: Rc<CompilerResource>) -> Self {
        Self(compiler_resource)
    }
}

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