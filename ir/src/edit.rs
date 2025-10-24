//! A system for organizing concurrent mutations to a [`HarvestIR`].

use crate::{HarvestIR, Id, Representation};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use thiserror::Error;

/// A tool for organizing concurrent modifications to a `HarvestIR`. This owns the `HarvestIR`,
/// providing read-only access via `snapshot` and an interface to create and apply `Edits` that
/// mutate the IR. `Organizer` does not allow two `Edit`s that can modify the same representation
/// to exist simultaneously.
#[derive(Default)]
pub struct Organizer {
    ir: Arc<HarvestIR>,
    shared: Arc<Shared>,
}

impl Organizer {
    /// Constructs a new Organizer with the provided `HarvestIR`.
    pub fn with_harvest_ir(ir: HarvestIR) -> Organizer {
        Organizer {
            ir: ir.into(),
            shared: Default::default(),
        }
    }

    /// Applies the edit in `Edit` to the IR. This will update the IR and mark the edit's IDs as
    /// unused.
    pub fn apply_edit(&mut self, mut edit: Edit) -> Result<(), WrongOrganizer> {
        // Note: we just drop `edit` to mark the IDs as no longer in use.
        if !Arc::ptr_eq(&self.shared, &edit.shared) {
            return Err(WrongOrganizer);
        }
        let ir = Arc::make_mut(&mut self.ir);
        for (&id, ref mut representation) in &mut edit.writable {
            if let Some(representation) = representation.take() {
                ir.representations.insert(id, representation);
            }
        }
        Ok(())
    }

    /// Creates a new `Edit` that can edit the given list of IDs. All IDs in `might_write` must be
    /// part of the current IR.
    ///
    /// The IDs in `might_write` will be marked as in use, and will only be freed when either the
    /// edit is applied (via [`apply_edit`]) or dropped.
    pub fn new_edit(&mut self, might_write: &HashSet<Id>) -> Result<Edit, NewEditError> {
        // An unknown ID generally represents a bug in the calling code, whereas IdInUse can be a
        // normal situation. Therefore, prioritize returning UnknownId so that IdInUse doesn't hide
        // bugs.
        if might_write.iter().any(|&id| !self.ir.contains_id(id)) {
            return Err(NewEditError::UnknownId);
        }
        let mut in_use = self.shared.in_use.lock().expect("in_use poisoned");
        if might_write.iter().any(|id| in_use.contains(id)) {
            return Err(NewEditError::IdInUse);
        }
        might_write.iter().for_each(|&id| {
            in_use.insert(id);
        });
        Ok(Edit {
            shared: self.shared.clone(),
            writable: might_write.iter().map(|&id| (id, None)).collect(),
        })
    }

    /// Returns the current value of the IR.
    pub fn snapshot(&self) -> Arc<HarvestIR> {
        self.ir.clone()
    }
}

/// Error type returned by `Organizer::apply_edit`.
#[derive(Debug, Error, Hash, PartialEq)]
#[error("edit is for a different Organizer")]
pub struct WrongOrganizer;

/// Error type returned by `Organizer::new_edit`.
#[derive(Debug, Error, Hash, PartialEq)]
pub enum NewEditError {
    #[error("one of the IDs is currently in use.")]
    IdInUse,
    #[error("an ID is not in this HarvestIR")]
    UnknownId,
}

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
    shared: Arc<Shared>,

    // Contains every ID this tool can write. IDs that contain Some() will be
    // written, and IDs that contain None will not be touched.
    //
    // Arc<> is used to avoid needing to copy the Representation into HarvestIR
    // when this edit is merged into the main IR; it is not expected for there
    // to be other references to these representations.
    writable: HashMap<Id, Option<Arc<Representation>>>,
}

