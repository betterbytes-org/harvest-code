pub mod load_raw_source;
pub mod raw_source_to_cargo_llm;
pub mod try_cargo_build;

use crate::cli::unknown_field_warning;
use harvest_ir::{Edit, HarvestIR, Id};
use serde::Deserialize;
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    sync::Arc,
};

#[derive(Debug, Deserialize)]
pub struct ToolConfigs {
    raw_source_to_cargo_llm: raw_source_to_cargo_llm::Config,

    #[serde(flatten)]
    unknown: HashMap<String, Value>,
}

impl ToolConfigs {
    pub fn validate(&self) {
        unknown_field_warning("tools", &self.unknown);
        self.raw_source_to_cargo_llm.validate();
    }
}

/// A tool invocation that the scheduler could choose to perform.
pub enum ToolInvocation {
    LoadRawSource(load_raw_source::Args),
    RawSourceToCargoLlm,
    TryCargoBuild,
}

impl ToolInvocation {
    pub fn create_tool(&self) -> Box<dyn Tool> {
        match self {
            Self::LoadRawSource(args) => Box::new(load_raw_source::LoadRawSource::new(args)),
            Self::RawSourceToCargoLlm => Box::new(raw_source_to_cargo_llm::RawSourceToCargoLlm),
            Self::TryCargoBuild => Box::new(crate::tools::try_cargo_build::TryCargoBuild),
        }
    }
}

impl Display for ToolInvocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Self::RawSourceToCargoLlm => f.write_str("RawSourceToCargoLlm"),
            Self::LoadRawSource(args) => write!(f, "LoadRawSource({})", args.directory.display()),
            Self::TryCargoBuild => f.write_str("TryCargoBuild"),
        }
    }
}

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
pub trait Tool: Send {
    /// Returns the IDs this tool may write, or `None` if it is unable to run on
    /// on this IR.
    ///
    /// The IDs returned may depend on the tool constructor's arguments as well
    /// as the contents of `ir`. Reasons might_write might return `None` include
    /// but are not limited to:
    /// 1. The tool requires input data that `ir` does not have.
    /// 2. The tool creates data that already exists in `ir` so there is nothing
    ///    to do.
    fn might_write(&mut self, ir: &HarvestIR) -> Option<HashSet<Id>>;

    /// Runs the tool logic. IR access and edits are made using `context`.
    ///
    /// If `Ok` is returned the changes will be applied to the IR, and if `Err`
    /// is returned the changes will not be applied.
    fn run(&mut self, context: Context) -> Result<(), Box<dyn std::error::Error>>;
}

/// Context a tool is provided when it is running. The tool uses this context to
/// access the IR, make IR changes, launch external processes (with
/// diagnostics), and anything else that requires hooking into the rest of
/// harvest_translate.
#[non_exhaustive]
pub struct Context<'a> {
    /// A set of changes to be applied to the IR when this tool completes
    /// successfully.
    pub ir_edit: &'a mut Edit,

    /// Read access to the IR. This will be the same IR as `might_write` was
    /// called with.
    // TODO: Remove once a tool has been implemented.
    #[allow(unused)]
    pub ir_snapshot: Arc<HarvestIR>,

    /// Configuration for the current harvest_translate run.
    pub config: Arc<crate::cli::Config>,
}
