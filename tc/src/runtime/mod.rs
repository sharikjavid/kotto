use std::borrow::Cow;
use std::cell::RefCell;
use std::error::Error as StdError;
use std::path::PathBuf;
use std::collections::HashMap;
use std::ops::Deref;
use std::rc::Rc;
use deno_ast::ModuleSpecifier;
use serde::{Serialize, Deserialize};

use deno_core::{op, Op, JsRuntime, ModuleId, ResourceId, OpState, AsyncRefCell, Resource, AsyncMutFuture, AsyncRefFuture, RcRef, Extension};
use deno_core::error::AnyError;
use futures::{SinkExt, TryFutureExt};
use crate::client::{Client, Session};

mod compile;
mod emit;
use emit::Emitter;

use crate::error::Error;
use crate::proto::{MessageBuilder, MessageCode};
use crate::proto::trackway::MessageType;
use crate::runtime::compile::TaskMapResource;

const CLIENT_RID: ResourceId = 0;
const TASK_MAP_RID: ResourceId = 1;

#[derive(Serialize, Deserialize, Hash, Eq, PartialEq, Clone)]
#[serde(transparent)]
pub struct AppName(String);

impl Deref for AppName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Serialize, Deserialize)]
pub struct ModuleConfig {
    pub name: AppName,
    pub description: String,
    pub methods: HashMap<String, MethodConfig>
}

#[derive(Serialize, Deserialize)]
pub struct MethodConfig {
    pub description: String
}

#[derive(Serialize, Deserialize)]
pub struct PollInstanceOp {
    method_name: String,
    args: Vec<serde_json::Value>,
    done: bool
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
        let task_map_resource = Rc::new(TaskMapResource::new());

        let rt = JsRuntime::new(deno_core::RuntimeOptions {
            module_loader: Some(Rc::new(compile::SimpleModuleLoader::new(task_map_resource.clone()))),
            extensions: vec![
                Extension {
                    ops: Cow::from(vec![
                        task_register_instance::DECL,
                        task_poll_instance::DECL,
                        task_send_output::DECL,
                        task_cancel_instance::DECL
                    ]),
                    op_state_fn: Some(Box::new(|op_state| {
                        let client_resource = ClientResource::from_client(client);
                        assert_eq!(op_state.resource_table.add(client_resource), CLIENT_RID);
                        assert_eq!(op_state.resource_table.add_rc(task_map_resource), TASK_MAP_RID);
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
    pub fn borrow_client_mut(self: Rc<Self>) -> AsyncMutFuture<Client> {
        RcRef::map(self, |this| &this.inner).borrow_mut()
    }
}

pub struct Instance {
    session: AsyncRefCell<Session>
}

impl Resource for Instance {}

impl Instance {
    pub fn borrow_session_mut(self: Rc<Self>) -> AsyncMutFuture<Session> {
        RcRef::map(self, |this| &this.session).borrow_mut()
    }

    pub fn borrow_session(self: Rc<Self>) -> AsyncRefFuture<Session> {
        RcRef::map(self, |this| &this.session).borrow()
    }

    pub async fn poll(self: Rc<Self>) -> Result<PollInstanceOp, Error> {
        let message = self.borrow_session_mut().await.recv().await?;
        // TODO(brokad): make sure we only receive the right messages here (control, call types)
        Ok(serde_json::from_slice(&message.data)?)
    }

    pub async fn send(self: Rc<Self>, output: &serde_json::Value) -> Result<(), Error> {
        let session = self.borrow_session().await;
        MessageBuilder::new()
            .message_type(MessageType::MessagePipe)
            .code(MessageCode::Unknown)
            .data(serde_json::to_vec(output)?)
            .send(&session)
            .await
    }

    pub fn from_op_state(state: Rc<RefCell<OpState>>, instance_id: ResourceId) -> Result<Rc<Self>, Error> {
        let instance = state.borrow_mut().resource_table.get::<Self>(instance_id).unwrap();
        Ok(instance)
    }

    #[tracing::instrument(skip(state))]
    pub async fn add_new(state: Rc<RefCell<OpState>>, task_id: String) -> Result<ResourceId, Error> {
        let (task_map, op_client) = {
            let resource_table = &mut state.borrow_mut().resource_table;
            (
                resource_table.get::<TaskMapResource>(TASK_MAP_RID).unwrap(),
                resource_table.get::<ClientResource>(CLIENT_RID).unwrap()
            )
        };

        let task = task_map.get_task(&task_id).unwrap();

        let mut session = op_client.borrow_client_mut().await.new_session().await?;

        session.do_handshake().await?;

        // TODO(brokad): send task
        //

        let instance = Self {
            session: AsyncRefCell::new(session)
        };
        Ok(state.borrow_mut().resource_table.add(instance))
    }
}

#[op]
#[tracing::instrument(skip(state))]
async fn task_register_instance(
    state: Rc<RefCell<OpState>>,
    task_id: String
) -> ResourceId {
    Instance::add_new(state, task_id)
        .await
        .unwrap()
}

#[op]
#[tracing::instrument(skip(state))]
async fn task_poll_instance(
    state: Rc<RefCell<OpState>>,
    instance_id: ResourceId
) -> Result<PollInstanceOp, AnyError> {
    Instance::from_op_state(state, instance_id)
        .unwrap()
        .poll()
        .await
        .map_err(|_| todo!())
}

#[op]
#[tracing::instrument(skip(state))]
async fn task_send_output(
    state: Rc<RefCell<OpState>>,
    instance_id: ResourceId,
    output: serde_json::Value
) -> Result<(), AnyError> {
    Instance::from_op_state(state, instance_id)
        .unwrap()
        .send(&output)
        .await
        .map_err(|_| todo!())
}

#[op]
#[tracing::instrument(skip(state))]
async fn task_cancel_instance(
    state: Rc<RefCell<OpState>>,
    instance_id: ResourceId
) -> Result<(), AnyError> {
    state.borrow_mut().resource_table.close(instance_id)
}