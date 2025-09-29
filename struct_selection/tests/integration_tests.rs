use harvest_struct_selection::cost::Cost;
use harvest_struct_selection::*;
use std::sync::Once;

static INIT: Once = Once::new();

/// Setup function that is only run once, even if called multiple times.
fn setup_logger() {
    INIT.call_once(|| {
        env_logger::init();
    });
}

#[test]
fn test_neither_backend_matches() {
    setup_logger();
    // Test case where neither Vec nor VecDeque can satisfy the requirements
    // Using the Len operation that neither backend implements
    let ctx = TranslationCtx::new();
    let c_analysis_result = CAnalysisResult {
        name: "unsupported_struct".to_string(),
        ops: vec![AbstractOp {
            label: AbstractOpLabel::Unsupported,
            cost: Cost::new(1, false),
        }], // Neither Vec nor VecDeque implements Unsupported
    };
    let rust_backends = ctx.select_rust_struct(c_analysis_result);

    // No backends should match since neither implements Len
    assert_eq!(rust_backends.len(), 0);
}

#[test]
fn test_both_backends_match() {
    setup_logger();
    // Test case where both Vec and VecDeque can satisfy the requirements
    let ctx = TranslationCtx::new();
    let c_analysis_result = CAnalysisResult {
        name: "stack_like_struct".to_string(),
        ops: vec![
            AbstractOp {
                label: AbstractOpLabel::PushBack,
                cost: Cost::new(0, false),
            },
            AbstractOp {
                label: AbstractOpLabel::PopBack,
                cost: Cost::new(0, false),
            },
        ],
    };
    let rust_backends = ctx.select_rust_struct(c_analysis_result);

    // Both Vec and VecDeque should match since both support PushBack and PopBack
    assert_eq!(rust_backends.len(), 2);
    assert!(rust_backends.contains(&RustBackendLabel::Vec));
    assert!(rust_backends.contains(&RustBackendLabel::VecDeque));
}

#[test]
fn test_only_vecdeque_matches() {
    setup_logger();
    // Test case where only VecDeque can satisfy the requirements
    let ctx = TranslationCtx::new();
    let c_analysis_result = CAnalysisResult {
        name: "queue_like_struct".to_string(),
        ops: vec![
            AbstractOp {
                label: AbstractOpLabel::PushFront,
                cost: Cost::new(0, false),
            },
            AbstractOp {
                label: AbstractOpLabel::PopBack,
                cost: Cost::new(0, false),
            },
        ],
    };
    let rust_backends = ctx.select_rust_struct(c_analysis_result);

    // Only VecDeque should match since Vec doesn't support PushFront
    assert_eq!(rust_backends.len(), 1);
    assert_eq!(rust_backends[0], RustBackendLabel::VecDeque);
}

#[test]
fn test_backend_ruled_out_by_cost() {
    use harvest_struct_selection::{
        AbstractOp, AbstractOpLabel, CAnalysisResult, RustBackend, RustBackendLabel, cost::Cost,
    };
    setup_logger();
    // Define a Dumbstack backend with linear push_back and constant pop_back
    #[derive(Debug, Clone)]
    struct DumbstackBackend;
    impl DumbstackBackend {
        fn backend() -> RustBackend {
            RustBackend {
                label: RustBackendLabel::Vec, // Use a dummy label or add a new one if needed
                ops: vec![
                    AbstractOp {
                        label: AbstractOpLabel::PushBack,
                        cost: Cost::new(1, false), // O(N)
                    },
                    AbstractOp {
                        label: AbstractOpLabel::PopBack,
                        cost: Cost::new(0, false), // O(1)
                    },
                ],
            }
        }
    }

    // Required: O(1) push_back and O(1) pop_back
    let required_ops = vec![
        AbstractOp {
            label: AbstractOpLabel::PushBack,
            cost: Cost::new(0, false), // O(1)
        },
        AbstractOp {
            label: AbstractOpLabel::PopBack,
            cost: Cost::new(0, false), // O(1)
        },
    ];

    // Compose a TranslationCtx with only Dumbstack using from_rust_backends
    let ctx =
        harvest_struct_selection::TranslationCtx::from_rust_backends(
            &[DumbstackBackend::backend()],
        );

    let c_analysis_result = CAnalysisResult {
        name: "stacklike_struct".to_string(),
        ops: required_ops,
    };

    let rust_backends = ctx.select_rust_struct(c_analysis_result);
    // Dumbstack should be ruled out due to slow push_back
    assert!(
        rust_backends.is_empty(),
        "Dumbstack should not be selected due to cost"
    );
}
