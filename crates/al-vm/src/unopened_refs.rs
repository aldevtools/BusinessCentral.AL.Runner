//! Members of a RecordRef that is not open and of a KeyRef never assigned, where corpus PR 1408 measured them
//! (tests/al-language/recordref-fieldref/UhUnopenedRecordRef.al, BC 27.5, 28.5 and 29.0; al-runner issues 7760, 7764, 8084,
//! 8001). The probe is merged and not yet pinned: every path here marks `recordref-not-open-members`,
//! `recordref-gettable-variant-not-record` or `keyref-unassigned-members`, except the cells the pinned corpus already measured
//! (cited where they are answered). Corpus PR 1411 (streams/instream-outstream/UhUntouchedStreamsAfter.al, codeunit 66996, merged,
//! not yet pinned) measured some of them after `Open(0)` and after `Open` then `Clear`. Every other shape keeps the answer of its
//! owner in `refs.rs` (which refuses it).

use crate::Session;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::native::{AlError, Signal};
use al_builtins::recordref_closed::not_open;
use al_builtins::value::{EmptiedRef, LibraryValue, RecordState, Value};

/// "The record is not open." (code RecordNotOpened), the error of a RecordRef that is not open.
pub(crate) fn record_not_open() -> Signal {
    Signal::Error(Box::new(AlError {
        message: "The record is not open.".into(),
        code: "RecordNotOpened".into(),
        call_stack: String::new(),
        ..Default::default()
    }))
}

/// Which states of a RecordRef that is not open the probe measured a member in.
#[derive(PartialEq)]
enum Measured {
    /// Never opened (`Empty`) and opened then closed (`ClosedRecordRef`).
    NeverOpenedAndClosed,
    /// Never opened only (`Empty`).
    NeverOpened,
}

/// The members of a RecordRef that is not open that UhUnopenedRecordRef.al measured raising "The record is not open.", with the
/// call form and the arguments of the probe. `id` is the overload the call site picked, `used` whether the result is used. Each
/// line cites its step and its tests.
fn measured_member(id: &str, method: &str, args: &[Value], used: bool) -> Option<Measured> {
    use Measured::*;
    let none = args.is_empty();
    let one = matches!(args, [Value::Integer(1)]);
    match method {
        // :48-51 `u_copylinks`, :168-172 `c_copylinks` (an open RecordRef as the source), :213-224 `u_copylinksrec`,
        // `c_copylinksrec` (a Record variable); tests :277, :577, :667, :677. A statement.
        "CopyLinks" if !used && copy_links_source_measured(id, args) => Some(NeverOpenedAndClosed),
        // :52-71 and :173-197 (`u_`/`c_` findset, readpermission, writepermission, fieldcount, findfirst); tests :287-335,
        // :587-635. The result is used.
        "FindSet" | "ReadPermission" | "WritePermission" | "FieldCount" | "FindFirst" if used && none => {
            Some(NeverOpenedAndClosed)
        }
        // :72-75 `u_findlast`, :84-87 `u_getfilters`, :156-159 `u_next`; tests :337, :367, :547. The result is used.
        "FindLast" | "GetFilters" | "Next" if used && none => Some(NeverOpened),
        // :88-91 `u_keyindex` (KR := RR.KeyIndex(1)); test :377.
        "KeyIndex" if used && one => Some(NeverOpened),
        // :80-83 `u_setrecfilter`, :92-95 `u_setloadfields` (SetLoadFields(1)); tests :357, :387. Statements.
        "SetRecFilter" if !used && none => Some(NeverOpened),
        "SetLoadFields" if !used && one => Some(NeverOpened),
        _ => None,
    }
}

/// The sources of `CopyLinks` the probe passed (UhUnopenedRecordRef.al:37-46): the Record variable `Rec` and the RecordRef `Src`
/// just opened (`Src.Open(66990)`), each not temporary, with no filter, standing on no row and with nothing written to its buffer
/// ([`fresh_source`]). `id` is the overload the call site picked: a Variant argument picks the Variant one (`#3`), which the probe
/// did not pass; a Record or RecordRef argument picks a typed one (`#1` Record, `#2` RecordRef).
fn copy_links_source_measured(id: &str, args: &[Value]) -> bool {
    if !matches!(id, "RecordRef.CopyLinks#1" | "RecordRef.CopyLinks#2") {
        return false;
    }
    match args {
        [Value::Record(r)] => fresh_source(&r.borrow()),
        [Value::Library(_, l)] => matches!(&*l.borrow(), LibraryValue::RecordRef(r) if fresh_source(&r.borrow())),
        _ => false,
    }
}

