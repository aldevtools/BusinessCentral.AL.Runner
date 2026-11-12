//! Builtins that answer from the host (ADR-006): the clock, the identity and the platform
//! answers of a service tier. The host is `al-host`'s `Host`; nothing here touches the OS.
//!
//! The model (see `al-host`): the session time zone is UTC, so `Today`, `Time` and
//! `CurrentDateTime` are views of one clock reading each. `Sleep` does not wait; it moves the
//! session's view of the clock forward, which is all AL can observe of a wait.

use crate::Session;
use al_builtins::builtin_id::ids;
use al_builtins::describe::Describe;
use al_builtins::format::day_from_civil;
use al_builtins::native::{Signal, unsupported};
use al_builtins::value::Value;

const MS_PER_DAY: i64 = 86_400_000;

/// Day number (1 = 0001-01-01) of 1970-01-01.
fn unix_epoch_day() -> i64 {
    i64::from(day_from_civil(1970, 1, 1))
}

/// A `DateTime` value for an instant in milliseconds since the Unix epoch.
pub(crate) fn datetime_of_unix_millis(ms: i64) -> Value {
    Value::DateTime((unix_epoch_day() - 1) * MS_PER_DAY + ms + 1)
}

/// The instant in milliseconds since the Unix epoch of a defined `DateTime` value (the inverse of
/// [`datetime_of_unix_millis`]).
pub(crate) fn unix_millis_of_datetime(v: i64) -> i64 {
    v - 1 - (unix_epoch_day() - 1) * MS_PER_DAY
}

/// The day number (1 = 0001-01-01) of an instant in milliseconds since the Unix epoch, as `Today` gives it.
pub(crate) fn day_number_of_unix_millis(ms: i64) -> i64 {
    ms.div_euclid(MS_PER_DAY) + unix_epoch_day()
}

impl Session {
    /// One reading of the clock, in milliseconds since the Unix epoch, plus the time `Sleep` has
    /// waited.
    pub(crate) fn now_millis(&self) -> i64 {
        self.host.clock.now_utc_millis() + self.waited_ms
    }

    /// One reading of the clock by AL code (`Today`, `Time`, `CurrentDateTime`, the .NET clock
    /// members): the instant now, after which the clock moves on by the host's step (1 ms, the resolution of `DateTime`
    /// and `Time`; `Host::clock_step_ms`), so the next
    /// reading is later (ADR-045). `Sleep` adds to the same counter, so there is one source.
    pub(crate) fn read_clock_millis(&mut self) -> i64 {
        let at = self.now_millis();
        self.waited_ms += self.host.clock_step_ms;
        at
    }

    /// Today as a day number.
    pub(crate) fn today_day_number(&self) -> i32 {
        (self.now_millis().div_euclid(MS_PER_DAY) + unix_epoch_day()) as i32
    }

    /// The instant for a system stamp: the clock, but always later than the previous stamp and a value the column holds, so
    /// two writes in one millisecond still tell which came last (record/TestRecordSystemId.al:449,
    /// TestXRecStampFields.al:179 compare stamps with `<>` and `>`).
    pub(crate) fn stamp_millis(&mut self) -> i64 {
        let now = self.now_millis();
        let mut at = now.max(self.last_stamp + 1);
        // A stamp is a value the SQL datetime column holds as it is (1/300 s, P130DateFilters.al:162-183): whether the
        // buffer holds a stamp rounded is not pinned (sqltime.rs `store`), and with such a value the question does not arise.
        while !crate::sqltime::holds_ms(at.rem_euclid(1000)) {
            at += 1;
        }
        self.last_stamp = at;
        // Many writes between two reads move the stamps past the clock. A stamp lies between the reads around it
        // (ADR-045 item 4) and SystemCreatedAt is the current time (record/TestSystemFieldStampsContracts.al:67), so the
        // clock catches up with the stamp: the next read is not earlier than it. It lands 0, 1 or 2 ms past the stamp,
        // drawn from the run seed (ADR-045 amendment 2026-10-31, owner): grid milliseconds are at least 3 apart, so two
        // reads in three right after a write are off the grid and change in a round trip through a Time or DateTime
        // field, as BC's wall clock does in 55 to 80 % of reads (al-runner issue 6524).
        self.clock_draws += 1;
        let draw = crate::ctx::raw_guid_for(self.clock_key, self.clock_draws);
        let past = (u64::from_be_bytes(draw[..8].try_into().expect("8 bytes")) % 3) as i64;
        self.waited_ms += at + past - now;
        at
    }

