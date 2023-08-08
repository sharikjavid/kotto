use futures::stream::unfold;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tonic::Request;
use tonic::transport::Channel;

use tokio::sync::mpsc::{self, Sender, Receiver};
use tokio::sync::broadcast;
use tracing::{event, Level};

use crate::proto::{trackway::trackway_client::TrackwayClient, Message, MessageCode, MessageBuilder};
use crate::error::Error;

#[derive(Debug)]
pub struct Client {
    grpc: TrackwayClient<Channel>,
    token: String
}

impl Client {
    pub async fn new(addr: &str, token: &str) -> Self {
        let grpc = TrackwayClient::connect(addr.to_string()).await.unwrap();

        Self {
            grpc,
            token: token.to_string()
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
            token: self.token.clone(),
            terminate: false
        })
    }
}

#[derive(Debug)]
pub struct Session {
    write: Sender<Message>,
    read: Receiver<Message>,
    subscribers: broadcast::Sender<Message>,
    token: String,
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
    pub async fn do_handshake(&mut self) -> Result<(), Error> {
        self.send(MessageBuilder::hello()).await?;

        let resp = self.recv().await?;
        assert_eq!(resp.code, MessageCode::Hello.to_string());

        let resp = self.recv().await?;
        assert_eq!(resp.code, MessageCode::SendToken.to_string());

        MessageBuilder::new()
            .code(MessageCode::Ok)
            .data(self.token.as_bytes())
            .send(self)
            .await?;

        let resp = self.recv().await?;
        assert_eq!(resp.code, MessageCode::Ok.to_string());

        Ok(())
    }
}