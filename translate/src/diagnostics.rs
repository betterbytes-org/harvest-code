//! Provides interfaces for writing and inspecting diagnostics. Diagnostics are collected into two
//! places:
//!
//! 1. The diagnostics directory (if one is configured)
//! 2. The [Diagnostics] struct, which is returned by `transpile`.
//!
//! This module also provides directories for tools to use, as those directories live under the
//! diagnostic directory.

use crate::cli::Config;
use crate::util::{EmptyDirError, empty_writable_dir};
use harvest_ir::HarvestIR;
use log::error;
use std::fmt::Write as _;
use std::fs::{canonicalize, create_dir, write};
use std::io;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};
use tempfile::{TempDir, tempdir};
use thiserror::Error;

/// Diagnostics produced by transpilation. Can be used by callers of `transpile` to inspect the
/// diagnostics produced during its execution.
pub struct Diagnostics {
    // TODO: Figure out what we want to have here, versus only on disk. From
    // https://github.com/betterbytes-org/harvest-code/issues/51#issuecomment-3524208160,
    // we at least want information on tool invocation results (successes and errors with error
    // messages).

    // TODO: If this needs to access the diagnostics directory, then we need to move
    // Option<TempDir> from `Collector` into here.
}

/// Component that collects diagnostics during the execution of `transpile`. Creating a Collector
/// will start collecting `tracing` events (writing them into log files and echoing some events to
/// stdout).
// TODO: Implement collecting `tracing` events.
pub(crate) struct Collector {
    shared: Arc<Mutex<Option<Shared>>>,
    // If no diagnostics directory has been configured, Collector will use a temporary directory
    // instead (as tools still need to be able to create temporary files). In that case, the
    // TempDir will be stored here so the directory is cleaned up when Collector is dropped.
    _tempdir: Option<TempDir>,
}

impl Collector {
    /// Creates a Collector, starting diagnostics collection.
    pub fn initialize(config: &Config) -> Result<Collector, CollectorNewError> {
        // We canonicalize the diagnostics path because it will be used to construct paths that are
        // passed as to external commands (as command-line arguments), and the canonicalized path
        // is probably the most compatible representation.
        let (diagnostics_dir, _tempdir) = match &config.diagnostics_dir {
            None => {
                let tempdir = tempdir()?;
                (canonicalize(tempdir.path()), Some(tempdir))
            }
            Some(path) => {
                empty_writable_dir(path, config.force)?;
                (canonicalize(path), None)
            }
        };
        let diagnostics_dir = diagnostics_dir.expect("invalid diagnostics path?");
        create_dir(PathBuf::from_iter([
            diagnostics_dir.as_path(),
            "ir".as_ref(),
        ]))?;
        Ok(Collector {
            shared: Arc::new(Mutex::new(Some(Shared {
                diagnostics: Diagnostics {},
                diagnostics_dir,
            }))),
            _tempdir,
        })
    }

    /// Consumes this [Collector], extracting the collected diagnostics. Diagnostics emitted after
    /// this is called will be dropped rather than written to the diagnostics directory (if e.g. an
    /// unjoined background thread tries to write diagnostics).
    pub fn diagnostics(self) -> Diagnostics {
        lock_shared(&self.shared)
            .take()
            .expect("diagnostics Shared missing")
            .diagnostics
    }

    /// Returns a new [Reporter] that passes diagnostics to this Collector.
    pub fn reporter(&self) -> Reporter {
        Reporter {
            shared: self.shared.clone(),
        }
    }
}

/// A handle used to report diagnostics. Created by using `Collector::reporter`.
#[derive(Clone)]
pub(crate) struct Reporter {
    shared: Arc<Mutex<Option<Shared>>>,
}

impl Reporter {
    /// Reports a new version of the IR.
    pub fn report_ir_version(&self, version: u64, snapshot: &HarvestIR) {
        let Some(ref mut shared) = *lock_shared(&self.shared) else {
            return;
        };
        let mut path = shared.diagnostics_dir.clone();
        path.push("ir");
        path.push(format!("{version:03}"));
        if let Err(error) = create_dir(&path) {
            error!("Failed to create IR directory: {error}");
            return;
        }
        let mut types = vec![];
        for (id, repr) in snapshot.iter() {
            let id_string = format!("{:03}", Into::<u64>::into(id));
            path.push(&id_string);
            if let Err(error) = repr.materialize(&path) {
                error!("Failed to materialize repr: {error}");
            }
            path.pop();
            types.push((id, id_string, repr.name()));
        }
        // TODO: For now, HarvestIR does not guarantee a particular iteration order, but it
        // *happens* to iterate in this same order. We should figure out what guarantees we want
        // HarvestIR to have, and then update this accordingly.
        types.sort_unstable_by_key(|t| t.0);
        let mut index = String::new();
        for (_, id_string, name) in types {
            let _ = writeln!(index, "{id_string}: {name}");
        }
        path.push("index");
        if let Err(error) = write(path, index) {
            error!("Failed to write IR index: {error}");
        }
    }
}

/// Error type returned by Collector::new.
#[derive(Debug, Error)]
pub(crate) enum CollectorNewError {
    #[error("diagnostics directory error")]
    DiagnosticsEmptyDir(#[from] EmptyDirError),
    #[error("I/O error")]
    IoError(#[from] io::Error),
}

/// Utility to lock one of the `Shared` references, logging an error if it is poisoned (and
/// unpoisoning it).
fn lock_shared<'m>(shared: &'m Mutex<Option<Shared>>) -> MutexGuard<'m, Option<Shared>> {
    match shared.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            error!("diagnostics mutex poisoned");
            shared.clear_poison();
            poisoned.into_inner()
        }
    }
}

/// Values shared by the Collector and various diagnostics handles. This is contained in an Option,
/// which is set to `None` when [Collector::diagnostics] is called (and must remain Some() until
/// then).
struct Shared {
    diagnostics: Diagnostics,
    // Path to the root of the diagnostics directory structure.
    diagnostics_dir: PathBuf,
}
