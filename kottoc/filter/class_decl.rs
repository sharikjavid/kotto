use std::collections::HashMap;
use std::ops::Deref;

use crate::CanPush;
use crate::{ast, visit};

use crate::filter::{TypeRef, TypeRefVisitor};

#[derive(Debug)]
pub struct ClassDecl {
    pub class_decl: ast::ClassDecl,
    pub class_members: HashMap<ast::PropName, ClassMember>,
}

#[derive(Debug)]
pub enum ClassMember {
    Method(ClassMethod),
    Prop(ClassProp),
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
pub struct ClassProp(pub ast::ClassProp);

impl Deref for ClassProp {
    type Target = ast::ClassProp;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub struct ClassDeclVisitor<'m, C>(pub &'m mut C);

impl<'m, C> visit::Visit for ClassDeclVisitor<'m, C>
    where
        C: CanPush<ClassDecl>
{
    fn visit_class_decl(&mut self, n: &ast::ClassDecl) {
        let mut class_members = HashMap::new();

        for class_member in &n.class.body {
            match class_member {
                ast::ClassMember::Method(class_method) => {
                    let mut class_method = class_method.clone();

                    let mut type_refs = Vec::new();
                    TypeRefVisitor(&mut type_refs).visit_class_method(&class_method);

                    // Trim the unnecessary stuff
                    class_method.function.body = None;
                    class_method.function.decorators.clear();

                    class_members.insert(
                        class_method.key.clone(),
                        ClassMember::Method(ClassMethod {
                            class_method,
                            type_refs
                        })
                    );
                }
                ast::ClassMember::ClassProp(class_prop) => {
                    let mut class_prop = class_prop.clone();
                    class_prop.value.take();
                    class_members.insert(
                        class_prop.key.clone(),
                        ClassMember::Prop(ClassProp(class_prop))
                    );
                }
                _ => {}
            }
        }

        let mut class_decl = n.clone();
        class_decl.class.decorators.clear();
        class_decl.class.body.clear();

        self.0.push(ClassDecl {
            class_decl,
            class_members,
        });
    }
}
