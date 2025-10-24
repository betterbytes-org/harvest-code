mod error;
mod io;
mod stats;
mod cli;
use crate::error::HarnessResult;
use crate::io::*;
use crate::stats::*;
use crate::cli::*;

#[tokio::main]
async fn main() {}

fn run() -> HarnessResult<()> {
    Ok(())
}
