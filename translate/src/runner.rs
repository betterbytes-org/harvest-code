use crate::tools::{RunContext, Tool};
use harvest_ir::edit::{self, NewEditError};
use harvest_ir::{Edit, HarvestIR, Id};
use std::collections::{HashMap, HashSet};
use std::fmt::{self, Debug, Formatter};
use std::iter::once;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::{self, JoinHandle, ThreadId, spawn};

/// Spawns off each tool execution in its own thread, and keeps track of those threads.
pub struct ToolRunner {
    invocations: HashMap<ThreadId, RunningInvocation>,

    // Channel used by threads to signal that they are completed running.
    receiver: Receiver<ThreadId>,
    sender: Sender<ThreadId>,
}

impl Default for ToolRunner {
    fn default() -> ToolRunner {
        let (sender, receiver) = channel();
        ToolRunner {
            invocations: HashMap::new(),
            receiver,
            sender,
        }
    }
}

impl ToolRunner {
    /// Waits until at least one tool has completed running, then process the results of all
    /// completed tool invocations. This will update the IR value in edit_organizer. Returns `true`
    /// if at least one tool completed, and `false` if no tools are currently running.
    pub fn process_tool_results(&mut self, edit_organizer: &mut edit::Organizer) -> bool {
        if self.invocations.is_empty() {
            return false;
        }
        for thread_id in
            once(self.receiver.recv().expect("sender dropped")).chain(self.receiver.try_iter())
        {
            let invocation = self
                .invocations
                .remove(&thread_id)
                .expect("missing invocation");
            let completed_invocation = invocation
                .join_handle
                .join()
                .expect("tool invocation thread panicked");
            if let Ok(edit) = completed_invocation
                && let Err(error) = edit_organizer.apply_edit(edit)
            {
                log::error!("Edit application error: {error:?}");
            }
        }
        true
    }

    /// Runs a tool. The tool is run in a new thread.
    pub fn spawn_tool(
        &mut self,
        edit_organizer: &mut edit::Organizer,
        tool: Box<dyn Tool>,
        ir_snapshot: Arc<HarvestIR>,
        might_write: HashSet<Id>,
    ) -> Result<(), SpawnToolError> {
        let mut edit = match edit_organizer.new_edit(&might_write) {
            Err(error) => return Err(SpawnToolError { cause: error, tool }),
            Ok(edit) => edit,
        };
        let sender = self.sender.clone();
        let join_handle = spawn(move || {
            // Tool::run is not necessarily unwind safe, which means that if it panics it might
            // leave shared data in a state that violates invariants. Types that are shared between
            // threads can generally handle this (e.g. Mutex and RwLock have poisoning), but
            // non-Sync types can sometimes have problems there. We don't want to require Tool::run
            // to be unwind safe, so instead this function needs to make sure that values *in this
            // same thread* that `tool` might touch are appropriately dropped/forgotten if `run`
            // panics.
            let result = catch_unwind(AssertUnwindSafe(|| {
                tool.run(RunContext {
                    ir_edit: &mut edit,
                    ir_snapshot,
                })
                .map(|_| edit)
            }));
            let _ = sender.send(thread::current().id());
            // TODO: Diagnostics module.
            match result {
                Err(panic_error) => {
                    log::error!("Tool panicked: {panic_error:?}");
                    Err(())
                }
                Ok(Err(tool_error)) => {
                    log::error!("Tool invocation failed: {tool_error}");
                    Err(())
                }
                Ok(Ok(edit)) => Ok(edit),
            }
        });
        self.invocations
            .insert(join_handle.thread().id(), RunningInvocation { join_handle });
        Ok(())
    }
}

/// An error returned from spawn_tool.
pub struct SpawnToolError {
    pub cause: NewEditError,
    pub tool: Box<dyn Tool>,
}

impl Debug for SpawnToolError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.cause)
    }
}

