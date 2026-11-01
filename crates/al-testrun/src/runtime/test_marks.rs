//! What a test leaves on its result beyond its outcome, taken from the session when the test ends and moved onto its
//! `TestResult`: the `--tdd` stubs it reached and the UI calls `--dev-endpoint-ui` answered for it.
use super::{Scratch, TestResult};
use al_vm::Session;

impl Scratch {
    /// Takes what the test `name` left in `session`; returns the `--tdd` stubs it reached and the UI calls answered.
    pub(super) fn take_marks(&mut self, name: &str, session: &mut Session) -> (Vec<String>, Vec<String>) {
        let reached = session.take_tdd_reached();
        if !reached.is_empty() {
            self.stubbed.push((name.to_string(), reached.clone()));
        }
        let answered = session.take_dev_endpoint_answers();
        if !answered.is_empty() {
            self.ui_answered.push((name.to_string(), answered.clone()));
        }
        (reached, answered)
    }

    /// Moves what `take_marks` kept for the test `name` onto its result.
    pub(super) fn move_marks(&mut self, name: &str, result: Option<&mut TestResult>) {
        let Some(r) = result else {
            return;
        };
        if let Some(i) = self.stubbed.iter().position(|(n, _)| n == name) {
            r.stubs = std::mem::take(&mut self.stubbed[i].1);
        }
        if let Some(i) = self.ui_answered.iter().position(|(n, _)| n == name) {
            r.ui_answered = std::mem::take(&mut self.ui_answered[i].1);
        }
    }
}
