//! The filter a Record's `SetRange` sets.

use super::*;

/// The range `SetRange` sets for the values after the field (`bounds`), converted to the field type `ty`; `None` when no value
/// is given (the field's filter is only removed). One value is the range from it to itself. Two Guids on a Media or MediaSet
/// field order them, which is not pinned (`al_builtins::filter::media_guid_order`): refused.
pub(crate) fn setrange_filter(ty: &RtType, bounds: &[Value]) -> Result<Option<FilterExpr>, Signal> {
    let Some(from) = bounds.first() else {
        return Ok(None);
    };
    let from = ops::convert(from.clone(), ty)?;
    let to = match bounds.get(1) {
        Some(v) => ops::convert(v.clone(), ty)?,
        None => from.clone(),
    };
    let range = FilterExpr::Range(Some(from), Some(to));
    if bounds.len() > 1 && al_builtins::filter::media_guid_order(ty, &range) {
        return Err(Signal::unsupported(
            "SetRange with two Guids on a Media or MediaSet field (the order of Guids is not pinned)",
        ));
    }
    Ok(Some(range))
}
