/// Some simple examples of abstract operations
/// Should be sufficient to define Stack, Queues, and Dequeues
#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub enum AbstractOp {
    PushFront,
    PushBack,
    PopFront,
    PopBack,
    Unsupported, // Mostly for testing purposes
}

pub struct CAnalysisResult {
    pub name: String,
    pub ops: Vec<AbstractOp>,
}
