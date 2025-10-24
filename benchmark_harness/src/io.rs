use crate::error::HarvestResult;
use crate::stats::*;
use std::error::Error;
use std::path::PathBuf;

/// Write CSV results to file
fn write_csv_results(file_path: &PathBuf, results: &[ProgramEvalStats]) -> HarvestResult<()> {
    let mut wtr = csv::Writer::from_path(file_path)?;

    // Write header
    wtr.write_record(&[
        "program_name",
        "translation_success",
        "rust_build_success",
        "total_tests",
        "passed_tests",
        "success_rate",
        "error_message",
    ])?;

    // Write data
    for result in results {
        wtr.write_record(&[
            &result.program_name,
            &result.translation_success.to_string(),
            &result.rust_build_success.to_string(),
            &result.total_tests.to_string(),
            &result.passed_tests.to_string(),
            &format!("{:.2}", result.success_rate()),
            result.error_message.as_deref().unwrap_or(""),
        ])?;
    }

    wtr.flush()?;
    Ok(())
}

pub fn validate_input_directory(input_dir: &PathBuf) -> HarvestResult<()> {
    if !input_dir.exists() || !input_dir.is_dir() {
        return Err(format!(
            "Input directory does not exist or is not a directory: {}",
            input_dir.display()
        )
        .into());
    }
    Ok(())
}

/// Create output directory if it doesn't exist
pub fn ensure_output_directory(output_dir: &PathBuf) -> HarvestResult<()> {
    if !output_dir.exists() {
        std::fs::create_dir_all(output_dir).map_err(|e| -> Box<dyn std::error::Error> {
            format!(
                "Failed to create output directory: {}. Error: {}",
                output_dir.display(),
                e
            )
            .into()
        })?;
    }
    Ok(())
}

/// Find all immediate child directories in the input directory
pub fn collect_program_dirs(input_dir: &PathBuf) -> HarvestResult<Vec<PathBuf>> {
    let mut program_dirs = Vec::new();
    for entry in std::fs::read_dir(input_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            program_dirs.push(path);
        }
    }
    // Standardize order before returning
    program_dirs.sort();
    Ok(program_dirs)
}
