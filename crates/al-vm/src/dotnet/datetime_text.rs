//! The text .NET gives for `ToString()` of a DotNet `System.DateTime`, set to BC's (al-runner issue 9395).
//!
//! The sidecar runs with culture en-US (tools/dotnet-sidecar `Program.cs`), and its ICU data writes a narrow no-break
//! space (U+202F) between the seconds and AM or PM. BC on Linux 27.5, 28.5 and 29.0 writes a plain space there
//! (assumption `dotnet-datetime-tostring-plain-space`, corpus pull request 1653,
//! tests/al-language-onprem-dotnet/dotnet/datetime-kind/DotNetDateTimeKind.al, the tostring step of every origin:
//! PM at :377 and :701, AM at :521). The call still goes to .NET; only that one character of its answer changes.
//! The probe ran in the formatting state the session starts in: a test codeunit with language en-US, no report running,
//! and no AL code that set the .NET culture or its date formats. Any other state keeps .NET's answer (main's): the
//! sidecar's culture changed since it started (`DotNetSession::formatting_at_start`, owned by the sidecar's
//! `SessionGuard`), another `GlobalLanguage`, or code inside a report run or `RunRequestPage`. Every other member and
//! form keeps .NET's answer too: a format argument (`G`, `t`), a culture argument, `ToLongTimeString`,
//! `ToShortTimeString`, the form without parentheses, `String.Format` and `Convert.ToString` of a DateTime and
//! `DateTimeOffset.ToString()` are not measured.

use super::Target;
use super::registry::MemberKind;
use crate::Session;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::format::FormatCtx;
use al_builtins::value::Value;
use al_host::LCID_EN_US;
use al_host::dotnet::DotNetValue;

const DATE_TIME: &str = "System.DateTime";
const NARROW_NO_BREAK_SPACE: char = '\u{202F}';

/// `result` of the call `member` on `target` with `values`, with BC's plain space before AM or PM when the call is
/// `ToString()` of a `System.DateTime` the variable holds (the probe's `Dn.ToString()`, DotNetDateTimeKind.al:178) and
/// the text ends in U+202F and `AM` or `PM`, in the state the probe ran in (`measured`, asked last). Any other call, text
/// or state: `result` unchanged.
pub(super) fn datetime_default_text(
    target: &Target,
    kind: MemberKind,
    member: &str,
    values: &[Value],
    result: Value,
    measured: impl FnOnce() -> bool,
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
    if !measured() {
        return result;
    }
    assume(Assumption::DotnetDatetimeTostringPlainSpace);
    Value::Text(format!("{moment} {}", &s[clock.len()..]).into())
}

impl Session {
    /// The formatting state DotNetDateTimeKind.al ran `Dn.ToString()` in (a test codeunit): no report runs (no report body
    /// or `OnInitReport`, `report_preview_pinned`, and no `RunRequestPage`), the session language en-US, no `FormatRegion`,
    /// and the sidecar's culture and its date formats as it started. Code of a report object called outside a run is the
    /// session's.
    pub(super) fn datetime_text_state_measured(&mut self) -> bool {
        self.report.report_preview_pinned.is_empty()
            && !self.report.request_only_page
            && self.language() == LCID_EN_US
            && self.region().is_none()
            && self.dotnet.session.as_mut().is_some_and(|s| s.formatting_at_start())
    }
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
            let v = datetime_default_text(&date_time(), MemberKind::Call, "ToString", &[], text(sidecar), || true);
            assert_eq!(shown(&v), bc);
        }
    }

    #[test]
    fn another_formatting_state_keeps_the_text_of_dotnet() {
        let narrow = "1/15/2023 12:00:00\u{202F}PM";
        let v = datetime_default_text(&date_time(), MemberKind::Call, "ToString", &[], text(narrow), || false);
        assert_eq!(shown(&v), narrow);
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
            let v = datetime_default_text(target, kind, member, &values, text(narrow), || true);
            assert_eq!(shown(&v), narrow, "{member} {kind:?}");
        }
    }
}
