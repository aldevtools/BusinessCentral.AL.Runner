//! A Rename through a stale record: another record variable changed and modified the row after this variable read it
//! (al-runner issue 7950).
//!
//! Business Central raises DB:RecordChanged with the stale-write text (`Session::record_changed`) and leaves the rows as they
//! were: record/table-relation/RenameBlankKeyRelation.al, the probe runner's Mode 5 (`:887`) and the cells `:2920` (X to Y)
//! and `:2894` (from the blank key, where this error comes and not the blank-key error of `rename_blank_key.rs`), the same
//! on all three majors. The `:line` citations are lines of that file.
//!
//! The error is raised only in the measured shape (`Session::measured_stale_rename`). Every other Rename through a stale
//! record that holds a committed row version is refused. Main's rename stays for a record that is not stale and for one
//! that read a row version written in the open transaction (pinned by record/filters/P68H_StaleModify.al:108-132).

use crate::Session;
use crate::globaltriggers::GlobalOp;
use crate::records::Rec;
use crate::rename::RenameRef;
use crate::state::TxWriteKind;
use al_builtins::native::{Signal, unsupported};
use al_builtins::value::{DirtyBase, RowOrigin, RtType, Value};
use al_data::{AccessContext, CodeStorage, Store, TableDef};
use al_ir::{FieldClass, RelationWhere};

/// The SystemRowVersion in `fields` (0 when it holds none); `None` for a table without the field.
pub(crate) fn row_version(t: &TableDef, fields: &[Value]) -> Option<i64> {
    let i = t.fields.iter().position(|f| f.id == 2_000_000_005)?;
    match fields.get(i) {
        Some(Value::BigInteger(n)) => Some(*n),
        _ => Some(0),
    }
}

