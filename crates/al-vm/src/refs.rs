//! `RecordRef`, `FieldRef` and `KeyRef` on the VM.
//!
//! An open `RecordRef` owns a [`RecordState`] exactly as a `Record` variable
//! does, so every data operation is the same code path as the record
//! methods in `records.rs` (triggers included). `FieldRef` and `KeyRef` point
//! into that buffer by field position and key index. Behaviour the corpus
//! does not pin raises `Signal::Unsupported` instead of guessing.

use crate::Session;
use al_builtins::catalog::Catalog;
use al_builtins::describe::Describe;
use al_builtins::native::{AlError, Signal};
use al_builtins::ops;
use al_builtins::record_op::RecordOp;
use al_builtins::value::{LibraryValue, RecordState, RtType, Value};
use al_data::{Store, TableDef};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::OnceLock;

type Rec = Rc<RefCell<RecordState>>;
type Lib = Rc<RefCell<LibraryValue>>;
/// Whether a field is an Enum (not an Option) and its members as (ordinal, name, caption).
type FieldMembers = (bool, Vec<(i32, String, String)>);

/// First number of the platform's system fields (`SystemId`, ...); they are
/// not counted by `FieldCount` (docs/probes-needed.md).
const SYSTEM_FIELD_BASE: u32 = 2_000_000_000;

/// The field number AL code sees for a field. SystemRowVersion is field 0 there: the corpus pins that
/// `KeyRef.FieldIndex(n).Number()` equals `FieldNo(SystemRowVersion)`
/// (recordref/TestKeyMetadataRowVersionRemoved.al:51, :85) and that the rowversion "sits at field id 0"
/// (record/TestCalcFormulaSystemFieldsTests.Codeunit.al:213-215); a real project reads
/// `KeyRef.FieldIndex(1).Number = 0` for a key that starts with it ("FieldRef 0 is rowversion").
/// No corpus test names the number itself: docs/probes-needed.md, batch 7. The other system fields
/// keep 2000000000 to 2000000004.
pub(crate) fn al_field_no(id: u32) -> i32 {
    if id == SYSTEM_ROW_VERSION_ID { 0 } else { id as i32 }
}

/// The internal id of SystemRowVersion, which AL code sees as field 0.
pub(crate) const SYSTEM_ROW_VERSION_ID: u32 = 2_000_000_005;

fn unsupported<T>(what: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(what.into()))
}

fn catalog() -> &'static Catalog {
    static C: OnceLock<Catalog> = OnceLock::new();
    C.get_or_init(Catalog::load)
}

fn handle(name: &str, v: LibraryValue) -> Value {
    Value::Library(name.into(), Rc::new(RefCell::new(v)))
}

/// A `RecordRef` handle on a copy of the record `rec` (what an event hands its subscriber).
pub(crate) fn record_ref_value(rec: &crate::records::Rec) -> Value {
    let copy = rec.borrow().clone();
    handle("RecordRef", LibraryValue::RecordRef(Rc::new(RefCell::new(copy))))
}

/// A `RecordRef` handle on the record `rec` itself: a change the subscriber makes reaches the row being written and the caller's
/// variable (events/lt-platform-events-v/PeVTests.al:170 GlobalTriggers_SubscriberChangesTheRecordItIsHanded_RowI: `dbmut` and `dbmod`).
pub(crate) fn record_ref_shared(rec: &crate::records::Rec) -> Value {
    handle("RecordRef", LibraryValue::RecordRef(rec.clone()))
}

fn al_error(message: String) -> Signal {
    Signal::Error(Box::new(AlError {
        message,
        code: "Dialog".into(),
        call_stack: String::new(),
        ..Default::default()
    }))
}

fn int_arg(args: &[Value], i: usize) -> Result<i64, Signal> {
    match args.get(i) {
        Some(Value::Integer(n)) => Ok(i64::from(*n)),
        Some(Value::BigInteger(n)) => Ok(*n),
        other => unsupported(format!("integer argument {i}: {}", other.describe())),
    }
}

/// The fields of key `k` of a RecordRef: a declared key, or the implicit key on `SystemId` that
/// follows them (corpus recordref/TestRecordRefRuntimeContracts.al:KeyRefAndFieldRef_Active_AreTrue_KeyCount
/// pins two keys for a table that declares only its primary key; tableextension/
/// TestTableExtKeysAndInitValue.al names it the implicit SystemId key).
fn key_fields(t: &TableDef, k: usize) -> Option<Vec<usize>> {
    if let Some(f) = t.keys.get(k) {
        return Some(f.clone());
    }
    if k == t.keys.len() {
        return t.fields.iter().position(|f| f.id == SYSTEM_FIELD_BASE).map(|p| vec![p]);
    }
    None
}

/// The table as the program declares it: without the ad hoc key a record may have appended.
impl Session {
    fn base_table(&self, t: &TableDef) -> std::sync::Arc<TableDef> {
        self.engine.tables[t.object as usize]
            .clone()
            .expect("a record's table has a schema")
    }
}

fn key_count(t: &TableDef) -> usize {
    t.keys.len() + usize::from(key_fields(t, t.keys.len()).is_some())
}

/// A table's fields a caller can enumerate (`FieldCount`, `FieldIndex`): in
/// field-number order, system fields left out.
fn enumerable_fields(s: &Session, t: &TableDef) -> Vec<usize> {
    let mut v: Vec<usize> = (0..t.fields.len())
        .filter(|&i| t.fields[i].id < SYSTEM_FIELD_BASE && s.field_enabled(t, i) && !s.field_removed(t, i))
        .collect();
    v.sort_by_key(|&i| t.fields[i].id);
    // A platform table lists its primary key fields first: Active Session is `2, 3, 1, 4, ...` and Time Zone
    // `3, 1, 2` (session/W6SessionTimeZone.al:ColumnNames_*, BC 27.5 and 29.0). The order of a table of an app
    // is by number, as before.
    if t.fields.first().is_some_and(|_| {
        s.engine.program.objects[t.object as usize]
            .id
            .is_some_and(|id| id >= SYSTEM_FIELD_BASE)
    }) {
        let key = &t.keys[0];
        v.sort_by_key(|&i| (key.iter().position(|&k| k == i).unwrap_or(usize::MAX), t.fields[i].id));
        return v;
    }
    // The FlowFields come after the other fields: Custom Report Layout lists 1, 2, 4, ..., 15 and then its FlowField 3
    // (metadata/lt-custom-report-layout-columns/CustomReportLayoutColumns.al:33-44). The FlowFilters come between the normal
    // fields and the FlowFields (the same file, PW_FieldIndex_FlowFilterAmongNormalFields: `1, 3, 5, 7, 20`, then `2, 6`, then `4, 8`).
    let class = |i: usize| match s.field_property(t, i, "FieldClass") {
        Some(c) if c.trim().eq_ignore_ascii_case("FlowField") => 2,
        Some(c) if c.trim().eq_ignore_ascii_case("FlowFilter") => 1,
        _ => 0,
    };
    if v.iter().any(|&i| class(i) > 0) {
        v.sort_by_key(|&i| (class(i), t.fields[i].id));
    }
    v
}

impl Session {
    /// A field without `Enabled = false` (record/TestRowVersionFieldNumberFieldActiveContracts.al:
    /// a disabled field is not counted, not listed, not `FieldExist` and not `FieldActive`).
    /// A field with `ObsoleteState = Removed`: RecordRef does not count it, list it or find it
    /// (recordref/PWRecordMisc.al:109-167, BC 27.5).
    pub(crate) fn field_removed(&self, t: &TableDef, pos: usize) -> bool {
        self.field_property(t, pos, "ObsoleteState")
            .is_some_and(|v| v.trim().eq_ignore_ascii_case("Removed"))
    }

    pub(crate) fn field_enabled(&self, t: &TableDef, pos: usize) -> bool {
        self.field_property(t, pos, "Enabled")
            .is_none_or(|v| !v.trim().eq_ignore_ascii_case("false"))
    }

    pub(crate) fn ref_op(&mut self, id: &str, recv: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
        let id = if id == "RecordRef.GetTableVariant#1" {
            self.get_table_from_variant = true;
            "RecordRef.GetTable#1"
        } else {
            id
        };
        let Some(sig) = catalog().by_id(id) else {
            return unsupported(format!("builtin {id}"));
        };
        let lib: Lib = match recv {
            Some(Value::Library(_, l)) => l.clone(),
            other => return unsupported(format!("{id} on {}", other.describe())),
        };
        match sig.receiver.as_str() {
            "RecordRef" => self.recordref_op(&sig.name, &lib, args),
            "FieldRef" => self.fieldref_op(&sig.name, &lib, args),
            "KeyRef" => self.keyref_op(&sig.name, &lib, args),
            // GetRecord answers a new RecordRef on the RecordId's table with the primary key set and
            // nothing else loaded; the row need not exist; a null RecordId raises (recordref/
            // TestRecordIdGetRecordContracts.al).
            "RecordId" if sig.name == "GetRecord" => {
                let (table, key) = match &*lib.borrow() {
                    LibraryValue::RecordId { table, key, .. } => (*table, key.clone()),
                    _ => {
                        return Err(Signal::Error(Box::new(al_builtins::native::AlError {
                            message: "The record is not open.".into(),
                            code: "RecordNotOpened".into(),
                            call_stack: String::new(),
                            ..Default::default()
                        })));
                    }
                };
                let Value::Record(rec) = self.default_value(&RtType::Record(table, false)) else {
                    return unsupported("record buffer for RecordId.GetRecord");
                };
                let t = self.table(&rec)?;
                for (&p, v) in t.keys[0].iter().zip(key) {
                    rec.borrow_mut().fields[p] = v;
                }
                Ok(handle("RecordRef", LibraryValue::RecordRef(rec)))
            }
            "RecordId" => match (&*lib.borrow(), sig.name.as_str()) {
                (LibraryValue::RecordId { table_no, .. }, "TableNo") => Ok(Value::Integer(*table_no as i32)),
                (LibraryValue::Empty, "TableNo") => Ok(Value::Integer(0)),
                (_, other) => unsupported(format!("RecordId.{other}")),
            },
            other => unsupported(format!("{other} methods")),
        }
    }

