use std::borrow::Cow;
use std::error::Error as StdError;
use std::ops::Deref;
use std::rc::Rc;
use deno_ast::ModuleSpecifier;
use serde::{Serialize, Deserialize};

use deno_core::{op, v8, Op, JsRuntime, ModuleId, ResourceId, OpState, AsyncRefCell, Resource, AsyncMutFuture, AsyncRefFuture, RcRef, Extension};
use deno_core::error::AnyError;
use futures::{SinkExt, TryFutureExt};

mod compile;
mod emit;
pub mod ext;
use emit::Emitter;

use crate::error::Error;
use crate::runtime::compile::{Compiler, CompilerResource, PassthruModuleLoader};

const COMPILER_RID: ResourceId = 0;

pub fn build_extensions() -> Result<Vec<Extension>, Error> {
    let compiler_resource = Compiler::new().into_resource();
    let extensions = vec![
        Extension {
            ops: Cow::from(vec![]),
            op_state_fn: Some(Box::new(|op_state| {
                assert_eq!(op_state.resource_table.add(compiler_resource), COMPILER_RID);
            })),
            ..Default::default()
        }
    ];
    Ok(extensions)
}

#[op]
fn extensions_are_enabled() -> bool {
    true
}

#[op]
fn get_method_decl(
    state: &mut OpState,
    loader_id: String,
    property_key: String
) -> Result<String, AnyError> {
    let compiler = state.resource_table.get::<CompilerResource>(COMPILER_RID).unwrap();
    let method_decl = compiler.borrow_mut().get_method_decl(&loader_id, &property_key).unwrap();
    Ok(method_decl)
}

#[op]
fn get_type_context(
    state: &mut OpState,
    loader_id: String
) -> Result<Vec<String>, AnyError> {
    let compiler = state.resource_table.get::<CompilerResource>(COMPILER_RID).unwrap();
    let type_context = compiler.borrow_mut().get_type_context(&loader_id).unwrap();
    Ok(type_context)
}