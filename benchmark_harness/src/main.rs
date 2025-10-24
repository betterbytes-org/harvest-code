mod cli;
mod error;
mod io;
mod stats;
use crate::cli::*;
use crate::error::HarvestResult;
use crate::io::*;
use crate::stats::*;
use clap::Parser;
use std::path::PathBuf;

// pub fn initialize() -> bool {
//     let args: Arc<_> = Args::parse().into();
//     ARGS.set(args.clone()).expect("cli already initialized");
//     let dirs = ProjectDirs::from("", "", "harvest").expect("no home directory");
//     if args.print_config_path {
//         println!("Config file location: {:?}", config_file(dirs.config_dir()));
//         return true;
//     }
//     let config = load_config(&args, dirs.config_dir());
//     unknown_field_warning("", &config.unknown);
//     config.tools.validate();
//     CONFIG.set(config.into()).expect("cli already initialized");
//     false
// }

pub fn summarize_found_programs(
    program_dirs: &[PathBuf],
    input_dir: &PathBuf,
) -> HarvestResult<()> {
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

pub async fn run_all_examples(
    program_dirs: &[PathBuf],
    output_dir: &PathBuf,
) -> HarvestResult<Vec<ProgramEvalStats>> {
    // Process all examples
    let mut results = Vec::new();
    let total_examples = program_dirs.len();

    for (i, example_dir) in program_dirs.iter().enumerate() {
        println!("\n{}", "=".repeat(80));
        println!("Processing example {} of {}", i + 1, total_examples);
        println!("{}", "=".repeat(80));

        let result = process_single_example(example_dir, output_dir).await;

        results.push(result);
    }

    Ok(results)
}

/// Process a single example program directory
/// TODO: Implement actual translation and testing logic
async fn process_single_example(program_dir: &PathBuf, output_dir: &PathBuf) -> ProgramEvalStats {
    unimplemented!()
    // let program_name = program_dir
    //     .file_name()
    //     .unwrap_or_default()
    //     .to_string_lossy()
    //     .to_string();

    // println!("Processing program: {}", program_name);

    // // TODO: Implement actual translation logic using harvest_translate
    // // TODO: Implement actual testing logic
    // // For now, return a placeholder result

    // ProgramEvalStats {
    //     program_name,
    //     translation_success: false, // TODO: Actually attempt translation
    //     rust_build_success: false,  // TODO: Actually attempt build
    //     total_tests: 0,             // TODO: Count actual tests
    //     passed_tests: 0,            // TODO: Run actual tests
    //     error_message: Some("Not yet implemented".to_string()),
    //     individual_test_results: Vec::new(),
    // }
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
    summarize_found_programs(&program_dirs, &args.input_dir)?;

    // Process all programs
    let results = run_all_examples(&program_dirs, &args.output_dir).await?;

    unimplemented!();
    // // TODO: Generate summary statistics and write results to CSV
    // println!("\nProcessed {} programs", results.len());
    // for result in &results {
    //     println!("  {}: Translation: {}, Build: {}, Tests: {}/{}",
    //         result.program_name,
    //         result.translation_success,
    //         result.rust_build_success,
    //         result.passed_tests,
    //         result.total_tests
    //     );
    // }

    Ok(())
}
