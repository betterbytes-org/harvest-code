pub mod load_raw_source;
pub mod raw_source_to_cargo_llm;
pub mod try_cargo_build;

use crate::cli::unknown_field_warning;
use harvest_ir::{Edit, HarvestIR, Id};
use serde::Deserialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct Config {
    raw_source_to_cargo_llm: raw_source_to_cargo_llm::Config,

    #[serde(flatten)]
    unknown: HashMap<String, Value>,
}

impl Config {
    pub fn validate(&self) {
        unknown_field_warning("tools", &self.unknown);
        self.raw_source_to_cargo_llm.validate();
    }
}

/// Trait implemented by each tool. Used by the scheduler to decide what tools
/// to run and to manage those tools.
///
/// An instance of Tool represents a particular invocation of that tool (i.e.
/// certain arguments and a certain initial IR state). The scheduler -- or other code -- constructs
/// a Tool when it is considering running that tool. The scheduler then decides whether to invoke
/// the tool based on which parts of the IR it writes.
///
/// The tool's constructor does not appear in the Tool trait, because at the
/// time the scheduler constructs the tool it is aware of the tool's concrete
/// type.
pub trait Tool: Send {
    /// This tool's name.
    fn name(&self) -> &'static str;

    /// Returns an indication of whether the tool can be run now, and if it can be run, which IDs
    /// it might write. The IDs returned may depend on the tool constructor's arguments as well as
    /// the contents of `context.ir`.
    ///
    /// might_write may be called multiple times before the tool is run. Returning
    /// `MightWriteOutcome::Runnable` does not guarantee that this tool will be executed.
    fn might_write(&mut self, context: MightWriteContext) -> MightWriteOutcome;

    /// Runs the tool logic. IR access and edits are made using `context`.
    ///
    /// If `Ok` is returned the changes will be applied to the IR, and if `Err`
    /// is returned the changes will not be applied.
    fn run(self: Box<Self>, context: RunContext) -> Result<(), Box<dyn std::error::Error>>;
}

/// Context passed to `Tool::might_write`. This is a struct so that new values may be added without
/// having to edit every Tool impl.
#[non_exhaustive]
pub struct MightWriteContext<'a> {
    /// Snapshot of the HarvestIR.
    pub ir: &'a HarvestIR,
}

/// Result of a `Tool::might_write` execution.
pub enum MightWriteOutcome {
    /// This tool is not and will not be runnable. Tells the scheduler to discard the tool.
    #[allow(unused)] // TODO: Remove when we have a tool that returns this.
    NotRunnable,

    /// This tool is runnable. The set of IDs returned are the IDs for representations in the
    /// HarvestIR that the tool might write if it is run.
    Runnable(HashSet<Id>),

    /// The tool cannot be run now (e.g. it might need input data that it did not find in the IR),
    /// but it might become runnable in the future so the scheduler should try again later.
    TryAgain,
}

/// Context a tool is provided when it is running. The tool uses this context to
/// access the IR, make IR changes, launch external processes (with
/// diagnostics), and anything else that requires hooking into the rest of
/// harvest_translate.
#[non_exhaustive]
pub struct RunContext<'a> {
    /// A set of changes to be applied to the IR when this tool completes
    /// successfully.
    pub ir_edit: &'a mut Edit,

    /// Read access to the IR. This will be the same IR as `might_write` was
    /// most recently called with.
    pub ir_snapshot: Arc<HarvestIR>,
}
