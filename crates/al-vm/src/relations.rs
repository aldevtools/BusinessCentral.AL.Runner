//! `TableRelation`: which table a field relates to (`FieldRef.Relation`) and whether a value
//! exists there (`Validate`).
//!
//! Pinned by corpus recordref-fieldref/fieldref/TestFieldRefRelation.al and TestFieldRefRelationFieldClass.al: the
//! first arm whose conditions hold applies, an else arm has none, a condition on a FlowFilter
//! field is not evaluated, `where(A = field(B))` reads B off the referencing row, and a value the
//! related table lacks is refused with "... cannot be found in the related table".

use crate::Session;
use al_builtins::filter::{FieldFilter, FilterExpr};
use al_builtins::native::{AlError, Signal, unsupported};
use al_builtins::ops;
use al_builtins::value::{RecordState, Value};
use al_data::{Store, TableDef};
use al_ir as ir;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::rc::Rc;

type Rec = Rc<RefCell<RecordState>>;

/// `SystemCreatedBy` relates to the `User` table (corpus record/TestFieldDataClassificationVirtualTable.al:92-100).
const USER_TABLE: i32 = 2_000_000_120;
const SYSTEM_CREATED_BY: u32 = 2_000_000_002;
const SYSTEM_MODIFIED_BY: u32 = 2_000_000_004;

impl Session {
    fn is_flow_field(&self, table: u32, pos: usize) -> bool {
        self.engine.field_facts[table as usize][pos].class == ir::FieldClass::FlowField
    }

    fn is_flow_filter(&self, table: u32, pos: usize) -> bool {
        self.engine.field_facts[table as usize][pos].class == ir::FieldClass::FlowFilter
    }

    /// How many rows of the related table pass `filters` on stored fields and `flow_field_filters` on FlowFields, each FlowField
    /// calculated for the row (stops at the first row that passes).
    fn count_passing_flow_fields(
        &mut self,
        cx: &al_data::AccessContext,
        related: &TableDef,
        filters: &[FieldFilter],
        flow_field_filters: &[FieldFilter],
    ) -> Result<usize, Signal> {
        let mut rows = Vec::new();
        self.each_row_of(None, cx, related, filters, |row| {
            rows.push(row.to_vec());
            Ok(())
        })?;
        let Value::Record(probe) = self.default_value(&al_builtins::value::RtType::Record(related.object, false))
        else {
            return Err(Signal::Unsupported("a record of the related table".into()));
        };
        for row in rows {
            if self.passes_flow_field_filters(&probe, &row, flow_field_filters)? {
                return Ok(1);
            }
        }
        Ok(0)
    }

    /// The FlowFields of `t` that a condition of field `pos`'s relation names.
    fn flow_field_conditions(&self, t: &TableDef, pos: usize) -> Vec<usize> {
        let Some(schema) = self.engine.program.objects[t.object as usize].table.as_ref() else {
            return Vec::new();
        };
        let field = &schema.fields[pos];
        let mut out: Vec<usize> = Vec::new();
        for arm in field.relation_arms.iter().chain(field.relation_runtime.iter()) {
            for c in &arm.conditions {
                if self.is_flow_field(t.object, c.field) && !out.contains(&c.field) {
                    out.push(c.field);
                }
            }
        }
        out
    }

    /// Whether the relation of field `pos` has an `if (...)` condition or more than one arm.
    fn relation_is_conditional(&self, t: &TableDef, pos: usize) -> bool {
        let Some(schema) = self.engine.program.objects[t.object as usize].table.as_ref() else {
            return false;
        };
        let field = &schema.fields[pos];
        let arms = || field.relation_arms.iter().chain(field.relation_runtime.iter());
        arms().count() > 1 || arms().any(|arm| !arm.conditions.is_empty())
    }

