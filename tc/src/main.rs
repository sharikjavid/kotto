use std::path::{Path, PathBuf};

use std::process::exit;
use clap::{Parser, Subcommand};
use futures::FutureExt;
use tracing_subscriber::prelude::*;
use tracing::{event, Level};
use crate::apps::AppsManager;
use crate::error::Error;

pub mod proto;
pub mod client;
pub mod error;
pub mod apps;
pub mod repl;
pub mod config;

pub(crate) mod ts_module_loader;

/// The Trackway agent.
/// Find out more at https://trackway.ai
#[derive(Parser, Debug)]
struct Cli {
    #[command(subcommand)]
    command: Commands
}

impl Cli {
    pub async fn do_cli(self) -> Result<(), Error> {
        match self.command {
            Commands::Run { script } => Self::run_script(script).await,
            Commands::Login { token } => Self::login(token).await
        }
    }

    pub async fn run_script<P: AsRef<Path>>(script: P) -> Result<(), Error> {
        let cfg = config::Config::load().await?;

        let manager = AppsManager::new().await?;

        let server_url = "http://ldn.damien.sh:8690";

        event!(Level::INFO, "Connecting to {}", server_url);
        let mut client = client::Client::new(server_url).await;

        event!(Level::INFO, "Starting new session");
        let mut session = client.new_session().await.unwrap();

        event!(Level::INFO, "Starting handshake");
        session.do_handshake(cfg.token.as_ref().expect("you must run `login` first")).await?;

        let task = if atty::is(atty::Stream::Stdin) {
            repl::Repl::from_session(&session).run().boxed()
        } else {
            todo!()
        };

        tokio::select!(
            res = session.serve(manager) => res,
            res = task => res
        )
    }

    pub async fn login(token: Option<String>) -> Result<(), Error> {
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
        eprintln!("error: {err}");
        exit(1)
    } else {
        exit(0)
    }
}
