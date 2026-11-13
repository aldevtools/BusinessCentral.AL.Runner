//! A Rename of a database record that changes a primary key field to or from its blank or zero value while a relation that
//! counts targets that key field (al-runner issue 7937).
//!
//! Business Central raises an error for it after the OnRename trigger and the database rename event, and OnAfterRenameEvent
//! does not run. Nothing in the pinned corpus measures it; the evidence is corpus pull request 1370,
//! record/table-relation/RenameBlankKeyRelation.al, measured on BC 27.5, 28.5 and 29.0 (assumption
//! rename-blank-key-with-relation). The `:line` citations in this file are lines of that file.
//!
//! The error is raised only in the cells that file measures: the base cell (`:1239` from blank, `:1279` to blank) and each
//! cell that differs from the base cell in one fact (`Session::measured_blank_rename`). Any other such rename is refused.
//! These keep main's rename and main's errors: a temporary record, a rename in which no key field that a counting relation
//! targets changes to or from blank, a row that is not stored and a new key that is stored.

use crate::Session;
use crate::globaltriggers::GlobalOp;
use crate::records::{BR, Rec};
use crate::rename::RenameRef;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::native::{AlError, Signal, unsupported};
use al_builtins::ops;
use al_builtins::value::{RowOrigin, RtType, Value};
use al_data::{AccessContext, CodeStorage, Store, TableDef};
use al_ir::{FieldClass, RelationArm, RelationWhere};
use std::cmp::Ordering;

/// The error a Rename raises once its triggers ran (`Session::blank_key_rename_raise`).
pub(crate) struct BlankKeyRename {
    error: AlError,
    /// The database's change counter before the triggers.
    counter: u64,
    /// The new primary key, as the Rename set it.
    key: Vec<Value>,
}

/// One fact of a rename against the base cell: `Ok(false)` the base cell's value, `Ok(true)` a value another cell measured,
/// `Err` a value no cell measured (what it is).
type Fact = Result<bool, &'static str>;

