//! A Rename to or from a blank or zero primary key value of a table that other tables relate to (al-runner issue 7937).
//!
//! The pinned corpus has no case of it. A table nothing relates to renames to zero (events/binding-senders/DbtTransactions.al:397
//! Rename_ToZeroAndNegative); that shape never reaches this module. The one shape with evidence is the reporter's repro on BC
//! online 29.1 (assumption rename-blank-key-with-relation): it raises BC's error. Every other shape that reaches this module is
//! refused until the probe measures it.

use crate::Session;
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
    /// Before the triggers of a Rename of `rec` (row `old` to `new`): the error of the reporter's shape, a refusal for every
    /// other rename that changes a primary key field to or from its blank value while other tables relate to `t`, and
    /// nothing for every other rename. `refs` are the relations to `t` (`rename_references`, empty for a temporary record).
    pub(crate) fn rename_blank_key(
        &self,
        cx: &AccessContext,
        rec: &Rec,
        t: &TableDef,
        refs: &[RenameRef],
        old: &[Value],
        new: &[Value],
    ) -> Result<(), Signal> {
        let blank_change = t.keys[0].iter().any(|&k| {
            let blank = Value::default_for(&t.fields[k].ty);
            let is_blank = |v: &Value| ops::compare(v, &blank).ok() == Some(Ordering::Equal);
            ops::compare(&old[k], &new[k]).ok() != Some(Ordering::Equal) && (is_blank(&old[k]) || is_blank(&new[k]))
        });
        if !blank_change {
            return Ok(());
        }
        let temporary = rec.borrow().temporary;
        let temp_refs;
        let refs = if temporary {
            temp_refs = self.rename_references(t)?;
            &temp_refs[..]
        } else {
            refs
        };
        if refs.is_empty() {
            return Ok(());
        }
        if !temporary && self.reporters_blank_rename(cx, rec, t, refs, old, new)? {
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
            "Rename of {}: a primary key field changes to or from a blank or zero key value while other tables relate to the table; \
             only one shape is measured (docs/probes-needed.md, rename-blank-key-with-relation)",
            t.name
        ))
    }

    /// The reporter's shape (al-runner issue 7937, `app/Repro.al` and `test/ReproTest.Codeunit.al`, BC online 29.1), every fact
    /// by equality: BC 29; a database record of a per-company table with a one-field Code primary key, no Caption on the table
    /// or the field, no table extension, no OnRename trigger, no subscriber to its events, the Global Triggers rename subscribers
    /// as measured (`global_rename_subscribers_as_measured`); its stored row's key is blank and the new key is not; one relation to it: a plain `TableRelation = <table>` of a
    /// validated Code field outside the primary key of another per-company table without extensions; one row holds the blank.
    fn reporters_blank_rename(
        &self,
        cx: &AccessContext,
        rec: &Rec,
        t: &TableDef,
        refs: &[RenameRef],
        old: &[Value],
        new: &[Value],
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
        let no_caption = |props: &[(String, String)]| {
            props
                .iter()
                .all(|(p, _)| !p.eq_ignore_ascii_case("Caption") && !p.eq_ignore_ascii_case("CaptionML"))
        };
        let master_facts = engine.target.as_ref().map(|v| v.bc.major) == Some(29)
            && t.per_company
            && matches!(t.fields[k].ty, RtType::Code(_))
            && no_caption(&master.properties)
            && no_caption(&master_schema.fields[k].properties)
            && master_schema.extensions.is_empty()
            && self.table_trigger(t, "OnRename").is_none()
            && !engine.events.any_subscriber_of(t.object as usize)
            && self.global_rename_subscribers_as_measured()
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
        let field = &child_schema.fields[pos];
        let arms = if field.relation_arms.is_empty() {
            &field.relation_runtime
        } else {
            &field.relation_arms
        };
        let child_facts = child_index != t.object as usize
            && child.per_company
            && child_schema.extensions.is_empty()
            && arms.len() == 1
            && arm.conditions.is_empty()
            && arm.wheres.as_deref() == Some(&[])
            && arm.field.is_none()
            && arm.assumes.is_empty()
            && engine.field_facts[child_index][pos].validate_relation
            && !child.keys[0].contains(&pos)
            && matches!(child.fields[pos].ty, RtType::Code(_));
        if !child_facts {
            return Ok(false);
        }
        let store_err = |e: al_data::StoreError| Signal::unsupported(format!("{e}"));
        let stored = self.store.get(cx, t, &t.primary_key(old)).map_err(store_err)?;
        let carrying = [FieldFilter {
            lang: al_builtins::filter::filter_language(),
            field: pos,
            expr: FilterExpr::Eq(Value::Code(String::new().into())),
            group: 0,
        }];
        let child_cx = self.cx_for(rec, child)?;
        Ok(stored.is_some() && self.store.count(&child_cx, child, &carrying).map_err(store_err)? == 1)
    }

    /// The subscribers of the Global Triggers rename events (`OnGlobalRename`, `OnDatabaseRename`) are none, or only Base
    /// Application's GlobalTriggerManagement (codeunit 49), which every BC environment with Base Application holds, the
    /// reporter's sandbox too (issue 7937). A subscriber of another object, or one declared but not compiled, is not measured.
    fn global_rename_subscribers_as_measured(&self) -> bool {
        let events = &self.engine.events;
        let program = &self.engine.program;
        ["onglobalrename", "ondatabaserename"].iter().all(|ev| {
            let key = (events.global_key, ev.to_string());
            !events.missing.contains_key(&key)
                && events.subscribers.get(&key).into_iter().flatten().all(|sub| {
                    let o = &program.objects[sub.object];
                    o.is_kind(al_ir::ObjKind::Codeunit)
                        && o.id == Some(49)
                        && program
                            .apps
                            .get(o.app as usize)
                            .is_some_and(|a| a.name == "Base Application" && a.publisher == "Microsoft")
                })
        })
    }
}
