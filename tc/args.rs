use clap::Parser;

use std::path::PathBuf;

#[derive(Parser, Debug)]
pub struct Flags {
    pub paths: Vec<PathBuf>
}
