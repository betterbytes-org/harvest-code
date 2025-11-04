mod cli;
mod error;
mod harness;
mod io;
mod ir_utils;
mod logger;
mod runner;
mod stats;
use crate::cli::*;
use crate::error::HarvestResult;
use crate::harness::*;
use crate::io::*;
use crate::ir_utils::{cargo_build_result, raw_cargo_package, raw_source};
use crate::logger::TeeLogger;
use crate::stats::*;
use clap::Parser;
use harvest_ir::HarvestIR;
use harvest_translate::cli::initialize;
use harvest_translate::transpile;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct TranspilationResult {
    translation_success: bool,
    build_success: bool,
    rust_binary_path: PathBuf,
    build_error: Option<String>,
}

impl TranspilationResult {
    pub fn from_ir(ir: &HarvestIR) -> Self {
        let translation_success = raw_cargo_package(ir).is_ok();
        let (build_success, rust_binary_path, build_error) = match cargo_build_result(ir) {
            Ok(artifacts) => (true, artifacts[0].clone(), None), // should check that there is only 1 artifact
            Err(err) => (false, PathBuf::new(), Some(err)),
        };

        Self {
            translation_success,
            build_success,
            rust_binary_path,
            build_error,
        }
    }
}

// Needs to return path to
pub async fn translate_c_directory_to_rust_project(
    input_dir: &Path,
    output_dir: &Path,
    config_overrides: &[String],
) -> TranspilationResult {
    let args: Arc<harvest_translate::cli::Args> = harvest_translate::cli::Args {
        input: Some(input_dir.to_path_buf()),
        output: Some(output_dir.to_path_buf()),
        print_config_path: false,
        config: config_overrides.to_vec(),
    }
    .into();
    let config = initialize(args).expect("Failed to generate config");
    let ir_result = transpile(config);
    let raw_c_source = raw_source(&ir_result.as_ref().unwrap()).unwrap();
    raw_c_source
        .materialize(&output_dir.join("c_src"))
        .expect("Failed to materialize C source");

    match ir_result {
        Ok(ir) => TranspilationResult::from_ir(&ir),
        Err(_) => TranspilationResult {
            translation_success: false,
            build_success: false,
            rust_binary_path: PathBuf::new(),
            build_error: Some("Failed to transpile".to_string()),
        },
    }
}

pub async fn run_all_benchmarks(
    program_dirs: &[PathBuf],
    output_dir: &Path,
    config_overrides: &[String],
    timeout: u64,
) -> HarvestResult<Vec<ProgramEvalStats>> {
    // Process all examples
    let mut results = Vec::new();
    let total_examples = program_dirs.len();

    for (i, program_dir) in program_dirs.iter().enumerate() {
        log::error!("\n{}", "=".repeat(80));
        log::info!("Processing example {} of {}", i + 1, total_examples);
        log::info!("{}", "=".repeat(80));

        let result =
            benchmark_single_program(program_dir, output_dir, config_overrides, timeout).await;

        results.push(result);
    }

    Ok(results)
}

