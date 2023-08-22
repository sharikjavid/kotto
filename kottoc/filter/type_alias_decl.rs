use std::collections::HashMap;
use std::ops::Deref;

use crate::CanPush;
use crate::{ast, visit};

impl CanPush<TypeAliasDecl> for HashMap<ast::Id, TypeAliasDecl> {
    fn push(&mut self, item: TypeAliasDecl) {
        HashMap::insert(self, item.id.to_id(), item);
    }
}

#[derive(Debug)]
pub struct TypeAliasDecl(pub ast::TsTypeAliasDecl);

impl Deref for TypeAliasDecl {
    type Target = ast::TsTypeAliasDecl;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub struct TypeAliasVisitor<'m, C>(pub &'m mut C);

impl<'m, C> visit::Visit for TypeAliasVisitor<'m, C>
where
    C: CanPush<TypeAliasDecl>
{
    fn visit_ts_type_alias_decl(&mut self, n: &ast::TsTypeAliasDecl) {
        self.0.push(TypeAliasDecl(n.clone()));
    }
}

