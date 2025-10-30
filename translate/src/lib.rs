pub mod cli;
pub mod scheduler;
pub mod tools;

#[cfg(test)]
mod test_util;

use scheduler::Scheduler;
use tools::{ToolInvocation, load_raw_source};

/// Performs the complete transpilation process using the scheduler.
///
/// This function sets up a scheduler with the necessary tool invocations
/// to load raw source, convert to Cargo LLM format, attempt a build,
/// and return the IR snapshot.
pub fn transpile(config: std::sync::Arc<cli::Config>) -> Result<(), Box<dyn std::error::Error>> {
    let mut scheduler = Scheduler::default();
    scheduler.set_config(config.clone());
    scheduler.queue_invocation(ToolInvocation::LoadRawSource(load_raw_source::Args {
        directory: config.in_performer.clone(),
    }));
    scheduler.queue_invocation(ToolInvocation::RawSourceToCargoLlm);
    scheduler.queue_invocation(ToolInvocation::TryCargoBuild);
    scheduler.main_loop()?;
    let ir = scheduler.ir_snapshot();
    log::info!("{}", ir);
    Ok(())
}
