//! A Rename that changes a primary key field to or from its blank or zero value while a field of another table relates to that
//! key field (al-runner issue 7937).
//!
//! The pinned corpus has no case of it. A table nothing relates to renames to zero (events/binding-senders/DbtTransactions.al:397
//! Rename_ToZeroAndNegative), and a relation to another field (Base Application renames "Sales Invoice Entity Aggregate" from
//! Posted = false while Gen. Journal Line relates to its Id) keeps main's rename: neither enters this module's path. The one
//! shape with evidence is the reporter's repro on BC online 29.1 (assumption rename-blank-key-with-relation): it raises BC's
//! error. Every other shape that enters is refused until the probe (corpus pull request 1370) measures it.

use crate::Session;
use crate::globaltriggers::GlobalOp;
use crate::records::Rec;
use crate::rename::RenameRef;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::filter::{FieldFilter, FilterExpr};
use al_builtins::native::{AlError, Signal, UNPINNED_ERROR, unsupported};
use al_builtins::ops;
use al_builtins::value::{RtType, Value};
use al_data::{AccessContext, Store, TableDef};
use std::cmp::Ordering;

/// The second paragraph of the error, as the reporter measured it (issue 7937); the two LF characters before it are part of
/// the assumption.
const RELATION_RENAME_TEXT: &str =
    "When a field has a relation defined, the field cannot be renamed to and from zero or an empty value.";

