//! Lifts a source code project into a RawSource representation.

use harvest_ir::fs::RawDir;
use harvest_ir::fs::RawSource;
use harvest_ir::{MightWriteContext, MightWriteOutcome, RunContext, Tool};
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
        "load_raw_source"
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
        context
            .ir_edit
            .add_representation(Box::new(RawSource { dir: rawdir }));
        Ok(())
    }
}
