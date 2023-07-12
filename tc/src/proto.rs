use trackway::Message;
use trackway::message::MessageType;
use crate::proto::trackway::message::MessageType::MessageControl;

pub mod trackway {
    tonic::include_proto!("trackway");
}

mod sealed {
    pub trait Sealed {}
    impl Sealed for super::Message {}
}

pub trait MessageExt: Into<Message> + sealed::Sealed {
    fn hello(self) -> Message {
        let mut message = self.into();
        message.set_message_type(MessageControl);
        message.chunk = "Hello.".to_string();
        message
    }
}

impl MessageExt for Message {}