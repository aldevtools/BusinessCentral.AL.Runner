//! A Rename whose new key value does not convert to a field of another table that relates to the renamed key (al-runner
//! issues 8988 and 9178).
//!
//! Measured by corpus PR 1610 (merged, not yet in the pinned corpus), record/table-relation/RenameShortRelatedField.al:248-429,
//! through the assumption `rename-short-related-field`: a related field with ObsoleteState = Removed is not checked (Text[4]
//! and Integer, Record.Rename and RecordRef.Rename); a related Text[4] field with ObsoleteState = Pending or without
//! ObsoleteState raises the reference-field conversion error, with or without child rows. Only those shapes run
//! (`Session::short_related_shape`), and only with the old key's measured conversion (old key 'AB', :197: it converts to
//! the Text[4] field and not to the Integer one); any other Rename whose old or new key does not convert to a related
//! field is refused. Old and new keys that both convert keep main's rename.
use crate::Session;
use crate::globaltriggers::GlobalOp;
use crate::rename::RenameRef;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::native::{Signal, raise, unsupported};
use al_builtins::value::{RtType, Value};
use al_data::{AccessContext, Store, TableDef};

/// The measured outcome of a related field the new key does not convert to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    /// ObsoleteState = Removed (RenameShortRelatedField.al:248-345): Rename does not check the field.
    Skip,
    /// ObsoleteState = Pending or none (RenameShortRelatedField.al:346-429): the reference-field conversion error.
    Error,
}

/// The facts of the Rename call that the measured shapes read.
pub(crate) struct RenameCall<'a> {
    /// The record variable the Rename ran on.
    pub rec: &'a crate::records::Rec,
    /// The renamed row's old field values.
    pub old: &'a [Value],
    /// The renamed row's new field values.
    pub new: &'a [Value],
    /// The old key value converts to the related field.
    pub old_converts: bool,
    /// The new key value converts to the related field.
    pub new_converts: bool,
    /// AL code called the statement form of Rename.
    pub statement_from_al: bool,
}

impl Session {
    /// Rename of `t`: the old or the new key value did not convert to the related field `r` of `child`. Returns `Ok(())` when Rename does not check the field, the measured
    /// error, or a refusal for every unmeasured shape.
    pub(crate) fn rename_unconvertible_reference(
        &self,
        cx: &AccessContext,
        t: &TableDef,
        child: &TableDef,
        r: &RenameRef,
        call: &RenameCall,
    ) -> Result<(), Signal> {
        let (_, pos, _) = r.parts();
        match self.short_related_shape(cx, t, child, r, call) {
            Some(Shape::Skip) => {
                assume(Assumption::RenameShortRelatedField);
                Ok(())
            }
            Some(Shape::Error) => {
                assume(Assumption::RenameShortRelatedField);
                // The record keeps its old key after the error, as on the pinned blank-key Rename error
                // (`blank_key_rename_raise`, rename_blank_key.rs); a direct cell is issue 9189.
                call.rec.borrow_mut().fields = call.old.to_vec().into();
                // RenameShortRelatedField.al:346-429 (the text and GetLastErrorCode of each Pending and live cell).
                raise(
                    format!(
                        "The record could not be renamed. A type conversion error occurred while updating reference field \
                         {}.{}. The record has data type Code and the referencing field has data type Text.",
                        child.name, child.fields[pos].name
                    ),
                    "InvalidType:StringLengthExceeded",
                )
            }
            None => unsupported(format!(
                "Rename of {}: the old or new key value does not convert to the related field {}.{} in a shape corpus PR \
                 1610 does not measure",
                t.name, child.name, child.fields[pos].name
            )),
        }
    }

