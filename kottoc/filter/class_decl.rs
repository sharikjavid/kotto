use std::collections::HashMap;
use std::ops::Deref;

use crate::CanPush;
use crate::{ast, visit};

use crate::filter::{TypeRef, TypeRefVisitor};

#[derive(Debug)]
pub struct ClassDecl {
    pub class_decl: ast::ClassDecl,
    pub class_methods: HashMap<ast::PropName, ClassMethod>,
}

#[derive(Debug)]
pub struct ClassMethod {
    pub class_method: ast::ClassMethod,
    pub type_refs: Vec<TypeRef>
}

impl Deref for ClassMethod {
    type Target = ast::ClassMethod;

    fn deref(&self) -> &Self::Target {
        &self.class_method
    }
}

#[derive(Debug)]
pub struct ClassDeclVisitor<'m, C>(pub &'m mut C);

impl<'m, C> visit::Visit for ClassDeclVisitor<'m, C>
    where
        C: CanPush<ClassDecl>
{
    fn visit_class_decl(&mut self, n: &ast::ClassDecl) {
        let class_methods = n.class.body
            .iter()
            .filter_map(|cm| {
                match cm {
                    ast::ClassMember::Method(method) => Some(method),
                    _ => None
                }
            })
            .cloned()
            .map(|mut class_method| {
                let mut type_refs = Vec::new();
                TypeRefVisitor(&mut type_refs).visit_class_method(&class_method);

                // Trim the unnecessary stuff
                class_method.function.body = None;
                class_method.function.decorators.clear();
                (
                    class_method.key.clone(),
                    ClassMethod {
                        class_method,
                        type_refs
                    }
                )
            })
            .collect();

        let mut class_decl = n.clone();
        class_decl.class.decorators.clear();
        class_decl.class.body.clear();

        self.0.push(ClassDecl {
            class_decl,
            class_methods,
        });
    }
}