    /// `Evaluate(RecordId, text)`: see `al_builtins::evaluate`. A one-field key is pinned; a composite key is an assumption.
    pub(crate) fn evaluate_record_id_text(&mut self, text: &str, number: i64) -> Result<Option<LibraryValue>, Signal> {
        let (name, key_text) = match text.split_once(':') {
            Some((n, k)) => (n.trim(), Some(k)),
            None => (text.trim(), None),
        };
        // `[<namespace>.<name>]` is what Format(RecordId, 0, 9) writes for a table in a namespace, and Evaluate reads it back
        // (misc/lt-inferred-behaviors/P69NsConsumers.al:198, :215, a name with spaces); the same text without the brackets
        // names no table (:202, :216).
        let qualified = name.strip_prefix('[').and_then(|n| n.strip_suffix(']'));
        let tables: Vec<usize> = self
            .engine
            .program
            .objects
            .iter()
            .enumerate()
            .filter(|(_, o)| {
                o.table.is_some()
                    && match (qualified, &o.namespace) {
                        (None, _) => o.name.eq_ignore_ascii_case(name),
                        (Some(q), Some(ns)) => {
                            q.get(..ns.len()).is_some_and(|p| p.eq_ignore_ascii_case(ns))
                                && q[ns.len()..]
                                    .strip_prefix('.')
                                    .is_some_and(|n| n.eq_ignore_ascii_case(&o.name))
                        }
                        (Some(_), None) => false,
                    }
            })
            .map(|(i, _)| i)
            .collect();
        if let (Some(q), [one]) = (qualified, tables.as_slice()) {
            if !self.engine.program.objects[*one].name.contains(' ') {
                return unsupported(format!(
                    "Evaluate into a RecordId of `[{q}]`, a table in a namespace whose name has no space (not pinned)"
                ));
            }
        }
        if qualified.is_some() && tables.is_empty() {
            return unsupported(format!(
                "Evaluate into a RecordId of the bracketed name `{name}`, which names no table in a namespace (not pinned)"
            ));
        }
        let object = match tables.as_slice() {
            [] => return Ok(None),
            [one] => *one,
            _ => return unsupported("Evaluate into a RecordId of a table name that two tables share (not pinned)"),
        };
        let o = &self.engine.program.objects[object];
        if o.uncompiled {
            return unsupported("Evaluate into a RecordId of a table of an app that is not compiled");
        }
        let table_no = o.id.unwrap_or(0);
        let table_name = o.name.clone();
        let schema = o.table.as_ref().expect("filtered on a table");
        let key = match key_text {
            None => Vec::new(),
            Some(k) => {
                let k = k.strip_prefix(' ').unwrap_or(k);
                let Some(t) = self.engine.tables.get(object).cloned().flatten() else {
                    return unsupported("Evaluate into a RecordId of a table without a compiled schema");
                };
                // No values after the colon: a RecordId with no key (types/lt-evaluate-recordid/EvaluateRecordIdComposite.al:
                // Evaluate_WrongNumberOfValues `noValues`; pinned for a table with a two-field key only).
                if k.is_empty() {
                    if schema.keys[0].len() == 1 {
                        return unsupported(
                            "Evaluate into a RecordId with an empty key text of a one-field key (not pinned)",
                        );
                    }
                    Vec::new()
                } else {
                    match self.evaluate_record_key(&t, k, number)? {
                        Some(v) => v,
                        None => return Ok(None),
                    }
                }
            }
        };
        Ok(Some(LibraryValue::RecordId {
            table: object as u32,
            table_no,
            name: table_name,
            key,
        }))
    }

    fn open_ref(lib: &Lib, what: &str) -> Result<Rec, Signal> {
        match &*lib.borrow() {
            LibraryValue::RecordRef(r)
            | LibraryValue::FieldRef(r, _)
            | LibraryValue::KeyFieldRef(r, _)
            | LibraryValue::KeyRef(r, _) => Ok(r.clone()),
            LibraryValue::SystemTable(n) => unsupported(format!(
                "{what} on a RecordRef opened on system table {n} (the platform's system tables are not modelled)"
            )),
            // The methods below raise "The record is not open." on a RecordRef never opened or closed
            // (recordref/lt-refs/RfTOpen.al:Closed_Errors, lt-recordref-fieldref-members/P92Position.al:GetPosition_UnopenedAndNotFound,
            // P92Counts.al:CountApprox_Unopened, P92ReadIsolation.al:ReadIsolation_Unopened, P92Fields.al:FieldExist).
            LibraryValue::Empty
                if what.strip_prefix("RecordRef.").is_some_and(|m| {
                    matches!(
                        m,
                        "Name"
                            | "Caption"
                            | "Count"
                            | "IsEmpty"
                            | "IsTemporary"
                            | "Field"
                            | "Find"
                            | "Insert"
                            | "Modify"
                            | "Delete"
                            | "DeleteAll"
                            | "KeyCount"
                            | "CurrentKeyIndex"
                            | "Init"
                            | "GetView"
                            | "SetView"
                            | "Reset"
                            | "RecordId"
                            | "GetPosition"
                            | "SetPosition"
                            | "CountApprox"
                            | "ReadIsolation"
                            | "FieldExist"
                    )
                }) =>
            {
                Err(Signal::Error(Box::new(AlError {
                    message: "The record is not open.".into(),
                    code: "RecordNotOpened".into(),
                    call_stack: String::new(),
                    ..Default::default()
                })))
            }
            // BC's answer is not pinned by the corpus: the error text, and whether `Get(RecordId)` opens the
            // table of the RecordId (Base Application's Data Type Management does `if not Get(RecID) then
            // Open(RecID.TableNo)`). See docs/probes-needed.md.
            _ => unsupported(format!(
                "{what} on a RecordRef that is not open: BC's error text, and whether Get(RecordId) opens the table of the RecordId, are not pinned (docs/probes-needed.md)"
            )),
        }
    }

    /// The table object with number `no`. A table declared `ObsoleteState = Moved` is not in the runtime app
    /// (platform/lt-aw-fresh-tenant/AwTests.al:426-442), so it is never the answer.
    pub(crate) fn table_object(&self, no: i64) -> Option<usize> {
        let no = u32::try_from(no).ok()?;
        self.engine
            .program
            .objects
            .iter()
            .enumerate()
            .position(|(i, o)| o.table.is_some() && o.id == Some(no) && !self.engine.object_facts[i].moved)
    }

    /// The manifest `target` of the app whose code is running now.
    fn caller_target(&self) -> Option<&str> {
        let o = *self.frames.last()?;
        let app = self.engine.program.objects[o].app as usize;
        self.engine.program.apps.get(app)?.target.as_deref()
    }

    /// Whether the only tables with number `no` are declared `ObsoleteState = Moved`.
    fn only_moved_table(&self, no: i64) -> bool {
        u32::try_from(no).is_ok_and(|no| {
            self.engine
                .program
                .objects
                .iter()
                .enumerate()
                .any(|(i, o)| o.table.is_some() && o.id == Some(no) && self.engine.object_facts[i].moved)
        })
    }

