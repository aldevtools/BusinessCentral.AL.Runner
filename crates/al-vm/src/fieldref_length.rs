//! `FieldRef.Length`: the length a field reports by its type.
use al_builtins::describe::Describe;
use al_builtins::native::Signal;
use al_builtins::value::{RtType, Value};

pub(crate) fn length(ty: &RtType) -> Result<Value, Signal> {
    match ty {
        RtType::Text(Some(n)) | RtType::Code(Some(n)) => Ok(Value::Integer(*n as i32)),
        // fieldref/lt-fieldref-length-kinds/FieldRefLengthKinds.al:78-126: Option and Enum 4 whatever the
        // member names, Integer 4, Decimal 12, Date 4, Boolean 4, Guid 16, BigInteger 8, Blob 8, Time 4,
        // Duration 8, DateTime 8. Text and Code without a length are not pinned.
        RtType::Option(_) | RtType::Enum(_) | RtType::Integer | RtType::Date | RtType::Boolean | RtType::Time => {
            Ok(Value::Integer(4))
        }
        RtType::Decimal => Ok(Value::Integer(12)),
        RtType::Guid => Ok(Value::Integer(16)),
        RtType::BigInteger | RtType::Duration | RtType::DateTime => Ok(Value::Integer(8)),
        RtType::Library(al_builtins::library_kind::LibraryKind::Blob) => Ok(Value::Integer(8)),
        // record/lt-media-length-find-empty/MediaLengthFindEmpty.al:56-75: Media 16, MediaSet 16.
        RtType::Library(
            al_builtins::library_kind::LibraryKind::Media | al_builtins::library_kind::LibraryKind::MediaSet,
        ) => Ok(Value::Integer(16)),
        // The Security Filter columns of Permission and Metadata Permission (permissions/W6PermissionColumns.al:82,119).
        RtType::Library(k) if k.as_str() == "TableFilter" => Ok(Value::Integer(504)),
        // DateFormula 32, RecordId 448 (al-runner issue 6663; awaiting Y66FieldRefLengthTypes.al:61).
        RtType::DateFormula | RtType::Library(al_builtins::library_kind::LibraryKind::RecordId) => {
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::FieldRefLengthDateFormulaRecordId);
            Ok(Value::Integer(if matches!(ty, RtType::DateFormula) { 32 } else { 448 }))
        }
        other => Err(Signal::Unsupported(format!(
            "FieldRef.Length of a {} field",
            other.describe()
        ))),
    }
}
