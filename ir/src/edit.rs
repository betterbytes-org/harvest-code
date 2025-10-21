use crate::{Id, Representation};
use std::collections::{HashMap, HashSet};

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
    // Contains an entry for every ID this tool can write.
    pub(crate) writable: HashMap<Id, IdState>,
}

impl Edit {
    /// Creates a new Edit, limited to changing the given set of IDs.
    pub fn new(might_change: &HashSet<Id>) -> Edit {
        Edit {
            writable: might_change
                .iter()
                .map(|&id| (id, IdState::new(false, None)))
                .collect(),
        }
    }

    /// Adds a representation with a new ID and returns the new ID.
    pub fn add_representation(&mut self, representation: Representation) -> Id {
        let id = Id::new();
        self.writable
            .insert(id, IdState::new(true, Some(representation.into())));
        id
    }

    /// Returns the set of IDs that this `Edit` changes.
    pub fn changed_ids(&self) -> Vec<Id> {
        self.writable
            .iter()
            .filter(|(_, state)| state.value.is_some())
            .map(|(&id, _)| id)
            .collect()
    }

    /// Returns the set of IDs passed to new() (not including IDs created by this Edit).
    pub fn might_change(&self) -> HashSet<Id> {
        self.writable
            .iter()
            .filter(|(_, s)| !s.new)
            .map(|(&id, _)| id)
            .collect()
    }

    /// Creates a new ID and gives this tool write access to it.
    pub fn new_id(&mut self) -> Id {
        let id = Id::new();
        self.writable.insert(id, IdState::new(true, None));
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
            .map(|v| v.value = Some(representation.into()))
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

pub(crate) struct IdState {
    /// `true` if this ID was created by the Edit, `false` if this ID was passed into `new()`.
    pub(crate) new: bool,

    /// `None` if this `Edit` does not change this ID, `Some` if it does change this ID.
    pub(crate) value: Option<Box<Representation>>,
}

impl IdState {
    pub fn new(new: bool, value: Option<Box<Representation>>) -> IdState {
        IdState { new, value }
    }
}

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
