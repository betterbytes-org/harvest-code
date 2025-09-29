// TODOS (not necessarily in this order)
// TODO: diagnostics for failed struct selection
// TODO: cost analysis
// TODO: formalize semantics of AbstractOps
// TODO: write up some good example to justify design
// Test against real-world C structures.

// Design goals:
// 1. Easy to extend with new AbstractOps and Rust backends
// 2. Adding more info to AbstractOp or Rust backend never makes selection worse
// 3. Diagnostics / interpretable output (perhaps usable by llm later?)
// 4. ???

/// Some simple examples of abstract operations
/// Should be sufficient to define Stack, Queues, and Dequeues
#[derive(PartialEq, Eq, Hash, Debug, Clone)]
enum AbstractOp {
    PushFront,
    PushBack,
    PopFront,
    PopBack,
    Unsupported,
}

struct CAnalysisResult {
    name: String,
    ops: Vec<AbstractOp>,
}

/// The orthodox Rust data structures that we will translate to
#[derive(PartialEq, Eq, Hash, Debug, Clone)]
enum RustBackendLabel {
    Vec,
    VecDeque,
}

struct RustBackend {
    label: RustBackendLabel,
    ops: Vec<AbstractOp>,
}

impl RustBackend {
    /// Check if this RustBackend implements the given abstract operation
    fn implements_all(&self, ops: &[AbstractOp]) -> bool {
        ops.iter().all(|op| self.ops.contains(op))
    }

    /// Hardcoded Vec backend
    fn vec() -> Self {
        Self {
            label: RustBackendLabel::Vec,
            ops: vec![AbstractOp::PushBack, AbstractOp::PopBack],
        }
    }

    /// Hardcoded VecDeque backend
    fn vecdeque() -> Self {
        Self {
            label: RustBackendLabel::VecDeque,
            ops: vec![
                AbstractOp::PushFront,
                AbstractOp::PushBack,
                AbstractOp::PopFront,
                AbstractOp::PopBack,
            ],
        }
    }
}

struct TranslationCtx {
    backends: Vec<RustBackend>,
}

impl TranslationCtx {
    fn new() -> Self {
        Self {
            backends: vec![RustBackend::vec(), RustBackend::vecdeque()],
        }
    }

    /// Solve constraints presented in the CAnalysisResult
    /// Selects an appropriate backend according to the following validity properties:
    /// Constraints:
    /// 1. Interface satisfaction: RustBackend must implement all AbstractOp in CAnalysisResult
    fn select_rust_struct(&self, c_analysis_result: CAnalysisResult) -> Vec<RustBackendLabel> {
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

fn main() {
    env_logger::init();
    println!("Hello world!");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_neither_backend_matches() {
        // Test case where neither Vec nor VecDeque can satisfy the requirements
        // Using the Len operation that neither backend implements
        let ctx = TranslationCtx::new();
        let c_analysis_result = CAnalysisResult {
            name: "unsupported_struct".to_string(),
            ops: vec![AbstractOp::Unsupported], // Neither Vec nor VecDeque implements Len
        };
        let rust_backends = ctx.select_rust_struct(c_analysis_result);

        // No backends should match since neither implements Len
        assert_eq!(rust_backends.len(), 0);
    }

    #[test]
    fn test_both_backends_match() {
        // Test case where both Vec and VecDeque can satisfy the requirements
        let ctx = TranslationCtx::new();
        let c_analysis_result = CAnalysisResult {
            name: "stack_like_struct".to_string(),
            ops: vec![AbstractOp::PushBack, AbstractOp::PopBack],
        };
        let rust_backends = ctx.select_rust_struct(c_analysis_result);

        // Both Vec and VecDeque should match since both support PushBack and PopBack
        assert_eq!(rust_backends.len(), 2);
        assert!(rust_backends.contains(&RustBackendLabel::Vec));
        assert!(rust_backends.contains(&RustBackendLabel::VecDeque));
    }

    #[test]
    fn test_only_vecdeque_matches() {
        // Test case where only VecDeque can satisfy the requirements
        let ctx = TranslationCtx::new();
        let c_analysis_result = CAnalysisResult {
            name: "queue_like_struct".to_string(),
            ops: vec![AbstractOp::PushFront, AbstractOp::PopBack],
        };
        let rust_backends = ctx.select_rust_struct(c_analysis_result);

        // Only VecDeque should match since Vec doesn't support PushFront
        assert_eq!(rust_backends.len(), 1);
        assert_eq!(rust_backends[0], RustBackendLabel::VecDeque);
    }
}
