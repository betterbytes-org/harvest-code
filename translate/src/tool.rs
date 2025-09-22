use harvest_ir::{HarvestIR, Id};
use std::sync::Arc;

/// Trait implemented by each tool. Used by the scheduler to decide what tools
/// to run and to manage those tools.
///
/// An instance of Tool represents a particular invocation of that tool (i.e.
/// certain arguments and a certain initial IR state). The scheduler constructs
/// a Tool when it is considering running that tool, and then decides whether to
/// invoke the tool based on which parts of the IR it writes.
///
/// The tool's constructor does not appear in the Tool trait, because at the
/// time the scheduler constructs the tool it is aware of the tool's concrete
/// type. Tool is Send because we will likely eventually run tools concurrently,
/// and at that point the scheduler will spawn a new thread for each tool it
/// chooses to invoke. Tool is also intentionally dyn compatible.
#[allow(dead_code)] // Remove when scheduler implemented.
pub trait Tool: Send {
    /// Returns the IDs this tool may write, or `None` if it is unable to run on
    /// on this PR.
    ///
    /// The IDs returned may depend on the tool constructor's arguments as well
    /// as the contents of `ir`. Reasons may_write might return `None` include
    /// but are not limited to:
    /// 1. The tool requires input data that `ir` does not have.
    /// 2. The tool creates data that already exists in `ir` so there is nothing
    ///    to do.
    fn may_write(&self, ir: &HarvestIR) -> Option<Vec<Id>>;

    /// Runs the tool logic. IR access and edits are made using `context`.
    ///
    /// If `Ok` is returned the changes will be applied to the IR, and if `Err`
    /// is returned the changes will not be applied.
    fn run(&self, context: Context) -> Result<(), Box<dyn std::error::Error>>;
}

/// Context a tool is provided when it is running. The tool uses this context to
/// access the IR, make IR changes, launch external processes (with
/// diagnostics), and anything else that requires hooking into the rest of
/// harvest_translate.
#[non_exhaustive]
pub struct Context {
    // IR this tool was launched with.
    _ir_snapshot: Arc<HarvestIR>,
}

// TODO: Add methods to Context for:
//   Reading representations from the IR
//   Getting write access to the IR
//   Allocating new IDs
//   Adding a new representation to the IR (combines allocating an ID with getting
//   write access to it).
// There are a few nonobvious things about this: how to handle requesting
// read-only access if the ID is writeable by this Tool, write access for
// newly-allocated IDs.

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies Tool is dyn-compatible.
    fn _dyn_compatible(_t: &dyn Tool) {}
}
