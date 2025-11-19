//! A framework for translating C code into Rust code. This is normally used through the
//! `translate` binary, but is exposed as a library crate as well.

pub mod cli;
mod diagnostics;
mod runner;
mod scheduler;
pub mod tools;
pub mod util;

#[cfg(test)]
mod test_util;

use crate::load_raw_source::LoadRawSource;
use crate::tools::raw_source_to_cargo_llm::RawSourceToCargoLlm;
use crate::tools::try_cargo_build::TryCargoBuild;
use crate::tools::{MightWriteContext, MightWriteOutcome};
use harvest_ir::HarvestIR;
use harvest_ir::edit::{self, NewEditError};
use runner::{SpawnToolError, ToolRunner};
use scheduler::Scheduler;
use std::sync::Arc;
use tools::load_raw_source;
use tracing::{debug, error, info};

/// Performs the complete transpilation process using the scheduler.
pub fn transpile(config: Arc<cli::Config>) -> Result<Arc<HarvestIR>, Box<dyn std::error::Error>> {
    let collector = diagnostics::Collector::initialize(&config)?;
    let mut ir_organizer = edit::Organizer::default();
    let mut runner = ToolRunner::new(collector.reporter());
    let mut scheduler = Scheduler::default();
    scheduler.queue_invocation(LoadRawSource::new(&config.input));
    scheduler.queue_invocation(RawSourceToCargoLlm);
    scheduler.queue_invocation(TryCargoBuild);
    loop {
        let snapshot = ir_organizer.snapshot();
        scheduler.next_invocations(|mut tool| {
            let name = tool.name();
            let might_write = match tool.might_write(MightWriteContext { ir: &snapshot }) {
                MightWriteOutcome::NotRunnable => {
                    debug!("Tool {name} is not runnable");
                    return None;
                }
                MightWriteOutcome::Runnable(might_write) => {
                    debug!("Tool {name} is runnable");
                    might_write
                }
                MightWriteOutcome::TryAgain => {
                    debug!("Tool {name} returned TryAgain");
                    return Some(tool);
                }
            };
            match runner.spawn_tool(
                &mut ir_organizer,
                tool,
                snapshot.clone(),
                might_write,
                config.clone(),
            ) {
                Err((SpawnToolError::CollectorDropped(_), _)) => panic!("collector dropped?"),
                Err((SpawnToolError::NewEdit(NewEditError::IdInUse), tool)) => {
                    debug!("Not spawning {name} because an ID it needs is in use.");
                    Some(tool)
                }
                Err((SpawnToolError::NewEdit(NewEditError::UnknownId), _)) => {
                    error!("Tool {name}: might_write returned an unknown ID");
                    None
                }
                Ok(()) => {
                    info!("Launched tool {name}");
                    None
                }
            }
        });
        if !runner.process_tool_results(&mut ir_organizer) {
            // No tools are running now, which also indicates that no tools are schedulable.
            // Eventually we need some way to determine whether this is a successful outcome or a
            // failure, but for now we can just assume success.
            break;
        }
    }
    collector.diagnostics(); // TODO: Return this value (see issue 51)
    Ok(ir_organizer.snapshot())
}