    /// The arm of field `pos`'s relation that applies to the buffer, if any.
    pub(crate) fn relation_arm(&self, rec: &Rec, t: &TableDef, pos: usize) -> Result<Option<ir::RelationArm>, Signal> {
        let schema = self.engine.program.objects[t.object as usize]
            .table
            .as_ref()
            .ok_or_else(|| Signal::Unsupported("relation of a table without a schema".into()))?;
        let field = &schema.fields[pos];
        if !field.relation {
            return Ok(None);
        }
        // The arms with `Field = filter(x)` conditions are kept apart (`relation_runtime`) and apply here only.
        let arms = if field.relation_arms.is_empty() {
            &field.relation_runtime
        } else {
            &field.relation_arms
        };
        if arms.is_empty() {
            return unsupported(format!(
                "the TableRelation of {}.{} has a shape that is not understood yet",
                t.name, field.name
            ));
        }
        let r = rec.borrow();
        for arm in arms {
            let mut holds = true;
            for c in &arm.conditions {
                // BC evaluates conditions on Normal fields and FlowFields only
                // (corpus recordref-fieldref/fieldref/TestFieldRefRelationFieldClass.al:106-145).
                if self.is_flow_filter(t.object, c.field) {
                    continue;
                }
                let ok = self.relation_cond_holds(t, &field.name, c, &r.fields[c.field])?;
                if !ok {
                    holds = false;
                    break;
                }
            }
            if holds {
                for a in &arm.assumes {
                    al_builtins::assumptions::assume(*a);
                }
                return Ok(Some(arm.clone()));
            }
        }
        Ok(None)
    }

    /// Whether the condition `c` of a TableRelation arm holds for `value`, the value of its field in a row of `t`
    /// (`field_name` is the field that has the relation, for the messages).
    pub(crate) fn relation_cond_holds(
        &self,
        t: &TableDef,
        field_name: &str,
        c: &ir::RelationCond,
        value: &Value,
    ) -> Result<bool, Signal> {
        if !c.filter {
            return const_cond_holds(value, &c.value, || format!("{}.{field_name}", t.name));
        }
        // `Field = filter(x)`: the arm applies when the field's value passes the filter, which reads
        // like a SetFilter text (`filter('')` the blank value, `filter(<> '')` any other; corpus types/
        // lt-filter-enum-relation-duration-guid/C_TableRelation.al: Zone_Blank_Arm_Loc and the `A|B`, `C` arms).
        let al_ir::Const::Text(text) = &c.value else {
            return unsupported("a filter condition of a TableRelation without its text");
        };
        // The AL string of `filter('A|B')` is the filter text `A|B`; the blank `filter('')` stays the blank value
        // (queries/joins-links/RelArms.al:P100_ZoneFilterArms).
        let unquoted;
        let text = match text.strip_prefix('\'').and_then(|r| r.strip_suffix('\'')) {
            Some(inner) if !inner.is_empty() && !inner.replace("''", "").contains('\'') => {
                unquoted = inner.replace("''", "'");
                &unquoted
            }
            _ => text,
        };
        let engine = self.engine.clone();
        let language = al_builtins::format::FormatCtx::language(self);
        let enums = |id: u32| crate::views::enum_filter_members(&engine.program.objects[id as usize], language);
        let expr = al_builtins::filter::parse_filter_with(text, &[], &t.fields[c.field].ty, &enums).map_err(|e| {
            Signal::unsupported(format!(
                "the filter condition `{text}` of the TableRelation of {}.{field_name}: {e}",
                t.name
            ))
        })?;
        // A value the filter cannot compare does not count as failing it: that would apply the next or the else arm (hard
        // rule 4; issue 6924).
        if let Some(e) = expr.compare_error(value) {
            return cond_refusal(e, &format!("{}.{field_name}", t.name));
        }
        expr.matches(value)
    }

