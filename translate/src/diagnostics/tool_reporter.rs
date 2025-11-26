//! Diagnostics-reporting infrastructure for tools.

use super::{Shared, lock_shared};
use crate::tools::Tool;
use log::info;
use std::fmt::{self, Display, Formatter};
use std::fs::create_dir;
use std::num::NonZeroU64;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::{collections::hash_map::Entry, path::PathBuf};

/// Diagnostics reporter for a specific tool run. These are provided to tools as part of their
/// context.
// TODO: Presumably Tool::might_write also wants a tool-specific reporter. Does it get a general
// Reporter, ToolReporter, or something else? For now I'm not handing a reporter to
// Tool::might_write.
#[derive(Clone)]
pub struct ToolReporter {
    run_shared: Arc<Mutex<RunShared>>,
}

impl ToolReporter {
    /// To construct a ToolReporter, use [Reporter::start_tool_run], which invokes this.
    pub(super) fn new(shared: Arc<Mutex<Shared>>, tool: &dyn Tool) -> (ToolJoiner, ToolReporter) {
        let (sender, receiver) = channel();
        let tool = ToolId::new(tool);
        let mut guard = lock_shared(&shared);
        let number = match guard.tool_run_counts.entry(tool) {
            Entry::Occupied(mut entry) => {
                let number = entry.get().checked_add(1).unwrap();
                entry.insert(number);
                number
            }
            Entry::Vacant(entry) => *entry.insert(NonZeroU64::MIN),
        };
        let tool_run = ToolRunId {
            tool,
            number,
            _private: (),
        };
        let tool_run_dir = PathBuf::from_iter([
            guard.diagnostics_dir.as_path(),
            "steps".as_ref(),
            tool_run.to_string().as_ref(),
        ]);
        drop(guard);
        create_dir(&tool_run_dir).expect("failed to create tool run directory");
        (
            ToolJoiner { receiver },
            ToolReporter {
                run_shared: Arc::new(Mutex::new(RunShared { sender })),
            },
        )
    }

    /// Initializes log collection for this thread. Tools should call this for each new thread they
    /// spawn, if they spawn threads. Note that the tool runner sets up the thread logger for the
    /// tool's main thread, so Tools that do not spawn any threads do not need to call this.
    pub fn setup_thread_logger(&self) -> ThreadGuard {
        // TODO: Set up this thread's tracing subscriber.
        ThreadGuard {
            run_shared: self.run_shared.clone(),
        }
    }
}

/// Guard returned by [ToolReporter::setup_thread_logger]. Cleans up the thread logger on drop.
pub struct ThreadGuard {
    /// [ToolJoiner::join] should not return until all ThreadGuards should be dropped, so we hold
    /// onto this reference to keep the [RunShared] alive.
    run_shared: Arc<Mutex<RunShared>>,
}

/// Identifies a particular tool. Conceptually, this is equivalent to the tool's name, but this
/// design allows us to optimize the representation in the future to e.g. use TypeId for faster
/// comparisons and hashing.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub(super) struct ToolId {
    /// The name returned by `Tool::name`.
    name: &'static str,
}

impl ToolId {
    /// Constructs a ToolId for this tool. Note that callers should prefer to construct a ToolId
    /// once and copy it around when possible rather than repeatedly construct `ToolId`s, in case
    /// future optimizations make `new` more expensive to decrease the cost of other operations.
    pub fn new(tool: &dyn Tool) -> ToolId {
        ToolId { name: tool.name() }
    }
}

impl Display for ToolId {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.name.fmt(f)
    }
}

/// An identifier for a tool run. Can be converted into a string, which will look like
/// `try_cargo_build_2`. This string should be suitable to use as a file/directory name.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub(super) struct ToolRunId {
    pub tool: ToolId,
    /// The first run of a particular tool has number 1, the second has 2, etc.
    pub number: NonZeroU64,

    // Prevents code outside this module from constructing this.
    _private: (),
}

impl Display for ToolRunId {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}_{}", self.tool, self.number)
    }
}

/// A struct that can wait for all diagnostics handles for a tool to be dropped.
pub(crate) struct ToolJoiner {
    // Receives a message from RunShared when RunShared is dropped.
    receiver: Receiver<()>,
}

impl ToolJoiner {
    /// Waits until all reporters for this tool run have been dropped. Note that this accepts and
    /// drops the ThreadGuard as well, so that it can emit diagnostics.
    pub fn join(&self, guard: ThreadGuard) {
        if Arc::strong_count(&guard.run_shared) > 1 {
            info!("Waiting for remaining tool reporters to be dropped");
        }
        drop(guard);
        self.receiver
            .recv()
            .expect("sender dropped without sending a message?");
    }
}

/// Data shared between the `ToolReporter`s for a particular tool run.
struct RunShared {
    // Used to send a message to ToolJoiner when RunShared is dropped.
    sender: Sender<()>,
}

impl Drop for RunShared {
    fn drop(&mut self) {
        let _ = self.sender.send(());
    }
}
