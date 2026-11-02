//! `FieldRef.Validate(Value)`.

use crate::Session;
use al_builtins::native::{AlError, Signal};
use al_builtins::record_op::RecordOp;
use al_builtins::value::{RecordState, RtType, Value};
use al_data::TableDef;
use std::cell::RefCell;
use std::rc::Rc;

impl Session {
    /// `FieldRef.Validate` of the field at `pos`: validates the record's field. An Enum field given the caption of a member that
    /// is no member name raises the Evaluate error of an Option from BC 28 and `The type NavOption is unknown.` before, and keeps
    /// its value (text/lt-y49-fieldref-into-text/Y49FieldRefIntoText.al:271-317, `fieldref validate text`). Another text into an
    /// Enum field is not pinned and is refused where it is converted.
    pub(crate) fn fieldref_validate(
        &mut self,
        rec: &Rc<RefCell<RecordState>>,
        t: &TableDef,
        pos: usize,
        args: &[Value],
    ) -> Result<Value, Signal> {
        if let (RtType::Enum(_), Some(Value::Text(s) | Value::Code(s))) = (&t.fields[pos].ty, args.first()) {
            let s = s.as_str();
            if let Some((_, members)) = self.members_of_field(t, pos)? {
                let is_name = members.iter().any(|(_, name, _)| name.eq_ignore_ascii_case(s));
                if !is_name && members.iter().any(|(_, _, caption)| caption == s) {
                    let (message, code) = if self.has(al_version::Gate::EnumFieldRefValidateCaptionEvaluateErrorFrom28)
                    {
                        (
                            format!("The value \"{s}\" can't be evaluated into type Option."),
                            "Evaluate:Format",
                        )
                    } else {
                        ("The type NavOption is unknown.".to_string(), "NotSupportedType")
                    };
                    return Err(Signal::Error(Box::new(AlError {
                        message,
                        code: code.into(),
                        ..Default::default()
                    })));
                }
            }
        }
        if let Some(v) = args.first() {
            if let Some(r) = self.variant_into_field_cells(t, pos, v, true)? {
                let mut a = vec![Value::Integer(pos as i32), r?];
                a.extend(args.iter().skip(1).cloned());
                return self.record_op(RecordOp::Validate, rec, &mut a);
            }
        }
        let mut a = vec![Value::Integer(pos as i32)];
        // A number, Enum, Option, Date or Boolean into a Text or Code field goes in as its Format text
        // (recordref/lt-y47-validate-fieldref/Y47ValidateFieldRef.al:461-492, Y57_ValidateVariantValue_IntoText).
        let first = args
            .first()
            .and_then(|v| al_builtins::fieldref_value_text::variant_into_text_field(v, &t.fields[pos].ty, &*self))
            .transpose()?;
        match first {
            Some(v) => {
                a.push(v);
                a.extend(args.iter().skip(1).cloned());
            }
            None => a.extend(args.iter().cloned()),
        }
        self.record_op(RecordOp::Validate, rec, &mut a)
    }

    /// `al_builtins::fieldref_variant_cells` for the field at `pos`: `None` for a cell it does not pin, and for a text into an Option
    /// or Enum field that names one of its members (that converts by the name).
    pub(crate) fn variant_into_field_cells(
        &self,
        t: &TableDef,
        pos: usize,
        v: &Value,
        validate: bool,
    ) -> Result<Option<Result<Value, Signal>>, Signal> {
        let ty = &t.fields[pos].ty;
        // A text into a Guid field: a Variant holding one that is no GUID raises the Evaluate error from BC 28 (Y67FieldRefValidateVariant.al
        // Validate_Into_G_B `text abc`), a Text variable the GUID format error (fieldref/TestFieldRefValue.al:282
        // FieldRef_Value_TextIntoBooleanDateGuidOption, `FieldRef.Value :=` only). The runtime does not see which of the two the
        // caller passed: `FieldRef.Value :=` keeps the Text's pinned error, `FieldRef.Validate`, which no test pins for a Text, is
        // refused.
        if let (RtType::Guid, Value::Text(s) | Value::Code(s)) = (ty, v) {
            let guid = al_builtins::ops::parse_guid(s.as_str()).is_some();
            return Ok(if validate && !guid {
                Some(Err(Signal::Unsupported(
                    "FieldRef.Validate of a text that is no GUID into a Guid field: a Variant and a Text raise different errors, and the runtime does not tell them apart (not pinned for a Text)".into(),
                )))
            } else {
                None
            });
        }
        if let (RtType::Option(_) | RtType::Enum(_), Value::Text(s) | Value::Code(s)) = (ty, v) {
            let named = match self.members_of_field(t, pos)? {
                Some((_, members)) => members.iter().any(|(_, name, caption)| {
                    name.eq_ignore_ascii_case(s.as_str()) || caption.eq_ignore_ascii_case(s.as_str())
                }),
                None => true,
            };
            if named {
                return Ok(None);
            }
        }
        let from28 = self.has(al_version::Gate::FieldRefTextNumberEvaluateErrorFrom28);
        Ok(al_builtins::fieldref_variant_cells::variant_into_field(
            v, ty, from28, self,
        ))
    }
}
