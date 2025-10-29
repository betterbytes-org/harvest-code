mod cli;
mod runner;
mod scheduler;
mod tools;

#[cfg(test)]
mod test_util;

use cli::get_config;
use harvest_ir::edit::{self, NewEditError};
use runner::ToolRunner;
use scheduler::{InvocationOutcome, Scheduler};
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
    let mut runner = ToolRunner::default();
    let mut scheduler = Scheduler::default();
    scheduler.queue_invocation(ToolInvocation::LoadRawSource(load_raw_source::Args {
        directory: get_config().input.clone(),
    }));
    scheduler.queue_invocation(ToolInvocation::RawSourceToCargoLlm);
    scheduler.queue_invocation(ToolInvocation::TryCargoBuild);
    loop {
        let snapshot = ir_organizer.snapshot();
        scheduler.next_invocations(|invocation| {
            let mut tool = invocation.create_tool();
            let Some(might_write) = tool.might_write(&snapshot) else {
                // TODO: Add a tool name to the `Tool` trait so that we can output a message like
                // "Tool X not currently runnable").
                return InvocationOutcome::Wait;
            };
            match runner.spawn_tool(&mut ir_organizer, tool, snapshot.clone(), might_write) {
                Err(NewEditError::IdInUse) => InvocationOutcome::Wait,
                Err(NewEditError::UnknownId) => {
                    // TODO: Tool name for diagnostics.
                    log::error!("Tool::might_write returned an unknown ID");
                    InvocationOutcome::Discard
                }
                Ok(()) => InvocationOutcome::Success,
            }
        });
        if !runner.process_tool_results(&mut ir_organizer) {
            // No tools are running now, which also indicates that no tools are schedulable.
            // Eventually we need some way to determine whether this is a successful outcome or a
            // failure, but for now we can just assume success.
            break;
        }
    }
    let ir = ir_organizer.snapshot();
    log::info!("{}", ir);
    Ok(())
}
