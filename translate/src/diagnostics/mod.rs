//! Provides interfaces for writing and inspecting diagnostics. Diagnostics are collected into two
//! places:
//!
//! 1. The diagnostics directory (if one is configured)
//! 2. The [Diagnostics] struct, which is returned by `transpile`.
//!
//! This module also provides directories for tools to use, as those directories live under the
//! diagnostic directory.

mod tool_reporter;

use crate::cli::Config;
use crate::tools::Tool;
use crate::util::{EmptyDirError, empty_writable_dir};
use harvest_ir::HarvestIR;
use std::collections::HashMap;
use std::fmt::{Arguments, Write as _};
use std::fs::{File, canonicalize, create_dir, write};
use std::io::{self, IoSlice, Write};
use std::num::NonZeroU64;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};
use tempfile::{TempDir, tempdir};
use thiserror::Error;
use tool_reporter::{ToolId, ToolRunId, RunShared};
use tracing::{dispatcher::DefaultGuard, error, subscriber};
use tracing_subscriber::{Layer as _, EnvFilter, Registry};
use tracing_subscriber::fmt::{MakeWriter, layer};
use tracing_subscriber::layer::SubscriberExt as _;

pub use tool_reporter::ToolReporter;

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

    // Guards that clean up values on drop.
    _tempdir: Option<TempDir>,
    _tracing_dispatcher: DefaultGuard,
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
        let messages_file = SharedWriter(Arc::new(Mutex::new(
            File::options()
                .append(true)
                .create_new(true)
                .open(PathBuf::from_iter([
                    diagnostics_dir.as_path(),
                    "messages".as_ref(),
                ]))?,
        )));
        let console_filter = EnvFilter::builder().parse(&config.log_filter)?;
        let _tracing_dispatcher = subscriber::set_default(
            Registry::default()
                .with(layer().with_ansi(false).with_writer(messages_file.clone()))
                .with(layer().with_filter(console_filter.clone())),
        );
        Ok(Collector {
            shared: Arc::new(Mutex::new(Some(Shared {
                console_filter,
                diagnostics: Diagnostics {},
                diagnostics_dir,
                messages_file,
                run_shared: HashMap::new(),
                tool_run_counts: HashMap::new(),
            }))),
            _tempdir,
            _tracing_dispatcher,
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
        let Some(ref shared) = *lock_shared(&self.shared) else {
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

    /// Reports the start of a tool's execution and returns a new [ToolReporter] for the tool.
    pub(crate) fn start_tool_run(&self, tool: &dyn Tool) -> Result<ToolReporter, CollectorDropped> {
        ToolReporter::new(self.shared.clone(), tool)
    }
}

/// Error type returned by Collector::new.
#[derive(Debug, Error)]
pub(crate) enum CollectorNewError {
    #[error("diagnostics directory error")]
    DiagnosticsEmptyDir(#[from] EmptyDirError),
    #[error("I/O error")]
    IoError(#[from] io::Error),
    #[error("invalid RUST_LOG filter")]
    LogFilterError(#[from] tracing_subscriber::filter::ParseError),
}

/// Some functions can only be called while diagnostics are being collected (the [Collector] is
/// still alive). This is the error return if one of those functions is called while diagnostics
/// are not being collected.
#[derive(Debug, Error)]
#[error("diagnostics::Collector already dropped")]
pub(crate) struct CollectorDropped;

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
    console_filter: EnvFilter,
    diagnostics: Diagnostics,
    // Path to the root of the diagnostics directory structure.
    diagnostics_dir: PathBuf,

    messages_file: SharedWriter<File>,
    run_shared: HashMap<ToolRunId, RunShared>,

    // The number of times each tool has been run. Tools that have not been run yet will not be
    // present in this map. This is incremented when a tool run starts, not when it ends.
    tool_run_counts: HashMap<ToolId, NonZeroU64>,
}

/// MakeWriter is not implemented for Arc<Mutex<_>>
/// (https://github.com/tokio-rs/tracing/issues/2687). This works around that by wrapping
/// Arc<Mutex<_>>.
struct SharedWriter<W: Write>(pub Arc<Mutex<W>>);

impl<W: Write> Clone for SharedWriter<W> {
    fn clone(&self) -> SharedWriter<W> {
        SharedWriter(self.0.clone())
    }
}

impl<'l, W: Write + 'l> MakeWriter<'l> for SharedWriter<W> {
    type Writer = MutexGuardWriter<'l, W>;
    fn make_writer(&'l self) -> MutexGuardWriter<'l, W> {
        MutexGuardWriter(match self.0.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                self.0.clear_poison();
                poisoned.into_inner()
            }
        })
    }
}

struct MutexGuardWriter<'l, W: Write>(MutexGuard<'l, W>);

impl<W: Write> Write for MutexGuardWriter<'_, W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
    fn write_vectored(&mut self, bufs: &[IoSlice]) -> io::Result<usize> {
        self.0.write_vectored(bufs)
    }
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.0.write_all(buf)
    }
    fn write_fmt(&mut self, args: Arguments) -> io::Result<()> {
        self.0.write_fmt(args)
    }
}
