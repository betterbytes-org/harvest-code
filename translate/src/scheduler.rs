//! # harvest_translate scheduler
//!
//! The scheduler is responsible for determining which tools to invoke and also
//! for invoking them. It is effectively the main loop of harvest_translate.

use crate::tools::{Context, Tool, ToolInvocation};
use harvest_ir::{HarvestIR, Edit, Id};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Arc, mpsc::{channel, Receiver, Sender}};
use std::thread::{self, JoinHandle, spawn, ThreadId};

pub struct Scheduler {
    in_use_ids: HashSet<Id>,  // IDs that currently-running tools might write.
    ir: Arc<HarvestIR>,
    queued_invocations: Vec<ToolInvocation>,
    running_invocations: HashMap<ThreadId, RunningInvocation>,

    // Channel used by tool invocation threads to signal that they are completed running.
    tool_receiver: Receiver<ThreadId>,
    tool_sender: Sender<ThreadId>,
}

impl Default for Scheduler {
    fn default() -> Scheduler {
        let (tool_sender, tool_receiver) = channel();
        Scheduler {
            in_use_ids: HashSet::new(),
            ir: Default::default(),
            running_invocations: HashMap::new(),
            queued_invocations: vec![],
            tool_receiver,
            tool_sender,
        }
    }
}

impl Scheduler {
    /// Returns the current IR snapshot.
    pub fn ir_snapshot(&self) -> Arc<HarvestIR> {
        self.ir.clone()
    }

    /// The scheduler main loop -- invokes tools until done.
    pub fn main_loop(&mut self) {
        loop {
            self.queued_invocations.retain(|invocation| {
                let mut tool = invocation.create_tool();
                // TODO: Diagnostics for tools that are not runnable (which is not necessarily an
                // error).
                let Some(ids) = tool.might_write(&self.ir) else {
                    return true;
                };
                if let Err(IdInUse) = self.spawn_tool(tool, ids) {
                    return true;
                }
                false
            });
            
        }
        // TODO: This is just a temporary implementation to make the
        // LoadRawSource invocation run; this all needs to be restructured to
        // fit the design doc.
        for invocation in &self.queued_invocations {
            let mut tool = invocation.create_tool();
            // TODO: Diagnostics for tools that are not runnable (which is not
            // necessarily an error).
            let Some(ids) = tool.might_write(&self.ir) else {
                continue;
            };
            // TODO: Track which IDs are in use and allow tools to write IDs.
            // This implementation is conservative which is correct but too
            // limited (tools can only add new IDs).
            if !ids.is_empty() {
                continue;
            };
            // TODO: Catch panics and handle errors appropriately.
            let mut ir_edit = harvest_ir::Edit::new(&ids);
            tool.run(Context {
                ir_edit: &mut ir_edit,
                ir_snapshot: self.ir.clone(),
            })
            .expect("tool invocation failed");
            // TODO: Verify that ir_edit doesn't touch any IDs that might_write
            // did not return (it's theoretically possible -- though not
            // sensible -- for tools to replace ir_edit entirely).
            Arc::make_mut(&mut self.ir).apply_edit(ir_edit);
        }
    }

    /// Add a tool invocation to the scheduler's queue. Note that scheduling a
    /// tool invocation does not guarantee the tool will run, as a tool may
    /// indicate that it is not runnable.
    pub fn queue_invocation(&mut self, invocation: ToolInvocation) {
        self.queued_invocations.push(invocation);
    }

    /// Waits until one or more tools have completed running, then updates the IR with the results
    /// of those tool invocations.
    pub fn wait_tool_invocation(&mut self) {
        for thread_id in std::iter::once(self.tool_receiver.recv().expect("sender dropped")).chain(self.tool_receiver.try_iter()) {
            let invocation = self.running_invocations.remove(&thread_id).expect("missing join handle");
            let completed_invocation = invocation.join_handle.join().expect("tool invocation thread panicked");
            invocation.might_write.iter().for_each(|id| { self.in_use_ids.remove(id); });
            let edit = match completed_invocation {
                CompletedInvocation::Panic(error) => {
                    // TODO: Diagnostics module.
                    println!("Tool invocation panicked: {error:?}");
                    return;
                },
                CompletedInvocation::Success(edit) => edit,
                // This error was already reported by the tool invocation thread.
                CompletedInvocation::ToolError => return,
            };
            // Verify that every ID touched by `edit` is either:
            // 1. In might_write
            // 2. Created by `edit` (in which case it will not be in the IR yet).
            let might_write: HashSet<Id> = invocation.might_write.into_iter().collect();
            if edit.changed_ids().iter().any(|id| !might_write.contains(id) && self.ir.get(*id).is_some()) {
                // TODO: Diagnostics module.
                println!("Tool replaced its IR edit");
                return;
            }
            Arc::make_mut(&mut self.ir).apply_edit(edit);
        }
    }

    /// Spawns a tool invocation.
    fn spawn_tool(&mut self, tool: Box<dyn Tool>, might_write: Vec<Id>) -> Result<(), IdInUse> {
        for (i, &id) in might_write.iter().enumerate() {
            if self.in_use_ids.insert(id) {
                // Undo the changes to self.in_use_ids before returning the error.
                might_write[..i].iter().for_each(|id| { self.in_use_ids.remove(id); });
                return Err(IdInUse);
            }
        }
        let ir_snapshot = self.ir.clone();
        let sender = self.tool_sender.clone();
        let mut edit = harvest_ir::Edit::new(&might_write);
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
                }).map(|_| edit)
            });
            // TODO: Verify that ir_edit doesn't touch any IDs that might_write
            // did not return (it's theoretically possible -- though not
            // sensible -- for tools to replace ir_edit entirely).
            let _ = sender.send(thread::current().id());
            match result {
                Err(panic_error) => CompletedInvocation::Panic(panic_error),
                Ok(Err(tool_error)) => {
                    // TODO: Add diagnostics module
                    println!("Tool invocation failed: {tool_error}");
                    CompletedInvocation::ToolError
                },
                Ok(Ok(edit)) => CompletedInvocation::Success(edit),
            }
        });
        self.running_invocations.insert(join_handle.thread().id(), RunningInvocation {
            join_handle,
            might_write,
        });
        Ok(())
    }
}

/// Results of a tool invocation that has completed running.
enum CompletedInvocation {
    Panic(Box<dyn std::any::Any + Send + 'static>),
    Success(Edit),
    ToolError,
}

/// Information about a currently-running tool invocation.
struct RunningInvocation {
    join_handle: JoinHandle<CompletedInvocation>,
    might_write: Vec<Id>,
}

#[derive(Debug, thiserror::Error)]
#[error("id already in use")]
struct IdInUse;
