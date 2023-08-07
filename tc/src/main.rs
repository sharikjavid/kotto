use std::path::{Path, PathBuf};

use std::process::exit;
use clap::{Parser, Subcommand};
use tracing_subscriber::prelude::*;
use tracing::{event, Level};
use crate::client::Client;
use crate::runtime::Runtime;

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

    pub async fn run_script<P: AsRef<Path>>(script_path: P) -> Result<(), error::Error> {
        let cfg = config::Config::load().await?;

        let token = cfg.token.as_ref().expect("you must run `login` first");
        let server_url = "http://localhost:8000";

        event!(Level::INFO, "connecting to {server_url}");
        let client = Client::new(server_url, token).await;

        let mut runtime = Runtime::new_with_client(client);
        let module_specifier = deno_core::resolve_path(
            script_path.as_ref().to_str().unwrap(),
            &std::env::current_dir().unwrap()
        )?;

        let module_id = runtime.load_main_module(&module_specifier).await?;

        runtime.evaluate_module(module_id).await?;

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
