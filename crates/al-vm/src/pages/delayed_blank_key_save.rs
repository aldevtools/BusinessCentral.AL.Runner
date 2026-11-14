//! The save of a new row on a DelayedInsert page whose NotBlank primary key field is still blank.
//!
//! The corpus pins the save of a DelayedInsert row that fails on its typed key (testpage/TestPageFailedInsert_Tests.al:1-30, a
//! duplicate key) and the save of a blank key without NotBlank (codeunits/run-onrun/BlankKeyRowNextTest.al:319
//! H1_Write_DelayedCardOpenNewSetValue, a Guid key). It does not pin a blank key whose table field has NotBlank = true: an
//! outside reporter's BC run shows a validation error 'Code must be filled in.' after `New()`, a SetValue on another field and
//! `Next()` on a List (al-runner issue 6924), and neither the text, the step that raises it nor whether the row is inserted is
//! measured. Such a save is refused before it inserts only when nothing can fill the key on the way to the write (see
//! `key_can_be_filled_on_insert`); with insert-time code the save keeps main's behavior (not measured either,
//! docs/probes-needed.md row "TestPage, DelayedInsert new row").

use super::*;
use crate::globaltriggers::GlobalOp;

impl Session {
    /// Refuses the insert of the new row `st` holds when the page has DelayedInsert = true (the page or an extension), a
    /// primary key field of the source table has NotBlank = true and holds its blank value, and nothing can fill the key
    /// before the write.
    pub(super) fn refuse_delayed_blank_not_blank_key(&mut self, st: &St) -> Result<(), Signal> {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let delayed = effective_prop(&engine, object, "DelayedInsert")?.is_some_and(|v| v.eq_ignore_ascii_case("true"));
        if !delayed {
            return Ok(());
        }
        let rec = self.page_rec(st);
        let Some(field) = blank_not_blank_key(&engine, &rec) else {
            return Ok(());
        };
        if self.key_can_be_filled_on_insert(st, &rec)? {
            return Ok(());
        }
        unsupported(format!(
            "saving a new row of a DelayedInsert page while its NotBlank primary key field '{field}' is blank and no insert code \
             can fill it (the validation error and whether the row is inserted are not pinned)"
        ))
    }

    /// Whether code or the platform can run between the save and the write of `rec`: the page's or a page extension's
    /// OnInsertRecord, an AutoSplitKey on the page, an AutoIncrement field of the table, the table's OnInsert, a table
    /// extension's OnBeforeInsert or OnInsert, a subscriber (bound or not) to the table's OnBeforeInsertEvent or
    /// OnAfterInsertEvent, or the database insert trigger (`Global Triggers` OnDatabaseInsert) on for the table. The database
    /// trigger setup is asked last, only when nothing else can run: the insert asks it then anyway, with no AL code between.
    fn key_can_be_filled_on_insert(&mut self, st: &St, rec: &Rec) -> Result<bool, Signal> {
        let engine = self.engine();
        let t = self.table(rec)?;
        let page = page_of(&engine, st);
        let page_code = page.triggers.iter().any(|(n, _)| n == "oninsertrecord")
            || page.ext_triggers.iter().any(|(n, _, _)| n == "oninsertrecord");
        let auto_increment = engine.program.objects[t.object as usize]
            .table
            .as_ref()
            .is_some_and(|schema| schema.fields.iter().any(|f| f.auto_increment));
        let table_code = self.table_trigger(&t, "OnInsert").is_some()
            || !self.ext_triggers(&t, "OnBeforeInsert").is_empty()
            || !self.ext_triggers(&t, "OnInsert").is_empty();
        let subscribed = ["onbeforeinsertevent", "onafterinsertevent"]
            .iter()
            .any(|e| engine.events.is_subscribed(t.object as usize, e));
        if page_code || self.has_auto_split_key(st) || auto_increment || table_code || subscribed {
            return Ok(true);
        }
        // A temporary record raises no database event and asks no setup (globaltriggers.rs `global_trigger`).
        if rec.borrow().temporary {
            return Ok(false);
        }
        self.global_enabled(&t, GlobalOp::Insert)
    }
}

/// The name of the first primary key field of `rec`'s table whose table field has NotBlank = true and that holds its blank
/// value.
fn blank_not_blank_key(engine: &Engine, rec: &Rec) -> Option<String> {
    let r = rec.borrow();
    let table = engine.program.objects[r.table as usize].table.as_ref()?;
    table.keys.first()?.iter().find_map(|&k| {
        let field = &table.fields[k];
        let not_blank = field
            .properties
            .iter()
            .any(|(n, v)| n.eq_ignore_ascii_case("NotBlank") && v.trim().eq_ignore_ascii_case("true"));
        let blank = al_builtins::ops::compare(&r.fields[k], &Value::default_for(&field.ty)).ok()
            == Some(std::cmp::Ordering::Equal);
        (not_blank && blank).then(|| field.name.clone())
    })
}
