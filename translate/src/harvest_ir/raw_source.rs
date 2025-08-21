use std::{collections::BTreeMap, ffi::OsString, fs::ReadDir};

/// A representation of a file-system directory entry.
pub enum RawEntry {
    Dir(RawDir),
    File(Vec<u8>),
}

impl RawEntry {
    fn dir(&self) -> Option<&RawDir> {
        match self {
            RawEntry::Dir(raw_dir) => Some(raw_dir),
            _ => None,
        }
    }

    fn file(&self) -> Option<&Vec<u8>> {
        match self {
            RawEntry::File(file) => Some(file),
            _ => None,
        }
    }
}

/// A representation of a file-system directory tree.
pub struct RawDir(BTreeMap<OsString, RawEntry>);

impl RawDir {
    /// Create a [RawDir] from a local file system directory
    pub fn populate_from(read_dir: ReadDir) -> std::io::Result<Self> {
	let mut result = BTreeMap::default();
        for entry in read_dir.filter_map(|e| e.ok()) {
            let metadata = entry.metadata()?;
            if metadata.is_dir() {
                let subdir = RawDir::populate_from(std::fs::read_dir(entry.path())?)?;
                result.insert(entry.file_name(), RawEntry::Dir(subdir));
            } else if metadata.is_file() {
                let contents = std::fs::read(entry.path())?;
                result.insert(entry.file_name(), RawEntry::File(contents));
            } else {
                // symlinks not yet supported... what the heck do I do with these?
            }
        }
        Ok(RawDir(result))
    }

    /// Print a representation of the directory to standard out.
    ///
    /// # Arguments
    ///
    /// * `level` - The level of this directory relative to the
    ///             root. Used to add padding to before entry names.
    pub fn display(&self, level: usize) {
        let pad = "  ".repeat(level);
        for (name, entry) in self
            .0
            .iter()
            .filter_map(|(name, entry)| entry.dir().map(|e| (name, e)))
        {
            println!("{pad}{}", name.to_string_lossy());
            entry.display(1);
        }

        for (name, entry) in self
            .0
            .iter()
            .filter_map(|(name, entry)| entry.file().map(|e| (name, e)))
        {
            println!("{pad}{} ({}B)", name.to_string_lossy(), entry.len());
        }
    }
}
