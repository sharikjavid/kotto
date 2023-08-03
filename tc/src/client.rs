use futures::stream::unfold;
use futures::StreamExt;
use tonic::Request;
use tonic::transport::Channel;

use tokio::sync::mpsc::{self, Sender, Receiver};
use tokio::sync::broadcast;
use tracing::{event, Level};
use crate::apps::{AppsManager, RegistryEntry, RunCommand};

use crate::proto::{trackway::trackway_client::TrackwayClient, Message, MessageCode, MessageBuilder};
use crate::error::Error;
use crate::proto::trackway::MessageType;

pub trait ToMessageData {
    fn to_data(&self) -> Result<Vec<u8>, Error>;
}

impl<T> ToMessageData for T
    where
        T: serde::Serialize
{
    fn to_data(&self) -> Result<Vec<u8>, Error> {
        Ok(serde_json::to_vec(self)?)
    }
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
            subscribers
        })
    }
}

#[derive(Debug)]
pub struct Session {
    write: Sender<Message>,
    read: Receiver<Message>,
    subscribers: broadcast::Sender<Message>
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
            .data(token)
            .send(self)
            .await?;

        let resp = self.recv().await?;
        assert_eq!(resp.code, MessageCode::Ok.to_string());

        Ok(())
    }

    #[tracing::instrument(skip(manager))]
    pub async fn handle_control(&mut self, manager: &mut AppsManager, code: MessageCode, data: &[u8]) -> Result<(), Error> {
        match code {
            MessageCode::Bye => return Ok(()),
            MessageCode::Ready => {
                event!(Level::INFO, "Ready")
            },
            MessageCode::Install => {
                let entry: RegistryEntry = serde_json::from_slice(data)?;
                let config = manager.install_app(entry).await?.to_data()?;
                MessageBuilder::new()
                    .message_type(MessageType::MessageControl)
                    .code(MessageCode::Ok)
                    .data(config)
                    .send(&self)
                    .await?;
            },
            MessageCode::Uninstall => {
                todo!()
            },
            MessageCode::Call => {
                let run: RunCommand = serde_json::from_slice(data)?;
                let res = manager.call(&run.app, &run.command).await.unwrap()?;
                MessageBuilder::new()
                    .message_type(MessageType::MessagePipe)
                    .code(MessageCode::Unknown)
                    .data(res.to_data()?)
                    .send(&self)
                    .await?;
            },
            _ => {}
        };
        Ok(())
    }

    pub async fn serve(mut self, mut manager: AppsManager) -> Result<(), Error> {
        loop {
            let msg = self.recv().await?;

            self.subscribers.send(msg.clone()).unwrap();

            let res = match msg {
                msg if msg.is_control() => self.handle_control(&mut manager, msg.code(), &msg.data).await,
                _ => {
                    Ok(())
                }
            };

            if let Err(err) = res {
                event!(Level::ERROR, "{err:?}");
                // TODO emit an error message if required
            }
        }
    }
}