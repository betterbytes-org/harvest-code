//! # harvest_translate scheduler
//!
//! The scheduler is responsible for determining which tools to invoke and also
//! for invoking them. It is effectively the main loop of harvest_translate.

use crate::tools::{Context, ToolInvocation};
use harvest_ir::edit;

#[derive(Default)]
pub struct Scheduler {
    queued_invocations: Vec<ToolInvocation>,
}

impl Scheduler {
    /// The scheduler main loop -- invokes tools until done.
    pub fn main_loop(
        &mut self,
        ir: &mut edit::Organizer,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // TODO: This is just a temporary implementation to make the
        // LoadRawSource invocation run; this all needs to be restructured to
        // fit the design doc.
        for invocation in &self.queued_invocations {
            log::debug!("Attempting to invoke {invocation}");
            let mut tool = invocation.create_tool();
            let snapshot = ir.snapshot();
            // TODO: Diagnostics for tools that are not runnable (which is not
            // necessarily an error).
            let Some(ids) = tool.might_write(&snapshot) else {
                continue;
            };
            // TODO: Catch panics and handle errors appropriately.
            let mut ir_edit = match ir.new_edit(&ids) {
                Ok(edit) => edit,
                Err(error) => {
                    log::error!("Tool::might_write ID error: {error}");
                    continue;
                }
            };
            tool.run(Context {
                ir_edit: &mut ir_edit,
                ir_snapshot: snapshot,
            })
            .map_err(|e| {
                log::debug!("Invoking {invocation} failed: {e}");
                e
            })?;
            if let Err(error) = ir.apply_edit(ir_edit) {
                log::error!("Failed to apply edit: {error}");
            }
        }
        Ok(())
    }

    /// Add a tool invocation to the scheduler's queue. Note that scheduling a
    /// tool invocation does not guarantee the tool will run, as a tool may
    /// indicate that it is not runnable.
    pub fn queue_invocation(&mut self, invocation: ToolInvocation) {
        self.queued_invocations.push(invocation);
    }
}
