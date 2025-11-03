use clap::Parser;
use harvest_translate::cli::{self, Args};
use harvest_translate::transpile;
use std::sync::Arc;

fn main() {
    if let Err(e) = run() {
        log::error!("{}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let args: Arc<_> = Args::parse().into();
    let Some(config) = cli::initialize(args) else {
        return Ok(()); // An early-exit argument was passed.
    };
    let ir = transpile(config)?;
    log::info!("{}", ir);
    Ok(())
}
