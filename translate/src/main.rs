mod cli;

use clap::Parser as _;
use cli::Args;
use harvest_ir::HarvestIR;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let name = args.in_performer.to_string_lossy();
    let harvest_ir = HarvestIR::from_raw_source(&args.in_performer)?;
    if let Some(_repr) = harvest_ir::c_ast::C2RustCAst::run_stage(harvest_ir) {
        println!("{name} Done");
    } else {
        panic!("{name} Only reachable if the setup was wrong or casting code is incorrect");
    }
    Ok(())
}
