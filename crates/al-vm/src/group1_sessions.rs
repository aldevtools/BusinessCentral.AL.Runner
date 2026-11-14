//! The filters in filter group 1 (the global group) a session holds, per table, and when they end.
//!
//! In-session: BC keeps a group 1 filter on every record of the table for the rest of the session (records.rs
//! `set_global_group_filter`, `global_group_check`).
//!
//! Across sessions (assumption `global-filter-ends-with-install-session`, corpus PR 1405, open: tests/al-language/
//! install-upgrade/G1sInstallSessionTests.al with tests/al-language-xapp-a/G1sXaInstall.al and
//! tests/al-language-xapp-b/G1sXbInstall.al, measured alike on all three supported BC majors): a filter that a Record variable set
//! with SetFilter or SetRange in filter group 1, in the per-company part of an app's install session in the session
//! company, in one of two places ([`Group1InstallShape`]): the install codeunit's OnInstallAppPerCompany trigger
//! (G1sXaInstall.al:115-117), or a "System Initialization".OnAfterLogin subscriber that the session's Company Triggers
//! raise (:88-96). It is not there in the next app's install session (G1sInstallSessionTests.al:58, B after A; :68, the
//! main app after B) nor in the test session after the install sessions (:80). Only an entry set in that shape ends; every
//! other entry stays as main keeps it.
//!
//! Where AL code then does what main refused (a record of the table without the ended filter), it relies on the assumption
//! and marks it. The tables install code wrote after the first end are then carried to the tests (the install baseline
//! keeps them, `Baseline::group1_flagged`): a test that touches one relies on the assumption too.

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
    /// The install codeunit's OnInstallAppPerCompany trigger (G1sXaInstall.al:115-117).
    InstallTrigger,
    /// The session's Company Triggers: only a filter set inside one of these routines, the "System Initialization".
    /// OnAfterLogin subscribers (G1sXaInstall.al:88-96), ends.
    Login(Rc<[(usize, usize)]>),
}

/// One table's group 1 filters.
struct Entry {
    /// The filters, as text to compare (records.rs `group1_signature`).
    signature: String,
    /// Set in a [`Group1InstallShape`] other than `Off`: it ends with its install session.
    install_session: bool,
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
    /// From the test session on: the tables install code wrote after the first end, when it relied on the assumption.
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
}

impl Session {
    /// Notes what group 1 of table `table` holds now; `by_record`: a Record variable of AL code set it.
    pub(crate) fn note_group1(&mut self, table: u32, signature: String, by_record: bool) {
        if signature.is_empty() {
            self.global_group1.filters.by_table.remove(&table);
            return;
        }
        let install_session = by_record
            && match &self.global_group1.shape {
                Group1InstallShape::Off => false,
                Group1InstallShape::InstallTrigger => true,
                Group1InstallShape::Login(subscribers) => {
                    (self.frames.iter().zip(&self.call_stack)).any(|(&o, f)| subscribers.contains(&(o, f.routine)))
                }
            };
        let entry = Entry {
            signature,
            install_session,
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
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::GlobalFilterEndsWithInstallSession);
        }
    }

    /// A test touches table `object`: install code wrote it after it relied on the assumption.
    pub(crate) fn group1_flagged_touch(&self, object: u32) {
        if self.global_group1.flagged.contains(&object) {
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::GlobalFilterEndsWithInstallSession);
        }
    }

    /// The place in the install session the session runs now (module documentation).
    pub fn set_group1_install_shape(&mut self, shape: Group1InstallShape) {
        self.global_group1.shape = shape;
    }

    /// An app's install session begins: the group 1 filters set in the shape of an earlier install session end
    /// (G1sInstallSessionTests.al:58, :68; module documentation).
    pub fn end_group1_install_session(&mut self) {
        self.collect_group1_writes();
        let counter = self.store.change_counter();
        let g = &mut self.global_group1;
        g.shape = Group1InstallShape::Off;
        let ended: Vec<u32> = (g.filters.by_table.iter())
            .filter(|(_, e)| e.install_session)
            .map(|(t, _)| *t)
            .collect();
        for t in ended {
            if let Some(e) = g.filters.by_table.remove(&t) {
                g.filters.ended.insert(t, e.signature);
            }
            g.written_from.get_or_insert(counter);
        }
    }

    /// The install sessions end and the test session follows (G1sInstallSessionTests.al:80): the filters of the last end,
    /// and the tables install code wrote since the first end become the ones a test relies on the assumption through,
    /// when install code relied on it.
    pub fn end_group1_install_sessions(&mut self) {
        self.end_group1_install_session();
        self.collect_group1_writes();
        let g = &mut self.global_group1;
        g.written_from = None;
        let written = std::mem::take(&mut g.written);
        if g.relied.get() {
            g.flagged.extend(written);
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
