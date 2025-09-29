pub mod abstractops;
pub mod cost;
pub mod rustbackends;

// TODOS (not necessarily in this order)
// --------------------------------------
// TODO: diagnostics for failed struct selection
// TODO: test cost analysis once we have a struct that could be ruled out by cost
// TODO: write up some good example to justify design
// TODO: test against real-world C structures.
// TODO: expand spec to include structures modeled by other tools (take inspiration from Dafny, Hanoi, etc)

// Design goals:
// 1. Easy to extend with new AbstractOps and Rust backends
// 2. Adding more info to AbstractOp or Rust backend never makes selection worse
// 3. Diagnostics / interpretable output (perhaps usable by llm later?)
// 4. ???

pub use abstractops::*;
pub use rustbackends::*;

pub struct TranslationCtx {
    backends: Vec<RustBackend>,
}

impl Default for TranslationCtx {
    fn default() -> Self {
        Self::new()
    }
}

impl TranslationCtx {
    pub fn new() -> Self {
        Self {
            backends: vec![RustBackend::vec(), RustBackend::vecdeque()],
        }
    }

    pub fn from_rust_backends(backends: &[RustBackend]) -> Self {
        Self {
            backends: backends.to_vec(),
        }
    }

    /// Solve constraints presented in the CAnalysisResult
    /// Selects an appropriate backend according to the following validity properties:
    /// Constraints:
    /// 1. Interface satisfaction: RustBackend must implement all AbstractOp in CAnalysisResult
    /// 2. Performance: Every Abstractop in Rustbackend should be at least as efficient as CanalysisResult
    pub fn select_rust_struct(&self, c_analysis_result: CAnalysisResult) -> Vec<RustBackendLabel> {
        log::info!("Selecting Rust backend for {}", c_analysis_result.name);
        let valid_backends: Vec<RustBackendLabel> = self
            .backends
            .iter()
            .filter(|backend| backend.implements_all_efficiently(&c_analysis_result.ops))
            .map(|backend| backend.label)
            .collect();
        log::info!(
            "Filtered based on validity constraints to: {:?}",
            valid_backends
        );
        valid_backends
    }
}
