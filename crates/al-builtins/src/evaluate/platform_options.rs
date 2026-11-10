//! `Evaluate` into the platform options read apart from the options an app declares: the types of
//! format-evaluate/SoEvaluateSystemOptions.al (`crate::system_option_eval`), PageBackgroundTaskErrorLevel, SecurityFilter and
//! ClientType, AuditCategory and SecurityOperationResult.

use super::{Parsed, unsupported};
use crate::native::{Ctx, Signal};
use crate::value::Value;

/// Whether `Evaluate` into an option of these members takes `read`.
pub(super) fn reads(members: &[String]) -> bool {
    crate::system_option_eval::reads(members)
        || crate::sysenum::is_page_background_task_error_level(members)
        || crate::sysenum::is_security_filter(members)
        || crate::sysenum::is_client_type(members)
        || crate::sysenum::audit_option_listing(members).is_some()
}

/// `Evaluate` into AuditCategory and SecurityOperationResult (format-evaluate/ACAuditCategory.al:308-535,
/// format-evaluate/ACSecurityOperationResult.al:116-216, BC 27, 28, 29):
/// - a member name in any case, also after a leading space, gives that member (`name_lowercase`, `space_ok`);
/// - the ordinal of a member gives it (`number_1`, `past_end_ok` 4 of SecurityOperationResult); a number that is no
///   member's ordinal answers false (0 of SecurityOperationResult `No/0`, 19 of AuditCategory, 99);
/// - the empty text gives the first member of `listing` (`empty_ok`: 0 and 1);
/// - any other text answers false with the result used and leaves the error `'<text>' is not an option. The existing
///   options are: <members joined by ", ">`, code InvalidOptionString (`bogus`, `large`, `negative`, `decimal`, `past_end`).
///
/// Not pinned and refused: the statement form of a rejected text, a rejected text with a space around it, a text of
/// spaces only, and a number written with a leading zero or sign that would name a member.
fn audit_option(
    c: &mut dyn Ctx,
    members: &std::sync::Arc<[String]>,
    listing: &[&str],
    text: &str,
) -> Result<Parsed, Signal> {
    let ordinal_of = |name: &str| members.iter().position(|m| m == name).expect("listed member") as i32;
    if text.is_empty() {
        return Ok(Parsed::Value(Value::Option(ordinal_of(listing[0]), members.clone())));
    }
    let t = text.trim();
    if t.is_empty() {
        return unsupported("Evaluate of a text of spaces into AuditCategory or SecurityOperationResult (not pinned)");
    }
    if let Some(name) = listing.iter().find(|m| m.eq_ignore_ascii_case(t)) {
        return Ok(Parsed::Value(Value::Option(ordinal_of(name), members.clone())));
    }
    if t.bytes().all(|b| b.is_ascii_digit()) {
        if t.len() > 1 && t.starts_with('0') {
            return unsupported(format!(
                "Evaluate of `{text}`, a number with a leading zero, into AuditCategory or SecurityOperationResult (not pinned)"
            ));
        }
        if let Some(n) = t
            .parse::<usize>()
            .ok()
            .filter(|n| members.get(*n).is_some_and(|m| !m.is_empty()))
        {
            return Ok(Parsed::Value(Value::Option(n as i32, members.clone())));
        }
    } else if t
        .strip_prefix('+')
        .is_some_and(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()))
    {
        return unsupported(format!(
            "Evaluate of `{text}`, a number with a plus sign, into AuditCategory or SecurityOperationResult (not pinned)"
        ));
    }
    if !c.ret_used() {
        return unsupported(format!(
            "Evaluate of `{text}` into AuditCategory or SecurityOperationResult failing as a statement (only the form with the result used is pinned)"
        ));
    }
    if t != text {
        return unsupported(format!(
            "Evaluate of `{text}`, a text with a space around it that names no member, into AuditCategory or SecurityOperationResult (not pinned)"
        ));
    }
    Ok(Parsed::RejectedLastError(
        format!(
            "'{text}' is not an option. The existing options are: {}",
            listing.join(", ")
        ),
        "InvalidOptionString",
    ))
}

/// `Evaluate` into the platform options read apart from the declared ones.
pub(super) fn read(c: &mut dyn Ctx, members: &std::sync::Arc<[String]>, text: &str) -> Result<Parsed, Signal> {
    Ok(match () {
        // The platform's option types of format-evaluate/SoEvaluateSystemOptions.al (`system_option_eval`).
        () if crate::system_option_eval::reads(members) => {
            match crate::system_option_eval::evaluate(&*c, members, text)? {
                Some(v) => Parsed::Value(v),
                None if c.ret_used() => Parsed::Rejected,
                None => return crate::system_option_eval::rejected_error(&*c, members, text),
            }
        }
        // Evaluate into PageBackgroundTaskErrorLevel is not pinned: collections/QbEvaluateBgTask.al is an observe-only probe, not a case.
        () if crate::sysenum::is_page_background_task_error_level(members) => {
            return unsupported(format!(
                "Evaluate of `{text}` into PageBackgroundTaskErrorLevel (not pinned)"
            ));
        }
        // An ordinal into SecurityFilter: `Evaluate(Filter, '3')` gives Disallowed
        // (recordref-fieldref/recordref/TestPlatformOptionsLockingContracts.al:42-43). Other options are not pinned.
        () if crate::sysenum::is_security_filter(members) => {
            match text.trim().parse::<usize>().ok().filter(|n| *n < members.len()) {
                Some(n) => Parsed::Value(Value::Option(n as i32, members.clone())),
                None => {
                    return unsupported(format!(
                        "Evaluate of `{text}` into SecurityFilter (only an ordinal is pinned)"
                    ));
                }
            }
        }
        // ClientType has no OptionCaption: a number below 16 or a member name evaluates, any other text answers false
        // (types/PWMisc.al:ClientType_OrdinalsAndNames: 16 to 30 are rejected).
        () if crate::sysenum::is_client_type(members) => {
            let t = text.trim();
            let by_name = members.iter().position(|m| m.eq_ignore_ascii_case(t));
            match (t.parse::<usize>().ok().filter(|n| *n < members.len()), by_name) {
                (Some(n), _) | (None, Some(n)) => Parsed::Value(Value::Option(n as i32, members.clone())),
                _ => Parsed::Rejected,
            }
        }
        () => match crate::sysenum::audit_option_listing(members) {
            Some(listing) => return audit_option(c, members, listing, text),
            None => unreachable!("reads"),
        },
    })
}
