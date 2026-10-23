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
use al_builtins::native::Signal;
use al_builtins::value::Value;

const MS_PER_DAY: i64 = 86_400_000;

fn unsupported<T>(what: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(what.into()))
}

/// How far a read of the clock moves it on: the resolution of `DateTime` and `Time` (ADR-045).
pub(crate) const READ_STEP_MS: i64 = 1;

/// Day number (1 = 0001-01-01) of 1970-01-01.
fn unix_epoch_day() -> i64 {
    i64::from(day_from_civil(1970, 1, 1))
}

/// A `DateTime` value for an instant in milliseconds since the Unix epoch.
pub(crate) fn datetime_of_unix_millis(ms: i64) -> Value {
    Value::DateTime((unix_epoch_day() - 1) * MS_PER_DAY + ms + 1)
}

impl Session {
    /// One reading of the clock, in milliseconds since the Unix epoch, plus the time `Sleep` has
    /// waited.
    pub(crate) fn now_millis(&self) -> i64 {
        self.host.clock.now_utc_millis() + self.waited_ms
    }

    /// One reading of the clock by AL code (`Today`, `Time`, `CurrentDateTime`, the .NET clock
    /// members): the instant now, after which the clock moves on by [`READ_STEP_MS`], so the next
    /// reading is later (ADR-045). `Sleep` adds to the same counter, so there is one source.
    pub(crate) fn read_clock_millis(&mut self) -> i64 {
        let at = self.now_millis();
        self.waited_ms += READ_STEP_MS;
        at
    }

    /// Today as a day number.
    pub(crate) fn today_day_number(&self) -> i32 {
        (self.now_millis().div_euclid(MS_PER_DAY) + unix_epoch_day()) as i32
    }

    /// The instant for a system stamp: the clock, but always later than the previous stamp, so
    /// two writes in one millisecond still tell which came last (record/TestRecordSystemId.al:449,
    /// TestXRecStampFields.al:179 compare stamps with `<>` and `>`).
    pub(crate) fn stamp_millis(&mut self) -> i64 {
        let at = self.now_millis().max(self.last_stamp + 1);
        self.last_stamp = at;
        at
    }

