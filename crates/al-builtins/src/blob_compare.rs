//! `=` and `<>` of two Blob values (`FieldRef.Value` of a Blob field; two Blob fields of records or two Variants do not compile:
//! tests/must-not-compile/cases/2555-blob-field-compare-direct.al, 0502-variant-comparison-operators.al; `<` between two FieldRef
//! values does not either, 2556-fieldref-value-ordered-compare.al).

use crate::native::Signal;
use crate::stream::BlobRef;
use std::cmp::Ordering;

/// What a comparison sees of a Blob in a record variable (recordref/lt-y49-blob-compare/Y49BlobCompare.al, corpus master):
/// - not loaded (read with Get, or the copy a record assignment made; `FieldRef.Value` of such a Blob with content is the
///   `unread_content` stand-in): only whether the row has content. Rows `abc` and `xyz`
///   are equal, `abc` and none differ (:189-233 `1-3`, `1-4`, `4-4`; :120-149 `copy`; :152-187 `temp vs stored`).
/// - loaded (CalcFields, a stream write, a fresh record, TransferFields): the bytes (:189-233 `... calc`; :235-255).
/// - a loaded value against one that is not differs (:120-149 `copy calc rec only`, `copy rec written same text`; :152-187
///   `temp calc only`, `transfer vs stored`; :189-233 `1 calc-3`), pinned only for content on both sides: an empty loaded Blob
///   against a value that is not loaded is refused.
#[derive(PartialEq)]
enum Seen {
    HasContent(bool),
    Bytes(Vec<u8>),
}

fn seen(b: &BlobRef) -> Result<Seen, Signal> {
    let buf = b.borrow();
    if buf.render_pending.is_some() {
        return Err(Signal::Unsupported(
            "comparison of a Blob that holds a rendered report (ADR-040)".into(),
        ));
    }
    Ok(if buf.unread_content {
        Seen::HasContent(true)
    } else if buf.loaded && !buf.refetch {
        Seen::Bytes(buf.bytes.clone())
    } else {
        Seen::HasContent(!buf.bytes.is_empty())
    })
}

/// `Equal` or `Less` (an order of two Blobs does not compile).
pub(crate) fn compare(a: &BlobRef, b: &BlobRef) -> Result<Ordering, Signal> {
    let (x, y) = (seen(a)?, seen(b)?);
    match (&x, &y) {
        (Seen::Bytes(bytes), Seen::HasContent(_)) | (Seen::HasContent(_), Seen::Bytes(bytes)) if bytes.is_empty() => {
            Err(Signal::Unsupported(
                "comparison of an empty loaded Blob with a Blob that is not loaded (not pinned)".into(),
            ))
        }
        _ => Ok(if x == y { Ordering::Equal } else { Ordering::Less }),
    }
}
