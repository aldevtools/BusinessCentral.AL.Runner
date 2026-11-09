//! A trial run of a subscriber whose install order is not known (al-runner issues 6972 and 6644, ADR-046 amendment 8).
//!
//! While an app X installs, a subscriber S of an app P with no dependency path to X runs exactly when BC installed P
//! first (corpus install-upgrade/install-triggers/Y79InstallUnrelated.al:6-12), and that order is not known
//! (`dispatch.rs`, `order_unknown_apps`). Instead of refusing at once, the raise runs S once under watch and keeps the
//! result only when running S and not running it leave the same database:
//!
//! - **S wrote nothing that lasts:** no row of a table that is not temporary (`record_op_inner`), no isolated storage, no
//!   number sequence, no AutoIncrement counter, no commit of the caller's pending writes, no WorkDate or GlobalLanguage
//!   change, no page, report or .NET object.
//! - **S read only tables whose rows do not depend on the order:** none that the install code of the apps whose order is
//!   open may write (`Session::set_install_written_tables`; when that is not known, no table at all). What S reads is
//!   then the same in both orders, and S's SingleInstance state starts fresh in each install session (one session per
//!   app, ADR-046 A4), so S computes the same in the order where it runs.
//! - **S changed nothing its caller sees:** every `var` argument it gets (other than `Sender`) holds a plain value, and it
//!   holds the same value after S, every other argument is a plain value or a record (a copy) (al-runner issue 6972: a trigger setup subscriber that answers from its own app's empty
//!   setup table through a SingleInstance cache leaves the flags as they were), and S entered no codeunit of another app
//!   that holds state, beyond what the next point allows.
//! - **Or S only handed something to `Sender`** (al-runner issue 6644 a, `Sender.Register(MyLogger)` of System
//!   Application's "Telemetry Loggers"): S entered state of a codeunit of the publisher's app that is not SingleInstance,
//!   so the change lives in the publisher's instance and ends with it. What the publisher then calls on what S handed
//!   over is code of P: from then on, until the next app installs, a write of a non-temporary row while code of P is on
//!   the call stack is refused as before (`Session::trial_guard_write`). That the publisher's own code writes the same rows
//!   whether or not a subscriber registered something is the plan stated on issue 6644 (owner side, 2026-11-09); it is
//!   recorded as decided without owner review in ADR-046 amendment 8.
//!
//! What S changes in its own app's memory (its SingleInstance cache) only code of P reads, and code of P can only run in
//! X's install through P's subscribers, which each pass through this trial or are refused.
//!
//! Anything else (an error, a refusal inside S, a write, a read of a table install code may write) is the refusal it was
//! before: the install code stops, and the pair goes into the comparison of the orders (`take_order_unknown_hits`).

use crate::Session;
use crate::impact::Touches;
use al_builtins::value::Value;
use std::collections::BTreeSet;

/// The trial state of one session (`EventState::trial`).
#[derive(Debug, Default)]
pub(crate) struct TrialState {
    /// Set by `raise_to_subscribers` while it picks its targets: a subscriber that would be refused for the install order
    /// may be picked for a trial instead.
    pub allowed: bool,
    /// The subscribers (object, routine) picked for a trial in the last pick.
    pub picked: Vec<(usize, usize)>,
    /// Above 0 while a trial runs (no trial inside a trial).
    pub depth: u32,
    /// The first table of a write that lasts during the trial.
    pub wrote: Option<u32>,
    /// The tables (not temporary) read during the trial.
    pub read: BTreeSet<u32>,
    /// Apps whose subscriber handed something to its publisher: their code writes no row until the next app installs.
    pub guarded: BTreeSet<u32>,
}

/// What a trial showed.
pub(crate) enum Verdict {
    /// Running S and not running it leave the same database.
    NoEffect,
    /// S handed something to `Sender`; its app's code must write nothing afterwards.
    HandedToSender,
}

