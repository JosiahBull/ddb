#[macro_use]
extern crate static_assertions;

use clap::{Command, Parser, ValueHint};
use clap_complete::{generate, Generator, Shell};
use std::io::stdout;

pub mod error;
pub mod single;
pub mod threaded;
pub mod utils;

const BLOCK_SIZE: usize = 1024 * 5;
const MIN_BLOCK_SIZE: usize = 512;

const_assert!(BLOCK_SIZE >= MIN_BLOCK_SIZE);
const_assert!(BLOCK_SIZE % MIN_BLOCK_SIZE == 0);
const_assert!(BLOCK_SIZE % 2 == 0);
const_assert!(BLOCK_SIZE < 1024 * 1024 * 1024);

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Dds {
    #[arg(short, long, value_hint = ValueHint::FilePath)]
    pub input: String,
    #[arg(short, long, value_hint = ValueHint::FilePath)]
    pub output: String,

    #[arg(short, long)]
    pub threaded: bool,

    #[arg(long = "generate", hide = true)]
    pub generate: Option<Shell>,
}

pub fn print_completions<G: Generator>(gen: G, cmd: &mut Command) {
    generate(gen, cmd, cmd.get_name().to_string(), &mut stdout());
}

#[cfg(test)]
mod tests {
    use crate::Dds;
    use clap::CommandFactory;

    #[test]
    fn verify_cli() {
        Dds::command().debug_assert();
    }
}
