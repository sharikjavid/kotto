use crate::error::Error;

use std::io::{self, Write};
use std::process::exit;
use tokio::task::spawn_blocking;
use tokio::sync::{broadcast, mpsc};
use crate::client::Session;
use crate::proto::{Message, MessageBuilder};
use crate::proto::trackway::MessageType;

pub struct Repl {
    subscriber: broadcast::Receiver<Message>,
    sender: mpsc::Sender<Message>
}

impl Repl {
    pub fn from_session(session: &Session) -> Self {
        Self {
            subscriber: session.subscribe(),
            sender: session.sender()
        }
    }

    pub fn read_line() -> String {
        let mut buf = String::new();
        print!("> ");
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut buf).unwrap();
        buf
    }

    pub async fn run(mut self) -> Result<(), Error> {
        loop {
            let next_line = spawn_blocking(|| Self::read_line()).await?;

            if let Some(control) = next_line.trim().strip_prefix(":") {
                match control {
                    "exit" => exit(0),
                    cmd => println!("unknown command: {cmd}")
                }
                continue;
            }

            let msg = MessageBuilder::new()
                .message_type(MessageType::MessagePrompt)
                .data(next_line.into_bytes())
                .build();
            self.sender.send(msg).await.unwrap();

            loop {
                let msg = self.subscriber.recv().await.unwrap();
                if msg.is_prompt() {
                    println!("{}", String::from_utf8(msg.data).unwrap());
                    break;
                }
            }
        }
    }
}