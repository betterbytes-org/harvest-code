mod cli;
mod scheduler;
mod tools;

#[cfg(test)]
mod test_util;

use cli::get_config;
use harvest_ir::edit;
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
    let mut ir_organizer = edit::Organizer::default();
    let mut scheduler = Scheduler::default();
    scheduler.queue_invocation(ToolInvocation::LoadRawSource(load_raw_source::Args {
        directory: get_config().in_performer.clone(),
    }));
    scheduler.queue_invocation(ToolInvocation::RawSourceToCargoLlm);
    scheduler.queue_invocation(ToolInvocation::TryCargoBuild);
    scheduler.main_loop(&mut ir_organizer)?;
    let ir = ir_organizer.snapshot();
    log::info!("{}", ir);
    Ok(())
}