    /// `FieldRef.Relation`: the id of the related table, 0 without a relation.
    pub(crate) fn field_relation(&self, rec: &Rec, t: &TableDef, pos: usize) -> Result<i32, Signal> {
        let id = t.fields[pos].id;
        // SystemId, SystemCreatedAt and SystemModifiedAt relate to nothing, SystemCreatedBy and SystemModifiedBy to the User table, on
        // an own and on a Base Application table (recordref-fieldref/fieldref/Y67SystemFieldRelation.al:71-126).
        if id >= 2_000_000_000 {
            return match id {
                SYSTEM_CREATED_BY | SYSTEM_MODIFIED_BY => Ok(USER_TABLE),
                2_000_000_000 | 2_000_000_001 | 2_000_000_003 => Ok(0),
                _ => unsupported(format!("the relation of system field {}", t.fields[pos].name)),
            };
        }
        match self.relation_arm(rec, t, pos)? {
            None => Ok(0),
            Some(arm) => match arm.table {
                Some(table) => Ok(self.engine.program.objects[table].id.unwrap_or(0) as i32),
                None => unsupported(format!(
                    "relation target `{}` is not declared in the workspace",
                    arm.table_name
                )),
            },
        }
    }

    /// Before `Validate` assigns `value`: the related table must hold it, unless the field says
    /// `ValidateTableRelation = false` or the value is blank. A refused value is never stored
    /// (corpus record/TestCalcFormulaSystemFieldsTests.Codeunit.al:253-259).
    pub(crate) fn validate_relation(
        &mut self,
        rec: &Rec,
        t: &TableDef,
        pos: usize,
        value: Value,
    ) -> Result<(), Signal> {
        // Typed field facts, worked out once (a scan of the field's properties on each Validate showed in the Tests-ERM profile).
        if !self.engine.field_facts[t.object as usize][pos].validate_relation {
            return Ok(());
        }
        // The check calculates a FlowField that an if condition of the relation names, without a CalcFields first
        // (record/PnF29RelationByFlowField.al:242-260, FlowRelation_Condition_NoCalcFields). Like the
        // where() case below, whether the calculated value stays on the record is not observed.
        let flow_conds = self.flow_field_conditions(t, pos);
        if !flow_conds.is_empty() {
            self.calc_fields(rec, &flow_conds)?;
        }
        let Some(arm) = self.relation_arm(rec, t, pos)? else {
            return Ok(());
        };
        if crate::recmeta::is_blank(&value)? {
            return Ok(());
        }
        let Some(table) = arm.table else {
            return unsupported(format!(
                "relation target `{}` is not declared in the workspace",
                arm.table_name
            ));
        };
        let Some(wheres) = &arm.wheres else {
            return unsupported(format!(
                "the where() clause of the relation to `{}` is not understood yet",
                arm.table_name
            ));
        };
        let related = self.engine.tables[table]
            .clone()
            .ok_or_else(|| Signal::Unsupported("relation to a table without a schema".into()))?;
        let rel_pos = arm.field.unwrap_or(related.keys[0][0]);
        let mut flow_field_filters: Vec<FieldFilter> = Vec::new();
        let mut filters = vec![FieldFilter {
            lang: al_builtins::filter::filter_language(),
            field: rel_pos,
            expr: FilterExpr::Eq(ops::convert(value.clone(), &related.fields[rel_pos].ty)?),
            group: 0,
        }];
        let mut used_source_flowfilter = false;
        for w in wheres {
            // A condition on a FlowFilter field of the related table is a flow filter: it selects no row. Not pinned;
            // Item Journal Line's "Bin Code" relation names the Bin's "Item Filter" and "Variant Filter"
            // (Tests-SCM Invt. Doc. Pst Preview TestInventoryShipmentPreviewWithBin; assumption
            // relation-where-on-a-flowfilter-of-the-target).
            let related_pos = match w {
                ir::RelationWhere::Value { related, .. } | ir::RelationWhere::Field { related, .. } => *related,
            };
            if self.is_flow_filter(related.object, related_pos) {
                // record/filters/RelationWhereFlowFilter.al:ValidateFlowFilterConstRelation, ValidateFlowFilterFieldRelation_ExistingValue.
                continue;
            }
            // A condition on a FlowField of the related table compares each row's calculated value, not the stored one: a lookup,
            // exist and count FlowField with const, filter and field() (al-runner issue 6979; corpus
            // record/filters/RelationWhereFlowFilter.al:182-292, RelationOnRelatedFlowField_*).
            let target = if self.is_flow_field(related.object, related_pos) {
                &mut flow_field_filters
            } else {
                &mut filters
            };
            if let ir::RelationWhere::Field { own, .. } = w
                && self.is_flow_filter(t.object, *own)
                && let Some(exact) = self.source_relation_flowfilter(rec, t, *own)?
            {
                used_source_flowfilter = true;
                if let Some(value) = exact {
                    target.push(FieldFilter {
                        lang: al_builtins::filter::filter_language(),
                        field: related_pos,
                        expr: FilterExpr::Eq(ops::convert(value, &related.fields[related_pos].ty)?),
                        group: 0,
                    });
                }
                continue;
            }
            target.push(match w {
                ir::RelationWhere::Value { related: p, text } => FieldFilter {
                    lang: al_builtins::filter::filter_language(),
                    field: *p,
                    expr: self
                        .parse_filter_text(text, &related.fields[*p].ty)
                        .map_err(|e| Signal::unsupported(format!("where() of a relation: {e}")))?,
                    group: 0,
                },
                ir::RelationWhere::Field { related: p, own } => FieldFilter {
                    lang: al_builtins::filter::filter_language(),
                    field: *p,
                    expr: FilterExpr::Eq({
                        // The check calculates a FlowField the filter names (staging types/PZMisc.al:
                        // Validate_TableRelationFilteredByAFlowField, BC 29.0). Whether the calculated value
                        // stays on the record is not observed (docs/probes-needed.md).
                        if self.is_flow_field(t.object, *own) {
                            self.calc_fields(rec, &[*own])?;
                        }
                        // A condition that names the validated field itself (`Item Variant.Code where("Item No." =
                        // field("Item No."), Code = field("Variant Code"))` on Prod. Order Line, Assembly Header and
                        // Assembly Line "Variant Code") reads the value being validated, not the old one; the old
                        // blank value would make the filter contradict the key. Pinned by record/table-relation/
                        // RelationWhereValidated.al (Validate_OldBlank_ExistingAndMissing, Validate_OldNonBlank_NewExisting).
                        let own_value = if *own == pos {
                            value.clone()
                        } else {
                            rec.borrow().fields[*own].clone()
                        };
                        ops::convert(own_value, &related.fields[*p].ty)?
                    }),
                    group: 0,
                },
            });
        }
        let cx = self.cx_for(rec, &related)?;
        let found = if flow_field_filters.is_empty() {
            self.store
                .count(&cx, &related, &filters)
                .map_err(|e| Signal::unsupported(format!("{e}")))?
        } else {
            self.count_passing_flow_fields(&cx, &related, &filters, &flow_field_filters)?
        };
        if found > 0 {
            return Ok(());
        }
        // A related table the baseline could not fill may hold the row in BC.
        self.unseeded_read(&related, false, "the TableRelation check")?;
        Err(Signal::Error(Box::new(AlError {
            message: format!(
                "The field {} of table {} contains a value ({}) that cannot be found in the related table ({}).",
                self.field_caption(t, pos),
                self.caption(t),
                self.message_value(&value)?,
                self.relation_error_caption(&related)
            ),
            code: if used_source_flowfilter {
                al_builtins::native::UNPINNED_ERROR.into()
            } else {
                self.relation_error_code(t, pos, &related, rel_pos, wheres)
            },
            call_stack: String::new(),
            ..Default::default()
        })))
    }

