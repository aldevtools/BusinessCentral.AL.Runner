//! The standard format numbers of a RecordId: 0 and 1 format the key values as format 0 does, 2 and 9 with that number, and 3
//! to 8 do not exist (types/lt-implicit-conversions/P79RecordId.al:183-200 TryFmt, :330-346 CodeKey_FormatNumbers and
//! DateKey_FormatNumbers). 10 is the bookmark ([`bookmark_text`]). Any other number is not pinned and refused.

use super::{FormatCtx, FormatFail, no_such_format};

/// The format number the key values of a RecordId are formatted with, for the RecordId's format number `standard`.
pub(super) fn recordid_key_standard(standard: i32) -> Result<i32, FormatFail> {
    match standard {
        0 | 1 => Ok(0),
        2 | 9 => Ok(standard),
        3..=8 => Err(no_such_format(standard, "RecordID")),
        _ => Err(FormatFail::Unsupported(format!(
            "Format of a RecordId with standard format {standard} (not pinned)"
        ))),
    }
}

/// `Format(RecordId, 0, 10)`: the bookmark of the key as GetUrl writes it, unescaped (`17;D2sBAAJ7/0EALQAx`; the blank RecordId
/// of a Code key `11;D2sBAAJ7`; types/lt-y49-recordid-format10/Y49RecordIdFormat10.al:117-148, `crate::bookmark`). A RecordId
/// that names no table, or a key of another shape, is refused.
pub(super) fn bookmark_text(rid: &crate::value::LibraryValue, ctx: &dyn FormatCtx) -> Result<String, FormatFail> {
    let crate::value::LibraryValue::RecordId {
        table, table_no, key, ..
    } = rid
    else {
        return Err(FormatFail::Unsupported(
            "Format(RecordId, 0, 10) of a RecordId that names no table (not pinned)".into(),
        ));
    };
    let types = ctx
        .record_id_key_types(*table)
        .ok_or_else(|| FormatFail::Unsupported("Format(RecordId, 0, 10) without the table's key".into()))?;
    let shape: Vec<&crate::value::RtType> = types.iter().collect();
    if !crate::bookmark::record_id_shape_pinned(&shape) || key.len() != shape.len() {
        return Err(FormatFail::Unsupported(
            "Format(RecordId, 0, 10) of a key of a shape the corpus does not pin".into(),
        ));
    }
    let values: Vec<&crate::value::Value> = key.iter().collect();
    let (size, data) = crate::bookmark::encode(*table_no, &shape, &values).map_err(FormatFail::Unsupported)?;
    Ok(format!("{size};{}", crate::platform_native::b64_encode(&data, false)))
}
