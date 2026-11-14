//! The filters in filter group 1 (the global group) a session holds, per table, and when they end.
//!
//! In-session: BC keeps a group 1 filter on every record of the table for the rest of the session (records.rs
//! `set_global_group_filter`, `global_group_check`).
//!
//! Across sessions (assumption `group1-filter-ends-with-install-session`, corpus PR 1405, open: tests/al-language/
//! install-upgrade/G1sInstallSessionTests.al with tests/al-language-xapp-a/G1sXaInstall.al and
//! tests/al-language-xapp-b/G1sXbInstall.al, measured alike on BC 27.5, 28.5 and 29.0): a filter that a Record variable set
//! with SetFilter or SetRange in filter group 1, in the per-company part of an app's install session in the session company
//! (the install routine, G1sXaInstall.al:115-117, or a "System Initialization".OnAfterLogin subscriber that the session's
//! Company Triggers raise, :88-96), is not there in the next app's install session (G1sInstallSessionTests.al:58, B after
//! A; :68, the main app after B) nor in the test session after the install sessions (:80). Only an entry set in that shape
//! is marked; every other entry (set by a FieldRef, in the per-database part, in another company, by Company-Initialize or
//! a company build step, in the test session) stays as main keeps it.

use super::*;

/// One table's group 1 filters.
struct Entry {
    /// The filters, as text to compare (records.rs `group1_signature`).
    signature: String,
    /// Set by a Record variable while [`Group1Filters::install_shape`] was on: it ends with its install session.
    install_session: bool,
}

/// The session's group 1 filters (`Session::global_group1`).
#[derive(Default)]
pub(crate) struct Group1Filters {
    by_table: std::collections::HashMap<u32, Entry>,
    /// The per-company part of an app's install session runs in the session company
    /// ([`Session::set_group1_install_shape`]).
    install_shape: bool,
}

impl Group1Filters {
    pub(crate) fn is_empty(&self) -> bool {
        self.by_table.is_empty()
    }

    /// The group 1 filters of table `table`, as text to compare.
    pub(crate) fn signature(&self, table: u32) -> Option<&str> {
        self.by_table.get(&table).map(|e| e.signature.as_str())
    }

    /// Notes what group 1 of table `table` holds now; `by_record`: a Record variable of AL code set it.
    pub(crate) fn set(&mut self, table: u32, signature: String, by_record: bool) {
        if signature.is_empty() {
            self.by_table.remove(&table);
            return;
        }
        let install_session = by_record && self.install_shape;
        self.by_table.insert(
            table,
            Entry {
                signature,
                install_session,
            },
        );
    }

    /// Drops the entries set in an install session; whether there was one.
    fn end_install_session(&mut self) -> bool {
        let before = self.by_table.len();
        self.by_table.retain(|_, e| !e.install_session);
        self.install_shape = false;
        self.by_table.len() != before
    }
}

impl Session {
    /// The per-company part of an app's install session runs in the session company (`on`), or no longer: a group 1 filter
    /// a Record variable sets while it is on ends with the install session (module documentation).
    pub fn set_group1_install_shape(&mut self, on: bool) {
        self.global_group1.install_shape = on;
    }

    /// An app's install session begins, or the install sessions end and the test session follows: the group 1 filters set
    /// in an earlier install session are gone (G1sInstallSessionTests.al:58, :68, :80; module documentation).
    pub fn end_group1_install_session(&mut self) {
        if self.global_group1.end_install_session() {
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::Group1FilterEndsWithInstallSession);
        }
    }
}
