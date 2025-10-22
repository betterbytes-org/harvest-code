//! # harvest_translate scheduler
//!
//! The scheduler is responsible for determining which tools to invoke and also
//! for invoking them. It is effectively the main loop of harvest_translate.

use crate::tools::{Context, ToolInvocation};
use harvest_ir::HarvestIR;
use std::sync::Arc;

#[derive(Default)]
pub struct Scheduler {
    ir: Arc<HarvestIR>,
    queued_invocations: Vec<Box<dyn ToolInvocation>>,
}

impl Scheduler {
    /// Returns the current IR snapshot.
    pub fn ir_snapshot(&self) -> Arc<HarvestIR> {
        self.ir.clone()
    }

    /// The scheduler main loop -- invokes tools until done.
    pub fn main_loop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // TODO: This is just a temporary implementation to make the
        // LoadRawSource invocation run; this all needs to be restructured to
        // fit the design doc.
        for invocation in &self.queued_invocations {
            log::debug!("Attempting to invoke {invocation}");
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
            .map_err(|e| {
                log::debug!("Invoking {invocation} failed: {e}");
                e
            })?;
            // TODO: Verify that ir_edit doesn't touch any IDs that might_write
            // did not return (it's theoretically possible -- though not
            // sensible -- for tools to replace ir_edit entirely).
            Arc::make_mut(&mut self.ir).apply_edit(ir_edit);
        }
        Ok(())
    }

    /// Add a tool invocation to the scheduler's queue. Note that scheduling a
    /// tool invocation does not guarantee the tool will run, as a tool may
    /// indicate that it is not runnable.
    pub fn queue_invocation<T: ToolInvocation + 'static>(&mut self, invocation: T) {
        self.queued_invocations.push(Box::new(invocation));
    }
}
