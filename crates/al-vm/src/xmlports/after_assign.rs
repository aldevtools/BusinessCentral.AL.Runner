//! Import of a field node into a field with a TableRelation, in the layouts corpus PR 1366 measures
//! (xmlports/import/XpAfterAssignTableRelation.al:1437-1872, al-runner issue 7863). With OnAfterAssignField the trigger runs
//! first and the field is validated with the value it left, under the assumption `xmlport-after-assign-before-validate`.
//! Without triggers, a temporary table element validates nothing, under `xmlport-temporary-import-not-validated`. Every
//! other shape keeps `xp_validate_field`.

use super::*;
use al_builtins::assumptions::{Assumption, assume};

/// What follows OnAfterAssignField in a measured shape.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum AfterAssign {
    /// The field is validated with the value the trigger left: the TableRelation check (where ValidateTableRelation is
    /// not false), then OnValidate.
    Validate,
    /// Nothing is validated.
    NoValidation,
}

/// The measured fields of table 77701 (XpAfterAssignTableRelation.al:61-120), each related to the primary key of a table
/// whose key is one field: Code[20] (:45-59) or Integer (:122-135).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum MeasuredField {
    /// "Rel Code" (:69-77): Code[20], TableRelation, OnValidate.
    Code20,
    /// "Rel Code NV" (:78-87): Code[20], TableRelation with ValidateTableRelation = false, OnValidate.
    Code20Unchecked,
    /// "Rel Code NoTrig" (:88-92): Code[20], TableRelation, no OnValidate (the reporter's field).
    Code20NoOnValidate,
    /// "Rel Int" (:93-101): Integer, TableRelation to an Integer key, OnValidate.
    Integer,
    /// "Rel Code10" (:102-110): Code[10], TableRelation to a Code[20] key, OnValidate.
    Code10,
}

impl Session {
    /// The measured shape of `node` with OnAfterAssignField, or `None` for every other shape. Each fact is required by
    /// equality: the layout al-sema `xmlports/assumed_shapes.rs` `after_assign_layout` marks, a field of `MeasuredField`,
    /// the table element's temporary state matching the record's, and the combinations in `measured_after_assign`.
    pub(super) fn xp_after_assign_shape(
        &mut self,
        schema: &Schema,
        node: usize,
        rec: &Rec,
        pos: usize,
    ) -> Result<Option<AfterAssign>, Signal> {
        let n = schema.node(node);
        if !n.after_assign_layout {
            return Ok(None);
        }
        let Some(stored) = stored(schema, node, rec) else {
            return Ok(None);
        };
        let Some(field) = self.xp_measured_field(rec, pos)? else {
            return Ok(None);
        };
        Ok(measured_after_assign(stored, n.kind, n.field_validate, field))
    }

    /// A field node without triggers in a measured layout on a temporary table element: nothing is validated. True means
    /// the caller stores the value and validates nothing; the assumption is marked here.
    pub(super) fn xp_temporary_import_not_validated(
        &mut self,
        schema: &Schema,
        node: usize,
        rec: &Rec,
        pos: usize,
    ) -> Result<bool, Signal> {
        if !self.xp_plain_temporary_shape(schema, node, rec, pos)? {
            return Ok(false);
        }
        assume(Assumption::XmlportTemporaryImportNotValidated);
        Ok(true)
    }

    /// The layout al-sema `plain_import_layout` marks, a temporary table element and record, the table's Integer key field
    /// on the first node (`xp_first_node_is_integer_key`), and a combination in `measured_plain_temporary`.
    fn xp_plain_temporary_shape(
        &mut self,
        schema: &Schema,
        node: usize,
        rec: &Rec,
        pos: usize,
    ) -> Result<bool, Signal> {
        let n = schema.node(node);
        if !n.plain_import_layout || stored(schema, node, rec) != Some(false) {
            return Ok(false);
        }
        if !self.xp_first_node_is_integer_key(schema, node, rec)? {
            return Ok(false);
        }
        let Some(field) = self.xp_measured_field(rec, pos)? else {
            return Ok(false);
        };
        Ok(measured_plain_temporary(n.kind, n.field_validate, field))
    }

