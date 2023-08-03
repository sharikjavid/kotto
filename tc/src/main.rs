use clap::Parser;
use tracing_subscriber::prelude::*;
use tracing::{event, Level};
use crate::apps::AppsManager;
use crate::proto::MessageBuilder;

pub mod proto;
pub mod client;
pub mod error;
pub mod apps;
pub mod repl;

pub(crate) mod ts_module_loader;

/// The Trackway agent.
/// Find out more at https://trackway.ai
#[derive(Parser, Debug)]
struct Args {
    /// Address of the server
    server: String,
    /// Agent token
    #[arg(long, env = "AGENT_TOKEN")]
    token: String
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

    resp.data = args.token.into_bytes();
    session.send(resp).await.unwrap();

    let repl = repl::Repl::from_session(&session);
    let manager = AppsManager::new().await?;
    tokio::select!(
        _ = session.serve(manager) => {},
        _ = repl.run() => {}
    );

    println!("Success!");

    Ok(())
}
