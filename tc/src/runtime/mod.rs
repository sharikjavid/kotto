use std::borrow::Cow;
use std::cell::{RefCell, RefMut};
use std::error::Error as StdError;
use std::path::PathBuf;
use std::collections::HashMap;
use std::io::{BufWriter, Cursor};
use std::ops::Deref;
use std::rc::Rc;
use deno_ast::ModuleSpecifier;
use serde::{Serialize, Deserialize};

use deno_core::{op, v8, Op, JsRuntime, ModuleId, ResourceId, OpState, AsyncRefCell, Resource, AsyncMutFuture, AsyncRefFuture, RcRef, Extension};
use deno_core::error::AnyError;
use futures::{SinkExt, TryFutureExt};
use crate::client::{Client, Session};

mod compile;
mod emit;
use emit::Emitter;

use crate::error::Error;
use crate::proto::{MessageBuilder, MessageCode};
use crate::proto::trackway::MessageType;
use crate::runtime::compile::{Compiler, CompilerResource, PassthruModuleLoader};

const CLIENT_RID: ResourceId = 0;
const COMPILER_RID: ResourceId = 1;

#[derive(Serialize)]
pub struct NewInstanceMessage {
    task_name: String,
    task_description: String,
    task_context: String
}

#[derive(Deserialize)]
pub struct EvaluateScriptMessage {
    source_code: String
}

#[derive(Serialize)]
pub struct JsonValueMessage {
    json_value: serde_json::Value
}

pub struct RuntimeOptions {
    client: Client,
    source: PathBuf,
    emitter: Box<dyn Emitter>
}

pub struct Runtime {
    rt: JsRuntime
}

impl Runtime {
    #[tracing::instrument(skip(client))]
    pub fn new_with_client(client: Client) -> Self {
        let compiler_resource = Rc::new(Compiler::new().into_resource());
        let module_loader = PassthruModuleLoader::from_compiler_resource(compiler_resource.clone());

        let rt = JsRuntime::new(deno_core::RuntimeOptions {
            module_loader: Some(Rc::new(module_loader)),
            extensions: vec![
                Extension {
                    ops: Cow::from(vec![
                        task_register_instance::DECL,
                        task_poll_instance::DECL,
                        task_run_with_side_effects::DECL,
                        task_cancel_instance::DECL
                    ]),
                    op_state_fn: Some(Box::new(|op_state| {
                        let client_resource = ClientResource::from_client(client);
                        assert_eq!(op_state.resource_table.add(client_resource), CLIENT_RID);
                        assert_eq!(op_state.resource_table.add_rc(compiler_resource), COMPILER_RID);
                    })),
                    ..Default::default()
                }
            ],
            ..Default::default()
        });

        Self {
            rt
        }
    }

    #[tracing::instrument(skip(self))]
    pub async fn load_main_module(&mut self, module_specifier: &ModuleSpecifier) -> Result<ModuleId, Error> {
        self.rt.load_main_module(module_specifier, None).await.map_err(Into::into)
    }

    #[tracing::instrument(skip(self))]
    pub async fn evaluate_module(&mut self, module_id: ModuleId) -> Result<(), Error> {
        let eval_task = self.rt.mod_evaluate(module_id);
        self.rt.run_event_loop(false).await?;
        Ok(eval_task.await.unwrap()?)
    }
}

pub struct ClientResource {
    inner: AsyncRefCell<Client>
}

impl ClientResource {
    pub fn from_client(client: Client) -> Self {
        Self {
            inner: AsyncRefCell::new(client)
        }
    }
}

impl Resource for ClientResource {}

impl ClientResource {
    pub fn borrow_mut(self: Rc<Self>) -> AsyncMutFuture<Client> {
        RcRef::map(self, |this| &this.inner).borrow_mut()
    }
}

pub type SlotId = ResourceId;

pub enum Slot {
    Source(String),
    Ok(serde_json::Value)
}

pub struct Instance {
    session: Session,
    slots: HashMap<SlotId, Slot>,
    next_slot: SlotId
}

impl Instance {
    pub fn into_resource(self) -> InstanceResource {
        InstanceResource(RefCell::new(self))
    }

    pub fn from_session(session: Session) -> Self {
        Self {
            session,
            slots: HashMap::new(),
            next_slot: 0
        }
    }