    /// The first node of the table element of `node` has the field that `Id` is in the four measured ports
    /// (XpAfterAssignTableRelation.al:592, :710, :781, :951): the one field of the table's primary key (:115), an Integer
    /// without TableRelation and without OnValidate (:68), with no table extension validate trigger and no subscriber to
    /// its validate events.
    fn xp_first_node_is_integer_key(&mut self, schema: &Schema, node: usize, rec: &Rec) -> Result<bool, Signal> {
        let Some(table_node) = schema.node(node).table_node else {
            return Ok(false);
        };
        let first = schema.children(table_node).into_iter().next().map(|c| schema.node(c));
        let Some(pos) = first.filter(|c| c.table_node == Some(table_node)).and_then(|c| c.field) else {
            return Ok(false);
        };
        let t = self.table(rec)?;
        let engine = self.engine.clone();
        let Some(table) = engine.program.objects[t.object as usize].table.as_ref() else {
            return Ok(false);
        };
        let Some(field) = table.fields.get(pos) else {
            return Ok(false);
        };
        let plain_integer_key = matches!(table.keys.first().map(|k| &k[..]), Some([k]) if *k == pos)
            && field.ty == RtType::Integer
            && !field.relation
            && field.relation_arms.is_empty()
            && field.relation_runtime.is_empty()
            && field.on_validate.is_none()
            && field.ext_before_validate.is_empty()
            && field.ext_after_validate.is_empty();
        Ok(plain_integer_key
            && !self.listened(&t, "onbeforevalidateevent", Some(&field.name))?
            && !self.listened(&t, "onaftervalidateevent", Some(&field.name))?)
    }

    /// The field at `pos`, when it is one of `MeasuredField`: one TableRelation arm to the primary key of a table without
    /// conditions, where() or a named field, that key one field of the type the measured related table has, no table
    /// extension validate trigger and no subscriber to the validate events.
    fn xp_measured_field(&mut self, rec: &Rec, pos: usize) -> Result<Option<MeasuredField>, Signal> {
        let t = self.table(rec)?;
        let engine = self.engine.clone();
        let objects = &engine.program.objects;
        let Some(field) = objects[t.object as usize]
            .table
            .as_ref()
            .and_then(|s| s.fields.get(pos))
        else {
            return Ok(None);
        };
        let [arm] = &field.relation_arms[..] else {
            return Ok(None);
        };
        let plain_arm = arm.conditions.is_empty()
            && arm.field.is_none()
            && arm.wheres.as_ref().is_some_and(|w| w.is_empty())
            && arm.assumes.is_empty();
        let key_ty = arm.table.and_then(|o| objects[o].table.as_ref()).and_then(|related| {
            match related.keys.first().map(|k| &k[..]) {
                Some([k]) => related.fields.get(*k).map(|f| f.ty.clone()),
                _ => None,
            }
        });
        if !(field.relation
            && plain_arm
            && field.relation_runtime.is_empty()
            && field.ext_before_validate.is_empty()
            && field.ext_after_validate.is_empty())
        {
            return Ok(None);
        }
        let checked = engine.field_facts[t.object as usize][pos].validate_relation;
        let on_validate = field.on_validate.is_some();
        let code20 = Some(RtType::Code(Some(20)));
        let shape = match (&field.ty, key_ty, checked, on_validate) {
            (RtType::Code(Some(20)), k, true, true) if k == code20 => MeasuredField::Code20,
            (RtType::Code(Some(20)), k, false, true) if k == code20 => MeasuredField::Code20Unchecked,
            (RtType::Code(Some(20)), k, true, false) if k == code20 => MeasuredField::Code20NoOnValidate,
            (RtType::Code(Some(10)), k, true, true) if k == code20 => MeasuredField::Code10,
            (RtType::Integer, Some(RtType::Integer), true, true) => MeasuredField::Integer,
            _ => return Ok(None),
        };
        if self.listened(&t, "onbeforevalidateevent", Some(&field.name))?
            || self.listened(&t, "onaftervalidateevent", Some(&field.name))?
        {
            return Ok(None);
        }
        Ok(Some(shape))
    }

