//! The standard format numbers of a RecordId: 0 and 1 format the key values as format 0 does, 2 and 9 with that number, and 3
//! to 8 do not exist (types/lt-implicit-conversions/P79RecordId.al:183-200 TryFmt, :330-346 CodeKey_FormatNumbers and
//! DateKey_FormatNumbers). Any other number is not pinned and refused: 10, the bookmark form Microsoft's Page Summary Provider uses,
//! may give another text altogether (gap G5-10).

use super::{FormatFail, no_such_format};

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
