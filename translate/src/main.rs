mod cli;
mod scheduler;
mod tools;

#[cfg(test)]
mod test_util;

use cli::get_config;
use scheduler::Scheduler;
use tools::{ToolInvocation, load_raw_source};

fn main() {
    if let Err(e) = run() {
        log::error!("{}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    if cli::initialize() {
        return Ok(()); // An early-exit argument was passed.
    }
    let mut scheduler = Scheduler::default();
    scheduler.queue_invocation(ToolInvocation::LoadRawSource(load_raw_source::Args {
        directory: get_config().in_performer.clone(),
    }));
    scheduler.queue_invocation(ToolInvocation::RawSourceToCargoLlm);
    scheduler.main_loop()?;
    let ir = scheduler.ir_snapshot();
    log::info!("{}", ir);

    for (_, representation) in ir.iter() {
        if let repr @ harvest_ir::Representation::CargoPackage(_) = representation {
            repr.materialize(get_config().output.clone())?;
            break;
        }
    }
    Ok(())
}