    /// The builtins of this module, `None` for any other id.
    pub(crate) fn host_op(&mut self, id: al_ir::BuiltinId, args: &mut [Value]) -> Option<Result<Value, Signal>> {
        // What a new session holds of these is not measured (bgsession.rs).
        if self.background_session {
            let what = match id {
                ids::SYSTEM_WORKDATE_1 => Some("WorkDate"),
                ids::SYSTEM_GLOBALLANGUAGE_1 => Some("GlobalLanguage"),
                ids::SYSTEM_GUIALLOWED_1 => Some("GuiAllowed"),
                ids::SESSION_CURRENTCLIENTTYPE_1 => Some("CurrentClientType"),
                _ => None,
            };
            if let Some(what) = what {
                return Some(self.refuse_in_background_session(what).map(|_| Value::Null));
            }
        }
        Some(match id {
            ids::SYSTEM_TODAY_1 => {
                let day = self.today_day_number();
                self.waited_ms += self.host.clock_step_ms;
                Ok(Value::Date(day, false))
            }
            ids::SYSTEM_TIME_1 => Ok(Value::Time(self.read_clock_millis().rem_euclid(MS_PER_DAY) + 1)),
            ids::SYSTEM_CURRENTDATETIME_1 => Ok(datetime_of_unix_millis(self.read_clock_millis())),
            ids::SYSTEM_WORKDATE_1 => self.work_date_op(args),
            ids::SYSTEM_SLEEP_1 => match args.first().map(|a| match a {
                // A Duration argument is the Integer it converts to, its milliseconds (al-runner issue 6301;
                // types/duration/D_Duration.al:DateTimeMinusDateTime_To_Integer).
                Value::Duration(_) => al_builtins::ops::convert(a.clone(), &al_builtins::value::RtType::Integer),
                other => Ok(other.clone()),
            }) {
                Some(Err(e)) => Err(e),
                // A negative wait is not pinned.
                Some(Ok(Value::Integer(n))) if n >= 0 => {
                    let n = &n;
                    self.waited_ms += i64::from(*n);
                    // The time asked for counts toward the test's time limit (al-runner issue 5605): past it, the test
                    // is stopped the way the CPU limit stops it, so a retry loop around Sleep whose exit never comes ends.
                    self.slept_in_test = self.slept_in_test.saturating_add(i64::from(*n));
                    if self.sleep_budget_passed() {
                        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
                        return Some(Err(Signal::Unsupported(crate::STOPPED.into())));
                    }
                    Ok(Value::Null)
                }
                other => unsupported(format!(
                    "Sleep with {} (only a non-negative number is pinned)",
                    other.and_then(Result::ok).describe()
                )),
            },
            ids::SYSTEM_RANDOM_1 => self.random_op(args),
            ids::SYSTEM_RANDOMIZE_1 => match args.first() {
                // `new Random(seed)` (ALSystemNumeric.ALRandomize(int)); the sequence is .NET's.
                // The sequence is the session's: the next test of the codeunit goes on with it, no reset per test
                // (types/TestRandomSeededContracts.al:112-129).
                Some(Value::Integer(seed)) => {
                    self.reseeded_in_test = true;
                    self.randomized_with_seed = true;
                    self.rng = al_host::random::NetRandom::new(*seed);
                    Ok(Value::Null)
                }
                // BC seeds from the clock, so two calls give different sequences. Here the seed is the test
                // method's derived seed (or the host's) plus a count, which differs per call and repeats with the
                // run seed (ADR-009, ADR-061).
                None => {
                    self.reseeded_in_test = true;
                    self.unseeded_randomizes = self.unseeded_randomizes.wrapping_add(1);
                    let seed = self.unseeded_base.wrapping_add(self.unseeded_randomizes);
                    self.rng = al_host::random::NetRandom::new(seed);
                    Ok(Value::Null)
                }
                Some(other) => unsupported(format!("Randomize with {}", other.describe())),
            },
            ids::DATABASE_USERID_1 => Ok(Value::Text(self.host.identity.user_id().into())),
            ids::DATABASE_USERSECURITYID_1 => Ok(Value::Guid(self.host.identity.user_security_id())),
            ids::DATABASE_SESSIONID_1 => Ok(Value::Integer(self.session_id())),
            // An id no session has is not active (session-system/TestSessionComplete.al:45-52: 99999; metadata/platform-tables/
            // PlTSession.al:295-296: 0 and 2147483647); the running session is (PlTSession.al:294 SessionId_Relations).
            ids::SESSION_ISSESSIONACTIVE_1 => match args.first() {
                Some(Value::Integer(n)) if *n == self.session_id() => Ok(Value::Boolean(true)),
                Some(Value::Integer(n)) if *n != self.session_id() => Ok(Value::Boolean(false)),
                other => unsupported(format!(
                    "Session.IsSessionActive({other}): only an id other than the running session's is pinned",
                    other = other.describe()
                )),
            },
            // No session is started in a test run, so no id but the running session's is a session: StopSession answers false for
            // 0, a negative id, a huge id and the id a refused StartSession left (metadata/session-system/
            // P94Session.al:Stop_*). The running session's own id is never stopped there and is not pinned: refused.
            ids::SESSION_STOPSESSION_1 => match args.first() {
                Some(Value::Integer(n)) if *n != self.session_id() => Ok(Value::Boolean(false)),
                other => unsupported(format!(
                    "Session.StopSession({other}): only an id other than the running session's is pinned",
                    other = other.describe()
                )),
            },
            ids::DATABASE_SERVICEINSTANCEID_1 => Ok(Value::Integer(self.host.identity.service_instance_id())),
            ids::DATABASE_TENANTID_1 => Ok(Value::Text(self.host.identity.tenant_id().into())),
            // One fixed value per run: the same at every call, not blank, at most 250 characters
            // (metadata/platform-tables/DbSerialNumber.al:33-41). The text itself is not pinned; AL Runner v2
            // answered `STANDALONE` too (al-runner issue 6055).
            ids::DATABASE_SERIALNUMBER_1 => Ok(Value::Text("STANDALONE".into())),
            // A step of the Contoso company build runs without a client (`set_no_client`).
            ids::SYSTEM_GUIALLOWED_1 if self.no_client => {
                al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::ContosoBuildNoClient);
                Ok(Value::Boolean(false))
            }
            // Install code has no client (errors/RlInstallUi.al:210, :239;
            // metadata/CmInstallLeads.al:51).
            ids::SYSTEM_GUIALLOWED_1 if self.install_without_client => Ok(Value::Boolean(false)),
            // A page background task worker has no client (pages/background-tasks/BjNoGuiDialogs.al:204 `gui=No`).
            ids::SYSTEM_GUIALLOWED_1 if self.pages.bg_worker.is_some() => Ok(Value::Boolean(false)),
            // `--no-client`: the test session has no client (pages/background-tasks/BjNoGuiDialogs.al:8, as the worker above).
            ids::SYSTEM_GUIALLOWED_1 if self.ui_state.clientless => {
                al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::NoClientSession);
                Ok(Value::Boolean(false))
            }
            ids::SYSTEM_GUIALLOWED_1 => Ok(Value::Boolean(self.host.identity.gui_allowed())),
            ids::SYSTEM_ISSERVICETIER_1 => Ok(Value::Boolean(self.host.identity.is_service_tier())),
            // The folder AL code writes scratch files to. BC's answer ends with `TEMP\` on the Linux tier
            // too (al-language-onprem/reports/TestRequestPageSaveAsXmlContracts.al:TemporaryPath_EndsWithTheTempFolderAndABackslash);
            // the path starts with a separator, has no drive colon and ends with a separator
            // (al-language-onprem/session-system/server-system-functions/PTemporaryPath.al:P109_TemporaryPath_Shape), so it is `\TEMP\`; the files
            // directory (or the in-memory files) keeps it inside the run, a leading backslash being dropped there (docs/owner-review.md).
            ids::SYSTEM_TEMPORARYPATH_1 => Ok(Value::Text("\\TEMP\\".into())),
            ids::SYSTEM_WINDOWSLANGUAGE_1 => Ok(Value::Integer(self.host.identity.windows_language())),
            ids::SYSTEM_GLOBALLANGUAGE_1 => match args.first() {
                None => Ok(Value::Integer(self.global_language)),
                // These ids raise NotSupportedLanguage and leave the language as it was
                // (metadata/platform-tables/PlTSession.al:245-266, GlobalLanguage_InvalidIds: 0, -1, 99999;
                // :326-360, GlobalLanguage_PositiveIds: 1000, 4096, 8192, 9999, 32767, 65535). Other ids are set as before.
                Some(Value::Integer(n @ (0 | -1 | 99999 | 1000 | 4096 | 8192 | 9999 | 32767 | 65535))) => {
                    Err(crate::transactions::al_error_code(
                        &format!(
                            "'{n}' is not a valid language code. You must specify a locale identifier (LCID) or code in the format of language-country/region, such as en-US, de-DE, or da-DK."
                        ),
                        "NotSupportedLanguage",
                    ))
                }
                Some(Value::Integer(n)) if *n > 0 => {
                    self.global_language = *n;
                    self.events.event_stats.language.set(*n);
                    // A Code assignment under Turkish or Azerbaijani upper-cases `i` to `İ` (CaseConv.Codeunit.al:280-310).
                    al_builtins::ops::set_turkic_code(al_builtins::native::is_turkic_language(*n));
                    al_builtins::filter::set_filter_language(*n);
                    Ok(Value::Null)
                }
                other => unsupported(format!(
                    "GlobalLanguage with {other} (only a positive language id, 0 and -1 are pinned)",
                    other = other.describe()
                )),
            },
            // SelectLatestVersion drops what CaptionClassTranslate kept (types/PSCaptionClass.al:103-115); the session reads the
            // store as it is, so there is nothing else to refresh.
            ids::DATABASE_SELECTLATESTVERSION_1 | ids::DATABASE_SELECTLATESTVERSION_2 => {
                self.caption_translations.clear();
                Ok(Value::Null)
            }
            // corpus types/PSCaptionClass.al:5-8, :58-81 (CaptionClassTranslate_Expressions).
            ids::SYSTEM_CAPTIONCLASSTRANSLATE_1 => match args.first().and_then(al_builtins::native::text_of) {
                Some(t) => self
                    .caption_class_translate_statement(&t)
                    .map(|s| Value::Text(s.into())),
                None => unsupported("CaptionClassTranslate of something that is not text"),
            },
            // ADR-042: the host's server base address and the pinned query forms (geturl.rs).
            ids::SYSTEM_GETURL_1 | ids::SYSTEM_GETURL_2 | ids::SYSTEM_GETURL_3 | ids::SYSTEM_GETURL_4 => {
                self.get_url(args)
            }
            // Install code runs in a SOAP session on the corpus CI's publish path, the same on BC 27.5, 28.5 and 29.0
            // (errors/RlInstallUi.al:211).
            ids::SESSION_CURRENTCLIENTTYPE_1 if self.install_without_client => Ok(client_type_named("SOAP")),
            ids::SESSION_CURRENTCLIENTTYPE_1 => Ok(client_type(self.host.identity.current_client_type())),
            ids::SESSION_DEFAULTCLIENTTYPE_1 => Ok(client_type(self.host.identity.default_client_type())),
            _ => return None,
        })
    }

    /// `Random(MaxNumber)`: 1 to `MaxNumber`. The documented range is the only part the corpus
    /// states (corpus/scripts/al-surface.json, system-random-method); the rest follows the
    /// service tier's `ALSystemNumeric.ALRandom` and is recorded as an assumption in
    /// docs/probes-needed.md: a negative bound counts as its absolute value, 0 as 1, and the
    /// number is `Next(bound) + 1` on the session's generator.
    fn random_op(&mut self, args: &mut [Value]) -> Result<Value, Signal> {
        let bound = match args.first() {
            Some(Value::Integer(n)) if *n == i32::MIN => {
                // The absolute value of the smallest integer overflows
                // (types/TestRandomSeededContracts.al:84-92, BC 27.5).
                return Err(crate::transactions::al_error_code(
                    "Arithmetic operation resulted in an overflow.",
                    "AL:Overflow",
                ));
            }
            Some(Value::Integer(n)) => n.abs().max(1),
            // A Decimal: a whole one draws as that Integer (`Random(1.0)` is always 1, `Random(3.0)` 1 to 3), one with a fraction
            // is the conversion error (format-evaluate/AjScalars.al:375-433, Random_Decimal_*: 0.4, 1.5, 2.5, 2.9). A whole
            // Decimal beyond an Integer is not pinned.
            Some(Value::Decimal(d)) if !d.fract().is_zero() => {
                return Err(crate::transactions::al_error_code(
                    &format!(
                        "Overflow under conversion of Microsoft.Dynamics.Nav.Runtime.Decimal18 value {d} to System.Int32."
                    ),
                    "ConversionCast",
                ));
            }
            Some(Value::Decimal(d)) if i32::try_from(d.trunc().mantissa() / 10i128.pow(d.trunc().scale())).is_ok() => {
                let n = i32::try_from(d.trunc().mantissa() / 10i128.pow(d.trunc().scale())).unwrap_or(1);
                if n == i32::MIN {
                    return unsupported("Random with the smallest Integer as a Decimal (not pinned)");
                }
                n.abs().max(1)
            }
            other => return unsupported(format!("Random with {}", other.describe())),
        };
        self.random_draws += 1;
        if !self.reseeded_in_test {
            self.inherited_draws += 1;
            if !self.run_seeded_in_test {
                self.carried_draws += 1;
            }
        }
        Ok(Value::Integer(self.rng.next_below(bound) + 1))
    }

    /// `WorkDate()` answers the date set, or today until AL sets one; `WorkDate(Date)` sets it
    /// (session-system/TestSessionFunctions.al:53-65, session-system/TestDateTimeContracts.al:150-158). The
    /// date is the session's, so a new session starts at today again (ADR-001).
    /// The work date as a day number: the date set, or today.
    pub(crate) fn work_date_day(&self) -> Option<i32> {
        // `--no-client`: the date nothing set is the one the skipped login code would have replaced (`clientless.rs`).
        if self.work_date.is_none() {
            self.clientless_work_date_read();
        }
        Some(
            self.work_date
                .unwrap_or_else(|| (self.now_millis().div_euclid(MS_PER_DAY) + unix_epoch_day()) as i32),
        )
    }

    fn work_date_op(&mut self, args: &mut [Value]) -> Result<Value, Signal> {
        match args.first() {
            None => Ok(Value::Date(self.work_date_day().expect("always a date"), false)),
            // The call returns the work date it set, so it is usable as an expression
            // (session-system/session/R6862WorkDateExpression.al, BC 28.5 and 29.0).
            Some(Value::Date(d, false)) if *d > 0 => {
                self.work_date = Some(*d);
                Ok(Value::Date(*d, false))
            }
            // The undefined date sets the work date back to today (session-system/TestProbeSessionMiscContracts.al:
            // WorkDate_SetUndefined).
            // The call returns that date (R6862WorkDateExpression.al: `empty: result = Today`).
            Some(Value::Date(0, false)) => {
                self.work_date = None;
                Ok(Value::Date(self.work_date_day().expect("always a date"), false))
            }
            Some(other) => unsupported(format!(
                "WorkDate({other}): only a defined, non-closing Date is pinned",
                other = other.describe()
            )),
        }
    }
}

