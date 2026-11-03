//! `TestField.SetValue` of a value that is not a Text, on a control bound to a Text or Code field or variable (moved out of controls.rs).
//! Pinned by pages/lt-y80-setvalue-types/Y80SetValueTypes.al (needs corpus bump).

use super::*;

/// What a Text or Code control makes of a value that is not a Text.
pub(super) enum TextConv {
    /// The text the control takes.
    Text(String),
    /// The text is longer than the field: the control shows it, and the Close raises the error (`PageState::deferred_overlong`).
    Deferred,
    /// Not handled here: the generic conversion follows.
    Other,
}

/// The length a Text or Code control's field allows.
fn bounded_text_len(ty: &RtType) -> Option<usize> {
    match ty {
        RtType::Text(Some(n)) => Some(*n as usize),
        RtType::Code(Some(n)) => Some(*n as usize),
        _ => None,
    }
}

impl Session {
    /// Notes a SetValue whose text is longer than the field (see `PageState::deferred_overlong`).
    fn defer_overlong(&mut self, st: &St, idx: usize, max: usize, full: &str) -> TextConv {
        let len = Value::text_len(full);
        let message = format!(
            "The length of the string is {len}, but it must be less than or equal to {max} characters. Value: {full}"
        );
        st.borrow_mut().deferred_overlong = Some((idx, full.to_string(), message));
        TextConv::Deferred
    }

    /// The text a Text or Code control takes from `arg`:
    /// - a Decimal is its Format text (1.11, `1,234`, `100,000`; misc/lt-y14-triage/Y14Small.al:237; Y80_ST_Card_TxtCtl_Decimals and
    ///   _CdCtl_Decimals);
    /// - an Integer or BigInteger is its digits, with no thousands separator (Y80_ST_Card_TxtCtl_Integers `int 1234567`,
    ///   _BigIntegers `big 5000000000`; the same on a List and on a Code control). On a control bound to a variable it is unprobed
    ///   (assumption setvalue-whole-number-into-text-control; WhseItemTrackingPageHandler of SCM RTAM Item Tracking-II sets a Decimal);
    /// - a Boolean (`Yes`, `YES` on a Code control), a Duration, an Option and an Enum value are their Format text:
    ///   `1 minute 30 seconds`, `B`, the caption `First Kind` (Y80_ST_Card_TxtCtl_Others, _TxtCtl_Enums, and the _CdCtl_ twins);
    /// - any value whose text is longer than the field is not refused at the SetValue: the control shows it and the save at Close
    ///   raises the error with the code TestWrapped (Y80_ST_*_ShortCtl_*, and a Guid on a Text[30] control in Y80_ST_*_TxtCtl_Others).
    pub(super) fn text_control_value(
        &mut self,
        st: &St,
        idx: usize,
        ty: &RtType,
        arg: &Value,
        rec_bound: bool,
    ) -> Result<TextConv, Signal> {
        if !matches!(ty, RtType::Text(_) | RtType::Code(_)) || matches!(arg, Value::Text(_) | Value::Code(_)) {
            return Ok(TextConv::Other);
        }
        // Pinned on a control bound to a variable, Text and Code: an Integer (14, 1234, -3) and the Decimal 14.0 enter their digits
        // (pages/lt-ao-testpage/AoTests.al:363-404 NumberIntoTextVariable, NumberIntoCodeVariable). A BigInteger and any other
        // Decimal (a fraction, a thousand or more: Format puts a separator in) are not.
        let pinned_number = match arg {
            Value::Integer(_) => true,
            Value::Decimal(d) => d.fract().is_zero() && d.abs() < rust_decimal::Decimal::from(1000),
            _ => false,
        };
        if matches!(arg, Value::Decimal(_) | Value::BigInteger(_)) && !rec_bound && !pinned_number {
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::SetValueWholeNumberIntoTextControl);
        }
        let text = match arg {
            Value::Integer(i) => i.to_string(),
            Value::BigInteger(i) => i.to_string(),
            Value::Decimal(_) | Value::Duration(_) | Value::Option(..) | Value::Enum(..) | Value::Boolean(_) => {
                match al_builtins::format::format_value(arg, 0, 0, &*self) {
                    Ok(t) => t,
                    Err(_) => return unsupported("SetValue on a text control whose Format fails"),
                }
            }
            other => match ops::convert(other.clone(), &RtType::Text(None)) {
                Ok(Value::Text(full)) => {
                    // Dates, times, GUIDs and the like convert the generic way unless they are longer than the field.
                    return Ok(match bounded_text_len(ty) {
                        Some(max) if rec_bound && Value::text_len(&full) > max => {
                            self.defer_overlong(st, idx, max, &full)
                        }
                        _ => TextConv::Other,
                    });
                }
                _ => return Ok(TextConv::Other),
            },
        };
        if rec_bound && let Some(max) = bounded_text_len(ty).filter(|max| Value::text_len(&text) > *max) {
            return Ok(self.defer_overlong(st, idx, max, &text));
        }
        // The Format text goes through the control's own conversion (upper case on a Code control, the length of a variable).
        let converted = ops::convert(Value::Text(text.into()), ty)?;
        Ok(TextConv::Text(self.control_text(st, idx, &converted)?.to_string()))
    }

    /// The text a control shows after a SetValue whose text is longer than the field (`PageState::deferred_overlong`).
    pub(super) fn deferred_value_read(st: &St, idx: usize) -> Option<Value> {
        let held = st.borrow().deferred_overlong.clone()?;
        (held.0 == idx).then(|| Value::Text(held.1.into()))
    }
}
