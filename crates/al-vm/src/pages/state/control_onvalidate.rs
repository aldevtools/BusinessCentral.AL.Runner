//! What a page records while the OnValidate trigger of one of its controls runs (`Session::run_control_onvalidate`): the store's
//! write mark and change counter when it began, the `CurrPage.SaveRecord` and `CurrPage.Update` calls on the page since, and the
//! last record call page code made that moved the page's Rec. A nested OnValidate records its own and gives the outer one back
//! when it ends.

use al_builtins::record_op::RecordOp;

/// A `CurrPage.SaveRecord()` or `CurrPage.Update(save)` that page code called on the page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrPageSave {
    SaveRecord,
    /// `Update()` is `Update(true)`.
    Update(bool),
}

/// The running control OnValidate of a page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlOnValidate {
    /// The store's write mark (`Session::write_mark`) when it began.
    pub writes: (i64, u64),
    /// The store's change counter when it began: unlike the write mark, a Modify that changes no field moves it too.
    pub changes: u64,
    /// The `CurrPage.SaveRecord` and `CurrPage.Update` calls on the page since it began, in order, the running one last.
    pub saves: Vec<CurrPageSave>,
    /// The last record call of page code since it began that loaded a row into the page's Rec or moved it
    /// (`Session::note_page_rec_call`), `None` before the first.
    pub positioned: Option<RecCall>,
    /// On the page's new line: the SystemRowVersion of the row the table field stored for it when the OnValidate began
    /// (`code_inserted_extra.rs` `extra_code_row_rewritten`).
    pub row_version: Option<i64>,
}

/// A record call on the page's Rec: the method and, for `Find`, its argument (`None` for `Find()`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecCall {
    pub op: RecordOp,
    pub which: Option<String>,
}

impl ControlOnValidate {
    /// An OnValidate that begins when the store's write mark is `writes` and its change counter `changes`.
    pub fn new(writes: (i64, u64), changes: u64) -> Self {
        Self {
            writes,
            changes,
            saves: Vec::new(),
            positioned: None,
            row_version: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ControlOnValidate, CurrPageSave};

    #[test]
    fn a_new_onvalidate_has_no_saves() {
        let v = ControlOnValidate::new((3, 1), 7);
        assert_eq!((v.writes, v.changes), ((3, 1), 7));
        assert!(v.saves.is_empty() && v.positioned.is_none());
        assert_ne!(CurrPageSave::Update(true), CurrPageSave::Update(false));
    }
}
