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
        let mut a = vec![Value::Integer(pos as i32)];
        a.extend(args.iter().cloned());
        self.record_op(RecordOp::Validate, rec, &mut a)
    }
}