impl Session {
    /// Before the triggers of a Rename of `rec` from `old` to `new` (before any write): `Ok` when the record is not stale (a
    /// temporary record, a record that was never read, a row that is not stored or not newer than the record) or read a row
    /// version written in the open transaction, else the
    /// RecordChanged error in the measured shape and a refusal outside it. `from_al`: AL code called Rename
    /// (`TxState::rename_from_al`); `own_note`: the length of `TxState::writes` before the Rename's own note; `refs`: the
    /// relations to `t` (`rename_references`).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn stale_rename(
        &self,
        cx: &AccessContext,
        rec: &Rec,
        t: &TableDef,
        refs: &[RenameRef],
        old: &[Value],
        new: &[Value],
        from_al: bool,
        own_note: usize,
    ) -> Result<(), Signal> {
        if rec.borrow().temporary || self.crm_connection_of(rec).is_some() {
            return Ok(());
        }
        let Some(mine) = row_version(t, old) else {
            return Ok(());
        };
        if mine == 0 {
            return Ok(());
        }
        let old_pk = Self::sql_key(rec, t, old);
        let store_err = |e: al_data::StoreError| Signal::unsupported(format!("{e}"));
        let Some(stored) = self.store.get(cx, t, &old_pk).map_err(store_err)? else {
            return Ok(());
        };
        let theirs = row_version(t, &stored).unwrap_or(0);
        if theirs <= mine {
            return Ok(());
        }
        // A record that read a row version written in the open transaction keeps main's rename: one such shape is pinned, a
        // Get of a row inserted in the open transaction, another variable's Modify, then the Rename, which writes the
        // record's values under the new key (corpus record/filters/P68H_StaleModify.al:108-132).
        if mine > self.tx.committed_row_version {
            return Ok(());
        }
        let new_stored = self
            .store
            .get(cx, t, &Self::sql_key(rec, t, new))
            .map_err(store_err)?
            .is_some();
        let measured = self
            .measured_stale_rename(rec, t, &old_pk, mine, new_stored, from_al, own_note)
            .and_then(|()| self.measured_stale_table(t, refs));
        match measured {
            Ok(()) => Err(self.record_changed(t, rec)?),
            Err(what) => unsupported(format!(
                "Rename of {} through a stale record (another variable wrote the row after this one read it), in a shape that \
                 is not measured ({what}; RenameBlankKeyRelation.al:887 measures one)",
                t.name
            )),
        }
    }

    /// Whether the stale Rename is the measured shape (:887 Mode 5, :2894, :2920). Every fact is required as measured.
    #[allow(clippy::too_many_arguments)]
    fn measured_stale_rename(
        &self,
        rec: &Rec,
        t: &TableDef,
        old_pk: &[Value],
        mine: i64,
        new_stored: bool,
        from_al: bool,
        own_note: usize,
    ) -> Result<(), &'static str> {
        let r = rec.borrow();
        // The call: AL code calls the statement form of Rename on a Record variable (`WhereMaster.Rename(NewCode)`, :887),
        // in the OnRun of the one guarded `Codeunit.Run` running (`if Runner.Run()`, :1646), with no TryFunction between.
        let tx = &self.tx;
        if !(from_al && !self.ret_used && !self.through_recordref) {
            return Err("a Rename that is not the statement form called by AL code on a Record variable");
        }
        if !(tx.guarded_runs.len() == 1
            && tx.guarded_runs[0] + 1 == self.call_stack.len()
            && tx.try_depth == 0
            && tx.used_tries.is_empty())
        {
            return Err("a Rename that is not in the OnRun of the one guarded Codeunit.Run running");
        }
        // The record: the session company, no filter (`WhereMaster` is a plain local variable, :784).
        if !(r.company.is_none() && r.filters.is_empty()) {
            return Err("a record of another company or with a filter");
        }
        // The record read the committed row by Get in the open transaction and no field was written since
        // (`WhereMaster.Get(OldCode)` right after the test's Commit, :887 and the test's `RunRename`).
        let read_by_get = r.row_origin == Some(RowOrigin::new(false, mine, self.commits));
        if !(read_by_get && matches!(r.is_dirty_base, DirtyBase::Synced)) {
            return Err("a record that did not get its row by Get in the open transaction, or that changed since");
        }
        // The new key is not stored (:2894 '' to X, :2920 X to Y); a Rename to the old key finds it stored too.
        if new_stored {
            return Err("a new key that is stored, or the old key");
        }
        // The table: per company, no table extension, no OnRename trigger, no subscriber, the database rename trigger
        // known to be off (table 79114 "RBK Where Master", :183-192).
        let object = t.object as usize;
        let no_extension = self.engine.program.objects[object]
            .table
            .as_ref()
            .is_some_and(|s| s.extensions.is_empty());
        if !(t.per_company
            && no_extension
            && self.table_trigger(t, "OnRename").is_none()
            && !self.engine.events.any_subscriber_of(object)
            && self.global_known_off(t, GlobalOp::Rename))
        {
            return Err(
                "a table that is not per company, or has a table extension, an OnRename trigger, a subscriber or \
                        the database rename trigger on",
            );
        }
        // What made it stale: since the last Commit the only write is one Record Modify of this row (`StaleCopy.Modify()`,
        // :887), beside the Rename's own note, and no lock or rollback.
        let one_modify = match &tx.writes[..own_note] {
            [w] => {
                w.kind == TxWriteKind::Modify
                    && w.bulk.is_none()
                    && !w.through_recordref
                    && w.table == t.object
                    && w.company.is_none()
                    && crate::records::same_key(&w.pk, old_pk)
            }
            _ => false,
        };
        if !(one_modify
            && tx.writes.len() == own_note + 1
            && self.writes_all_noted()
            && !tx.rolled_back
            && !tx.lock_tx
            && tx.locked_tables.is_empty()
            && !tx.recref_lock_tx
            && tx.lock_unpinned.is_none()
            && !self.locking_read_in_tx())
        {
            return Err(
                "a record made stale by something other than one Modify of its row by another variable since \
                        the last Commit, or after a lock or a rollback",
            );
        }
        Ok(())
    }
    /// The tables of the measured shape (:183-201): the renamed table declares its primary key only, one Code[20] key field
    /// without a property, no table property but DataClassification; one relation in the program targets it, the sole arm of a validated
    /// Normal Code[20] field without a property other than TableRelation, outside the primary key of another per-company
    /// table of the same app that declares its primary key only and has no extension, trigger, subscriber or table property
    /// but DataClassification,
    /// with one `const` where() on a field of the renamed table outside its key (`TableRelation = "RBK Where Master"
    /// where(Name = const('N'))`, :200).
    fn measured_stale_table(&self, t: &TableDef, refs: &[RenameRef]) -> Result<(), &'static str> {
        let engine = &self.engine;
        let object = t.object as usize;
        // Both tables state DataClassification and nothing else (:185, :196).
        let declares_none = |o: usize| {
            engine.program.objects[o]
                .properties
                .iter()
                .all(|(p, _)| p.eq_ignore_ascii_case("DataClassification"))
        };
        let [k] = t.keys[0][..] else {
            return Err("a primary key of more than one field");
        };
        let key_plain = t.keys.len() == 1
            && t.fields[k].ty == RtType::Code(Some(20))
            && t.fields[k].code_storage == CodeStorage::Text
            && engine.program.objects[object]
                .table
                .as_ref()
                .is_some_and(|s| s.fields[k].properties.is_empty() && !s.fields[k].auto_increment)
            && self.caption_refused(object).is_none()
            && declares_none(object);
        if !key_plain {
            return Err(
                "a key other than one Code[20] field without a property, a secondary key, or a table property other than DataClassification",
            );
        }
        let [r] = refs else {
            return Err("not exactly one relation targets the table");
        };
        let (child_index, pos, arm) = r.parts();
        let (Some(child), Some(child_schema)) = (
            engine.tables[child_index].as_ref(),
            engine.program.objects[child_index].table.as_ref(),
        ) else {
            return Err("a related table without a schema");
        };
        let facts = engine.field_facts[child_index][pos];
        let const_where = matches!(
            arm.wheres.as_deref().unwrap_or_default(),
            [RelationWhere::Value { related, filter: false, .. }] if *related != k
        );
        let arm_measured =
            r.sole_arm() && arm.assumes.is_empty() && arm.conditions.is_empty() && arm.field.is_none() && const_where;
        let field_measured = facts.class == FieldClass::Normal
            && facts.validate_relation
            && child.fields[pos].ty == RtType::Code(Some(20))
            && !child.keys[0].contains(&pos)
            && child_schema.fields[pos]
                .properties
                .iter()
                .all(|(p, _)| p.eq_ignore_ascii_case("TableRelation"));
        let table_measured = child_index != object
            && child.per_company
            && child.keys.len() == 1
            && child_schema.extensions.is_empty()
            && declares_none(child_index)
            && engine.program.objects[child_index].app == engine.program.objects[object].app
            && ["OnInsert", "OnModify", "OnDelete", "OnRename"]
                .iter()
                .all(|n| self.table_trigger(child, n).is_none())
            && !engine.events.any_subscriber_of(child_index);
        if arm_measured && field_measured && table_measured {
            Ok(())
        } else {
            Err("a relation to the table other than the measured one")
        }
    }
}
