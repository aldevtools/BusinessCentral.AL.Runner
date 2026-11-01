//! More texts Evaluate reads into a DateTime without a number and with number 0 (al-runner issue 6655). Pinned by corpus
//! types/lt-evaluate-format-lasterror-setview/P86Eval.al:932-1110, `t4` to `t8` of Dflt1_* and Fmt01_*.

use super::Parsed;
use crate::value::Value;

/// `yyyy-mm-dd hh:mm:ss` (`t4`, that time in UTC), `yyyy-mm-dd` (`t5`, its midnight), `10:20` (`t6`, 20 October of the
/// current year at midnight: month and day) and `15.01.2024 10:20` (`t8`, rejected). `None`: not one of these shapes. A shape
/// that holds no such date or time (a day that does not exist, a month past 12) is not pinned: `Some(None)`, and the caller
/// refuses.
pub(super) fn read(t: &str, today: Option<i32>) -> Option<Option<Parsed>> {
    let b = t.as_bytes();
    let digits = |s: &[u8]| !s.is_empty() && s.iter().all(u8::is_ascii_digit);
    let iso_date = |s: &[u8]| {
        s.len() == 10 && s[4] == b'-' && s[7] == b'-' && digits(&s[..4]) && digits(&s[5..7]) && digits(&s[8..])
    };
    if iso_date(b) {
        return Some(midnight(t));
    }
    let clock = |s: &[u8]| {
        s.len() == 8 && s[2] == b':' && s[5] == b':' && digits(&s[..2]) && digits(&s[3..5]) && digits(&s[6..])
    };
    if b.len() == 19 && b[10] == b' ' && iso_date(&b[..10]) && clock(&b[11..]) {
        return Some(
            crate::iso::parse_datetime(&format!("{}T{}Z", &t[..10], &t[11..]))
                .map(|v| Parsed::Value(Value::DateTime(v))),
        );
    }
    if let Some((m, d)) = t.split_once(':')
        && (1..=2).contains(&m.len())
        && (1..=2).contains(&d.len())
        && digits(m.as_bytes())
        && digits(d.as_bytes())
    {
        let year = crate::format::civil_from_day(today?).0;
        let (m, d): (u32, u32) = (m.parse().ok()?, d.parse().ok()?);
        return Some(midnight(&format!("{year:04}-{m:02}-{d:02}")));
    }
    let dotted = b.len() == 16
        && b[2] == b'.'
        && b[5] == b'.'
        && b[10] == b' '
        && b[13] == b':'
        && [&b[..2], &b[3..5], &b[6..10], &b[11..13], &b[14..]]
            .iter()
            .all(|s| digits(s));
    dotted.then_some(Some(Parsed::Rejected))
}

/// The start of the ISO date `ymd` in UTC, `None` when it is no date.
fn midnight(ymd: &str) -> Option<Parsed> {
    crate::iso::parse_datetime(&format!("{ymd}T00:00:00Z")).map(|v| Parsed::Value(Value::DateTime(v)))
}
