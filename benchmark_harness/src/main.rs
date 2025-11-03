mod cli;
mod error;
mod harness;
mod io;
mod logging;
mod stats;
use crate::cli::*;
use crate::error::HarvestResult;
use crate::harness::*;
use crate::io::*;
use crate::logging::*;
use crate::stats::*;
use clap::Parser;
use harvest_ir::HarvestIR;
use harvest_translate::cli::initialize;
use harvest_translate::transpile;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub async fn translate_c_directory_to_rust_project(
    input_dir: &Path,
    output_dir: &Path,
    config_overrides: &[String],
) -> HarvestResult<Arc<HarvestIR>> {
    let args: Arc<harvest_translate::cli::Args> = harvest_translate::cli::Args {
        input: Some(input_dir.to_path_buf()),
        output: Some(output_dir.to_path_buf()),
        print_config_path: false,
        config: config_overrides.to_vec(),
    }
    .into();
    let config = initialize(args).expect("Failed to generate config");
    transpile(config)
}

// TODO: switch println! to proper logging

pub async fn run_all_benchmarks(
    program_dirs: &[PathBuf],
    output_dir: &PathBuf,
    config_overrides: &[String],
) -> HarvestResult<Vec<ProgramEvalStats>> {
    // Process all examples
    let mut results = Vec::new();
    let total_examples = program_dirs.len();

    for (i, program_dir) in program_dirs.iter().enumerate() {
        println!("\n{}", "=".repeat(80));
        println!("Processing example {} of {}", i + 1, total_examples);
        println!("{}", "=".repeat(80));

        let result = benchmark_single_program(program_dir, output_dir, config_overrides).await;

        results.push(result);
    }

    Ok(results)
}

/// Run all benchmarks for a single program
async fn benchmark_single_program(
    program_dir: &PathBuf,
    output_root_dir: &PathBuf,
    config_overrides: &[String],
) -> ProgramEvalStats {
    let program_name = program_dir
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    // TODO: switch to implementation that doesn't need mutable variable
    let mut result = ProgramEvalStats {
        program_name: program_name.clone(),
        translation_success: false, // TODO: Actually attempt translation
        rust_build_success: false,  // TODO: Actually attempt build
        total_tests: 0,             // TODO: Count actual tests
        passed_tests: 0,            // TODO: Run actual tests
        error_message: None,
        test_results: Vec::new(),
    };

    let mut error_messages = Vec::new();

    println!("Translating program: {}", program_name);
    println!("Input directory: {}", program_dir.display());

    // get program output directory
    let output_dir = output_root_dir.join(&program_name);
    println!("Output directory: {}", output_dir.display());

    // Check for required subdirectories & log error if we don't find them
    let (test_case_src_dir, test_vectors_dir) = match parse_benchmark_dir(program_dir) {
        Ok(dirs) => dirs,
        Err(e) => {
            result.error_message = Some(e.to_string());
            error_messages.push(e.to_string());
            return result;
        }
    };

    // Parse test vectors
    let test_vectors = match parse_test_vectors(test_vectors_dir) {
        Ok(vectors) => vectors,
        Err(e) => {
            result.error_message = Some(e.to_string());
            error_messages.push(e.to_string());
            return result;
        }
    };

    // Log test case parsing success
    if test_vectors.len() > 0 {
        println!("✅ Successfully parsed {} test case(s)", test_vectors.len());
    }

    // Do the actual translation
    let translation_success = match translate_c_directory_to_rust_project(
        &test_case_src_dir,
        &output_dir,
        config_overrides,
    )
    .await
    {
        Ok(_) => {
            result.translation_success = true;
            println!("✅ Translation completed successfully!");
        }
        Err(e) => {
            let error = format!("Failed to translate C project: {}", e);
            result.error_message = Some(error.clone());
            error_messages.push(error);
            println!("❌ Translation failed");
        }
    };

    unimplemented!()
    // Step 5: run program against test cases

    // TODO: Implement actual translation logic using harvest_translate
    // TODO: Implement actual testing logic
    // For now, return a placeholder result
}

#[tokio::main]
async fn main() -> HarvestResult<()> {
    let args = Args::parse();

    // Validate input directory exists
    validate_input_directory(&args.input_dir)?;

    // Create output directory if it doesn't exist
    ensure_output_directory(&args.output_dir)?;
    run(args).await
}

async fn run(args: Args) -> HarvestResult<()> {
    println!("Running Benchmarks");
    println!("Input directory: {}", args.input_dir.display());
    println!("Output directory: {}", args.output_dir.display());

    // Get the programs to evaluate
    // Should be in directories that are immediate children of input_dir
    let program_dirs = collect_program_dirs(&args.input_dir)?;
    log_found_programs(&program_dirs, &args.input_dir)?;

    // Process all programs
    let results = run_all_benchmarks(&program_dirs, &args.output_dir, &args.config).await?;
    let csv_output_path = args.output_dir.join("results.csv");
    write_csv_results(&csv_output_path, &results)?;

    let summary_stats = SummaryStats::from_results(&results);
    log_summary_stats(&summary_stats);

    println!("\nOutput Files:");
    println!(
        "  CSV results (original format): {}",
        csv_output_path.display()
    );
    println!("  Translated projects: {}", args.output_dir.display());
    println!("  Error logs: results.err files in each translated project directory");

    // Print examples with issues
    log_failing_programs(&results);

    println!("\nProcessing complete! Check the CSV file and individual project directories for detailed results.");

    cleanup_benchmarks(&results, &args.output_dir);

    Ok(())
}
