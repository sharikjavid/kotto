use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
pub struct Flags {
    pub paths: Vec<deno_ast::ModuleSpecifier>,
    #[clap(short)]
    pub output: Option<PathBuf>
}
