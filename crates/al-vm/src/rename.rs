//! Rename propagation: renaming a primary key rewrites the fields of other tables whose
//! `TableRelation` points at the renamed row.
//!
//! Pinned by corpus record/TestRenamePropagation.al, TestRenamePropagationConditional.al and
//! TestRenameKeyRelation.al: a field follows the rename unless it has no relation or says
//! `ValidateTableRelation = false`; an `if` arm applies only to rows whose conditions hold, while
//! an `else` arm carries no condition at all; a `where()` on the related table limits which
//! renamed rows count; a referencing field inside the child's primary key re-keys the child, and
//! the child's own OnRename does not run.

use crate::Session;
use al_builtins::filter::{FieldFilter, FilterExpr};
use al_builtins::native::{Signal, unsupported};
use al_builtins::ops;
use al_builtins::value::Value;
use al_data::{RenameOutcome, Scan, Store, TableDef, WriteOutcome, position};
use al_ir as ir;
use std::cmp::Ordering;

/// A field (of table object `table`, at `pos`) whose relation arm `arm` points at the renamed table.
pub(crate) struct RenameRef {
    table: usize,
    pos: usize,
    arm: ir::RelationArm,
    /// The field's relation has this one arm.
    sole_arm: bool,
}

impl RenameRef {
    /// The referencing table object, the position of its field and the arm.
    pub(crate) fn parts(&self) -> (usize, usize, &ir::RelationArm) {
        (self.table, self.pos, &self.arm)
    }

    /// The field's relation has this one arm.
    pub(crate) fn sole_arm(&self) -> bool {
        self.sole_arm
    }

    /// The field of the renamed table `t` the arm relates to: the one it names, or the first primary key field.
    pub(crate) fn related_field(&self, t: &TableDef) -> usize {
        self.arm.field.unwrap_or(t.keys[0][0])
    }
}