    /// A source FlowFilter with a default buffer contributes no condition when unset
    /// (record/filters/RelationWhereFlowFilter.al:398-412), or one equality for an exact
    /// filter in group 0 (RelationWhereFlowFilter.al:426-441 accepts a match, :444-459
    /// rejects a mismatch; BC 27, 28, 29). The outer None preserves the existing
    /// buffer-based comparison for a field assigned without a filter.
    fn source_relation_flowfilter(
        &self,
        rec: &Rec,
        table: &TableDef,
        field: usize,
    ) -> Result<Option<Option<Value>>, Signal> {
        let default = Value::default_for(&table.fields[field].ty);
        let source = rec.borrow();
        let default_buffer = ops::compare(&source.fields[field], &default)?.is_eq();
        let mut filters = source.filters.iter().filter(|filter| filter.field == field);
        let exact = match (filters.next(), filters.next()) {
            (None, _) if !default_buffer => return Ok(None),
            (None, _) if source.filter_group == 0 => None,
            (Some(filter), None) if default_buffer && filter.group == 0 && source.filter_group == 0 => {
                match &filter.expr {
                    FilterExpr::Eq(value) => Some(value.clone()),
                    FilterExpr::Range(Some(lower), Some(upper)) if ops::compare(lower, upper)?.is_eq() => {
                        Some(lower.clone())
                    }
                    _ => {
                        return unsupported(
                            "TableRelation validation requires one exact source FlowFilter value (not pinned)",
                        );
                    }
                }
            }
            _ => {
                return unsupported(
                    "TableRelation validation with a source FlowFilter outside the default group or with an assigned buffer value (not pinned)",
                );
            }
        };
        Ok(Some(exact))
    }