impl Edit {
    /// Adds a representation with a new ID and returns the new ID.
    pub fn add_representation(&mut self, representation: Representation) -> Id {
        let id = Id::new();
        self.writable.insert(id, Some(representation.into()));
        id
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

impl Drop for Edit {
    fn drop(&mut self) {
        // Mark this Edit's IDs as no longer in use.
        let mut in_use = self.shared.in_use.lock().expect("in_use poisoned");
        self.writable.iter().for_each(|(id, _)| {
            in_use.remove(id);
        });
    }
}

#[derive(Debug, Eq, PartialEq, Error)]
#[error("cannot write this id")]
pub struct NotWritable;

/// State shared between the `Organizer` and the `Edit`s it creates.
#[derive(Default)]
struct Shared {
    in_use: Mutex<HashSet<Id>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::new_representation;
    use std::panic::catch_unwind;

    #[test]
    fn organizer() {
        let mut organizer = Organizer::default();

        // Apply an edit to add two new reprs to the IR.
        let mut edit = organizer
            .new_edit(&[].into())
            .expect("no-ID new_edit failed");
        let [a, b, c] = [
            edit.add_representation(new_representation()),
            edit.add_representation(new_representation()),
            edit.new_id(),
        ];
        assert_eq!(organizer.apply_edit(edit), Ok(()));
        // a and b should be applied but c should not.
        assert_eq!(
            HashSet::from_iter(organizer.snapshot().representations.keys()),
            HashSet::from([&a, &b]),
            "apply_edit set incorrect representations"
        );

        // Nested change creation: create two Edits. Apply the second one, then drop the first.
        let mut edit1 = organizer.new_edit(&[a].into()).expect("new_edit failed");
        let mut edit2 = organizer.new_edit(&[b].into()).expect("new_edit failed");
        assert_eq!(
            *organizer.shared.in_use.lock().expect("in_use poisoned"),
            HashSet::from([a, b])
        );
        let [_, _] = [(); 2].map(|_| edit1.add_representation(new_representation()));
        let [f, g] = [(); 2].map(|_| edit2.add_representation(new_representation()));
        assert_eq!(organizer.apply_edit(edit2), Ok(()), "apply_edit failed");
        assert_eq!(
            *organizer.shared.in_use.lock().expect("in_use poisoned"),
            HashSet::from([a])
        );
        // a, b, f, and g should be set but d and e should not.
        drop(edit1);
        assert_eq!(
            *organizer.shared.in_use.lock().expect("in_use poisoned"),
            HashSet::from([])
        );
        assert_eq!(
            HashSet::from_iter(organizer.snapshot().representations.keys().copied()),
            HashSet::from([a, b, f, g])
        );

        // Swap Edits between Organizer instances.
        assert_eq!(
            organizer.apply_edit(
                Organizer::default()
                    .new_edit(&[].into())
                    .expect("new_edit failed")
            ),
            Err(WrongOrganizer),
            "apply_edit accepted Edit from another Organizer"
        );

        // new_edit error cases.
        let edit = organizer.new_edit(&[f].into()).expect("new_edit failed");
        assert_eq!(
            organizer.new_edit(&[c, f].into()).err(),
            Some(NewEditError::UnknownId),
            "with both an unknown ID and an in use ID, new_edit should return an UnknownId error"
        );
        assert_eq!(
            organizer.new_edit(&[f, g].into()).err(),
            Some(NewEditError::IdInUse),
            "new_edit accepted in use ID"
        );
        drop(edit);
    }

    #[test]
    fn edit() {
        let [a, b, c] = Id::new_array();
        let mut organizer = Organizer::with_harvest_ir(HarvestIR {
            representations: [a, b].map(|id| (id, new_representation().into())).into(),
        });
        let mut edit = organizer.new_edit(&[a, b].into()).unwrap();
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
        assert_eq!(
            HashSet::from_iter(
                edit.writable
                    .iter()
                    .filter(|(_, r)| r.is_some())
                    .map(|(&i, _)| i)
            ),
            HashSet::from([a, d, e]),
            "changed IDs incorrect"
        );
        assert!(
            catch_unwind(move || edit.write_id(c, new_representation())).is_err(),
            "set unwritable ID"
        );
    }
}