fn client_type(t: al_host::ClientType) -> Value {
    client_type_named(t.name())
}

fn client_type_named(name: &str) -> Value {
    let members: std::sync::Arc<[String]> = al_builtins::sysenum::option_members("ClientType")
        .expect("a system option")
        .into();
    let ordinal = members.iter().position(|m| m == name).expect("a member of ClientType");
    Value::Option(ordinal as i32, members)
}

// ---------------------------------------------------------------- system field stamps

const SYSTEM_CREATED_AT: u32 = 2_000_000_001;
const SYSTEM_CREATED_BY: u32 = 2_000_000_002;
const SYSTEM_MODIFIED_AT: u32 = 2_000_000_003;
const SYSTEM_MODIFIED_BY: u32 = 2_000_000_004;
const SYSTEM_ROW_VERSION: u32 = 2_000_000_005;
/// The ids of the platform's system fields, SystemId (2000000000) to SystemRowVersion.
const SYSTEM_FIELDS: std::ops::RangeInclusive<u32> = 2_000_000_000..=2_000_000_005;

/// The values a stamp replaced, to give back when the write is refused.
pub(crate) type Replaced = Vec<(usize, Value)>;

fn position_of(t: &al_data::TableDef, id: u32) -> Option<usize> {
    t.fields.iter().position(|f| f.id == id)
}

