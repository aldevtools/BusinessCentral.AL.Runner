//! Record builtins that read table metadata or guard on session state:
//! `TestField`, `Truncate`, `FieldCaption`, `FieldName`, `FieldNo`,
//! `TableCaption`, `TableName`.

use crate::Session;
use crate::blob_transfer::BlobMove;
use al_builtins::describe::Describe;
use al_builtins::native::{AlError, Signal, unsupported};
use al_builtins::ops;
use al_builtins::record_op::RecordOp;
use al_builtins::value::{RecordState, RtType, Value};
use al_data::{Store, TableDef};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::rc::Rc;

type Rec = Rc<RefCell<RecordState>>;

/// [`error`] with the text `GetLastErrorText(true)` answers for it.
fn error_redacted(message: String, code: &str, redacted: String) -> Signal {
    Signal::Error(Box::new(AlError {
        message,
        code: code.into(),
        origin: al_builtins::native::ErrorOrigin::Redacted(redacted),
        ..Default::default()
    }))
}

fn error(message: String, code: &str) -> Signal {
    Signal::Error(Box::new(AlError {
        message,
        code: code.into(),
        call_stack: String::new(),
        ..Default::default()
    }))
}

/// The text of an AL string literal property value (`'it''s'` → `it's`), if it is one.
pub(crate) fn string_literal(v: &str) -> Option<String> {
    let v = v.trim();
    let inner = v.strip_prefix('\'')?.strip_suffix('\'')?;
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c == '\'' {
            // Only a doubled quote may appear inside the literal.
            if chars.next() != Some('\'') {
                return None;
            }
        }
        out.push(c);
    }
    Some(out)
}

/// Whether a `TableRelation` value names `table` (lower case) anywhere: as a quoted
/// identifier or as a bare word. Over-approximates (a field of that name also counts), which
/// only makes callers refuse more.
pub(crate) fn mentions_table(relation: &str, table: &str) -> bool {
    let lower = relation.to_lowercase();
    if lower.contains(&format!("\"{table}\"")) {
        return true;
    }
    lower
        .split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .any(|w| !w.is_empty() && w == table)
}

/// The text of the error a write raises on a field it may not change (`ModifyAll` on SystemId or an AutoIncrement field):
/// "You cannot change the value of the <field> field.", two `BR`, "Table: <table>".
pub(crate) fn cannot_change_value_text(field: &str, table: &str) -> String {
    let br = crate::records::BR;
    format!("You cannot change the value of the {field} field.{br}{br}Table: {table}")
}

/// Whether a value is its type's blank value (what `TestField(Field)` rejects).
pub(crate) fn is_blank(v: &Value) -> Result<bool, Signal> {
    Ok(match v {
        Value::Integer(0) | Value::BigInteger(0) | Value::Byte(0) | Value::Char(0) => true,
        Value::Decimal(d) => d.is_zero(),
        Value::Boolean(b) => !b,
        Value::Text(s) | Value::Code(s) => s.is_empty(),
        Value::DateFormula(s) => s.is_empty(),
        Value::Date(d, _) => *d == 0,
        Value::Time(t) | Value::DateTime(t) | Value::Duration(t) => *t == 0,
        Value::Guid(g) => *g == [0; 16],
        Value::Option(o, _) => *o == 0,
        Value::Enum(_, o) => *o == 0,
        // A RecordId field is blank while nothing was assigned; the id of an initialized record that holds no key
        // is a value (recordref-fieldref/P135RecordIdTestField.al:P135_TestField_BlankRecordId,
        // P135_TestField_AssignedRecordId; record/testfield/P128TestFieldRecordId.al:P128_TestField_RecordIdField_Blank).
        Value::Library(al_builtins::library_kind::LibraryKind::RecordId, cell) => {
            matches!(*cell.borrow(), al_builtins::value::LibraryValue::Empty)
        }
        Value::Integer(_) | Value::BigInteger(_) | Value::Byte(_) | Value::Char(_) => false,
        // A Blob with no content is blank, after CalcFields or not; one with content is a value, written through a stream and not
        // saved, fetched by CalcFields, or read by Get or Find without CalcFields (recordref-fieldref/PnE08RecordMisc.al:
        // TestField_Blob noDataNoCalc, noDataCalc, dataNoCalcPassed; record/testfield/TestFieldBlobDecimal.al:
        // TestField_Blob_StoredContent `after_get_without_calcfields`, `after_calcfields`, `after_clear_saved_and_calcfields`;
        // TestField_Blob_AutoCalcAndSetLoadFields).
        Value::Blob(b) => {
            let buf = b.borrow();
            if let Some(what) = &buf.render_pending {
                if al_builtins::stream::is_unloaded_write_mark(what) {
                    return Err(al_builtins::stream::render_pending_error(what));
                }
                return unsupported("TestField of a Blob that holds the output of a report run for its effects only");
            }
            buf.bytes.is_empty()
        }
        other => {
            return unsupported(format!(
                "TestField of a {} field: when it counts as blank, and the error text, are not pinned (docs/probes-needed.md)",
                other.describe()
            ));
        }
    })
}

fn arg_pos(args: &[Value], i: usize) -> Result<usize, Signal> {
    match args.get(i) {
        Some(Value::Integer(p)) => Ok(*p as usize),
        other => unsupported(format!("field argument {}", other.describe())),
    }
}

impl Session {
    /// `Format(Record)`, `Format(RecordRef)` and `Format(FieldRef)`. A record formats as the
    /// `Format` of its fields joined by a TAB: the normal fields in field-number order, then the
    /// FlowFields in field-number order with the value they hold, system fields left out
    /// (record/TestFormatRecordContracts.al:111-145). A FlowFilter field is left out: nothing
    /// pins it (docs/probes-needed.md).
    pub(crate) fn format_composite_value(&mut self, v: &Value) -> Option<Result<String, Signal>> {
        let (rec, only) = match v {
            Value::Record(r) => (r.clone(), None),
            Value::Library(_, l) => match &*l.borrow() {
                al_builtins::value::LibraryValue::RecordRef(r) => (r.clone(), None),
                al_builtins::value::LibraryValue::FieldRef(r, p)
                | al_builtins::value::LibraryValue::KeyFieldRef(r, p) => (r.clone(), Some(*p)),
                al_builtins::value::LibraryValue::UserNameFieldRef { .. } => {
                    return Some(Err(Signal::Unsupported(
                        "Format of a FieldRef to a user-name system FlowField (not pinned)".into(),
                    )));
                }
                _ => return None,
            },
            _ => return None,
        };
        Some(self.format_record_fields(&rec, only))
    }

    fn format_record_fields(&self, rec: &Rc<RefCell<RecordState>>, only: Option<usize>) -> Result<String, Signal> {
        const SYSTEM_FIELD_BASE: u32 = 2_000_000_000;
        let t = self.table(rec)?;
        let fields = rec.borrow().fields.clone();
        let one = |pos: usize| -> Result<String, Signal> {
            let value = &fields[pos];
            match (&t.fields[pos].ty, value) {
                (al_builtins::value::RtType::Option(_), Value::Option(o, _)) => {
                    let Some((_, members)) = self.members_of_field(&t, pos)? else {
                        return unsupported("Format of an Option field");
                    };
                    Ok(members
                        .iter()
                        .find(|(ord, _, _)| ord == o)
                        .map_or_else(|| o.to_string(), |(_, _, caption)| caption.clone()))
                }
                _ => al_builtins::format::try_format(value, 0, 0, self).map_err(Signal::unsupported),
            }
        };
        if let Some(pos) = only {
            return one(pos);
        }
        let mut order: Vec<usize> = (0..t.fields.len())
            .filter(|&i| t.fields[i].id < SYSTEM_FIELD_BASE)
            .collect();
        order.sort_by_key(|&i| t.fields[i].id);
        // The stored fields come first, then the FlowFilter fields and then the FlowFields, each in field number
        // order; a FlowFilter field shows the value the field holds (0 for an Integer) whatever filter was set on it
        // (types/ProbeTypeFormsAndFilters.al:FormatRecord_WithFlowFilter_BeforeAndAfterSetRange and
        // types/W7FormatRecord.al:FormatRecord_FlowFilterField, BC 27.5 and 29.0).
        // All FlowFilter fields come before the FlowFields, whatever their numbers
        // (recordref-fieldref/PnE08RecordMisc.al:FormatRecord_FlowFilterPlacement `afterCalc`: filter 2, FlowField 3,
        // filter 4 show as `0 0 1`).
        let (mut normal, calculated): (Vec<usize>, Vec<usize>) = order.into_iter().partition(|&i| t.fields[i].stored);
        // The primary key fields come first, in key order, then the other stored fields in field number order: a key on the
        // last field gives `7<TAB>Last<TAB>3` (format-evaluate/record-format/RecordAsSubstitutionArg.al:246
        // RecordArg_KeyOnLastField_EmptyAndFilled `filled format`).
        if let Some(key) = t.keys.first() {
            let rest: Vec<usize> = normal.iter().copied().filter(|i| !key.contains(i)).collect();
            normal = key.iter().copied().filter(|i| normal.contains(i)).chain(rest).collect();
        }
        let schema = self.engine.program.objects[t.object as usize].table.as_ref();
        let (filters, flow_fields): (Vec<usize>, Vec<usize>) = calculated
            .into_iter()
            .partition(|&i| schema.and_then(|s| s.fields.get(i)).is_some_and(|f| f.flow_filter));
        let mut parts = Vec::new();
        for i in normal.into_iter().chain(filters).chain(flow_fields) {
            parts.push(one(i)?);
        }
        Ok(parts.join("\t"))
    }

    /// `FieldCaption` / `FieldRef.Caption`: refused for a system field whose caption the corpus does not pin. `rec` is the
    /// record whose caption is read; without one (a query column) a CaptionClass expression is refused.
    pub(crate) fn field_caption_checked(
        &mut self,
        t: &TableDef,
        pos: usize,
        rec: Option<&Rc<RefCell<RecordState>>>,
    ) -> Result<String, Signal> {
        let written = self.field_caption_written(t, pos)?;
        // With an OnAfterCaptionClassResolve subscriber an area 3 CaptionClass is translated as well, which raises the event
        // (Y71CaptionClassEvents.al:168-170; FieldCaption: record/captions/WcExtFieldAfterResolve.al
        // WcExtField_Area3_FieldCaption). `3,` alone raises nothing and gives the Caption (WcExtField_ThreeAlone_FieldCaption).
        if matches!(written, WrittenCaption::Text(_)) && self.after_resolve_subscribed() {
            if let Some(class) = self
                .field_property(t, pos, "CaptionClass")
                .and_then(|v| string_literal(&v))
            {
                if class.len() > 2 && class.starts_with("3,") {
                    // FieldCaption raises no CaptionClassTranslate of UI Helper Triggers (events/platform-triggers/
                    // PtObserve.al:135-150 `Captions`, `fieldcap` log=[]).
                    return self.caption_class_translate_silent(&class);
                }
            }
        }
        match written {
            WrittenCaption::Text(caption) => Ok(caption),
            WrittenCaption::Class(class) => {
                if !class.contains(',') && !self.after_resolve_subscribed() {
                    return unsupported(format!(
                        "FieldCaption of a field whose CaptionClass `{class}` has no comma (not pinned)"
                    ));
                }
                self.caption_class_translate(&class)
            }
            // A CaptionClass that is an expression runs once per read, with the record that reads it (record/lt-qd-captionclass-
            // areas/QdCaptionClassAreas.al:323-347 Expression_PerRead_CallCount, :350-372 Expression_ChangesRec), and its
            // result goes through the same resolvers as a literal (:262-296 Expressions_FieldCaption, :376-418
            // Expressions_FieldRefCaption).
            WrittenCaption::Expr(expr) => {
                let class = self.field_caption_class_value(t, pos, rec, &expr)?;
                match expression_class(&class) {
                    ExprCaption::FieldCaption => Ok(self.field_caption(t, pos)),
                    ExprCaption::Text(c) => self.expression_text_caption(&class, c),
                    ExprCaption::Translate => self.caption_class_translate(&class),
                    ExprCaption::Plain => self.expression_plain_caption(&t.fields[pos].name, &class),
                }
            }
        }
    }

    /// The caption of field `pos` that its declaration gives, a literal CaptionClass the platform translates, or the
    /// CaptionClass expression.
    pub(crate) fn field_caption_written(&self, t: &TableDef, pos: usize) -> Result<WrittenCaption, Signal> {
        if let Some(why) = self.field_caption_class_refusal(t, pos) {
            return unsupported(why);
        }
        // A translation file of the table's app gives the caption in another language (ADR-053 step 2).
        match self.field_caption_translation(t, pos) {
            Err(why) => return unsupported(why),
            Ok(Some(text)) => return Ok(WrittenCaption::Text(text)),
            Ok(None) => {}
        }
        let has_caption = self.field_property(t, pos, "Caption").is_some();
        if t.fields[pos].id >= 2_000_000_000 && !has_caption {
            return unsupported(format!("the caption of system field {}", t.fields[pos].name));
        }
        // A field with a CaptionClass: `3,<text>` reads as <text> ('3,' alone as the Caption); a numeric area other than 3
        // or an empty area goes to the platform's translation, which raises OnResolveCaptionClass and keeps what it translated
        // for the session (record/captions/QdCaptionClassAreas.al:233-258 Literals_FieldCaption: '2,0,x' reads
        // `x Excl. VAT`, '5,3,' `Bill-to County` through Base Application's subscribers; pages/triggers/
        // P84Captions.al: a text no subscriber resolves stays as written, and a second read raises no event). A non-numeric
        // area goes the same way: OnResolveCaptionClass with the area, the rest and the session language, the caption a
        // subscriber resolves, else the whole text (also with a Caption property), kept for the session and shared with
        // CaptionClassTranslate (corpus branch al-runtime-rs-staging-r6985, record/captions/R6985FieldCaptionArea.al:
        // FieldCaption_ClaimedArea, _SecondRead, _UnclaimedArea, FieldRefCaption_ClaimedArea, FieldCaption_ThenCaptionClassTranslate,
        // FieldCaption_InOnValidate; on master testpage/AnTestPageTests.al:583-589 An19_TableFieldCaptionClass_NonNumericArea,
        // 'X,Y' unclaimed). A text with no comma is pinned only with an OnAfterCaptionClassResolve subscriber: refused otherwise.
        let declared = self.field_property(t, pos, "CaptionClass");
        if let Some(v) = declared.as_ref().filter(|v| string_literal(v).is_none()) {
            return Ok(WrittenCaption::Expr(v.clone()));
        }
        if let Some(class) = declared.and_then(|v| string_literal(&v)) {
            return match class.split_once(',') {
                Some(("3", "")) => Ok(WrittenCaption::Text(self.field_caption(t, pos))),
                Some(("3", rest)) => Ok(WrittenCaption::Text(rest.to_string())),
                Some(_) => Ok(WrittenCaption::Class(class)),
                // With an OnAfterCaptionClassResolve subscriber the read raises the event (`FieldCaption`'s Class arm;
                // WcExtField_NoComma_FieldCaption); without one the shape stays refused.
                None => Ok(WrittenCaption::Class(class)),
            };
        }
        Ok(WrittenCaption::Text(self.field_caption(t, pos)))
    }