impl Session {
    /// Before the triggers of a Rename of `rec` (row `old` to `new`): the error to raise after them, when a relation that
    /// counts (`relation_counts`) targets a primary key field of `t` (`RenameRef::related_field`) that changes to or from
    /// its blank value, the old row is stored and the new key is not. Such a rename outside the measured cells is refused.
    /// `None`: the rename goes on as on main. `refs` are the relations to `t` (`rename_references`); `from_al`: AL code
    /// called Rename on the record (`TxState::rename_from_al`).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn blank_key_rename(
        &self,
        cx: &AccessContext,
        rec: &Rec,
        t: &TableDef,
        refs: &[RenameRef],
        old: &[Value],
        new: &[Value],
        from_al: bool,
    ) -> Result<Option<BlankKeyRename>, Signal> {
        // A temporary record renames while a database row relates to the blank key (:1572, :1583).
        if rec.borrow().temporary {
            return Ok(None);
        }
        let changes: Vec<usize> = t.keys[0]
            .iter()
            .copied()
            .filter(|&k| changes_to_or_from_blank(t, k, old, new))
            .collect();
        let targeted = refs
            .iter()
            .any(|r| changes.contains(&r.related_field(t)) && self.relation_counts(r));
        if !targeted {
            return Ok(None);
        }
        // A row that is not stored (:2229, :2242) and a new key that is stored (:2178, :2191) get their own errors first.
        let store_err = |e: al_data::StoreError| Signal::unsupported(format!("{e}"));
        let Some(stored) = self.store.get(cx, t, &t.primary_key(old)).map_err(store_err)? else {
            return Ok(None);
        };
        if self.store.get(cx, t, &t.primary_key(new)).map_err(store_err)?.is_some() {
            return Ok(None);
        }
        let texts = |k: usize| -> Result<[String; 2], Signal> {
            let text = |v: &Value| al_builtins::format::try_format(v, 0, 0, self).map_err(Signal::unsupported);
            Ok([text(&old[k])?, text(&new[k])?])
        };
        let measured = match changes.as_slice() {
            &[k] => texts(k).map(|texts| (k, texts)).and_then(|(k, texts)| {
                Ok(self
                    .measured_blank_rename(rec, t, refs, k, &stored, old, new, &texts, from_al)
                    .map(|()| (k, texts)))
            })?,
            _ => Err("more than one key field changes to or from blank"),
        };
        match measured {
            // Captions name the field and the table where both have one (:2034, :2047); two backslash characters and no CR
            // or LF separate the sentences, and the code is DB:Ok (:1239).
            Ok((k, [from, to])) => Ok(Some(BlankKeyRename {
                error: AlError {
                    message: format!(
                        "The {} field on the {} table cannot be renamed from \"{from}\" to \"{to}\".{BR}{BR}When a field has a \
                         relation defined, the field cannot be renamed to and from zero or an empty value.",
                        self.field_caption(t, k),
                        self.caption(t),
                    ),
                    code: "DB:Ok".into(),
                    call_stack: String::new(),
                    ..Default::default()
                },
                counter: self.store.change_counter(),
                key: t.primary_key(new),
            })),
            Err(what) => unsupported(format!(
                "Rename of {}: a primary key field another table relates to changes to or from a blank or zero key value, in a \
                 shape that is not measured ({what}; docs/probes-needed.md, rename-blank-key-with-relation)",
                t.name
            )),
        }
    }

    /// After the OnRename trigger and the database rename event of a Rename (:1979, :1993, :2078, :2092): raises the error
    /// `blank_key_rename` found and gives the record its old row back. In the measured cells the triggers and subscribers
    /// wrote nothing to the database and left the record's key; anything else is refused.
    pub(crate) fn blank_key_rename_raise(
        &mut self,
        t: &TableDef,
        rec: &Rec,
        pending: Option<BlankKeyRename>,
        old: &[Value],
    ) -> Result<(), Signal> {
        let Some(pending) = pending else {
            return Ok(());
        };
        let key_kept = ops::compare_keys(&t.primary_key(&rec.borrow().fields), &pending.key)?;
        if self.store.change_counter() != pending.counter || !key_kept {
            return unsupported(format!(
                "Rename of {}: a primary key field another table relates to changes to or from a blank or zero key value, and \
                 a trigger or subscriber of the Rename wrote to the database or changed the key (not measured)",
                t.name
            ));
        }
        assume(Assumption::RenameBlankKeyWithRelation);
        rec.borrow_mut().fields = old.to_vec();
        Err(Signal::Error(Box::new(pending.error)))
    }

    /// Whether the relation `r` makes Business Central raise the error. These four kinds do not, each measured as the only
    /// relation to the table: a FlowFilter field (:2422, :2431), a FlowField (:2456, :2466), ValidateTableRelation = false
    /// (:1859, :1869) and ObsoleteState = Removed (:2747, :2757).
    fn relation_counts(&self, r: &RenameRef) -> bool {
        let (child, pos, _) = r.parts();
        let facts = self.engine.field_facts[child][pos];
        let removed = self.engine.tables[child]
            .as_ref()
            .is_some_and(|c| self.field_removed(c, pos));
        let no_error = facts.class == FieldClass::FlowFilter
            || facts.class == FieldClass::FlowField
            || !facts.validate_relation
            || removed;
        !no_error
    }

    /// Whether the Rename is the base cell or differs from it in one measured fact. The base cell (:1239, :1279): AL code
    /// calls the statement form of Rename on a Record variable that read the committed row by Get and has no filter; the
    /// table is per company, without a table extension, a Caption, an OnRename trigger or a subscriber, its database rename
    /// trigger known to be off, its primary key one plain Code[20] field; one relation in the program targets the table: a
    /// plain `TableRelation = <table>` of a validated Normal Code[20] field outside the primary key of another per-company
    /// table without extensions, triggers or subscribers. Each `Ok(true)` below is one other cell.
    #[allow(clippy::too_many_arguments)]
    fn measured_blank_rename(
        &self,
        rec: &Rec,
        t: &TableDef,
        refs: &[RenameRef],
        k: usize,
        stored: &[Value],
        old: &[Value],
        new: &[Value],
        texts: &[String; 2],
        from_al: bool,
    ) -> Result<(), &'static str> {
        let engine = &self.engine;
        let object = t.object as usize;
        let master = &engine.program.objects[object];
        let [r] = refs else {
            return Err("more than one relation targets the table");
        };
        let (child_index, pos, arm) = r.parts();
        let (Some(schema), Some(child), Some(child_schema)) = (
            &master.table,
            engine.tables[child_index].as_ref(),
            &engine.program.objects[child_index].table,
        ) else {
            return Err("a table without a schema");
        };
        if !r.sole_arm() || !arm.assumes.is_empty() {
            return Err("a relation with more than one arm");
        }
        let from_blank = is_blank(t, k, &old[k]);
        let (value, text) = if from_blank {
            (&new[k], &texts[1])
        } else {
            (&old[k], &texts[0])
        };
        let key_field = &schema.fields[k];
        let key_prop = |name: &str| self.field_property(t, k, name);
        let table_prop = |name: &str| master.properties.iter().any(|(p, _)| p.eq_ignore_ascii_case(name));
        let child_prop = |name: &str| {
            self.field_property(child, pos, name)
                .map(|v| v.trim().to_ascii_lowercase())
        };
        let child_trigger = |name: &str| self.table_trigger(child, name).is_some();
        let [key, arm_form] = key_and_arm(t, child, k, arm, old, new, from_blank);
        let facts: [Fact; 16] = [
            // The call: the return-value form (:2281, :2295) and RecordRef.Rename (:2321, :2334) raise the same error.
            match (from_al, self.ret_used, self.through_recordref) {
                (true, false, false) => Ok(false),
                (true, true, false) | (true, false, true) => Ok(true),
                _ => Err("a Rename that AL code did not call, or RecordRef.Rename with its return value used"),
            },
            self.buffer_fact(rec, t, stored, old, from_blank),
            if rec.borrow().filters.is_empty() {
                Ok(false)
            } else {
                Err("a record with a filter")
            },
            if t.per_company && schema.extensions.is_empty() {
                Ok(false)
            } else {
                Err("a renamed table that is not per company or has a table extension")
            },
            key,
            arm_form,
            value_fact(value, text),
            if !key_field.auto_increment
                && t.fields[k].code_storage == CodeStorage::Text
                && key_field.caption_class.is_none()
                && key_field.caption_class_refusal.is_none()
                && ["OptionCaption", "OptionCaptionML", "ObsoleteState", "CaptionML"]
                    .iter()
                    .all(|p| key_prop(p).is_none())
            {
                Ok(false)
            } else {
                Err(
                    "a key field with AutoIncrement, SQLDataType, CaptionClass, CaptionML, OptionCaption or ObsoleteState",
                )
            },
            // The key field with NotBlank (:2819, :2832) or with its own TableRelation (:2845, :2858).
            match (key_prop("NotBlank"), key_field.relation) {
                (None, false) => Ok(false),
                (Some(v), false) if v.trim().eq_ignore_ascii_case("true") => Ok(true),
                (None, true) => Ok(true),
                _ => Err("a key field with NotBlank and its own TableRelation"),
            },
            // A Caption on the table and on the key field (:2034, :2047).
            match (
                table_prop("Caption"),
                key_prop("Caption").is_some(),
                table_prop("CaptionML"),
            ) {
                (false, false, false) if self.caption_refused(object).is_none() => Ok(false),
                (true, true, false) if self.caption_refused(object).is_none() => Ok(true),
                _ => Err("a Caption on only one of the table and the key field, or a CaptionML"),
            },
            // Code that runs around the Rename: an OnRename trigger with OnBeforeRenameEvent and OnAfterRenameEvent subscribers
            // (:1979, :1993), the database rename trigger switched on with an OnBeforeRenameEvent subscriber (:2078, :2092).
            Ok(!(self.table_trigger(t, "OnRename").is_none()
                && !engine.events.any_subscriber_of(object)
                && self.global_known_off(t, GlobalOp::Rename))),
            if child.keys[0].contains(&pos) {
                Err("a related field in the primary key of its table")
            } else {
                // A Code[10] field against the Code[20] key (:2647, :2660).
                match (&t.fields[k].ty, &child.fields[pos].ty) {
                    (key, field) if key == field => Ok(false),
                    (RtType::Code(Some(20)), RtType::Code(Some(10))) => Ok(true),
                    _ => Err("a related field of another type than the key field"),
                }
            },
            // ObsoleteState = Pending on the related field (:2767, :2780).
            match child_prop("ObsoleteState").as_deref() {
                None => Ok(false),
                Some("pending") => Ok(true),
                _ => Err("a related field with an ObsoleteState other than Pending"),
            },
            // TestTableRelation = false on the related field (:2793, :2806).
            match child_prop("TestTableRelation").as_deref() {
                None => Ok(false),
                Some("false") => Ok(true),
                _ => Err("a related field with TestTableRelation = true"),
            },
            // A field of the renamed table itself (:1937, :1949) or of a table extension (:1895, :1908).
            match (child_schema.extensions.as_slice(), self.declaring_extension(child, pos)) {
                _ if child_index == object => Ok(true),
                _ if !child.per_company => Err("a related table that is not per company"),
                ([], None) => Ok(false),
                ([ext], Some(declared)) if *ext == declared => Ok(true),
                _ => Err("a related table with a table extension that does not declare the related field"),
            },
            // An OnModify trigger on the related table does not run (:2871, from blank).
            match (
                [
                    child_trigger("OnInsert"),
                    child_trigger("OnDelete"),
                    child_trigger("OnRename"),
                ],
                child_trigger("OnModify"),
                child_index != object && engine.events.any_subscriber_of(child_index),
            ) {
                ([false, false, false], false, false) => Ok(false),
                ([false, false, false], true, false) if from_blank => Ok(true),
                _ => Err("a related table with a trigger other than OnModify, or with a subscriber"),
            },
        ];
        let mut variants = 0;
        for fact in facts {
            variants += usize::from(fact?);
        }
        match variants {
            0 | 1 => Ok(()),
            _ => Err("more than one fact differs from the measured base cell"),
        }
    }

    /// How the record got the row it renames. It holds the stored row as it is, every stored field: read by Get of a
    /// committed row (the base cell), or inserted by the variable itself (:2674, :3007, from blank). A stale record gets
    /// another error on Business Central (:2894), which is not modelled.
    fn buffer_fact(&self, rec: &Rec, t: &TableDef, stored: &[Value], old: &[Value], from_blank: bool) -> Fact {
        let same = |i: usize| ops::compare(&stored[i], &old[i]).ok() == Some(Ordering::Equal);
        if !t.fields.iter().enumerate().all(|(i, f)| !f.stored || same(i)) {
            return Err("a record that does not hold the stored row as it is");
        }
        let version = t
            .fields
            .iter()
            .position(|f| f.id == 2_000_000_005)
            .and_then(|i| match &old[i] {
                Value::BigInteger(n) => Some(*n),
                _ => None,
            });
        match rec.borrow().row_origin.filter(|o| Some(o.version) == version) {
            Some(o) if o == RowOrigin::new(false, o.version, o.commits) => Ok(false),
            Some(o) if o == RowOrigin::new(true, o.version, o.commits) && from_blank => Ok(true),
            _ => Err("a record that got its row otherwise than by Get of a committed row or by its own Insert"),
        }
    }
}

