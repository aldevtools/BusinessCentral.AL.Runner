//! `<Second dec>` on a Time and a DateTime: the fraction of the second (al-runner issue 6650). Real BC's values: corpus writer
//! Y64, branch al-runtime-rs-staging-y64 85d86e7b, types/lt-datetime/Format.al:1066-1300; `<Second dec>` and `<Second dec,3>` of
//! .078 also on master (Format.al:296-321, :600-659).

use super::INVALID_FORMAT_MARK;
use crate::value::Value;

/// The part `<Second dec>` writes, in any case and also spelt `<Second dec.>`, with an optional width `,N`. `None`: not this
/// code, or a value that is no Time or DateTime.
///
/// - No width: a period and the milliseconds with their trailing zeros dropped (`.078`, `.5`, `.05`); nothing when there are
///   none, also on the undefined Time.
/// - Width N from 1 to 5: that text padded with zeros on the right to N characters (`.50`, `.0780`), or N asterisks when it is
///   longer (`<Second dec,3>` of .078 is `***`). No milliseconds give N zeros on a Time and a period and N-1 zeros on a DateTime.
/// - `<Second dec..>` is an invalid format. Another width is not pinned.
pub(super) fn part(v: &Value, code: &str) -> Option<Result<String, String>> {
    let (name, width) = match code.split_once(',') {
        Some((n, w)) => (n, Some(w.trim())),
        None => (code, None),
    };
    let lname = name.trim_end().to_ascii_lowercase();
    let base = lname.strip_suffix('.').unwrap_or(&lname);
    if base.trim_end_matches('.') != "second dec" {
        return None;
    }
    let (ms, datetime) = match v {
        Value::Time(0) => (0, false),
        Value::Time(t) => ((t - 1) % 1000, false),
        Value::DateTime(t) if *t > 0 => ((t - 1).rem_euclid(1000), true),
        _ => return None,
    };
    if base != "second dec" {
        return Some(Err(INVALID_FORMAT_MARK.to_string()));
    }
    let digits = if ms == 0 {
        String::new()
    } else {
        format!(".{ms:03}").trim_end_matches('0').to_string()
    };
    let Some(w) = width else {
        return Some(Ok(digits));
    };
    let n = match w.parse::<usize>() {
        Ok(n @ 1..=5) => n,
        _ => return Some(Err(format!("code `<{code}>` with this width (not pinned)"))),
    };
    Some(Ok(if digits.len() > n {
        "*".repeat(n)
    } else if digits.is_empty() && !datetime {
        "0".repeat(n)
    } else if digits.is_empty() {
        format!(".{}", "0".repeat(n - 1))
    } else {
        format!("{digits:0<n$}")
    }))
}

/// Whether `code` is `<Second dec>` or `<Second dec.>` without a width: before `<Filler Character,0>` it writes what it writes
/// alone (Format.al:1081, the twelfth string of Y64Strings).
pub(super) fn without_width(code: &str) -> bool {
    matches!(
        code.trim_end().to_ascii_lowercase().as_str(),
        "second dec" | "second dec."
    )
}
