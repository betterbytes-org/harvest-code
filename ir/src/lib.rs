pub mod edit;
pub mod fs;
mod id;

pub use edit::Edit;
pub use id::Id;
use std::any::Any;
use std::collections::BTreeMap;
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
    /// Returns `true` if this `HarvestIR` contains a representation under ID `id`, `false`
    /// otherwise.
    pub fn contains_id(&self, id: Id) -> bool {
        self.representations.contains_key(&id)
    }

    /// Returns all contained Representations of the given type.
    pub fn get_by_representation<R: Representation>(&self) -> impl Iterator<Item = &R> {
        // TODO: Add a `TypeId -> Id` map to HarvestIR that allows us to look these up without
        // scanning through all the other representations.
        self.representations
            .values()
            .filter_map(|repr| <dyn Any>::downcast_ref(repr))
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
