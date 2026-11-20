//! What a page records while the OnValidate trigger of one of its controls runs (`Session::run_control_onvalidate`): the store's
//! write mark and change counter when it began, and the `CurrPage.SaveRecord` and `CurrPage.Update` calls on the page since. A
//! nested OnValidate records its own and gives the outer one back when it ends.

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
}

impl ControlOnValidate {
    /// An OnValidate that begins when the store's write mark is `writes` and its change counter `changes`.
    pub fn new(writes: (i64, u64), changes: u64) -> Self {
        Self {
            writes,
            changes,
            saves: Vec::new(),
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
        assert!(v.saves.is_empty());
        assert_ne!(CurrPageSave::Update(true), CurrPageSave::Update(false));
    }
}