    /// OnAfterAssignField, then what `shape` says, for the value already in the field. A blank field attribute runs neither
    /// (XpAfterAssignTableRelation.al:1437, :1489, :1822 `blank`), a blank field element runs both (:1450, :1501, :1786).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn xp_after_assign_then_validate(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        node: usize,
        rec: &Rec,
        pos: usize,
        shape: AfterAssign,
        blank: bool,
    ) -> Result<(usize, Option<Flow>), Signal> {
        assume(Assumption::XmlportAfterAssignBeforeValidate);
        if schema.node(node).kind == ir::XmlNodeKind::FieldAttr && blank {
            return Ok((pos, None));
        }
        let flow = self.xp_site(inst, schema, node, "onafterassignfield")?;
        if shape == AfterAssign::Validate {
            if flow.is_some() {
                return unsupported(format!(
                    "XmlPort import: Skip or Break in OnAfterAssignField of {} before the field is validated (not pinned)",
                    schema.node(node).name
                ));
            }
            let value = rec.borrow().fields[pos].clone();
            self.platform_validate(crate::records::ValidateSite::XmlPortField, rec, pos, value)?;
        }
        Ok((pos, flow))
    }

    /// Two rows in one document are not measured (every case of XpAfterAssignTableRelation.al imports one row, :1410-1424,
    /// :1564-1578): the second row of a table element with a field node in a measured shape refuses before its fields are
    /// assigned. Except where the measured rule gives main's answer for any second row, so the row imports with main's
    /// answer: a field attribute with FieldValidate = No whose value in `elem` is not blank (the trigger, then no validation;
    /// :1463, :1514), and a field element with FieldValidate = No (the trigger, also on a blank element, then no validation;
    /// :1786, :1799).
    pub(super) fn xp_after_assign_second_row(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        table: usize,
        elem: &Elem,
    ) -> Result<(), Signal> {
        let rec = self.xp_record(inst, schema.node(table))?;
        for child in schema.children(table) {
            let c = schema.node(child);
            let Some(pos) = c.field else {
                continue;
            };
            if c.table_node != Some(table) {
                continue;
            }
            if self.xp_plain_temporary_shape(schema, child, &rec, pos)? {
                return unsupported(format!(
                    "XmlPort import of a second row of temporary table element {} with a validated field (not pinned)",
                    schema.node(table).name
                ));
            }
            if self.xp_after_assign_shape(schema, child, &rec, pos)?.is_none() {
                continue;
            }
            let mains_answer = c.field_validate == Some(false)
                && match c.kind {
                    ir::XmlNodeKind::Field => true,
                    _ => elem
                        .attrs
                        .iter()
                        .any(|(name, value)| *name == c.xml_name && !value.trim().is_empty()),
                };
            if !mains_answer {
                return unsupported(format!(
                    "XmlPort import of a second row of {} with OnAfterAssignField on a validated field (not pinned)",
                    schema.node(table).name
                ));
            }
        }
        Ok(())
    }
}

/// Whether the table element of field node `node` stores its rows: `Some(true)` for a table element without UseTemporary
/// and a record that is not temporary, `Some(false)` for UseTemporary and a temporary record, `None` otherwise.
fn stored(schema: &Schema, node: usize, rec: &Rec) -> Option<bool> {
    let table_node = schema.node(schema.node(node).table_node?);
    match (table_node.use_temporary, rec.borrow().temporary) {
        (true, true) => Some(false),
        (false, false) => Some(true),
        _ => None,
    }
}

/// The measured combinations with OnAfterAssignField: (stored, node kind, FieldValidate, field).
fn measured_after_assign(
    stored: bool,
    kind: ir::XmlNodeKind,
    field_validate: Option<bool>,
    field: MeasuredField,
) -> Option<AfterAssign> {
    use AfterAssign::{NoValidation as None_, Validate};
    use MeasuredField::*;
    use ir::XmlNodeKind::{Field as Elem, FieldAttr as Attr};
    Some(match (stored, kind, field_validate, field) {
        // A temporary table element (UseTemporary): nothing is validated (:1437 attribute, :1450 element, :1463 FieldValidate =
        // No, :1476 ValidateTableRelation = false, :1642 and :1654 no OnValidate, :1740 FieldValidate = Yes, :1786 element
        // with FieldValidate = No, :1822 Integer).
        (false, Attr, None, Code20 | Code20Unchecked | Code20NoOnValidate | Integer)
        | (false, Attr, Some(_), Code20)
        | (false, Elem, None | Some(false), Code20) => None_,
        // A stored table element: the TableRelation check and OnValidate after the trigger (:1489 attribute, :1501 element,
        // :1677 no OnValidate, :1753 FieldValidate = Yes, :1836 Integer, :1861 Code[10]); OnValidate without the check
        // (:1527 ValidateTableRelation = false); nothing with FieldValidate = No (:1514 attribute, :1799 element).
        (true, Attr, None, Code20 | Code20Unchecked | Code20NoOnValidate | Integer | Code10)
        | (true, Attr, Some(true), Code20)
        | (true, Elem, None, Code20) => Validate,
        (true, Attr | Elem, Some(false), Code20) => None_,
        _ => return None,
    })
}

/// The measured combinations without triggers on a temporary table element, where nothing is validated: (node kind,
/// FieldValidate, field). XpAfterAssignTableRelation.al:1700 attribute, :1720 element, :1766 attribute with FieldValidate =
/// Yes, :1667 attribute into the field without OnValidate.
fn measured_plain_temporary(kind: ir::XmlNodeKind, field_validate: Option<bool>, field: MeasuredField) -> bool {
    use MeasuredField::*;
    use ir::XmlNodeKind::{Field as Elem, FieldAttr as Attr};
    matches!(
        (kind, field_validate, field),
        (Attr, None | Some(true), Code20) | (Elem, None, Code20) | (Attr, None, Code20NoOnValidate)
    )
}