impl Session {
    /// Before the triggers of a Rename of `rec` (row `old` to `new`): when a field of another table relates to a primary key
    /// field of `t` (`RenameRef::related_field`) and that field changes to or from its blank value, the error of the reporter's
    /// shape or a refusal. Every other rename is left alone. `refs` are the relations to `t` (`rename_references`, empty for a
    /// temporary record); `from_al`: AL code called Rename on the record (`TxState::rename_from_al`).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn rename_blank_key(
        &self,
        cx: &AccessContext,
        rec: &Rec,
        t: &TableDef,
        refs: &[RenameRef],
        old: &[Value],
        new: &[Value],
        from_al: bool,
    ) -> Result<(), Signal> {
        let blank_changes: Vec<usize> = t.keys[0]
            .iter()
            .copied()
            .filter(|&k| changes_to_or_from_blank(t, k, old, new))
            .collect();
        if blank_changes.is_empty() {
            return Ok(());
        }
        let temporary = rec.borrow().temporary;
        let temp_refs;
        let refs = if temporary {
            temp_refs = self.relation_refs_unfollowed(t)?;
            &temp_refs[..]
        } else {
            refs
        };
        if !refs.iter().any(|r| blank_changes.contains(&r.related_field(t))) {
            return Ok(());
        }
        if !temporary && self.reporters_blank_rename(cx, rec, t, refs, old, new, from_al)? {
            assume(Assumption::RenameBlankKeyWithRelation);
            let k = t.keys[0][0];
            return Err(Signal::Error(Box::new(AlError {
                message: format!(
                    "The {} field on the {} table cannot be renamed from \"\" to \"{}\".\n\n{RELATION_RENAME_TEXT}",
                    t.fields[k].name,
                    t.name,
                    al_builtins::native::text_of(&new[k]).unwrap_or_default()
                ),
                code: UNPINNED_ERROR.into(),
                call_stack: String::new(),
                ..Default::default()
            })));
        }
        unsupported(format!(
            "Rename of {}: a primary key field another table relates to changes to or from a blank or zero key value; \
             only one shape is measured (docs/probes-needed.md, rename-blank-key-with-relation)",
            t.name
        ))
    }

    /// The reporter's shape (al-runner issue 7937, `app/Repro.al` and `test/ReproTest.Codeunit.al`, BC online 29.1), every fact
    /// by equality:
    /// - the call: the statement form of `Rename` that AL code calls on a Record variable of a database record (not its
    ///   return value, not RecordRef, not the page client's rename at close);
    /// - the renamed table: per company, its primary key one Code field, no Caption on the table or the field, no table
    ///   extension, no OnRename trigger, no subscriber to its events, its database rename trigger known to be off (`global_known_off`:
    ///   no app subscribes to Global Triggers, or the setup an earlier write asked leaves Rename off; the reporter's sandbox had
    ///   no Change Log setup);
    /// - the rows: the record holds the stored row of the old key (the same SystemId, as after the reporter's Insert); that key
    ///   is blank, the new one is not and no row holds it;
    /// - the one relation to it: a plain `TableRelation = <table>` (one arm, no condition, no where(), no related field named)
    ///   of a validated Normal Code field of the key's length, outside the primary key of another per-company table without
    ///   extensions; that table holds one row, and it holds the blank.
    #[allow(clippy::too_many_arguments)]
    fn reporters_blank_rename(
        &self,
        cx: &AccessContext,
        rec: &Rec,
        t: &TableDef,
        refs: &[RenameRef],
        old: &[Value],
        new: &[Value],
        from_al: bool,
    ) -> Result<bool, Signal> {
        let engine = &self.engine;
        let master = &engine.program.objects[t.object as usize];
        let Some(master_schema) = &master.table else {
            return Ok(false);
        };
        let ([k], [r]) = (t.keys[0].as_slice(), refs) else {
            return Ok(false);
        };
        let k = *k;
        let call_facts = from_al && !self.ret_used && !self.through_recordref;
        let no_caption = |props: &[(String, String)]| {
            props
                .iter()
                .all(|(p, _)| !p.eq_ignore_ascii_case("Caption") && !p.eq_ignore_ascii_case("CaptionML"))
        };
        let master_facts = call_facts
            && t.per_company
            && matches!(t.fields[k].ty, RtType::Code(_))
            && no_caption(&master.properties)
            && no_caption(&master_schema.fields[k].properties)
            && master_schema.extensions.is_empty()
            && self.table_trigger(t, "OnRename").is_none()
            && !engine.events.any_subscriber_of(t.object as usize)
            && self.global_known_off(t, GlobalOp::Rename)
            && matches!(&old[k], Value::Code(s) if s.is_empty())
            && matches!(&new[k], Value::Code(s) if !s.is_empty());
        if !master_facts {
            return Ok(false);
        }
        let (child_index, pos, arm) = r.parts();
        let Some(child) = engine.tables[child_index].as_ref() else {
            return Ok(false);
        };
        let Some(child_schema) = &engine.program.objects[child_index].table else {
            return Ok(false);
        };
        let facts = engine.field_facts[child_index][pos];
        let child_facts = child_index != t.object as usize
            && child.per_company
            && child_schema.extensions.is_empty()
            && r.sole_arm()
            && arm.conditions.is_empty()
            && arm.wheres.as_deref() == Some(&[])
            && arm.field.is_none()
            && arm.assumes.is_empty()
            && facts.validate_relation
            && facts.class == al_ir::FieldClass::Normal
            && !child.keys[0].contains(&pos)
            && child.fields[pos].ty == t.fields[k].ty;
        if !child_facts {
            return Ok(false);
        }
        let store_err = |e: al_data::StoreError| Signal::unsupported(format!("{e}"));
        let Some(system_id) = t.fields.iter().position(|f| f.id == 2_000_000_000) else {
            return Ok(false);
        };
        let stored = self.store.get(cx, t, &t.primary_key(old)).map_err(store_err)?;
        let holds_stored_row =
            stored.is_some_and(|row| ops::compare(&row[system_id], &old[system_id]).ok() == Some(Ordering::Equal));
        if !holds_stored_row || self.store.get(cx, t, &t.primary_key(new)).map_err(store_err)?.is_some() {
            return Ok(false);
        }
        let carrying = [FieldFilter {
            lang: al_builtins::filter::filter_language(),
            field: pos,
            expr: FilterExpr::Eq(Value::Code(String::new().into())),
            group: 0,
        }];
        let child_cx = self.cx_for(rec, child)?;
        Ok(self.store.count(&child_cx, child, &[]).map_err(store_err)? == 1
            && self.store.count(&child_cx, child, &carrying).map_err(store_err)? == 1)
    }
}

/// Primary key field `k` of `t` holds a different value in `new` than in `old`, and one of the two is its type's blank or zero.
fn changes_to_or_from_blank(t: &TableDef, k: usize, old: &[Value], new: &[Value]) -> bool {
    let blank = Value::default_for(&t.fields[k].ty);
    let is_blank = |v: &Value| ops::compare(v, &blank).ok() == Some(Ordering::Equal);
    matches!(
        ops::compare(&old[k], &new[k]).ok(),
        Some(Ordering::Less | Ordering::Greater)
    ) && (is_blank(&old[k]) || is_blank(&new[k]))
}