/// The state of the probe's sources: a declared Record and a RecordRef right after `Open`, in the session's company.
fn fresh_source(r: &RecordState) -> bool {
    !r.temporary
        && r.company.is_none()
        && r.filters.is_empty()
        && r.marks.is_empty()
        && r.position.is_none()
        && r.row_origin.is_none()
        && !r.touched
}

impl Session {
    /// `Some` with "The record is not open." when `method` (by the overload `id`) on the RecordRef `lib`, which is not open, is a
    /// measured cell ([`measured_member`]) in the state the probe measured it in; `None` to go on to `refs.rs` `recordref_op`.
    pub(crate) fn recordref_not_open_measured(
        &self,
        id: &str,
        method: &str,
        lib: &LibraryValue,
        args: &[Value],
    ) -> Option<Signal> {
        if !not_open(lib) {
            return None;
        }
        let measured = measured_member(id, method, args, self.ret_used)?;
        let state_measured = match lib {
            LibraryValue::Empty => true,
            LibraryValue::ClosedRecordRef => measured == Measured::NeverOpenedAndClosed,
            // UhUntouchedStreamsAfter.al:353-379 AfterOpenZero_Members and AfterOpenClear_Members (setups :219-225, members
            // :228-248): exactly the never-opened-only members raise there.
            LibraryValue::EmptiedRef(EmptiedRef::ClearedOpenRecordRef) => measured == Measured::NeverOpened,
            // And FieldCount after Open(0), pinned below.
            LibraryValue::EmptiedRef(EmptiedRef::ZeroOpenedRecordRef) => {
                measured == Measured::NeverOpened || method == "FieldCount"
            }
            _ => false,
        };
        if !state_measured {
            return None;
        }
        // FieldCount of a RecordRef never opened, with the result used, is pinned (recordref-fieldref/recordref/RfTOpen.al:191,
        // Closed_Errors `fieldcount`, a RecordRef that no Open reached; :630 Open_Zero_Members, after `Open(0)`).
        let pinned = method == "FieldCount"
            && matches!(
                lib,
                LibraryValue::Empty | LibraryValue::EmptiedRef(EmptiedRef::ZeroOpenedRecordRef)
            );
        if !pinned {
            assume(Assumption::RecordRefNotOpenMembers);
        }
        Some(record_not_open())
    }

    /// `Some` with the error of the statement `RecordRef.GetTable(V)` when the Variant `V` holds no record and the shape is
    /// measured; `None` to go on. Pinned (recordref-fieldref/recordref/GetTableVariantKinds.al): `V` holds nothing, on a RecordRef
    /// never opened (:79-116: never assigned, a copy, cleared, a parameter) and on one open on a table that is not temporary
    /// (:154-172, it stays open), "NavVariant variable not initialized."; `V` holds the Integer 5 or the Text 'ALT Universal' on
    /// a RecordRef never opened (:119-144), the Conversion error naming NavInteger or NavText. Assumed
    /// (`recordref-gettable-variant-not-record`): the Text '' or 'abc' on a RecordRef never opened (UhUnopenedRecordRef.al:225-230
    /// `gt_empty`, :242-247 `gt_text`; tests :687, :717), the Conversion error naming NavText. A RecordRef emptied by `Clear` or
    /// `Open(0)` is not one never opened: no probe measured GetTable there.
    pub(crate) fn get_table_variant_not_record(&self, lib: &LibraryValue, arg: Option<&Value>) -> Option<Signal> {
        if self.ret_used {
            return None;
        }
        let never_opened = matches!(lib, LibraryValue::Empty);
        let open_stored = matches!(lib, LibraryValue::RecordRef(r) if !r.borrow().temporary);
        let conversion = |from: &str| {
            crate::transactions::al_error_code(
                &format!(
                    "Unable to convert from Microsoft.Dynamics.Nav.Runtime.{from} to Microsoft.Dynamics.Nav.Runtime.INavRecordHandle."
                ),
                "Conversion",
            )
        };
        match arg? {
            Value::Null if never_opened || open_stored => Some(crate::recordref_variant::variant_not_initialized()),
            Value::Integer(5) if never_opened => Some(conversion("NavInteger")),
            Value::Text(t) if never_opened && t.as_str() == "ALT Universal" => Some(conversion("NavText")),
            Value::Text(t) if never_opened && (t.as_str() == "" || t.as_str() == "abc") => {
                assume(Assumption::RecordRefGetTableVariantNotRecord);
                Some(conversion("NavText"))
            }
            _ => None,
        }
    }

