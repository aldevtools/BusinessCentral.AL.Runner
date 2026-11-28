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
//!
//! On every OS this module also owns the kind of a DateTime a `TimeZoneInfo` member gets (al-runner issue 8587,
//! `zone_datetime_kind`): `GetUtcOffset` of a zone the host holds reads an AL DateTime as a
//! UTC moment (assumption `zone-datetime-utc-instant`); a DateTime whose origin is not an AL DateTime, a Date, and any
//! other `TimeZoneInfo` member with a DateTime or Date argument are refused. The members of a DotNet DateTime that read
//! its kind are refused too (`datetime_kind_member`, issue 9052): the runner does not track the kind. Three of them give
//! the same answer for every kind while the service tier's zone is UTC, as the runner's session zone is (al-host), and
//! are answered here with no .NET call (`datetime_utc_zone_member`, assumption `dotnet-datetime-utc-zone-conversions`).
//!
//! Off Windows, the `TimeZoneInfo` checks that need the arguments (a zone passed as an argument,
//! `GetUtcOffset`) run after the policy (issue 8012).

use super::registry::MemberKind;
use super::{TICKS_PER_MS, Target};
use super::policy::Policy;
use crate::Session;
use crate::hostvalues::{day_number_of_unix_millis, unix_millis_of_datetime};
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::format::civil_from_day;
use al_builtins::value::{DotNetKind, RtType, Value};
use al_host::dotnet::{DotNetArg, DotNetValue};
use al_ir as ir;

