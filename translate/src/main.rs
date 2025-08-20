mod cli;

use clap::Parser as _;
use cli::Args;

fn main() {
    let _args = Args::parse();
}
