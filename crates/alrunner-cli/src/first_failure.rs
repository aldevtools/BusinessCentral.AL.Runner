//! `stopAtFirstFailure` of `runTests` (upstream issue 8210): the field's values, the combinations that are an `error`,
//! and how many stops a request has. What a stop does while the tests run is `al_testrun::failure_stop`'s.

use crate::args::TestArgs;
use crate::run;
pub use al_testrun::failure_stop::{Counts, FailureStop, StopScope};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

/// `stopAtFirstFailure` of a request. Absent or null: `None`.
pub fn scope_of(fields: &Map<String, Value>) -> Result<Option<StopScope>, String> {
    match fields.get("stopAtFirstFailure") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) if s == "request" => Ok(Some(StopScope::Request)),
        Some(Value::String(s)) if s == "group" => Ok(Some(StopScope::Group)),
        Some(_) => Err("`stopAtFirstFailure` is \"request\" or \"group\"".into()),
    }
}

/// `stopGroup` of an entry, as text: a number and a text that read alike are one group, as the entry ids are
/// (`serve::entries_of`). Absent or null: `None`.
pub fn group_of(entry: &Map<String, Value>) -> Result<Option<String>, String> {
    match entry.get("stopGroup") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(n @ Value::Number(_)) => Ok(Some(n.to_string())),
        Some(_) => Err("the `stopGroup` of an entry is a text or a number".into()),
    }
}

/// The end of the error of a field the stop cannot be combined with, where a failed codeunit may run again.
const RUNS_AGAIN: &str = "a failed test codeunit may run again there, so the first failure does not decide the result";

/// The combinations of a request that are an `error`.
pub fn check(scope: Option<StopScope>, args: &TestArgs, entries: &[run::Entry]) -> Result<(), String> {
    let Some(scope) = scope else {
        return match entries.iter().any(|e| e.stop_group.is_some()) {
            true => Err("`stopGroup` needs `stopAtFirstFailure: \"group\"`".into()),
            false => Ok(()),
        };
    };
    let refused = |with: &str, why: &str| Err(format!("`stopAtFirstFailure` cannot be combined with {with}: {why}"));
    if scope == StopScope::Request && entries.iter().any(|e| e.stop_group.is_some()) {
        return Err("`stopGroup` needs `stopAtFirstFailure: \"group\"`".into());
    }
    if scope == StopScope::Group && entries.is_empty() {
        return Err(
            "`stopAtFirstFailure: \"group\"` needs `entries`: a group is the entries with the same `stopGroup`".into(),
        );
    }
    // Under strict order a failed codeunit may run again serially (al-testrun `runtime/single_lane.rs`), and its result
    // may change.
    if args.strict_order {
        return refused("`strictOrder`", RUNS_AGAIN);
    }
    if args.test_driver == Some(al_testrun::runtime::TestDriver::BcContainerHelper) {
        return refused("`testDriver: \"bccontainerhelper\"`", RUNS_AGAIN);
    }
    if args.isolation == Some(al_testrun::isolation::TestIsolation::Disabled) {
        return refused(
            "`testIsolation: \"disabled\"`",
            "the result of a test codeunit depends on the ones before it, so the first failure does not decide the result",
        );
    }
    if args.affected {
        return refused("`affectedOnly`", "a stopped run is not a result to store");
    }
    Ok(())
}

/// The stops of one request: one for `Request`, one per group key for `Group`.
pub struct Stops {
    scope: Option<StopScope>,
    request: Option<FailureStop>,
    groups: BTreeMap<String, FailureStop>,
}

impl Stops {
    pub fn new(scope: Option<StopScope>) -> Stops {
        Stops {
            scope,
            request: (scope == Some(StopScope::Request)).then(|| FailureStop::new(StopScope::Request)),
            groups: BTreeMap::new(),
        }
    }

    /// The stop of the request's own run (no entries); a `Group` scope has none (`check` refuses it).
    pub fn of_request(&self, counts: Counts) -> Option<FailureStop> {
        self.request.as_ref().map(|s| s.counting(counts))
    }

    /// The stop of an entry; `group`: its `stopGroup` (`None`: a group of its own).
    pub fn of_entry(&mut self, group: Option<&str>, counts: Counts) -> Option<FailureStop> {
        match self.scope? {
            StopScope::Request => self.of_request(counts),
            StopScope::Group => {
                let stop = match group {
                    Some(g) => self
                        .groups
                        .entry(g.to_string())
                        .or_insert_with(|| FailureStop::new(StopScope::Group))
                        .clone(),
                    None => FailureStop::new(StopScope::Group),
                };
                Some(stop.counting(counts))
            }
        }
    }
}
