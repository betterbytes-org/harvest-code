mod cli;
mod scheduler;
mod tools;

use clap::Parser as _;
use cli::Args;
use scheduler::Scheduler;
use tools::{ToolInvocation, load_raw_source};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let mut scheduler = Scheduler::default();
    scheduler.queue_invocation(ToolInvocation::LoadRawSource(load_raw_source::Args {
        directory: args.in_performer,
    }));
    scheduler.queue_invocation(ToolInvocation::RawSourceToCargoLlm);
    scheduler.main_loop();
    println!("{}", scheduler.ir_snapshot());
    Ok(())
}