    /// The measured shape, read fact by fact from RenameShortRelatedField.al (corpus PR 1610). Both shapes: the renamed
    /// table's only key is one Code[20] field (:22-26); the relation is one `TableRelation = <table>` arm (:33-34; the
    /// outside report of issue 8988 names the key field, `<table>.Code`) without where() or if, of a Normal field
    /// without Enabled or TestTableRelation of a per-company table without a table extension. The error also reads the
    /// facts of the probe tables: both per company, in one app, without a table extension, a table trigger, a subscriber
    /// or a database rename trigger, the related table's one Integer key its first field and the related field its
    /// second; the related field is Text[4] (Removed :33-38, Pending
    /// :73-78, live :94) or Integer (Removed :53-58); the old key has 2 characters and the new one 10 (:197); the child
    /// table holds no row or one (:174-181); AL code called the statement form of Record.Rename or RecordRef.Rename
    /// (:126-150).
    fn short_related_shape(
        &self,
        cx: &AccessContext,
        t: &TableDef,
        child: &TableDef,
        r: &RenameRef,
        call: &RenameCall,
    ) -> Option<Shape> {
        let engine = &self.engine;
        let (child_index, pos, arm) = r.parts();
        let master = t.object as usize;
        let schema = engine.program.objects[master].table.as_ref()?;
        let child_schema = engine.program.objects[child_index].table.as_ref()?;
        let [key] = t.keys.as_slice() else { return None };
        let [k] = key.as_slice() else { return None };
        let [child_key] = child.keys.as_slice() else {
            return None;
        };
        let [ck] = child_key.as_slice() else { return None };
        let no_trigger = |d: &TableDef| {
            ["OnInsert", "OnModify", "OnDelete", "OnRename"]
                .iter()
                .all(|n| self.table_trigger(d, n).is_none())
        };
        let obsolete = self
            .field_property(child, pos, "ObsoleteState")
            .map(|v| v.trim().to_ascii_lowercase());
        // The shape, and whether the measured old key 'AB' (:197) converts to the related field: to Text[4] it does, to
        // Integer it does not.
        let (shape, old_converts) = match (&child.fields[pos].ty, obsolete.as_deref()) {
            (RtType::Text(Some(4)), Some("removed")) => (Shape::Skip, true),
            (RtType::Integer, Some("removed")) => (Shape::Skip, false),
            (RtType::Text(Some(4)), Some("pending")) | (RtType::Text(Some(4)), None) => (Shape::Error, true),
            _ => return None,
        };
        let text_len = |v: &Value| match v {
            Value::Code(s) => Some(Value::text_len(s)),
            _ => None,
        };
        // Read for both shapes: the relation, the related field and the call.
        let field_facts = [
            child.per_company && child_schema.extensions.is_empty(),
            self.declaring_extension(child, pos).is_none(),
            child_index != master,
            t.fields[*k].ty == RtType::Code(Some(20)),
            r.sole_arm() && arm.assumes.is_empty() && arm.field.is_none_or(|f| f == *k),
            arm.wheres.as_deref().unwrap_or_default().is_empty() && arm.conditions.is_empty(),
            engine.field_facts[child_index][pos].class == al_ir::FieldClass::Normal,
            ["Enabled", "TestTableRelation"]
                .iter()
                .all(|p| self.field_property(child, pos, p).is_none()),
            text_len(&call.new[*k]) == Some(10),
            call.old_converts == old_converts,
            !call.new_converts,
            call.statement_from_al,
        ];
        // Read for the error only. The skip does not read them: the outside report (issue 8988) measured it on
        // Salesperson/Purchaser, a table with triggers, subscribers and many relations, with `TableRelation = <table>.Code`.
        let table_facts = [
            t.per_company && schema.extensions.is_empty(),
            engine.program.objects[child_index].app == engine.program.objects[master].app,
            no_trigger(t) && no_trigger(child),
            !engine.events.any_subscriber_of(master) && !engine.events.any_subscriber_of(child_index),
            self.global_known_off(t, GlobalOp::Rename),
            child.fields[*ck].ty == RtType::Integer && *ck == 0 && pos == 1,
            arm.field.is_none(),
            matches!(self.store.count(cx, child, &[]), Ok(0) | Ok(1)),
        ];
        let measured = field_facts.iter().all(|f| *f) && (shape == Shape::Skip || table_facts.iter().all(|f| *f));
        measured.then_some(shape)
    }
}