/// Run all benchmarks for a single program
async fn benchmark_single_program(
    program_dir: &PathBuf,
    output_root_dir: &Path,
    config_overrides: &[String],
    timeout: u64,
) -> ProgramEvalStats {
    let program_name = program_dir
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let mut result = ProgramEvalStats {
        program_name: program_name.clone(),
        translation_success: false,
        rust_build_success: false,
        total_tests: 0,
        passed_tests: 0,
        error_message: None,
        test_results: Vec::new(),
    };

    let mut error_messages = Vec::new();

    log::info!("Translating program: {}", program_name);
    log::info!("Input directory: {}", program_dir.display());

    // get program output directory
    let output_dir = output_root_dir.join(&program_name);
    log::info!("Output directory: {}", output_dir.display());

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
    let test_cases = match parse_test_vectors(test_vectors_dir) {
        Ok(vectors) => vectors,
        Err(e) => {
            result.error_message = Some(e.to_string());
            error_messages.push(e.to_string());
            return result;
        }
    };

    result.total_tests = test_cases.len();

    // Log test case parsing success
    if !test_cases.is_empty() {
        log::info!("✅ Successfully parsed {} test case(s)", test_cases.len());
    }

    // Do the actual translation
    let translation_result =
        translate_c_directory_to_rust_project(&test_case_src_dir, &output_dir, config_overrides)
            .await;

    result.translation_success = translation_result.translation_success;
    result.rust_build_success = translation_result.build_success;

    if translation_result.translation_success {
        log::info!("✅ Translation completed successfully!");
    } else {
        let error = format!(
            "Failed to translate C project: {:?}",
            translation_result.build_error
        );
        result.error_message = Some(error.clone());
        error_messages.push(error);
        log::info!("❌ Translation failed");
        return result;
    }

    if translation_result.build_success {
        log::info!("✅ Rust build completed successfully!");
    } else {
        let error = format!(
            "Failed to build Rust project: {:?}",
            translation_result.build_error
        );
        result.error_message = Some(error.clone());
        error_messages.push(error);
        log::info!("❌ Rust build failed");
        return result;
    }

    assert!(translation_result.rust_binary_path.exists());

    // Step 5: run program against test cases
    log::info!("Validating Rust binary outputs against test cases...");

    // Run validation tests
    for (i, test_case) in test_cases.iter().enumerate() {
        log::info!(
            "Running test case {} ({} of {})...",
            test_case.filename,
            i + 1,
            test_cases.len()
        );

        log::info!(
            "Validating output for test case with args: {:?} stdin: {:?}",
            test_case.argv,
            test_case.stdin,
        );
        let timeout_opt = Some(timeout);
        match validate_binary_output(&translation_result.rust_binary_path, test_case, timeout_opt) {
            Ok(()) => {
                result.passed_tests += 1;
                result.test_results.push(TestResult {
                    filename: test_case.filename.clone(),
                    passed: true,
                });
                log::info!("✅ Test case {} passed", test_case.filename);
            }
            Err(e) => {
                result.test_results.push(TestResult {
                    filename: test_case.filename.clone(),
                    passed: false,
                });
                let error = format!("Test case {} failed: {}", test_case.filename, e);
                error_messages.push(error);
                log::info!("❌ Test case {} failed: {}", test_case.filename, e);
                // TODO: write to error file
                test_case
                    .write_to_disk(&output_dir)
                    .expect("failed to write test case to disk");
            }
        }
    }

    // Print summary for this example
    log::info!("\nResults for {}:", program_name);
    log::info!(
        "  Translation: {}",
        if result.translation_success {
            "✅"
        } else {
            "❌"
        }
    );
    log::info!(
        "  Rust Build: {}",
        if result.rust_build_success {
            "✅"
        } else {
            "❌"
        }
    );
    log::info!(
        "  Tests: {}/{} passed ({:.1}%)",
        result.passed_tests,
        result.total_tests,
        result.success_rate()
    );

    // Write error messages to results.err file in the output directory if it was created
    if !error_messages.is_empty() {
        let error_file_path = output_dir.join("results.err");
        if let Err(e) = write_error_file(&error_file_path, &error_messages) {
            log::info!("Warning: Failed to write error file: {}", e);
        }
    }

    result
}

#[tokio::main]
async fn main() -> HarvestResult<()> {
    let args = Args::parse();

    // Validate input directory exists
    validate_input_directory(&args.input_dir)?;

    // Create output directory if it doesn't exist
    ensure_output_directory(&args.output_dir)?;

    let log_file = File::create(args.output_dir.join("output.log"))?;
    TeeLogger::init(log::LevelFilter::Info, log_file)?;
    run(args).await
}

async fn run(args: Args) -> HarvestResult<()> {
    log::info!("Running Benchmarks");
    log::info!("Input directory: {}", args.input_dir.display());
    log::info!("Output directory: {}", args.output_dir.display());

    // Get the programs to evaluate
    // Should be in directories that are immediate children of input_dir
    let program_dirs = collect_program_dirs(&args.input_dir)?;
    log_found_programs(&program_dirs, &args.input_dir)?;

    // Process all programs
    let results =
        run_all_benchmarks(&program_dirs, &args.output_dir, &args.config, args.timeout).await?;
    let csv_output_path = args.output_dir.join("results.csv");
    write_csv_results(&csv_output_path, &results)?;

    let summary_stats = SummaryStats::from_results(&results);
    log_summary_stats(&summary_stats);

    log::info!("\nOutput Files:");
    log::info!("  Translated projects: {}", args.output_dir.display());
    log::info!("  CSV results: {}", csv_output_path.display());
    log::info!("  Error logs: results.err files in each translated project directory");

    // Print examples with issues
    log_failing_programs(&results);

    log::info!("\nProcessing complete! Check the CSV file and individual project directories for detailed results.");

    cleanup_benchmarks(&results, &args.output_dir);

    Ok(())
}