/// Data the ToolRunner tracks for each currently-running thread. These are accessed from the main
/// thread.
struct RunningInvocation {
    join_handle: JoinHandle<Result<Edit, ()>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MightWriteOutcome::Runnable, test_util::MockTool};
    use harvest_ir::Representation::RawSource;
    use harvest_ir::edit::{self, NewEditError};
    use harvest_ir::fs::RawDir;

    #[test]
    fn new_edit_errors() {
        let mut edit_organizer = edit::Organizer::default();
        let mut edit = edit_organizer.new_edit(&[].into()).unwrap();
        let [a, b, c] = [(); 3].map(|_| edit.add_representation(RawSource(RawDir::default())));
        edit_organizer.apply_edit(edit).expect("setup edit failed");
        let mut runner = ToolRunner::default();
        let unknown_id = Id::new();
        let snapshot = edit_organizer.snapshot();
        assert_eq!(
            runner
                .spawn_tool(
                    &mut edit_organizer,
                    MockTool::new()
                        .might_write(move |_| Runnable([a, unknown_id].into()))
                        .boxed(),
                    snapshot.clone(),
                    [a, unknown_id].into()
                )
                .err()
                .map(|e| e.cause),
            Some(NewEditError::UnknownId)
        );
        let (sender, receiver) = channel();
        assert!(
            runner
                .spawn_tool(
                    &mut edit_organizer,
                    MockTool::new()
                        .might_write(move |_| Runnable([b, c].into()))
                        .run(move |_| { receiver.recv().map_err(Into::into) })
                        .boxed(),
                    snapshot.clone(),
                    [a, b].into(),
                )
                .is_ok()
        );
        assert_eq!(
            runner
                .spawn_tool(
                    &mut edit_organizer,
                    MockTool::new()
                        .might_write(move |_| Runnable([b, c].into()))
                        .boxed(),
                    snapshot,
                    [b, c].into()
                )
                .err()
                .map(|e| e.cause),
            Some(NewEditError::IdInUse),
            "spawned tool with in-use ID"
        );
        sender.send(()).expect("receiver dropped");
        runner.process_tool_results(&mut edit_organizer);
    }

    #[test]
    fn replaced_edit() {
        let mut edit_organizer = edit::Organizer::default();
        let mut edit = edit_organizer.new_edit(&[].into()).unwrap();
        let a = edit.add_representation(RawSource(RawDir::default()));
        edit_organizer.apply_edit(edit).expect("setup edit failed");
        let mut runner = ToolRunner::default();
        let (sender, receiver) = channel();
        let snapshot = edit_organizer.snapshot();
        runner
            .spawn_tool(
                &mut edit_organizer,
                MockTool::new()
                    .might_write(move |_| Runnable([a].into()))
                    .run(move |c| {
                        *c.ir_edit = receiver.recv()?;
                        Ok(())
                    })
                    .boxed(),
                snapshot,
                [a].into(),
            )
            .expect("tool spawn failed");
        // Verify that `a` was marked as in use
        assert!(edit_organizer.new_edit(&[a].into()).err() == Some(NewEditError::IdInUse));
        let mut edit = edit_organizer.new_edit(&[].into()).unwrap();
        let b = edit.add_representation(RawSource(RawDir::default()));
        sender.send(edit).expect("receiver dropped");
        runner.process_tool_results(&mut edit_organizer);
        let ir_ids: Vec<Id> = edit_organizer.snapshot().iter().map(|(id, _)| id).collect();
        // We don't really need this *exact* behavior, but we do need to verify the runner does
        // something reasonable.
        assert_eq!(ir_ids, [a, b]);
    }

    #[test]
    fn success() {
        let mut edit_organizer = edit::Organizer::default();
        let mut runner = ToolRunner::default();
        let snapshot = edit_organizer.snapshot();
        runner
            .spawn_tool(
                &mut edit_organizer,
                MockTool::new()
                    .run(|c| {
                        c.ir_edit.add_representation(RawSource(RawDir::default()));
                        Ok(())
                    })
                    .boxed(),
                snapshot,
                [].into(),
            )
            .expect("tool spawn failed");
        let ir_count = edit_organizer.snapshot().iter().count();
        assert_eq!(ir_count, 0, "edit applied early");
        runner.process_tool_results(&mut edit_organizer);
        let ir_count = edit_organizer.snapshot().iter().count();
        assert_eq!(ir_count, 1, "edit not applied on success");
    }

    #[test]
    fn tool_error() {
        let mut edit_organizer = edit::Organizer::default();
        let mut runner = ToolRunner::default();
        let snapshot = edit_organizer.snapshot();
        runner
            .spawn_tool(
                &mut edit_organizer,
                MockTool::new().run(|_| Err("test error".into())).boxed(),
                snapshot,
                [].into(),
            )
            .expect("tool spawn failed");
        runner.process_tool_results(&mut edit_organizer);
        let ir_count = edit_organizer.snapshot().iter().count();
        assert_eq!(ir_count, 0, "edit applied when tool errored");
    }

    #[test]
    fn tool_panic() {
        let mut edit_organizer = edit::Organizer::default();
        let mut runner = ToolRunner::default();
        let snapshot = edit_organizer.snapshot();
        runner
            .spawn_tool(
                &mut edit_organizer,
                MockTool::new().run(|_| panic!("test panic")).boxed(),
                snapshot,
                [].into(),
            )
            .expect("tool spawn failed");
        runner.process_tool_results(&mut edit_organizer);
        let ir_count = edit_organizer.snapshot().iter().count();
        assert_eq!(ir_count, 0, "edit applied when tool panicked");
    }
}