impl Session {
    /// Every relation arm that points at table `t`, or why the rename cannot be done faithfully.
    pub(crate) fn rename_references(&self, t: &TableDef) -> Result<Vec<RenameRef>, Signal> {
        let name = t.name.to_lowercase();
        let mut refs = Vec::new();
        for (oi, o) in self.engine.program.objects.heads().iter().enumerate() {
            let Some(schema) = &o.table else { continue };
            for (pos, f) in schema.fields.iter().enumerate() {
                if !f.relation {
                    continue;
                }
                // The arms with `Field = filter(x)` conditions are kept apart (`relation_runtime`); a rename follows them as
                // validation does (assumption rename-follows-filter-condition-arms).
                let arms = if f.relation_arms.is_empty() {
                    &f.relation_runtime
                } else {
                    &f.relation_arms
                };
                if arms.is_empty() {
                    let mentions = f.properties.iter().any(|(k, v)| {
                        k.eq_ignore_ascii_case("TableRelation") && crate::recmeta::mentions_table(v, &name)
                    });
                    if mentions {
                        return unsupported(format!(
                            "Rename of {}: {}.{} relates to it through a TableRelation shape that is not understood yet",
                            t.name, o.name, f.name
                        ));
                    }
                    continue;
                }
                for arm in arms {
                    match arm.table {
                        Some(a) if a == t.object as usize => {
                            // A `Field = filter(<> X)` arm is pinned (record/table-relation/TableRelationArms.al:192
                            // Rename_FollowsOnlyTheArmsThatApply: the rows the filter takes follow the rename, the others
                            // stay); any other filter form is the assumption.
                            if arm.conditions.iter().any(|c| c.filter && !single_not_equal(&c.value)) {
                                al_builtins::assumptions::assume(
                                    al_builtins::assumptions::Assumption::RenameFollowsFilterConditionArms,
                                );
                            }
                            if arm.wheres.is_none() {
                                return unsupported(format!(
                                    "Rename of {}: the where() of {}.{}'s relation is not understood yet",
                                    t.name, o.name, f.name
                                ));
                            }
                            for a in &arm.assumes {
                                al_builtins::assumptions::assume(*a);
                            }
                            refs.push(RenameRef {
                                table: oi,
                                pos,
                                arm: arm.clone(),
                                sole_arm: arms.len() == 1,
                            });
                        }
                        // An arm whose target the workspace does not declare may still be this table
                        // through a name we could not resolve.
                        None if crate::recmeta::mentions_table(&arm.table_name, &name) => {
                            return unsupported(format!(
                                "Rename of {}: {}.{} relates to a table of the same name this workspace cannot resolve",
                                t.name, o.name, f.name
                            ));
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(refs)
    }

    /// After the row at `old` became `new`: rewrites every referencing row.
    pub(crate) fn propagate_rename(
        &mut self,
        rec: &crate::records::Rec,
        t: &TableDef,
        refs: &[RenameRef],
        old: &[Value],
        new: &[Value],
    ) -> Result<(), Signal> {
        for r in refs {
            let Some(child) = self.engine.tables[r.table].clone() else {
                continue;
            };
            let cx = self.cx_for(rec, &child)?;
            // The rename of a row of a table that is not per company reaches the referencing rows of
            // every company (record/TestRenamePropagationCompany.al:5-8); only the record's own
            // company is walked here, so rows of another company are refused.
            if !t.per_company
                && child.per_company
                && self
                    .store
                    .companies_with_rows(child.object)
                    .iter()
                    .any(|c| !c.eq_ignore_ascii_case(&cx.company))
            {
                return unsupported(format!(
                    "Rename of {}: {} holds rows in another company, which the rename must also walk (not modelled)",
                    t.name, child.name
                ));
            }
            let engine = self.engine.clone();
            let schema = engine.program.objects[r.table].table.as_ref().expect("table");
            if !self.engine.field_facts[r.table][r.pos].validate_relation {
                continue;
            }
            let rel = r.related_field(t);
            if rename_equal(&old[rel], &new[rel])? {
                continue;
            }
            // The renamed row must pass the arm's where() on the related table (the field() links are per child row).
            let wheres = r.arm.wheres.clone().unwrap_or_default();
            let mut parent_ok = true;
            for w in &wheres {
                if let ir::RelationWhere::Value { related, text } = w {
                    let expr = self
                        .parse_filter_text(text, &t.fields[*related].ty)
                        .map_err(|e| Signal::unsupported(format!("where() of a relation: {e}")))?;
                    // A value the filter cannot compare does not count as failing it (hard rule 4; issue 6924).
                    if let Some(e) = expr.compare_error(&new[*related]) {
                        return Err(rename_refusal(e));
                    }
                    parent_ok &= expr.matches(&new[*related])?;
                }
            }
            if !parent_ok {
                continue;
            }
            let old_value = ops::convert(old[rel].clone(), &child.fields[r.pos].ty)?;
            let new_value = ops::convert(new[rel].clone(), &child.fields[r.pos].ty)?;
            // The rows carrying the old value, snapshotted before any is rewritten.
            let filters = [FieldFilter {
                lang: al_builtins::filter::filter_language(),
                field: r.pos,
                expr: FilterExpr::Eq(old_value),
                group: 0,
            }];
            let mut rows = Vec::new();
            let mut after: Option<Vec<Value>> = None;
            loop {
                let next = self
                    .store
                    .next(
                        &cx,
                        &child,
                        &Scan {
                            key: 0,
                            filters: &filters,
                            ascending: true,
                            after: after.as_deref(),
                        },
                    )
                    .map_err(|e| Signal::unsupported(format!("{e}")))?;
                let Some(row) = next else { break };
                after = Some(position(&child, 0, &row));
                rows.push(row);
            }
            for row in rows {
                // An `if` arm applies to rows whose conditions hold; a condition on a FlowFilter
                // field is not evaluated.
                let mut holds = true;
                for c in &r.arm.conditions {
                    if ir::FieldFacts::of(&schema.fields[c.field]).class == ir::FieldClass::FlowFilter {
                        continue;
                    }
                    if !self.relation_cond_holds(&child, &schema.fields[r.pos].name, c, &row[c.field])? {
                        holds = false;
                        break;
                    }
                }
                if !holds {
                    continue;
                }
                let mut linked = true;
                for w in &wheres {
                    if let ir::RelationWhere::Field { related, own } = w {
                        let want = ops::convert(row[*own].clone(), &t.fields[*related].ty)?;
                        linked &= rename_equal(&new[*related], &want)?;
                    }
                }
                if !linked {
                    continue;
                }
                let old_pk = child.primary_key(&row);
                let mut changed = row.clone();
                changed[r.pos] = new_value.clone();
                if let Some(i) = child.fields.iter().position(|f| f.id == 2_000_000_005) {
                    self.next_row_version += 1;
                    // A row version of the database the open transaction's Inserts and Modifies do not account for
                    // (`Session::last_used_row_version`).
                    self.tx.row_versions.handed_out += 1;
                    self.tx.row_versions.last = self.next_row_version;
                    changed[i] = Value::BigInteger(self.next_row_version);
                }
                self.stamp_row_modified(&child, &mut changed);
                let changed = self.stored_view(&child, &changed);
                let in_key = child.keys[0].contains(&r.pos);
                let done = if in_key {
                    match self
                        .store
                        .rename(&cx, &child, &old_pk, changed)
                        .map_err(|e| Signal::unsupported(format!("{e}")))?
                    {
                        RenameOutcome::Renamed => true,
                        RenameOutcome::NotFound => true,
                        RenameOutcome::AlreadyExists => false,
                    }
                } else {
                    matches!(
                        self.store
                            .modify(&cx, &child, changed)
                            .map_err(|e| Signal::unsupported(format!("{e}")))?,
                        WriteOutcome::Done | WriteOutcome::NotFound
                    )
                };
                if !done {
                    return unsupported(format!(
                        "Rename of {}: re-keying a related row of {} collides with an existing row (not pinned)",
                        t.name, child.name
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Whether a `filter(...)` condition's text is one `<> value` (no `&`, `|`, range or wildcard).
fn single_not_equal(v: &al_ir::Const) -> bool {
    match v {
        al_ir::Const::Text(t) => t.trim().strip_prefix("<>").is_some_and(|rest| {
            !rest.trim().is_empty() && !rest.contains(['&', '|', '.', '<', '>', '=', '*', '?', '@', '(', ')'])
        }),
        _ => false,
    }
}

/// Whether two values of a relation are equal; a comparison that cannot be answered is refused, never counted as "differ"
/// (hard rule 4; issue 6924).
fn rename_equal(a: &Value, b: &Value) -> Result<bool, Signal> {
    ops::compare(a, b).map(|o| o == Ordering::Equal).map_err(rename_refusal)
}

fn rename_refusal(e: Signal) -> Signal {
    // A refusal keeps its key behind the prefix (ADR-071).
    let why: al_builtins::refusal::Refusal = match e {
        Signal::Unsupported(why) => why,
        Signal::Error(e) => e.message.into(),
        _ => "the comparison did not give an order".into(),
    };
    Signal::Unsupported(why.map_reason(|why| format!("a rename cannot evaluate a TableRelation condition: {why}")))
}