    /// Why a read of field `pos`'s caption is refused (see `ir::Field::caption_class_refusal`).
    pub(crate) fn field_caption_class_refusal(&self, t: &TableDef, pos: usize) -> Option<String> {
        self.engine.program.objects[t.object as usize]
            .table
            .as_ref()
            .and_then(|s| s.fields.get(pos))
            .and_then(|f| f.caption_class_refusal.clone())
    }

    /// The text field `pos`'s CaptionClass expression `expr` gives, run with `rec` as Rec (see `field_caption_checked`).
    pub(crate) fn field_caption_class_value(
        &mut self,
        t: &TableDef,
        pos: usize,
        rec: Option<&Rc<RefCell<RecordState>>>,
        expr: &str,
    ) -> Result<String, Signal> {
        let routine = self.engine.program.objects[t.object as usize]
            .table
            .as_ref()
            .and_then(|s| s.fields.get(pos))
            .and_then(|f| f.caption_class);
        let (Some((object, routine)), Some(rec)) = (routine, rec) else {
            return unsupported(format!(
                "the caption of field {} whose CaptionClass is the expression `{}`, read without a record",
                t.fields[pos].name,
                expr.trim()
            ));
        };
        let class = self.call_on_record(object, routine, rec.clone(), None, Vec::new())?.0;
        Self::refuse_rendered_caption_class(&class)?;
        match class {
            Value::Text(c) | Value::Code(c) => Ok(c.into_string()),
            other => unsupported(format!(
                "the CaptionClass expression of field {} gave a {}",
                t.fields[pos].name,
                other.describe()
            )),
        }
    }

    /// A field's caption: its `Caption` property, else the ENU entry of its `CaptionML` (record/crud/RcBInit.al:
    /// Props_BlankZero_Option_Caption `field caption`), else its name.
    pub(crate) fn field_caption(&self, t: &TableDef, pos: usize) -> String {
        self.engine.program.objects[t.object as usize]
            .table
            .as_ref()
            .and_then(|s| s.fields.get(pos))
            .and_then(|f| {
                let prop = |n: &str| {
                    f.properties
                        .iter()
                        .find(|(k, _)| k.eq_ignore_ascii_case(n))
                        .map(|(_, v)| v.as_str())
                };
                match (prop("Caption"), prop("CaptionML")) {
                    (Some(c), _) => string_literal(c),
                    (None, Some(ml)) => crate::platform::objects::declared_caption(None, Some(ml), "").ok(),
                    (None, None) => None,
                }
            })
            .unwrap_or_else(|| t.fields[pos].name.clone())
    }

    /// Formats a value for an error message; kinds `Format` cannot render yet are refused.
    pub(crate) fn message_value(&self, v: &Value) -> Result<String, Signal> {
        match v {
            Value::Null
            | Value::Boolean(_)
            | Value::Integer(_)
            | Value::BigInteger(_)
            | Value::Byte(_)
            | Value::Char(_)
            | Value::Decimal(_)
            | Value::Text(_)
            | Value::Code(_)
            | Value::Option(..)
            | Value::Enum(..)
            | Value::Guid(_)
            | Value::Date(..)
            // A DateTime reads as Format gives it, to the minute (record/testfield/FieldRefTestFieldTypedTemporalCode.al:
            // P189_FldRef_DateTime_Value, `03/15/24 12:00 PM` for 12:00:00 and 12:00:01).
            | Value::DateTime(_)
            // A Time reads as Format gives it: a 12-hour clock with the hour padded by a space, no milliseconds, the undefined
            // time as nothing (record/testfield/P96Temporal.al:111-128, recordref-fieldref/fieldref/
            // FieldRefTestFieldTime.al:36 ` 1:00:00 PM`).
            | Value::Time(_)
            // A Duration reads as Format gives it: `41 days 16 hours`, `1 second`, the zero Duration as nothing (`''` in a filter
            // text; recordref-fieldref/fieldref/TestFieldRefFilter.al FieldRef_SetFilter_Zero_Duration, corpus PR 1248).
            | Value::Duration(_)
            | Value::DateFormula(_) => al_builtins::format::try_format(v, 0, 0, self).map_err(Signal::unsupported),
            // A RecordId reads as Format gives it (`Table: key`, record/TestProbeRecordRefContracts.al:472-502); an error
            // message shows it the same way (record/filters/RecordIdInMessage.al:P168_Error_RecordIdAsArgument_PlainKey and siblings).
            Value::Library(al_builtins::library_kind::LibraryKind::RecordId, _) => {
                al_builtins::format::try_format(v, 0, 0, self).map_err(Signal::unsupported)
            }
            other => unsupported(format!("a {} value in an error message", other.describe())),
        }
    }

    /// `Code=ABC` style identification of the row's primary key, as in TestField errors
    /// (corpus record/TestRecordTestFieldLookupPageId_Tests.al:54-72 pins the single-field form).
    /// The platform's insert trigger of Document Service (2000000114): with an empty Folder it raises the TestField error of
    /// the Folder, code TestField (al-language-onprem/metadata/PtOnPremTables.al:103-123, `Folder must have a value in
    /// Document Service: Service ID=LTPT. It cannot be zero or empty.`). What it does with a Folder is not pinned: refused.
    pub(crate) fn document_service_insert(&self, t: &TableDef, rec: &crate::records::Rec) -> Result<(), Signal> {
        if self.engine.program.objects[t.object as usize].id != Some(2_000_000_114) || rec.borrow().temporary {
            return Ok(());
        }
        let Some(pos) = t.fields.iter().position(|f| f.id == 13) else {
            return unsupported("Document Service has no Folder field");
        };
        let row = rec.borrow().fields.clone();
        if !is_blank(&row[pos])? {
            return unsupported(
                "Insert(true) of a Document Service with a Folder: what the platform's trigger does is not pinned",
            );
        }
        let (caption, table, key) = (self.field_caption(t, pos), self.caption(t), self.key_text(t, &row)?);
        Err(error_redacted(
            format!("{caption} must have a value in {table}: {key}. It cannot be zero or empty."),
            "TestField",
            format!("{caption} must have a value in {table}: <redacted>. It cannot be zero or empty."),
        ))
    }

    fn key_text(&self, t: &TableDef, fields: &[Value]) -> Result<String, Signal> {
        let mut parts = Vec::new();
        for &i in &t.keys[0] {
            parts.push(format!(
                "{}={}",
                self.field_caption(t, i),
                self.message_value(&fields[i])?
            ));
        }
        Ok(parts.join(", "))
    }

    /// Where the record's marks hold the primary key `pk`. A mark whose key does not compare with it refuses (al-runner
    /// issue 7644).
    fn mark_index(rec: &Rec, pk: &[Value]) -> Result<Option<usize>, Signal> {
        for (i, m) in rec.borrow().marks.iter().enumerate() {
            if ops::compare_keys(m, pk)? {
                return Ok(Some(i));
            }
        }
        Ok(None)
    }

    /// `ReadIsolation` and `SecurityFiltering`, read (no argument) or set (an option member).
    fn isolation_or_security_filtering(
        &mut self,
        method: RecordOp,
        rec: &Rec,
        args: &[Value],
    ) -> Result<Value, Signal> {
        let (enum_name, is_iso) = if method == RecordOp::ReadIsolation {
            ("IsolationLevel", true)
        } else {
            ("SecurityFilter", false)
        };
        let members = al_builtins::sysenum::option_members(enum_name).expect("known");
        match args.first() {
            None => {
                let r = rec.borrow();
                let i = if is_iso {
                    r.read_isolation
                } else {
                    r.security_filtering_read()?
                };
                Ok(Value::Option(i, members.into()))
            }
            Some(Value::Option(i, _)) => {
                let mut r = rec.borrow_mut();
                if is_iso {
                    let before = r.read_isolation;
                    r.read_isolation = *i;
                    // The ASSIGNMENT of UpdLock or RepeatableRead on a database table opens the write transaction, the
                    // read under it does not (codeunits/TestCodeunitRunWriteTransaction.al:254-278 `updlock_set`,
                    // :340-355; corpus PR 896 format-evaluate/Np4Misc.al Y9_UpdLock_SetAfterCommit_FirstRead,
                    // Y9_RepeatableRead_SetAfterCommit_FirstRead `before=Yes`). Assigning the level the record already
                    // has opens none (Y9_ReadIsolation_ValueAcrossCommit `after setting again UpdLock|tx=No`); another
                    // level does, also on a fresh record (Y9_ReadIsolation_SetAgainAfterCommit `different level`,
                    // `same level other record`); ReadCommitted does not (`read committed`). RepeatableRead assigned
                    // again is pinned only before a Commit; either on a temporary table is not pinned.
                    match (*i, r.temporary) {
                        (ISOLATION_UPDLOCK | ISOLATION_REPEATABLE_READ, false) if before == *i => {
                            if *i == ISOLATION_REPEATABLE_READ && !self.tx.lock_tx && self.tx.lock_unpinned.is_none() {
                                self.tx.lock_unpinned = Some(crate::records::REPEATABLE_READ_ASSIGNED_AGAIN);
                            }
                        }
                        (ISOLATION_UPDLOCK | ISOLATION_REPEATABLE_READ, false) => {
                            self.tx.lock_tx = true;
                            self.tx.isolation_lock = true;
                            let key = (r.table, r.company.clone());
                            self.tx.note_isolation_lock(key.0, key.1);
                        }
                        (ISOLATION_REPEATABLE_READ | ISOLATION_UPDLOCK, true) => {
                            self.tx.lock_unpinned = Some("a locking ReadIsolation on a temporary table")
                        }
                        _ => {}
                    }
                } else {
                    r.security_filtering = *i;
                    r.security_filtering_unpinned = false;
                }
                Ok(Value::Null)
            }
            Some(other) => unsupported(format!("{method}({})", other.describe())),
        }
    }

