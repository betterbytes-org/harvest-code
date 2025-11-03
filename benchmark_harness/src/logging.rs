use crate::error::HarvestResult;
use std::path::{Path, PathBuf};

pub fn log_found_programs(program_dirs: &[PathBuf], input_dir: &Path) -> HarvestResult<()> {
    if program_dirs.is_empty() {
        println!("No program directories found in: {}", input_dir.display());
        return Ok(());
    }

    println!(
        "\nFound {} program directories to process:",
        program_dirs.len()
    );
    for dir in program_dirs {
        println!("  - {}", dir.file_name().unwrap().to_string_lossy());
    }

    Ok(())
}