    /// The builtins of this module, `None` for any other id.
    pub(crate) fn host_op(&mut self, id: al_ir::BuiltinId, args: &mut [Value]) -> Option<Result<Value, Signal>> {
        Some(match id {
            ids::SYSTEM_TODAY_1 => {
                let day = self.today_day_number();
                self.waited_ms += READ_STEP_MS;
                Ok(Value::Date(day, false))
            }
            ids::SYSTEM_TIME_1 => Ok(Value::Time(self.read_clock_millis().rem_euclid(MS_PER_DAY) + 1)),
            ids::SYSTEM_CURRENTDATETIME_1 => Ok(datetime_of_unix_millis(self.read_clock_millis())),
            ids::SYSTEM_WORKDATE_1 => self.work_date_op(args),
            ids::SYSTEM_SLEEP_1 => match args.first() {
                // A negative wait is not pinned.
                Some(Value::Integer(n)) if *n >= 0 => {
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
                    other.describe()
                )),
            },
            ids::SYSTEM_RANDOM_1 => self.random_op(args),
            ids::SYSTEM_RANDOMIZE_1 => match args.first() {
                // `new Random(seed)` (ALSystemNumeric.ALRandomize(int)); the sequence is .NET's.
                // The sequence is the session's: the next test of the codeunit goes on with it, no reset per test
                // (types/TestRandomSeededContracts.al:112-129).
                Some(Value::Integer(seed)) => {
                    self.reseeded_in_test = true;
                    self.rng = al_host::random::NetRandom::new(*seed);
                    Ok(Value::Null)
                }
                // BC seeds from the clock, so two calls give different sequences. Here the seed is
                // the host's plus a count, which differs per call and repeats per run (ADR-009).
                None => {
                    self.reseeded_in_test = true;
                    self.unseeded_randomizes = self.unseeded_randomizes.wrapping_add(1);
                    let seed = self.host.random_seed.wrapping_add(self.unseeded_randomizes);
                    self.rng = al_host::random::NetRandom::new(seed);
                    Ok(Value::Null)
                }
                Some(other) => unsupported(format!("Randomize with {}", other.describe())),
            },
            ids::DATABASE_USERID_1 => Ok(Value::Text(self.host.identity.user_id().into())),
            ids::DATABASE_USERSECURITYID_1 => Ok(Value::Guid(self.host.identity.user_security_id())),
            ids::DATABASE_SESSIONID_1 => Ok(Value::Integer(self.session_id())),
            // Only an id no session has is pinned (session/TestSessionComplete.al:45-52: 99999 is not active).
            ids::SESSION_ISSESSIONACTIVE_1 => match args.first() {
                Some(Value::Integer(n)) if *n != self.session_id() => Ok(Value::Boolean(false)),
                other => unsupported(format!(
                    "Session.IsSessionActive({other}): only an id other than the running session's is pinned",
                    other = other.describe()
                )),
            },
            // No session is started in a test run, so no id but the running session's is a session: StopSession answers false for
            // 0, a negative id, a huge id and the id a refused StartSession left (platform/lt-xmlport-session-navapp-cookie/
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
            ids::SYSTEM_GUIALLOWED_1 => Ok(Value::Boolean(self.host.identity.gui_allowed())),
            ids::SYSTEM_ISSERVICETIER_1 => Ok(Value::Boolean(self.host.identity.is_service_tier())),
            // The folder AL code writes scratch files to. BC's answer ends with `TEMP\` on the Linux tier
            // too (al-language-onprem/handlers/TestRequestPageSaveAsXmlContracts.al:TemporaryPath_EndsWithTheTempFolderAndABackslash);
            // the path starts with a separator, has no drive colon and ends with a separator
            // (al-language-onprem/lt-system-database-onprem/PTemporaryPath.al:P109_TemporaryPath_Shape), so it is `\TEMP\`; the files
            // directory (or the in-memory files) keeps it inside the run, a leading backslash being dropped there (docs/owner-review.md).
            ids::SYSTEM_TEMPORARYPATH_1 => Ok(Value::Text("\\TEMP\\".into())),
            ids::SYSTEM_WINDOWSLANGUAGE_1 => Ok(Value::Integer(self.host.identity.windows_language())),
            ids::SYSTEM_GLOBALLANGUAGE_1 => match args.first() {
                None => Ok(Value::Integer(self.global_language)),
                Some(Value::Integer(n)) if *n > 0 => {
                    self.global_language = *n;
                    self.events.event_stats.language.set(*n);
                    // A Code assignment under Turkish or Azerbaijani upper-cases `i` to `İ` (CaseConv.Codeunit.al:280-310).
                    al_builtins::ops::set_turkic_code(al_builtins::native::is_turkic_language(*n));
                    Ok(Value::Null)
                }
                other => unsupported(format!(
                    "GlobalLanguage with {other} (only a positive language id is pinned)",
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
                Some(t) => self.caption_class_translate(&t).map(|s| Value::Text(s.into())),
                None => unsupported("CaptionClassTranslate of something that is not text"),
            },
            // ADR-042: the host's server base address and the pinned query forms (geturl.rs).
            ids::SYSTEM_GETURL_1 | ids::SYSTEM_GETURL_2 | ids::SYSTEM_GETURL_3 | ids::SYSTEM_GETURL_4 => {
                self.get_url(args)
            }
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
            other => return unsupported(format!("Random with {}", other.describe())),
        };
        self.random_draws += 1;
        if !self.reseeded_in_test {
            self.inherited_draws += 1;
        }
        Ok(Value::Integer(self.rng.next_below(bound) + 1))
    }

    /// `WorkDate()` answers the date set, or today until AL sets one; `WorkDate(Date)` sets it
    /// (session/TestSessionFunctions.al:53-65, session/TestDateTimeContracts.al:150-158). The
    /// date is the session's, so a new session starts at today again (ADR-001).
    /// The work date as a day number: the date set, or today.
    pub(crate) fn work_date_day(&self) -> Option<i32> {
        Some(
            self.work_date
                .unwrap_or_else(|| (self.now_millis().div_euclid(MS_PER_DAY) + unix_epoch_day()) as i32),
        )
    }

    fn work_date_op(&mut self, args: &mut [Value]) -> Result<Value, Signal> {
        match args.first() {
            None => Ok(Value::Date(self.work_date_day().expect("always a date"), false)),
            Some(Value::Date(d, false)) if *d > 0 => {
                self.work_date = Some(*d);
                Ok(Value::Null)
            }
            // The undefined date sets the work date back to today (session/TestProbeSessionMiscContracts.al:
            // WorkDate_SetUndefined).
            Some(Value::Date(0, false)) => {
                self.work_date = None;
                Ok(Value::Null)
            }
            Some(other) => unsupported(format!(
                "WorkDate({other}): only a defined, non-closing Date is pinned",
                other = other.describe()
            )),
        }
    }
}

fn client_type(t: al_host::ClientType) -> Value {
    let members: std::sync::Arc<[String]> = al_builtins::sysenum::option_members("ClientType")
        .expect("a system option")
        .into();
    let ordinal = members
        .iter()
        .position(|m| m == t.name())
        .expect("the host names a member of ClientType");
    Value::Option(ordinal as i32, members)
}

// ---------------------------------------------------------------- system field stamps

const SYSTEM_CREATED_AT: u32 = 2_000_000_001;
const SYSTEM_CREATED_BY: u32 = 2_000_000_002;
const SYSTEM_MODIFIED_AT: u32 = 2_000_000_003;
const SYSTEM_MODIFIED_BY: u32 = 2_000_000_004;
/// The ids of the platform's system fields, SystemId (2000000000) to SystemRowVersion.
const SYSTEM_FIELDS: std::ops::RangeInclusive<u32> = 2_000_000_000..=2_000_000_005;

/// The values a stamp replaced, to give back when the write is refused.
pub(crate) type Replaced = Vec<(usize, Value)>;

fn position_of(t: &al_data::TableDef, id: u32) -> Option<usize> {
    t.fields.iter().position(|f| f.id == id)
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
    /// the user's security id, the same instant in both pairs (pagesave/TestPageSaveDataLayer.al:265).
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
    /// row has, whatever the buffer held. `stored` is the row before the write.
    pub(crate) fn stamp_modify(
        &mut self,
        t: &al_data::TableDef,
        rec: &crate::records::Rec,
        stored: Option<&[Value]>,
    ) -> Replaced {
        let Some(stored) = stored else {
            return Vec::new();
        };
        if rec.borrow().temporary {
            return Vec::new();
        }
        if self.bulk_modify || changed(t, stored, &rec.borrow().fields) {
            return self.stamp_modified_pair(t, rec);
        }
        let keep: Vec<(u32, Value)> = [SYSTEM_MODIFIED_AT, SYSTEM_MODIFIED_BY]
            .into_iter()
            .filter_map(|id| position_of(t, id).map(|i| (id, stored[i].clone())))
            .collect();
        self.write_stamps(t, rec, &keep)
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

    /// Gives back what `stamp_*` replaced, for a write the store refused.
    pub(crate) fn unstamp(rec: &crate::records::Rec, replaced: Replaced) {
        for (i, v) in replaced {
            rec.borrow_mut().fields[i] = v;
        }
    }
}
