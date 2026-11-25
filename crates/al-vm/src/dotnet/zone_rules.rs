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
//!
//! On every OS this module also owns the kind of a DateTime a `TimeZoneInfo` member gets (al-runner issue 8587,
//! `zone_datetime_kind`): `GetUtcOffset` and `IsDaylightSavingTime` of a zone the host holds read an AL DateTime as a
//! UTC moment (assumption `zone-datetime-utc-instant`); a DateTime whose origin is not an AL DateTime, a Date, and any
//! other `TimeZoneInfo` member with a DateTime or Date argument are refused.
//!
//! off Windows, the `TimeZoneInfo` checks that need the arguments (a zone passed as an argument,
//! `GetUtcOffset`), after the policy (issue 8012).

use super::Target;
use super::policy::Policy;
use crate::Session;
use crate::hostvalues::{day_number_of_unix_millis, unix_millis_of_datetime};
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::builtin_id::ids;
use al_builtins::format::civil_from_day;
use al_builtins::value::{DotNetKind, RtType, Value};
use al_host::dotnet::{DotNetArg, DotNetValue};
use al_ir as ir;

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

    /// Bit `i` set: argument `i` of a DotNet call holds a DateTime whose AL type is DateTime: a DateTime variable, field or
    /// parameter (by its declared type), or a value `value_origin` names. A DotNet DateTime variable, a .NET result and any
    /// other expression leave the bit clear (the call refuses where the kind matters, `zone_datetime_kind`).
    pub(crate) fn al_datetime_args(
        &self,
        args: &[ir::Arg],
        origins: &[ValueOrigin],
        values: &[Value],
        f: &crate::Frame,
    ) -> u64 {
        let mut bits = 0u64;
        for (i, ((a, o), v)) in args.iter().zip(origins).zip(values).enumerate().take(64) {
            if !matches!(v, Value::DateTime(_)) {
                continue;
            }
            let al = match (a, o) {
                (ir::Arg::Ref(p), _) => self.place_type(p, f) == RtType::DateTime,
                (ir::Arg::Value(_), ValueOrigin::AlDateTime) => true,
                (ir::Arg::Value(_), ValueOrigin::Call { object, routine }) => self
                    .engine
                    .program
                    .objects
                    .get(*object)
                    .and_then(|o| o.routines.get(*routine))
                    .is_some_and(|r| r.ret == Some(RtType::DateTime)),
                (ir::Arg::Value(_), ValueOrigin::Other) => false,
            };
            if al {
                bits |= 1 << i;
            }
        }
        bits
    }

    /// The kind of the DateTime a `TimeZoneInfo` member gets (al-runner issue 8587). `GetUtcOffset(DateTime)` and
    /// `IsDaylightSavingTime(DateTime)` of a zone the host holds read an AL DateTime as a UTC moment: the wire value
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
        let reads_moment =
            member.eq_ignore_ascii_case("GetUtcOffset") || member.eq_ignore_ascii_case("IsDaylightSavingTime");
        if let (true, Target::Host(zone, _), [Value::DateTime(_)]) = (reads_moment, target, values) {
            let utc_zone = self
                .dotnet
                .session
                .as_ref()
                .is_some_and(|s| s.is_in_process_utc_zone(*zone));
            if utc_zone && member.eq_ignore_ascii_case("GetUtcOffset") {
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
            "a Date or DateTime passed to '{ZONE}.{member}': the kind BC gives it is not measured (only GetUtcOffset and \
             IsDaylightSavingTime of an AL DateTime run)"
        ))
    }

    /// The year of the session clock, the one `CurrentDateTime` reads (no read: the clock does not move).
    fn clock_year(&self) -> Option<i32> {
        year_of_day(day_number_of_unix_millis(self.now_millis()))
    }
}

/// Where a by-value argument of a DotNet call comes from, read when the call is compiled (the compiled site keeps no
/// expression of a by-value argument): an AL builtin that returns a DateTime (`CreateDateTime`, `CurrentDateTime`), a
/// call of an AL procedure (its declared return type is read when the call runs), or anything else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ValueOrigin {
    AlDateTime,
    Call { object: usize, routine: usize },
    Other,
}

pub(crate) fn value_origin(a: &ir::Arg) -> ValueOrigin {
    match a {
        ir::Arg::Value(ir::Expr::Builtin { id, .. })
            if *id == ids::SYSTEM_CREATEDATETIME_1 || *id == ids::SYSTEM_CURRENTDATETIME_1 =>
        {
            ValueOrigin::AlDateTime
        }
        ir::Arg::Value(ir::Expr::Call { object, routine, .. }) => ValueOrigin::Call {
            object: *object,
            routine: *routine,
        },
        _ => ValueOrigin::Other,
    }
}

fn year_of_day(day: i64) -> Option<i32> {
    i32::try_from(day).ok().map(|d| civil_from_day(d).0)
}

fn is_zone(v: &Value) -> bool {
    matches!(v, Value::DotNet(o) if matches!(&o.kind, DotNetKind::Host { clr_type, .. } if clr_type.eq_ignore_ascii_case(ZONE)))
}