/// A `Modify(false)` without modify-event subscribers that changes no field, of a buffer that holds the stored row's version and differs from it in exactly one
/// field of the modified pair (the test assigned SystemModifiedAt or SystemModifiedBy): the stored row keeps its pair and the
/// buffer keeps the assigned value (record/system-fields/AssignedAuditFields.al:271-295, `buffer after Modify` = assigned,
/// `stored row` = same as before). Every other buffer gets the stored pair, as before.
fn no_change_keeps_buffer_pair(
    t: &al_data::TableDef,
    stored: &[Value],
    buffer: &[Value],
    buffer_version: Option<i64>,
) -> bool {
    let same = |i: usize| al_builtins::ops::compare(&stored[i], &buffer[i]).ok() == Some(std::cmp::Ordering::Equal);
    let Some(v) = position_of(t, SYSTEM_ROW_VERSION) else {
        return false;
    };
    if buffer_version.is_none() || !matches!(&stored[v], Value::BigInteger(n) if Some(*n) == buffer_version) {
        return false;
    }
    let differing = [SYSTEM_MODIFIED_AT, SYSTEM_MODIFIED_BY]
        .into_iter()
        .filter_map(|id| position_of(t, id))
        .filter(|&i| !same(i))
        .count();
    differing == 1
}

