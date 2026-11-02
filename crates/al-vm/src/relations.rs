//! `TableRelation`: which table a field relates to (`FieldRef.Relation`) and whether a value
//! exists there (`Validate`).
//!
//! Pinned by corpus fieldref/TestFieldRefRelation.al and TestFieldRefRelationFieldClass.al: the
//! first arm whose conditions hold applies, an else arm has none, a condition on a FlowFilter
//! field is not evaluated, `where(A = field(B))` reads B off the referencing row, and a value the
//! related table lacks is refused with "... cannot be found in the related table".

use crate::Session;
use al_builtins::filter::{FieldFilter, FilterExpr};
use al_builtins::native::{AlError, Signal};
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

fn unsupported<T>(what: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(what.into()))
}

impl Session {
    fn is_flow_field(&self, table: u32, pos: usize) -> bool {
        self.engine.field_facts[table as usize][pos].class == ir::FieldClass::FlowField
    }

    fn is_flow_filter(&self, table: u32, pos: usize) -> bool {
        self.engine.field_facts[table as usize][pos].class == ir::FieldClass::FlowFilter
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

    /// The arm of field `pos`'s relation that applies to the buffer, if any.
    fn relation_arm(&self, rec: &Rec, t: &TableDef, pos: usize) -> Result<Option<ir::RelationArm>, Signal> {
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
                // (corpus fieldref/TestFieldRefRelationFieldClass.al:106-145).
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
            return Ok(ops::compare(value, &crate::constant(&c.value)).ok() == Some(Ordering::Equal));
        }
        // `Field = filter(x)`: the arm applies when the field's value passes the filter, which reads
        // like a SetFilter text (`filter('')` the blank value, `filter(<> '')` any other; corpus types/
        // lt-filter-enum-relation-duration-guid/C_TableRelation.al: Zone_Blank_Arm_Loc and the `A|B`, `C` arms).
        let al_ir::Const::Text(text) = &c.value else {
            return unsupported("a filter condition of a TableRelation without its text");
        };
        // The AL string of `filter('A|B')` is the filter text `A|B`; the blank `filter('')` stays the blank value
        // (query/lt-query-links-relations-httpstream/RelArms.al:P100_ZoneFilterArms).
        let unquoted;
        let text = match text.strip_prefix('\'').and_then(|r| r.strip_suffix('\'')) {
            Some(inner) if !inner.is_empty() && !inner.replace("''", "").contains('\'') => {
                unquoted = inner.replace("''", "'");
                &unquoted
            }
            _ => text,
        };
        let engine = self.engine.clone();
        let enums = |id: u32| crate::views::enum_filter_members(&engine.program.objects[id as usize]);
        let expr = al_builtins::filter::parse_filter_with(text, &[], &t.fields[c.field].ty, &enums).map_err(|e| {
            Signal::Unsupported(format!(
                "the filter condition `{text}` of the TableRelation of {}.{field_name}: {e}",
                t.name
            ))
        })?;
        Ok(expr.matches(value))
    }

