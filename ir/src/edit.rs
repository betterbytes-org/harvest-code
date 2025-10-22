use crate::{Id, Representation};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// A tool for making changes to (a subset of) the IR. When an `Edit` is
/// created, it is given a limited set of representations which it can modify
/// (by ID). An `Edit` can replace those representations as well as create new
/// representations.
///
/// The general pattern for a tool to edit an existing ID's Representation is:
///
/// 1. Read the Representation out of `context.ir_snapshot`.
/// 2. Clone the Representation to get an owned copy.
/// 3. Edit the copied Representation.
/// 4. Store the edited Representation into `context.ir_edit` using `write_id`.
pub struct Edit {
    // Contains every ID this tool can write. IDs that contain Some() will be
    // written, and IDs that contain None will not be touched.
    //
    // Arc<> is used to avoid needing to copy the Representation into HarvestIR
    // when this edit is merged into the main IR; it is not expected for there
    // to be other references to these representations.
    pub(crate) writable: HashMap<Id, Option<Arc<Representation>>>,
}

impl Edit {
    /// Creates a new Edit, limited to changing the given set of IDs.
    pub fn new(might_change: &HashSet<Id>) -> Edit {
        Edit {
            writable: might_change.iter().map(|&id| (id, None)).collect(),
        }
    }

    /// Adds a representation with a new ID and returns the new ID.
    pub fn add_representation(&mut self, representation: Representation) -> Id {
        let id = Id::new();
        self.writable.insert(id, Some(representation.into()));
        id
    }

    /// Returns the set of IDs that this `Edit` changes.
    pub fn changed_ids(&self) -> Vec<Id> {
        self.writable
            .iter()
            .filter(|(_, r)| r.is_some())
            .map(|(&id, _)| id)
            .collect()
    }

    /// Creates a new ID and gives this tool write access to it.
    pub fn new_id(&mut self) -> Id {
        let id = Id::new();
        self.writable.insert(id, None);
        id
    }

    /// Writes `representation` to the given `id`. Errors if this tool cannot
    /// write `id`.
    pub fn try_write_id(
        &mut self,
        id: Id,
        representation: Representation,
    ) -> Result<(), NotWritable> {
        self.writable
            .get_mut(&id)
            .map(|v| *v = Some(representation.into()))
            .ok_or(NotWritable)
    }

    /// Writes `representation` to the given `id`. Panics if this tool cannot
    /// write `id`.
    #[track_caller]
    pub fn write_id(&mut self, id: Id, representation: Representation) {
        if self.try_write_id(id, representation).is_err() {
            panic!("cannot write this id");
        }
    }
}

#[derive(Debug, Eq, PartialEq, thiserror::Error)]
#[error("cannot write this id")]
pub struct NotWritable;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::new_representation;
    use std::panic::catch_unwind;

    #[test]
    fn edit() {
        // Checks whether left and right contain the same elements.
        fn set_equal(left: &[Id], right: &[Id]) -> bool {
            let mut left: Vec<_> = left.iter().collect();
            let mut right: Vec<_> = right.iter().collect();
            left.sort_unstable();
            right.sort_unstable();
            left == right
        }

        let [a, b, c] = Id::new_array();
        let mut edit = Edit::new(&[a, b].into());
        let d = edit.add_representation(new_representation());
        let e = edit.new_id();
        assert_eq!(
            edit.try_write_id(a, new_representation()),
            Ok(()),
            "failed to set writable ID"
        );
        assert_eq!(
            edit.try_write_id(c, new_representation()),
            Err(NotWritable),
            "set unwritable ID"
        );
        edit.write_id(d, new_representation());
        edit.write_id(e, new_representation());
        assert!(
            set_equal(&edit.changed_ids(), &[a, d, e]),
            "changed IDs incorrect"
        );
        assert!(
            catch_unwind(move || edit.write_id(c, new_representation())).is_err(),
            "set unwritable ID"
        );
    }
}
