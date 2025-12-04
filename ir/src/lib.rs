//! The Harvest Intermediate Representation ([HarvestIR]), types it depends on (e.g.
//! [Representation]), and utilities for working with them.

pub mod edit;
pub mod fs;
mod id;

pub use edit::Edit;
pub use id::Id;
use serde::Deserialize;
use std::any::Any;
use std::collections::{BTreeMap, HashSet};
use std::fmt::Display;
use std::fs::File;
use std::io::Write as _;
use std::path::Path;
use std::sync::Arc;

/// Harvest Intermediate Representation
///
/// The Harvest IR is a collection of [Representation]s of a
/// program. Transformations of the IR may add or modify
/// representations.
#[derive(Clone, Default)]
pub struct HarvestIR {
    // The IR is composed of a set of [Representation]s identified by
    // some [Id] that is unique to that [Resentation] (at least for a
    // particular run of the pipeline). There may or may not be a
    // useful ordering for [Id]s, but for now using an ordered map at
    // least gives us a stable ordering when iterating, e.g. to print
    // the IR.
    representations: BTreeMap<Id, Arc<dyn Representation>>,
}

/// An abstract representation of a program
pub trait Representation: Any + Display + Send + Sync {
    /// Materialize the [Representation] to a directory at the
    /// provided `path`.
    ///
    /// Materializing stores an on-disk version of the
    /// [Representation]. The format is specific to each
    /// [Representation] variant.
    ///
    /// [Representation] provides an implementation that writes
    /// the Display output into a file. Representations may override
    /// materialize to provide a different output structure, such as
    /// a directory tree.
    fn materialize(&self, path: &Path) -> std::io::Result<()> {
        writeln!(File::create_new(path)?, "{self}")
    }
}

impl HarvestIR {
    /// Adds a representation with a new ID and returns the new ID.
    pub fn add_representation(&mut self, representation: Box<dyn Representation>) -> Id {
        let id = Id::new();
        self.representations.insert(id, representation.into());
        id
    }

    /// Returns `true` if this `HarvestIR` contains a representation under ID `id`, `false`
    /// otherwise.
    pub fn contains_id(&self, id: Id) -> bool {
        self.representations.contains_key(&id)
    }

    /// Returns all contained Representations of the given type.
    pub fn get_by_representation<R: Representation>(&self) -> impl Iterator<Item = (Id, &R)> {
        // TODO: Add a `TypeId -> Id` map to HarvestIR that allows us to look these up without
        // scanning through all the other representations.
        self.representations
            .iter()
            .filter_map(|(&i, r)| <dyn Any>::downcast_ref(&**r).map(|r| (i, r)))
    }

    /// Returns an iterator over the IDs and representations in this IR.
    pub fn iter(&self) -> impl Iterator<Item = (Id, &dyn Representation)> {
        self.representations.iter().map(|(&id, repr)| (id, &**repr))
    }
}

impl Display for HarvestIR {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, r) in self.representations.iter() {
            writeln!(f, "{i}: {r}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::fmt::{self, Display, Formatter};

    /// A simple Representation that contains no data.
    pub struct EmptyRepresentation;
    impl Display for EmptyRepresentation {
        fn fmt(&self, f: &mut Formatter) -> fmt::Result {
            write!(f, "EmptyRepresentation")
        }
    }
    impl Representation for EmptyRepresentation {}

    /// A Representation that contains only an ID number.
    #[derive(Debug, Eq, Hash, PartialEq)]
    pub struct IdRepresentation(pub usize);
    impl Display for IdRepresentation {
        fn fmt(&self, f: &mut Formatter) -> fmt::Result {
            write!(f, "IdRepresentation({})", self.0)
        }
    }
    impl Representation for IdRepresentation {}

    #[test]
    fn get_by_representation() {
        let mut ir = HarvestIR::default();
        ir.add_representation(Box::new(EmptyRepresentation));
        let b = ir.add_representation(Box::new(IdRepresentation(1)));
        ir.add_representation(Box::new(EmptyRepresentation));
        let d = ir.add_representation(Box::new(IdRepresentation(2)));
        assert_eq!(
            HashSet::from_iter(ir.get_by_representation::<IdRepresentation>()),
            HashSet::from([(b, &IdRepresentation(1)), (d, &IdRepresentation(2))])
        );
    }
}

/// Combined configuration for all Tools in this crate.
#[derive(Debug, Deserialize)]
pub struct ToolConfigs {
    //pub raw_source_to_cargo_llm: raw_source_to_cargo_llm::Config,

    //#[serde(flatten)]
    //unknown: HashMap<String, Value>,
}

impl ToolConfigs {
    pub fn validate(&self) {
        //unknown_field_warning("tools", &self.unknown);
        //self.raw_source_to_cargo_llm.validate();
    }

    /// Returns a mock config for testing.
    pub fn mock() -> Self {
        Self {
            //raw_source_to_cargo_llm: raw_source_to_cargo_llm::Config::mock(),
            //unknown: HashMap::new(),
        }
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
pub trait Tool: Send + 'static {
    /// This tool's name. Should be snake case, as this will be used to create directory and/or
    /// file names.
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
pub struct RunContext<'a> {
    /// A set of changes to be applied to the IR when this tool completes
    /// successfully.
    pub ir_edit: &'a mut Edit,

    /// Read access to the IR. This will be the same IR as `might_write` was
    /// most recently called with.
    pub ir_snapshot: Arc<HarvestIR>,

    /// Configuration for the current harvest_translate run.
    pub config: serde_json::Value, //crate::cli::Config>,
}
