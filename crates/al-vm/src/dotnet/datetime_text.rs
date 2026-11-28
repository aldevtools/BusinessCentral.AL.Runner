//! The text .NET gives for `ToString()` of a DotNet `System.DateTime`, set to BC's (al-runner issue 9395).
//!
//! The sidecar runs with culture en-US (tools/dotnet-sidecar `Program.cs`), and its ICU data writes a narrow no-break
//! space (U+202F) between the seconds and AM or PM. BC on Linux 27.5, 28.5 and 29.0 writes a plain space there
//! (assumption `dotnet-datetime-tostring-plain-space`, corpus pull request 1653,
//! tests/al-language-onprem-dotnet/dotnet/datetime-kind/DotNetDateTimeKind.al, the tostring step of every origin:
//! PM at :377 and :701, AM at :521). The call still goes to .NET; only that one character of its answer changes.
//! Every other member and form keeps .NET's answer (main's): a format argument (`G`, `t`), a culture argument,
//! `ToLongTimeString`, `ToShortTimeString`, the form without parentheses, `String.Format` and `Convert.ToString` of a
//! DateTime and `DateTimeOffset.ToString()` are not measured.

use super::Target;
use super::registry::MemberKind;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::value::Value;
use al_host::dotnet::DotNetValue;

const DATE_TIME: &str = "System.DateTime";
const NARROW_NO_BREAK_SPACE: char = '\u{202F}';

/// `result` of the call `member` on `target` with `values`, with BC's plain space before AM or PM when the call is
/// `ToString()` of a `System.DateTime` the variable holds (the probe's `Dn.ToString()`, DotNetDateTimeKind.al:178) and
/// the text ends in U+202F and `AM` or `PM`. Any other call or text: `result` unchanged.
pub(super) fn datetime_default_text(
    target: &Target,
    kind: MemberKind,
    member: &str,
    values: &[Value],
    result: Value,
) -> Value {
    let (Target::Boxed(DotNetValue::DateTime(_), clr_type), MemberKind::Call, [], Value::Text(text)) =
        (target, kind, values, &result)
    else {
        return result;
    };
    if !(clr_type.eq_ignore_ascii_case(DATE_TIME) && member.eq_ignore_ascii_case("ToString")) {
        return result;
    }
    let s = text.as_str();
    let Some(clock) = s.strip_suffix("AM").or_else(|| s.strip_suffix("PM")) else {
        return result;
    };
    let Some(moment) = clock.strip_suffix(NARROW_NO_BREAK_SPACE) else {
        return result;
    };
    assume(Assumption::DotnetDatetimeTostringPlainSpace);
    Value::Text(format!("{moment} {}", &s[clock.len()..]).into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date_time() -> Target {
        Target::Boxed(DotNetValue::DateTime(638_093_808_000_000_000), DATE_TIME.to_string())
    }

    fn text(s: &str) -> Value {
        Value::Text(s.into())
    }

    fn shown(v: &Value) -> String {
        match v {
            Value::Text(t) => t.as_str().to_string(),
            other => panic!("not a text: {other:?}"),
        }
    }

    #[test]
    fn to_string_of_a_date_time_gets_a_plain_space_before_am_or_pm() {
        for (sidecar, bc) in [
            ("1/15/2023 12:00:00\u{202F}PM", "1/15/2023 12:00:00 PM"),
            ("1/15/2023 10:00:00\u{202F}AM", "1/15/2023 10:00:00 AM"),
        ] {
            let v = datetime_default_text(&date_time(), MemberKind::Call, "ToString", &[], text(sidecar));
            assert_eq!(shown(&v), bc);
        }
    }

    #[test]
    fn every_other_call_keeps_the_text_of_dotnet() {
        let narrow = "12:00\u{202F}PM";
        let offset = Target::Boxed(DotNetValue::DateTime(0), "System.DateTimeOffset".to_string());
        let cases: [(&Target, MemberKind, &str, Vec<Value>); 4] = [
            (&date_time(), MemberKind::Get, "ToString", vec![]),
            (&date_time(), MemberKind::Call, "ToString", vec![text("t")]),
            (&date_time(), MemberKind::Call, "ToShortTimeString", vec![]),
            (&offset, MemberKind::Call, "ToString", vec![]),
        ];
        for (target, kind, member, values) in cases {
            let v = datetime_default_text(target, kind, member, &values, text(narrow));
            assert_eq!(shown(&v), narrow, "{member} {kind:?}");
        }
    }
}