/// What the trial watches besides the rows: the state that lasts beyond S and is not a table.
pub(crate) struct Watch {
    isolated: std::collections::HashMap<(String, String), (String, bool, bool)>,
    sequences: std::collections::HashMap<(String, String), crate::sessionops::Sequence>,
    autoinc: std::collections::HashMap<(String, u32), i64>,
    work_date: Option<i32>,
    language: i32,
    uncommitted: bool,
    outer_trace: Option<Touches>,
    outer_on: bool,
}

/// Whether `v` is a plain value whose equality before and after says whether S changed it.
fn plain(v: &Value) -> bool {
    matches!(
        v,
        Value::Null
            | Value::Boolean(_)
            | Value::Integer(_)
            | Value::BigInteger(_)
            | Value::Decimal(_)
            | Value::Byte(_)
            | Value::Char(_)
            | Value::Text(_)
            | Value::Code(_)
            | Value::Date(..)
            | Value::Time(_)
            | Value::DateTime(_)
            | Value::Duration(_)
            | Value::Guid(_)
            | Value::DateFormula(_)
            | Value::Option(..)
            | Value::Enum(..)
    )
}

/// Whether two plain values are the same (their encodings agree).
fn same(a: &Value, b: &Value) -> bool {
    let (mut x, mut y) = (Vec::new(), Vec::new());
    al_builtins::value_codec::encode_row(std::slice::from_ref(a), &mut x)
        && al_builtins::value_codec::encode_row(std::slice::from_ref(b), &mut y)
        && x == y
}

impl Session {
    /// Starts a trial: notes what lasts, and watches the routines S enters.
    pub(crate) fn trial_start(&mut self) -> Watch {
        let t = &mut self.events.trial;
        t.depth += 1;
        t.wrote = None;
        t.read.clear();
        let outer_trace = self.coverage.impact.borrow_mut().replace(Touches::default());
        let outer_on = std::mem::replace(&mut self.coverage.impact_on, true);
        Watch {
            isolated: self.isolated_storage.clone(),
            sequences: self.sequences.clone(),
            autoinc: self.autoinc.clone(),
            work_date: self.work_date,
            language: self.global_language,
            uncommitted: self.store.has_uncommitted(),
            outer_trace,
            outer_on,
        }
    }

