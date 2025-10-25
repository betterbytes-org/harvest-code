use crate::HarvestResult;
use serde::{Deserialize, Serialize};
use std::fs;
/// This module `harness` is intented to contain code that is specific to a particular set of benchmarks,
/// for example, parsing code for benchmark-specific configs.
/// Currently, that is just the MITLL tractor benchmarks.
use std::path::{Path, PathBuf};

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
