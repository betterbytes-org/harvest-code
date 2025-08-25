mod cli;

use clap::Parser as _;
use cli::Args;
use harvest_ir::HarvestIR;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let harvest_ir = HarvestIR::from_raw_source(args.in_performer)?;
    println!("{harvest_ir}");
    if let Some(repr) = harvest_ir::c_ast::CAst::run_stage(harvest_ir) {
        println!("{repr:?}");
    } else {
        panic!("WTF?");
    }
    Ok(())
}
