use crate::runner;
use crate::stats::ProgramEvalStats;
use crate::HarvestResult;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
/// This module `harness` is intented to contain code that is specific to a particular set of benchmarks,
/// for example, parsing code for benchmark-specific configs.
/// Currently, that is just the MITLL tractor benchmarks.
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

/// Represents the expected stdout pattern in a test case
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StdoutPattern {
    pub pattern: String,
    #[serde(default)]
    pub is_regex: bool,
}

impl Default for StdoutPattern {
    fn default() -> Self {
        Self {
            pattern: String::new(),
            is_regex: false,
        }
    }
}

/// Represents a test case with command arguments, input, and expected output
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TestCase {
    #[serde(default)]
    pub argv: Vec<String>,
    #[serde(default)]
    pub stdin: Option<String>,
    #[serde(default)]
    pub stdout: StdoutPattern,
    #[serde(default)]
    pub rc: Option<usize>,
    #[serde(default)]
    pub has_ub: Option<String>,
    #[serde(skip)] // Don't serialize/deserialize this field as it's not part of the JSON
    pub filename: String,
}

impl Default for TestCase {
    fn default() -> Self {
        Self {
            argv: Vec::new(),
            stdin: None,
            stdout: StdoutPattern::default(),
            rc: None,
            has_ub: None,
            filename: String::new(),
        }
    }
}

/// Parses a JSON string into a TestCase struct
pub fn parse_test_case_json(json_str: &str) -> HarvestResult<TestCase> {
    let mut test_case: TestCase = serde_json::from_str(json_str)
        .map_err(|e| format!("Failed to parse test case JSON: {}", e))?;
    // Initialize filename to empty string - it should be set by the caller
    if test_case.filename.is_empty() {
        test_case.filename = String::new();
    }
    Ok(test_case)
}

/// Validate that required benchmark subdirectories exist
/// Returns paths to (input/test_case/src, input/test_vectors)
pub fn parse_benchmark_dir(input_dir: &PathBuf) -> HarvestResult<(PathBuf, PathBuf)> {
    if !input_dir.exists() {
        return Err(format!("Input directory does not exist: {}", input_dir.display()).into());
    }
    if !input_dir.is_dir() {
        return Err(format!("Input path is not a directory: {}", input_dir.display()).into());
    }

    // Check for required subdirectories
    let test_case_dir = input_dir.join("test_case");
    let test_case_src_dir = test_case_dir.join("src");
    let test_vectors_dir = input_dir.join("test_vectors");

    if !test_case_dir.exists() || !test_case_dir.is_dir() {
        return Err(format!(
            "Required test_case directory not found: {}",
            test_case_dir.display()
        )
        .into());
    }

    if !test_case_src_dir.exists() || !test_case_src_dir.is_dir() {
        return Err(format!(
            "Required test_case/src directory not found: {}",
            test_case_src_dir.display()
        )
        .into());
    }

    if !test_vectors_dir.exists() || !test_vectors_dir.is_dir() {
        return Err(format!(
            "Required test_vectors directory not found: {}",
            test_vectors_dir.display()
        )
        .into());
    }

    Ok((test_case_src_dir, test_vectors_dir))
}

/// Reads all files in a directory and parses them as TestCase JSON files
/// These sorts of files can be found in the test_vectors/ directory of the benchmark
pub fn parse_test_vectors<P: AsRef<Path>>(directory_path: P) -> HarvestResult<Vec<TestCase>> {
    let dir_path = directory_path.as_ref();

    // Read directory entries
    let entries = fs::read_dir(dir_path)
        .map_err(|e| format!("Failed to read directory {}: {}", dir_path.display(), e))?;

    // Process each file and collect successful test cases
    let mut test_cases = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| {
            format!(
                "Failed to read directory entry in {}: {}",
                dir_path.display(),
                e
            )
        })?;
        let file_path = entry.path();

        // Try to read and parse the file as a test case JSON
        if let Ok(json_content) = fs::read_to_string(&file_path) {
            if let Ok(mut test_case) = parse_test_case_json(&json_content) {
                // Set the filename field to the file name
                test_case.filename = file_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap() // Should never happen
                    .to_string();
                test_cases.push(test_case);
            }
        }
    }
    Ok(test_cases)
}