/// The system fields a `Modify` writes from the stored row, whatever the buffer holds: SystemId, and for a database record the
/// created pair (an assigned SystemCreatedAt or SystemCreatedBy stays in the buffer only,
/// record/system-fields/AssignedAuditFields.al:243-267, :301-325). A temporary record writes the created pair the buffer holds
/// (AssignedAuditFields.al:638-666 `blank / assigned / assigned / assigned` after SystemCreatedAt set blank and Modify).
pub(crate) fn modify_kept(t: &al_data::TableDef, stored: Option<&[Value]>, temporary: bool) -> Replaced {
    let Some(old) = stored else {
        return Vec::new();
    };
    let ids: &[u32] = if temporary {
        &[2_000_000_000]
    } else {
        &[2_000_000_000, SYSTEM_CREATED_AT, SYSTEM_CREATED_BY]
    };
    ids.iter()
        .filter_map(|id| position_of(t, *id))
        .map(|i| (i, old[i].clone()))
        .collect()
}

/// The row a `Rename` of a database record writes: the buffer under its new key, with the created pair of the stored row
/// (record/system-fields/AssignedAuditFields.al:568-590: after SystemCreatedAt and SystemCreatedBy were assigned, the buffer holds
/// them and the renamed row the stored row's). A temporary record writes the buffer as it is.
pub(crate) fn rename_row(
    t: &al_data::TableDef,
    buffer: &[Value],
    stored: Option<&[Value]>,
    temporary: bool,
) -> Vec<Value> {
    let mut row = buffer.to_vec();
    if let (Some(old), false) = (stored, temporary) {
        for i in [SYSTEM_CREATED_AT, SYSTEM_CREATED_BY]
            .into_iter()
            .filter_map(|id| position_of(t, id))
        {
            row[i] = old[i].clone();
        }
    }
    row
}

