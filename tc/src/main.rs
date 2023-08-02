use clap::Parser;
use tracing_subscriber::prelude::*;
use tracing::{event, Level};
use crate::apps::AppsManager;
use crate::proto::MessageBuilder;

pub mod proto;
pub mod client;
pub mod error;
pub mod apps;

/// The Trackway agent.
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

    let args = Args::parse();

    event!(Level::INFO, "Connecting to {}", &args.server);
    let mut client = client::Client::new(&args.server).await;

    event!(Level::INFO, "Starting new session");
    let mut session = client.new_session().await.unwrap();

    event!(Level::INFO, "Sending `hello`");
    session.send(MessageBuilder::hello()).await.unwrap();

    let resp = session.recv().await.unwrap();
    assert_eq!(resp.code, proto::MessageCode::Hello.to_string());

    let mut resp = session.recv().await.unwrap();
    assert_eq!(resp.code, proto::MessageCode::SendToken.to_string());

    resp.data = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string().into_bytes();
    session.send(resp).await.unwrap();

    let manager = AppsManager::new().await?;
    session.serve(manager).await.unwrap();

    println!("Success!");

    Ok(())
}
