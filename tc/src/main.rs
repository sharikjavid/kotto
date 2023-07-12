use clap::Parser;
use tracing_subscriber::prelude::*;
use tracing::{event, Level};

pub mod proto;
pub mod client;

use proto::MessageExt;

/// The Trackway agent
/// Find out more at https://trackway.ai
#[derive(Parser, Debug)]
struct Args {
    /// Address of the server
    server: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer());

    tracing::subscriber::set_global_default(subscriber)?;

    tracing_log::LogTracer::init()?;

    event!(Level::INFO, "Starting up");

    let args = Args::parse();

    let mut client = client::Client::new(&args.server).await;

    client.send(proto::trackway::Message::default().hello()).await?;

    while let Ok(Some(message)) = client.recv().await {
        println!("message: {}", message.chunk);
    }

    Ok(())
}
