use std::io::Cursor;
use serde::{Serialize, Deserialize};

pub use trackway::{Message, MessageCode};
use trackway::message::MessageType;
use crate::client::{ClientError, Session};

pub mod trackway {
    use std::convert::Infallible;
    use std::fmt::{Display, Formatter};
    use std::str::FromStr;

    tonic::include_proto!("trackway");

    pub use message::MessageType;

    #[derive(Debug, Eq, PartialEq)]
    pub enum MessageCode {
        Hello,
        SendToken,
        SendApps,
        RunApp,
        Unknown,
        Bye
    }

    impl Display for MessageCode {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            let to_string = match self {
                Self::Hello => "hello",
                Self::SendToken => "send_token",
                Self::SendApps => "send_apps",
                Self::RunApp => "run_app",
                Self::Unknown => "unknown",
                Self::Bye => "bye"
            };
            write!(f, "{to_string}")
        }
    }

    impl FromStr for MessageCode {
        type Err = Infallible;
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let res = match s {
                "hello" => Self::Hello,
                "send_token" => Self::SendToken,
                "send_apps" => Self::SendApps,
                "run_app" => Self::RunApp,
                "bye" => Self::Bye,
                _ => Self::Unknown
            };
            Ok(res)
        }
    }

    impl Message {
        pub fn is_control(&self) -> bool {
            self.message_type == i32::from(message::MessageType::MessageControl)
        }

        pub fn code(&self) -> MessageCode {
            MessageCode::from_str(&self.code).unwrap()
        }

        pub fn is_bye(&self) -> bool {
            self.is_control() && self.code() == MessageCode::Bye
        }
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

    pub fn data(mut self, data: Vec<u8>) -> Self {
        self.message.data = data;
        self
    }

    pub fn build(self) -> Message {
        self.message
    }

    pub async fn send(self, session: &Session) -> Result<(), ClientError> {
        session.send(self.build()).await
    }
}