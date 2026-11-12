//! Import of a field node with OnAfterAssignField into a field that has validation code (al-runner issue 7863): the shapes
//! corpus PR 1366 measures (xmlports/import/XpAfterAssignTableRelation.al:578-676) run the trigger first and validate the value it
//! left, under the assumption `xmlport-after-assign-before-validate`. Every other shape keeps `xp_validate_field`.

use super::*;
use al_builtins::assumptions::{assume, Assumption};

/// What follows OnAfterAssignField in a measured shape.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum AfterAssign {
    /// The field is validated with the value the trigger left: the TableRelation check (where ValidateTableRelation is
    /// not false), then OnValidate.
    Validate,
    /// Nothing is validated.
    NoValidation,
}

impl Session {
    /// The measured shape of `node`, or `None` for every other shape. Each fact is required by equality:
    /// - Format = Xml, DefaultFieldsValidation not declared false (every port of XpAfterAssignTableRelation.al, :88);
    /// - a field attribute or field element with an OnAfterAssignField trigger and no FieldValidate = Yes;
    /// - its table element right below the root text element, without LinkTable (:96-98);
    /// - a Code field with one TableRelation to a table without conditions, where() or a named field (:53-70), no table
    ///   extension validate trigger and no subscriber to the validate events;
    /// - the combinations of temporary, node kind, FieldValidate, ValidateTableRelation and OnValidate in `measured`.
    pub(super) fn xp_after_assign_shape(
        &mut self,
        schema: &Schema,
        node: usize,
        rec: &Rec,
        pos: usize,
    ) -> Result<Option<AfterAssign>, Signal> {
        let n = schema.node(node);
        let xp = schema.xp();
        let Some(table_node) = n.table_node.map(|t| schema.node(t)) else {
            return Ok(None);
        };
        let below_root = table_node.parent.is_some_and(|p| {
            let p = schema.node(p);
            p.parent.is_none() && p.kind == ir::XmlNodeKind::Text
        });
        if !(xp.format == ir::XmlFormat::Xml
            && !xp.no_default_fields_validation
            && matches!(n.kind, ir::XmlNodeKind::Field | ir::XmlNodeKind::FieldAttr)
            && n.trigger("onafterassignfield").is_some()
            && below_root
            && table_node.link_node.is_none())
        {
            return Ok(None);
        }
        let t = self.table(rec)?;
        let engine = self.engine.clone();
        let Some(field) = engine.program.objects[t.object as usize]
            .table
            .as_ref()
            .and_then(|s| s.fields.get(pos))
        else {
            return Ok(None);
        };
        let plain_relation = matches!(&field.relation_arms[..], [arm]
            if arm.conditions.is_empty()
                && arm.table.is_some()
                && arm.field.is_none()
                && arm.wheres.as_ref().is_some_and(|w| w.is_empty())
                && arm.assumes.is_empty());
        if !(matches!(t.fields[pos].ty, RtType::Code(_))
            && field.relation
            && plain_relation
            && field.relation_runtime.is_empty()
            && field.ext_before_validate.is_empty()
            && field.ext_after_validate.is_empty())
        {
            return Ok(None);
        }
        if self.listened(&t, "onbeforevalidateevent", Some(&field.name))?
            || self.listened(&t, "onaftervalidateevent", Some(&field.name))?
        {
            return Ok(None);
        }
        let temporary = rec.borrow().temporary;
        let stored = match (table_node.use_temporary, temporary) {
            (true, true) => false,
            (false, false) => true,
            _ => return Ok(None),
        };
        let checked = engine.field_facts[t.object as usize][pos].validate_relation;
        Ok(measured(
            stored,
            n.kind,
            n.field_validate,
            checked,
            field.on_validate.is_some(),
        ))
    }

    /// OnAfterAssignField, then what `shape` says, for the value already in the field. A blank field attribute runs neither
    /// (XpAfterAssignTableRelation.al:583, :635 `blank`), a blank field element runs both (:596, :647).
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

    /// Two rows in one document are not measured (every case of XpAfterAssignTableRelation.al imports one row, :557-559): the
    /// second row of a table element with a field node in a measured shape refuses before its fields are assigned.
    pub(super) fn xp_after_assign_second_row(&mut self, inst: &Inst, schema: &Schema, table: usize) -> Result<(), Signal> {
        let rec = self.xp_record(inst, schema.node(table))?;
        for child in schema.children(table) {
            let Some(pos) = schema.node(child).field else {
                continue;
            };
            if schema.node(child).table_node == Some(table)
                && self.xp_after_assign_shape(schema, child, &rec, pos)?.is_some()
            {
                return unsupported(format!(
                    "XmlPort import of a second row of {} with OnAfterAssignField on a validated field (not pinned)",
                    schema.node(table).name
                ));
            }
        }
        Ok(())
    }
}

/// The measured combinations: (stored, node kind, FieldValidate, ValidateTableRelation not false, OnValidate).
fn measured(
    stored: bool,
    kind: ir::XmlNodeKind,
    field_validate: Option<bool>,
    checked: bool,
    on_validate: bool,
) -> Option<AfterAssign> {
    use ir::XmlNodeKind::{Field as Elem, FieldAttr as Attr};
    use AfterAssign::{NoValidation as None_, Validate};
    Some(match (stored, kind, field_validate, checked, on_validate) {
        // A temporary table element (UseTemporary): nothing is validated (:578 attribute, :591 element, :604 FieldValidate =
        // No, :617 ValidateTableRelation = false).
        (false, Attr, None, true, true) | (false, Elem, None, true, true) => None_,
        (false, Attr, Some(false), true, true) | (false, Attr, None, false, true) => None_,
        // The reporter's shape (al-runner issue 7863): the same field without OnValidate, assumed from :578.
        (false, Attr, None, true, false) => None_,
        // A stored table element: the TableRelation check and OnValidate after the trigger (:630 attribute, :642 element);
        // OnValidate without the check (:668 ValidateTableRelation = false); nothing with FieldValidate = No (:655).
        (true, Attr, None, true, true) | (true, Elem, None, true, true) | (true, Attr, None, false, true) => Validate,
        (true, Attr, Some(false), true, true) => None_,
        _ => return None,
    })
}