    /// `RecordRef.Open(No, Temp, CompanyName)`.
    fn rr_open(&mut self, lib: &Lib, args: &[Value]) -> Result<Value, Signal> {
        // Open on a RecordRef that holds a table raises (types/lt-variant/VaTIsChecks.al:Is_Record_RecordRef_FieldRef `af137`,
        // after GetTable).
        if matches!(&*lib.borrow(), LibraryValue::RecordRef(_)) {
            return Err(Signal::Error(Box::new(al_builtins::native::AlError {
                message: "The record is already open.".into(),
                code: "RecordAlreadyOpened".into(),
                call_stack: String::new(),
                ..Default::default()
            })));
        }
        let no = int_arg(args, 0)?;
        let temp = match args.get(1) {
            Some(v) => ops::truthy(v)?,
            None => false,
        };
        let company = match args.get(2) {
            None => None,
            Some(c) => {
                let name = al_builtins::native::text_of(c).unwrap_or_default();
                match self.resolve_company(&name)? {
                    Some(company) => company,
                    None => return Err(Self::company_missing(&name)),
                }
            }
        };
        // corpus recordref/TestRecordRefOpenCompilationTarget.al:27-35: an internal system table is
        // refused for a Cloud-target app (this app is Cloud), whether or not a dependency app
        // declares an object with that number (the platform's `System.app` does, as
        // `Object Metadata`; a complete compile of the dependencies must not change the answer).
        // Agent Creation Control (2000000297) is refused the same way from BC 28 on, where the System app declares it
        // (platform/lt-aw-fresh-tenant/AwTests.al:590-616, BC28PLUS arm :612); on BC 27 no table has the number and
        // the open fails as for a missing object (:614).
        if no == 2_000_000_297 && self.table_object(no).is_some() {
            match self.caller_target() {
                Some(t) if t.eq_ignore_ascii_case("Cloud") => {
                    return Err(al_error(format!(
                        "You cannot open record {no} from a RecordRef data type when you are using target Cloud."
                    )));
                }
                // An OnPrem app opens it and reads its one row (onprem platform/lt-aw-fresh-tenant/AwOnPremTests.al:234-253).
                Some(t) if t.eq_ignore_ascii_case("OnPrem") => {}
                other => {
                    return unsupported(format!(
                        "RecordRef.Open(2000000297) from an app whose target is {} (only Cloud and OnPrem are pinned)",
                        other.unwrap_or("not known")
                    ));
                }
            }
        }
        if no == 2_000_000_071 {
            return Err(al_error(format!(
                "You cannot open record {no} from a RecordRef data type when you are using target Cloud."
            )));
        }
        let Some(table) = self.table_object(no) else {
            // A moved table: the open fails with AL:MetadataNotFound (AwTests.al:440). Opened by the test method itself, the
            // text names the test runner (platform/lt-aw-fresh-tenant/AwTests.al:434-438 MovedTable_OwnApp_ReadAndWrite).
            // Opened from a helper or inside a try function it is not pinned.
            if self.only_moved_table(no) {
                if self.in_test_method_itself() {
                    return Err(self.missing_object_from("Table", no, "Test Runner - Mgt"));
                }
                return Err(Signal::Error(Box::new(al_builtins::native::unpinned_text_error(
                    "AL:MetadataNotFound",
                    "the text of RecordRef.Open on a table declared ObsoleteState = Moved (AwTests.al:434-439)",
                ))));
            }
            return match no {
                // :37-62: these system tables open for a Cloud-target app and report their number.
                2_000_000_026 | 2_000_000_187 | 2_000_000_188 => {
                    *lib.borrow_mut() = LibraryValue::SystemTable(no);
                    Ok(Value::Null)
                }
                // recordref/lt-recordref-open-system-tables/RecordRefOpenSystemTables.al:RecordRef_Open_SystemTables_ByNumber: no
                // table has these numbers (t2000000999, t2000000000); other numbers in the system range are not pinned.
                2_000_000_999 | 2_000_000_000 => Err(self.missing_object("Table", no)),
                n if n >= i64::from(SYSTEM_FIELD_BASE) => unsupported(format!(
                    "RecordRef.Open on system table {n} (whether a Cloud target may open it is not pinned)"
                )),
                // A table of a dependency app that nothing reached is not compiled (ADR-013); BC has it.
                n if self
                    .engine
                    .program
                    .objects
                    .iter()
                    .any(|o| o.is_shell_of(al_ir::ObjKind::Table) && i64::from(o.id.unwrap_or(0)) == n) =>
                {
                    unsupported(format!(
                        "RecordRef.Open on table {n}, which belongs to a dependency app and was not compiled (no code the workspace reaches uses it)"
                    ))
                }
                // corpus recordref/TestRecordRefOpenMissingTable.al:31-49 pins both halves; the full
                // text and the code AL:MetadataNotFound: recordref/TestRecordRefRuntimeContracts.al:30.
                // Table 0 opens nothing and raises nothing (recordref/lt-refs/RfTOpen.al:Open_InvalidTableNumbers `zero`).
                0 => Ok(Value::Null),
                n => Err(self.missing_object("Table", n)),
            };
        };
        // A table of TableType CRM can only be opened through a registered table connection; with none
        // registered the open fails (session/TestCrmTableNoConnectionContracts.al:111-118, BC 27.5).
        // With one registered, what the open does is not pinned. Other table types are not pinned.
        let table_type = self.engine.object_facts[table].table_type.as_ref();
        match table_type {
            None | Some(al_ir::TableType::Normal) | Some(al_ir::TableType::Temporary) => {}
            Some(al_ir::TableType::Crm) if !self.table_connections.iter().any(|(kind, _)| *kind == 0) => {
                return Err(crate::transactions::al_error_code(
                    "Table connection  for table type CRM must be registered using RegisterTableConnection or cmdlet New-NAVTableConnection before it can be used.",
                    "Argument",
                ));
            }
            Some(other) => {
                return unsupported(format!(
                    "RecordRef.Open of a table with TableType = {} (a table connection is not modelled)",
                    other.lower()
                ));
            }
        }
        let Value::Record(rec) = self.default_value(&RtType::Record(table as u32, temp)) else {
            return unsupported("record buffer for RecordRef.Open");
        };
        if company.is_some() {
            if temp {
                return unsupported("RecordRef.Open of a temporary record on another company (not pinned)");
            }
            rec.borrow_mut().company = company;
        }
        *lib.borrow_mut() = LibraryValue::RecordRef(rec);
        Ok(Value::Null)
    }

    /// `RecordRef.Get(RecordId)` where the RecordRef is not open or is open on another table than the RecordId's. Pinned:
    /// a found row opens the RecordRef on the RecordId's table, from a never opened or closed ref and from one open on another table
    /// (errors/lt-ms-refusals-131/P131RecordRefGet.al:P131_Get_Unopened_ExistingRow, P131_Get_Closed_RecordRef, P131_Get_OpenOnAnotherTable
    /// `A existing`; errors/lt-ms-refusals-135/P135RecordRefGet.al:P135_Get_RecordId_OtherTable_ThenOwn). A missing row leaves a ref that was
    /// not open closed, answering No (P131_Get_Unopened_MissingRow, P135 `get deleted`) or, with the result unused, raising "does not
    /// exist" (P131_Get_Unopened_ResultUnused `missing`). A blank RecordId answers No and changes nothing (P131_Get_EmptyRecordId).
    /// Returns `None` when the ref is open on the RecordId's table (the ordinary Get applies).
    fn rr_get_record_id(&mut self, lib: &Lib, args: &[Value]) -> Result<Option<Value>, Signal> {
        let id = match args.first() {
            Some(Value::Library(_, l)) => match &*l.borrow() {
                LibraryValue::RecordId { table, key, .. } => Some((*table, key.clone())),
                LibraryValue::Empty => None,
                other => return unsupported(format!("RecordRef.Get of {}", other.describe())),
            },
            other => return unsupported(format!("RecordRef.Get of {}", other.describe())),
        };
        let current = match &*lib.borrow() {
            LibraryValue::RecordRef(r) => Some(r.clone()),
            LibraryValue::Empty => None,
            _ => return Ok(None),
        };
        let Some((table, key)) = id else {
            if !self.ret_used {
                // RecordRefGetRecordId.al:Get_BlankRecordId_AsStatement: the table with the ID 0 does not exist.
                return Err(self.missing_object("Table", 0));
            }
            return Ok(Some(Value::Boolean(false)));
        };
        if let Some(cur) = &current {
            if cur.borrow().table == table {
                return Ok(None);
            }
            let c = cur.borrow();
            // The filters of the ref are gone with the table it leaves (RecordRefGetRecordId.al:Get_RecordIdOfAnotherTable);
            // marks and a temporary table are not pinned.
            if c.temporary || c.marked_only || !c.marks.is_empty() {
                return unsupported(
                    "RecordRef.Get of a RecordId of another table on a ref with marks or a temporary table (not pinned)",
                );
            }
        }
        let Value::Record(rec) = self.default_value(&RtType::Record(table, false)) else {
            return unsupported("record buffer for RecordRef.Get");
        };
        let mut a = key;
        let found = self.record_op(RecordOp::Get, &rec, &mut a)?;
        match found {
            Value::Boolean(true) => {
                *lib.borrow_mut() = LibraryValue::RecordRef(rec);
                Ok(Some(found))
            }
            Value::Boolean(false) if current.is_none() || self.ret_used => Ok(Some(found)),
            _ => unsupported("RecordRef.Get of a missing row of another table than the open one (not pinned)"),
        }
    }

    /// Whether the buffer differs from what the ref last read or wrote (`dirty_base`); a buffer that came from elsewhere
    /// (GetTable, Init) is compared with the row stored under its primary key, or with a fresh buffer when there is none
    /// (assumption record-ref-is-dirty-against-stored-row). Stored fields only (a FlowField is not part of the row). On a
    /// partial load a field that was written without being loaded is dirty whatever it holds, and one that was never
    /// loaded is not compared (RecordRefIsDirtyEdges.al:IsDirty_SetLoadFields_*). A value that cannot be compared (a Blob)
    /// is refused: nothing pins it.
    fn rr_is_dirty(&mut self, rec: &Rec, t: &TableDef) -> Result<Value, Signal> {
        let fields = rec.borrow().fields.clone();
        let base = match rec.borrow().dirty_base.clone() {
            Some(b) => b,
            None => {
                al_builtins::assumptions::assume(
                    al_builtins::assumptions::Assumption::RecordRefIsDirtyAgainstStoredRow,
                );
                let cx = self.cx_for(rec, t)?;
                let pk = t.primary_key(&fields);
                let stored = self
                    .store_for(rec)
                    .get(&cx, t, &pk)
                    .map_err(|e| Signal::Unsupported(format!("{e}")))?;
                match stored {
                    Some(row) => row,
                    None => self.initial_fields(t.object as usize),
                }
            }
        };
        let (partial, have, unsure) = {
            let r = rec.borrow();
            match (&r.load.buffer, r.load.partial) {
                (Some(b), true) => (true, b.have.clone(), b.unsure.clone()),
                _ => (
                    false,
                    al_builtins::value::FieldSet::All,
                    al_builtins::value::FieldSet::Of(Vec::new()),
                ),
            }
        };
        for (i, f) in t.fields.iter().enumerate() {
            if !f.stored {
                continue;
            }
            if partial {
                if unsure.contains(i) {
                    return Ok(Value::Boolean(true));
                }
                if !have.contains(i) {
                    continue;
                }
            }
            match ops::compare(&fields[i], &base[i]) {
                Ok(std::cmp::Ordering::Equal) => {}
                Ok(_) => return Ok(Value::Boolean(true)),
                Err(_) => {
                    return unsupported(format!(
                        "RecordRef.IsDirty on field {} of a type that cannot be compared (not pinned)",
                        f.name
                    ));
                }
            }
        }
        Ok(Value::Boolean(false))
    }

