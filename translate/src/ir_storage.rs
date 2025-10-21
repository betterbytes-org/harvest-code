use harvest_ir::{Edit, HarvestIR, Id};
use std::{collections::HashSet, sync::Arc};
use thiserror::Error;

/// Stores the IR and tracks which representations are in use by currently-running tools.
#[derive(Default)]
pub struct IrStorage {
    ids_in_use: HashSet<Id>,
    ir: Arc<HarvestIR>,
}

impl IrStorage {
    /// Marks the IDs from `might_write` as no longer in use. Then verifies that `edit` matches
    /// `might_write` (this catches the case where a tool replaces its Edit with another one, which
    /// is an error). If they match, it applies `edit` to the IR.
    pub fn apply_edit(
        &mut self,
        edit: Edit,
        might_write: &HashSet<Id>,
    ) -> Result<(), EditReplaced> {
        might_write.iter().for_each(|id| {
            self.ids_in_use.remove(id);
        });
        if edit.might_change() != *might_write {
            return Err(EditReplaced);
        }
        Arc::make_mut(&mut self.ir).apply_edit(edit);
        Ok(())
    }

    /// Marks the specified IDs as no longer in use. For use when a tool's execution has failed. If
    /// any tool was not in use, returns Err(NotInUse), but still clears all the IDs in ids.
    pub fn clear_in_use(&mut self, ids: &HashSet<Id>) -> Result<(), NotInUse> {
        let mut err = false;
        ids.iter().for_each(|id| err |= !self.ids_in_use.remove(id));
        match err {
            false => Ok(()),
            true => Err(NotInUse),
        }
    }

    /// Returns a snapshot of the current value of the IR.
    pub fn ir_snapshot(&self) -> Arc<HarvestIR> {
        self.ir.clone()
    }

    // CHECKPOINT: Figure out who is responsible for verifying that might_write is acceptable.
    /// Verifies that all `ids` are part of the current HarvestIR (it is a logic error for
    /// `Tool::might_write` to return an ID that is not in the IR), then marks the IDs as
    /// currently-in-use. Returns an error if one of the IDs is currently in use. To be called
    /// before invoking the tool.
    pub fn mark_in_use(&mut self, might_write: &HashSet<Id>) -> Result<(), IdInUse> {
        for id in might_write {
            if !self.ir.contains_id(*id) {
                return Err(MarkInUseError::UnknownId);
            }
            if self.ids_in_use.contains(id) {
                return Err(MarkInUseError::IdInUse);
            }
        }
        might_write.iter().for_each(|&id| {
            self.ids_in_use.insert(id);
        });
        Ok(())
    }
}

#[derive(Debug, Error, PartialEq)]
#[error("A tool replaced its Edit with another Edit")]
pub struct EditReplaced;

#[derive(Debug, Error, PartialEq)]
#[error("id already in use")]
pub struct IdInUse;

#[derive(Debug, Error, PartialEq)]
#[error("tried to clear in-use status of ID that was not in use")]
pub struct NotInUse;

#[cfg(test)]
mod tests {
    use super::*;
    use harvest_ir::{Edit, Representation::RawSource, fs::RawDir};

    // Tests the in-use-ID tracking functionality (mark_in_use and apply_edit).
    #[test]
    fn in_use() {
        let mut storage = IrStorage::default();

        assert_eq!(storage.mark_in_use(&[].into()), Ok(()));
        let mut edit = Edit::new(&[].into());
        let [a, b, c, d] = [(); 4].map(|_| edit.add_representation(RawSource(RawDir::default())));
        let e = edit.new_id();
        // Test applying an edit that contains an ID that does not match might_write.
        assert_eq!(
            storage.apply_edit(Edit::new(&[a, b].into()), &[].into()),
            Err(EditReplaced)
        );
        // Test applying the correct edit.
        assert_eq!(storage.apply_edit(edit, &[].into()), Ok(()));
        assert!(storage.ir_snapshot().contains_id(a));
        assert!(storage.ir_snapshot().contains_id(b));
        assert!(storage.ir_snapshot().contains_id(c));
        assert!(storage.ir_snapshot().contains_id(d));

        assert_eq!(storage.mark_in_use(&[a, c].into()), Ok(()));
        // Verify that mark_in_use returns an error if it is called with an in-use ID.
        assert_eq!(
            storage.mark_in_use(&[b, c].into()),
            Err(MarkInUseError::IdInUse)
        );
        // Verify that mark_in_use returns an error if it is called with an unknown ID.
        assert_eq!(
            storage.mark_in_use(&[b, e].into()),
            Err(MarkInUseError::UnknownId)
        );
        // Verify that mark_in_use can return successfully when called with non-in-use IDs.
        assert_eq!(storage.mark_in_use(&[b].into()), Ok(()));
        assert_eq!(storage.clear_in_use(&[b].into()), Ok(()));
        assert_eq!(storage.clear_in_use(&[b].into()), Err(NotInUse));
        // Test apply_edit with an edit that is missing an ID from might_write.
        assert_eq!(
            storage.apply_edit(Edit::new(&[a].into()), &[a, c].into()),
            Err(EditReplaced)
        );
        let mut edit = Edit::new(&[a, c].into());
        edit.write_id(a, RawSource(RawDir::default()));
        let f = edit.add_representation(RawSource(RawDir::default()));
        assert_eq!(storage.apply_edit(edit, &[a, c].into()), Ok(()));
        assert!(storage.ir_snapshot().contains_id(a));
        assert!(storage.ir_snapshot().contains_id(b));
        assert!(storage.ir_snapshot().contains_id(c));
        assert!(storage.ir_snapshot().contains_id(d));
        assert!(storage.ir_snapshot().contains_id(f));
    }
}
