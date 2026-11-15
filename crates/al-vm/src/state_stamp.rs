//! What a run of code left in the session besides the database rows: the session language, WorkDate, the number
//! sequences and isolated storage (al-testrun `login.rs` compares it before and after the apps' own login subscribers
//! ran in a session where what BC's login does is not known; the store's watch follows the rows).

use crate::Session;
use crate::sessionops::Sequence;
use std::collections::{BTreeMap, BTreeSet};

/// The session state [`Session::state_stamp`] takes.
#[derive(PartialEq)]
pub struct StateStamp {
    language: i32,
    work_date: Option<i32>,
    sequences: BTreeMap<(String, String), Sequence>,
    isolated_storage: BTreeMap<(String, String), (String, bool, bool)>,
}

impl StateStamp {
    /// The first kind of state that differs between `self` and `later`, for a reason text.
    pub fn changed(&self, later: &StateStamp) -> Option<&'static str> {
        if self.language != later.language {
            Some("the session language")
        } else if self.work_date != later.work_date {
            Some("WorkDate")
        } else if self.sequences != later.sequences {
            Some("a number sequence")
        } else if self.isolated_storage != later.isolated_storage {
            Some("isolated storage")
        } else {
            None
        }
    }
}

impl Session {
    /// The session language, WorkDate, number sequences and isolated storage now.
    pub fn state_stamp(&self) -> StateStamp {
        StateStamp {
            language: self.global_language,
            work_date: self.work_date_day(),
            sequences: self.sequences.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            isolated_storage: self
                .isolated_storage
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        }
    }
}

/// What a run of code left in the session besides the database rows, wider than [`StateStamp`]: also the SingleInstance
/// codeunits and their globals, the manual event bindings, the `Random` sequence and the filters in filter group 1 (al-testrun
/// `partner_login.rs` compares it before and after the partner dependency apps' login subscribers ran in the test session).
pub struct LeftStamp {
    state: StateStamp,
    singles: BTreeMap<usize, Vec<crate::single_track::Fp>>,
    bindings: usize,
    random: (bool, i32, u64),
    group1: BTreeMap<u32, String>,
}

impl LeftStamp {
    /// The first kind of state other than rows and group 1 filters that differs between `self` and `later`, for a
    /// reason text. A global with no comparable form (a .NET object, a stream) counts as changed.
    pub fn changed(&self, later: &LeftStamp) -> Option<&'static str> {
        if let Some(what) = self.state.changed(&later.state) {
            Some(what)
        } else if self.singles.keys().ne(later.singles.keys()) || self.singles.values().ne(later.singles.values()) {
            Some("a SingleInstance codeunit or its globals")
        } else if self.bindings != later.bindings {
            Some("the manual event bindings")
        } else if self.random != later.random {
            Some("the Random sequence")
        } else {
            None
        }
    }

    /// The tables whose filters in filter group 1 differ between `self` and `later`.
    pub fn group1_changed(&self, later: &LeftStamp) -> BTreeSet<u32> {
        let tables = self.group1.keys().chain(later.group1.keys());
        tables
            .filter(|t| self.group1.get(t) != later.group1.get(t))
            .copied()
            .collect()
    }
}

impl Session {
    /// The state [`LeftStamp`] takes, now.
    pub fn left_stamp(&self) -> LeftStamp {
        let singles = (self.singletons.iter())
            .map(|(&o, i)| {
                let globals = i
                    .borrow()
                    .globals
                    .iter()
                    .map(|v| crate::single_track::fp(v, &self.engine))
                    .collect();
                (o, globals)
            })
            .collect();
        let group1 = self.global_group1.signatures();
        LeftStamp {
            state: self.state_stamp(),
            singles,
            bindings: self.events.bindings.len(),
            random: (self.randomized_with_seed, self.unseeded_randomizes, self.random_draws),
            group1,
        }
    }
}
