use crate::error::HarnessResult;
use crate::stats::*;
use std::path::PathBuf;

/// Write CSV results to file
fn write_csv_results(file_path: &PathBuf, results: &[ProgramEvalStats]) -> HarnessResult<()> {
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
