mod cli;
mod runner;
mod scheduler;
mod tools;
mod util;

#[cfg(test)]
mod test_util;

use cli::get_config;
use harvest_ir::edit::{self, NewEditError};
use log::{debug, error, info};
use runner::{SpawnToolError, ToolRunner};
use scheduler::Scheduler;
use tools::MightWriteContext;
use tools::MightWriteOutcome;
use tools::Tool;
use tools::load_raw_source::LoadRawSource;
use tools::raw_source_to_cargo_llm::RawSourceToCargoLlm;
use tools::try_cargo_build::TryCargoBuild;
use util::empty_writable_dir;

fn main() {
    if let Err(e) = run() {
        error!("{}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    if cli::initialize() {
        return Ok(()); // An early-exit argument was passed.
    }
    let config = get_config();
    empty_writable_dir(&config.output, config.delete_output_contents)
        .expect("output directory error");
    let mut ir_organizer = edit::Organizer::default();
    let mut runner = ToolRunner::default();
    let mut scheduler = Scheduler::default();
    scheduler.queue_invocation(LoadRawSource::new(&config.input.clone()));
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
            match runner.spawn_tool(&mut ir_organizer, tool, snapshot.clone(), might_write) {
                Err(SpawnToolError {
                    cause: NewEditError::IdInUse,
                    tool,
                }) => {
                    debug!("Not spawning {name} because an ID it needs is in use.");
                    Some(tool)
                }
                Err(SpawnToolError {
                    cause: NewEditError::UnknownId,
                    tool: _,
                }) => {
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
    let ir = ir_organizer.snapshot();
    info!("{}", ir);
    Ok(())
}
