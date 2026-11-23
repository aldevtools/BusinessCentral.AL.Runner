//! The session facts the login replay check compares besides the SingleInstance globals, the bindings and the counters
//! of [`Session::state_lines`] (al-testrun `login_replay.rs`, al-runner issue 8211): state the login leaves that a later
//! test or the runner reads. Each fact is a line that compares across sessions, or a reason why it has none.
//!
//! the facts `Session::state_lines` compares besides the globals, bindings and counters, one line
//! each (al-testrun `login_replay.rs`, al-runner issue 8211): the Global Triggers setups asked, the
//! event subscriber call counts, the media store, the commits and the last commit's facts, the
//! committed store's marks (emptied tables, company copies), the SQL counters when inactive and at
//! rest (else a reason, no form).

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
        lines.push(self.transaction_line());
        lines.push(self.committed_marks_line());
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

    /// The marks of the committed store the committed rows leave out (`MemStore::committed_marks`), sorted: the tables a
    /// DeleteAll without filters emptied (a read of a table whose rows BC holds but the runner does not know answers only
    /// while its table is emptied, records.rs `is_unseeded`) and the company copies (a read in a copy answers with its
    /// source's backing rows, `MemStore::note_copy`). With either, two sessions with the same rows can answer otherwise.
    fn committed_marks_line(&self) -> String {
        let (emptied, copies) = self.store.committed_marks();
        format!("emptied {} company copies {}", content(&emptied), content(&copies))
    }

    /// How often the session committed (a record's own read is current against it, `state_render.rs`) and what the last
    /// Commit left that a later test reads: whether it made writes durable (`MinimumActiveRowVersion`), the transaction
    /// type and phase (`CurrentTransactionType`), the lock timeout, a rollback since, and whether the store changed since
    /// (`Session::stale_verdict`).
    fn transaction_line(&self) -> String {
        let tx = &self.tx;
        format!(
            "transaction commits {} last commit wrote {} type {} phase {} lock timeout {} {} rolled back {} changed since {}",
            self.commits,
            tx.last_commit_wrote,
            tx.transaction_type,
            content(&tx.phase),
            tx.lock_timeout,
            tx.lock_timeout_duration,
            tx.rolled_back,
            tx.commit_counter != self.store.change_counter()
        )
    }
}

#[cfg(test)]
mod tests {
    // A DeleteAll without filters marks its table emptied, and the mark outlives the Commit: a later read of a table whose
    // rows BC holds but the runner does not know answers only when the mark stands (records.rs `is_unseeded`). Two
    // sessions that differ only in that mark do not hold the same, though their rows are equal.
    #[test]
    fn a_table_a_delete_all_emptied_is_a_state_line() {
        use al_data::Store;
        let (_, l) = al_sema::lower::compile(Vec::new(), &Default::default());
        let engine = crate::Engine::new(l.program, Vec::new());
        let mut emptied = crate::Session::new(engine.clone());
        let mut fresh = crate::Session::new(engine);
        let cx = al_data::contract::cx("CRONUS");
        emptied.store.delete_all(&cx, &al_data::contract::table(), &[]).unwrap();
        emptied.commit_transaction();
        fresh.commit_transaction();
        let (a, b) = (emptied.state_lines(), fresh.state_lines());
        let differ: Vec<_> = a.lines.iter().zip(&b.lines).filter(|(x, y)| x != y).collect();
        assert_eq!(differ.len(), 1, "{differ:?}");
        assert!(differ[0].0.starts_with("emptied "), "{differ:?}");
    }

    // A company copy (`MemStore::note_copy`: a company that holds the backing rows of another) is a committed mark the
    // committed rows leave out (`MemStore::committed_marks`), and it outlives the Commit: a read in the copy answers with
    // its source's backing rows. Two sessions that differ only in a copy do not hold the same, though their rows are
    // equal.
    #[test]
    fn a_company_copy_is_a_state_line() {
        let (_, l) = al_sema::lower::compile(Vec::new(), &Default::default());
        let engine = crate::Engine::new(l.program, Vec::new());
        let mut copied = crate::Session::new(engine.clone());
        let mut fresh = crate::Session::new(engine);
        copied.store.note_copy("COPY", "CRONUS");
        copied.commit_transaction();
        fresh.commit_transaction();
        let (a, b) = (copied.state_lines(), fresh.state_lines());
        let differ: Vec<_> = a.lines.iter().zip(&b.lines).filter(|(x, y)| x != y).collect();
        assert_eq!(differ.len(), 1, "{differ:?}");
        assert!(differ[0].0.contains("company copies "), "{differ:?}");
    }
}