/// The error text of `ModifyAll` on an audit field of a database table: SystemCreatedAt, SystemCreatedBy and SystemModifiedAt cannot
/// be changed (record/system-fields/AssignedAuditFields.al:594-632, one row, without RunTrigger, code `DB:Ok`, which the caller
/// sets). The text names the field by its name, not its caption (Created At), and the table by its name, which is its caption
/// there. `None` for any other field. SystemModifiedBy, another row count, RunTrigger, a temporary record and a table whose caption
/// differs from its name are not pinned and are refused.
pub(crate) fn modify_all_audit_error(
    t: &al_data::TableDef,
    pos: usize,
    rows: usize,
    run_trigger: bool,
    temporary: bool,
    table_caption_is_name: bool,
) -> Option<Result<String, Signal>> {
    let id = t.fields[pos].id;
    if !(SYSTEM_CREATED_AT..=SYSTEM_MODIFIED_BY).contains(&id) {
        return None;
    }
    // The pinned cell, every fact by equality: one of the three measured fields, one row, RunTrigger false, a database record,
    // a table named as its caption.
    let pinned = matches!(id, SYSTEM_CREATED_AT | SYSTEM_CREATED_BY | SYSTEM_MODIFIED_AT)
        && rows == 1
        && !run_trigger
        && !temporary
        && table_caption_is_name;
    if !pinned {
        return Some(Err(Signal::unsupported(format!(
            "ModifyAll on {} over other than one row, with RunTrigger, on a temporary record, of a table with a caption, or on SystemModifiedBy (not pinned)",
            t.fields[pos].name
        ))));
    }
    let br = crate::records::BR;
    Some(Ok(format!(
        "You cannot change the value of the {} field.{br}{br}Table: {}",
        t.fields[pos].name, t.name
    )))
}

