mod cli;

use clap::Parser as _;
use cli::Args;
use harvest_ir::HarvestIR;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let harvest_ir = HarvestIR::from_raw_source(args.in_performer)?;
    harvest_ir.display();
    Ok(())
}
