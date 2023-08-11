use clap::Parser;
use anyhow::Error as AnyError;

use deno_ast::swc::ast;
use deno_ast::swc::visit;
use deno_ast::swc::codegen;
use deno_ast::swc::common;

mod args;
mod util;
mod prompts;
mod tasks;
mod filter;
mod emit;

use args::Flags;

pub trait CanPush<T> {
    fn push(&mut self, item: T);
}

async fn run_subcommand(flags: Flags) -> Result<i32, AnyError> {
    tasks::compile_prompts(&flags.paths).await?;
    Ok(0)
}

fn unwrap_or_exit<T>(result: Result<T, AnyError>) -> T {
    match result {
        Ok(value) => value,
        Err(err) => {
            // TODO
            let error_code = 1;
            eprintln!("{}", err);
            std::process::exit(error_code)
        }
    }
}

#[tokio::main]
async fn main() {
    util::setup_tracing().unwrap();

    let flags = Flags::parse();

    let exit_code = unwrap_or_exit(run_subcommand(flags).await);

    std::process::exit(exit_code)
}