/// Whether a write changed anything: any field but the system fields differs.
fn changed(t: &al_data::TableDef, stored: &[Value], buffer: &[Value]) -> bool {
    t.fields.iter().enumerate().any(|(i, f)| {
        if SYSTEM_FIELDS.contains(&f.id) || !f.stored {
            return false;
        }
        match (&stored[i], &buffer[i]) {
            (Value::Blob(a), Value::Blob(b)) => {
                b.borrow().written || (!std::sync::Arc::ptr_eq(a, b) && a.borrow().bytes != b.borrow().bytes)
            }
            (a, b) => al_builtins::ops::compare(a, b).map_or(true, |o| o != std::cmp::Ordering::Equal),
        }
    })
}

impl Session {
    /// `SystemCreatedAt/By` and `SystemModifiedAt/By` of a row the platform inserts: the clock and
    /// the user's security id, the same instant in both pairs (pages/record-save/TestPageSaveDataLayer.al:265).
    /// They are written below the trigger, so `OnInsert` and `OnBeforeInsertEvent` still see the
    /// default values and the buffer holds the stamps when `Insert` returns
    /// (record/TestXRecStampFields.al:75-99, TestStorageLayerFieldWriteRoutes.al:114-125).
    ///
    /// A temporary record is not stamped: the stamps are written by the SQL layer, which a
    /// temporary record never reaches (docs/probes-needed.md; unpinned).
    pub(crate) fn stamp_insert(&mut self, t: &al_data::TableDef, rec: &crate::records::Rec) -> Replaced {
        if rec.borrow().temporary {
            return Vec::new();
        }
        let at = datetime_of_unix_millis(self.stamp_millis());
        let by = Value::Guid(self.host.identity.user_security_id());
        self.write_stamps(
            t,
            rec,
            &[
                (SYSTEM_CREATED_AT, at.clone()),
                (SYSTEM_CREATED_BY, by.clone()),
                (SYSTEM_MODIFIED_AT, at),
                (SYSTEM_MODIFIED_BY, by),
            ],
        )
    }

    /// Whether a `Modify` of `rec` writes nothing new over the stored row `stored` (no field but the system fields differs).
    pub(crate) fn modify_changes_nothing(t: &al_data::TableDef, stored: &[Value], rec: &crate::records::Rec) -> bool {
        !changed(t, stored, &rec.borrow().fields)
    }

