use crate::cost::Cost;

/// Some simple examples of abstract operations
/// Should be sufficient to define Stack, Queues, and Dequeues
#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub enum AbstractOpLabel {
    PushFront,
    PushBack,
    PopFront,
    PopBack,
    Unsupported, // Mostly for testing purposes
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct AbstractOp {
    pub label: AbstractOpLabel,
    pub cost: Cost,
}

pub struct CAnalysisResult {
    pub name: String,
    pub ops: Vec<AbstractOp>,
}
