use crate::ir_storage::{IrStorage, MarkInUseError};
use crate::tools::{Context, Tool};
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
    /// completed tool invocations. This will update the IR value in ir_storage.
    pub fn process_tool_results(&mut self, ir_storage: &mut IrStorage) {
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
            if let Ok(edit) = completed_invocation {
                if ir_storage
                    .apply_edit(edit, &invocation.might_write)
                    .is_err()
                {
                    println!("Tool replaced edit");
                }
            }
        }
    }

    /// Runs a tool. The tool is run in a new thread.
    pub fn spawn_tool(
        &mut self,
        ir_storage: &mut IrStorage,
        tool: Box<dyn Tool>,
        ir_snapshot: Arc<HarvestIR>,
        might_write: HashSet<Id>,
    ) -> Result<(), MarkInUseError> {
        ir_storage.mark_in_use(&might_write)?;
        let mut edit = Edit::new(&might_write);
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
                    println!("Tool panicked: {panic_error:?}");
                    Err(())
                }
                Ok(Err(tool_error)) => {
                    println!("Tool invocation failed: {tool_error}");
                    Err(())
                }
                Ok(Ok(edit)) => Ok(edit),
            }
        });
        self.invocations.insert(
            join_handle.thread().id(),
            RunningInvocation {
                join_handle,
                might_write,
            },
        );
        Ok(())
    }
}

/// Data the ToolRunner tracks for each currently-running thread. These are accessed from the main
/// thread.
struct RunningInvocation {
    join_handle: JoinHandle<Result<Edit, ()>>,
    might_write: HashSet<Id>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir_storage::{IrStorage, MarkInUseError};
    use harvest_ir::{Representation::RawSource, fs::RawDir};
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
            match self.receiver.recv().expect("sender dropped") {
                Behavior::Error => Err("test error".into()),
                Behavior::Panic => panic!("test panic"),
                Behavior::SwapEdit(edit) => {
                    // Try to make an edit using the context's edit, then apply the edit passed
                    // through the channel.
                    context
                        .ir_edit
                        .add_representation(RawSource(RawDir::default()));
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
    fn mark_in_use_errors() {
        let mut ir_storage = IrStorage::default();
        let mut edit = Edit::new(&[].into());
        let a = edit.add_representation(RawSource(RawDir::default()));
        ir_storage
            .apply_edit(edit, &[].into())
            .expect("setup edit failed");
        let mut runner = ToolRunner::default();
        let (tool, sender) = TestTool::new(&[a]);
        let snapshot = ir_storage.ir_snapshot();
        runner
            .spawn_tool(&mut ir_storage, Box::new(tool), snapshot, [a].into())
            .expect("tool spawn failed");
    }

    #[test]
    fn replaced_edit() {
        let mut ir_storage = IrStorage::default();
        let mut edit = Edit::new(&[].into());
        let a = edit.add_representation(RawSource(RawDir::default()));
        ir_storage
            .apply_edit(edit, &[].into())
            .expect("setup edit failed");
        let mut runner = ToolRunner::default();
        let (tool, sender) = TestTool::new(&[a]);
        let snapshot = ir_storage.ir_snapshot();
        runner
            .spawn_tool(&mut ir_storage, Box::new(tool), snapshot, [a].into())
            .expect("tool spawn failed");
        // Verify that `a` was marked as in use
        assert_eq!(
            ir_storage.mark_in_use(&[a].into()),
            Err(MarkInUseError::IdInUse)
        );
        let mut edit = Edit::new(&[].into());
        edit.add_representation(RawSource(RawDir::default()));
        sender
            .send(Behavior::SwapEdit(edit))
            .expect("receiver dropped");
        runner.process_tool_results(&mut ir_storage);
        let ir_ids: Vec<Id> = ir_storage.ir_snapshot().iter().map(|(id, _)| id).collect();
        assert_eq!(ir_ids, [a], "applied Edit that was swapped");
    }
}