    /// `Some` with "Microsoft.Dynamics.Nav.Runtime.NavKeyRef variable not initialized." (NotInitialized) for the members of a
    /// KeyRef nothing assigned that UhUnopenedRecordRef.al measured, each with its result used: FieldCount and Active
    /// (:248-251 `kr_fieldcount`, `kr_active`; tests :727, :737), FieldIndex(1) (:252-256, test :747) and Record (:257-261,
    /// test :757); and on a KeyRef that held a key, after `Clear`, Active with its result used (UhUntouchedStreamsAfter.al:285-299
    /// `TryKeyRef`, test :418; FieldCount there is observed, not asserted, :419). `None` to go on.
    pub(crate) fn keyref_unassigned_measured(
        &self,
        method: &str,
        lib: &LibraryValue,
        args: &[Value],
    ) -> Option<Signal> {
        if !self.ret_used {
            return None;
        }
        let measured = match (lib, method) {
            (LibraryValue::Empty, "FieldCount" | "Active" | "Record") => args.is_empty(),
            (LibraryValue::Empty, "FieldIndex") => matches!(args, [Value::Integer(1)]),
            (LibraryValue::EmptiedRef(EmptiedRef::ClearedAssignedKeyRef), "Active") => args.is_empty(),
            _ => false,
        };
        if !measured {
            return None;
        }
        assume(Assumption::KeyRefUnassignedMembers);
        Some(crate::transactions::al_error_code(
            "Microsoft.Dynamics.Nav.Runtime.NavKeyRef variable not initialized.",
            "NotInitialized",
        ))
    }
}

/// Whether `method` on the RecordRef `lib` is an `Open` or a `Get` on a RecordRef never opened or emptied ([`after_open_attempt`]).
pub(crate) fn open_attempt_on_new_ref(method: &str, lib: &LibraryValue) -> bool {
    matches!(method, "Open" | "Get") && matches!(lib, LibraryValue::Empty | LibraryValue::EmptiedRef(_))
}

/// After an `Open` that raised, or a `Get` that left the ref not open (a miss, a blank RecordId, an error), on a RecordRef never
/// opened: the ref counts as closed. Both answer alike everywhere except the members the probe measured only on a new variable
/// ([`measured_member`], `Measured::NeverOpened`), which then keep their answer before the probe (a refusal). An `Open(0)` that
/// raised nothing on a RecordRef never opened leaves `EmptiedRef::ZeroOpenedRecordRef`: it answers as one never opened
/// ([`al_builtins::recordref_closed::not_open`]; Open_Zero_Members, recordref-fieldref/recordref/RfTOpen.al:584-641, Number, Name,
/// FieldCount, KeyCount, IsEmpty, IsTemporary and Count), and [`measured_member`] runs there what UhUntouchedStreamsAfter.al
/// measured. An `Open` or a `Get` that leaves an emptied RecordRef not open leaves `EmptiedRef::OtherRecordRef`, which no probe
/// measured.
pub(crate) fn after_open_attempt(method: &str, lib: &std::cell::RefCell<LibraryValue>, r: &Result<Value, Signal>) {
    let next = match &*lib.borrow() {
        LibraryValue::Empty if method == "Get" || r.is_err() => LibraryValue::ClosedRecordRef,
        LibraryValue::Empty => LibraryValue::EmptiedRef(EmptiedRef::ZeroOpenedRecordRef),
        LibraryValue::EmptiedRef(_) => LibraryValue::EmptiedRef(EmptiedRef::OtherRecordRef),
        _ => return,
    };
    *lib.borrow_mut() = next;
}

impl Session {
    /// `Get(RecordId)` on a RecordRef not open: opens the table, asks `Get`, and a miss (or an open that fails) leaves the ref as
    /// it was, not open ([`after_open_attempt`] then counts a ref never opened as closed).
    pub(crate) fn get_on_unopened_ref(
        &mut self,
        lib: &std::rc::Rc<std::cell::RefCell<LibraryValue>>,
        table_no: u32,
        args: &mut [Value],
    ) -> Result<Value, Signal> {
        let before = std::mem::take(&mut *lib.borrow_mut());
        if let Err(e) = self.rr_open(lib, &[Value::Integer(table_no as i32)]) {
            *lib.borrow_mut() = before;
            return Err(e);
        }
        let found = self.recordref_op("Get", lib, args);
        if !matches!(found, Ok(Value::Boolean(true))) {
            *lib.borrow_mut() = before;
        }
        found
    }
}
