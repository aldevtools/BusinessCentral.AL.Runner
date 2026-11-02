//! The `<Options>` of a report parameters text: its `Field` elements and the value each one gives the variable a request page
//! control shows (`apply_request_parameters`). Pinned by report/lt-y68-selfclosing-params/Y68SelfClosingParams.al (corpus
//! master 0d01b0cc, BC 27.5, 28.5, 29.0) and report/lt-report-requestpage/RptRequestPage.al:878-893; the rest is refused.

use super::*;
use al_builtins::describe::Describe;

/// The `Field` elements of an Options text, as (name, text). `<Field name="X" />` and `<Field name="X"></Field>` both give an
/// empty text (Y68SelfClosingParams.al:263-343: RunRequestPage writes the first form for an empty value). Text after the
/// last element this reads, or an element in another form, ends the list (as before: such texts are not pinned).
pub(crate) fn option_fields(options: &str) -> Vec<(String, String)> {
    const OPEN: &str = "<Field name=\"";
    const CLOSE: &str = "</Field>";
    let mut out = Vec::new();
    let mut rest = options;
    while let Some(at) = rest.find(OPEN) {
        let after = &rest[at + OPEN.len()..];
        let Some(q) = after.find('"') else { break };
        let name = super::request::xml_unescape(&after[..q]);
        let tail = &after[q + 1..];
        if let Some(more) = tail.strip_prefix(" />").or_else(|| tail.strip_prefix("/>")) {
            out.push((name, String::new()));
            rest = more;
            continue;
        }
        let Some(body) = tail.strip_prefix('>') else { break };
        let Some(end) = body.find(CLOSE) else { break };
        out.push((name, super::request::xml_unescape(&body[..end])));
        rest = &body[end + CLOSE.len()..];
    }
    out
}

fn is_number(text: &str) -> bool {
    let digits = text.strip_prefix('-').unwrap_or(text);
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
}

impl Session {
    /// The value the option `name` with the text `text` gives a variable that holds `current`.
    pub(crate) fn request_option_value(&self, name: &str, text: &str, current: &Value) -> Result<Value, Signal> {
        let refuse = || unsupported(format!("the parameter {name} = {text} (this form is not pinned)"));
        // An empty text sets the variable to its blank value, whatever the request page's OnInit or OnInitReport set: Text
        // and Code '', Date, Time and DateTime undefined, Integer, Decimal and BigInteger 0, Boolean No, an Option its first
        // member and an Enum the value 0 (Y68SelfClosingParams.al:263-343, :373-405).
        if text.is_empty() {
            return match current {
                Value::Text(_) => Ok(Value::Text("".into())),
                Value::Code(_) => Ok(Value::Code("".into())),
                Value::Date(..) => Ok(Value::Date(0, false)),
                Value::Time(_) => Ok(Value::Time(0)),
                Value::DateTime(_) => Ok(Value::DateTime(0)),
                Value::Integer(_) => Ok(Value::Integer(0)),
                Value::BigInteger(_) => Ok(Value::BigInteger(0)),
                Value::Decimal(_) => Ok(Value::default_for(&RtType::Decimal)),
                Value::Boolean(_) => Ok(Value::Boolean(false)),
                Value::Option(_, m) => Ok(Value::Option(0, m.clone())),
                // Only an enum that has the value 0 is pinned (its first value, `First`).
                Value::Enum(id, _)
                    if al_builtins::native::Ctx::enum_members(self, *id)
                        .is_some_and(|m| m.first().is_some_and(|(n, _)| *n == 0)) =>
                {
                    Ok(Value::Enum(*id, 0))
                }
                other => unsupported(format!(
                    "an empty parameter {name} for a control holding {}",
                    other.describe()
                )),
            };
        }
        // The forms RunRequestPage writes (Y68SelfClosingParams.al:345-371 `filled all`, :446-470 `seeded returned`).
        match current {
            Value::Text(_) => Ok(Value::Text(text.into())),
            // A Code is uppercased (`xy` gives `XY`).
            Value::Code(_) => Ok(Value::Code(text.to_uppercase().as_str().into())),
            Value::Boolean(_) if text == "true" || text == "false" => Ok(Value::Boolean(text == "true")),
            // `yyyy-MM-dd` (also RptRequestPage.al:878-893 RunRequestPage_ThenExecute_AppliesValues).
            Value::Date(..) => match al_builtins::iso::parse_date(text) {
                Some(d) if text.len() == 10 => Ok(Value::Date(d, false)),
                _ => refuse(),
            },
            // `HH:mm:ss`.
            Value::Time(_) => match al_builtins::iso::parse_time(text) {
                Some(t) if text.len() == 8 => Ok(Value::Time(t)),
                _ => refuse(),
            },
            // `yyyy-MM-ddTHH:mm:ssZ`.
            Value::DateTime(_) => match al_builtins::iso::parse_datetime(text) {
                Some(t) if text.len() == 20 && text.ends_with('Z') => Ok(Value::DateTime(t)),
                _ => refuse(),
            },
            Value::Integer(_) => match text.parse::<i32>() {
                Ok(n) if is_number(text) => Ok(Value::Integer(n)),
                // A text of letters fails the Execute with the text `NavType` and the code Argument (`filled bad int`).
                _ if text.bytes().all(|b| b.is_ascii_alphabetic()) => Err(Signal::Error(Box::new(AlError {
                    message: "NavType".into(),
                    code: "Argument".into(),
                    ..Default::default()
                }))),
                _ => refuse(),
            },
            Value::BigInteger(_) => match text.parse::<i64>() {
                Ok(n) if is_number(text) => Ok(Value::BigInteger(n)),
                _ => refuse(),
            },
            // `0.75`, `2.5`: digits with an optional `.` part.
            Value::Decimal(_) => {
                let (whole, frac) = text.split_once('.').unwrap_or((text, "0"));
                if !is_number(whole) || !frac.bytes().all(|b| b.is_ascii_digit()) || frac.is_empty() {
                    return refuse();
                }
                ops::convert(Value::Text(text.into()), &RtType::Decimal)
            }
            // An Option by its index; a member name fails the Execute with the Evaluate error from BC 28 and
            // `The type NavOption is unknown.` before (`filled by name`, guarded BC28PLUS).
            Value::Option(_, m) => match text.parse::<i32>() {
                Ok(n) if is_number(text) && n >= 0 && (n as usize) < m.len() => Ok(Value::Option(n, m.clone())),
                Ok(_) => refuse(),
                Err(_) if m.iter().any(|x| x == text) => {
                    let bc28 = self.engine.target.as_ref().map_or(29, |t| t.bc.major) >= 28;
                    let (message, code) = if bc28 {
                        (
                            format!("The value \"{text}\" can't be evaluated into type Option."),
                            "Evaluate:Format",
                        )
                    } else {
                        ("The type NavOption is unknown.".to_string(), "NotSupportedType")
                    };
                    Err(Signal::Error(Box::new(AlError {
                        message,
                        code: code.into(),
                        ..Default::default()
                    })))
                }
                Err(_) => refuse(),
            },
            // An Enum by its ordinal, one of its values.
            Value::Enum(id, _) => match text.parse::<i32>() {
                Ok(n)
                    if is_number(text)
                        && al_builtins::native::Ctx::enum_members(self, *id)
                            .is_some_and(|m| m.iter().any(|(o, _)| *o == n)) =>
                {
                    Ok(Value::Enum(*id, n))
                }
                _ => refuse(),
            },
            Value::Boolean(_) => refuse(),
            other => unsupported(format!(
                "a parameter for the control {name} holding {}",
                other.describe()
            )),
        }
    }
}
