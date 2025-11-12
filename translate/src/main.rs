use clap::Parser;
use harvest_translate::{Args, initialize, transpile};
use log::{error, info};
use std::sync::Arc;

fn main() {
    if let Err(e) = run() {
        error!("{}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let args: Arc<_> = Args::parse().into();
    let Some(config) = initialize(args) else {
        return Ok(()); // An early-exit argument was passed.
    };
    let ir = transpile(config)?;
    info!("{}", ir);
    Ok(())
}
