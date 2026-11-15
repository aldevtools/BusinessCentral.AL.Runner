//! The filters in filter group 1 (the global group) a session holds, per table, and when they end.
//!
//! In-session: BC keeps a group 1 filter on every record of the table for the rest of the session (records.rs
//! `set_global_group_filter`, `global_group_check`).
//!
//! Across sessions (pinned by tests/al-language/install-upgrade/G1sInstallSessionTests.al with
//! tests/al-language-xapp-a/G1sXaInstall.al and tests/al-language-xapp-b/G1sXbInstall.al, BC 27 to 29): a filter that a
//! Record variable set with SetFilter or SetRange in filter group 1, in the per-company part of an app's install session
//! in the session company, in the body of one of two routines ([`Group1InstallShape`]; the innermost AL frame is that
//! routine): the install codeunit's OnInstallAppPerCompany trigger (G1sXaInstall.al:131-155, the filters at :142-144
//! and :151-153), or a "System Initialization".OnAfterLogin subscriber that the session's Company Triggers raise
//! (:116-124); and the install trigger, or the Company Triggers that raised the subscriber, ran to its end (the probe's
//! routines all do). It is not there in the next app's install session (G1sInstallSessionTests.al:55-63, B after A;
//! :65-73, the main app after B) nor in the test session after the install sessions (:75-85); the same for a per-company
//! table (:107-123, table 98485). An OnAfterLogin subscriber sets it again in each session (:87-105). Only an entry set
//! in that shape ends; every other entry stays, and a record without it is refused (records.rs `global_group_check`).

use super::*;
use std::collections::HashMap;

/// The place in an app's install session that the session runs now, of the two the probe measured; set by the runner
/// around the per-company part in the session company.
#[derive(Clone, Default)]
pub enum Group1InstallShape {
    /// Neither: a group 1 filter set now stays for the session.
    #[default]
    Off,
    /// The install codeunit's OnInstallAppPerCompany trigger, as (object, routine): only a filter its body sets ends
    /// (G1sXaInstall.al:131-155).
    InstallTrigger((usize, usize)),
    /// The session's Company Triggers: only a filter the body of one of these routines, the "System Initialization".
    /// OnAfterLogin subscribers, sets ends (G1sXaInstall.al:116-124).
    Login(Rc<[(usize, usize)]>),
}

/// Whether a group 1 filter ends with its install session.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Ends {
    /// Set outside the measured shape: it stays for the session.
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
}

/// The session's group 1 filters (`Session::global_group1`) and the install shape they are set in.
#[derive(Default)]
pub(crate) struct Group1Filters {
    pub(crate) filters: Filters,
    shape: Group1InstallShape,
}

impl Group1Filters {
    pub(crate) fn is_empty(&self) -> bool {
        self.filters.by_table.is_empty()
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

    /// The place in the install session the session runs now (module documentation). The runner ends each routine it
    /// runs with [`Self::end_group1_install_routine`]; an entry still waiting for its routine stays for the session.
    pub fn set_group1_install_shape(&mut self, shape: Group1InstallShape) {
        self.settle_group1_entries(false);
        self.global_group1.shape = shape;
    }

    /// A routine the runner ran returned (`ran_to_end`: it returned Ok), whether it ran in a [`Group1InstallShape`] or
    /// not: the filters it set in the measured shape end with their install session only if it ran to its end (the probe
    /// measured routines that do, G1sXaInstall.al:116-124, :131-155); otherwise they stay for the session. The shape is
    /// `Off` again.
    pub fn end_group1_install_routine(&mut self, ran_to_end: bool) {
        self.settle_group1_entries(ran_to_end);
        self.global_group1.shape = Group1InstallShape::Off;
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

    /// An app's install session begins, or the test session follows the install sessions: the group 1 filters set in
    /// the shape of an earlier install session end (G1sInstallSessionTests.al:55-63, :65-73, :75-85; module
    /// documentation).
    pub fn end_group1_install_session(&mut self) {
        let g = &mut self.global_group1;
        g.shape = Group1InstallShape::Off;
        g.filters.by_table.retain(|_, e| e.ends != Ends::WithInstallSession);
    }
}