    /// The error code of a failed TableRelation check of field `pos` of `t` against `related` (the related field `rel_pos`, the
    /// where() conditions `wheres`). Where the corpus does not pin it, the code is `UNPINNED_ERROR`: `GetLastErrorCode` then
    /// refuses, while the text stays readable (al-runner issue 7438). The text is one template in all 12 shapes measured
    /// (RelationWhereKeyCoverage.al:232-427); that the shapes with an unpinned code use it too is not measured (docs/probes-needed.md).
    fn relation_error_code(
        &self,
        t: &TableDef,
        pos: usize,
        related: &TableDef,
        rel_pos: usize,
        wheres: &[ir::RelationWhere],
    ) -> String {
        // The related table "Field" (2000000041) answers DB:RecordNotFound (staging types/PZMisc.al:
        // Validate_TableRelation_MissingFieldNumber, BC 29.0).
        if self.engine.program.objects[related.object as usize].id == Some(2_000_000_041) {
            return "DB:RecordNotFound".into();
        }
        let pk = &related.keys[0];
        let conditional = self.relation_is_conditional(t, pos);
        let on_key = |w: &ir::RelationWhere| pk.contains(&where_field(w));
        let code = match wheres {
            // One condition on a field outside the primary key: DB:NothingInsideFilter, whether the value is missing or fails the
            // filter (record/table-relation/RelationWhereEmptyTerm.al:173-177, RelationWhereEmptyValueFilter.al:203-214;
            // queries/joins-links/RelQuotedEnum.al:P100_Validate_EachKind).
            [w] if !on_key(w) => Some("DB:NothingInsideFilter"),
            // Several conditions, one or more outside the primary key: DB:NothingInsideFilter, plain (RelationWhereMulti.al:85,
            // :187-262; RelationWhereKeyCoverage.al:256, :352, :420) and inside an if() arm, beside a key condition
            // (RelationWhereKeyCoverage.al:224) or not (RelationWhereQualifiedConst.al:171-189 ConditionalChain).
            _ if wheres.len() > 1 && wheres.iter().any(|w| !on_key(w)) => Some("DB:NothingInsideFilter"),
            // Conditions on primary key fields only that name the whole key with the related field: DB:RecordNotFound, plain and
            // inside an if() arm (PnF13RelationFilterConditions.al:122-126, :144-169; RelationWhereValidated.al
            // Validate_OldBlank_ExistingAndMissing; RelationWhereMulti.al:160-182, :267-284; RelationWhereKeyCoverage.al:272,
            // :288, :320, :384; RelationIfDatabaseConst.al:269).
            _ if !wheres.is_empty() && covers_primary_key(related, rel_pos, wheres) => Some("DB:RecordNotFound"),
            // One condition on a primary key field that leaves a key field unnamed, plain: DB:NothingInsideFilter
            // (RelationWhereKeyCoverage.al:336, :400).
            [_] if !conditional => Some("DB:NothingInsideFilter"),
            [_, ..] => None,
            // No where(): a related field that is the whole primary key answers DB:PrimRecordNotFound
            // (TableRelationArms.al; RelationIfDatabaseConst.al:209-215), one in no key DB:NothingInsideFilter
            // (RelationWhereMulti.al:289-336 RelationToNonKeyField_Validate), the last field of a longer primary key, plain,
            // DB:NothingInsideFilter (RelationWhereKeyCoverage.al:368).
            [] if pk.as_slice() == [rel_pos] => Some("DB:PrimRecordNotFound"),
            [] if related.keys.iter().all(|k| !k.contains(&rel_pos)) => Some("DB:NothingInsideFilter"),
            [] if !conditional && pk.len() > 1 && pk.last() == Some(&rel_pos) => Some("DB:NothingInsideFilter"),
            [] => None,
        };
        code.map_or_else(|| al_builtins::native::UNPINNED_ERROR.into(), Into::into)
    }
}