    /// `SystemModifiedAt/By` of a row a `Modify` writes, when the write changes a field
    /// (record/TestRecordSystemId.al:677-750). A write that changes none keeps the pair the stored
    /// row has, whatever the buffer held. `stored` is the row before the write. Returns what the stamp
    /// replaced in the buffer and the values only the written row takes (see `no_change_keeps_buffer_pair`).
    /// `buffer_version` is the buffer's SystemRowVersion before a `Modify(false)`, `None` for any other write.
    pub(crate) fn stamp_modify(
        &mut self,
        t: &al_data::TableDef,
        rec: &crate::records::Rec,
        stored: Option<&[Value]>,
        buffer_version: Option<i64>,
    ) -> (Replaced, Replaced) {
        let Some(stored) = stored else {
            return (Vec::new(), Vec::new());
        };
        if rec.borrow().temporary {
            return (Vec::new(), Vec::new());
        }
        if self.bulk_modify || changed(t, stored, &rec.borrow().fields) {
            return (self.stamp_modified_pair(t, rec), Vec::new());
        }
        let keep: Vec<(u32, Value)> = [SYSTEM_MODIFIED_AT, SYSTEM_MODIFIED_BY]
            .into_iter()
            .filter_map(|id| position_of(t, id).map(|i| (id, stored[i].clone())))
            .collect();
        // The measured table has no modify-event subscriber and no trigger (AssignedAuditFields.al:17-27).
        let subscribed = |ev: &str| self.engine.events.is_subscribed(t.object as usize, ev);
        let no_subscriber = !subscribed("onbeforemodifyevent") && !subscribed("onaftermodifyevent");
        if no_subscriber && no_change_keeps_buffer_pair(t, stored, &rec.borrow().fields, buffer_version) {
            let row_only = keep
                .into_iter()
                .filter_map(|(id, v)| position_of(t, id).map(|i| (i, v)))
                .collect();
            return (Vec::new(), row_only);
        }
        (self.write_stamps(t, rec, &keep), Vec::new())
    }

    /// `SystemModifiedAt/By` of a renamed row. A rename always stamps, also to the key the row
    /// has (record/TestRecordSystemId.al:580-675,752-775); the created pair and SystemId stay.
    pub(crate) fn stamp_rename(&mut self, t: &al_data::TableDef, rec: &crate::records::Rec) -> Replaced {
        if rec.borrow().temporary {
            return Vec::new();
        }
        self.stamp_modified_pair(t, rec)
    }

    /// The modified pair of a row that is not in a record buffer (a row a rename of another table
    /// re-keys), if `row`'s table has one. Assumed to stamp like any other write
    /// (docs/probes-needed.md; unpinned).
    pub(crate) fn stamp_row_modified(&mut self, t: &al_data::TableDef, row: &mut [Value]) {
        let at = datetime_of_unix_millis(self.stamp_millis());
        let by = Value::Guid(self.host.identity.user_security_id());
        for (id, v) in [(SYSTEM_MODIFIED_AT, at), (SYSTEM_MODIFIED_BY, by)] {
            if let Some(i) = position_of(t, id) {
                row[i] = v;
            }
        }
    }

    fn stamp_modified_pair(&mut self, t: &al_data::TableDef, rec: &crate::records::Rec) -> Replaced {
        let at = datetime_of_unix_millis(self.stamp_millis());
        let by = Value::Guid(self.host.identity.user_security_id());
        self.write_stamps(t, rec, &[(SYSTEM_MODIFIED_AT, at), (SYSTEM_MODIFIED_BY, by)])
    }

    fn write_stamps(&mut self, t: &al_data::TableDef, rec: &crate::records::Rec, values: &[(u32, Value)]) -> Replaced {
        let mut replaced = Vec::new();
        for (id, v) in values {
            if let Some(i) = position_of(t, *id) {
                let old = std::mem::replace(&mut rec.borrow_mut().fields[i], v.clone());
                replaced.push((i, old));
            }
        }
        replaced
    }

    /// The row a `Rename` writes (`rename_row`), with the stored row under the old key `old_pk`.
    pub(crate) fn rename_row(
        &mut self,
        cx: &al_data::AccessContext,
        t: &al_data::TableDef,
        rec: &crate::records::Rec,
        old_pk: &[Value],
    ) -> Result<Vec<Value>, Signal> {
        let temporary = rec.borrow().temporary;
        let stored = if temporary {
            None
        } else {
            al_data::Store::get(self.store_for(rec), cx, t, old_pk).map_err(|e| Signal::unsupported(format!("{e}")))?
        };
        Ok(rename_row(t, &rec.borrow().fields, stored.as_deref(), temporary))
    }

    /// Gives back what `stamp_*` replaced, for a write the store refused.
    pub(crate) fn unstamp(rec: &crate::records::Rec, replaced: Replaced) {
        for (i, v) in replaced {
            rec.borrow_mut().fields[i] = v;
        }
    }
}