    fn recordref_op(&mut self, method: &str, lib: &Lib, args: &mut [Value]) -> Result<Value, Signal> {
        let r = self.recordref_op_inner(method, lib, args);
        // What the ref read or wrote is what IsDirty compares with. A read that found nothing leaves the buffer alone.
        let snapshot = match (&r, method) {
            (Ok(Value::Boolean(false)), _) | (Ok(Value::Integer(0)), "Next") => None,
            (
                Ok(_),
                "Open" | "Find" | "FindFirst" | "FindLast" | "FindSet" | "Next" | "Get" | "GetBySystemId" | "Insert"
                | "Modify" | "Rename",
            ) => Some(true),
            (Ok(_), "Init" | "Clear" | "Close" | "GetTable" | "Copy") => Some(false),
            _ => None,
        };
        if let Some(keep) = snapshot {
            if let LibraryValue::RecordRef(rec) = &*lib.borrow() {
                let mut s = rec.borrow_mut();
                s.dirty_base = keep.then(|| s.fields.clone());
            }
        }
        r
    }

    fn recordref_op_inner(&mut self, method: &str, lib: &Lib, args: &mut [Value]) -> Result<Value, Signal> {
        match method {
            "Open" => return self.rr_open(lib, args),
            "Get" if matches!(args.first(), Some(Value::Library(n, _)) if n.eq_ignore_ascii_case("RecordId")) => {
                if let Some(v) = self.rr_get_record_id(lib, args)? {
                    return Ok(v);
                }
            }
            // Close and Clear leave the ref unusable; closing an unopened ref is harmless.
            "Close" => {
                *lib.borrow_mut() = LibraryValue::Empty;
                return Ok(Value::Null);
            }
            // Doc (RecordRef.GetTable): the ref takes the record variable's table, buffer and
            // filters. The corpus pins the other half: the Record is left alone
            // (recordref/TestRecordRefGetSetTableContracts.al:30-60).
            // A ref that was never opened takes its table from the record (Assert.RecordIsEmpty does this).
            "GetTable" => {
                let from_variant = std::mem::take(&mut self.get_table_from_variant);
                let src = match args.first() {
                    // Through a Variant the record keeps its filters and loses its ChangeCompany
                    // (types/lt-cw-variant-leads/CwVariantChangeCompany.al:CW_Variant_KeepsOrDropsChangeCompany `variant to recordref`,
                    // record/TestChangeCompanyFiltersCopyDeleteContracts.al:248 `variantGetTableCompany`).
                    Some(Value::Record(src)) if from_variant => {
                        let mut copy = src.borrow().clone();
                        copy.company = None;
                        *lib.borrow_mut() = LibraryValue::RecordRef(Rc::new(RefCell::new(copy)));
                        return Ok(Value::Null);
                    }
                    Some(Value::Record(src)) => src.clone(),
                    // A RecordRef as the source (Microsoft's Gen. Jnl.-Post Preview): the same copy of its record.
                    Some(Value::Library(_, other)) => match &*other.borrow() {
                        LibraryValue::RecordRef(r) => {
                            al_builtins::assumptions::assume(
                                al_builtins::assumptions::Assumption::RecordRefGetTableOfRecordRef,
                            );
                            r.clone()
                        }
                        _ => return unsupported("RecordRef.GetTable of a RecordRef that is not open"),
                    },
                    other => return unsupported(format!("RecordRef.GetTable of {}", other.describe())),
                };
                let copy = src.borrow().clone();
                *lib.borrow_mut() = LibraryValue::RecordRef(Rc::new(RefCell::new(copy)));
                return Ok(Value::Null);
            }

            // RecordRef.Copy(FromRecordRef [, ShareTable]): the opened target takes the source's buffer, filters, key,
            // direction and current row as Record.Copy does; a target that was never opened raises "The record is
            // not open." (RecordNotOpened); ShareTable is accepted only for two temporary records, which then share
            // the table (types/PWMisc.al:RecordRef_Copy_*, records/W4SortKeys.al:RecordRef_Ascending, BC 27.5 and
            // 29.0). A Record as the source is not pinned.
            "Copy" => {
                let dst = match &*lib.borrow() {
                    LibraryValue::RecordRef(r) => r.clone(),
                    LibraryValue::Empty => {
                        return Err(Signal::Error(Box::new(AlError {
                            message: "The record is not open.".into(),
                            code: "RecordNotOpened".into(),
                            call_stack: String::new(),
                            ..Default::default()
                        })));
                    }
                    _ => return unsupported("RecordRef.Copy on a RecordRef opened on a system table"),
                };
                let src = match args.first() {
                    Some(Value::Library(_, l)) => match &*l.borrow() {
                        LibraryValue::RecordRef(r) => r.clone(),
                        _ => return unsupported("RecordRef.Copy from a RecordRef that is not open (not pinned)"),
                    },
                    other => {
                        return unsupported(format!("RecordRef.Copy from {} (not pinned)", other.describe()));
                    }
                };
                if src.borrow().table != dst.borrow().table {
                    return unsupported("RecordRef.Copy between two tables (not pinned)");
                }
                let t = self.table(&dst)?;
                let mut forwarded = vec![Value::Record(src)];
                forwarded.extend(args.get(1).cloned());
                return self.record_meta_op(RecordOp::Copy, &dst, &t, &mut forwarded);
            }

            "Number" => match &*lib.borrow() {
                LibraryValue::SystemTable(n) => return Ok(Value::Integer(*n as i32)),
                // A RecordRef never opened is table 0 (corpus branch al-runtime-rs-staging, ae3fe88,
                // tests/al-language/types/PXEnumRecordRef.al:293-299).
                LibraryValue::Empty => return Ok(Value::Integer(0)),
                _ => {}
            },
            // A RecordRef never opened answers false (PXEnumRecordRef.al:293-298); with the result
            // unused nothing pins what it raises.
            "GetBySystemId" if matches!(&*lib.borrow(), LibraryValue::Empty) => {
                return if self.ret_used {
                    Ok(Value::Boolean(false))
                } else {
                    unsupported(
                        "RecordRef.GetBySystemId on a RecordRef that is not open with the result unused: what BC raises is not pinned (docs/probes-needed.md)",
                    )
                };
            }
            // `Get(RecordId)` on a RecordRef never opened opens the table of the RecordId and answers whether the row is
            // there; when it is not, the ref stays closed (Number answers 0) so that Data Type Management's
            // `if not RecRef.Get(RecID) then RecRef.Open(RecID.TableNo)` opens it (misc/lt-probes-needed-4 batch 4:
            // W4 RecRef Get Tests, Get_OnUnopenedRecordRef_ExistingRow and _MissingRow, GetOrOpen_Pattern_OfDataTypeManagement).
            // With the result unused the error of a missing row is not pinned (open_ref refuses).
            "Get" if matches!(&*lib.borrow(), LibraryValue::Empty) && self.ret_used => {
                let table_no = match args.first() {
                    Some(Value::Library(_, l)) => match &*l.borrow() {
                        LibraryValue::RecordId { table_no, .. } => *table_no,
                        _ => return Self::open_ref(lib, "RecordRef.Get").map(|_| Value::Null),
                    },
                    _ => return Self::open_ref(lib, "RecordRef.Get").map(|_| Value::Null),
                };
                self.rr_open(lib, &[Value::Integer(table_no as i32)])?;
                let found = self.recordref_op("Get", lib, args);
                if !matches!(found, Ok(Value::Boolean(true))) {
                    *lib.borrow_mut() = LibraryValue::Empty;
                }
                return found;
            }
            _ => {}
        }
        let rec = Self::open_ref(lib, &format!("RecordRef.{method}"))?;
        let t = self.table(&rec)?;
        match method {
            "Number" => Ok(Value::Integer(
                self.engine.program.objects[t.object as usize].id.unwrap_or(0) as i32,
            )),
            "Name" => Ok(Value::Text(t.name.clone().into())),
            "Caption" => self.caption_checked(&t).map(|c| Value::Text(c.into())),
            // corpus recordref/TestRecordRefGetSet.al:16-30 pins only that it is not empty.
            "CurrentCompany" => Ok(Value::Text(self.current_company_of(&rec).into())),
            "ChangeCompany" => self.change_company(&rec, args),
            "IsTemporary" => Ok(Value::Boolean(rec.borrow().temporary)),
            "HasFilter" => Ok(Value::Boolean(Self::has_filter(&rec))),
            // Pinned: corpus misc/lt-open-overloads-collections-isolated/P107RecRefNames.al:101-135 and
            // recordref/lt-recordref-fieldref-members/P92DirtyRef.al:21-95 (a fresh ref, a found row, a written
            // field with the same and with another value, after Modify, after Find, after Init, after Validate).
            "IsDirty" => self.rr_is_dirty(&rec, &t),
            "FilterGroup" => {
                // The getter and the setter for any group are pinned (filters/W4Filters.al:RecordRef_FilterGroup_*,
                // record/PWFilterGroupOr.al:OrGroup_RecordRef).
                Self::filter_group(&rec, args)
            }
            "GetFilters" => self.get_filters(&rec, &t).map(|s| Value::Text(s.into())),
            "GetView" => {
                let use_names = match args.first() {
                    Some(v) => ops::truthy(v)?,
                    None => true,
                };
                self.get_view(&rec, &t, use_names).map(|s| Value::Text(s.into()))
            }
            "SetView" => {
                let view = al_builtins::native::text_of(args.first().unwrap_or(&Value::Null)).unwrap_or_default();
                self.set_view(&rec, &t, &view).map(|()| Value::Null)
            }
            // The load set of a RecordRef is given by field numbers (record/TestRecordPartialLoad.al
            // PartialLoad_SetLoadFields_UnknownFieldNo_Throws: a number no field has is an error that
            // names it; its text is not pinned). The rest is the Record rule (partial.rs).
            "SetLoadFields" | "AddLoadFields" | "AreFieldsLoaded" => {
                let mut positions = Vec::with_capacity(args.len());
                for a in args.iter() {
                    let Value::Integer(n) = a else {
                        return unsupported(format!("RecordRef.{method} with {}", a.describe()));
                    };
                    match t.fields.iter().position(|f| i64::from(f.id) == i64::from(*n)) {
                        Some(p) => positions.push(Value::Integer(p as i32)),
                        None => return Err(Signal::Error(Box::new(al_builtins::native::unpinned_error()))),
                    }
                }
                let op = if method == "AreFieldsLoaded" {
                    RecordOp::AreFieldsLoaded
                } else {
                    record_method(method)
                };
                self.load_fields_op(op, &rec, &t, &positions)
            }
            // The fields are given by number; a read then calculates them as for a Record (record/TestSetAutoCalcFieldsOnGet.Codeunit.al:112-124
            // RecordRef_SetAutoCalcFields_ThenGet_Calculates pins Get with one FlowField; the other reads share the Record rule).
            "SetAutoCalcFields" => {
                let mut positions = Vec::with_capacity(args.len());
                for a in args.iter() {
                    let Value::Integer(n) = a else {
                        return unsupported(format!("RecordRef.SetAutoCalcFields with {}", a.describe()));
                    };
                    match t.fields.iter().position(|f| i64::from(f.id) == i64::from(*n)) {
                        Some(p) => positions.push(p),
                        None => return Err(Signal::Error(Box::new(al_builtins::native::unpinned_error()))),
                    }
                }
                self.flow_op(
                    RecordOp::SetAutoCalcFields,
                    &rec,
                    &positions.iter().map(|&p| Value::Integer(p as i32)).collect::<Vec<_>>(),
                )
            }
            // Doc (RecordRef.Duplicate): a new ref with the same table, buffer and filters, independent of the original.
            "RecordId" => Ok(self.record_id(&rec, &t)),
            "LockTable" => {
                // RecordRef.LockTable(true, true) raises the Wait = FALSE error (recordref/lt-refs/RfTViews.al:314-322).
                if matches!(args, [Value::Boolean(true), Value::Boolean(true)]) {
                    return Err(crate::recmeta::lock_table_bad_parm(&t));
                }
                // What a RecordRef's LockTable does to its ReadIsolation is not pinned (only Record's is,
                // misc/lt-cv-odd-leads/CvReadIsolationState.al:47-48).
                if rec.borrow().read_isolation != 0 && !matches!(args.first(), Some(Value::Boolean(false))) {
                    return unsupported("RecordRef.LockTable after a ReadIsolation was set (not pinned)");
                }
                let mut a: Vec<Value> = args.iter().take(1).cloned().collect();
                self.record_meta_op(RecordOp::LockTable, &rec, &t, &mut a)
            }
            "ReadConsistency" | "RecordLevelLocking" | "ReadPermission" | "WritePermission" | "CountApprox"
            | "ReadIsolation" | "SecurityFiltering" | "GetPosition" | "SetPosition" | "Ascending" | "SetRecFilter"
            | "Mark" | "MarkedOnly" | "ClearMarks" => self.record_meta_op(record_method(method), &rec, &t, args),
            // Doc (RecordRef.Get): loads the row a RecordId names; the table must be the ref's own.
            "Get" => {
                let key = match args.first() {
                    Some(Value::Library(_, l)) => match &*l.borrow() {
                        LibraryValue::RecordId { table, key, .. } if *table == t.object => key.clone(),
                        LibraryValue::RecordId { .. } => {
                            return unsupported(
                                "RecordRef.Get of a RecordId of another table (error text not pinned yet)",
                            );
                        }
                        other => return unsupported(format!("RecordRef.Get of {}", other.describe())),
                    },
                    other => return unsupported(format!("RecordRef.Get of {}", other.describe())),
                };
                let mut a = key;
                self.record_op(RecordOp::Get, &rec, &mut a)
            }
            // As Record.GetBySystemId, and a miss with the result unused raises "The <table> does not
            // exist" naming the Guid (PXEnumRecordRef.al:199-301).
            "GetBySystemId" => self.get_by_system_id(&rec, &t, args, true),
            "AddLink" | "DeleteLink" | "DeleteLinks" | "HasLinks" | "CopyLinks" => {
                self.record_links(record_method(method), &rec, &t, args)
            }
            "Duplicate" => {
                // A duplicate of a temporary RecordRef is temporary and sees the same rows (record/
                // TestProbeRecordRefContracts.al:Duplicate_OfTemporaryRef: `istemporary=Yes count=1`).
                let copy = rec.borrow().clone();
                Ok(handle(
                    "RecordRef",
                    LibraryValue::RecordRef(Rc::new(RefCell::new(copy))),
                ))
            }
            "SystemIdNo" | "SystemCreatedAtNo" | "SystemCreatedByNo" | "SystemModifiedAtNo" | "SystemModifiedByNo" => {
                let field = method.trim_end_matches("No").replace("SystemIdNo", "SystemId");
                match t.fields.iter().find(|f| f.name == field) {
                    Some(f) => Ok(Value::Integer(f.id as i32)),
                    None => unsupported(format!("RecordRef.{method} on a table without {field}")),
                }
            }
            // A RecordRef write inside a TryFunction whose result is used is not pinned (only Record.Insert is,
            // misc/lt-cu-odd-leads/CuTryTests.al:83-150).
            "Insert" | "Modify" | "Delete" | "DeleteAll" | "Rename"
                if !self.tx.used_tries.is_empty() && !rec.borrow().temporary =>
            {
                unsupported(format!(
                    "RecordRef.{method} inside a TryFunction whose result is used (not pinned)"
                ))
            }
            "IsEmpty" | "Count" | "Init" | "Insert" | "Modify" | "Delete" | "DeleteAll" | "Find" | "FindFirst"
            | "FindLast" | "FindSet" | "Next" | "Reset" | "Rename" => self.record_op(record_method(method), &rec, args),
            "Field" => {
                let no = int_arg(args, 0)?;
                // BC 29 has the user-name FlowFields at 2000000005-8 (platform/fields.rs `USER_NAME_SYSTEM_FIELDS`); the
                // runtime does not hold them.
                if (2_000_000_005..=2_000_000_008).contains(&no)
                    && self.has(al_version::Gate::PlatformMetadataColumnsFrom29)
                {
                    return unsupported(format!(
                        "RecordRef.Field({no}): the user-name system FlowFields (SystemCreatedByUserName and the others) are not modelled"
                    ));
                }
                match t.fields.iter().position(|f| i64::from(al_field_no(f.id)) == no) {
                    // A removed field raises the obsolete error with its reason (recordref/PWRecordMisc.al:128-167).
                    Some(pos) if self.field_removed(&t, pos) => {
                        let Some(reason) = self
                            .field_property(&t, pos, "ObsoleteReason")
                            .and_then(|r| crate::recmeta::string_literal(&r))
                        else {
                            return unsupported(format!(
                                "RecordRef.Field({no}) of a removed field without an ObsoleteReason (the text is not pinned)"
                            ));
                        };
                        let table_id = self.engine.program.objects[t.object as usize].id.unwrap_or(0);
                        Err(crate::transactions::al_error_code(
                            &format!(
                                "Field {} ({no}) of table {} ({table_id}) is obsoleted, reason: {reason}",
                                t.fields[pos].name, t.name
                            ),
                            "MetadataObsolete",
                        ))
                    }
                    Some(pos) => Ok(handle("FieldRef", LibraryValue::FieldRef(rec, pos))),
                    None => unsupported(format!(
                        "RecordRef.Field({no}) on a field {} does not have (error text not pinned yet)",
                        t.name
                    )),
                }
            }
            "FieldExist" => {
                let no = int_arg(args, 0)?;
                // The rowversion's own number exists on BC 29 (recordref/lt-recordref-fieldref-members/P92Fields.al:72,
                // `#if BC29PLUS`); before 29 nothing pins it.
                if no == i64::from(SYSTEM_ROW_VERSION_ID) && t.fields.iter().any(|f| f.id == SYSTEM_ROW_VERSION_ID) {
                    if self.has(al_version::Gate::PlatformMetadataColumnsFrom29) {
                        return Ok(Value::Boolean(true));
                    }
                    return unsupported(
                        "RecordRef.FieldExist(2000000005) on this BC version: no corpus test pins it (docs/probes-needed.md)",
                    );
                }
                // Field 0 (SystemRowVersion) and a disabled field do not exist
                // (record/TestRowVersionFieldNumberFieldActiveContracts.al:73-75, 120-121).
                Ok(Value::Boolean(t.fields.iter().enumerate().any(|(i, f)| {
                    i64::from(f.id) == no
                        && f.id != SYSTEM_ROW_VERSION_ID
                        && self.field_enabled(&t, i)
                        && !self.field_removed(&t, i)
                })))
            }
            "FieldCount" => Ok(Value::Integer(enumerable_fields(self, &t).len() as i32)),
            "FieldIndex" => {
                let i = int_arg(args, 0)?;
                // Past the count the index reaches the system fields in number order: $systemId,
                // SystemCreatedAt, SystemCreatedBy, SystemModifiedAt, SystemModifiedBy (corpus
                // recordref/TestRecordRefRuntimeContracts.al:RecordRef_FieldCount_ExcludesSystemFields_...);
                // what follows them is not pinned.
                let normal = enumerable_fields(self, &t);
                let reached = usize::try_from(i - 1).ok().and_then(|i| match normal.get(i) {
                    Some(&p) => Some(p),
                    None => {
                        let id = SYSTEM_FIELD_BASE + (i - normal.len()) as u32;
                        (id <= SYSTEM_FIELD_BASE + 4)
                            .then(|| t.fields.iter().position(|f| f.id == id))
                            .flatten()
                    }
                });
                match reached {
                    Some(pos) => Ok(handle("FieldRef", LibraryValue::FieldRef(rec, pos))),
                    // record/TestRowVersionFieldNumberFieldActiveContracts.al:84-86.
                    None => Err(Signal::Error(Box::new(al_builtins::native::AlError {
                        message: "Index out of bounds.".into(),
                        code: "IndexOutOfBounds".into(),
                        call_stack: String::new(),
                        ..Default::default()
                    }))),
                }
            }
            "KeyCount" => Ok(Value::Integer(key_count(&self.base_table(&t)) as i32)),
            "CurrentKey" => self.record_meta_op(RecordOp::CurrentKey, &rec, &t, args),
            // recordref/TestRecordRefKeyIndexSetRecFilterContracts.al:75-176. Keys number from 1 in
            // declaration order and the hidden SystemId key is last; an index outside them raises
            // DB:KeyNotFound; a current key of fields no declared key starts with reports -1.
            "CurrentKeyIndex" => {
                let bt = self.base_table(&t);
                let base = bt.keys.len();
                if let Some(v) = args.first() {
                    let i = int_arg(std::slice::from_ref(v), 0)?;
                    let count = key_count(&bt) as i64;
                    if i < 1 || i > count {
                        return Err(Signal::Error(Box::new(AlError {
                            message: format!("The {} table does not have an active key at index {i}.", t.name),
                            code: "DB:KeyNotFound".into(),
                            call_stack: String::new(),
                            ..Default::default()
                        })));
                    }
                    let mut r = rec.borrow_mut();
                    if (i as usize) <= base {
                        r.key = i as usize - 1;
                        r.adhoc = None;
                    } else {
                        let fields = key_fields(&bt, base).expect("a counted key");
                        r.key = base;
                        r.adhoc = Some((base, fields));
                    }
                    return Ok(Value::Integer(i as i32));
                }
                let r = rec.borrow();
                Ok(Value::Integer(match &r.adhoc {
                    _ if r.key < base => r.key as i32 + 1,
                    Some((_, f)) if r.key == base && key_fields(&bt, base).as_ref() == Some(f) => base as i32 + 1,
                    _ => -1,
                }))
            }
            "KeyIndex" => {
                let i = int_arg(args, 0)?;
                match usize::try_from(i - 1)
                    .ok()
                    .filter(|k| *k < key_count(&self.base_table(&t)))
                {
                    Some(k) => Ok(handle("KeyRef", LibraryValue::KeyRef(rec, k))),
                    None => unsupported(format!(
                        "RecordRef.KeyIndex({i}) out of range (error text not pinned yet)"
                    )),
                }
            }
            // Doc (RecordRef.SetTable): the record variable takes the ref's buffer and
            // filters. The corpus pins that the Record's old buffer does not flow into the
            // ref (recordref/TestRecordRefGetSetTableContracts.al:78-104).
            "SetTable" => {
                let Some(Value::Record(dst)) = args.first() else {
                    return unsupported(format!("RecordRef.SetTable of {}", args.first().describe()));
                };
                let same_table = rec.borrow().table == dst.borrow().table;
                if !same_table {
                    return unsupported("RecordRef.SetTable into a record of another table");
                }
                // The record variable keeps its own rows and its own kind: a temporary RecordRef set into a permanent record leaves it
                // permanent on the table's rows, a permanent one set into a temporary record leaves it temporary and empty of them; only the
                // current record and the filters pass (recordref/lt-settable-temporary/SetTableTemporaryToPermanent.al:
                // SetTable_TempIntoPermanent_State, _Insert, _ModifyDelete, _FilteredSource, SetTable_PermanentIntoTemporary_State).
                if !Rc::ptr_eq(&rec, dst) {
                    let s = rec.borrow();
                    // SetTable does not move the target either (:123-127); GetTable takes the company.
                    let mut d = dst.borrow_mut();
                    d.field_descending = s.field_descending;
                    // The ReadIsolation passes too (recordref/lt-recordref-fieldref-members/P92ReadIsolation.al:ReadIsolation_FromRecordAndToRecord).
                    d.read_isolation = s.read_isolation;
                    d.fields = s.fields.clone();
                    d.filters = s.filters.clone();
                    d.key = s.key;
                    d.adhoc = s.adhoc.clone();
                    d.ascending = s.ascending;
                    d.position = s.position.clone();
                    d.load.buffer = s.load.buffer.clone();
                    d.load.partial = s.load.partial;
                }
                Ok(Value::Null)
            }
            other => unsupported(format!("RecordRef.{other}")),
        }
    }

