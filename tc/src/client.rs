use std::error::Error;
use std::fmt::{Display, Formatter};
use futures::stream::unfold;
use futures::StreamExt;
use tonic::Request;
use tonic::transport::Channel;

use tokio::sync::mpsc::{self, Sender, Receiver};
use tracing::{event, Level};

use crate::proto::{trackway::trackway_client::TrackwayClient, Message, MessageCode};

#[derive(Debug)]
pub enum ClientError {
    Cascade(Box<dyn Error>)
}

impl From<Box<dyn Error>> for ClientError {
    fn from(value: Box<dyn Error>) -> Self {
        Self::Cascade(value)
    }
}

impl Display for ClientError {
    fn fmt(&self, _: &mut Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

impl Error for ClientError {}

#[derive(Debug)]
pub struct Client {
    grpc: TrackwayClient<Channel>,

}

impl Client {
    pub async fn new(addr: &str) -> Self {
        let grpc = TrackwayClient::connect(addr.to_string()).await.unwrap();

        Self {
            grpc,
        }
    }

    pub async fn new_session(&mut self) -> Result<Session, ClientError> {
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

        Ok(Session {
            write: write_send,
            read: read_receive,
        })
    }
}

#[derive(Debug)]
pub struct Session {
    write: Sender<Message>,
    read: Receiver<Message>,

}

impl Session {
    #[tracing::instrument]
    pub async fn recv(&mut self) -> Result<Message, ClientError> {
        let next = self.read.recv().await.unwrap();
        event!(Level::DEBUG, "<- Message: {:?}", next);
        Ok(next)
    }

    #[tracing::instrument]
    pub async fn send(&self, message: Message) -> Result<(), ClientError> {
        event!(Level::DEBUG, "-> Message: {:?}", message);
        Ok(self.write.send(message).await.unwrap())
    }

    pub async fn serve(mut self) -> Result<(), ClientError> {
        loop {
            let msg = self.recv().await?;
            match msg.code() {
                MessageCode::Bye => return Ok(()),
                _ => {}
            }
        }
    }
}