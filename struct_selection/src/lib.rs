pub mod abstractops;
pub mod rustbackends;
pub mod cost;

// TODOS (not necessarily in this order)
// --------------------------------------
// TODO: diagnostics for failed struct selection
// TODO: cost analysis
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

impl TranslationCtx {
    pub fn new() -> Self {
        Self {
            backends: vec![RustBackend::vec(), RustBackend::vecdeque()],
        }
    }

    /// Solve constraints presented in the CAnalysisResult
    /// Selects an appropriate backend according to the following validity properties:
    /// Constraints:
    /// 1. Interface satisfaction: RustBackend must implement all AbstractOp in CAnalysisResult
    pub fn select_rust_struct(&self, c_analysis_result: CAnalysisResult) -> Vec<RustBackendLabel> {
        log::info!("Selecting Rust backend for {}", c_analysis_result.name);
        let selected_backends = self
            .backends
            .iter()
            .filter(|backend| backend.implements_all(&c_analysis_result.ops))
            .map(|backend| backend.label.clone())
            .collect();
        log::info!("Selected Rust backends: {:?}", selected_backends);
        selected_backends
    }
}
