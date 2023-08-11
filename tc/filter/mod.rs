use crate::ast;
use crate::visit;
use crate::AnyError;

use visit::Visit;

mod type_alias_decl;

pub use type_alias_decl::{TypeAliasDecl, TypeAliasVisitor};

#[derive(Debug, Default)]
pub struct FilteredModule {
    pub type_alias_decls: Vec<TypeAliasDecl>,
}

#[derive(Debug)]
pub struct FilterParams {
    enable_type_alias_decls: bool
}

impl Default for FilterParams {
    fn default() -> Self {
        Self {
            enable_type_alias_decls: true
        }
    }
}

pub async fn run_filters(params: FilterParams, module: &ast::Module) -> Result<FilteredModule, AnyError> {
    let mut result = FilteredModule::default();

    if params.enable_type_alias_decls {
        TypeAliasVisitor(&mut result.type_alias_decls).visit_module(module);
    }

    Ok(result)
}