use harvest_struct_selection::*;

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