const ZONE: &str = "System.TimeZoneInfo";
const DATE_TIME: &str = "System.DateTime";
/// 1601-01-01T00:00:00 in .NET ticks, the start of a Windows file time (133182576000000000 is 2023-01-15T12:00:00Z,
/// DotNetDateTimeKind.al:367).
const FILE_TIME_EPOCH_TICKS: i64 = 504_911_232_000_000_000;

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

    /// Bit `i` set: argument `i` of a DotNet call is a by-value DateTime parameter of the running AL procedure, the one origin
    /// the probe measured (corpus pull request 1582, DotNetTimeZoneDst.al `DoMember`/`Run`: `Dt: DateTime` handed to
    /// `GetUtcOffset`). A local variable, global, record field, procedure result, `CreateDateTime`, `CurrentDateTime`, a DotNet
    /// DateTime and any other expression leave the bit clear (the call refuses where the kind matters, `zone_datetime_kind`).
    pub(crate) fn al_datetime_args(&self, args: &[ir::Arg], values: &[Value], f: &crate::Frame) -> u64 {
        let r = &self.engine.program.objects[f.object].routines[f.routine];
        let mut bits = 0u64;
        for (i, (a, v)) in args.iter().zip(values).enumerate().take(64) {
            let by_value_param = matches!(a, ir::Arg::Ref(ir::Place::Local(l))
                if *l < r.params && !r.by_var.get(*l).copied().unwrap_or(true));
            if by_value_param
                && matches!(v, Value::DateTime(_))
                && matches!(a, ir::Arg::Ref(p) if self.place_type(p, f) == RtType::DateTime)
            {
                bits |= 1 << i;
            }
        }
        bits
    }

    /// The kind of the DateTime a `TimeZoneInfo` member gets (al-runner issue 8587). `GetUtcOffset(DateTime)` of a zone the
    /// host holds reads an AL DateTime as a UTC moment: the wire value
    /// becomes `DateTimeUtc` (assumption `zone-datetime-utc-instant`). The zone of the id `UTC` made in process keeps
    /// main's answer (0 at any moment). Every other `TimeZoneInfo` call with a DateTime or a Date argument is refused:
    /// .NET reads the kind there, and what BC hands it is not measured. `Err` is the refusal reason.
    pub(super) fn zone_datetime_kind(
        &self,
        target: &Target,
        clr_type: &str,
        member: &str,
        values: &[Value],
        al_datetime_args: u64,
        wire: &mut [DotNetArg],
    ) -> Result<(), String> {
        if !clr_type.eq_ignore_ascii_case(ZONE)
            || !values.iter().any(|v| matches!(v, Value::DateTime(_) | Value::Date(..)))
        {
            return Ok(());
        }
        // `IsDaylightSavingTime` was measured by BC on Linux only (corpus pull request 1582), not on the Windows zone data a
        // Windows host reads: refused until a Windows run measures it.
        let reads_moment = member.eq_ignore_ascii_case("GetUtcOffset");
        if let (true, Target::Host(zone, _), [Value::DateTime(_)]) = (reads_moment, target, values) {
            let utc_zone = self
                .dotnet
                .session
                .as_ref()
                .is_some_and(|s| s.is_in_process_utc_zone(*zone));
            if utc_zone {
                return Ok(());
            }
            if al_datetime_args == 1
                && let [
                    DotNetArg {
                        value: DotNetValue::DateTime(ticks),
                        ..
                    },
                ] = wire
            {
                assume(Assumption::ZoneDatetimeUtcInstant);
                wire[0].value = DotNetValue::DateTimeUtc(*ticks);
                return Ok(());
            }
            return Err(format!(
                "a DateTime that is no AL DateTime value passed to '{ZONE}.{member}': the kind BC gives it is not measured"
            ));
        }
        Err(format!(
            "a Date or DateTime passed to '{ZONE}.{member}': the kind BC gives it is not measured (only GetUtcOffset of an AL \
             DateTime runs)"
        ))
    }

    /// The members of a `System.DateTime` the variable holds that read its kind: refused (al-runner issue 9052). BC gives a
    /// DotNet DateTime assigned from an AL DateTime the kind Utc (corpus pull request 1582, `kind_2023-01-15T12:00:00Z`:
    /// `ToString('o')` ends in `Z`, `Kind` is `Utc`); the runner keeps only the ticks of any DotNet DateTime and sends kind
    /// Unspecified. `Kind`, `ToUniversalTime`, `ToLocalTime`, `ToFileTime`, `ToBinary` and `ToString` with the round-trip
    /// format (`o`, `O`) or a format that holds `K` or `z` read the kind. `datetime_utc_zone_member` answers the measured
    /// calls of three of them first. `Err` is the refusal reason.
    pub(super) fn datetime_kind_member(target: &Target, member: &str, values: &[Value]) -> Result<(), String> {
        let Target::Boxed(DotNetValue::DateTime(_), clr_type) = target else {
            return Ok(());
        };
        if !clr_type.eq_ignore_ascii_case(DATE_TIME) {
            return Ok(());
        }
        let reads_kind = ["Kind", "ToUniversalTime", "ToLocalTime", "ToFileTime", "ToBinary"]
            .iter()
            .any(|m| member.eq_ignore_ascii_case(m))
            || (member.eq_ignore_ascii_case("ToString")
                && matches!(values.first(), Some(Value::Text(f) | Value::Code(f))
                    if f.as_str() == "o" || f.as_str() == "O" || f.as_str().contains(['K', 'z'])));
        if reads_kind {
            return Err(format!(
                "'{DATE_TIME}.{member}' reads the kind of the DateTime, which the runner does not track (BC gives one \
                 assigned from an AL DateTime the kind Utc; al-runner issue 9052)"
            ));
        }
        Ok(())
    }

    /// `ToUniversalTime()`, `ToLocalTime()` and `ToFileTime()` of a DotNet `System.DateTime` the variable holds, answered
    /// without .NET (al-runner issues 9052, 9092; assumption `dotnet-datetime-utc-zone-conversions`, corpus pull request 1653,
    /// tests/al-language-onprem-dotnet/dotnet/datetime-kind/DotNetDateTimeKind.al). The service tier's zone is UTC there
    /// (Host_TimeZone_Facts :254) and in the runner, so the answer is the same for every kind: the probe's touniversal,
    /// tolocal and tofiletime steps give the same moment for kind Utc (Origin_AlParam :367), Local (Origin_CtorLocal :457,
    /// Now :280) and Unspecified (Origin_CtorUnspecified :421). The result DateTime goes back to AL as every .NET DateTime
    /// does, without its kind. `None`: not one of these calls, or `ToFileTime` of a moment before 1601 (.NET raises there;
    /// not measured), which keeps main's refusal (`datetime_kind_member`).
    pub(super) fn datetime_utc_zone_member(target: &Target, kind: MemberKind, member: &str, values: &[Value]) -> Option<Value> {
        let (Target::Boxed(DotNetValue::DateTime(ticks), clr_type), MemberKind::Call | MemberKind::Get, []) =
            (target, kind, values)
        else {
            return None;
        };
        if !clr_type.eq_ignore_ascii_case(DATE_TIME) {
            return None;
        }
        let answer = if member.eq_ignore_ascii_case("ToUniversalTime") || member.eq_ignore_ascii_case("ToLocalTime") {
            // The same moment, read back as `wire_to_value` reads a .NET DateTime.
            Value::DateTime(ticks.div_euclid(TICKS_PER_MS) + 1)
        } else if member.eq_ignore_ascii_case("ToFileTime") && *ticks >= FILE_TIME_EPOCH_TICKS {
            Value::BigInteger(ticks - FILE_TIME_EPOCH_TICKS)
        } else {
            return None;
        };
        assume(Assumption::DotnetDatetimeUtcZoneConversions);
        Some(answer)
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