    /// `FieldRef.Relation`: the id of the related table, 0 without a relation.
    pub(crate) fn field_relation(&self, rec: &Rec, t: &TableDef, pos: usize) -> Result<i32, Signal> {
        let id = t.fields[pos].id;
        // SystemId, SystemCreatedAt and SystemModifiedAt relate to nothing, SystemCreatedBy and SystemModifiedBy to the User table, on
        // an own and on a Base Application table (fieldref/lt-y67-system-field-relation/Y67SystemFieldRelation.al:71-126).
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
        // (misc/lt-probes-needed-6/PnF29RelationByFlowField.al:242-260, FlowRelation_Condition_NoCalcFields). Like the
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
        let mut filters = vec![FieldFilter {
            field: rel_pos,
            expr: FilterExpr::Eq(ops::convert(value.clone(), &related.fields[rel_pos].ty)?),
            group: 0,
        }];
        for w in wheres {
            // A condition on a FlowFilter field of the related table is a flow filter: it selects no row. Not pinned;
            // Item Journal Line's "Bin Code" relation names the Bin's "Item Filter" and "Variant Filter"
            // (Tests-SCM Invt. Doc. Pst Preview TestInventoryShipmentPreviewWithBin; assumption
            // relation-where-on-a-flowfilter-of-the-target).
            let related_pos = match w {
                ir::RelationWhere::Value { related, .. } | ir::RelationWhere::Field { related, .. } => *related,
            };
            if self.is_flow_filter(related.object, related_pos) {
                // record/lt-relation-where-flowfilter/RelationWhereFlowFilter.al:ValidateFlowFilterConstRelation, ValidateFlowFilterFieldRelation_ExistingValue.
                continue;
            }
            filters.push(match w {
                ir::RelationWhere::Value { related: p, text } => FieldFilter {
                    field: *p,
                    expr: self
                        .parse_filter_text(text, &related.fields[*p].ty)
                        .map_err(|e| Signal::Unsupported(format!("where() of a relation: {e}")))?,
                    group: 0,
                },
                ir::RelationWhere::Field { related: p, own } => FieldFilter {
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
                        // blank value would make the filter contradict the key. Pinned by record/lt-relation-where-validated/
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
        let found = self
            .store
            .count(&cx, &related, &filters)
            .map_err(|e| Signal::Unsupported(format!("{e}")))?;
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
            // The related table "Field" (2000000041) answers DB:RecordNotFound (staging types/PZMisc.al:
            // Validate_TableRelation_MissingFieldNumber, BC 29.0); other tables DB:PrimRecordNotFound.
            // A relation with a where() filter answers DB:NothingInsideFilter, whether the value is missing or fails the filter
            // (query/lt-query-links-relations-httpstream/RelQuotedEnum.al:P100_Validate_EachKind).
            code: if self.engine.program.objects[related.object as usize].id == Some(2_000_000_041) {
                "DB:RecordNotFound".into()
            } else if let [ir::RelationWhere::Value { related: w, .. } | ir::RelationWhere::Field { related: w, .. }] =
                wheres.as_slice()
            {
                // A single where() on a field of the related table's primary key answers DB:RecordNotFound (corpus
                // misc/lt-probes-needed-6/PnF13RelationFilterConditions.al:122-126, 144-149, 166-169, primary key (Code, Zone)); on
                // another field DB:NothingInsideFilter (record/lt-relation-where-empty-term/RelationWhereEmptyTerm.al:173-177). Several conditions are refused.
                if related.keys[0].contains(w) {
                    "DB:RecordNotFound".into()
                } else {
                    "DB:NothingInsideFilter".into()
                }
            } else if wheres.len() > 1 && covers_primary_key(&related, rel_pos, wheres) {
                // Conditions on primary key fields only, which with the related field name the whole key, answer RecordNotFound:
                // `Item Variant.Code where("Item No." = field("Item No."), Code = field("Variant Code"))`
                // (record/lt-relation-where-validated/RelationWhereValidated.al:Validate_OldBlank_ExistingAndMissing,
                // Validate_OldNonBlank_NewMissing, Validate_OtherItemNo_SameValue). Any other set of several conditions is refused below.
                "DB:RecordNotFound".into()
            } else if wheres.len() > 1 && !wheres.iter().any(|w| related.keys[0].contains(&where_field(w))) {
                // Several conditions, none on a primary key field: the code of the one-condition form on a non-key field
                // (record/lt-relation-where-empty-term/RelationWhereEmptyTerm.al:173-177), on an assumption.
                al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::TableRelationSeveralWhereNonKey);
                "DB:NothingInsideFilter".into()
            } else if !wheres.is_empty() {
                // Several conditions: which code BC answers is not pinned for the other mixes, so the error path is refused.
                return Err(Signal::Unsupported(
                    "the error code of a TableRelation check with several where() conditions (not pinned)".into(),
                ));
            } else {
                "DB:PrimRecordNotFound".into()
            },
            call_stack: String::new(),
            ..Default::default()
        })))
    }
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
