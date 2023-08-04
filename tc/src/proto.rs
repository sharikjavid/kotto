use std::io::Cursor;

use serde::Serialize;

pub use trackway::{Message, MessageCode};
use trackway::message::MessageType;

use crate::error::Error;
use crate::client::Session;

pub mod trackway {
    use std::convert::Infallible;
    use std::fmt::{Display, Formatter};
    use std::str::FromStr;

    tonic::include_proto!("trackway");

    pub use message::MessageType;
    use crate::proto::trackway::MessageType::{MessageControl, MessagePipe, MessagePrompt};

    #[derive(Debug, Eq, PartialEq)]
    pub enum MessageCode {
        Hello,
        SendToken,
        SendExports,
        Exports,
        Ready,
        Call,
        Ok,
        Err,
        Unknown,
        Bye
    }

    impl Display for MessageCode {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            let to_string = match self {
                Self::Hello => "hello",
                Self::SendToken => "send_token",
                Self::SendExports => "send_exports",
                Self::Exports => "exports",
                Self::Ready => "ready",
                Self::Call => "call",
                Self::Ok => "ok",
                Self::Err => "err",
                Self::Unknown => "unknown",
                Self::Bye => "bye"
            };
            write!(f, "{to_string}")
        }
    }

    impl FromStr for MessageCode {
        type Err = Infallible;
        fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
            let res = match s {
                "hello" => Self::Hello,
                "send_token" => Self::SendToken,
                "send_exports" => Self::SendExports,
                "exports" => Self::Exports,
                "ready" => Self::Ready,
                "call" => Self::Call,
                "ok" => Self::Ok,
                "err" => Self::Err,
                "bye" => Self::Bye,
                _ => Self::Unknown
            };
            Ok(res)
        }
    }

    impl Message {
        pub fn is_control(&self) -> bool {
            self.message_type == i32::from(MessageControl)
        }

        pub fn is_pipe(&self) -> bool {
            self.message_type == i32::from(MessagePipe)
        }

        pub fn is_prompt(&self) -> bool {
            self.message_type == i32::from(MessagePrompt)
        }

        pub fn code(&self) -> MessageCode {
            MessageCode::from_str(&self.code).unwrap()
        }

        pub fn is_bye(&self) -> bool {
            self.is_control() && self.code() == MessageCode::Bye
        }
    }
}

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

pub struct MessageBuilder {
    message: Message,
}

impl Default for MessageBuilder {
    fn default() -> Self {
        Self {
            message: Message::default()
        }
    }
}

impl MessageBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_existing(message: Message) -> Self {
        Self {
            message
        }
    }

    pub fn message_type(mut self, message_type: MessageType) -> Self {
        self.message.set_message_type(message_type);
        self
    }

    pub fn code<S: ToString>(mut self, code: S) -> Self {
        self.message.code = code.to_string();
        self
    }

    pub fn hello() -> Message {
        Self::default().message_type(MessageType::MessageControl).code(MessageCode::Hello).build()
    }

    pub fn body_json<T: Serialize>(mut self, body: T) -> Self {
        let w = Cursor::new(&mut self.message.data);
        serde_json::to_writer_pretty(w, &body).unwrap();
        self
    }

    pub fn data<D: AsRef<[u8]>>(mut self, data: D) -> Self {
        self.message.data = data.as_ref().to_vec();
        self
    }

    pub fn build(self) -> Message {
        self.message
    }

    pub async fn send(self, session: &Session) -> Result<(), Error> {
        session.send(self.build()).await
    }
}