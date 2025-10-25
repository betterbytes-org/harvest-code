use serde::Serialize;

/// Statistics for a a single test on a program
#[derive(Debug, Clone, Serialize)]
pub struct TestResult {
    // program_name: String,
    test_filename: String,
    passed: bool,
}

/// Statistics for running many tests on a single program
#[derive(Debug, Serialize)]
pub struct ProgramEvalStats {
    pub program_name: String,
    pub translation_success: bool,
    pub rust_build_success: bool,
    pub total_tests: usize,
    pub passed_tests: usize,
    pub error_message: Option<String>,
    // Store individual test results with filenames and pass/fail status
    pub test_results: Vec<TestResult>,
}

impl ProgramEvalStats {
    /// Calculate success rate as a float percentage
    pub fn success_rate(&self) -> f64 {
        if self.total_tests == 0 {
            0.0
        } else {
            (self.passed_tests as f64 / self.total_tests as f64) * 100.0
        }
    }
}

/// Summary statistics across all program runs
#[derive(Debug, Serialize)]
pub struct SummaryStats {
    pub num_programs: usize,
    pub successful_translations: usize,
    pub successful_rust_builds: usize,
    pub total_tests: usize,
    pub total_passed_tests: usize,
}

impl SummaryStats {
    /// Calculate overall test success rate as a float percentage
    pub fn overall_success_rate(&self) -> f64 {
        if self.total_tests == 0 {
            0.0
        } else {
            (self.total_passed_tests as f64 / self.total_tests as f64) * 100.0
        }
    }

    pub fn translation_success_rate(&self) -> f64 {
        if self.num_programs == 0 {
            0.0
        } else {
            (self.successful_translations as f64 / self.num_programs as f64) * 100.0
        }
    }

    pub fn rust_build_success_rate(&self) -> f64 {
        if self.num_programs == 0 {
            0.0
        } else {
            (self.successful_rust_builds as f64 / self.num_programs as f64) * 100.0
        }
    }

    pub fn from_results(results: &[ProgramEvalStats]) -> Self {
        // Calculate summary statistics
        SummaryStats {
            num_programs: results.len(),
            successful_translations: results.iter().filter(|r| r.translation_success).count(),
            successful_rust_builds: results.iter().filter(|r| r.rust_build_success).count(),
            total_tests: results.iter().map(|r| r.total_tests).sum(),
            total_passed_tests: results.iter().map(|r| r.passed_tests).sum(),
        }
    }
}