    fn keyref_op(&mut self, method: &str, lib: &Lib, args: &mut [Value]) -> Result<Value, Signal> {
        let (rec, k) = match &*lib.borrow() {
            LibraryValue::KeyRef(r, k) => (r.clone(), *k),
            _ => return unsupported(format!("KeyRef.{method} on a KeyRef that was never assigned")),
        };
        let t = self.table(&rec)?;
        match method {
            // Every key a table declares is enabled: the lowerer does not read key
            // properties, and no corpus key sets Enabled = false.
            "Active" => Ok(Value::Boolean(true)),
            "FieldCount" => Ok(Value::Integer(key_fields(&t, k).map_or(0, |f| f.len()) as i32)),
            "Record" => Ok(handle("RecordRef", LibraryValue::RecordRef(rec))),
            "FieldIndex" => {
                let i = int_arg(args, 0)?;
                match usize::try_from(i - 1)
                    .ok()
                    .and_then(|i| key_fields(&t, k).and_then(|f| f.get(i).copied()))
                {
                    Some(pos) => Ok(handle("FieldRef", LibraryValue::KeyFieldRef(rec, pos))),
                    // types/TestTypeSystemContracts.al:261 pins the fragment "Index out of bounds"
                    // (recordref/TestBCSumIndexFieldContracts.al:188-198 pins that it throws).
                    None => Err(Signal::Error(Box::new(AlError {
                        message: "Index out of bounds".into(),
                        code: String::new(),
                        call_stack: String::new(),
                        ..Default::default()
                    }))),
                }
            }
            other => unsupported(format!("KeyRef.{other}")),
        }
    }

