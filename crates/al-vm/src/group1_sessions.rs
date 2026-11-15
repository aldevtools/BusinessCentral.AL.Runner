//! The filters in filter group 1 (the global group) a session holds, per table, and when they end.
//!
//! In-session: BC keeps a group 1 filter on every record of the table for the rest of the session (records.rs
//! `set_global_group_filter`, `global_group_check`).
//!
//! Across sessions (assumption `global-filter-ends-with-install-session`, corpus PR 1405, open: tests/al-language/
//! install-upgrade/G1sInstallSessionTests.al with tests/al-language-xapp-a/G1sXaInstall.al and
//! tests/al-language-xapp-b/G1sXbInstall.al, measured alike on all three supported BC majors): a filter that a Record variable set
//! with SetFilter or SetRange in filter group 1, in the per-company part of an app's install session in the session
//! company, in the body of one of two routines ([`Group1InstallShape`]; the innermost AL frame is that routine): the
//! install codeunit's OnInstallAppPerCompany trigger (G1sXaInstall.al:115-117), or a "System Initialization".OnAfterLogin
//! subscriber that the session's Company Triggers raise (:88-96); and the install trigger, or the Company Triggers that
//! raised the subscriber, ran to its end (the probe's routines all do). It is not there in the next app's install session
//! (G1sInstallSessionTests.al:58, B after A; :68, the main app after B) nor in the test session after the install sessions
//! (:80). Only an entry set in that shape ends; every other entry stays as main keeps it.
//!
//! Where AL code then does what main refused (a record of the table without the ended filter), it relies on the assumption
//! and marks it. Carried to the tests (the install baseline keeps them, `Baseline::group1_flagged`), so that a test that
//! touches one relies on the assumption too: the tables whose filter ended, and, once install code relied, the tables
//! install code wrote after the first end and the tables the runner would have refused for each relying routine
//! (`Session::flag_group1_tables`).

use super::*;
use std::cell::Cell;
use std::collections::{BTreeSet, HashMap};

/// The place in an app's install session that the session runs now, of the two the probe measured; set by the runner
/// around the per-company part in the session company.
#[derive(Clone, Default)]
pub enum Group1InstallShape {
    /// Neither: a group 1 filter set now stays for the session, as main keeps it.
    #[default]
    Off,
    /// The install codeunit's OnInstallAppPerCompany trigger, as (object, routine): only a filter its body sets ends
    /// (G1sXaInstall.al:115-117).
    InstallTrigger((usize, usize)),
    /// The session's Company Triggers: only a filter the body of one of these routines, the "System Initialization".
    /// OnAfterLogin subscribers, sets ends (G1sXaInstall.al:88-96).
    Login(Rc<[(usize, usize)]>),
}

/// Whether a group 1 filter ends with its install session.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Ends {
    /// Set outside the measured shape: it stays for the session, as main keeps it.
    Never,
    /// Set in a [`Group1InstallShape`] other than `Off`, by a routine that has not returned yet: it ends only when the
    /// routine runs to its end ([`Session::end_group1_install_routine`]).
    AwaitsRoutine,
    /// Set in the measured shape by a routine that ran to its end: it ends with its install session.
    WithInstallSession,
}

/// One table's group 1 filters.
struct Entry {
    /// The filters, as text to compare (records.rs `group1_signature`).
    signature: String,
    ends: Ends,
}

/// The group 1 filters of one session; StartSession parks them (bgsession.rs).
#[derive(Default)]
pub(crate) struct Filters {
    by_table: HashMap<u32, Entry>,
    /// Table -> the group 1 filters an install session ended, which main would still hold.
    ended: HashMap<u32, String>,
}

/// The session's group 1 filters (`Session::global_group1`) and what their end left.
#[derive(Default)]
pub(crate) struct Group1Filters {
    pub(crate) filters: Filters,
    shape: Group1InstallShape,
    /// The store's change counter from which install code wrote since the first end; `None` before it.
    written_from: Option<u64>,
    /// The tables install code wrote since the first end, collected at each session boundary and rewind.
    written: BTreeSet<u32>,
    /// AL code did what main refused on a table whose filter ended.
    relied: Cell<bool>,
    /// The same, since the last [`Session::end_group1_install_routine`].
    relied_in_routine: Cell<bool>,
    /// The tables the runner would have refused for the install routines that relied on the assumption
    /// ([`Session::flag_group1_tables`]).
    relied_tables: BTreeSet<u32>,
    /// From the test session on: the tables a test relies on the assumption through (module documentation).
    pub(crate) flagged: BTreeSet<u32>,
}

impl Group1Filters {
    pub(crate) fn is_empty(&self) -> bool {
        self.filters.by_table.is_empty() && self.filters.ended.is_empty()
    }

    /// The group 1 filters of table `table`, as text to compare.
    pub(crate) fn signature(&self, table: u32) -> Option<&str> {
        self.filters.by_table.get(&table).map(|e| e.signature.as_str())
    }

    /// The group 1 filters of every table that holds some, as text to compare (al-vm `state_stamp.rs`).
    pub(crate) fn signatures(&self) -> std::collections::BTreeMap<u32, String> {
        (self.filters.by_table.iter())
            .map(|(&t, e)| (t, e.signature.clone()))
            .collect()
    }
}

