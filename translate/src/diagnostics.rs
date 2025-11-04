//! The diagnostics component of harvest_translate. Manages the diagnostics directory, offering
//! APIs to write diagnostics into that directory. Also provides tools the ability to create
//! intermediate files and directories (as those should live in the diagnostics directory).

use crate::{cli::Config, util::empty_writable_dir};
use tempfile::TempDir;

/// Diagnostics produced by transpilation. Can be used by callers of `transpile` to inspect the
/// diagnostics produced during its execution.
pub struct Diagnostics {
    
}

/// Component that collects diagnostics during the execution of `transpile`. Creating a Collector
/// will start collecting `tracing` events (writing them into log files and echoing some events to
/// stdout).
pub(crate) struct Collector {
    // If no diagnostics directory has been configured, Collector will use a temporary directory
    // instead (as tools still need to be able to create temporary files). In that case, the
    // TempDir will be stored here so the directory is cleaned up when Collector is dropped.
    tempdir: Option<TempDir>,
}

impl Collector {
    pub fn new(config: &Config) -> Collector {
        #[cfg(miri)]
        let tempdir = None;
        #[cfg(not(miri))]
        let (diagnostics_path, tempdir) = match config.diagnostics_dir {
            None => {
                let tempdir = empty_writable_dir();
            },
        };
        Collector {
        }
    }

    /// Consumes this Collector, extracting the collected diagnostics. Diagnostics emitted after
    /// this is called will be dropped rather than written to the diagnostics directory (if e.g. an
    /// unjoined background thread tries to write diagnostics).
    #[allow(unused)] // TODO: Remove
    pub fn diagnostics(self) -> Diagnostics {
        self.diagnostics
    }
}

/// Values shared by the Collector and various diagnostics handles.
struct Shared {
    diagnostics: Diagnostics,
}
