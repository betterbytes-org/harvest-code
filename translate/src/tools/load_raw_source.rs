//! Lifts a source code project into a RawSource representation.

use crate::tools::{Context, Tool};
use harvest_ir::{HarvestIR, Id, Representation, fs::RawDir};
use std::{collections::HashSet, fs::read_dir, path::PathBuf};

use super::ToolInvocation;

pub struct Invocation {
    pub directory: PathBuf,
}

impl ToolInvocation for Invocation {
    fn create_tool(&self) -> Box<dyn Tool> {
        Box::new(LoadRawSource::new(self.directory.clone()))
    }
}

impl std::fmt::Display for Invocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "LoadRawSource({})", self.directory.display())
    }
}

pub struct LoadRawSource {
    directory: Option<PathBuf>,
}

impl LoadRawSource {
    pub fn new(directory: PathBuf) -> LoadRawSource {
        LoadRawSource {
            directory: Some(directory),
        }
    }
}

impl Tool for LoadRawSource {
    // LoadRawSource will create a new representation, not modify an existing
    // one.
    fn might_write(&mut self, _ir: &HarvestIR) -> Option<HashSet<Id>> {
        Some([].into())
    }

    fn run(&mut self, context: Context) -> Result<(), Box<dyn std::error::Error>> {
        let dir_name = self.directory.take().ok_or("already run")?;
        let dir = read_dir(dir_name.clone())?;
        let (rawdir, directories, files) = RawDir::populate_from(dir)?;
        log::info!(
            "Loaded {directories} directories and {files} files from {}.",
            dir_name.display()
        );
        let representation = Representation::RawSource(rawdir);
        context.ir_edit.add_representation(representation);
        Ok(())
    }
}