/// Whether a `Field = const(x)` condition holds for `value` (`relation` names the field with the relation, for the
/// refusal).
/// A comparison that cannot be answered is refused: counting it as "does not hold" would apply the next or the else arm
/// (hard rule 4).
fn const_cond_holds(value: &Value, c: &ir::Const, relation: impl Fn() -> String) -> Result<bool, Signal> {
    match ops::compare(value, &crate::constant(c)) {
        Ok(o) => Ok(o == Ordering::Equal),
        // An error of the comparison itself (a Text that reads as no Guid) is not pinned for a relation either.
        Err(e) => cond_refusal(e, &relation()),
    }
}

/// The refusal of a TableRelation condition of `relation` (table.field) whose comparison failed with `e`.
fn cond_refusal<T>(e: Signal, relation: &str) -> Result<T, Signal> {
    // A refusal keeps its key behind the prefix (ADR-071).
    let why: al_builtins::refusal::Refusal = match e {
        Signal::Unsupported(why) => why,
        Signal::Error(e) => e.message.into(),
        _ => "the comparison did not give an order".into(),
    };
    unsupported(
        why.map_reason(|why| format!("a condition of the TableRelation of {relation} cannot be evaluated: {why}")),
    )
}

/// Whether every condition of a relation's `where()` is on a primary key field of the related table and, with the related field
/// itself, they name the whole key.
fn where_field(w: &ir::RelationWhere) -> usize {
    match w {
        ir::RelationWhere::Value { related, .. } | ir::RelationWhere::Field { related, .. } => *related,
    }
}

fn covers_primary_key(related: &TableDef, rel_pos: usize, wheres: &[ir::RelationWhere]) -> bool {
    let pk = &related.keys[0];
    let on = |w: &ir::RelationWhere| match w {
        ir::RelationWhere::Value { related, .. } | ir::RelationWhere::Field { related, .. } => *related,
    };
    wheres.iter().all(|w| pk.contains(&on(w))) && pk.iter().all(|k| *k == rel_pos || wheres.iter().any(|w| on(w) == *k))
}

#[cfg(test)]
mod tests {
    use super::*;

    // A condition whose comparison cannot be answered must refuse the relation, not count as "does not hold" and let the
    // next or the else arm apply (hard rule 4; issue 6924, a Validate checked against the else arm's table).
    #[test]
    fn a_condition_that_cannot_be_compared_refuses() {
        let unknown = Value::Unknown("the value is not known".into());
        match const_cond_holds(&unknown, &ir::Const::Text("A".into()), || "T.F".into()) {
            Err(Signal::Unsupported(why)) => {
                assert!(why.contains("TableRelation of T.F"), "{why}");
                assert!(why.contains("the value is not known"), "{why}");
            }
            other => panic!("expected a refusal, got {other:?}"),
        }
    }

    #[test]
    fn a_condition_that_compares_answers() {
        let c = ir::Const::Enum(7, 1);
        assert!(const_cond_holds(&Value::Enum(7, 1), &c, String::new).unwrap());
        assert!(!const_cond_holds(&Value::Enum(7, 2), &c, String::new).unwrap());
    }
}
