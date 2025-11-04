//! Lifts a source code project into a RawSource representation.

use crate::tools::{MightWriteContext, MightWriteOutcome, RunContext, Tool};
use harvest_ir::{Representation, fs::RawDir};
use std::fs::read_dir;
use std::path::{Path, PathBuf};

pub struct LoadRawSource {
    directory: PathBuf,
}

impl LoadRawSource {
    pub fn new(directory: &Path) -> LoadRawSource {
        LoadRawSource {
            directory: directory.into(),
        }
    }
}

impl Tool for LoadRawSource {
    fn name(&self) -> &'static str {
        "LoadRawSource"
    }

    // LoadRawSource will create a new representation, not modify an existing
    // one.
    fn might_write(&mut self, _context: MightWriteContext) -> MightWriteOutcome {
        MightWriteOutcome::Runnable([].into())
    }

    fn run(self: Box<Self>, context: RunContext) -> Result<(), Box<dyn std::error::Error>> {
        let dir = read_dir(self.directory.clone())?;
        let (rawdir, directories, files) = RawDir::populate_from(dir)?;
        log::info!(
            "Loaded {directories} directories and {files} files from {}.",
            self.directory.display()
        );
        let representation = Representation::RawSource(rawdir);
        context.ir_edit.add_representation(representation);
        Ok(())
    }
}
