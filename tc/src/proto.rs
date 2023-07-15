use std::fmt::{Display, Formatter};
use serde::{Serialize, Deserialize};

pub use trackway::Message;
use trackway::message::MessageType;

pub mod trackway {
    tonic::include_proto!("trackway");
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

pub enum MessageCode {
    Hello,
    SendToken,
    SendApps,
    RunApp,
    Bye
}

impl Display for MessageCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let to_string = match self {
            Self::Hello => "hello",
            Self::SendToken => "send_token",
            Self::SendApps => "send_apps",
            Self::RunApp => "run_app",
            Self::Bye => "bye"
        };
        write!(f, "{to_string}")
    }
}

impl MessageBuilder {
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

    pub fn build(self) -> Message {
        self.message
    }
}