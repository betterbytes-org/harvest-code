use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
pub struct Args {
    // Currently, this is the only input format supported, so --in_performer is
    // required. However, in the future, we'll want to be able to take a
    // different input format that conveys more information (such as the version
    // control history, code review comments, etc). When that format has been
    // defined, we'll add a separate flag to specify it, and change the
    // requirement to "pass either --in_performer or the other input flag".
    /// Path to an input project in the TRACTOR performer format.
    #[arg(long)]
    in_performer: PathBuf,
}
