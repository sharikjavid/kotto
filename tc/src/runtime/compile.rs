use std::collections::HashMap;
use deno_ast::swc::{self, visit::{VisitMut, Visit}};
use deno_ast::swc::ast::{CallExpr, ClassDecl, Ident, Param, Str, TsTypeAliasDecl, TsTypeAnn, TsTypeRef, Lit, Decorator, Class, ClassMethod, Module, Id};

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

#[derive(Debug)]
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
            methods
        })
    }
}

#[derive(Debug)]
pub struct TaskMap(HashMap<Id, Task>);

pub struct TaskVisitor<'s, E> {
    type_alias: &'s TypeAliasMap,
    visited_tasks: HashMap<Id, Task>,
    emitter: E
}

impl<'s, E> TaskVisitor<'s, E> {
    pub fn new(type_alias: &'s TypeAliasMap, emitter: E) -> Self {
        Self {
            type_alias,
            visited_tasks: HashMap::new(),
            emitter
        }
    }
}

impl<'s, E> TaskVisitor<'s, E>
    where
        E: Emitter
{
    pub fn run(mut self, n: &Module) -> TaskMap {
        self.visit_module(n);
        TaskMap(self.visited_tasks)
    }
}

impl<'s, E> Visit for TaskVisitor<'s, E>
    where
        E: Emitter
{
    fn visit_class_decl(&mut self, n: &ClassDecl) {
        if let Some(task) = Task::match_class_decl(n) {

            // short description
            if task.description.value.split(" ").count() < TASK_WARN_MIN_SIZE {
                self.emitter.emit(&task.description.span, Level::WARN, "the `@task` description is short: try expanding on what the task does");
            }

            for (_, TaskMethod { hint, .. }) in &task.methods {
                if hint.value.split(" ").count() < HINT_WARN_MIN_SIZE {
                    self.emitter.emit(&hint.span, Level::WARN, "the `@hint` description is short: try expanding on how the method is used");
                }
            }

            if task.output.type_name.as_ident().map(Ident::to_id).and_then(|id| self.type_alias.get_decl(&id)).is_none() {
                self.emitter.emit(&task.output.span, Level::WARN, "unable to resolve the output type of this task: try moving the type declaration to the same source file");
            }

            self.visited_tasks.insert(n.ident.to_id(), task);
        }
    }
}

#[derive(Debug)]
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