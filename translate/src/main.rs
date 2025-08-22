mod cli;

use clap::Parser as _;
use cli::Args;
use harvest_ir::{HarvestIR, Id, Representation};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let harvest_ir = HarvestIR::from_raw_source(args.in_performer)?;
    println!("{harvest_ir}");
    if let Some(Representation::RawSource(rd)) = harvest_ir.get(&Id(0)) {
        println!("{:?}", harvest_ir::c_ast::CAst::populate_from(rd));
    } else {
        panic!("WTF?");
    }
    Ok(())
}
