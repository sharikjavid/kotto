use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use crate::ast;
use crate::visit;
use crate::AnyError;
use crate::CanPush;

use visit::Visit;

mod type_alias_decl;
mod class_decl;
mod type_ref;

pub use type_alias_decl::{TypeAliasDecl, TypeAliasVisitor};
pub use class_decl::{ClassDecl, ClassDeclVisitor};
pub use type_ref::{TypeRef, TypeRefVisitor};

#[derive(Debug, Default)]
pub struct FilteredModule {
    pub type_alias_decls: HashMap<ast::Id, TypeAliasDecl>,
    pub class_decls: Vec<ClassDecl>
}

impl FilteredModule {
    pub fn find_closure_of_type_refs<'i, I: 'i>(&self, refs: I) -> HashSet<ast::Id>
    where
        I: IntoIterator<Item = &'i TypeRef>
    {
        let mut closure = HashSet::new();
        let mut to_visit: Vec<TypeRef> = refs.into_iter().cloned().collect();
        while let Some(next) = to_visit.pop() {
            if let Some(next_id) = next.type_name.clone().ident().as_ref().map(ast::Ident::to_id) {
                if !closure.insert(next_id.clone()) {
                    if let Some(next_decl) = self.type_alias_decls.get(&next_id) {
                        TypeRefVisitor(&mut to_visit).visit_ts_type_alias_decl(next_decl);
                    }
                }
            }
        }
        closure
    }
}

impl<T> CanPush<T> for Vec<T> {
    fn push(&mut self, item: T) {
        Vec::push(self, item)
    }
}

impl<T: Hash + Eq> CanPush<T> for HashSet<T> {
    fn push(&mut self, item: T) {
        HashSet::insert(self, item);
    }
}

#[derive(Debug)]
pub struct FilterParams {
    enable_type_alias_decls: bool,
    enable_class_decls: bool
}

impl Default for FilterParams {
    fn default() -> Self {
        Self {
            enable_type_alias_decls: true,
            enable_class_decls: true
        }
    }
}

pub async fn run_filters(params: FilterParams, module: &ast::Module) -> Result<FilteredModule, AnyError> {
    let mut result = FilteredModule::default();

    if params.enable_type_alias_decls {
        TypeAliasVisitor(&mut result.type_alias_decls).visit_module(module);
    }

    if params.enable_class_decls {
        ClassDeclVisitor(&mut result.class_decls).visit_module(module);
    }

    Ok(result)
}