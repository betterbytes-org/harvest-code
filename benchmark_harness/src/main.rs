mod cli;
mod error;
mod harness;
mod io;
mod logging;
mod runner;
mod stats;
use crate::cli::*;
use crate::error::HarvestResult;
use crate::harness::*;
use crate::io::*;
use crate::logging::*;
use crate::stats::*;
use clap::Parser;
use harvest_ir::fs::RawDir;
use harvest_ir::HarvestIR;
use harvest_ir::Representation;
use harvest_translate::cli::initialize;
use harvest_translate::transpile;
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn raw_cargo_package(ir: &HarvestIR) -> HarvestResult<&RawDir> {
    let cargo_packages: Vec<&RawDir> = ir
        .iter()
        .filter_map(|(_, repr)| match repr {
            Representation::CargoPackage(r) => Some(r),
            _ => None,
        })
        .collect();

    match cargo_packages.len() {
        0 => Err("No CargoPackage representation found in IR".into()),
        1 => Ok(cargo_packages[0]),
        n => Err(format!(
            "Found {} CargoPackage representations, expected at most 1",
            n
        )
        .into()),
    }
}
// Result<Vec<PathBuf>, String>
fn cargo_build_result(ir: &HarvestIR) -> Result<Vec<PathBuf>, String> {
    let build_results: Vec<Result<Vec<PathBuf>, String>> = ir
        .iter()
        .filter_map(|(_, repr)| match repr {
            Representation::CargoBuildResult(r) => Some(r.clone()),
            _ => None,
        })
        .collect();

    match build_results.len() {
        0 => Err("No artifacts built".into()),
        1 => build_results[0].clone(),
        n => Err(format!("Found {} build results, expected at most 1", n).into()),
    }
}

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
    let test_cases = match parse_test_vectors(test_vectors_dir) {
        Ok(vectors) => vectors,
        Err(e) => {
            result.error_message = Some(e.to_string());
            error_messages.push(e.to_string());
            return result;
        }
    };

    // Log test case parsing success
    if test_cases.len() > 0 {
        println!("✅ Successfully parsed {} test case(s)", test_cases.len());
    }

    // Do the actual translation
    let translation_result =
        translate_c_directory_to_rust_project(&test_case_src_dir, &output_dir, config_overrides)
            .await;

    result.translation_success = translation_result.translation_success;

    if translation_result.translation_success {
        println!("✅ Translation completed successfully!");
    } else {
        let error = format!(
            "Failed to translate C project: {:?}",
            translation_result.build_error
        );
        result.error_message = Some(error.clone());
        error_messages.push(error);
        println!("❌ Translation failed");
        return result;
    }

    // Step 5: run program against test cases
    println!("Validating Rust binary outputs against test cases...");

    assert!(translation_result.rust_binary_path.exists());

    // Run validation tests
    for (i, test_case) in test_cases.iter().enumerate() {
        println!(
            "Running test case {} ({} of {})...",
            test_case.filename,
            i + 1,
            test_cases.len()
        );

        println!(
            "Validating output for test case with args: {:?} stdin: {:?}",
            test_case.argv, test_case.stdin,
        );
        // TODO: make timeout configurable
        let timeout = Some(10);
        match validate_binary_output(&translation_result.rust_binary_path, test_case, timeout) {
            Ok(()) => {
                result.passed_tests += 1;
                result.test_results.push(TestResult {
                    filename: test_case.filename.clone(),
                    passed: true,
                });
                println!("✅ Test case {} passed", test_case.filename);
            }
            Err(e) => {
                result.test_results.push(TestResult {
                    filename: test_case.filename.clone(),
                    passed: false,
                });
                let error = format!("Test case {} failed: {}", test_case.filename, e);
                error_messages.push(error);
                println!("❌ Test case {} failed: {}", test_case.filename, e);
            }
        }
    }

    // Print summary for this example
    println!("\nResults for {}:", program_name);
    println!(
        "  Translation: {}",
        if result.translation_success {
            "✅"
        } else {
            "❌"
        }
    );
    println!(
        "  Rust Build: {}",
        if result.rust_build_success {
            "✅"
        } else {
            "❌"
        }
    );
    println!(
        "  Tests: {}/{} passed ({:.1}%)",
        result.passed_tests,
        result.total_tests,
        result.success_rate()
    );

    result
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
