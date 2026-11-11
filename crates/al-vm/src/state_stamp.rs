//! What a run of code left in the session besides the database rows: the session language, WorkDate, the number
//! sequences and isolated storage (al-testrun `login.rs` compares it before and after the apps' own login subscribers
//! ran in a session where what BC's login does is not known; the store's watch follows the rows).

use crate::Session;
use crate::sessionops::Sequence;
use std::collections::BTreeMap;

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