impl Session {
    /// Notes what group 1 of table `table` holds now; `by_record`: a Record variable of AL code set it.
    pub(crate) fn note_group1(&mut self, table: u32, signature: String, by_record: bool) {
        if signature.is_empty() {
            self.global_group1.filters.by_table.remove(&table);
            return;
        }
        // The innermost AL frame: the routine whose body sets the filter.
        let innermost = (self.frames.last())
            .zip(self.call_stack.last())
            .map(|(&o, f)| (o, f.routine));
        let in_shape = by_record
            && innermost.is_some_and(|r| match &self.global_group1.shape {
                Group1InstallShape::Off => false,
                Group1InstallShape::InstallTrigger(trigger) => r == *trigger,
                Group1InstallShape::Login(subscribers) => subscribers.contains(&r),
            });
        let entry = Entry {
            signature,
            ends: if in_shape { Ends::AwaitsRoutine } else { Ends::Never },
        };
        self.global_group1.filters.by_table.insert(table, entry);
    }

    /// `what` on `rec` of table `table`, which holds no group 1 filter now: where an install session ended one and main
    /// would refuse (records.rs `global_group_check`: not `FilterGroup`, and a temporary record or other filters), the call
    /// relies on the assumption.
    pub(crate) fn group1_ended_check(&self, rec: &crate::records::Rec, table: u32, what: &str) {
        let Some(ended) = self.global_group1.filters.ended.get(&table) else {
            return;
        };
        if what != "FilterGroup" && (rec.borrow().temporary || Self::group1_signature(rec) != *ended) {
            self.global_group1.relied.set(true);
            self.global_group1.relied_in_routine.set(true);
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::GlobalFilterEndsWithInstallSession);
        }
    }

    /// A test touches table `object`: install code wrote it after it relied on the assumption.
    pub(crate) fn group1_flagged_touch(&self, object: u32) {
        if self.global_group1.flagged.contains(&object) {
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::GlobalFilterEndsWithInstallSession);
        }
    }

    /// The place in the install session the session runs now (module documentation). The runner ends each routine it
    /// runs with [`Self::end_group1_install_routine`]; an entry still waiting for its routine keeps main's rule.
    pub fn set_group1_install_shape(&mut self, shape: Group1InstallShape) {
        self.settle_group1_entries(false);
        self.global_group1.shape = shape;
    }

    /// A routine the runner ran returned (`ran_to_end`: it returned Ok), whether it ran in a [`Group1InstallShape`] or
    /// not: the filters it set in the measured shape end with their install session only if it ran to its end (the probe
    /// measured routines that do, G1sXaInstall.al:88-96, :115-118); otherwise they stay as main keeps them. The shape is
    /// `Off` again. True when it ran to its end and relied on the assumption: the runner then names the tables it would
    /// have refused for it ([`Self::flag_group1_tables`]).
    pub fn end_group1_install_routine(&mut self, ran_to_end: bool) -> bool {
        self.settle_group1_entries(ran_to_end);
        let g = &mut self.global_group1;
        g.shape = Group1InstallShape::Off;
        g.relied_in_routine.replace(false) && ran_to_end
    }

    /// Settles the entries waiting for their routine: they end with their install session when it ran to its end.
    fn settle_group1_entries(&mut self, ran_to_end: bool) {
        let settled = if ran_to_end {
            Ends::WithInstallSession
        } else {
            Ends::Never
        };
        for e in self.global_group1.filters.by_table.values_mut() {
            if e.ends == Ends::AwaitsRoutine {
                e.ends = settled;
            }
        }
    }

    /// The tables the runner would have refused for an install routine that relied on the assumption: a test that touches
    /// one relies on it too (from the test session on, `end_group1_install_sessions`).
    pub fn flag_group1_tables(&mut self, tables: impl IntoIterator<Item = u32>) {
        self.global_group1.relied_tables.extend(tables);
    }

    /// An app's install session begins: the group 1 filters set in the shape of an earlier install session end
    /// (G1sInstallSessionTests.al:58, :68; module documentation).
    pub fn end_group1_install_session(&mut self) {
        self.collect_group1_writes();
        let counter = self.store.change_counter();
        let g = &mut self.global_group1;
        g.shape = Group1InstallShape::Off;
        let ended: Vec<u32> = (g.filters.by_table.iter())
            .filter(|(_, e)| e.ends == Ends::WithInstallSession)
            .map(|(t, _)| *t)
            .collect();
        for t in ended {
            if let Some(e) = g.filters.by_table.remove(&t) {
                g.filters.ended.insert(t, e.signature);
            }
            g.written_from.get_or_insert(counter);
        }
    }

    /// The install sessions end and the test session follows (G1sInstallSessionTests.al:80): the filters of the last end.
    /// A test relies on the assumption through the tables whose filter ended (cell d, :80) and, when install code relied
    /// on it, through the tables install code wrote since the first end and those of the relying routines.
    pub fn end_group1_install_sessions(&mut self) {
        self.end_group1_install_session();
        self.collect_group1_writes();
        let g = &mut self.global_group1;
        g.written_from = None;
        let written = std::mem::take(&mut g.written);
        let relied_tables = std::mem::take(&mut g.relied_tables);
        g.flagged.extend(g.filters.ended.keys().copied());
        if g.relied.get() {
            g.flagged.extend(written);
            g.flagged.extend(relied_tables);
        }
    }

    /// Adds the tables written since `written_from` to `written` and counts on from now: before the store is rewound
    /// (`rewind_install`) and at each session boundary.
    pub(crate) fn collect_group1_writes(&mut self) {
        if let Some(from) = self.global_group1.written_from {
            let w = self.store.tables_changed_since(from);
            self.global_group1.written.extend(w);
            self.restart_group1_writes();
        }
    }

    /// Counts the writes from the store's change counter now (after a rewind put an earlier counter back).
    pub(crate) fn restart_group1_writes(&mut self) {
        if self.global_group1.written_from.is_some() {
            self.global_group1.written_from = Some(self.store.change_counter());
        }
    }
}
