use std::path::{Path, PathBuf};

use std::process::exit;
use clap::{Parser, Subcommand};
use tracing_subscriber::prelude::*;
use tracing::{event, Level};

pub mod proto;
pub mod client;
pub mod error;
pub mod runtime;
pub mod config;

/// The Trackway agent.
/// Find out more at https://trackway.ai
#[derive(Parser, Debug)]
struct Cli {
    #[command(subcommand)]
    command: Commands
}

impl Cli {
    pub async fn do_cli(self) -> Result<(), error::Error> {
        match self.command {
            Commands::Run { script } => Self::run_script(script).await,
            Commands::Submit { .. } => todo!(),
            Commands::Login { token } => Self::login(token).await
        }
    }

    pub async fn run_script<P: AsRef<Path>>(script: P) -> Result<(), error::Error> {
        let cfg = config::Config::load().await?;

        let server_url = "http://localhost:8000";

        event!(Level::INFO, "Connecting to {}", server_url);
        let mut client = client::Client::new(server_url).await;

        event!(Level::INFO, "Starting new session");
        let mut session = client.new_session().await.unwrap();

        event!(Level::INFO, "Starting handshake");
        session.do_handshake(cfg.token.as_ref().expect("you must run `login` first")).await?;

        let task = tokio::spawn({
            let mut subscriber = session.subscribe();
            async move {
                loop {
                    let msg = subscriber.recv().await.unwrap();
                    if msg.is_prompt() { return msg; }
                }
            }
        });

        let rt = runtime::Runtime::new(script.as_ref()).await?;

        tokio::select!(
            _ = session.serve(rt) => {
                event!(Level::WARN, "Session terminated early");
            },
            msg = task => {
                println!("{}", String::from_utf8(msg.unwrap().data).unwrap());
            }
        );

        Ok(())
    }

    pub async fn login(token: Option<String>) -> Result<(), error::Error> {
        let mut cfg = config::Config::load().await?;
        cfg.token = Some(token.expect("--token is required"));
        cfg.save().await
    }
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Run {
        script: PathBuf
    },
    Submit {

    },
    Login {
        /// Set the agent token
        #[arg(long)]
        token: Option<String>
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer());

    tracing::subscriber::set_global_default(subscriber)?;

    tracing_log::LogTracer::init()?;

    let cli = Cli::parse();

    if let Err(err) = cli.do_cli().await {
        eprintln!("error: {err:?}");
        exit(1)
    } else {
        exit(0)
    }
}
