//! # harvest_translate scheduler
//!
//! The scheduler is responsible for determining which tools to invoke and also
//! for invoking them.

use crate::tools::Tool;
use std::mem::replace;
use tracing::debug;

#[derive(Default)]
pub struct Scheduler {
    queued_invocations: Vec<Box<dyn Tool>>,
}

impl Scheduler {
    /// Invokes `f` with the next suggested tool invocations. `f` is expected to try to run each
    /// tool. If the tool cannot be executed and should be tried again later, then `f` should
    /// return it.
    pub fn next_invocations<F: FnMut(Box<dyn Tool>) -> Option<Box<dyn Tool>>>(&mut self, mut f: F) {
        let new_queue = Vec::with_capacity(self.queued_invocations.len());
        for tool in replace(&mut self.queued_invocations, new_queue) {
            debug!("Trying to invoke tool {}", tool.name());
            if let Some(tool) = f(tool) {
                debug!("Returning {} to queue", tool.name());
                self.queued_invocations.push(tool);
            } else {
                debug!("Tool removed from queue");
            }
        }
    }

    /// Add a tool invocation to the scheduler's queue. Note that scheduling a
    /// tool invocation does not guarantee the tool will run, as a tool may
    /// indicate that it is not runnable.
    pub fn queue_invocation<T: Tool>(&mut self, invocation: T) {
        self.queued_invocations.push(Box::new(invocation));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::MockTool;

    #[test]
    fn next_invocation() {
        // Counters for the number of times the scheduler tries to run each tool invocation.
        let [mut a_count, mut b_count] = [0, 0];
        let mut scheduler = Scheduler::default();
        scheduler.queue_invocation(MockTool::new().name("a"));
        scheduler.queue_invocation(MockTool::new().name("b"));
        scheduler.next_invocations(|t| match t.name() {
            "a" => {
                a_count += 1;
                None
            }
            "b" => {
                b_count += 1;
                Some(t)
            }
            _ => panic!("unexpected tool invocation {}", t.name()),
        });
        assert_eq!([a_count, b_count], [1, 1]);
        scheduler.next_invocations(|t| match t.name() {
            "b" => {
                b_count += 1;
                None
            }
            _ => panic!("unexpected tool invocation {}", t.name()),
        });
        assert_eq!([a_count, b_count], [1, 2]);
        scheduler.next_invocations(|t| panic!("unexpected tool invocation {}", t.name()));
    }
}
