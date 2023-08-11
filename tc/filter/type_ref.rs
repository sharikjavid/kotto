use std::ops::Deref;

use crate::CanPush;
use crate::{ast, visit};

#[derive(Debug, Clone)]
pub struct TypeRef(pub ast::TsTypeRef);

impl Deref for TypeRef {
    type Target = ast::TsTypeRef;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub struct TypeRefVisitor<'m, C>(pub &'m mut C);

impl<'m, C> visit::Visit for TypeRefVisitor<'m, C>
    where
        C: CanPush<TypeRef>
{
    fn visit_ts_type_ref(&mut self, n: &ast::TsTypeRef) {
        self.0.push(TypeRef(n.clone()));
        visit::visit_ts_type_ref(self, n)
    }
}

