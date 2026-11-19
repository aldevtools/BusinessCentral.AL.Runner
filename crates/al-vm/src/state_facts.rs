//! The session facts the login replay check compares besides the SingleInstance globals, the bindings and the counters
//! of [`Session::state_lines`] (al-testrun `login_replay.rs`, al-runner issue 8211): state the login leaves that a later
//! test or the runner reads. Each fact is a line that compares across sessions, or a reason why it has none.

use crate::Session;
use crate::state_render::content;
use std::collections::BTreeMap;

impl Session {
    /// The lines of the facts of this module, in a fixed order; a fact with no comparable form goes to `opaque` with why.
    pub(crate) fn fact_lines(&self, lines: &mut Vec<String>, opaque: &mut Vec<String>) {
        let _ = opaque;
        lines.push(self.global_setups_line());
    }

    /// The Global Triggers setup BC asked for each table (`events.global_setups`): a later first write to a table asks
    /// only when the table is not in it. By table object index, with the table's id.
    fn global_setups_line(&self) -> String {
        let setups: BTreeMap<usize, (u32, crate::globaltriggers::GlobalSetup)> = (self.events.global_setups.iter())
            .map(|(&t, &s)| (t, (self.engine.program.objects[t].id.unwrap_or(0), s)))
            .collect();
        format!("global setups {}", content(&setups))
    }
}
