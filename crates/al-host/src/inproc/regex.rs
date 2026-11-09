//! `System.Text.RegularExpressions.Regex` objects built again from arguments the sidecar already accepted (ADR-038
//! amendment 2026-11-09). System Application's "Regex Impl." makes a new `Regex` for every `IsMatch(Input, Pattern)`:
//! `TimeSpan.FromTicks`, the constructor, the match, and a release of the previous object, four requests per match.
//!
//! The matching stays in .NET: this module never interprets a pattern. The first `Regex` made from a set of
//! constructor arguments is made in the sidecar as before, so an invalid pattern fails there with .NET's own exception.
//! When it succeeds, the session keeps that sidecar object (the *canonical* one) and hands AL code a local object. Every
//! later `Regex` made from the same arguments is a local object too, with no request. A `Regex` is immutable, so its
//! matching members answer the same on any instance built from the same arguments: they go to the canonical object.
//! Every other member (`GetHashCode`, `Equals`, `ToString`, a property, the object passed as an argument) first builds
//! the object in the sidecar for real (the migration of ADR-038 section 2), so identity stays as in .NET.
//!
//! A case-insensitive pattern without `CultureInvariant` is not proxied: its case table is the culture current when the
//! object is made, and AL code can change the culture between two constructions.

use crate::dotnet::DotNetValue;

pub(super) const REGEX: &str = "System.Text.RegularExpressions.Regex";

/// `RegexOptions.IgnoreCase` and `RegexOptions.CultureInvariant`.
const IGNORE_CASE: i64 = 1;
const CULTURE_INVARIANT: i64 = 512;

/// The members that only read the immutable pattern and options: they answer the same on the canonical object.
const MATCHING: &[&str] = &[
    "IsMatch",
    "Match",
    "Matches",
    "Replace",
    "Split",
    "GetGroupNames",
    "GetGroupNumbers",
    "GroupNameFromNumber",
    "GroupNumberFromName",
];

pub(super) fn is_matching(member: &str) -> bool {
    MATCHING.iter().any(|m| m.eq_ignore_ascii_case(member))
}

/// The key of a constructor call that may be proxied: plain values only (text, numbers, an enum, a `TimeSpan`).
/// `None` sends the construction to the sidecar every time.
pub(super) fn key(args: &[&DotNetValue]) -> Option<String> {
    let mut key = String::new();
    for v in args {
        // One tagged field per argument, `\u{1}` between them (a pattern cannot hold `\u{1}` ambiguously: the tag and
        // the field count fix where each argument ends).
        let part = match v {
            DotNetValue::Str(s) => format!("s{}:{s}", s.len()),
            DotNetValue::TimeSpan(t) => format!("t{t}"),
            DotNetValue::Bool(b) => format!("b{}", u8::from(*b)),
            DotNetValue::Int32(o) => {
                culture_free(i64::from(*o))?;
                format!("i{o}")
            }
            DotNetValue::Int64(o) => {
                culture_free(*o)?;
                format!("l{o}")
            }
            DotNetValue::Enum { ty, ordinal, .. } => {
                culture_free(*ordinal)?;
                format!("e{}:{ty}:{ordinal}", ty.len())
            }
            _ => return None,
        };
        key.push_str(&part);
        key.push('\u{1}');
    }
    Some(key)
}

/// A number among the arguments is the options (`Regex(String, RegexOptions[, TimeSpan])`).
fn culture_free(options: i64) -> Option<()> {
    (options & IGNORE_CASE == 0 || options & CULTURE_INVARIANT != 0).then_some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_arguments_have_a_key_and_others_do_not() {
        let p = DotNetValue::Str("^a$".into());
        let t = DotNetValue::TimeSpan(10_000_000);
        assert!(key(&[&p]).is_some());
        assert!(key(&[&p, &DotNetValue::Int32(0), &t]).is_some());
        assert_ne!(key(&[&p, &DotNetValue::Int32(0)]), key(&[&p, &DotNetValue::Int32(2)]));
        assert!(key(&[&p, &DotNetValue::Int32(IGNORE_CASE as i32)]).is_none());
        assert!(key(&[&p, &DotNetValue::Int32((IGNORE_CASE | CULTURE_INVARIANT) as i32)]).is_some());
        assert!(key(&[&DotNetValue::Null]).is_none());
        assert!(
            key(&[&DotNetValue::Handle {
                handle: 1,
                ty: "x".into()
            }])
            .is_none()
        );
    }
}