    // TODO(brokad): make sure we only receive the right kind of messages here (control, call types)
    pub async fn poll(&mut self, slot_id: Option<SlotId>) -> Result<Option<SlotId>, Error> {
        if let Some(slot_id) = slot_id {
            match self.slots.remove(&slot_id) {
                Some(Slot::Ok(value)) => MessageBuilder::new()
                    .message_type(MessageType::MessagePipe)
                    .code(MessageCode::Ok)
                    .data(serde_json::to_vec(&JsonValueMessage {
                        json_value: value
                    }).unwrap())
                    .send(&self.session)
                    .await?,
                _ => {}
            };
        }
        let message = self.session.recv().await?;

        if message.is_bye() {
            return Ok(None);
        }

        let EvaluateScriptMessage { source_code, .. } = serde_json::from_slice(&message.data)?;

        let slot_id = self.next_slot;
        self.slots.insert(slot_id, Slot::Source(source_code));
        self.next_slot += 1;

        Ok(Some(slot_id))
    }

    pub fn run<'s>(&mut self, scope: &mut v8::HandleScope<'s>, slot_id: SlotId) -> Result<(), Error> {
        let slot = match self.slots.remove(&slot_id).unwrap() {
            Slot::Source(source_code) => {
                // TODO(brokad): Run this in isolate
                let source_value = v8::String::new(scope, &source_code).unwrap();
                let script = v8::Script::compile(scope, source_value, None).unwrap();
                let result_value = script.run(scope).unwrap();
                let as_json: serde_json::Value = serde_v8::from_v8(scope, result_value).unwrap();
                Slot::Ok(as_json)
            },
            otherwise => otherwise
        };

        self.slots.insert(slot_id, slot).unwrap();

        Ok(())
    }
}

pub struct InstanceResource(RefCell<Instance>);

impl Resource for InstanceResource {}

impl InstanceResource {
    pub fn from_op_state(state: Rc<RefCell<OpState>>, instance_id: ResourceId) -> Rc<Self> {
        state.borrow_mut().resource_table.get::<Self>(instance_id).unwrap()
    }

    pub fn borrow_mut(&self) -> RefMut<'_, Instance> {
        self.0.borrow_mut()
    }
}

#[op]
#[tracing::instrument(skip(state))]
async fn task_register_instance(
    state: Rc<RefCell<OpState>>,
    task_name: String
) -> Result<ResourceId, AnyError> {
    let (compiler, op_client) = {
        let resource_table = &mut state.borrow_mut().resource_table;
        (
            resource_table.get::<CompilerResource>(COMPILER_RID).unwrap(),
            resource_table.get::<ClientResource>(CLIENT_RID).unwrap()
        )
    };

    let mut session = op_client.borrow_mut().await.new_session().await.unwrap();

    session.do_handshake().await.unwrap();

    let mut buf = Vec::new();
    compiler.borrow_mut().print_task_context(&task_name, Cursor::new(&mut buf)).unwrap();
    let task_context = String::from_utf8(buf).unwrap();

    let task_description = compiler.borrow_mut().get_task_description(&task_name).unwrap();

    let msg = NewInstanceMessage {
        task_name,
        task_description,
        task_context
    };

    MessageBuilder::new()
        .message_type(MessageType::MessageControl)
        .code(MessageCode::Task)
        .data(&serde_json::to_string(&msg).unwrap())
        .send(&session)
        .await
        .unwrap();

    let instance = Instance::from_session(session);

    let resource_id = state.borrow_mut().resource_table.add(instance.into_resource());

    Ok(resource_id)
}

#[op]
#[tracing::instrument(skip(state))]
async fn task_poll_instance(
    state: Rc<RefCell<OpState>>,
    instance_id: ResourceId,
    slot_id: Option<SlotId>
) -> Result<Option<SlotId>, AnyError> {
    InstanceResource::from_op_state(state, instance_id)
        .borrow_mut()
        .poll(slot_id)
        .await
        .map_err(|_| todo!())
}

#[op(v8)]
#[tracing::instrument(skip(state))]
fn task_run_with_side_effects<'s>(
    scope: &mut v8::HandleScope<'s>,
    state: Rc<RefCell<OpState>>,
    instance_id: ResourceId,
    slot_id: Option<ResourceId>,
) -> Result<(), AnyError> {
    if let Some(slot_id) = slot_id {
        InstanceResource::from_op_state(state, instance_id)
            .borrow_mut()
            .run(scope, slot_id)
            .unwrap();
    }
    Ok(())
}

#[op]
#[tracing::instrument(skip(state))]
async fn task_cancel_instance(
    state: Rc<RefCell<OpState>>,
    instance_id: ResourceId
) -> Result<(), AnyError> {
    state.borrow_mut().resource_table.close(instance_id)
}