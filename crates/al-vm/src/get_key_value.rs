//! One value handed to `Record.Get`, converted to the type of its primary key field.

use crate::Session;
use al_builtins::native::Signal;
use al_builtins::ops;
use al_builtins::value::{RtType, Value};

impl Session {
    /// The key value `v` for a key field of type `ty`.
    pub(crate) fn get_key_value(&self, v: &Value, ty: &RtType) -> Result<Value, Signal> {
        Ok(match v {
            // A text for an Integer or BigInteger key reads as Evaluate reads it: surrounding spaces and a sign are fine, anything
            // else raises (GetWithTextArgument.al:Get_TextForIntegerKey, Get_TextVariableDirect; record/Y70Probes.al
            // RecordGet_TextOnIntegerKey `int_text5`, `int_padded`, `int_plus`, `int_x`, `int_empty`, `int_decimal_text`, and
            // RecordGet_TextOnBigIntegerKey; BC 27 names the type unknown, BC 28 and later say the value can't be evaluated). A one-character
            // literal is a Char in a Variant and reads as the text of that character (`int_text5` passes '5', `int_x` passes 'x').
            Value::Text(_) | Value::Code(_) | Value::Char(_) if matches!(ty, RtType::Integer | RtType::BigInteger) => {
                let s = match v {
                    Value::Char(c) => String::from_utf16_lossy(&[*c]),
                    other => al_builtins::native::text_of(other)
                        .map(|t| t.to_string())
                        .unwrap_or_default(),
                };
                let (name, parsed) = match ty {
                    RtType::Integer => ("Integer", s.trim_matches(' ').parse::<i32>().ok().map(Value::Integer)),
                    _ => (
                        "BigInteger",
                        s.trim_matches(' ').parse::<i64>().ok().map(Value::BigInteger),
                    ),
                };
                match parsed {
                    Some(n) => n,
                    None if self.engine.target.as_ref().map_or(29, |t| t.bc.major) >= 28 => {
                        return Err(crate::transactions::al_error_code(
                            &format!("The value \"{s}\" can't be evaluated into type {name}."),
                            "Evaluate:Format",
                        ));
                    }
                    None => {
                        return Err(crate::transactions::al_error_code(
                            &format!("The type Nav{name} is unknown."),
                            "NotSupportedType",
                        ));
                    }
                }
            }
            // A non-negative Integer for a Text key is its digits: Get(1) finds the row '1', Get(0) the row '0'
            // (record/Y70Probes.al RecordGet_BooleanOnTextKey `text_int1`, `text_int0`). A negative one and one longer
            // than the field are not pinned.
            Value::Integer(i) if matches!(ty, RtType::Text(_)) => {
                let digits = i.to_string();
                let fits = matches!(ty, RtType::Text(len) if len.is_none_or(|l| digits.len() <= l as usize));
                if *i < 0 || !fits {
                    return Err(Signal::Unsupported(
                        "Get with a negative or too long Integer on a Text key (not pinned)".into(),
                    ));
                }
                Value::Text(digits.into())
            }
            // An Integer (literal or variable) for a Code key reads as its digits: found, not found and the
            // not-found error (record/crud/AtGetIntegerOnCodeKey.al:86-140, Integer 123 on a
            // Code[20] key). A negative number, one longer than the field and a Text key are not pinned.
            Value::Integer(i) if matches!(ty, RtType::Code(_)) => {
                let digits = i.to_string();
                let fits = matches!(ty, RtType::Code(len) if len.is_none_or(|l| digits.len() <= l as usize));
                if *i < 0 || !fits {
                    return Err(Signal::Unsupported(
                        "Get with a negative or too long Integer on a Code key (not pinned)".into(),
                    ));
                }
                Value::Code(digits.into())
            }
            // An Option or Enum value for a Code key looks for the member's name: `ClientType::Web`, an own Option variable and an
            // Enum value find the row `WEB`, not the row of the ordinal (types/operators-expressions/Y71ValueConversions.al:
            // 218-328). An Enum member with a caption of its own, a name longer than the field and a Text key are not pinned.
            Value::Option(..) | Value::Enum(..) if matches!(ty, RtType::Code(_)) => {
                let name = self.member_name_for_key(v)?;
                let fits = matches!(ty, RtType::Code(len) if len.is_none_or(|l| name.chars().count() <= l as usize));
                if !fits {
                    return Err(Signal::Unsupported(
                        "Get with an Option or Enum value whose name is longer than the Code key (not pinned)".into(),
                    ));
                }
                Value::Code(name.to_uppercase().into())
            }
            v => ops::convert(v.clone(), ty)?,
        })
    }

    fn member_name_for_key(&self, v: &Value) -> Result<String, Signal> {
        let refused = || {
            Signal::Unsupported("Get with an Option or Enum value of a member without a name, or with a caption of its own, on a Code key (not pinned)".into())
        };
        match v {
            Value::Option(o, members) => members
                .get(*o as usize)
                .filter(|m| !m.is_empty())
                .cloned()
                .ok_or_else(refused),
            Value::Enum(id, value) => {
                if let Some(why) = self.enum_ml_unpinned(*id, Some(*value), "the key value of an enum") {
                    return Err(Signal::Unsupported(why));
                }
                let o = &self.engine.program.objects[*id as usize];
                let name = o
                    .enum_values
                    .iter()
                    .find(|(x, _)| x == value)
                    .map(|(_, n)| n.clone())
                    .ok_or_else(refused)?;
                match o.enum_captions.iter().find(|(x, _)| x == value) {
                    Some((_, c)) if !c.eq_ignore_ascii_case(&name) => Err(refused()),
                    _ => Ok(name),
                }
            }
            _ => Err(refused()),
        }
    }
}