/// Clean up build artifacts from successfully translated Rust projects
pub fn cleanup_benchmarks(results: &[ProgramEvalStats], output_dir: &PathBuf) {
    let mut cleaned_count = 0;
    let mut cleanup_errors = Vec::new();

    for result in results {
        if result.translation_success {
            let project_dir = output_dir.join(&result.program_name);

            // Check if Cargo.toml exists to confirm it's a Rust project
            if project_dir.join("Cargo.toml").exists() {
                // println!("Cleaning build artifacts for: {}", result.program_name);

                match Command::new("cargo")
                    .arg("clean")
                    .current_dir(&project_dir)
                    .output()
                {
                    Ok(output) if output.status.success() => {
                        cleaned_count += 1;
                        // println!("  ✅ Cleaned successfully");
                    }
                    Ok(output) => {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        let error_msg =
                            format!("Failed to clean {}: {}", result.program_name, stderr);
                        cleanup_errors.push(error_msg.clone());
                        println!("  ❌ {}", error_msg);
                    }
                    Err(e) => {
                        let error_msg = format!(
                            "Failed to execute cargo clean for {}: {}",
                            result.program_name, e
                        );
                        cleanup_errors.push(error_msg.clone());
                        println!("  ❌ {}", error_msg);
                    }
                }
            } else {
                println!("Skipping {}: No Cargo.toml found", result.program_name);
            }
        }
    }

    println!("\nCleanup Summary:");
    println!("  Successfully cleaned: {} projects", cleaned_count);
    if !cleanup_errors.is_empty() {
        println!("  Cleanup errors: {} projects", cleanup_errors.len());
        for error in &cleanup_errors {
            println!("    - {}", error);
        }
    }

    println!("\nAll processing and cleanup complete!");
}

/// Runs a binary with test case inputs and compares its output against expected values.
///
/// This function executes the binary with the command line arguments and stdin
/// from the provided test case, then compares the actual stdout and stderr
/// against the expected values in the test case.
pub fn validate_binary_output(
    binary_path: &Path,
    test_case: &TestCase,
    timeout_seconds: Option<u64>,
) -> HarvestResult<()> {
    let timeout = Duration::from_secs(timeout_seconds.unwrap_or(10));

    // Run the binary
    let output = runner::run_binary_with_timeout(binary_path, test_case, timeout)
        .map_err(|e| format!("Failed to run binary {}: {}", binary_path.display(), e))?;

    let actual_stdout = String::from_utf8_lossy(&output.stdout);
    let actual_stderr = String::from_utf8_lossy(&output.stderr);

    // Compare stdout against expected pattern
    let matches = if test_case.stdout.is_regex {
        // Use regex matching
        let regex = Regex::new(&test_case.stdout.pattern).map_err(|e| {
            format!(
                "Invalid regex pattern '{}': {}",
                test_case.stdout.pattern, e
            )
        })?;
        regex.is_match(actual_stdout.trim())
    } else {
        // Use simple equality matching
        actual_stdout.trim() == test_case.stdout.pattern.trim()
    };

    if matches {
        let match_type = if test_case.stdout.is_regex {
            "regex"
        } else {
            "exact"
        };
        Ok(())
    } else {
        let pattern_type = if test_case.stdout.is_regex {
            "regex pattern"
        } else {
            "expected stdout"
        };
        Err(format!(
            "❌ Binary produced unexpected output:\n\n{}: {}\n\nActual stdout:\n{}\n\nActual stderr:\n{}",
            pattern_type.chars().next().unwrap().to_uppercase().collect::<String>() + &pattern_type[1..],
            test_case.stdout.pattern,
            actual_stdout,
            actual_stderr
        ).into())
    }
}
