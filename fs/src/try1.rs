#![allow(unused_variables)]  // TODO: Remove when implementations finished

// Design assumptions:
//
// 1. The diagnostic directory is on a single filesystem, and that entire filesystem either *does*
//    support reflink copies or *does not* support reflink copies.
//    -> This enables the implementation to try a reflink copy first, and if one ever fails, to
//    never try a reflink copy again.
// 2. Once a file has been written as a 

use std::{path::{Path, PathBuf}, sync::Arc};

/// Handle for the translation diagnostic directory. The [Dir], [File], and [TextFile] types take
/// references to the diagnostic directory so that they can save their contents to disk.
pub struct DiagnosticDir {}

impl DiagnosticDir {
    /// Creates the diagnostic directory.
    // TODO: This should probably take a config struct reference, because `force` most likely won't
    // be the only configuration this reads. Figure out how to do that given the harvest crate
    // hierarchy. Also figure out the error type.
    pub fn create<P: AsRef<Path>>(path: P, force: bool) -> Result<Arc<DiagnosticDir>, ()> {todo!()}
}

/// A file. Conceptually, this is equivalent to `Arc<[u8]>`, but implements filesystem
/// optimizations to minimize copies and avoid redundant UTF-8 validation checks (for conversion
/// into TextFile).
pub struct File {
    inner: Arc<Inner>,
}

impl File {
    /// Loads a file from disk. This file does not need to be in the diagnostic directory.
    pub fn read_any<P: AsRef<Path>>(diagnostic_dir: Arc<DiagnosticDir>, path: P) -> Result<File, ReadError> { todo!() }

    /// Writes a read-only copy of this file into the diagnostic directory.
    ///
    /// Implementation details: If this file already exists in the diagnostic directory, the new
    /// copy will be hardlinked to the existing copy. 
}

/// An error returned by File::read();
pub enum ReadError {}

/// A text file (that is, a file composed of valid UTF-8). Conceptually, this is equivalent to
/// `Arc<str>`, but implements filesystem optimizations to minimize copies.
pub struct TextFile {
    inner: Arc<Inner>,
}

impl TextFile {
}

/// Inner state of a `File`/`TextFile`. Shared between all clones of this file.
struct Inner {}

/// Backing storage for a `File`/`TextFile`.
enum Storage {
    /// This file is stored in the diagnostic directory as a read-only file.
    Disk(OnDisk),
}

/// File contents that are stored on disk. For a file to be stored on disk, it must:
///
/// 1. Be stored in the diagnostic directory.
/// 2. Be read-only.
/// 3. Not be deleted.
///
/// Methods that write read-only files into the diagnostic directory generally have the
/// postcondition that those files are not deleted until the corresponding [DiagnosticDir] has been
/// dropped, which is how requirement 3 is realized in practice.
struct OnDisk {
    /// Is this file UTF-8 or not? If unknown, this will be None.
    // TODO: This likely belongs in a Mutex or RwLock.
    is_utf8: Option<bool>,
    path: PathBuf,
}
