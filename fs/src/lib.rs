#![allow(unused_variables)]  // TODO: Remove when implementations finished

use std::path::Path;

/// Mutable view of a diagnostics directory path. New elements may be added to a RwDir, and then
/// those elements (or the entire RwDir) frozen.
pub struct RwDir {
    // Note: The diagnostics system would track which entities within the diagnostic system have
    // been frozen and which are still mutable. I don't yet know if RwDirs themselves would form
    // the nodes of that tree, or merely reference it.
}

impl RwDir {
    /// Recursively makes this directory read-only. Does not follow symlinks. Returns a new Dir
    /// containing the contents of this directory.
    ///
    /// Postcondition: This directory and its recursive contents are not modified or deleted after
    /// freeze() is called until the DiagnosticDir is deleted.
    pub fn freeze(self) -> Result<Dir, Error> { todo!() }

    /// Freezes a sub-path of this directory.
    pub fn freeze_path(&self, path: &Path) -> Result<DirEntry, Error> { todo!() }

    /// Writes a subdirectory into this directory at the given path, read-only. Note that this may
    /// hardlink the subdirectory into the new location rather than performing a full copy.
    pub fn write_subdir_ro(&self, path: &Path, dir: Dir) -> Result<(), Error> { todo!() }
}

/// View of a read-only directory element.
pub enum DirEntry {
    Dir(Dir),
    File(File),
    Symlink(Symlink),
}

// Note: File and TextFile are internally Arc<> to a single shared type. That way, the UTF-8-ness
// of the file can be shared between the copies, because it is computed lazily.

/// A read-only directory.
pub struct Dir {}
/// A read-only file.
pub struct File {}
/// A read-only symlink. Note that all that is frozen is the path; the thing it points to may not
/// exist and may still change (also, this may be relative or absolute and may point anywhere).
pub struct Symlink {}
/// A read-only UTF-8 file.
pub struct TextFile {}

// Stand-in error type so I don't have to implement the real error types while I design the API.
pub struct Error;
