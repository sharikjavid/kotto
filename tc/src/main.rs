use std::path::{Path, PathBuf};

use std::process::exit;
use clap::{Parser, Subcommand};
use deno_runtime::permissions::{Permissions, PermissionsContainer};
use tracing_subscriber::prelude::*;
use tracing::{event, Level};

pub mod proto;
pub mod error;
pub mod runtime;
pub mod config;
pub mod client;

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
        /*
        let token = cfg.token.as_ref().expect("you must run `login` first");
        let server_url = "http://localhost:8000";
        event!(Level::INFO, "connecting to {server_url}");
         */

        let flags = deno::args::flags_from_vec(vec![
            "deno".to_string(),
            "run".to_string(),
            script_path.as_ref().to_string_lossy().to_string()
        ]).unwrap();
        let factory = deno::factory::CliFactory::from_flags(flags).await?;
        let cli_options = factory.cli_options();

        let main_module = cli_options.resolve_main_module()?;

        let permissions = PermissionsContainer::new(Permissions::from_options(
            &cli_options.permissions_options(),
        )?);
        let worker_factory = factory.create_cli_main_worker_factory().await?;
        let mut worker = worker_factory
            .create_custom_worker(
                main_module,
                permissions,
                vec![
                    runtime::ext::my_extension::default()
                ],
                Default::default(),
            )
            .await?;

        let exit_code = worker.run().await?;

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer());

    tracing::subscriber::set_global_default(subscriber)?;

    tracing_log::LogTracer::init()?;

    let cli = Cli::parse();

    deno_runtime::tokio_util::create_and_run_current_thread(cli.do_cli()).unwrap();

    Ok(())
}