    fn fieldref_op(&mut self, method: &str, lib: &Lib, args: &mut [Value]) -> Result<Value, Signal> {
        let (rec, pos) = match &*lib.borrow() {
            LibraryValue::FieldRef(r, p) | LibraryValue::KeyFieldRef(r, p) => (r.clone(), *p),
            // Name, Number, Value(), Type, Length and Record raise on a FieldRef that was never assigned
            // (types/PSUnassignedFieldRef.al UnassignedFieldRef_Members).
            _ if matches!(method, "Name" | "Number" | "Type" | "Length" | "Record")
                || (method == "Value" && args.is_empty()) =>
            {
                return Err(Signal::Error(Box::new(AlError {
                    message: "Microsoft.Dynamics.Nav.Runtime.NavFieldRef variable not initialized.".into(),
                    code: "NotInitialized".into(),
                    call_stack: String::new(),
                    ..Default::default()
                })));
            }
            _ => {
                return unsupported(format!(
                    "FieldRef.{method} on a FieldRef that was never assigned: what BC does (an error, or a no-op) is not pinned (docs/probes-needed.md)"
                ));
            }
        };
        let t = self.table(&rec)?;
        let field = &t.fields[pos];
        match method {
            "Value" => match args.first() {
                // An Option field reads with its OptionCaption, which Format shows (recordref/lt-refs/RfTWrite.al:Init_ClearsFields
                // `after_opt` = Erste, recordref/lt-refs/RfTValue*.al:Value_Get_Defaults).
                None => {
                    let v = rec.borrow().fields[pos].clone();
                    // A column the platform row leaves unknown (its value nothing pins) is refused, naming the table and the field.
                    if let Value::Unknown(why) = &v {
                        return unsupported(why.to_string());
                    }
                    // A coverage Test ID read by number: no pinned shape (platform::coverage_test_id_refusal).
                    if !rec.borrow().temporary && self.is_coverage_test_id(&t, field.id) {
                        return unsupported(al_builtins::platform::coverage_test_id_refusal(
                            "FieldRef.Value of the Test ID",
                        ));
                    }
                    let order = {
                        let r = rec.borrow();
                        self.install_row_order(r.table, r.temporary, pos, &r.fields)
                    };
                    if let Some(why) = order {
                        return unsupported(why);
                    }
                    // A Blob of a stored row read without CalcFields: a record's own field answers HasValue true and its stream reads
                    // nothing (streams/TestInOutStream.al InOutStream_WriteText_BlobHasValue, streams/TestBlob.al
                    // Blob_Get_WithoutCalcFields_InStreamReadsEmpty), and the value FieldRef.Value hands out is an empty Blob:
                    // HasValue No, Length 0, an empty text, after Get + RecordRef.GetTable and after RecordRef.Open + FindFirst
                    // (recordref/lt-refs/RfTValues.al:QP_Blob_GetTable_ValueWithoutCalcFields, QP_Blob_OpenFind_ValueWithoutCalcFields).
                    // Microsoft's Temp Blob Impl.FromFieldRef relies on it (it asks HasValue, then CalcField).
                    if let Value::Blob(b) = &v {
                        let unloaded = {
                            let buf = b.borrow();
                            !buf.loaded && !buf.bytes.is_empty() && !buf.temp_row
                        };
                        if unloaded {
                            return Ok(Value::Blob(al_builtins::stream::new_blob()));
                        }
                    }
                    // A read loads a field the partial buffer left out (RecordRefIsDirtyEdges.al:IsDirty_SetLoadFields_ReadUnloadedThenCompare).
                    if rec.borrow().load.partial {
                        rec.borrow_mut().load.touch(pos);
                    }
                    if let (Value::Option(o, _), Ok(Some((false, members)))) = (&v, self.members_of_field(&t, pos)) {
                        let captions: std::sync::Arc<[String]> =
                            members.iter().map(|m| m.2.clone()).collect::<Vec<_>>().into();
                        return Ok(Value::Option(*o, captions));
                    }
                    Ok(v)
                }
                Some(v) => {
                    // Assigning Value does not run OnValidate (that is `Validate`). An Enum value goes into a Text or Code field as its caption
                    // (Code in upper case): types/lt-enum-to-text-fieldref/EnumToTextFieldRef.al:FieldRefValue_EnumToTextField_PerMember,
                    // FieldRefValue_EnumToCodeField. A number goes in as its Format text; one longer than the field raises and leaves
                    // the field as it was (types/lt-number-to-text/NumberToTextIsFormat.al:239-246, FieldRefSetValue_NumberOnCodeAndText_Result).
                    // A BigInteger beyond Integer into an Option or an Enum field raises Overflow, where an assignment to a
                    // variable raises TypeConversion (types/lt-bigint-enum-option/BigIntToEnumOption.al:401-413
                    // QO_Variant_ToOptionFieldRef_Value, :433-447 QO_Variant_ToEnumFieldRef_Value).
                    if let Value::BigInteger(n) = v {
                        if i32::try_from(*n).is_err() && matches!(field.ty, RtType::Option(_) | RtType::Enum(_)) {
                            return al_builtins::native::raise(
                                "Arithmetic operation resulted in an overflow.",
                                "AL:Overflow",
                            );
                        }
                    }
                    let converted = ops::number_into_text_field(v, &field.ty, &*self);
                    let v = match converted {
                        Some(r) => r?,
                        None => ops::convert(v.clone(), &field.ty)?,
                    };
                    let mut r = rec.borrow_mut();
                    r.fields[pos] = v;
                    if r.load.partial {
                        r.load.written(pos);
                    }
                    Ok(Value::Null)
                }
            },
            "Number" => Ok(Value::Integer(al_field_no(field.id))),
            // SystemId is `$systemId` and the rowversion `timestamp` to a FieldRef (record/
            // TestRowVersionFieldNumberFieldActiveContracts.al:70-72, 82-84).
            "Name" if field.id == SYSTEM_FIELD_BASE => Ok(Value::Text("$systemId".into())),
            "Name" if field.id == SYSTEM_ROW_VERSION_ID => Ok(Value::Text("timestamp".into())),
            "Name" => Ok(Value::Text(field.name.clone().into())),
            "Caption" => self
                .field_caption_checked(&t, pos, Some(&rec))
                .map(|s| Value::Text(s.into())),
            "Type" => self.field_type(&t, pos),
            "Class" => {
                let class = self
                    .field_property(&t, pos, "FieldClass")
                    .unwrap_or_default()
                    .trim()
                    .to_ascii_lowercase();
                let ord = match class.as_str() {
                    "flowfield" => 1,
                    "flowfilter" => 2,
                    "" | "normal" => 0,
                    other => return unsupported(format!("FieldClass `{other}`")),
                };
                let m = al_builtins::sysenum::option_members("FieldClass").expect("known");
                Ok(Value::Option(ord, m.into()))
            }
            "Relation" => self.field_relation(&rec, &t, pos).map(Value::Integer),
            "GetFilter" => self.field_filter_text(&rec, pos).map(|s| Value::Text(s.into())),
            // recordref/lt-refs/RfTFieldFilters.al:183 (GetRangeMinMax); the rules are the Record ones.
            "GetRangeMin" | "GetRangeMax" => {
                let op = if method == "GetRangeMin" {
                    RecordOp::GetRangeMin
                } else {
                    RecordOp::GetRangeMax
                };
                self.record_meta_op(op, &rec, &t, &mut [Value::Integer(pos as i32)])
            }
            "Record" => Ok(handle("RecordRef", LibraryValue::RecordRef(rec))),
            // fieldref/TestRelationTargetNamespaceScope.al:63-75. Record.CalcFields raises a pinned
            // error for a field that is not a FlowField (record/TestCalcFieldsFieldClass.Codeunit.al);
            // whether FieldRef.CalcField says the same is not pinned, so it is refused.
            "CalcField" => {
                // A Blob is accepted by Record.CalcFields (record/TestCalcFieldsFieldClass.Codeunit.al), and Temp Blob Impl.FromFieldRef
                // calls CalcField on every Blob it reads; FieldRef.CalcField of a Blob goes the same way (assumption
                // fieldref-calcfield-of-a-blob).
                let blob = self.engine.program.objects[t.object as usize]
                    .table
                    .as_ref()
                    .is_some_and(|s| {
                        matches!(
                            s.fields[pos].ty,
                            RtType::Library(al_builtins::library_kind::LibraryKind::Blob)
                        )
                    });
                if blob {
                    al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::FieldRefCalcFieldOfABlob);
                }
                let stored = !blob
                    && self.engine.program.objects[t.object as usize]
                        .table
                        .as_ref()
                        .is_some_and(|s| s.fields[pos].stored || s.fields[pos].flow_filter);
                if stored {
                    return unsupported("FieldRef.CalcField of a field that is not a FlowField (not pinned)");
                }
                self.flow_op(RecordOp::CalcFields, &rec, &[Value::Integer(pos as i32)])
            }
            // FieldRef.CalcSum sums a stored Decimal or Integer field over the rows the RecordRef's filters pass, a field in
            // no SumIndexFields list as well, puts the sum into the field and answers Yes
            // (recordref/lt-fieldref-calcsum-filters/FieldRefCalcSumFilters.al:P184_FieldRefCalcSum_Decimal_NoFilter_And_GroupFilter,
            // _Integer_And_TwiceAndEmpty, _DecimalNotInSumIndex). A Text or Code field and a FlowField raise
            // `Cannot calculate a sum of the values in the <field> field in the <table> table because one or more values are not
            // normal numeric fields.` (code DB:Ok; same file P184_FieldRefCalcSum_TextField_Errors, recordref/lt-refs/RfTFieldFilters.al:
            // CalcSum_Variants for a Sum and a Count FlowField). Other types are not pinned.
            "CalcSum" => {
                let (stored, flow_filter, ty) = {
                    let f = &self.engine.program.objects[t.object as usize]
                        .table
                        .as_ref()
                        .expect("table")
                        .fields[pos];
                    (f.stored, f.flow_filter, f.ty.clone())
                };
                let flow_field = !stored && !flow_filter;
                // Pinned for every type of FieldRefCalcSumTypes.al:CalcSum_NumericTypes, CalcSum_NonNumericTypes: a FlowFilter,
                // Option, Boolean, Date, Time, DateTime, Text and Code field raise too; BigInteger and Duration sum.
                if flow_field
                    || flow_filter
                    || (stored
                        && matches!(
                            ty,
                            RtType::Text(_)
                                | RtType::Code(_)
                                | RtType::Option(..)
                                | RtType::Boolean
                                | RtType::Date
                                | RtType::Time
                                | RtType::DateTime
                        ))
                {
                    return Err(crate::transactions::al_error_code(
                        &format!(
                            "Cannot calculate a sum of the values in the {} field in the {} table because one or more values are not normal numeric fields.",
                            self.field_caption(&t, pos),
                            self.caption(&t)
                        ),
                        "DB:Ok",
                    ));
                }
                if !stored
                    || !matches!(
                        ty,
                        RtType::Decimal | RtType::Integer | RtType::BigInteger | RtType::Duration
                    )
                {
                    return unsupported("FieldRef.CalcSum on a field of another type (not pinned)");
                }
                self.calc_sums(&rec, &[pos], true)?;
                Ok(Value::Boolean(true))
            }
            "Validate" => {
                let mut a = vec![Value::Integer(pos as i32)];
                a.extend(args.iter().cloned());
                self.record_op(RecordOp::Validate, &rec, &mut a)
            }
            "TestField" | "FieldError" => {
                let mut a = vec![Value::Integer(pos as i32)];
                a.extend(args.iter().cloned());
                self.record_meta_op(record_method(method), &rec, &t, &mut a)
            }
            "IsOptimizedForTextSearch" => Ok(Value::Boolean(
                self.field_property(&t, pos, "OptimizeForTextSearch")
                    .is_some_and(|v| v.trim().eq_ignore_ascii_case("true")),
            )),
            "IsEnum"
            | "EnumValueCount"
            | "GetEnumValueName"
            | "GetEnumValueCaption"
            | "GetEnumValueOrdinal"
            | "GetEnumValueNameFromOrdinalValue"
            | "GetEnumValueCaptionFromOrdinalValue"
            | "OptionCaption"
            | "OptionMembers"
            | "OptionString" => self.enum_metadata(method, &t, pos, args),
            // A field with `Enabled = false` is not active (record/TestProbeRecordRefContracts.al:KeyRef_And_FieldRef_Active).
            "Active" => Ok(Value::Boolean(self.field_enabled(&t, pos))),
            "Length" => match &field.ty {
                RtType::Text(Some(n)) | RtType::Code(Some(n)) => Ok(Value::Integer(*n as i32)),
                // fieldref/lt-fieldref-length-kinds/FieldRefLengthKinds.al:78-126: Option and Enum 4 whatever the
                // member names, Integer 4, Decimal 12, Date 4, Boolean 4, Guid 16, BigInteger 8, Blob 8, Time 4,
                // Duration 8, DateTime 8. Text and Code without a length are not pinned.
                RtType::Option(_)
                | RtType::Enum(_)
                | RtType::Integer
                | RtType::Date
                | RtType::Boolean
                | RtType::Time => Ok(Value::Integer(4)),
                RtType::Decimal => Ok(Value::Integer(12)),
                RtType::Guid => Ok(Value::Integer(16)),
                RtType::BigInteger | RtType::Duration | RtType::DateTime => Ok(Value::Integer(8)),
                RtType::Library(al_builtins::library_kind::LibraryKind::Blob) => Ok(Value::Integer(8)),
                other => unsupported(format!("FieldRef.Length of a {} field", other.describe())),
            },
            "SetRange" | "SetFilter" => {
                let mut a = vec![Value::Integer(pos as i32)];
                a.extend(args.iter().cloned());
                self.record_op(record_method(method), &rec, &mut a)
            }
            other => unsupported(format!("FieldRef.{other}")),
        }
    }
}

