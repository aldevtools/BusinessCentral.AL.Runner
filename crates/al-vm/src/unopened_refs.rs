//! Members of a RecordRef that is not open and of a KeyRef never assigned, where corpus PR 1408 measured them
//! (tests/al-language/recordref-fieldref/UhUnopenedRecordRef.al, BC 27.5, 28.5 and 29.0; al-runner issues 7760, 7764, 8084,
//! 8001). The probe is merged and not yet pinned: every path here marks `recordref-not-open-members`,
//! `recordref-gettable-variant-not-record` or `keyref-unassigned-members`, except the cells the pinned corpus already measured
//! (cited where they are answered). Every other shape keeps the answer of its owner in `refs.rs` (which refuses it).

use crate::Session;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::native::{AlError, Signal};
use al_builtins::value::{LibraryValue, Value};

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
/// call form and the arguments of the probe. `used` is whether the result is used. Each line cites its step and its tests.
fn measured_member(method: &str, args: &[Value], used: bool) -> Option<Measured> {
    use Measured::*;
    let none = args.is_empty();
    let one = matches!(args, [Value::Integer(1)]);
    match method {
        // :48-51 `u_copylinks`, :168-172 `c_copylinks` (an open RecordRef as the source), :213-224 `u_copylinksrec`,
        // `c_copylinksrec` (a Record variable); tests :277, :577, :667, :677. A statement.
        "CopyLinks" if !used && copy_links_source_measured(args) => Some(NeverOpenedAndClosed),
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

/// The sources of `CopyLinks` the probe passed: a RecordRef open on a table and a Record variable, neither temporary
/// (UhUnopenedRecordRef.al:37-46, `Rec` and `Src.Open(66990)`).
fn copy_links_source_measured(args: &[Value]) -> bool {
    match args {
        [Value::Record(r)] => !r.borrow().temporary,
        [Value::Library(name, l)] if name.eq_ignore_ascii_case("RecordRef") => {
            matches!(&*l.borrow(), LibraryValue::RecordRef(r) if !r.borrow().temporary)
        }
        _ => false,
    }
}

impl Session {
    /// `Some` with "The record is not open." when `method` on the RecordRef `lib`, which is not open, is a measured cell
    /// ([`measured_member`]) in the state the probe measured it in; `None` to go on to `refs.rs` `open_ref`.
    pub(crate) fn recordref_not_open_measured(
        &self,
        method: &str,
        lib: &LibraryValue,
        args: &[Value],
    ) -> Option<Signal> {
        let measured = measured_member(method, args, self.ret_used)?;
        let state_measured = match lib {
            LibraryValue::Empty => true,
            LibraryValue::ClosedRecordRef => measured == Measured::NeverOpenedAndClosed,
            _ => false,
        };
        if !state_measured {
            return None;
        }
        // FieldCount of a RecordRef never opened, with the result used, is pinned (recordref-fieldref/recordref/RfTOpen.al:191,
        // Closed_Errors `fieldcount`, a RecordRef that no Open reached).
        let pinned = method == "FieldCount" && matches!(lib, LibraryValue::Empty);
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
    /// `gt_empty`, :242-247 `gt_text`; tests :687, :717), the Conversion error naming NavText.
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
            Value::Null if never_opened || open_stored => Some(crate::transactions::al_error_code(
                "Microsoft.Dynamics.Nav.Runtime.NavVariant variable not initialized.",
                "NotInitialized",
            )),
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
    /// test :757). `None` to go on.
    pub(crate) fn keyref_unassigned_measured(
        &self,
        method: &str,
        lib: &LibraryValue,
        args: &[Value],
    ) -> Option<Signal> {
        if !matches!(lib, LibraryValue::Empty) || !self.ret_used {
            return None;
        }
        let measured = match method {
            "FieldCount" | "Active" | "Record" => args.is_empty(),
            "FieldIndex" => matches!(args, [Value::Integer(1)]),
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
