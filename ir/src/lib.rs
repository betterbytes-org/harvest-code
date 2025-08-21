pub mod raw_source;

use std::{collections::BTreeMap, path::Path};

/// Harvest Intermediate Representation
///
/// The Harvest IR is a collection of [Representation]s of a
/// program. Transformations of the IR may add or modify
/// representations.
pub struct HarvestIR {
    representations: BTreeMap<Id, Representation>,
}

pub type Id = u128;

/// An abstract representation of a program
pub enum Representation {
    /// An verbatim copy of the original source code project's
    /// directories and files.
    RawSource(raw_source::RawDir),
}

impl HarvestIR {
    /// Lift a source code project into a [HarvestIR].
    ///
    /// # Arguments
    ///
    /// * `path` - the [Path] to the source code project's root directory.
    ///
    /// # Examples
    ///
    /// ```
    /// # use harvest_ir::HarvestIR;
    /// # fn main() -> std::io::Result<()> {
    /// # let dir = tempdir::TempDir::new("harvest_test")?;
    /// # let path = dir.path();
    /// let harvest_ir = HarvestIR::from_raw_source(path)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_raw_source<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let dir = std::fs::read_dir(path)?;
        let root_dir = raw_source::RawDir::populate_from(dir)?;
        Ok(HarvestIR {
            representations: [(0, Representation::RawSource(root_dir))].into(),
        })
    }

    /// Print a representation of the IR to standard out.
    pub fn display(&self) {
        for r in self.representations.values() {
            match r {
                Representation::RawSource(r) => r.display(0),
            }
        }
    }
}
