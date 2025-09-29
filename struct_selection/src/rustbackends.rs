use crate::abstractops::AbstractOp;

/// The orthodox Rust data structures that we will translate to
#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub enum RustBackendLabel {
    Vec,
    VecDeque,
}

pub struct RustBackend {
    pub label: RustBackendLabel,
    pub ops: Vec<AbstractOp>,
}

impl RustBackend {
    /// Check if this RustBackend implements the given abstract operation
    pub fn implements_all(&self, ops: &[AbstractOp]) -> bool {
        ops.iter().all(|op| self.ops.contains(op))
    }

    /// Hardcoded Vec backend
    pub fn vec() -> Self {
        Self {
            label: RustBackendLabel::Vec,
            ops: vec![AbstractOp::PushBack, AbstractOp::PopBack],
        }
    }

    /// Hardcoded VecDeque backend
    pub fn vecdeque() -> Self {
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
