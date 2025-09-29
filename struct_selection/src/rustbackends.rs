use crate::abstractops::{AbstractOp, AbstractOpLabel};
use crate::cost::Cost;

/// The orthodox Rust data structures that we will translate to
#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum RustBackendLabel {
    Vec,
    VecDeque,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct RustBackend {
    pub label: RustBackendLabel,
    pub ops: Vec<AbstractOp>,
}

impl RustBackend {
    /// Check if this RustBackend implements the given abstract operations
    /// Only considers operation labels, ignores cost
    pub fn implements_all(&self, ops: &[AbstractOp]) -> bool {
        ops.iter().all(|required_op| {
            self.ops
                .iter()
                .any(|backend_op| backend_op.label == required_op.label)
        })
    }

    pub fn implements_all_efficiently(&self, ops: &[AbstractOp]) -> bool {
        ops.iter().all(|required_op| {
            self.ops.iter().any(|backend_op| {
                backend_op.label == required_op.label && backend_op.cost <= required_op.cost
            })
        })
    }

    /// Hardcoded Vec backend
    pub fn vec() -> Self {
        Self {
            label: RustBackendLabel::Vec,
            ops: vec![
                AbstractOp {
                    label: AbstractOpLabel::PushBack,
                    cost: Cost::new(0, false), // O(1) amortized
                },
                AbstractOp {
                    label: AbstractOpLabel::PopBack,
                    cost: Cost::new(0, false), // O(1)
                },
            ],
        }
    }

    /// Hardcoded VecDeque backend
    pub fn vecdeque() -> Self {
        Self {
            label: RustBackendLabel::VecDeque,
            ops: vec![
                AbstractOp {
                    label: AbstractOpLabel::PushFront,
                    cost: Cost::new(0, false), // O(1) amortized
                },
                AbstractOp {
                    label: AbstractOpLabel::PushBack,
                    cost: Cost::new(0, false), // O(1) amortized
                },
                AbstractOp {
                    label: AbstractOpLabel::PopFront,
                    cost: Cost::new(0, false), // O(1)
                },
                AbstractOp {
                    label: AbstractOpLabel::PopBack,
                    cost: Cost::new(0, false), // O(1)
                },
            ],
        }
    }
}
