use harvest_translate::transpile;
use harvest_translate::{cli, runner};

fn main() {
    if let Err(e) = run() {
        log::error!("{}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let Some(config) = cli::initialize() else {
        return Ok(()); // An early-exit argument was passed.
    };
    let ir = transpile(config)?;
    log::info!("{}", ir);
    Ok(())
}
