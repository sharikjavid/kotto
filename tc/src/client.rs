use std::error::Error;
use std::fmt::{Display, Formatter};
use futures::stream::unfold;
use tonic::{Request, Streaming};
use tonic::transport::Channel;

use tokio::sync::mpsc::{self, Sender};

use crate::proto::trackway::{trackway_client::TrackwayClient, Message};

#[derive(Debug)]
pub enum ClientError {}

impl Display for ClientError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

impl Error for ClientError {}

#[derive(Debug)]
pub struct Client {
    grpc: TrackwayClient<Channel>,
    send: Sender<Message>,
    recv: Streaming<Message>,
}

impl Client {
    pub async fn new(addr: &str) -> Self {
        let mut grpc = TrackwayClient::connect(addr.to_string()).await.unwrap();

        let (send, receiver) = mpsc::channel::<Message>(256);

        let receiver_channel = unfold(receiver, |mut receiver| async move {
            let next = receiver.recv().await.unwrap();
            Some((next, receiver))
        });

        let recv = grpc.channel(Request::new(receiver_channel)).await.unwrap().into_inner();

        Self {
            grpc,
            send,
            recv,
        }
    }

    #[tracing::instrument]
    pub async fn recv(&mut self) -> Result<Option<Message>, ClientError> {
        Ok(self.recv.message().await.unwrap())
    }

    #[tracing::instrument]
    pub async fn send(&self, message: Message) -> Result<(), ClientError> {
        Ok(self.send.send(message).await.unwrap())
    }
}