impl Session {
    /// `FieldRef.Type`. An Enum field is an Option field at run time (corpus fieldref/
    /// TestFieldRefEnumKeywordCase.al:95-106).
    fn field_type(&self, t: &TableDef, pos: usize) -> Result<Value, Signal> {
        let name = match &t.fields[pos].ty {
            RtType::Integer => "Integer",
            RtType::BigInteger => "BigInteger",
            RtType::Decimal => "Decimal",
            RtType::Boolean => "Boolean",
            RtType::Text(_) => "Text",
            RtType::Code(_) => "Code",
            RtType::Date => "Date",
            RtType::Time => "Time",
            RtType::DateTime => "DateTime",
            RtType::Duration => "Duration",
            RtType::Guid => "GUID",
            RtType::DateFormula => "DateFormula",
            RtType::Option(_) | RtType::Enum(_) => "Option",
            // The names are those of recordref/TestPlatformOptionsLockingContracts.al:46-63 (13 is
            // BLOB) and :66-76 (RecordID, TableFilter, MediaSet name themselves).
            RtType::Library(n) => match n.to_ascii_lowercase().as_str() {
                "blob" => "BLOB",
                "recordid" => "RecordID",
                "tablefilter" => "TableFilter",
                "media" => "Media",
                "mediaset" => "MediaSet",
                _ => return unsupported(format!("FieldRef.Type of a {} field", t.fields[pos].ty.describe())),
            },
            other => return unsupported(format!("FieldRef.Type of a {} field", other.describe())),
        };
        let members = al_builtins::sysenum::option_members("FieldType").expect("known");
        let ord = members.iter().position(|m| m == name).expect("listed") as i32;
        Ok(Value::Option(ord, members.into()))
    }