/// The key and the arm of the relation. One key field: Code[20] in the base cell; Integer (:1333, :1346), Guid (:1441,
/// :1456), Option (:1489, :1502) and Date (:1533, :1546) are measured with a plain relation of a field of the same type. The
/// arm then is plain, has one `const` condition (:1676, :1691, :1706, met or not), one `const` where() on another field
/// (:1738, :1751) or names the key field (:1780, :1793). Two key fields are one cell (:1400): both Code[10], the second
/// changes from blank while the first keeps its value, and the arm is `<table>.<second> where(<first> = field(<Code[10]
/// field>))`.
fn key_and_arm(
    t: &TableDef,
    child: &TableDef,
    k: usize,
    arm: &RelationArm,
    old: &[Value],
    new: &[Value],
    from_blank: bool,
) -> [Fact; 2] {
    let code10 = RtType::Code(Some(10));
    let wheres = arm.wheres.as_deref().unwrap_or_default();
    match t.keys[0].as_slice() {
        [_] => [
            match &t.fields[k].ty {
                RtType::Code(Some(20)) => Ok(false),
                RtType::Integer | RtType::Guid | RtType::Option(_) | RtType::Date => Ok(true),
                _ => Err("a key field of a type that is not measured"),
            },
            match (arm.conditions.as_slice(), wheres, arm.field) {
                ([], [], None) => Ok(false),
                ([c], [], None) if !c.filter => Ok(true),
                ([], [RelationWhere::Value { related, .. }], None) if *related != k => Ok(true),
                ([], [], Some(field)) if field == k => Ok(true),
                _ => Err("a relation arm of a form that is not measured"),
            },
        ],
        &[first, second] => {
            let cell = second == k
                && from_blank
                && t.fields[first].ty == code10
                && t.fields[second].ty == code10
                && ops::compare(&old[first], &new[first]).ok() == Some(Ordering::Equal)
                && arm.conditions.is_empty()
                && arm.field == Some(second)
                && matches!(wheres, [RelationWhere::Field { related, own }]
                    if *related == first && child.fields[*own].ty == code10);
            if cell {
                [Ok(true), Ok(false)]
            } else {
                [Err("a two-field key outside the one measured cell"), Ok(false)]
            }
        }
        _ => [Err("a key of more than two fields"), Ok(false)],
    }
}

