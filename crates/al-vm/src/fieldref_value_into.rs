//! `FieldRef.Value := V`: the value converted into the field's type, or the error BC raises (moved out of `refs.rs`).

use crate::Session;
use al_builtins::native::Signal;
use al_builtins::ops;
use al_builtins::value::{RtType, Value};
use al_data::TableDef;

fn unsupported<T>(what: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(what.into()))
}

impl Session {
    /// The value `v` for the field at `pos` of type `ty`: a Variant of a kind the field does not take converts or raises as
    /// `fieldref_variant_cells` says (`variant_into_field_cells`); a text that is no number raises its Evaluate error; a value into
    /// a Text or Code field is its Format text; anything else converts as an assignment does.
    pub(crate) fn fieldref_value_into(
        &self,
        t: &TableDef,
        pos: usize,
        fty: &RtType,
        v: &Value,
    ) -> Result<Value, Signal> {
        if let Some(r) = self.variant_into_field_cells(t, pos, v, false)? {
            return r;
        }
        // A text that is no number into an Integer, BigInteger, Decimal or Boolean field raises the Evaluate error from BC 28 and
        // `The type Nav<Type> is unknown.` before; the field keeps its value (fieldref/TestFieldRefValue.al:202
        // FieldRef_Value_TextIntoInteger_ErrorText, :229 FieldRef_Value_TextIntoNumberTypes): `abc` into each type, an
        // empty text into an Integer.
        if let Value::Text(s) | Value::Code(s) = v {
            let s = s.as_str();
            let letters = s.len() > 1 && s.bytes().all(|b| b.is_ascii_alphabetic());
            let (ty, nav, empty_pinned) = match fty {
                RtType::Integer => ("Integer", "NavInteger", true),
                RtType::BigInteger => ("BigInteger", "NavBigInteger", false),
                RtType::Decimal => ("Decimal", "NavDecimal", false),
                // `abc` and `true` into a Boolean field (TestFieldRefValue.al:282
                // FieldRef_Value_TextIntoBooleanDateGuidOption).
                RtType::Boolean => ("Boolean", "NavBoolean", false),
                // `abc` into an Option field that has no such member (same test; the split by version is
                // record/lt-u-table-field-props/UTableCdsOrdinals.al:170,175).
                RtType::Option(m) if !m.iter().any(|x| x.eq_ignore_ascii_case(s)) => ("Option", "NavOption", false),
                _ => ("", "", false),
            };
            // A text of letters into a Date field: the date format error with code Evaluate:Conversion from BC 28,
            // a conversion error before (TestFieldRefValue.al:282 FieldRef_Value_TextIntoBooleanDateGuidOption,
            // `Date Field := abc`); the field keeps its value.
            if letters && matches!(fty, RtType::Date) {
                let (message, code) = if self.has(al_version::Gate::FieldRefTextNumberEvaluateErrorFrom28) {
                    (
                        format!(
                            "The format of the Date '{s}' does not match your device's date settings.\\\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format 'yyyy-mm-dd'."
                        ),
                        "Evaluate:Conversion",
                    )
                } else {
                    (
                        "Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavText to System.DateTime.".to_string(),
                        "Conversion",
                    )
                };
                return Err(al_builtins::native::Signal::Error(Box::new(
                    al_builtins::native::AlError {
                        message,
                        code: code.into(),
                        ..Default::default()
                    },
                )));
            }
            if !ty.is_empty() && (letters || s.is_empty()) {
                if s.is_empty() && !empty_pinned {
                    return unsupported(format!("FieldRef.Value := '' on a {ty} field (not pinned)"));
                }
                let gate = if ty == "Option" {
                    al_version::Gate::OptionFieldRefSetEvaluateErrorFrom28
                } else {
                    al_version::Gate::FieldRefTextNumberEvaluateErrorFrom28
                };
                let (message, code) = if self.has(gate) {
                    (
                        format!("The value \"{s}\" can't be evaluated into type {ty}."),
                        "Evaluate:Format",
                    )
                } else {
                    (format!("The type {nav} is unknown."), "NotSupportedType")
                };
                return Err(al_builtins::native::Signal::Error(Box::new(
                    al_builtins::native::AlError {
                        message,
                        code: code.into(),
                        ..Default::default()
                    },
                )));
            }
        }
        match al_builtins::fieldref_value_text::variant_into_text_field(v, fty, self) {
            Some(r) => r,
            None => ops::convert(v.clone(), fty),
        }
    }
}