    /// A field property as written (`OptionCaption`, ...), if declared.
    pub(crate) fn field_property(&self, t: &TableDef, pos: usize, name: &str) -> Option<String> {
        self.engine.program.objects[t.object as usize]
            .table
            .as_ref()?
            .fields
            .get(pos)?
            .properties
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.clone())
    }

    /// The members of an Option or Enum field as (ordinal, name, caption), in declared order.
    pub(crate) fn members_of_field(&self, t: &TableDef, pos: usize) -> Result<Option<FieldMembers>, Signal> {
        self.members_of_field_in(t, pos, false)
    }

    /// [`Self::members_of_field`]; with `session_language`, a field with OptionCaptionML takes the captions of the session
    /// language (record/lt-x-table-props/XTablePropsTests.al:175-232, `FieldRef.OptionCaption`), else it is refused: what the
    /// other readers (reports, XMLports, filters, TestField) show for it is not pinned.
    pub(crate) fn members_of_field_in(
        &self,
        t: &TableDef,
        pos: usize,
        session_language: bool,
    ) -> Result<Option<FieldMembers>, Signal> {
        match &t.fields[pos].ty {
            RtType::Enum(id) => {
                self.note_enum(*id);
                let o = &self.engine.program.objects[*id as usize];
                let members = o
                    .enum_values
                    .iter()
                    .map(|(ord, name)| {
                        let caption = o
                            .enum_captions
                            .iter()
                            .find(|(c, _)| c == ord)
                            .map_or_else(|| name.clone(), |(_, c)| c.clone());
                        (*ord, name.clone(), caption)
                    })
                    .collect();
                Ok(Some((true, members)))
            }
            RtType::Option(names) => {
                // A table field's OptionCaptionML gives the captions of the session language (record/lt-x-table-props/
                // XTablePropsTests.al:219-232).
                let ml = self.field_property(t, pos, "OptionCaptionML");
                let captions: Vec<String> = match (ml, self.field_property(t, pos, "OptionCaption")) {
                    (Some(ml), _) if session_language => {
                        let language = al_builtins::format::FormatCtx::language(self);
                        al_builtins::format::ml_text_in_language(&ml, language)
                            .map_err(|e| Signal::Unsupported(format!("OptionCaptionML {e}")))?
                            .split(',')
                            .map(str::to_string)
                            .collect()
                    }
                    (Some(_), _) => {
                        return unsupported(format!(
                            "OptionCaptionML of option field {} read here (not pinned for this reader)",
                            t.fields[pos].name
                        ));
                    }
                    (None, Some(v)) => match crate::recmeta::string_literal(&v) {
                        Some(list) => list.split(',').map(str::to_string).collect(),
                        None => return unsupported(format!("OptionCaption `{v}`")),
                    },
                    (None, None) => names.clone(),
                };
                if captions.len() != names.len() {
                    return unsupported(
                        "an OptionCaption list as long as the OptionMembers list is not what this field has",
                    );
                }
                let members = names
                    .iter()
                    .zip(captions)
                    .enumerate()
                    .map(|(i, (n, c))| (i as i32, n.clone(), c))
                    .collect();
                Ok(Some((false, members)))
            }
            _ => Ok(None),
        }
    }

    /// `IsEnum`, `EnumValueCount`, `GetEnumValue*`, `OptionCaption/Members/String` of a FieldRef.
    /// Corpus: fieldref/TestFieldRefEnumOrdinalByIndex.al (index is a 1-based position, out of range
    /// answers -1 / ''), TestFieldRefEnum.al, TestPrecompiledFieldOptionCaption.al:34-48.
    fn enum_metadata(&self, method: &str, t: &TableDef, pos: usize, args: &[Value]) -> Result<Value, Signal> {
        // OptionCaption and OptionMembers of a field with OptionCaptionML are pinned (XTablePropsTests.al:192-196); the other methods
        // are not.
        let session_language = matches!(method, "OptionCaption" | "OptionMembers");
        let Some((is_enum, members)) = self.members_of_field_in(t, pos, session_language)? else {
            return match method {
                "IsEnum" => Ok(Value::Boolean(false)),
                "EnumValueCount" => Ok(Value::Integer(0)),
                // An empty text, not an error (corpus recordref/TestRecordRefRuntimeContracts.al:
                // FieldRef_EnumMetadata_OnEnumOptionAndOtherFields).
                "OptionCaption" | "OptionMembers" | "OptionString" => Ok(Value::Text(String::new().into())),
                other => unsupported(format!("FieldRef.{other} on a field that is neither Option nor Enum")),
            };
        };
        let at = |args: &[Value]| -> Result<Option<&(i32, String, String)>, Signal> {
            let i = int_arg(args, 0)?;
            Ok(usize::try_from(i - 1).ok().and_then(|i| members.get(i)))
        };
        let by_ordinal = |args: &[Value]| -> Result<&(i32, String, String), Signal> {
            let o = int_arg(args, 0)?;
            members.iter().find(|m| i64::from(m.0) == o).map_or_else(
                || unsupported(format!("{method}({o}) for an ordinal the field does not have")),
                Ok,
            )
        };
        match method {
            "IsEnum" => Ok(Value::Boolean(is_enum)),
            "EnumValueCount" => Ok(Value::Integer(members.len() as i32)),
            "GetEnumValueOrdinal" => Ok(Value::Integer(at(args)?.map_or(-1, |m| m.0))),
            "GetEnumValueName" => Ok(Value::Text(at(args)?.map(|m| m.1.clone()).unwrap_or_default().into())),
            "GetEnumValueCaption" => Ok(Value::Text(at(args)?.map(|m| m.2.clone()).unwrap_or_default().into())),
            "GetEnumValueNameFromOrdinalValue" => Ok(Value::Text(by_ordinal(args)?.1.clone().into())),
            "GetEnumValueCaptionFromOrdinalValue" => Ok(Value::Text(by_ordinal(args)?.2.clone().into())),
            // An Enum field answers the names and the captions of its members like an Option (record/
            // TestProbeRecordRefContracts.al:FieldRef_EnumAndOptionInfo).
            "OptionMembers" | "OptionString" => Ok(Value::Text(
                members
                    .iter()
                    .map(|m| m.1.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
                    .into(),
            )),
            "OptionCaption" => Ok(Value::Text(
                members
                    .iter()
                    .map(|m| m.2.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
                    .into(),
            )),
            other => unsupported(format!("FieldRef.{other}")),
        }
    }
}

/// The record method a RecordRef method forwards to; the RecordRef arms that forward are listed by name.
fn record_method(method: &str) -> RecordOp {
    RecordOp::from_name(method).expect("a RecordRef method that forwards to the record")
}