    pub(crate) fn record_meta_op(
        &mut self,
        method: RecordOp,
        rec: &Rec,
        t: &TableDef,
        args: &mut [Value],
    ) -> Result<Value, Signal> {
        self.connection_check(rec)?;
        self.crm_method_check(t, rec, method, args)?;
        match method {
            RecordOp::AddLink
            | RecordOp::DeleteLink
            | RecordOp::DeleteLinks
            | RecordOp::HasLinks
            | RecordOp::CopyLinks => self.record_links(method, rec, t, args),
            RecordOp::TableName => Ok(Value::Text(t.name.clone().into())),
            RecordOp::TableCaption => self.caption_checked(t).map(|c| Value::Text(c.into())),
            RecordOp::FieldName => Ok(Value::Text(t.fields[arg_pos(args, 0)?].name.clone().into())),
            RecordOp::FieldCaption => self
                .field_caption_checked(t, arg_pos(args, 0)?, Some(rec))
                .map(|s| Value::Text(s.into())),
            RecordOp::FieldNo => Ok(Value::Integer(crate::refs::al_field_no(t.fields[arg_pos(args, 0)?].id))),
            // True for an enabled field (corpus record/TestRecordTriggers.al:223-231). What a field
            // with `Enabled = false` answers is not pinned: refused (docs/probes-needed.md).
            RecordOp::FieldActive => {
                let pos = arg_pos(args, 0)?;
                // False for `Enabled = false`, true for the rest, a Pending obsolete field included
                // (record/TestRowVersionFieldNumberFieldActiveContracts.al:110-119).
                Ok(Value::Boolean(self.field_enabled(t, pos)))
            }
            RecordOp::RecordId => Ok(self.record_id(rec, t)),
            RecordOp::IsTemporary => Ok(Value::Boolean(rec.borrow().temporary)),
            RecordOp::IsDirty => self.record_is_dirty(rec),
            // Without an argument, whether the current row is marked; with one, marks or unmarks it.
            // What the call returns then (the previous state?) is not pinned, so a used result is refused.
            RecordOp::Mark => {
                let pk = t.primary_key(&rec.borrow().fields);
                let at = Self::mark_index(rec, &pk)?;
                match args.first() {
                    None => Ok(Value::Boolean(at.is_some())),
                    // Mark(SetMark) returns the new state
                    // (record/TestRecordViewCopyMarkContracts.al:113, Mark_ReturnsTheNewState).
                    Some(v) => {
                        let want = ops::truthy(v)?;
                        let mut r = rec.borrow_mut();
                        match (want, at) {
                            (true, None) => r.marks.push(pk),
                            (false, Some(i)) => {
                                r.marks.remove(i);
                            }
                            _ => {}
                        }
                        Ok(Value::Boolean(want))
                    }
                }
            }
            RecordOp::MarkedOnly => match args.first() {
                None => Ok(Value::Boolean(rec.borrow().marked_only)),
                Some(v) => {
                    let mut r = rec.borrow_mut();
                    r.marked_only = ops::truthy(v)?;
                    r.position = None;
                    Ok(Value::Null)
                }
            },
            RecordOp::ClearMarks => {
                rec.borrow_mut().marks.clear();
                Ok(Value::Null)
            }
            // The bounds of a range filter on the field, whatever filter group it sits in (corpus record/filters/
            // GetRangeFilterGroups.al: P168_GetRange_FilterOnlyInGroup2, P168_GetRange_FilterInGroup0ReadFromGroup2). The filters of
            // several groups on the field intersect; an empty intersection and a field with no filter raise the errors that file pins.
            RecordOp::GetRangeMin | RecordOp::GetRangeMax => {
                use al_builtins::filter::FilterExpr as F;
                let pos = arg_pos(args, 0)?;
                let found: Vec<al_builtins::filter::FieldFilter> = rec
                    .borrow()
                    .filters
                    .iter()
                    .filter(|f| f.field == pos)
                    .cloned()
                    .collect();
                let t_cap = self.caption(t);
                let f_cap = self.field_caption(t, pos);
                if found.is_empty() {
                    return Err(Signal::Error(Box::new(AlError {
                        message: format!("Specify a filter for the {f_cap} field in the {t_cap} table."),
                        code: "DB:NoFilter".into(),
                        call_stack: String::new(),
                        ..Default::default()
                    })));
                }
                // One filter that is not a closed range (record/filters/P68B_RangeMinMax.al: RangeFilters_* and
                // NonRangeFilters_Text_Date; FieldRef: recordref-fieldref/recordref/RfTFieldFilters.al:183 GetRangeMinMax). A filter of
                // several ranges (`<>`, `|`) raises the one-range error; `>` and `>=` have no upper bound, `<` and `<=` no lower
                // bound, a wildcard neither. The bound of `>2` is 2 itself.
                if let [only] = found.as_slice() {
                    let want_max = method == RecordOp::GetRangeMax;
                    let text = || -> Result<String, Signal> {
                        let table = self.table(rec)?;
                        self.enum_ml_filter_check(&table, only)?;
                        self.filter_text(&self.with_option_captions(&table, pos, &only.expr))
                    };
                    let no_bound = |side: &str| -> Result<Value, Signal> {
                        Err(Signal::Error(Box::new(AlError {
                            message: format!(
                                "The filter \"{}\" on the {f_cap} field in the {t_cap} table has no {side} bound.",
                                text()?
                            ),
                            code: "DB:Ok".into(),
                            call_stack: String::new(),
                            ..Default::default()
                        })))
                    };
                    match &only.expr {
                        F::Gt(v) | F::Ge(v) if !want_max => return Ok(v.clone()),
                        F::Gt(_) | F::Ge(_) => return no_bound("upper"),
                        F::Lt(v) | F::Le(v) if want_max => return Ok(v.clone()),
                        F::Lt(_) | F::Le(_) => return no_bound("lower"),
                        F::Like(..) => return no_bound(if want_max { "upper" } else { "lower" }),
                        // An open range `..b` or `a..` (recordref-fieldref/fieldref/RangeMinMaxTypes.al:167-208
                        // Integer, :301-386 Code, :479-565 Date, :657-743 Option, :835-921 Enum): the missing side
                        // raises the no-bound error, the other side answers.
                        F::Range(None, _) if !want_max => return no_bound("lower"),
                        F::Range(_, None) if want_max => return no_bound("upper"),
                        F::Ne(_) | F::Or(_) => {
                            return Err(Signal::Error(Box::new(AlError {
                                message: format!(
                                    "The filter expression \"{}\" applied to the field {f_cap} in the {t_cap} table results in more than one range.",
                                    text()?
                                ),
                                code: "DB:NotIntervalFilter".into(),
                                call_stack: String::new(),
                                ..Default::default()
                            })));
                        }
                        _ => {}
                    }
                }
                // The bounds of every filter, as (low, high); a filter that is not a single range is not modelled.
                let mut lo: Option<Value> = None;
                let mut hi: Option<Value> = None;
                for f in &found {
                    let (a, b) = match &f.expr {
                        F::Eq(v) => (Some(v.clone()), Some(v.clone())),
                        F::Range(a, b) => (a.clone(), b.clone()),
                        _ => return unsupported(format!("{method} of a filter that is not a single range")),
                    };
                    if let Some(a) = a {
                        if lo.as_ref().is_none_or(|l| ops::compare(&a, l).is_ok_and(|o| o.is_gt())) {
                            lo = Some(a);
                        }
                    }
                    if let Some(b) = b {
                        if hi.as_ref().is_none_or(|h| ops::compare(&b, h).is_ok_and(|o| o.is_lt())) {
                            hi = Some(b);
                        }
                    }
                }
                // One reversed range gives its bounds as written (record/filters/P68B_RangeMinMax.al: RangeFilters_Integer,
                // `4..2`); only the filters of several groups can be empty together.
                if let (true, Some(l), Some(h)) = (found.len() > 1, &lo, &hi) {
                    if ops::compare(l, h).is_ok_and(|o| o.is_gt()) {
                        return Err(Signal::Error(Box::new(AlError {
                            message: format!(
                                "The filter expression \"<><>\" applied to the field {f_cap} in the {t_cap} table results in empty set."
                            ),
                            code: "DB:NotIntervalFilter".into(),
                            call_stack: String::new(),
                            ..Default::default()
                        })));
                    }
                }
                match (method, lo, hi) {
                    (RecordOp::GetRangeMin, Some(l), _) => Ok(l),
                    (RecordOp::GetRangeMax, _, Some(h)) => Ok(h),
                    _ => unsupported(format!("{method} of a filter that is not a single range")),
                }
            }
            // corpus record/TestRecordExtended.al:15-40: Record.Relation on a field without a TableRelation
            // raises an error whose text is not pinned (FieldRef.Relation answers 0 instead).
            RecordOp::Relation => {
                let pos = arg_pos(args, 0)?;
                let has = self.engine.program.objects[t.object as usize]
                    .table
                    .as_ref()
                    .is_some_and(|s| s.fields[pos].relation);
                if !has {
                    return unsupported("Relation of a field without a TableRelation (error text not pinned)");
                }
                self.field_relation(rec, t, pos).map(Value::Integer)
            }
            // Locking and permissions are not observable in one session: only the answers the corpus pins.
            RecordOp::LockTable => {
                // Wait = false fails, with or without VersionCheck (record/crud/RcBMeta.al:415-419,
                // record/TestRecordLock.al:38, session-system/TestBCLockingContracts.al:124; the RecordRef forms in
                // recordref-fieldref/recordref/RfTViews.al:332-360). The text names the table by its name (RcBMeta.al:418).
                if matches!(args.first(), Some(Value::Boolean(false))) {
                    return Err(lock_table_bad_parm(t));
                }
                // Record.LockTable(true, true): only the RecordRef form is pinned (RfTViews.al:314-315, handled in refs.rs).
                if matches!(args.get(1), Some(Value::Boolean(true))) {
                    return unsupported("Record.LockTable(true, true) (only the RecordRef form is pinned)");
                }
                // LockTable sets ReadIsolation back to Default (record/CvReadIsolationState.al:47-48, :59).
                rec.borrow_mut().read_isolation = 0;
                // On a database table it starts the write transaction, on a temporary one not
                // (codeunits/TestCodeunitRunWriteTransaction.al:211-252 RN_LockTable_WriteTransactionState, _Temporary_).
                if !rec.borrow().temporary {
                    self.tx.lock_tx = true;
                    let table = rec.borrow().table;
                    self.tx.locked_tables.push(table);
                }
                Ok(Value::Null)
            }
            RecordOp::Consistent => {
                let consistent = match args.first() {
                    Some(Value::Boolean(b)) => *b,
                    _ => return unsupported("Consistent without a Boolean argument"),
                };
                let (table, temporary) = {
                    let r = rec.borrow();
                    (r.table, r.temporary)
                };
                self.set_consistent(table, temporary, consistent)?;
                Ok(Value::Null)
            }
            RecordOp::ReadConsistency => Ok(Value::Boolean(false)),
            RecordOp::RecordLevelLocking => Ok(Value::Boolean(true)),
            RecordOp::ReadPermission | RecordOp::WritePermission => self.permission_answer(rec, method),
            RecordOp::CountApprox => self.count_approx(rec),
            RecordOp::CurrentCompany => Ok(Value::Text(self.current_company_of(rec).into())),
            RecordOp::ReadIsolation | RecordOp::SecurityFiltering => {
                self.isolation_or_security_filtering(method, rec, args)
            }
            RecordOp::GetPosition => {
                let use_names = match args.first() {
                    Some(v) => ops::truthy(v)?,
                    None => true,
                };
                self.get_position(rec, t, use_names).map(|s| Value::Text(s.into()))
            }
            RecordOp::SetPosition => {
                let text = al_builtins::native::text_of(args.first().unwrap_or(&Value::Null)).unwrap_or_default();
                self.set_position(rec, t, &text).map(|()| Value::Null)
            }
            RecordOp::TransferFields => self.transfer_fields(rec, t, args),
            RecordOp::GetBySystemId => self.get_by_system_id(rec, t, args),
            RecordOp::ModifyAll => self.modify_all(rec, t, args),
            // Doc (Record.Ascending): with an argument sets the sort direction of the current key,
            // without one reads it (corpus record/TestRecordSort.al:111, TestCopyOntoTemporaryRecord.al).
            RecordOp::Ascending => {
                // After a per-field SetAscending(Field, false) the record reads as descending (record/TestProbeRecordRefContracts.al:
                // GetTable_CopiesBufferFiltersKeyDirection, `rr_ascending` No).
                match args.first() {
                    None => Ok(Value::Boolean({
                        // Primary key fields after the key do not count (RecordSortAscendingViews.al:P161_SetAscending_Single_
                        // AscendingMethodAndTies `back.asc`).
                        let r = rec.borrow();
                        let key_bits = (1u32 << t.keys[r.key].len().min(31)) - 1;
                        r.ascending && r.field_descending & key_bits == 0
                    })),
                    Some(v) => {
                        let mut r = rec.borrow_mut();
                        r.ascending = ops::truthy(v)?;
                        r.field_descending = 0;
                        Ok(Value::Null)
                    }
                }
            }
            // The direction of one field of the current sorting: the key's fields, then the primary key's. A field outside
            // it raises `Cannot call <method> on field <field> because it is not part of the current sorting.` (code DB:Ok;
            // record/filters/P68D_Ascending.al:Field_Not_In_Key, Composite_Key_Default). Each field of a
            // composite key has a direction of its own, the rest of the sorting stays ascending (P68D_Ascending.al:
            // Composite_Key_First_Descending, _Second_Descending, _Both_Descending; record/W4SortKeys.al:
            // SetAscending_TwoFieldKey_OneFieldAtATime). Single field: record/TestRecordSort.al:184-203 (SetAscending(Amount, false)
            // makes FindFirst return the largest), :158-181 (GetAscending).
            // The companies are the session company and the rows of the Company table; a record in
            // another company reads and writes that company's rows (record/TestRecordChangeCompanyRows.al).
            RecordOp::ChangeCompany => self.change_company(rec, args),
            RecordOp::SetAscending | RecordOp::GetAscending => {
                let pos = arg_pos(args, 0)?;
                let key = rec.borrow().key;
                let Some(bit) = t.keys[key].iter().chain(&t.keys[0]).position(|&p| p == pos) else {
                    let name = if method == RecordOp::GetAscending {
                        "GetAscending"
                    } else {
                        "SetAscending"
                    };
                    // The field's name, not its caption (record/lt-filters-keys-finds/SetAscendingEdges.al:
                    // SetAscending_FieldOutsideTheKey_ErrorText, `Qty` for the field captioned 'LT Quantity').
                    return Err(crate::transactions::al_error_code(
                        &format!(
                            "Cannot call {name} on field {} because it is not part of the current sorting.",
                            t.fields[pos].name
                        ),
                        "DB:Ok",
                    ));
                };
                // A primary key field that only follows the current key can be read but not set
                // (SetAscendingEdges.al:SetAscending_PrimaryKeyFieldFollowingTheKey; record/keys-sorting/
                // SetAscendingTies.al:153-154, :174). Asking it for the direction it already runs in after a per-field
                // SetAscending(KeyField, false) is accepted and changes nothing (SetAscendingTies.al:155 `grp_desc_id_asc`).
                // The other requests for the direction it already has are not pinned.
                if method == RecordOp::SetAscending && bit >= t.keys[key].len() {
                    let wanted = match args.get(1) {
                        Some(v) => ops::truthy(v)?,
                        None => return unsupported("SetAscending without a direction"),
                    };
                    let (ascending, descending) = {
                        let r = rec.borrow();
                        (r.ascending, r.field_descending)
                    };
                    if ascending && descending != 0 && wanted {
                        return Ok(Value::Null);
                    }
                    if !((ascending && descending == 0 && !wanted) || (!ascending && wanted)) {
                        return unsupported(format!(
                            "SetAscending({}, {wanted}) on a primary key field that follows the key: the outcome is not pinned",
                            t.fields[pos].name
                        ));
                    }
                    return Err(crate::transactions::al_error_code(
                        &format!(
                            "The specified sorting could not be set on field '{}' with the key consisting of '{}'.",
                            t.fields[pos].name,
                            t.keys[key]
                                .iter()
                                .map(|&p| t.fields[p].name.as_str())
                                .collect::<Vec<_>>()
                                .join(",")
                        ),
                        "InvalidSortingFields",
                    ));
                }
                if bit >= 32 {
                    return unsupported(format!("{method} on the {bit}th field of a sorting"));
                }
                if method == RecordOp::GetAscending {
                    let r = rec.borrow();
                    return Ok(Value::Boolean(r.ascending && r.field_descending & (1 << bit) == 0));
                }
                let ascending = match args.get(1) {
                    Some(v) => ops::truthy(v)?,
                    None => return unsupported("SetAscending without a direction"),
                };
                // The direction holds for this record until a page hands its view back and forth
                // (testpage/TestPageViewRoundTripRoutes.al:Action_SetAscending_ReadInTheSameTrigger_IsDescending
                // against ..._ReadByALaterAction); `Ascending` keeps its own, view-borne flag.
                let mut r = rec.borrow_mut();
                if !r.ascending {
                    // After Ascending(false) every field of the key reads as descending, and the one field changes
                    // (SetAscendingEdges.al:SetAscending_AfterAscendingFalse, `then_grp_true_view`). The primary key
                    // fields that follow the key run descending too, so rows with equal key values stay in descending
                    // order (record/keys-sorting/RecordSortAscendingViews.al:P161_SetAscending_Single_AscendingMethodAndTies
                    // `back.walk`).
                    r.ascending = true;
                    let fields = t.keys[key].len() + t.keys[0].iter().filter(|p| !t.keys[key].contains(p)).count();
                    r.field_descending = if fields >= 32 { u32::MAX } else { (1u32 << fields) - 1 };
                }
                if ascending {
                    r.field_descending &= !(1 << bit);
                } else {
                    r.field_descending |= 1 << bit;
                }
                Ok(Value::Null)
            }
            // The key's field names (corpus record/TestCopyOntoTemporaryRecord.al:93 `'No.'`,
            // TestBCMetadataContracts.al:196-208 contains the names).
            RecordOp::CurrentKey => {
                let key = rec.borrow().key;
                Ok(Value::Text(
                    t.keys[key]
                        .iter()
                        // The hidden key on SystemId (recordref-fieldref/recordref/TestRecordRefKeyIndexSetRecFilterContracts.al:101); a field with
                        // a Caption shows it (records/lt-record-meta/RcBMeta*.al:CurrentKey_Default_And_Set `Identifier`).
                        .map(|&p| {
                            if t.fields[p].id == 2_000_000_000 {
                                "System ID".to_string()
                            } else {
                                self.field_caption(t, p)
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(",")
                        .into(),
                ))
            }
            RecordOp::HasFilter => Ok(Value::Boolean(Self::has_filter(rec))),
            RecordOp::GetFilters => self.get_filters(rec, t).map(|s| Value::Text(s.into())),
            RecordOp::GetFilter => self
                .field_filter_text(rec, arg_pos(args, 0)?)
                .map(|s| Value::Text(s.into())),
            RecordOp::GetView => {
                let use_names = match args.first() {
                    Some(v) => ops::truthy(v)?,
                    None => true,
                };
                self.get_view(rec, t, use_names).map(|s| Value::Text(s.into()))
            }
            RecordOp::SetView => {
                let view = al_builtins::native::text_of(args.first().unwrap_or(&Value::Null)).unwrap_or_default();
                self.set_view(rec, t, &view).map(|()| Value::Null)
            }
            // Returns the group it sets, or the current one without an argument (corpus
            // record/TestRecordRuntimeTextsAndViews.al:Record_FilterGroup_ReturnsTheGroupJustSet_...).
            RecordOp::FilterGroup => Self::filter_group(rec, args),
            // Doc (Record.SetRecFilter): filters the primary key fields to the current values; the
            // filters on other fields stay (record/TestRecordViewCopyMarkContracts.al:41,
            // SetRecFilter_KeepsFiltersOnOtherFields_AndFiltersOnThePrimaryKey).
            // With another current key it also filters that key's fields, after the primary key's
            // (record/W4SortKeys.al:RecordRef_SetRecFilter_AndRecordSetRecFilter: current key Txt gives
            // `Id: 3, Txt: b`, on a Record and on a RecordRef, BC 27.5 and 29.0).
            RecordOp::SetRecFilter => {
                let group = rec.borrow().filter_group;
                let mut pk: Vec<usize> = t.keys[0].clone();
                let current = rec.borrow().key;
                if let Some(k) = t.keys.get(current) {
                    for &p in k {
                        if !pk.contains(&p) {
                            pk.push(p);
                        }
                    }
                }
                let pk = &pk;
                let mut r = rec.borrow_mut();
                r.filters.retain(|f| !(pk.contains(&f.field) && f.group == group));
                for &p in pk {
                    let v = r.fields[p].clone();
                    r.filters.push(al_builtins::filter::FieldFilter {
                        lang: al_builtins::filter::filter_language(),
                        field: p,
                        expr: al_builtins::filter::FilterExpr::Range(Some(v.clone()), Some(v)),
                        group,
                    });
                }
                Ok(Value::Null)
            }
            // `CopyFilter(FromField, Target.ToField)` (the lowerer passes the target record and the
            // position): the filter of the source field arrives on the target field and replaces
            // what that field had; a source field without a filter clears the target's
            // (record/TestRecordFilter.al:226-239, TestFilterAdvancedContracts.al:216-223). Both
            // fields have to be of one type, and only the group 0 filters are copied: how a filter
            // group or a conversion between types behaves is not pinned.
            RecordOp::CopyFilter => {
                let [Value::Integer(from), Value::Record(dst), Value::Integer(to)] = args else {
                    return unsupported(format!("CopyFilter with {}", args.describe()));
                };
                let (from, to) = (*from as usize, *to as usize);
                let dt = self.table(dst)?;
                // A Code field's filter cannot go to a Text field: the constant must be compatible with the
                // target field (types/ProbeTypeFormsAndFilters.al:361-395, BC 27.5). The other combinations of
                // different types are not pinned.
                if t.fields[from].ty != dt.fields[to].ty {
                    if matches!(
                        (&t.fields[from].ty, &dt.fields[to].ty),
                        (RtType::Code(_), RtType::Text(_))
                    ) {
                        return Err(error(
                            format!(
                                "The type of the constant must be compatible with that of the {to_name} field, and the length of the constant must not exceed the length of the field.\\\\Type: Text <-- Code\\Field: {to_name} <-- {from_name}\\Table: {table}",
                                to_name = dt.fields[to].name,
                                from_name = t.fields[from].name,
                                table = dt.name,
                            ),
                            "NCLCSRTS:TableEqualValue",
                        ));
                    }
                    return unsupported("CopyFilter between fields of different types (not pinned)");
                }
                // The filters of the source's current group go to the target's current group; a field the source
                // does not filter in that group clears the target's (same file, :386-393).
                let (to_group, copied): (i32, Vec<_>) = {
                    let r = rec.borrow();
                    let g = r.filter_group;
                    (
                        dst.borrow().filter_group,
                        r.filters
                            .iter()
                            .filter(|f| f.field == from && f.group == g)
                            .cloned()
                            .collect(),
                    )
                };
                Self::put_copied_filters(dst, to, to_group, copied);
                Ok(Value::Null)
            }
            RecordOp::CopyFilters => Self::copy_filters(rec, args),
            RecordOp::Copy => self.record_copy(rec, args),
            RecordOp::TestField => self.test_field(rec, t, args),
            RecordOp::FieldError => self.field_error(rec, t, args),
            other => unsupported(format!("Record.{other}")),
        }
    }

    /// `Record.Copy(Source [, ShareTable])`. Doc (Record.Copy) and corpus record/TestCopyOntoTemporaryRecord.al:66-110: the
    /// buffer, filters, key and direction come over; a temporary target keeps its own rows and stays temporary.
    fn record_copy(&self, rec: &Rec, args: &[Value]) -> Result<Value, Signal> {
        let Some(Value::Record(src)) = args.first() else {
            return unsupported(format!("Copy from {}", args.first().describe()));
        };
        let share = match args.get(1) {
            Some(v) => ops::truthy(v)?,
            None => false,
        };
        if Rc::ptr_eq(src, rec) {
            return Ok(Value::Null);
        }
        if crate::records::media_key_marked(src) {
            return unsupported(format!("Copy of a record {}", crate::records::MEDIA_KEY));
        }
        let s = src.borrow().clone();
        // Copy takes the fields, filters and key but does not move the target to the source's company
        // (record/TestChangeCompanyFiltersCopyDeleteContracts.al:117-121).
        let mut d = rec.borrow_mut();
        // shareTable needs both records temporary; a plain Copy of a temporary record onto
        // a normal one copies values and filters only
        // (record/TestRecordViewCopyMarkContracts.al:89, Copy_ShareTable_RequiresBothRecordsToBeTemporary).
        if share && !(s.temporary && d.temporary) {
            return Err(error(
                "The COPY function can only be used with the shareTable argument set to true if both records are temporary.".into(),
                "Argument",
            ));
        }
        // The row version the buffer got comes over with the fields, and where it came from (record/crud/
        // Y76StaleBuffers.al Y76_Part1_Copy_TransferFromRecord, `copy/*`).
        (d.fields, d.row_origin) = (s.fields, s.row_origin);
        // Between two temporary records or two that are not, the Blobs move as `blob_transfer::moved` says; between
        // one of each they come as before.
        if s.temporary == d.temporary {
            let table = d.table as usize;
            self.move_blobs(table, &mut d.fields, None, BlobMove::RecordCopy)?;
        }
        d.filters = s.filters;
        d.filter_group = s.filter_group;
        d.key = s.key;
        d.adhoc = s.adhoc;
        d.ascending = s.ascending;
        d.field_descending = s.field_descending;
        d.position = s.position;
        // ReadIsolation comes over with Copy (record/CvReadIsolationState.al:49-51, :61); SecurityFiltering stays the
        // target's (al-language/reports/runtime/ACAttributes.al:385-387).
        d.read_isolation = s.read_isolation;
        // The marks come over with Copy, not with an assignment (record/crud/RcAMisc.al:
        // Mark_Separate_Variables_Do_Not_Share `copied`).
        d.marks = s.marks;
        d.marked_only = s.marked_only;
        // The SetAutoCalcFields set comes over (record/crud/RcBFlowFields.al:SetAutoCalcFields_AfterInsertRead
        // `copy keeps autocalc`).
        d.auto_calc = s.auto_calc;
        // SetLoadFields comes over with Copy (record/crud/RcAMisc.al:SetLoadFields_Copy_And_Assignment).
        d.load.request = s.load.request;
        d.load.buffer = s.load.buffer;
        d.load.partial = s.load.partial;
        if share && s.temporary {
            d.temp_id = s.temp_id.clone();
        }
        Ok(Value::Null)
    }

    /// `TestField(Field [, Value])`. The "must have a value" text is pinned by the corpus
    /// (record/TestRecordTestFieldLookupPageId_Tests.al:54-72); the mismatch text is Assert.al's
    /// documented "new format" (docs/probes-needed.md).
    /// A value as TestField shows it: an Option or Enum member by its caption (a member without one by
    /// its name), anything else as in any error message.
    fn member_caption(&self, t: &TableDef, pos: usize, v: &Value) -> Result<String, Signal> {
        let ordinal = match v {
            Value::Option(o, _) | Value::Enum(_, o) => *o,
            other => return self.message_value(other),
        };
        match self.members_of_field(t, pos)? {
            Some((_, members)) => Ok(members
                .iter()
                .find(|(o, _, _)| *o == ordinal)
                .map_or_else(|| ordinal.to_string(), |(_, _, caption)| caption.clone())),
            None => self.message_value(v),
        }
    }

    pub(crate) fn test_field(&mut self, rec: &Rec, t: &TableDef, args: &[Value]) -> Result<Value, Signal> {
        // `TestField(Field [, Value], ErrorInfo)`: a check that holds raises nothing and leaves the ErrorInfo as it
        // was. A failing one raises the same text and code as the form without an ErrorInfo, whatever Message the
        // ErrorInfo carries; in a collecting scope a collectible ErrorInfo is collected instead (a copy with its own
        // Message, the tested FieldNo and the TableId), and a call that is not collectible keeps FieldNo and TableId
        // 0 on the caller's variable (types/PUTestFieldErrorInfo.al, BC 27.5 and 29.0).
        if let Some((info @ Value::Library(kind, _), plain)) = args.split_last()
            && kind.as_str() == "ErrorInfo"
        {
            return match self.test_field(rec, t, plain) {
                Err(Signal::Error(e)) => {
                    if let Some(copy) = al_builtins::errorinfo::copy_of(info) {
                        let pos = arg_pos(plain, 0)?;
                        let collectible = al_builtins::errorinfo::set_origin_of(
                            &copy,
                            self.engine.program.objects[t.object as usize].id.unwrap_or(0) as i32,
                            t.fields[pos].id as i32,
                        );
                        // No Message on the ErrorInfo: the collected one says what the uncollected call raises. The
                        // given Message is pinned (types/PUTestFieldErrorInfo.al:143-166), the empty one is not.
                        if collectible && self.collect_error_if_collecting() {
                            if al_builtins::errorinfo::message_of(&copy).is_some_and(|m| m.is_empty()) {
                                al_builtins::assumptions::assume(
                                    al_builtins::assumptions::Assumption::TestfieldErrorinfoEmptyMessageCollected,
                                );
                                al_builtins::errorinfo::set_message_of(&copy, &e.message);
                            }
                            // The collected error names the record it was raised on: Gen. Jnl.-Check Line's callers match it
                            // with the line (ErrInfo.RecordId in Error Message Management.CollectErrors).
                            al_builtins::assumptions::assume(
                                al_builtins::assumptions::Assumption::TestfieldErrorinfoCollectedRecordId,
                            );
                            al_builtins::errorinfo::set_record_id_of(&copy, self.record_id(rec, t));
                            self.collect_error(&copy);
                            return Ok(Value::Null);
                        }
                    }
                    Err(Signal::Error(e))
                }
                other => other,
            };
        }
        if args.len() > 2 {
            return unsupported("TestField with an ErrorInfo");
        }
        let pos = arg_pos(args, 0)?;
        let (current, row) = {
            let r = rec.borrow();
            (r.fields[pos].clone(), r.fields.clone())
        };
        let caption = self.field_caption(t, pos);
        let table = self.caption(t);
        match args.get(1) {
            None => {
                if is_blank(&current)? {
                    // "<field> must not be <caption of the zero member> in <table>: <key>", code TestField
                    // (record/TestTestFieldOptionEnumContracts.al:TestField_ZeroMember_RaisesMustNotBeTheCaption).
                    if matches!(current, Value::Option(..) | Value::Enum(..)) {
                        let key = self.key_text(t, &row)?;
                        let member = self.member_caption(t, pos, &current)?;
                        // The flag hides the key and keeps the member (errors/last-error/
                        // TestFieldOptionEnumExcludeFlag.al:80, :83, :111, :114).
                        return Err(error_redacted(
                            format!("{caption} must not be {member} in {table}: {key}"),
                            "TestField",
                            format!("{caption} must not be {member} in {table}: <redacted>"),
                        ));
                    }
                    let key = self.key_text(t, &row)?;
                    // The flag hides the key: P86LastError.al:233, :236, P70_02LastErrFlag.al:123, :141.
                    return Err(error_redacted(
                        format!("{caption} must have a value in {table}: {key}. It cannot be zero or empty."),
                        "TestField",
                        format!("{caption} must have a value in {table}: <redacted>. It cannot be zero or empty."),
                    ));
                }
            }
            Some(want) => {
                // Text against a text field and a number against a number field are compared as given, without converting to the
                // field's type: a Code field is not upper-cased or trimmed, a Text longer than the field raises no overflow, a Decimal
                // against an Integer field is not an overflow (record/testfield/P96Text.al, P96LongText.al,
                // P96Numbers.al). The message shows the value as given.
                // A one-character text that came through a Variant is a Char: it is compared as the text it is (P96Text.al
                // `ref_empty_cd_blank`).
                let want_owned;
                let want = match want {
                    Value::Char(c) if matches!(current, Value::Text(_) | Value::Code(_)) => {
                        want_owned = Value::Text(char::from_u32(u32::from(*c)).unwrap_or(' ').to_string().into());
                        &want_owned
                    }
                    other => other,
                };
                let text_pair = matches!(
                    (&current, want),
                    (Value::Text(_) | Value::Code(_), Value::Text(_) | Value::Code(_))
                );
                let number_pair = matches!(
                    (&current, want),
                    (
                        Value::Integer(_) | Value::BigInteger(_) | Value::Decimal(_),
                        Value::Integer(_) | Value::BigInteger(_) | Value::Decimal(_)
                    )
                );
                // The expected value as the message writes it, for the typed values the corpus pins (record/testfield/):
                // an Enum on an Integer field shows its member (`One`, RecordTestFieldWrongTypeEnumOnInteger.al); a Byte is written as an
                // escaped character from BC 28 (`\u0008`, FieldRefTestFieldTypedNumeric.al:P189_FldRef_Byte_Value; a number before);
                // a Guid on a Text field is never equal and is written with braces from BC 28 (RecordTestFieldWrongTypeGuidOnText.al).
                // Other Byte values and a Guid against another field type are not pinned.
                let from28 = self.has(al_version::Gate::TestFieldTypedValueTexts);
                let mut shown: Option<String> = None;
                let mut never_equal = false;
                let mut byte_unpinned = false;
                let mut text_number_equal = false;
                match (&current, want) {
                    (Value::Integer(_), Value::Enum(id, o)) => {
                        if let Some(why) = self.enum_caption_refusal(*id, Some(*o)) {
                            return unsupported(why);
                        }
                        if let Some(why) = self.enum_ml_unpinned(*id, Some(*o), "a field comparison read") {
                            return unsupported(why);
                        }
                        shown = Some(al_builtins::format::FormatCtx::enum_name(self, *id, *o))
                    }
                    // An Enum against a Text field is never equal and is written as its member (:229-231, Knd_result No, 'Two').
                    (Value::Text(_) | Value::Code(_), Value::Enum(id, o)) => {
                        if let Some(why) = self.enum_caption_refusal(*id, Some(*o)) {
                            return unsupported(why);
                        }
                        if let Some(why) = self.enum_ml_unpinned(*id, Some(*o), "a field comparison read") {
                            return unsupported(why);
                        }
                        shown = Some(al_builtins::format::FormatCtx::enum_name(self, *id, *o));
                        never_equal = true;
                    }
                    (Value::Text(_) | Value::Code(_), Value::Guid(g)) => {
                        let braced = al_builtins::format::format_guid(g);
                        shown = Some(if from28 {
                            braced
                        } else {
                            braced.trim_matches(['{', '}']).to_string()
                        });
                        never_equal = true;
                    }
                    // A Guid field against a different Guid: the wanted value is braced from BC 28 and unbraced before, the current
                    // value is braced on every version (record/testfield/RecordTestFieldGroupB.al:Guid_Plain,
                    // recordref-fieldref/fieldref/FieldRefTestFieldGroupB.al:Guid_Plain; `#if BC28PLUS`).
                    (Value::Guid(_), Value::Guid(g)) if !from28 => {
                        shown = Some(al_builtins::format::format_guid(g).trim_matches(['{', '}']).to_string());
                    }
                    // Only the pinned values: 6 and 8 as `\u000N` (recordref-fieldref/fieldref/FieldRefTestFieldGroupA.al:254-260,
                    // record/testfield/FieldRefTestFieldTypedNumeric.al:166), 10 as `\n` (record/lt-as-records/
                    // AsTestFieldTexts.al:41-42) and 200 as U+02DC small tilde (:60-62). An equal value shows nothing, so only a
                    // mismatch of another value is refused (below).
                    (_, Value::Byte(b)) if from28 && matches!(current, Value::Integer(_)) => match *b {
                        6 | 8 => shown = Some(format!("\\u{:04X}", b)),
                        10 => shown = Some("\\n".to_string()),
                        200 => shown = Some("\u{02DC}".to_string()),
                        _ => byte_unpinned = true,
                    },
                    // A number against a Text or Code field is equal when the field's text reads as that number
                    // (record/testfield/FieldRefTestFieldMismatchedType.al:206-211 and :223,
                    // P189_FldRefMismatch_OnTextField: Big, Dec and Int 5 against '5' are Yes). The number is not converted to the
                    // field's type (a number into a Text is a type-conversion error). A number that is not equal has no pinned
                    // message, so it is refused below.
                    (Value::Text(_) | Value::Code(_), Value::Integer(_) | Value::BigInteger(_) | Value::Decimal(_)) => {
                        text_number_equal = ops::convert(current.clone(), &RtType::Decimal)
                            .ok()
                            .and_then(|d| ops::compare(&d, want).ok())
                            == Some(Ordering::Equal);
                        if !text_number_equal {
                            return unsupported(
                                "TestField of a number against a Text field that holds another value (message not pinned)",
                            );
                        }
                    }
                    // A Boolean true against a Text field holding '5' is equal (:219, P189_FldRefMismatch_OnTextField Bool_result Yes).
                    // Why is not known (BC does not convert '5' to a Boolean), so only that value is implemented; any other text is
                    // refused.
                    (Value::Text(t) | Value::Code(t), Value::Boolean(b)) => {
                        if !(*b && t.as_ref() == "5") {
                            return unsupported(
                                "TestField of a Boolean against a Text field (only true against '5' is pinned)",
                            );
                        }
                        text_number_equal = true;
                    }
                    (Value::Guid(_), Value::Guid(_)) => {}
                    (_, Value::Guid(_)) => {
                        return unsupported("TestField against a Guid on a field that is not Text (not pinned)");
                    }
                    _ => {}
                }
                let want = if text_pair || number_pair || never_equal || text_number_equal {
                    want.clone()
                } else {
                    ops::convert(want.clone(), &t.fields[pos].ty)?
                };
                let equal = if never_equal {
                    false
                } else if text_number_equal {
                    true
                } else if text_pair {
                    // Exact, case and trailing blanks included; a Code field reads the value as Code does upper-cased
                    // (P96Text.al: `cd_lower` is equal, `cd_trailing_blank` and `txt_case` are not).
                    let (a, b) = (
                        al_builtins::native::text_of(&current),
                        al_builtins::native::text_of(&want),
                    );
                    let code_field = matches!(t.fields[pos].ty, RtType::Code(_));
                    matches!((a, b), (Some(a), Some(b)) if if code_field { a == b.to_uppercase() } else { a == b })
                } else {
                    ops::compare(&current, &want)? == Ordering::Equal
                };
                if !equal {
                    if byte_unpinned {
                        return unsupported("TestField against a Byte whose escaped form is not pinned");
                    }
                    let key = self.key_text(t, &row)?;
                    // record/TestRecordRuntimeTextsAndViews.al:Record_TestField_MismatchTexts: the
                    // wanted and the current value are quoted, and two spaces precede "in".
                    // The flag hides the key and both values: P86LastError.al:229, :233, P70_02LastErrFlag.al:129, :135.
                    return Err(error_redacted(
                        format!(
                            "{caption} must be equal to '{}'  in {table}: {key}. Current value is '{}'.",
                            match shown {
                                Some(text) => text,
                                None => self.member_caption(t, pos, &want)?,
                            },
                            self.member_caption(t, pos, &current)?
                        ),
                        "TestField",
                        format!(
                            "{caption} must be equal to '<redacted>'  in {table}: <redacted>. Current value is '<redacted>'."
                        ),
                    ));
                }
            }
        }
        Ok(Value::Null)
    }

    /// `GetPosition`: the primary key fields, whatever the current key is, as `Caption=CONST(value)`
    /// (`Field<n>=0(value)` with `UseNames = false`); corpus record/TestRecordRuntimeTextsAndViews.al:
    /// RecordId_Format_And_GetPosition, recordref-fieldref/recordref/P92Position.al:GetPosition_NamesAndDefault
    /// (the field's caption, else its name). A value with a parenthesis is written in double quotes; a comma and an
    /// apostrophe stay as they are (:GetPosition_CompositeKey, :GetPosition_SpecialCharactersInKey).
    fn get_position(&self, rec: &Rec, t: &TableDef, use_names: bool) -> Result<String, Signal> {
        let r = rec.borrow();
        let mut parts = Vec::new();
        for &p in &t.keys[0] {
            let mut value = self.message_value(&r.fields[p])?;
            // A double quote is written as it is when the value needs no quotes around it (record/crud/RcAMisc.al:
            // GetPosition_Text_With_Special_Characters `a"b`); inside quotes it is not pinned.
            if value.contains(['(', ')']) {
                if value.contains('"') {
                    return unsupported("a position value with a double quote and a parenthesis");
                }
                value = format!("\"{value}\"");
            }
            if use_names {
                parts.push(format!("{}=CONST({value})", self.field_caption(t, p)));
            } else {
                parts.push(format!("Field{}=0({value})", t.fields[p].id));
            }
        }
        Ok(parts.join(","))
    }

    /// `SetPosition`: puts the key values of a position into the buffer (the row is not read). The text is a list of
    /// `Name=CONST(value)` or `Field<n>=0(value)` items; a value with a parenthesis is in double quotes. A text that
    /// is no such list is an "Invalid expression of type: position" error; a list that is not exactly the primary key
    /// fields in order is "not a valid position specification" (both code DB:Ok); then each value is evaluated into its
    /// field (recordref-fieldref/recordref/P92Position.al:SetPosition_Malformed, :SetPosition_FromOtherTable).
    fn set_position(&self, rec: &Rec, t: &TableDef, text: &str) -> Result<(), Signal> {
        let db_ok = |message: String| {
            Signal::Error(Box::new(al_builtins::native::AlError {
                message,
                code: "DB:Ok".into(),
                call_stack: String::new(),
                ..Default::default()
            }))
        };
        let invalid = || db_ok(format!("Invalid expression of type: position.\\{text}"));
        let mut items: Vec<(String, String)> = Vec::new();
        let mut rest = text;
        while !rest.is_empty() {
            let (name, after) = rest.split_once('=').ok_or_else(invalid)?;
            let after = after
                .strip_prefix("CONST(")
                .or_else(|| after.strip_prefix("0("))
                .ok_or_else(invalid)?;
            let (value, after) = if let Some(quoted) = after.strip_prefix('"') {
                let (v, tail) = quoted.split_once('"').ok_or_else(invalid)?;
                (v, tail.strip_prefix(')').ok_or_else(invalid)?)
            } else {
                after.split_once(')').ok_or_else(invalid)?
            };
            items.push((name.to_string(), value.to_string()));
            rest = match after.strip_prefix(',') {
                Some(more) if !more.is_empty() => more,
                Some(_) => return Err(invalid()),
                None if after.is_empty() => "",
                None => return Err(invalid()),
            };
        }
        let key = &t.keys[0];
        let mut matched = items.len() == key.len();
        if matched {
            for ((name, _), &p) in items.iter().zip(key) {
                let by_id = name
                    .strip_prefix("Field")
                    .and_then(|n| n.parse::<u32>().ok())
                    .is_some_and(|id| id == t.fields[p].id);
                let caption = self.field_caption(t, p);
                if !by_id && !name.eq_ignore_ascii_case(&caption) {
                    if name.eq_ignore_ascii_case(&t.fields[p].name) {
                        return unsupported(format!("position with the field name `{name}`, not its caption"));
                    }
                    matched = false;
                }
            }
        }
        if !matched {
            return Err(db_ok(format!(
                "'{text}' is not a valid position specification for the {} table. The position must contain all the primary key fields, in order, and only primary key fields.",
                self.caption(t)
            )));
        }
        let mut values = Vec::new();
        for ((_, value), &pos) in items.iter().zip(key) {
            let ty = &t.fields[pos].ty;
            let v = match ty {
                RtType::Text(_) | RtType::Code(_) => ops::convert(Value::Text(value.as_str().into()), ty)?,
                _ if value.is_empty() => Value::default_for(ty),
                RtType::Integer | RtType::BigInteger | RtType::Decimal => match self.parse_filter_text(value, ty) {
                    Ok(al_builtins::filter::FilterExpr::Eq(v)) => v,
                    _ => {
                        let name = match ty {
                            RtType::Integer => "Integer",
                            RtType::BigInteger => "BigInteger",
                            _ => "Decimal",
                        };
                        return Err(error(
                            format!("The value \"{value}\" can't be evaluated into type {name}."),
                            "Evaluate",
                        ));
                    }
                },
                _ => match self.parse_filter_text(value, ty) {
                    Ok(al_builtins::filter::FilterExpr::Eq(v)) => v,
                    _ => return unsupported(format!("position value `{value}`")),
                },
            };
            values.push((pos, v));
        }
        let mut r = rec.borrow_mut();
        for (pos, v) in values {
            r.fields[pos] = v;
        }
        Ok(())
    }

    /// `TransferFields(From [, InitPrimaryKeyFields [, SkipFieldsNotMatchingType]])`: normal fields
    /// are matched by field number; a primary key field is only taken when asked for; a field of
    /// another type is skipped or refused (corpus record/TestRecordTransferFields.al:14-95).
    fn transfer_fields(&mut self, rec: &Rec, t: &TableDef, args: &[Value]) -> Result<Value, Signal> {
        let Some(Value::Record(src)) = args.first() else {
            return unsupported(format!("TransferFields from {}", args.first().describe()));
        };
        let init_pk = match args.get(1) {
            Some(v) => ops::truthy(v)?,
            None => true,
        };
        let skip_mismatch = match args.get(2) {
            Some(v) => ops::truthy(v)?,
            None => false,
        };
        // The target keeps its own SecurityFiltering, also one a SecurityFiltering(x) call set (al-language/reports/runtime/
        // SFSecurityFilteringFlow.al:185-208); Base Application tests read it after a TransferFields (codeunit 137409
        // CalcAnalysisReportValueUnitPrice, 137400 SalesUnitPriceFromCustomerPriceLineGroup).
        if Rc::ptr_eq(src, rec) {
            return Ok(Value::Null);
        }
        let (src_table, src_fields) = {
            let s = src.borrow();
            (s.table as usize, s.fields.clone())
        };
        let from = self.engine.tables[src_table]
            .clone()
            .ok_or_else(|| Signal::Unsupported("TransferFields from a table without a schema".into()))?;
        let same_kind = |a: &al_builtins::value::RtType, b: &al_builtins::value::RtType| {
            std::mem::discriminant(a) == std::mem::discriminant(b)
        };
        let cross_app = {
            let objects = &self.engine.program.objects;
            objects[t.object as usize].app != objects[from.object as usize].app
        };
        // The kinds the probes of a skipped field across apps cover (Y38TransferXApp.al, RcBTransfer.al:553); others stay refused.
        let skipped_across_apps = |a: &RtType, b: &RtType| {
            let probed = |ty: &RtType| {
                matches!(
                    ty,
                    RtType::Boolean
                        | RtType::Integer
                        | RtType::Decimal
                        | RtType::Option(_)
                        | RtType::Date
                        | RtType::DateTime
                        | RtType::Text(_)
                        | RtType::Code(_)
                )
            };
            probed(a) && probed(b)
        };
        let mut updates = Vec::new();
        let engine = self.engine.clone();
        let schema = engine.program.objects[t.object as usize].table.as_ref().expect("table");
        for (dp, df) in t.fields.iter().enumerate() {
            // Within one table a FlowField's value moves too, assigned or calculated; from or into another table it does not
            // (record/flowfields/DiFwWriteTests.al:504-513, `assigned to normal`, `normal to flow`).
            if df.id < 2_000_000_000 && !df.stored && !schema.fields[dp].flow_filter && src_table == t.object as usize {
                updates.push((dp, src_fields[dp].clone()));
                continue;
            }
            if df.id >= 2_000_000_000 || !df.stored {
                continue;
            }
            if !init_pk && t.keys[0].contains(&dp) {
                continue;
            }
            let Some(sp) = from.fields.iter().position(|f| f.id == df.id && f.stored) else {
                continue;
            };
            // Fields of one number declared by different apps count as fields of different apps, also on two tables of one app:
            // a table extension's field against another app's extension field or against the table's own field
            // (record/transferfields-cross-app/TransferFieldsFieldApps.al:131-163 on corpus master
            // 3eb812ed, al-runner issue 6982).
            let cross_app = cross_app || self.field_app(t, dp) != self.field_app(&from, sp);
            // An Enum field and an Option field with the same field number are the same kind of field to
            // TransferFields: Base Application's "Sales Order Entity Buffer" holds `Shipping Advice` as an
            // Option and `Sales Header` as an Enum, and its TransferFields from the header works in BC
            // (corpus record/TransferEnumOption.al). The ordinal moves.
            let enum_option = |a: &al_builtins::value::RtType, b: &al_builtins::value::RtType| {
                use al_builtins::value::RtType as T;
                matches!((a, b), (T::Enum(_), T::Option(_)) | (T::Option(_), T::Enum(_)))
            };
            if enum_option(&from.fields[sp].ty, &df.ty) {
                let ordinal = match &src_fields[sp] {
                    Value::Enum(_, o) | Value::Option(o, _) => *o,
                    other => {
                        return unsupported(format!(
                            "TransferFields of {} between an Enum and an Option",
                            other.describe()
                        ));
                    }
                };
                let moved = match &df.ty {
                    al_builtins::value::RtType::Option(m) => Value::Option(ordinal, m.clone()),
                    // An ordinal the Enum lacks moves too, with no error (corpus record/lt-record-conversion-assumptions/
                    // TransferEnumOption.al: P98_OptionToEnum_MissingOrdinal, P98_EnumToOption_MissingOrdinal).
                    al_builtins::value::RtType::Enum(id) => Value::Enum(*id, ordinal),
                    _ => unreachable!("enum_option"),
                };
                updates.push((dp, moved));
                continue;
            }
            // A Code field moves into a Text field and a Text field into a Code field, within one app and across apps, with
            // and without SkipFieldsWithoutCheck (record/transferfields-cross-app/Y38TransferXApp.al:449-512 `Cod`, `Txt`; :549 p41,
            // :565 p43, :589-591; RcBTransfer.al:553 `search`).
            let textual = |ty: &RtType| matches!(ty, RtType::Text(_) | RtType::Code(_));
            let text_pair = textual(&from.fields[sp].ty) && textual(&df.ty);
            if let (RtType::Text(_), RtType::Code(Some(to_len)), Value::Text(s)) =
                (&from.fields[sp].ty, &df.ty, &src_fields[sp])
                && Value::text_len(s) > *to_len as usize
            {
                // Across apps it is skipped, with and without SkipFieldsWithoutCheck (TransferFieldsFieldApps.al:147-148,
                // `mode 1 long text`, `mode 3 long text`).
                if cross_app {
                    continue;
                }
                return unsupported(format!(
                    "TransferFields of a Text too long for the Code field {} of {} (not pinned)",
                    df.name, t.name
                ));
            }
            if !same_kind(&from.fields[sp].ty, &df.ty) && !text_pair {
                // Across apps BC skips a field of another kind, whatever SkipFieldsWithoutCheck says: the target keeps its value
                // (Y38TransferXApp.al:449-499 X1-X5: Boolean, Integer into Decimal, Option into Integer, Date into DateTime,
                // Decimal into Boolean; RcBTransfer.al:553 Transfer_From_Another_Apps_Table_Type_Mismatch_BQ: Text into Integer).
                if cross_app && skipped_across_apps(&from.fields[sp].ty, &df.ty) {
                    continue;
                }
                if skip_mismatch {
                    continue;
                }
                if let Some(e) = self.transfer_type_mismatch(rec, t, &from, (dp, sp), &updates) {
                    return Err(e);
                }
                return unsupported(format!(
                    "TransferFields across fields of different types (error text not pinned): field {} of {} and field {} of {}",
                    from.fields[sp].name, from.name, df.name, t.name
                ));
            }
            // A column a platform row leaves unknown moves as it is into a field of the very same type: the copy reads
            // nothing (Tests-ERM `AddFieldToBuffer`: `TempField.TransferFields(Field, true)`), and a later read of the
            // copy is refused. Into another type the conversion could fail or not, which is not known: `convert` refuses.
            if src_fields[sp].unknown().is_some() && from.fields[sp].ty == df.ty {
                updates.push((dp, src_fields[sp].clone()));
                continue;
            }
            // A Code value too long for the Text field: across apps (both flags) and with SkipFieldsWithoutCheck the field is
            // skipped and keeps its value; within one app without it the size error below (Y38TransferXApp.al:504-512
            // XA_TF_LongCode, :569 and :594 p41_long). A Text one across apps with SkipFieldsWithoutCheck is skipped too
            // (RcBTransfer.al:553 `longAddressSkip`); across apps without it is not pinned.
            if let (
                RtType::Text(Some(_)) | RtType::Code(Some(_)),
                RtType::Text(Some(to_len)),
                Value::Text(s) | Value::Code(s),
            ) = (&from.fields[sp].ty, &df.ty, &src_fields[sp])
                && Value::text_len(s) > *to_len as usize
            {
                let from_code = matches!(from.fields[sp].ty, RtType::Code(_));
                if (from_code && (cross_app || skip_mismatch)) || (cross_app && skip_mismatch) {
                    continue;
                }
                if cross_app {
                    return unsupported(format!(
                        "TransferFields across apps of a text too long for field {} of {} (not pinned)",
                        df.name, t.name
                    ));
                }
                // A --tdd stub field of unknown length refuses first.
                tdd_fits(&schema.fields[dp], &src_fields[sp])?;
                let from_schema = engine.program.objects[from.object as usize]
                    .table
                    .as_ref()
                    .expect("table");
                if let Some(why) = &from_schema.fields[sp].tdd_length_unknown {
                    return Err(Signal::unsupported(why.clone()));
                }
                return Err(self.transfer_too_small(rec, t, &from, (dp, sp), updates));
            }
            tdd_fits(&schema.fields[dp], &src_fields[sp])?;
            // A Blob moves as `blob_transfer::moved` says.
            if let Value::Blob(_) = &src_fields[sp] {
                let cur = rec.borrow().fields[dp].clone();
                let how = BlobMove::TransferFields(crate::blob_transfer::TransferCall::of(args));
                let kind = || {
                    self.blob_field_kind(src_table, sp)
                        .with(self.blob_field_kind(t.object as usize, dp))
                };
                let v = crate::blob_transfer::moved(Some(&cur), src_fields[sp].clone(), how, kind)?;
                updates.push((dp, ops::convert(v, &df.ty)?));
                continue;
            }
            updates.push((dp, ops::convert(src_fields[sp].clone(), &df.ty)?));
        }
        // From a row of the same table with the primary key (the default), the buffer takes the source's row version: a Modify of a
        // stale buffer then succeeds; without the key it does not (record/crud/CnTransferFieldsModify.al:133-150,
        // :153-173). From another table the version stays (only a current buffer is pinned there, :194-214).
        if init_pk && src_table == t.object as usize {
            if let Some(i) = t.fields.iter().position(|f| f.id == 2_000_000_005) {
                updates.push((i, src_fields[i].clone()));
            }
        }
        // From another table the buffer keeps its own SystemId and SystemRowVersion, so a stale buffer stays stale and a current
        // one modifies fine, and it takes the source's four audit fields as they are (record/crud/
        // CnTransferFieldsModify.al:11-16, :250-479 QE_TF_Other_*; :501 QH_TF_Other_ByFields_SourceFromDb for SystemCreatedBy and
        // SystemModifiedBy): a source built in memory, or a temporary row after Insert, holds blank At and null By fields and the
        // buffer gets those (:543 QH_TF_Other_SourceInMemory_AuditFields, :612 _SourceTemporary_, :717
        // _TargetInMemory_SourceInMemory_); a temporary row filled from a database row keeps that row's stamps (:670).
        if src_table != t.object as usize {
            for id in 2_000_000_001..=2_000_000_004 {
                let Some(i) = t.fields.iter().position(|f| f.id == id) else {
                    continue;
                };
                let v = match from.fields.iter().position(|f| f.id == id) {
                    Some(sp) => src_fields[sp].clone(),
                    None => Value::Unknown(std::sync::Arc::from(
                        "an audit field after a TransferFields from a table without it (not pinned)",
                    )),
                };
                updates.push((i, v));
            }
        }
        let mut r = rec.borrow_mut();
        for (dp, v) in updates {
            r.fields[dp] = v;
        }
        // The row version came over from the source (above), which a Commit may have left behind. Where it came from decides the
        // stale write (record/crud/Y76StaleBuffers.al Y76_Part1_Copy_TransferFromRecord, _GetThenTransferFields,
        // _TransferFromTemp): from a temporary copy never written; from a record read by Get or Find, written only after a LockTable
        // and Get of the row (a buffer that read the row by Get itself keeps its Get rule).
        if init_pk
            && src_table == t.object as usize
            && !r.temporary
            && let Some(i) = t.fields.iter().position(|f| f.id == 2_000_000_005)
            && let Value::BigInteger(v) = src_fields[i]
            && v != 0
        {
            let (src_temporary, src_origin) = {
                let s = src.borrow();
                (s.temporary, s.row_origin)
            };
            let from_insert = src_origin.is_some_and(|o| o.own_write && !o.read_in_write_tx);
            let origin = |own_write, lock_only| al_builtins::value::RowOrigin {
                lock_only,
                ..al_builtins::value::RowOrigin::new(own_write, v, self.commits)
            };
            if src_temporary {
                r.row_origin = Some(origin(false, false));
            } else if src_origin.is_some_and(|o| o.version == v)
                && !from_insert
                && r.row_origin.is_none_or(|o| o.version != v)
            {
                r.row_origin = Some(origin(v > self.tx.committed_row_version, true));
            }
        }
        Ok(Value::Null)
    }

    /// Two fields of one number and different types, between two tables of the same app: DB:TypeMisMatch naming the fields, the
    /// tables and the types, whatever the value, and the fields copied before the failing one stay copied
    /// (record/crud/RcBTransfer.al:265-357, Transfer_TypeDifferences_Narrowing, _DecimalFraction, _Widening,
    /// Transfer_DifferentPrimaryKey). Pinned for Decimal into Integer or BigInteger, Integer into Code, and Boolean and Text either way, between tables whose
    /// captions are their names; anything else gives `None` and stays refused. Between tables of different apps BC skips the field
    /// instead (Transfer_From_Another_Apps_Table_Type_Mismatch_BQ; `transfer_fields`).
    /// The app that declares field `pos` of `t`: the app of the table or table extension that declares it, or the table's own
    /// app for a field without one (a system field).
    fn field_app(&self, t: &TableDef, pos: usize) -> u32 {
        let objects = &self.engine.program.objects;
        let declared = objects[t.object as usize]
            .table
            .as_ref()
            .and_then(|schema| schema.fields.get(pos))
            .and_then(|f| f.declared_in)
            .map_or(t.object as usize, |o| o as usize);
        objects[declared].app
    }

    fn transfer_type_mismatch(
        &mut self,
        rec: &Rec,
        t: &TableDef,
        from: &TableDef,
        (dp, sp): (usize, usize),
        updates: &[(usize, Value)],
    ) -> Option<Signal> {
        let objects = &self.engine.program.objects;
        if objects[t.object as usize].app != objects[from.object as usize].app {
            return None;
        }
        let name = |ty: &RtType| match ty {
            RtType::Integer => Some("Integer"),
            RtType::Decimal => Some("Decimal"),
            RtType::BigInteger => Some("BigInteger"),
            RtType::Code(_) => Some("Code"),
            RtType::Text(_) => Some("Text"),
            RtType::Boolean => Some("Boolean"),
            RtType::Option(_) => Some("Option"),
            RtType::Date => Some("Date"),
            RtType::DateTime => Some("DateTime"),
            _ => None,
        };
        let (to_ty, from_ty) = (name(&t.fields[dp].ty)?, name(&from.fields[sp].ty)?);
        // Text and Boolean either way: record/crud/PnF07TransferEnumOption.al:185-206 Transfer_BooleanAndText.
        // Boolean or Integer into Decimal, Option into Integer, Date into DateTime, Decimal into Boolean:
        // record/transferfields-cross-app/Y38TransferXApp.al:517-523 S1, :539-570 p33, p40, p42, p44, p45.
        if !matches!(
            (to_ty, from_ty),
            ("Integer" | "BigInteger", "Decimal")
                | ("Code", "Integer")
                | ("Text", "Boolean")
                | ("Boolean", "Text")
                | ("Decimal", "Boolean" | "Integer")
                | ("Integer", "Option")
                | ("DateTime", "Date")
                | ("Boolean", "Decimal")
                // Two extension fields of one app: TransferFieldsFieldApps.al:204-205 (corpus master 3eb812ed).
                | ("Code", "Boolean")
        ) {
            return None;
        }
        let caption_is_name = |o: usize| {
            let obj = &objects[o];
            obj.properties
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("Caption"))
                .is_none_or(|(_, v)| v.trim().trim_matches('\'') == obj.name)
        };
        if self.field_caption(t, dp) != t.fields[dp].name
            || self.field_caption(from, sp) != from.fields[sp].name
            || !caption_is_name(t.object as usize)
            || !caption_is_name(from.object as usize)
        {
            return None;
        }
        let mut r = rec.borrow_mut();
        for (p, v) in updates {
            r.fields[*p] = v.clone();
        }
        let (to, src) = (&t.fields[dp].name, &from.fields[sp].name);
        Some(crate::transactions::al_error_code(
            &format!(
                "The following fields must have the same type:\\\\Field: {to} <-- {src}\\Table: {} <-- {}\\Type: {to_ty} <-- {from_ty}",
                t.name, from.name
            ),
            "DB:TypeMisMatch",
        ))
    }

    /// A Text or Code value longer than the target Text field: DB:TypeMisMatch with the size text, the target field keeps its value,
    /// and the fields copied before it stay copied (record/crud/P130TransferTypes.al:424-480
    /// RN_Transfer_Text100ToText10_ByValueLength, al-runner issue 5773; :366 for the text; record/transferfields-cross-app/
    /// Y38TransferXApp.al:569 p41_long, where CustNo stays copied). The text names fields and tables by name; a field or table
    /// whose caption differs is not pinned.
    fn transfer_too_small(
        &mut self,
        rec: &Rec,
        t: &TableDef,
        from: &TableDef,
        (dp, sp): (usize, usize),
        updates: Vec<(usize, Value)>,
    ) -> Signal {
        let caption_is_name = |o: usize| {
            let obj = &self.engine.program.objects[o];
            obj.properties
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("Caption"))
                .is_none_or(|(_, v)| v.trim().trim_matches('\'') == obj.name)
        };
        if self.field_caption(t, dp) != t.fields[dp].name
            || self.field_caption(from, sp) != from.fields[sp].name
            || !caption_is_name(t.object as usize)
            || !caption_is_name(from.object as usize)
        {
            return Signal::Unsupported(
                "the TransferFields size error of a field or table whose caption is not its name (not pinned)".into(),
            );
        }
        let mut r = rec.borrow_mut();
        for (p, v) in updates {
            r.fields[p] = v;
        }
        let (to, src) = (&t.fields[dp].name, &from.fields[sp].name);
        // `Type: Text5 <-- Code10` for a Code source (Y38TransferXApp.al:569 p41_long).
        let kind = |ty: &RtType| match ty {
            RtType::Text(Some(n)) => format!("Text{n}"),
            RtType::Code(Some(n)) => format!("Code{n}"),
            _ => "Text0".into(),
        };
        let (from_len, to_len) = (kind(&from.fields[sp].ty), kind(&t.fields[dp].ty));
        crate::transactions::al_error_code(
            &format!(
                "The contents of the {src} field cannot be copied to the {to} field because the size of the {to} field is too small.\\\\Type: {to_len} <-- {from_len}\\Field: {to} <-- {src}\\Table: {} <-- {}",
                t.name, from.name
            ),
            "DB:TypeMisMatch",
        )
    }

    /// `GetBySystemId(Guid)`: loads the row holding that SystemId, whatever the filters; a miss is
    /// `false` and leaves the buffer. A miss with the result unused raises, for a Record
    /// (record/PnD04_GetBySystemId.al:36-47, :111-124 the null Guid) and a RecordRef
    /// (types/PXEnumRecordRef.al:199-301).
    pub(crate) fn get_by_system_id(&mut self, rec: &Rec, t: &TableDef, args: &[Value]) -> Result<Value, Signal> {
        let Some(i) = t.fields.iter().position(|f| f.id == 2_000_000_000) else {
            return unsupported("GetBySystemId on a table without SystemId");
        };
        // A Text argument binds to the Guid parameter as a Text assigns to a Guid
        // (PXEnumRecordRef.al:232 passes the null Guid as a text literal).
        let parsed = match args.first() {
            Some(Value::Guid(g)) => Some(*g),
            Some(Value::Text(s) | Value::Code(s)) => ops::parse_guid(s),
            _ => None,
        };
        let Some(g) = &parsed else {
            return match args.first() {
                // GetWithTextArgument.al:GetBySystemId_TextAndGuid, `empty_text`, `text_not_a_guid`, `integer`.
                Some(Value::Text(_) | Value::Code(_)) => Err(crate::transactions::al_error_code(
                    "Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits.",
                    "InvalidGuidFormat",
                )),
                Some(Value::Integer(_)) => Err(crate::transactions::al_error_code(
                    "The expression Variant cannot be type-converted to a GUID value.",
                    "TypeConversion",
                )),
                other => unsupported(format!("GetBySystemId of {}", other.describe())),
            };
        };
        // On BC 27 every row of the platform table Field has the null Guid as its SystemId, and GetBySystemId of the null Guid finds
        // the first row, table 3 field 1 (metadata/platform-tables/FieldRowSystemId.al:146-172, the `#else` branch).
        let null_field_rows = self.engine.program.objects[t.object as usize].id == Some(2_000_000_041)
            && !self.has(al_version::Gate::FieldRowSystemIdFrom28);
        let found = if *g == [0; 16] && !null_field_rows {
            None
        } else {
            let filters = [al_builtins::filter::FieldFilter {
                lang: al_builtins::filter::filter_language(),
                field: i,
                expr: al_builtins::filter::FilterExpr::Eq(Value::Guid(*g)),
                group: 0,
            }];
            let cx = self.cx.clone();
            self.store_for(rec)
                .next(
                    &cx,
                    t,
                    &al_data::Scan {
                        key: 0,
                        filters: &filters,
                        ascending: true,
                        after: None,
                    },
                )
                .map_err(|e| Signal::unsupported(format!("{e}")))?
        };
        match found {
            Some(row) => {
                // Through `load`: SetAutoCalcFields fields are calculated when a row is read.
                self.sysid_load = true;
                let loaded = self.load(rec, row);
                self.sysid_load = false;
                loaded?;
                Ok(Value::Boolean(true))
            }
            // As for Get (records.rs `fail`): a miss on a table whose seeded rows could not be
            // created is not BC's answer (ADR-013 decision 5).
            None if !rec.borrow().temporary && self.install_unknown(t.object).is_some() => unsupported(format!(
                "{} row not found by GetBySystemId: {}",
                t.name,
                self.install_unknown(t.object).unwrap_or_default()
            )),
            None if self.is_unseeded(t) && !rec.borrow().temporary => unsupported(format!(
                "{} row not found by GetBySystemId: the rows a dependency app's install and Company-Initialize create could not be created (ADR-013 decision 5)",
                t.name
            )),
            None if self.ret_used => Ok(Value::Boolean(false)),
            // "The PX Row does not exist. Identification fields and values: {x}" / DB:RecordNotFound
            // (PXEnumRecordRef.al:264-267; the probe masks what stands between the braces, here
            // the Guid as Format writes it: docs/probes-needed.md).
            None => {
                let id = al_builtins::format::try_format(&Value::Guid(*g), 0, 0, self).map_err(Signal::unsupported)?;
                Err(Signal::Error(Box::new(AlError {
                    message: format!(
                        "The {} does not exist. Identification fields and values: {id}",
                        self.caption(t)
                    ),
                    code: "DB:RecordNotFound".into(),
                    origin: al_builtins::native::ErrorOrigin::Redacted(format!(
                        "The {} does not exist. Identification fields and values: <redacted>",
                        self.caption(t)
                    )),
                    ..Default::default()
                })))
            }
        }
    }

    /// `ModifyAll(Field, Value [, RunTrigger])`: every row passing the filters, as they stand when
    /// the call starts, gets the value through `Modify(RunTrigger)`; the buffer is left alone
    /// (corpus record/TestRecordModify.al:87-170, TestTriggerContracts.al:123-146).
    fn modify_all(&mut self, rec: &Rec, t: &TableDef, args: &[Value]) -> Result<Value, Signal> {
        let at = self.note_bulk_write(rec, t, crate::state::TxWriteKind::ModifyAll);
        let r = self.modify_all_noted(rec, t, args);
        self.note_bulk_end(at);
        r
    }

    /// `modify_all` inside its notes for the stale-buffer rule.
    fn modify_all_noted(&mut self, rec: &Rec, t: &TableDef, args: &[Value]) -> Result<Value, Signal> {
        let pos = arg_pos(args, 0)?;
        if t.fields[pos].id == 2_000_000_000 {
            // Wording after the pinned fragment: record/TestRecordSystemId.al:230-262, docs/probes-needed.md.
            // record/TestSystemFieldStampsContracts.al:153 (SystemId_Duplicate_And_ModifyAll_Texts).
            return Err(error(cannot_change_value_text("SystemId", &self.caption(t)), "Dialog"));
        }
        let value = ops::convert(args.get(1).cloned().unwrap_or(Value::Null), &t.fields[pos].ty)?;
        let run_trigger = match args.get(2) {
            Some(v) => ops::truthy(v)?,
            None => false,
        };
        let mut rows = Vec::new();
        let (key, mut after) = (rec.borrow().key, None);
        while let Some(row) = self.scan(rec, t, true, after.take())? {
            after = Some(al_data::position(t, key, &row));
            rows.push(row);
        }
        // A field of the primary key is refused (events/binding-senders/DbtTransactions.al:ModifyAll_PrimaryKeyField, two rows, names as
        // captions). Over no rows, and for a name that differs from its caption, the text is not pinned.
        if t.keys[0].contains(&pos) {
            let (field, table) = (t.fields[pos].name.clone(), t.name.clone());
            if rows.is_empty() || self.field_caption(t, pos) != field || self.caption(t) != table {
                return unsupported(
                    "ModifyAll on a primary key field with no rows or with a caption (error text not pinned)",
                );
            }
            return Err(error(
                format!(
                    "The MODIFYALL function cannot be used on the {field} field in the {table} table because it is part of the primary key."
                ),
                "DB:Ok",
            ));
        }
        // An AutoIncrement field outside the key cannot be changed (record/TestAutoIncrementEdges.al:569-580, two rows, names as
        // captions). Over no rows, and for a name that differs from its caption, the text is not pinned.
        if self.auto_increment_field(t)? == Some(pos) {
            let (field, table) = (t.fields[pos].name.clone(), t.name.clone());
            if rows.is_empty() || self.field_caption(t, pos) != field || self.caption(t) != table {
                return unsupported(
                    "ModifyAll on an AutoIncrement field with no rows or with a caption (error text not pinned)",
                );
            }
            return Err(error(cannot_change_value_text(&field, &table), "DB:Ok"));
        }
        // One fresh set of table globals for all rows, not the caller's (ADR-021 decision 6; corpus
        // probe TableGlobals_ModifyAll_WhatTheTriggersAndSubscribersSee).
        let shared = al_builtins::value::TableGlobals::new_shared();
        // Without triggers run, modify-event subscribers and database triggers, ModifyAll writes every row, also a row that has the
        // value already: its SystemModifiedAt changes (record/crud/RcBTriggers.al:682-722 ModifyAll_Equal_Value_Rows_BQ,
        // `row1StampChangedFalse`), while ModifyAll(.., true) leaves such a row as a Modify does (`row1StampChanged`). A declared
        // OnBeforeModifyEvent or OnAfterModifyEvent subscriber, bound or not, makes it write row by row: a row that has the value
        // keeps SystemModifiedAt and SystemRowVersion (record/crud/ModifyWithoutChange.al:318-365, a manual
        // subscriber not bound in QD_ModifyAll_SameValue_NoSubscriber, bound in the After and Before cases).
        let declared = |ev: &str| self.engine.events.is_subscribed(t.object as usize, ev);
        let bulk = !run_trigger
            && !rec.borrow().temporary
            && !declared("onbeforemodifyevent")
            && !declared("onaftermodifyevent")
            && !self.global_enabled(t, crate::globaltriggers::GlobalOp::Modify)?;
        let outer = std::mem::replace(&mut self.bulk_modify, bulk);
        let r = self.modify_all_rows(rec, rows, pos, &value, run_trigger, shared);
        self.bulk_modify = outer;
        r
    }

    fn modify_all_rows(
        &mut self,
        rec: &Rec,
        rows: Vec<Vec<Value>>,
        pos: usize,
        value: &Value,
        run_trigger: bool,
        shared: al_builtins::value::TableGlobals,
    ) -> Result<Value, Signal> {
        for mut row in rows {
            row[pos] = value.clone();
            let copy = {
                let r = rec.borrow();
                RecordState {
                    fields: row,
                    // The caller's filters and current key stay: the Rec and xRec of the modify events show them (corpus
                    // record/events/Y7408XRecInModifyEvent.al:310 `ModifyAll(true) under a filter`, al-runner issue 7408).
                    filters: r.filters.clone(),
                    marks: Vec::new(),
                    marked_only: false,
                    position: None,
                    last_modify: None,
                    failed_insert: None,
                    row_origin: None,
                    own_read: Default::default(),
                    globals: shared.share(),
                    // The scanned row is whole: a partial load of the variable is not the copy's (al-runner issue 6654).
                    load: Default::default(),
                    ..r.clone()
                }
            };
            let copy = Rc::new(RefCell::new(copy));
            self.record_op(RecordOp::Modify, &copy, &mut [Value::Boolean(run_trigger)])?;
        }
        Ok(Value::Null)
    }

    /// The `RecordId` of the row in `rec`'s buffer: the table and its primary key values.
    pub(crate) fn record_id(&self, rec: &Rec, t: &TableDef) -> Value {
        let mut key = t.primary_key(&rec.borrow().fields);
        // An Option key field with an OptionCaption shows its caption in the RecordId's text (format-evaluate/
        // OptionEvaluateCaptions.al:392 RecordId_OptionKeyWithCaption `format`: `LT Option Key Cap: Credit Memo cap,D-1`).
        for (k, &p) in key.iter_mut().zip(&t.keys[0]) {
            if let (Value::Option(o, _), Some(captions)) = (&*k, self.option_key_captions(t, p)) {
                *k = Value::Option(*o, captions);
            }
        }
        Value::Library(
            "RecordId".into(),
            Rc::new(RefCell::new(al_builtins::value::LibraryValue::RecordId {
                table: t.object,
                table_no: self.engine.program.objects[t.object as usize].id.unwrap_or(0),
                name: t.name.clone(),
                key,
            })),
        )
    }

    /// The primary key values read from the text after the colon of `Evaluate` into a RecordId: the values as Format writes them,
    /// split at the commas outside double quotes (a quoted value may hold `""` for a quote). Fewer values than the key has fields
    /// give a shorter key, more are `Ok(None)` (format-evaluate/EvaluateRecordIdComposite.al:Evaluate_WrongNumberOfValues
    /// `tooFew`, `tooFew3`, `tooMany`, `tooMany1`; RoundTrip_KeyValueWithComma). Each value is read as Evaluate reads its field's
    /// type; a value that is none raises that type's error (`badInteger`, `badEnum`, `badDate`, `codeTooLong`). A Code, Integer, Enum
    /// and Date key field is pinned (RoundTrip_CodeCode, _EnumCode, _IntCodeCode, _DateCode); another type of a composite key is
    /// refused, a one-field key of another type reads as Evaluate reads it (format-evaluate/P128EvalRecordId.al).
    pub(crate) fn evaluate_record_key(
        &mut self,
        t: &TableDef,
        k: &str,
        number: i64,
    ) -> Result<Option<Vec<Value>>, Signal> {
        let fields = &t.keys[0];
        // Each value with whether it was quoted.
        let mut parts: Vec<(String, bool)> = Vec::new();
        let mut rest = k;
        loop {
            if let Some(q) = rest.strip_prefix('"') {
                let mut value = String::new();
                let mut chars = q.char_indices();
                let tail = loop {
                    match chars.next() {
                        Some((_, '"')) => match chars.clone().next() {
                            Some((_, '"')) => {
                                chars.next();
                                value.push('"');
                            }
                            Some((i, _)) => break &q[i..],
                            None => break "",
                        },
                        Some((_, c)) => value.push(c),
                        None => {
                            return unsupported(
                                "Evaluate into a RecordId of a key text with a quote that is not closed (not pinned)",
                            );
                        }
                    }
                };
                parts.push((value, true));
                match tail.strip_prefix(',') {
                    Some(more) => rest = more,
                    None if tail.is_empty() => break,
                    None => {
                        return unsupported(
                            "Evaluate into a RecordId of a key text with characters after a closing quote (not pinned)",
                        );
                    }
                }
            } else {
                // An empty value that is not quoted is checked per field type below.
                match rest.split_once(',') {
                    Some((v, more)) => {
                        parts.push((v.to_string(), false));
                        rest = more;
                    }
                    None => {
                        parts.push((rest.to_string(), false));
                        break;
                    }
                }
            }
        }
        if parts.len() > fields.len() {
            return Ok(None);
        }
        if let Some(key) = crate::recordid_code_text::code_text_key(t, &parts) {
            return Ok(key);
        }
        let mut key = Vec::new();
        for ((v, quoted), &p) in parts.iter().zip(fields) {
            let ty = &t.fields[p].ty;
            // An Option key field of a composite key (errors/WcUnpinnedSingles.al:112-146, al-runner issue 6322).
            if let (RtType::Option(members), true) = (ty, fields.len() > 1) {
                key.push(self.evaluate_option_key_part(t, p, members, v)?);
                continue;
            }
            if v.is_empty() && !quoted {
                return unsupported(
                    "Evaluate into a RecordId of a key text with an empty value that is not quoted (not pinned)",
                );
            }
            match ty {
                RtType::Code(_) | RtType::Integer | RtType::Enum(..) | RtType::Date => {
                    // The error of a value that is none is raised whether the result is used or not; the caller decides.
                    // A Code value with blanks around it is read trimmed, and the length check counts the trimmed text
                    // (record/Y70Probes.al:661-713 EvaluateRecordId_BlanksAroundCodeKey).
                    let v = if matches!(ty, RtType::Code(_)) && v.trim() != v.as_str() {
                        v.trim()
                    } else {
                        v.as_str()
                    };
                    let used = std::mem::replace(&mut self.ret_used, false);
                    let r = al_builtins::evaluate::evaluate_key_part(self, ty, v, number);
                    self.ret_used = used;
                    key.push(r?);
                }
                // A one-field Text key value that is not quoted is read trimmed: `Key:  lead` and `Key: a b ` give `lead` and
                // `a b` (format-evaluate/Y14Small.al:205 `eval leading blank`, `eval trailing blank`, corpus master).
                RtType::Text(_) if fields.len() == 1 && !quoted => {
                    match al_builtins::evaluate::evaluate_typed(self, ty, v.trim(), number)? {
                        Some(x) => key.push(x),
                        None => return Ok(None),
                    }
                }
                _ if fields.len() == 1 => match al_builtins::evaluate::evaluate_typed(self, ty, v, number)? {
                    Some(x) => key.push(x),
                    None => return Ok(None),
                },
                _ => {
                    return unsupported(format!(
                        "Evaluate into a RecordId of table `{}` with a key field of type {} (not pinned)",
                        t.name,
                        ty.describe()
                    ));
                }
            }
        }
        Ok(Some(key))
    }

    /// The OptionCaption of the Option field at `p` when it differs from the members, as the member list a RecordId key value
    /// carries so that its text shows the captions. None for a field without one, and for one whose captions are not known here
    /// (OptionCaptionML).
    fn option_key_captions(&self, t: &TableDef, p: usize) -> Option<std::sync::Arc<[String]>> {
        let (_, list) = self.members_of_field(t, p).ok()??;
        list.iter()
            .any(|(_, name, caption)| name != caption)
            .then(|| list.into_iter().map(|(_, _, caption)| caption).collect())
    }

    /// The value of the Option key field at `p` of a composite key, read from `v` by `Evaluate` into a RecordId
    /// (errors/WcUnpinnedSingles.al:112-146, under languages 1033 and 1031 alike): a member name (`Invoice`, `Credit Memo`)
    /// or an ordinal (`1`; `9`, past the members, is kept and Format shows `9`) gives that member; an empty value that is not
    /// quoted gives ordinal 0 (Format shows `" "`); another text raises `'Quote' is not an option. The existing options are:
    /// <members>` with code InvalidOptionString, and Evaluate answers false.
    /// format-evaluate/OptionEvaluateCaptions.al pins more:
    /// - :392 RecordId_OptionKeyWithCaption: with an OptionCaption the name, the caption and the ordinal all give the member,
    ///   and the error lists the captions;
    /// - :414 RecordId_OptionKeyShapes: a name in another case and blanks around the value read, a quoted `""` or `" "` gives
    ///   ordinal 0, a quoted name gives that member, `2147483647` is kept, `2147483648` and `-1` raise the error.
    ///
    /// A caption in another case, and a text that is the caption of one member and the name of another, are not pinned.
    fn evaluate_option_key_part(
        &mut self,
        t: &TableDef,
        p: usize,
        members: &[String],
        v: &str,
    ) -> Result<Value, Signal> {
        let refuse = |what: &str| {
            unsupported(format!(
                "Evaluate into a RecordId of table `{}` with an Option key field: {what} (not pinned)",
                t.name
            ))
        };
        let captions: Vec<String> = match self.members_of_field(t, p)? {
            Some((_, list)) if list.len() == members.len() => list.into_iter().map(|(_, _, caption)| caption).collect(),
            Some(_) => return refuse("captions of another number than the members"),
            None => members.to_vec(),
        };
        let shown = self
            .option_key_captions(t, p)
            .unwrap_or_else(|| std::sync::Arc::from(members));
        let value = |o: i32| Ok(Value::Option(o, shown.clone()));
        let v = v.trim();
        if v.is_empty() {
            return value(0);
        }
        if v.bytes().all(|b| b.is_ascii_digit()) {
            if let Ok(n) = v.parse::<i32>() {
                return value(n);
            }
        }
        let by_name = members.iter().position(|m| m.eq_ignore_ascii_case(v));
        let by_caption = captions.iter().position(|c| c == v);
        match (by_name, by_caption) {
            (Some(a), Some(b)) if a != b => {
                return refuse("a text that is the caption of one member and the name of another");
            }
            (Some(n), _) | (None, Some(n)) => return value(n as i32),
            (None, None) => {}
        }
        if captions.iter().any(|c| c.eq_ignore_ascii_case(v)) {
            return refuse("a caption in another case");
        }
        al_builtins::native::raise(
            format!(
                "'{v}' is not an option. The existing options are: {}",
                captions.join(",")
            ),
            "InvalidOptionString",
        )
    }

    /// `FieldError(Field [, Text])`. With a text: `<caption> <text> in <table> <key>.`; without:
    /// `<caption> must not be <value> in <table> <key>.` The key is `Name='value'` pairs joined by
    /// commas, no colon after the table (corpus record/TestRecordRuntimeTextsAndViews.al:
    /// Record_FieldError_Texts). The code is `NCLCSRTS:TableErrorStr`.
    fn field_error(&mut self, rec: &Rec, t: &TableDef, args: &[Value]) -> Result<Value, Signal> {
        let form = self
            .field_error_form
            .take()
            .unwrap_or(al_builtins::native::ErrorTextForm::Other);
        let pos = arg_pos(args, 0)?;
        let row = rec.borrow().fields.clone();
        let mut parts = Vec::new();
        for &i in &t.keys[0] {
            parts.push(format!(
                "{}='{}'",
                self.field_caption(t, i),
                self.message_value(&row[i])?
            ));
        }
        let key = parts.join(",");
        // FieldError(Field, ErrorInfo) raises the same text with the ErrorInfo's message, code NCLCSRTS:TableErrorStr,
        // and leaves the ErrorInfo as it was; a collectible one is collected in a collecting scope as a copy
        // that holds the full text (recordref-fieldref/PWRefusedBuiltins.al:113-168, BC 27.5).
        let mut info = None;
        // No text, an empty text and an ErrorInfo with an empty message all say what is wrong with the value: a blank Text, Code
        // (errors/last-error/FieldErrorTextVariable.al:122), Date or DateTime "You must specify <field>", any other value "<field> must not be <value>" (record/testfield/
        // P96FieldError.al:FieldError_NoText_Empty_Row; P96LongText.al:LongText_FieldError_Message `fe_empty`).
        let mut what = match args.get(1) {
            Some(v) => match (al_builtins::native::text_of(v), al_builtins::errorinfo::message_of(v)) {
                (Some(text), _) => Some(text.into_string()),
                (None, Some(message)) => {
                    info = Some(v.clone());
                    Some(message)
                }
                _ => return unsupported("FieldError with an argument that is neither text nor an ErrorInfo"),
            },
            None => None,
        }
        .filter(|w| !w.is_empty());
        let given_text = what.is_some();
        let info_given = info.is_some();
        let blank_value = is_blank(&row[pos])?
            && matches!(
                row[pos],
                Value::Text(_) | Value::Code(_) | Value::Date(..) | Value::DateTime(_)
            );
        let full = match what.take() {
            Some(w) => format!("{} {w} in {} {key}.", self.field_caption(t, pos), self.caption(t)),
            None if blank_value => format!(
                "You must specify {} in {} {key}.",
                self.field_caption(t, pos),
                self.caption(t)
            ),
            None => format!(
                "{} must not be {} in {} {key}.",
                self.field_caption(t, pos),
                self.message_value(&row[pos])?,
                self.caption(t)
            ),
        };
        if let Some(info) = info {
            if let Some(copy) = al_builtins::errorinfo::copy_of(&info) {
                let collectible = al_builtins::errorinfo::set_message_of(&copy, &full);
                if collectible && self.collect_error(&copy) {
                    return Ok(Value::Null);
                }
            }
        }
        // The flag: P86LastError.al:270, :276, :282 (no text); "You must specify" of a blank Text, Code or Date field
        // (errors/last-error/FieldErrorTextVariable.al:106, :112, :121); a text written as a literal,
        // a local Text or Code variable or a concatenation (P86LastError.al:273, :279, FieldErrorTextVariable.al:65-82,
        // :142-150); a global Label or TextConst (lt-last-error-text-exclude/LastErrorTextExcludeLabel.al:246,
        // FieldErrorTextConst.al:51, :54). A blank DateTime, a text of another form and an ErrorInfo are not pinned.
        use al_builtins::native::ErrorTextForm;
        let field = format!("{}.{}", self.caption(t), self.field_caption(t, pos));
        let redacted = match (given_text, blank_value, info_given) {
            (false, false, false) => Some(format!("FieldError for incorrect value of {field}")),
            (false, true, false) if !matches!(row[pos], Value::DateTime(_)) => {
                Some(format!("FieldError for unspecified value of {field}"))
            }
            (true, _, false) => match (form, args.get(1).and_then(al_builtins::native::text_of)) {
                (ErrorTextForm::Literal | ErrorTextForm::Computed, _) => Some(format!(
                    "FieldError on field {field}, message: Use FIELDERROR with a text constant to improve Telemetry details."
                )),
                (ErrorTextForm::Constant, Some(text)) if !text.contains("\r\n") => {
                    Some(format!("FieldError on field {field}, message: {text}"))
                }
                _ => None,
            },
            _ => None,
        };
        match redacted {
            Some(r) => Err(error_redacted(full, "NCLCSRTS:TableErrorStr", r)),
            None => Err(error(full, "NCLCSRTS:TableErrorStr")),
        }
    }
}

/// Member indexes of the IsolationLevel option (`al_builtins::sysenum`, "isolationlevel").
pub(crate) const ISOLATION_REPEATABLE_READ: i32 = 3;
pub(crate) const ISOLATION_UPDLOCK: i32 = 4;

/// The error of `LockTable` with Wait = false (record/crud/RcBMeta.al:418-419).
pub(crate) fn lock_table_bad_parm(t: &TableDef) -> Signal {
    error(
        format!(
            "The request to lock the {} table with a Wait parameter of FALSE is not supported.",
            t.name
        ),
        "AL:LockTableBadParm",
    )
}

/// What a table field's caption read gives for field `pos` (see `Session::field_caption_written`).
pub(crate) enum WrittenCaption {
    /// The caption, as written.
    Text(String),
    /// A literal CaptionClass the platform translates (`Session::caption_class_translate`).
    Class(String),
    /// The CaptionClass expression, which runs with the record.
    Expr(String),
}

/// What a CaptionClass expression's result reads as (QdCaptionClassAreas.al:7-21).
pub(crate) enum ExprCaption {
    /// An empty result and `3,`: the field's Caption on a table read (Expressions_FieldCaption `exprEmpty`, `expr3`), the empty
    /// text on a page control (Page_ControlCaptions `exprEmpty`, `expr3`).
    FieldCaption,
    /// `3,<text>`: <text> (pages/extensions/TableExtCaptionClass.al:124-149).
    Text(String),
    /// A numeric area other than 3, or an empty one: the platform translates it as a literal ('2,0,x', '5,3,', '9,x', '2,x',
    /// '101,0,..', '1,1,1', QdCaptionClassAreas.al:262-296).
    Translate,
    /// A text with no comma, or with an area that is not a number: the caption is the text as it is
    /// (`Session::expression_plain_caption`).
    Plain,
}

/// The reading of a CaptionClass expression's result `class`.
pub(crate) fn expression_class(class: &str) -> ExprCaption {
    match class.split_once(',') {
        _ if class.is_empty() => ExprCaption::FieldCaption,
        Some(("3", "")) => ExprCaption::FieldCaption,
        Some(("3", rest)) => ExprCaption::Text(rest.to_string()),
        Some((area, _)) if area.is_empty() || area.trim().parse::<u32>().is_ok() => ExprCaption::Translate,
        _ => ExprCaption::Plain,
    }
}

/// `--tdd` (ADR-051 decision 10): a text longer than the declared length of stub field `f` is refused, as the real
/// length is not known yet (`al_ir::Field::tdd_length_unknown`); the made-up overflow error would be a guess.
pub(crate) fn tdd_fits(f: &al_ir::Field, v: &Value) -> Result<(), Signal> {
    let Some(why) = &f.tdd_length_unknown else {
        return Ok(());
    };
    let (RtType::Text(Some(max)) | RtType::Code(Some(max))) = f.ty else {
        return Ok(());
    };
    match v {
        Value::Text(s) | Value::Code(s) if Value::text_len(s) > max as usize => Err(Signal::unsupported(why.clone())),
        _ => Ok(()),
    }
}
