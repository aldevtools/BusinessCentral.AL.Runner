//! A Variant holding a RecordId, a Record or a RecordRef, or going into a RecordId, Record or RecordRef slot: the errors BC
//! raises, from types/variant/VariantRecordIdConversions.al (the same on BC 27.5, 28.5 and 29.0). The sources measured are a
//! RecordId, a Record, a RecordRef, a Guid, a Code, a Text and an Integer; any other pair goes on to the conversion that
//! follows, and a conversion the measurement shows to work (a RecordId into a RecordId, a Record or RecordRef into a Record,
//! a RecordRef into a RecordRef) goes on too. types/variant/VaTConvert.al pins the same texts for an Integer and a Record in a
//! Variant (`int_to_record`, `record_to_recordref`, `record_to_int`, `record_to_text`).

use crate::library_kind::LibraryKind;
use crate::native::{Signal, raise};
use crate::value::{LibraryValue, RtType, Value};

/// The source kinds the probe measured.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Src {
    RecordId,
    Record,
    RecordRef,
    Guid,
    Code,
    Text,
    Integer,
}

impl Src {
    fn of(v: &Value) -> Option<Src> {
        Some(match v {
            Value::Record(_) => Src::Record,
            Value::Guid(_) => Src::Guid,
            Value::Code(_) => Src::Code,
            Value::Text(_) => Src::Text,
            Value::Integer(_) => Src::Integer,
            Value::Library(name, h) if name.eq_ignore_ascii_case("RecordRef") => {
                matches!(&*h.borrow(), LibraryValue::RecordRef(_)).then_some(Src::RecordRef)?
            }
            Value::Library(_, h) if matches!(&*h.borrow(), LibraryValue::RecordId { .. }) => Src::RecordId,
            _ => return None,
        })
    }

    /// The .NET type BC names in a Conversion error.
    fn nav(self) -> &'static str {
        match self {
            Src::RecordId => "Microsoft.Dynamics.Nav.Runtime.NavRecordId",
            Src::Record => "Microsoft.Dynamics.Nav.Runtime.NavRecordHandle",
            Src::RecordRef => "Microsoft.Dynamics.Nav.Runtime.NavRecordRef",
            Src::Guid => "Microsoft.Dynamics.Nav.Runtime.NavGuid",
            Src::Code => "Microsoft.Dynamics.Nav.Runtime.NavCode",
            Src::Text => "Microsoft.Dynamics.Nav.Runtime.NavText",
            Src::Integer => "Microsoft.Dynamics.Nav.Runtime.NavInteger",
        }
    }

    fn is_record_kind(self) -> bool {
        matches!(self, Src::RecordId | Src::Record | Src::RecordRef)
    }
}

/// The codes `Variant.Into#1` takes for a RecordId, a RecordRef and a Record slot, after the matrix kinds (`kind_code`).
const RECORD_ID: i32 = 100;
const RECORD_REF: i32 = 101;
pub(super) const RECORD: i32 = 102;

/// The number `Variant.Into#1` takes for a RecordId, RecordRef or Record slot, `None` for another type.
pub fn record_slot_code(t: &RtType) -> Option<i32> {
    match t {
        RtType::Library(LibraryKind::RecordId) => Some(RECORD_ID),
        RtType::Library(LibraryKind::RecordRef) => Some(RECORD_REF),
        RtType::Record(..) => Some(RECORD),
        _ => None,
    }
}

fn type_conversion(nav: &str) -> Result<Value, Signal> {
    raise(
        format!(
            "The expression Microsoft.Dynamics.Nav.Runtime.NavIndirectValue cannot be type-converted to a Microsoft.Dynamics.Nav.Runtime.{nav} value."
        ),
        "TypeConversion",
    )
}

fn conversion(src: Src, to: &str) -> Result<Value, Signal> {
    raise(format!("Unable to convert from {} to {to}.", src.nav()), "Conversion")
}

/// A Variant holding `v`, into the slot `code` names (one of [`record_slot_code`]): the measured error, or `v` handed on.
pub(super) fn into_record_slot(v: Value, code: i32) -> Result<Value, Signal> {
    let Some(src) = Src::of(&v) else {
        return Ok(v);
    };
    match (code, src) {
        // VariantRecordIdConversions.al:VariantRecordId_EveryTarget `recordid_to_recordid` ok; the others into a RecordId
        // (VariantRecord_ `record_to_recordid`, VariantRecordRef_, VariantGuid_, VariantScalars_ToRecordTypes `*_to_recordid`).
        (RECORD_ID, Src::RecordId) => Ok(v),
        (RECORD_ID, _) => type_conversion("NavRecordId"),
        // `recordref_to_recordref` ok; the others into a RecordRef, a Record among them (`record_to_recordref`).
        (RECORD_REF, Src::RecordRef) => Ok(v),
        (RECORD_REF, _) => type_conversion("NavRecordRef"),
        // `record_to_record` and `recordref_to_record` ok (of the same table); the others into a Record.
        (RECORD, Src::Record | Src::RecordRef) => Ok(v),
        (RECORD, _) => conversion(src, "Microsoft.Dynamics.Nav.Runtime.INavRecordHandle"),
        _ => Ok(v),
    }
}

/// A Variant holding a RecordId, Record or RecordRef `v`, into a Code, Text, Integer or Guid slot `t`: the measured error
/// (`recordid_to_code`, `_to_text`, `_to_integer`, `_to_guid`, and the same for `record_` and `recordref_`), else `None`.
pub(super) fn record_into_scalar(v: &Value, t: &RtType) -> Option<Result<Value, Signal>> {
    let src = Src::of(v).filter(|s| s.is_record_kind())?;
    Some(match t {
        RtType::Code(_) => type_conversion("NavCode"),
        RtType::Text(_) => type_conversion("NavText"),
        RtType::Integer => conversion(src, "System.Int32"),
        RtType::Guid => raise(
            "The expression Variant cannot be type-converted to a GUID value.",
            "TypeConversion",
        ),
        _ => return None,
    })
}