    /// Ends a trial of subscriber `(object, routine)` that got `before` and gave back `after` (`by_var`, `sender`: per
    /// parameter, whether it is `var`, and whether it is the publisher's `Sender`; `publisher`: the publisher object).
    /// `Err` with why S may make the install order visible (module comment).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn trial_end(
        &mut self,
        w: Watch,
        object: usize,
        before: &[Value],
        after: Option<&[Value]>,
        by_var: &[bool],
        sender: &[bool],
        publisher: usize,
    ) -> Result<Verdict, String> {
        let touched = std::mem::replace(&mut *self.coverage.impact.borrow_mut(), w.outer_trace).unwrap_or_default();
        self.coverage.impact_on = w.outer_on;
        if let Some(outer) = self.coverage.impact.borrow_mut().as_mut() {
            merge(outer, &touched);
        }
        self.events.trial.depth -= 1;
        let objects = &self.engine.program.objects;
        let name = |t: u32| objects.get(t as usize).map_or("?", |o| o.name.as_str()).to_string();
        let Some(after) = after else {
            return Err("it stopped with an error or a refusal".into());
        };
        if let Some(t) = self.events.trial.wrote {
            return Err(format!("it writes {}", name(t)));
        }
        let read = std::mem::take(&mut self.events.trial.read);
        match &self.events.install_written {
            None if !read.is_empty() => {
                return Err("it reads a table and the tables install code writes are not known".into());
            }
            Some(written) => {
                if let Some(t) = read.iter().find(|t| written.contains(t)) {
                    return Err(format!("it reads {}, which install code may write", name(*t)));
                }
            }
            None => {}
        }
        if self.isolated_storage != w.isolated
            || self.sequences != w.sequences
            || self.autoinc != w.autoinc
            || self.work_date != w.work_date
            || self.global_language != w.language
            || (w.uncommitted && !self.store.has_uncommitted())
        {
            return Err(
                "it changes isolated storage, a number sequence, a counter, WorkDate, the language or commits".into(),
            );
        }
        if touched.dotnet || !touched.pages.is_empty() || !touched.reports.is_empty() {
            return Err("it uses a page, a report or .NET".into());
        }
        for (i, b) in before.iter().enumerate() {
            if sender.get(i).copied().unwrap_or(false) {
                continue;
            }
            if !by_var.get(i).copied().unwrap_or(false) {
                // A value argument the caller shares (a RecordRef, a List, a codeunit) may have been changed through it; a
                // record argument is a copy (ADR-010).
                if !plain(b) && !matches!(b, Value::Record(_)) {
                    return Err("it gets an argument the caller shares".into());
                }
                continue;
            }
            let a = after.get(i).unwrap_or(&Value::Null);
            if !plain(b) || !plain(a) || !same(a, b) {
                return Err("it changes a var argument".into());
            }
        }
        let app = objects[object].app;
        let publisher_app = objects.get(publisher).map(|o| o.app);
        let mut handed = false;
        for &o in &touched.entered {
            let obj = &objects[o];
            if obj.app == app || obj.globals.is_empty() || obj.is_kind(al_ir::ObjKind::Table) {
                continue;
            }
            let single = obj
                .properties
                .iter()
                .any(|(k, v)| k.eq_ignore_ascii_case("SingleInstance") && v.trim().eq_ignore_ascii_case("true"));
            if obj.is_kind(al_ir::ObjKind::Codeunit)
                && !single
                && sender.contains(&true)
                && Some(obj.app) == publisher_app
            {
                handed = true;
                continue;
            }
            return Err(format!("it enters {}, which holds state", obj.name));
        }
        Ok(if handed {
            Verdict::HandedToSender
        } else {
            Verdict::NoEffect
        })
    }

    /// A write of a non-temporary row of `table`: noted during a trial; refused while code of an app whose subscriber
    /// handed something to its publisher is on the call stack (module comment).
    pub(crate) fn trial_guard_write(&mut self, table: u32) -> Result<(), al_builtins::native::Signal> {
        if self.events.trial.depth > 0 && self.events.trial.wrote.is_none() {
            self.events.trial.wrote = Some(table);
        }
        if self.events.trial.guarded.is_empty() {
            return Ok(());
        }
        let objects = &self.engine.program.objects;
        let Some(app) = self
            .frames
            .iter()
            .map(|&o| objects[o].app)
            .find(|a| self.events.trial.guarded.contains(a))
        else {
            return Ok(());
        };
        self.note_order_unknown_hit(app);
        Err(al_builtins::native::Signal::Unsupported(format!(
            "a write to {} by code of app {} while an app installs that it has no dependency path to; the install order of \
             the two is not known",
            objects.get(table as usize).map_or("?", |o| o.name.as_str()),
            self.engine
                .program
                .apps
                .get(app as usize)
                .map_or("?", |a| a.name.as_str())
        )))
    }

    /// A record operation on `table` (`record_op_inner`): noted for the impact trace (`Session::note_table`), and when the
    /// record is not temporary, a write passes [`Session::trial_guard_write`] and a read is noted during a trial.
    pub(crate) fn note_table_op(
        &mut self,
        table: u32,
        write: bool,
        temporary: bool,
    ) -> Result<(), al_builtins::native::Signal> {
        self.note_table(table, write);
        if temporary {
            return Ok(());
        }
        if write {
            return self.trial_guard_write(table);
        }
        if self.events.trial.depth > 0 {
            self.events.trial.read.insert(table);
        }
        Ok(())
    }
}

fn merge(outer: &mut Touches, t: &Touches) {
    outer.events.extend(t.events.iter().cloned());
    outer.tables.extend(&t.tables);
    outer.writes.extend(&t.writes);
    outer.pages.extend(&t.pages);
    outer.reports.extend(&t.reports);
    outer.enums.extend(&t.enums);
    outer.entered.extend(&t.entered);
    outer.dotnet |= t.dotnet;
    outer.routines.extend(&t.routines);
}
