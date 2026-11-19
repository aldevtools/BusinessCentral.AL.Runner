//! The session facts the login replay check compares besides the SingleInstance globals, the bindings and the counters
//! of [`Session::state_lines`] (al-testrun `login_replay.rs`, al-runner issue 8211): state the login leaves that a later
//! test or the runner reads. Each fact is a line that compares across sessions, or a reason why it has none.

use crate::Session;
use crate::state_render::content;
use std::collections::BTreeMap;

/// Why the SQL counters have no comparable form: a counter was read, or the record methods since the last Commit left
/// state that only the session's own log explains (`SqlCounters::left_inactive`).
const SQL_ACTIVE: &str = "SQL counters (read, or not at rest since the last Commit)";

impl Session {
    /// The lines of the facts of this module, in a fixed order; a fact with no comparable form goes to `opaque` with why.
    pub(crate) fn fact_lines(&self, lines: &mut Vec<String>, opaque: &mut Vec<String>) {
        lines.push(self.global_setups_line());
        lines.push(self.event_calls_line());
        lines.push(self.media_line());
        match self.sql.left_inactive() {
            Some(left) => lines.push(format!("sql {}", left.line())),
            None => opaque.push(SQL_ACTIVE.to_string()),
        }
    }

    /// The Global Triggers setup BC asked for each table (`events.global_setups`): a later first write to a table asks
    /// only when the table is not in it. By table object index, with the table's id.
    fn global_setups_line(&self) -> String {
        let setups: BTreeMap<usize, (u32, crate::globaltriggers::GlobalSetup)> = (self.events.global_setups.iter())
            .map(|(&t, &s)| (t, (self.engine.program.objects[t].id.unwrap_or(0), s)))
            .collect();
        format!("global setups {}", content(&setups))
    }

    /// How often each subscriber (object index, routine index) handled an event, and how many bound instances each
    /// subscriber object has (`events.event_stats`: the Event Subscription table's "Number of Calls" and "Active").
    fn event_calls_line(&self) -> String {
        let stats = &self.events.event_stats;
        let calls: BTreeMap<(usize, usize), i64> = stats.calls.borrow().iter().map(|(&k, &n)| (k, n)).collect();
        let active: BTreeMap<usize, i64> = stats.active.borrow().iter().map(|(&k, &n)| (k, n)).collect();
        format!("event calls {} active {}", content(&calls), content(&active))
    }

    /// The session's media (`media`: a row holds only a Media or MediaSet id, the content lives here): each object with its
    /// bytes, each set, the ids removed while a holder still names them, the ids kept out of Tenant Media; and the objects a
    /// rollback brings back (`media_removed`). Sorted by id.
    fn media_line(&self) -> String {
        let m = &self.media;
        let objects: BTreeMap<_, _> = m.objects.borrow().iter().map(|(k, v)| (*k, v.clone())).collect();
        let sets: BTreeMap<_, _> = m.sets.borrow().iter().map(|(k, v)| (*k, v.clone())).collect();
        let removed: std::collections::BTreeSet<_> = m.removed.borrow().iter().copied().collect();
        let outside: std::collections::BTreeSet<_> = m.outside_tenant.borrow().iter().copied().collect();
        format!(
            "media objects {} sets {} removed {} outside tenant {} removed by writes {}",
            content(&objects),
            content(&sets),
            content(&removed),
            content(&outside),
            content(&self.media_removed)
        )
    }
}
