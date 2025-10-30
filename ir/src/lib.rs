pub mod edit;
pub mod fs;
mod id;

pub use edit::Edit;
pub use id::Id;
use std::path::PathBuf;
use std::{collections::BTreeMap, fmt::Display, ops::Deref, path::Path, sync::Arc};

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
    representations: BTreeMap<Id, Arc<Representation>>,
}

/// An abstract representation of a program
pub enum Representation {
    /// A Rust artifact that has been built with `cargo build`.
    CargoBuildResult(Result<Vec<PathBuf>, String>),

    /// A cargo package, ready to be built with `cargo build`.
    CargoPackage(fs::RawDir),

    /// An verbatim copy of the original source code project's
    /// directories and files.
    RawSource(fs::RawDir),
}

impl Representation {
    /// Materialize the [Representation] to a directory at the
    /// provided `path`.
    ///
    /// Materializing stores an on-disk version of the
    /// [Representation]. The format is specific to each
    /// [Representation] variant.
    pub fn materialize<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        match self {
            Representation::CargoPackage(raw_dir) => raw_dir.materialize(path),
            Representation::RawSource(raw_dir) => raw_dir.materialize(path),
            Representation::CargoBuildResult(_) => Ok(()), // Building the artifact is the materialization
        }
    }
}

impl HarvestIR {
    /// Returns `true` if this `HarvestIR` contains a representation under ID `id`, `false`
    /// otherwise.
    pub fn contains_id(&self, id: Id) -> bool {
        self.representations.contains_key(&id)
    }

    /// Returns an iterator over the IDs and representations in this IR.
    pub fn iter(&self) -> impl Iterator<Item = (Id, &Representation)> {
        self.representations.iter().map(|(&id, repr)| (id, &**repr))
    }
}

impl Display for HarvestIR {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for r in self.representations.values() {
            match r.deref() {
                Representation::CargoBuildResult(r) => {
                    writeln!(f, "Built Rust artifact:")?;
                    match r {
                        Ok(artifact_filenames) => {
                            writeln!(f, "  Build succeeded. Artifacts:")?;
                            for filename in artifact_filenames {
                                writeln!(f, "    {}", filename.display())?;
                            }
                        }
                        Err(err) => writeln!(f, "  Build failed: {}", err)?,
                    }
                }

                Representation::CargoPackage(r) => {
                    writeln!(f, "Cargo package:")?;
                    r.display(0, f)?
                }
                Representation::RawSource(r) => {
                    writeln!(f, "Raw C source:")?;
                    r.display(0, f)?
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::RawDir;

    /// Returns a new Representation (for code that needs a Representation but
    /// doesn't care what it is).
    pub(crate) fn new_representation() -> Representation {
        Representation::RawSource(RawDir::default())
    }
}
