use std::num::NonZeroU64;
use std::process::abort;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

/// An opaque type that refers to a particular representation instance in
/// HarvestIR.
// Because IDs can be generated and dropped, it is possible (on 32-bit systems)
// for the ID counter to exceed usize::MAX. Therefore, we use 64-bit IDs (and in
// practice, we run on 64-bit systems, so that matches usize anyway). NonZeroU64
// is used to make Option<Id> smaller, because it's easy and doesn't have a
// downside.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Id(NonZeroU64);

impl Id {
    /// Returns a new ID that has not been seen before.
    #[allow(clippy::new_without_default, reason = "Id has no single default value")]
    pub fn new() -> Id {
        let [out] = Id::new_array();
        out
    }

    /// Returns an array of new, unique IDs.
    ///
    /// # Example
    /// ```
    /// # use harvest_ir::Id;
    /// # fn main() {
    ///     // Allocate two new IDs.
    ///     let [c_ast, rust_ast] = Id::new_array();
    /// # }
    /// ```
    pub fn new_array<const LEN: usize>() -> [Id; LEN] {
        // The highest ID allocated so far. Each new_array() call starts
        // allocating IDs at HIGHEST_ID + 1.
        static HIGHEST_ID: AtomicU64 = AtomicU64::new(0);
        // prev is the ID number immediately before the ID we are currently
        // trying to construct.
        let mut prev = HIGHEST_ID.fetch_add(LEN.try_into().expect("LEN > u64::MAX"), Relaxed);
        [(); LEN].map(|_| {
            let Some(num) = prev.checked_add(1).and_then(NonZeroU64::new) else {
                // We don't have any way to continue execution on overflow. If
                // we try to panic, this tool invocation will fail, but the
                // panic will be caught and we'll just run into this again.
                // Fortunately, it's basically impossible for this to overflow,
                // so we won't hit this case in any useful harvest_translate
                // execution.
                eprintln!("IR ID allocation overflow, cannot continue");
                abort();
            };
            prev = num.get();
            Id(num)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::spawn;

    /// Verifies that new() and new_array() work properly.
    #[test]
    fn new() {
        // Generate IDs from three unsynchronized threads, hoping they run at
        // the same time. Each thread uses a different strategy, but each thread
        // generates exactly 1000 IDs.
        let one_at_a_time = spawn(|| (0..1000).map(|_| Id::new()).collect());
        let chunks = spawn(|| (0..10).map(|_| Id::new_array::<100>()).flatten().collect());
        let all_at_once = Id::new_array::<1000>().into();
        let chunks: Vec<_> = chunks.join().unwrap();
        let one_at_a_time: Vec<_> = one_at_a_time.join().unwrap();
        // The contract of Id is that each ID is unique, but that's easy to
        // achieve by generating 3000 random u64s. This loop instead verifies
        // the internal implementation by verifying that the IDs generated are
        // exactly 1..=3000.
        let mut found = [false; 3000];
        for Id(n) in [all_at_once, chunks, one_at_a_time].iter().flatten() {
            let entry = found.get_mut(n.get() as usize - 1).expect("too-large ID");
            assert!(!*entry, "duplicate ID {}", n);
            *entry = true;
        }
        assert_eq!(found, [true; 3000], "missing ID");
    }
}
