use crate::tools::{Context, Tool};
use harvest_ir::edit::{self, NewEditError};
use harvest_ir::{Edit, HarvestIR, Id};
use std::collections::{HashMap, HashSet};
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
    ) -> Result<(), NewEditError> {
        let mut edit = edit_organizer.new_edit(&might_write)?;
        let sender = self.sender.clone();
        let join_handle = spawn(move || {
            // Tool::run is not necessarily unwind safe, which means that if it panics it might
            // leave shared data in a state that violates invariants. Types that are shared between
            // threads can generally handle this (e.g. Mutex and RwLock have poisoning), but
            // non-Sync types can sometimes have problems there. We don't want to require Tool::run
            // to be unwind safe, so instead this function needs to make sure that values *in this
            // same thread* that `tool` might touch are appropriately dropped/forgotten if `run`
            // panics.
            let mut tool = AssertUnwindSafe(tool);
            let result = catch_unwind(move || {
                tool.run(Context {
                    ir_edit: &mut edit,
                    ir_snapshot,
                })
                .map(|_| edit)
            });
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

/// Data the ToolRunner tracks for each currently-running thread. These are accessed from the main
/// thread.
struct RunningInvocation {
    join_handle: JoinHandle<Result<Edit, ()>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use harvest_ir::Representation::RawSource;
    use harvest_ir::edit::{self, NewEditError};
    use harvest_ir::fs::RawDir;
    use std::{error::Error, mem::take};

    /// A tool that can have several different behaviors.
    struct TestTool {
        might_write: HashSet<Id>,
        receiver: Receiver<Behavior>,
    }

    impl TestTool {
        pub fn new(might_write: &[Id]) -> (TestTool, Sender<Behavior>) {
            let (sender, receiver) = channel();
            (
                TestTool {
                    might_write: might_write.iter().copied().collect(),
                    receiver,
                },
                sender,
            )
        }
    }

    impl Tool for TestTool {
        fn might_write(&mut self, _ir: &HarvestIR) -> Option<HashSet<Id>> {
            Some(take(&mut self.might_write))
        }
        fn run(&mut self, context: Context) -> Result<(), Box<dyn Error>> {
            // Add a new representation so the test case can check whether the Edit was applied.
            context
                .ir_edit
                .add_representation(RawSource(RawDir::default()));
            match self.receiver.recv().expect("sender dropped") {
                Behavior::Error => Err("test error".into()),
                Behavior::Panic => panic!("test panic"),
                Behavior::SwapEdit(edit) => {
                    // Apply the edit passed through the channel.
                    *context.ir_edit = edit;
                    Ok(())
                }
                Behavior::Success => Ok(()),
            }
        }
    }

    enum Behavior {
        Error,          // Return Err(_)
        Panic,          // Panics
        SwapEdit(Edit), // Swaps the tool's Edit with another one.
        Success,        // Returns Ok(())
    }

    #[test]
    fn new_edit_errors() {
        let mut edit_organizer = edit::Organizer::default();
        let mut edit = edit_organizer.new_edit(&[].into()).unwrap();
        let [a, b, c] = [(); 3].map(|_| edit.add_representation(RawSource(RawDir::default())));
        edit_organizer.apply_edit(edit).expect("setup edit failed");
        let mut runner = ToolRunner::default();
        let unknown_id = Id::new();
        let (tool, _) = TestTool::new(&[a, unknown_id]);
        let snapshot = edit_organizer.snapshot();
        assert_eq!(
            runner
                .spawn_tool(
                    &mut edit_organizer,
                    Box::new(tool),
                    snapshot.clone(),
                    [a, unknown_id].into()
                )
                .err(),
            Some(NewEditError::UnknownId)
        );
        let (tool, sender) = TestTool::new(&[a, b]);
        runner
            .spawn_tool(
                &mut edit_organizer,
                Box::new(tool),
                snapshot.clone(),
                [a, b].into(),
            )
            .expect("tool spawn failed");
        let (tool, _) = TestTool::new(&[b, c]);
        assert_eq!(
            runner
                .spawn_tool(&mut edit_organizer, Box::new(tool), snapshot, [b, c].into())
                .err(),
            Some(NewEditError::IdInUse),
            "spawned tool with in-use ID"
        );
        sender.send(Behavior::Success).expect("receiver dropped");
        runner.process_tool_results(&mut edit_organizer);
    }

    #[test]
    fn replaced_edit() {
        let mut edit_organizer = edit::Organizer::default();
        let mut edit = edit_organizer.new_edit(&[].into()).unwrap();
        let a = edit.add_representation(RawSource(RawDir::default()));
        edit_organizer.apply_edit(edit).expect("setup edit failed");
        let mut runner = ToolRunner::default();
        let (tool, sender) = TestTool::new(&[a]);
        let snapshot = edit_organizer.snapshot();
        runner
            .spawn_tool(&mut edit_organizer, Box::new(tool), snapshot, [a].into())
            .expect("tool spawn failed");
        // Verify that `a` was marked as in use
        assert!(edit_organizer.new_edit(&[a].into()).err() == Some(NewEditError::IdInUse));
        let mut edit = edit_organizer.new_edit(&[].into()).unwrap();
        let b = edit.add_representation(RawSource(RawDir::default()));
        sender
            .send(Behavior::SwapEdit(edit))
            .expect("receiver dropped");
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
        let (tool, sender) = TestTool::new(&[]);
        let snapshot = edit_organizer.snapshot();
        runner
            .spawn_tool(&mut edit_organizer, Box::new(tool), snapshot, [].into())
            .expect("tool spawn failed");
        sender.send(Behavior::Success).expect("receiver dropped");
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
        let (tool, sender) = TestTool::new(&[]);
        let snapshot = edit_organizer.snapshot();
        runner
            .spawn_tool(&mut edit_organizer, Box::new(tool), snapshot, [].into())
            .expect("tool spawn failed");
        sender.send(Behavior::Error).expect("receiver dropped");
        runner.process_tool_results(&mut edit_organizer);
        let ir_count = edit_organizer.snapshot().iter().count();
        assert_eq!(ir_count, 0, "edit applied when tool errored");
    }

    #[test]
    fn tool_panic() {
        let mut edit_organizer = edit::Organizer::default();
        let mut runner = ToolRunner::default();
        let (tool, sender) = TestTool::new(&[]);
        let snapshot = edit_organizer.snapshot();
        runner
            .spawn_tool(&mut edit_organizer, Box::new(tool), snapshot, [].into())
            .expect("tool spawn failed");
        sender.send(Behavior::Panic).expect("receiver dropped");
        runner.process_tool_results(&mut edit_organizer);
        let ir_count = edit_organizer.snapshot().iter().count();
        assert_eq!(ir_count, 0, "edit applied when tool panicked");
    }
}
