use harvest_ir::{Id, Representation};

/// Trait implemented by each tool. Used by the scheduler to decide what tools
/// to run and to manage those tools.
///
/// An instance of Tool represents a particular invocation of that tool (i.e.
/// certain arguments and a certain initial IR state). The scheduler constructs
/// a Tool when it is considering running that tool, and then decides whether to
/// invoke the tool based on which portions of the IR it uses.
///
/// The tool's constructor does not appear in the Tool trait, because at the
/// time the scheduler constructs the tool it is aware of the tool's concrete
/// type. Tool is Send because we will likely eventually run tools concurrently,
/// and at that point the scheduler will spawn a new thread for each tool it
/// chooses to invoke. Tool is also intentionally dyn compatible.
#[allow(dead_code)] // Remove when scheduler implemented.
pub trait Tool: Send {
    /// Indicate which portion of the IR this tool will access. This is dynamic
    /// and not `const` because it can depend on the arguments the tool was
    /// constructed with.
    fn access_required(&self) -> IRAccess;

    /// Runs the tool. The portions of the IR that this tool needs (as it
    /// requested via `access_required`) are passed in as arguments. If that ID
    /// does not exist yet, it is passed as `None`.
    // Note: This is just a conceptual prototype. In practice, these will
    // probably need to be a runtime-tracked borrow because there's no correct
    // lifetime to put here.
    fn run(
        &self,
        reads: &[(Id, Option<&Representation>)],
        writes: &[(Id, &mut Option<Representation>)],
    );
}

/// Represents what portion of the IR a tool accesses.
pub struct IRAccess {
    /// The portion of the IR that this tool invocation might read. Note that
    /// these do not all need to exist; merely checking if a particular
    /// representation exists is a read operation.
    #[allow(dead_code)] // Remove when scheduler implemented.
    reads: Vec<harvest_ir::Id>,

    /// Which part of the IR this tool invocation might write. Note that both
    /// creating a representation and modifying an existing representation count
    /// as a write.
    #[allow(dead_code)] // Remove when scheduler implemented.
    writes: Vec<harvest_ir::Id>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies Tool is dyn-compatible.
    fn _dyn_compatible(_t: &dyn Tool) {}
}
