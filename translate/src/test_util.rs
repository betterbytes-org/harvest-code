//! Place to put utilities that are only used by tests.

/// Returns a new temporary directory. Unlike the defaults in the `tempdir` and `tempfile` crates,
/// this directory is not world-accessible by default.
#[cfg(not(miri))]
pub fn tempdir() -> std::io::Result<tempfile::TempDir> {
    use std::fs::Permissions;
    let mut builder = tempfile::Builder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        builder.permissions(Permissions::from_mode(0o700));
    }
    builder.tempdir()
}
