//! Off Windows, the `TimeZoneInfo` calls whose arguments decide whether .NET may answer (al-runner issue 8012).
//!
//! BC reads the Windows zone data; .NET on any other OS reads the IANA and ICU data of the host. The policy's
//! `members-off-windows` rule (`policy.txt`) names the members a zone takes there. This module owns the two checks that
//! also need the arguments, and is in force exactly while that rule is (it is lifted on a Windows .NET host):
//! - a zone passed to any member (`StringBuilder.Append(Object)` would call its `ToString` in .NET) is refused;
//! - `GetUtcOffset` runs only with one `DateTime`, on the zone of the id `UTC` that the in-process stand-in made
//!   (main's answer 0, ADR-038 amendment 2026-10-27) or for a date of the session clock's year. That is main's
//!   answer kept for the shapes Microsoft's callers use: Base App TypeHelper passes the current UTC time, and
//!   Microsoft's "Date and Time" test calls the System App "Time Zone" codeunit only with dates of the current year.
//!   For other years IANA history and the Windows rules differ (Hawaii 1945, Western Europe before 1996). Every
//!   other shape, the `DateTimeOffset` overload included, is refused.

use super::Target;
use super::policy::Policy;
use crate::Session;
use crate::hostvalues::{day_number_of_unix_millis, unix_millis_of_datetime};
use al_builtins::format::civil_from_day;
use al_builtins::value::{DotNetKind, Value};

const ZONE: &str = "System.TimeZoneInfo";

impl Session {
    /// Whether this call may go to .NET as far as zone data is concerned; `Err` is the refusal reason.
    pub(super) fn zone_rules_check(
        &self,
        policy: &Policy,
        clr_type: &str,
        member: &str,
        target: &Target,
        values: &[Value],
    ) -> Result<(), String> {
        let Some(rule) = policy.off_windows_members_rule(ZONE) else {
            return Ok(());
        };
        if values.iter().any(is_zone) {
            return Err(format!(
                "refused by policy: a {ZONE} passed to '{clr_type}.{member}' ({rule})"
            ));
        }
        if !(clr_type.eq_ignore_ascii_case(ZONE) && member.eq_ignore_ascii_case("GetUtcOffset")) {
            return Ok(());
        }
        if let (Target::Host(zone, _), [Value::DateTime(at)]) = (target, values) {
            let utc_zone = self
                .dotnet
                .session
                .as_ref()
                .is_some_and(|s| s.is_in_process_utc_zone(*zone));
            let year = year_of_day(day_number_of_unix_millis(unix_millis_of_datetime(*at)));
            if utc_zone || (year.is_some() && year == self.clock_year()) {
                return Ok(());
            }
        }
        Err(format!(
            "refused by policy: member '{ZONE}.GetUtcOffset' ({rule}); off Windows it runs only for one DateTime of the \
             session clock's year, or on the zone of the id UTC made in process"
        ))
    }

    /// The year of the session clock, the one `CurrentDateTime` reads (no read: the clock does not move).
    fn clock_year(&self) -> Option<i32> {
        year_of_day(day_number_of_unix_millis(self.now_millis()))
    }
}

fn year_of_day(day: i64) -> Option<i32> {
    i32::try_from(day).ok().map(|d| civil_from_day(d).0)
}

fn is_zone(v: &Value) -> bool {
    matches!(v, Value::DotNet(o) if matches!(&o.kind, DotNetKind::Host { clr_type, .. } if clr_type.eq_ignore_ascii_case(ZONE)))
}
