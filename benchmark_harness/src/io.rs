use crate::error::HarvestResult;
use crate::stats::*;
use std::error::Error;
use std::path::{Path, PathBuf};

/// Write CSV results to file
pub fn write_csv_results(file_path: &PathBuf, results: &[ProgramEvalStats]) -> HarvestResult<()> {
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

pub fn log_summary_stats(summary: &SummaryStats) {
    // Print final summary
    println!("\n{}", "=".repeat(80));
    println!("FINAL SUMMARY - All Benchmark Processing Complete!");
    println!("{}", "=".repeat(80));

    println!("\nSummary Statistics:");
    println!("  Total programs processed: {}", summary.num_programs);
    println!(
        "  Successful translations: {} ({:.1}%)",
        summary.successful_translations,
        summary.translation_success_rate()
    );
    println!(
        "  Successful Rust builds: {} ({:.1}%)",
        summary.successful_rust_builds,
        summary.rust_build_success_rate()
    );

    println!("\nTest Results:");
    println!("  Total test cases: {}", summary.total_tests);
    println!("  Passed: {} ✅", summary.total_passed_tests);
    println!(
        "  Failed: {} ❌",
        summary.total_tests - summary.total_passed_tests
    );
    println!(
        "  Overall success rate: {:.1}%",
        summary.overall_success_rate()
    );
}

/// Log programs that have issues (translation failures, build failures, or test failures)
pub fn log_failing_programs(results: &[ProgramEvalStats]) {
    let failed_examples: Vec<_> = results
        .iter()
        .filter(|r| !r.translation_success || !r.rust_build_success || r.success_rate() < 100.0)
        .collect();

    if !failed_examples.is_empty() {
        println!("\n⚠️  Examples with issues:");
        for example in failed_examples {
            println!("  - {}: ", example.program_name);
            if !example.translation_success {
                println!("    ❌ Translation failed");
            }
            if !example.rust_build_success {
                println!("    ❌ Rust build failed");
            }
            if example.success_rate() < 100.0 && example.total_tests > 0 {
                println!(
                    "    ⚠️  Tests: {}/{} passed ({:.1}%)",
                    example.passed_tests,
                    example.total_tests,
                    example.success_rate()
                );
            }
        }
    }
}