/// The non-blank value of the changed key field, with its text in the error. Measured: a Code value as stored (:1239,
/// :2968, :2987), the Integer 5 (:1333), a Guid of digits (:1441), an Option member without an OptionCaption (:1489) and a
/// Date (:1533). An Integer of four or more digits or below zero and a Guid with letters are not measured: their text is
/// not known.
fn value_fact(value: &Value, text: &str) -> Fact {
    match value {
        Value::Code(_) | Value::Option(..) | Value::Date(_, false) => Ok(false),
        Value::Integer(n) if (1..=999).contains(n) => Ok(false),
        Value::Guid(_) if !text.contains(|c: char| c.is_ascii_alphabetic()) => Ok(false),
        _ => Err("a key value whose text in the error is not measured"),
    }
}

/// `v` is the blank or zero value of field `k` of `t`.
fn is_blank(t: &TableDef, k: usize, v: &Value) -> bool {
    ops::compare(v, &Value::default_for(&t.fields[k].ty)).ok() == Some(Ordering::Equal)
}

/// Primary key field `k` of `t` holds a different value in `new` than in `old`, and one of the two is its type's blank or zero.
fn changes_to_or_from_blank(t: &TableDef, k: usize, old: &[Value], new: &[Value]) -> bool {
    matches!(
        ops::compare(&old[k], &new[k]).ok(),
        Some(Ordering::Less | Ordering::Greater)
    ) && (is_blank(t, k, &old[k]) || is_blank(t, k, &new[k]))
}
