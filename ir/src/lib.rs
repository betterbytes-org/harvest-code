pub mod edit;
pub mod fs;
mod id;

pub use edit::Edit;
pub use id::Id;
use std::{collections::BTreeMap, fmt::Display, ops::Deref, sync::Arc};

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
    /// A cargo package, ready to be built with `cargo build`.
    CargoPackage(fs::RawDir),

    /// An verbatim copy of the original source code project's
    /// directories and files.
    RawSource(fs::RawDir),
}

impl HarvestIR {
    pub fn apply_edit(&mut self, edit: Edit) {
        for (id, representation) in edit.writable {
            if let Some(representation) = representation {
                self.representations.insert(id, representation);
            }
        }
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
    use std::fs::read_dir;
    use tempdir::TempDir;

    /// Returns a new Representation (for code that needs a Representation but
    /// doesn't care what it is).
    pub(crate) fn new_representation() -> Representation {
        Representation::RawSource(
            RawDir::populate_from(read_dir(TempDir::new("harvest_test").unwrap().path()).unwrap())
                .unwrap(),
        )
    }

    #[test]
    fn apply_edit() {
        let initial_ir = Id::new_array::<3>().map(|i| (i, new_representation().into()));
        let mut ir = HarvestIR {
            representations: initial_ir.clone().into_iter().collect(),
        };
        let [(a, a_repr), (b, b_repr), (c, _)] = initial_ir;
        let mut edit = Edit::new(&[b, c]);
        edit.write_id(c, new_representation());
        let c_repr = edit.writable[&c].clone().unwrap();
        let d = edit.add_representation(new_representation());
        let d_repr = edit.writable[&d].clone().unwrap();
        edit.new_id();
        ir.apply_edit(edit);
        assert_eq!(ir.representations.len(), 4);
        assert!(Arc::ptr_eq(&a_repr, &ir.representations[&a]));
        assert!(Arc::ptr_eq(&b_repr, &ir.representations[&b]));
        assert!(Arc::ptr_eq(&c_repr, &ir.representations[&c]));
        assert!(Arc::ptr_eq(&d_repr, &ir.representations[&d]));
    }
}
