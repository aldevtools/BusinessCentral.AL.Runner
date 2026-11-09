//! `Evaluate` into the platform options read apart from the options an app declares: the types of
//! format-evaluate/SoEvaluateSystemOptions.al (`crate::system_option_eval`), PageBackgroundTaskErrorLevel, SecurityFilter and
//! ClientType.

use super::{Parsed, unsupported};
use crate::native::{Ctx, Signal};
use crate::value::Value;

/// Whether `Evaluate` into an option of these members takes `read`.
pub(super) fn reads(members: &[String]) -> bool {
    crate::system_option_eval::reads(members)
        || crate::sysenum::is_page_background_task_error_level(members)
        || crate::sysenum::is_security_filter(members)
        || crate::sysenum::is_client_type(members)
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
        () => unreachable!("reads"),
    })
}
