//! # harvest_translate scheduler
//!
//! The scheduler is responsible for determining which tools to invoke and also
//! for invoking them. It is effectively the main loop of harvest_translate.

use crate::ToolInvocation;

#[derive(Default)]
pub struct Scheduler {
    queued_invocations: Vec<ToolInvocation>,
}

impl Scheduler {
    /// Invokes `f` with the next suggested tool invocations. `f` is expected to try to run each
    /// tool.
    pub fn next_invocations<F: FnMut(&ToolInvocation) -> InvocationOutcome>(&mut self, mut f: F) {
        self.queued_invocations.retain(|i| {
            log::info!("Trying tool invocation {}", i);
            let outcome = f(i);
            log::info!("Tool invocation outcome: {outcome:?}");
            outcome == InvocationOutcome::Wait
        });
    }

    /// Add a tool invocation to the scheduler's queue. Note that scheduling a
    /// tool invocation does not guarantee the tool will run, as a tool may
    /// indicate that it is not runnable.
    pub fn queue_invocation(&mut self, invocation: ToolInvocation) {
        self.queued_invocations.push(invocation);
    }
}

/// Represents the outcome of trying to invoke a tool.
#[derive(Debug, Hash, PartialEq)]
pub enum InvocationOutcome {
    /// This tool invocation should be discarded and not tried again.
    Discard,
    /// The tool was successfully invoked.
    Success,
    /// The tool could not be launched now; wait and try later.
    Wait,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_invocation() {
        // Counters for the number of times the scheduler tries to run each tool invocation.
        let [mut to_cargo, mut cargo_build] = [0, 0];
        let mut scheduler = Scheduler::default();
        scheduler.queue_invocation(ToolInvocation::RawSourceToCargoLlm);
        scheduler.queue_invocation(ToolInvocation::TryCargoBuild);
        scheduler.next_invocations(|i| match i {
            ToolInvocation::RawSourceToCargoLlm => {
                to_cargo += 1;
                InvocationOutcome::Discard
            }
            ToolInvocation::TryCargoBuild => {
                cargo_build += 1;
                InvocationOutcome::Wait
            }
            i => panic!("unexpected tool invocation {i}"),
        });
        assert_eq!([to_cargo, cargo_build], [1, 1]);
        scheduler.next_invocations(|i| match i {
            ToolInvocation::TryCargoBuild => {
                cargo_build += 1;
                InvocationOutcome::Success
            }
            i => panic!("unexpected tool invocation {i}"),
        });
        assert_eq!([to_cargo, cargo_build], [1, 2]);
        scheduler.next_invocations(|i| panic!("unexpected tool invocation {i}"));
    }
}
