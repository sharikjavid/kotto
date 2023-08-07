use futures::stream::unfold;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tonic::Request;
use tonic::transport::Channel;

use tokio::sync::mpsc::{self, Sender, Receiver};
use tokio::sync::broadcast;
use tracing::{event, Level};
use crate::runtime::{ModuleConfig, RunCommandOptions, Runtime};

use std::collections::HashMap;

use crate::proto::{trackway::trackway_client::TrackwayClient, Message, MessageCode, MessageBuilder};
use crate::error::Error;
use crate::proto::trackway::MessageType;

#[derive(Serialize, Deserialize)]
pub struct ExportTask {
    name: String,
    attributes: HashMap<String, ExportAttribute>
}

impl ExportTask {
    pub fn from_module_config(config: &ModuleConfig) -> Self {
        Self {
            name: config.name.to_string(),
            attributes: config
                .commands
                .iter()
                .map(|(command_name, command_config)| {
                    (
                        command_name.clone(),
                        ExportAttribute {
                            description: command_config.description.clone()
                        }
                    )
                })
                .collect()
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct ExportAttribute {
    description: String
}

#[derive(Debug)]
pub struct Client {
    grpc: TrackwayClient<Channel>
}

impl Client {
    pub async fn new(addr: &str) -> Self {
        let grpc = TrackwayClient::connect(addr.to_string()).await.unwrap();

        Self {
            grpc,
        }
    }

    pub async fn new_session(&mut self) -> Result<Session, Error> {
        let (write_send, write_receive) = mpsc::channel::<Message>(256);
        let write_receiver_channel = unfold(write_receive, |mut receiver| async move {
            let next = receiver.recv().await?;
            Some((next, receiver))
        });

        let (read_send, read_receive) = mpsc::channel::<Message>(256);
        let mut read_stream = self.grpc.new_session(Request::new(write_receiver_channel)).await.unwrap().into_inner();
        tokio::spawn(async move {
            while let Some(Ok(next)) = read_stream.next().await {
                read_send.send(next).await.unwrap();
            }
        });

        let (subscribers, _) = broadcast::channel(256);

        Ok(Session {
            write: write_send,
            read: read_receive,
            subscribers,
            terminate: false
        })
    }
}

#[derive(Debug)]
pub struct Session {
    write: Sender<Message>,
    read: Receiver<Message>,
    subscribers: broadcast::Sender<Message>,
    terminate: bool
}

impl Session {
    pub fn subscribe(&self) -> broadcast::Receiver<Message> {
        self.subscribers.subscribe()
    }

    pub(crate) fn sender(&self) -> Sender<Message> {
        self.write.clone()
    }

    #[tracing::instrument]
    pub(crate) async fn recv(&mut self) -> Result<Message, Error> {
        let next = self.read.recv().await.unwrap();
        event!(Level::DEBUG, "<- Message: {:?}", next);
        Ok(next)
    }

    #[tracing::instrument]
    pub(crate) async fn send(&self, message: Message) -> Result<(), Error> {
        event!(Level::DEBUG, "-> Message: {:?}", message);
        Ok(self.write.send(message).await.unwrap())
    }

    #[tracing::instrument]
    pub async fn do_handshake(&mut self, token: &str) -> Result<(), Error> {
        self.send(MessageBuilder::hello()).await?;

        let resp = self.recv().await?;
        assert_eq!(resp.code, MessageCode::Hello.to_string());

        let resp = self.recv().await?;
        assert_eq!(resp.code, MessageCode::SendToken.to_string());

        MessageBuilder::new()
            .code(MessageCode::Ok)
            .data(token.as_bytes())
            .send(self)
            .await?;

        let resp = self.recv().await?;
        assert_eq!(resp.code, MessageCode::Ok.to_string());

        Ok(())
    }

    #[tracing::instrument(skip(rt))]
    pub async fn handle_control(&mut self, rt: &mut Runtime, code: MessageCode, data: &[u8]) -> Result<(), Error> {
        match code {
            MessageCode::Bye => {
                self.terminate = true;
            },
            MessageCode::Ready => {
                MessageBuilder::new()
                    .message_type(MessageType::MessagePrompt)
                    .data(&rt.get_config()?.task)
                    .send(&self)
                    .await?;
            }
            MessageCode::SendExports => {
                let exports = ExportTask::from_module_config(&rt.get_config()?);
                MessageBuilder::new()
                    .message_type(MessageType::MessageControl)
                    .code(MessageCode::Exports)
                    .data(&serde_json::to_vec(&exports)?)
                    .send(&self)
                    .await?;
            },
            MessageCode::Call => {
                let run: RunCommandOptions = serde_json::from_slice(data)?;
                let output = rt.call(run).await?;
                MessageBuilder::new()
                    .message_type(MessageType::MessagePipe)
                    .code(MessageCode::Unknown)
                    .data(&serde_json::to_vec(&output)?)
                    .send(&self)
                    .await?;
            },
            _ => {}
        };
        Ok(())
    }

    pub async fn serve(mut self, mut rt: Runtime) -> Result<(), Error> {
        while !self.terminate {
            let msg = self.recv().await?;

            self.subscribers.send(msg.clone()).unwrap();

            let res = match msg {
                msg if msg.is_control() => self.handle_control(&mut rt, msg.code(), &msg.data).await,
                _ => {
                    Ok(())
                }
            };

            if let Err(err) = res {
                event!(Level::ERROR, "{err:?}");
                // TODO emit an error message if required
            }
        }

        Ok(())
    }
}