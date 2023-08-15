use std::ops::Deref;

use crate::CanPush;
use crate::{ast, visit};

#[derive(Debug)]
pub struct FnDecl(pub ast::FnDecl);

impl Deref for FnDecl {
    type Target = ast::FnDecl;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub struct FnDeclVisitor<'m, C>(pub &'m mut C);

impl<'m, C> visit::Visit for FnDeclVisitor<'m, C>
    where
        C: CanPush<FnDecl>
{
    fn visit_fn_decl(&mut self, n: &ast::FnDecl) {
        let mut fn_decl = n.clone();
        fn_decl.function.body = None;
        fn_decl.function.decorators.clear();
        self.0.push(FnDecl(fn_decl));
    }
}

