use super::Shared;
use crate::tools::Tool;
use std::fmt::{self, Display, Formatter};
use std::num::NonZeroU64;
use std::sync::{Arc, Mutex};
use tracing::dispatcher::DefaultGuard;

/// Diagnostics-reporting tool for use by a specific tool run. These are provided to tools as part
/// of their context.
// TODO: Presumably Tool::might_write also wants a tool-specific reporter, right? Does it get a
// general Reporter, ToolReporter, or something else? For now I'm not handing a reporter to
// Tool::might_write.
#[derive(Clone)]
pub struct ToolReporter {
    shared: Arc<Mutex<Option<Shared>>>,
    tool_run: ToolRunId,
}

impl ToolReporter {
    pub(super) fn new(shared: Arc<Mutex<Option<Shared>>>, tool_run: ToolRunId) -> ToolReporter {
        ToolReporter { shared, tool_run }
    }

    /// Log messages reported by tools are written into the correct diagnostic directories by
    /// thread-local loggers. This sets up that thread-local logger for the current thread,
    /// logging messages into this tool's diagnostic directory. The returned guard should be held
    /// until this thread has completed running the tool.
    ///
    /// Tools only need to call this if they spawn additional threads, as the tool runner will call
    /// this automatically for the thread that `Tool::run` is called in.
    pub fn set_thread_logger(&self) -> DefaultGuard {
        todo!()
        //subscriber::set_default(
        //    Registry::default()
        //        .with(layer().with_ansi(false).with_writer(messages_file.clone()))
        //        .with(layer().with_filter(EnvFilter::builder().parse(&config.log_filter)?)),
        //)
    }
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
        write!(f, "{}", self.name)
    }
}

/// An identifier for a tool run. Can be converted into a string, which will look like
/// `try_cargo_build_2`. This string should be suitable to use as a file/directory name.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub(super) struct ToolRunId {
    pub tool: ToolId,
    /// The first run of a particular tool has number 1, the second has 2, etc.
    pub number: NonZeroU64,
}

impl Display for ToolRunId {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}_{}", self.tool, self.number)
    }
}
