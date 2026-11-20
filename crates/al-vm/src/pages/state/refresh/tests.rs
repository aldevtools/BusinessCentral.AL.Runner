//! Bounded comparison of `RefreshRequest` with the twelve flags it replaced.

/// The twelve flags `RefreshRequest` replaced, with the bodies the call sites had, for the bounded comparison. Delete it when a
/// probed behavior change makes the old flags wrong.
mod model {
    use super::super::TakenRefresh;
    use al_builtins::value::Value;

    #[derive(Debug, Clone, Default)]
    pub struct Flags {
        pub refresh: bool,
        pub refresh_by_update: bool,
        pub refresh_update_false: bool,
        pub update_left_view: Option<Vec<Vec<Value>>>,
        pub refresh_without_curr: bool,
        pub refresh_without_parts: bool,
        pub refresh_read_done: bool,
        pub refresh_from_code_inserted_line: bool,
        pub view_left: bool,
        pub save_after_first_read: bool,
        pub refresh_pinned: bool,
        pub read_keeps_xrec: bool,
    }

    impl Flags {
        // pages/pageops.rs ask_update_refresh, then part_update_left_view.rs note_part_update_left_view.
        pub fn ask_by_update(&mut self, save: bool) {
            self.refresh = true;
            self.refresh_by_update = true;
            self.refresh_update_false = !save;
        }
        pub fn set_update_left_view(&mut self, held: Option<Vec<Vec<Value>>>) {
            self.update_left_view = held;
        }
        // pages/lookup_refresh.rs run_lookup_refresh_left.
        pub fn ask_lookup_left(&mut self, with_parts: bool) {
            self.refresh = true;
            self.refresh_by_update = true;
            self.update_left_view = None;
            self.refresh_without_parts = !with_parts;
        }
        // pages/actions.rs (after an action), pages/dirty_step.rs.
        pub fn ask_after_action(&mut self) {
            self.refresh = true;
            self.refresh_by_update = false;
        }
        pub fn ask_after_action_pinned(&mut self, pinned: bool) {
            self.refresh = true;
            self.refresh_by_update = false;
            self.refresh_pinned = pinned;
        }
        pub fn pinned_run_done(&mut self) {
            self.refresh_pinned = false;
        }
        // pages/parts.rs, three sites.
        pub fn ask_host_reread(&mut self, without_parts: bool) {
            self.refresh = true;
            self.refresh_by_update = false;
            self.refresh_without_parts = without_parts;
            self.read_keeps_xrec = true;
        }
        pub fn host_reread_done(&mut self) {
            self.read_keeps_xrec = false;
        }
        // pages/actions.rs (a tree action), pages/part_host_update.rs.
        pub fn ask(&mut self) {
            self.refresh = true;
        }
        pub fn read_without_curr(&mut self) {
            self.refresh_without_curr = true;
        }
        pub fn note_reads_done(&mut self) {
            self.refresh_read_done = true;
        }
        pub fn land_on_code_inserted_line(&mut self) {
            self.refresh_from_code_inserted_line = true;
        }
        pub fn forget_code_inserted_line(&mut self) {
            self.refresh_from_code_inserted_line = false;
        }
        pub fn note_view_left(&mut self) {
            self.view_left = true;
        }
        pub fn take_view_left(&mut self) -> bool {
            std::mem::take(&mut self.view_left)
        }
        pub fn note_save_after_first_read(&mut self) {
            self.save_after_first_read = true;
        }
        pub fn take_save_after_first_read(&mut self) -> bool {
            std::mem::take(&mut self.save_after_first_read)
        }
        pub fn action_starts(&mut self) {
            self.refresh_update_false = false;
        }
        pub fn cancel(&mut self) {
            self.refresh = false;
        }
        pub fn cancel_update(&mut self) {
            self.refresh = false;
            self.refresh_by_update = false;
            self.refresh_update_false = false;
        }
        // pages/actions.rs run_refresh.
        pub fn take(&mut self) -> Option<TakenRefresh> {
            let without_curr = std::mem::take(&mut self.refresh_without_curr);
            let without_parts = std::mem::take(&mut self.refresh_without_parts);
            let read_done = std::mem::take(&mut self.refresh_read_done);
            let by_update = std::mem::take(&mut self.refresh_by_update);
            let code_inserted_line = std::mem::take(&mut self.refresh_from_code_inserted_line);
            if !std::mem::take(&mut self.refresh) || read_done {
                return None;
            }
            Some(TakenRefresh {
                by_update,
                without_curr,
                without_parts,
                from_code_inserted_line: code_inserted_line,
            })
        }
    }
}

use super::super::RefreshRequest;
use al_builtins::value::Value;
use model::Flags;

/// Every transition, with each argument both ways.
const STEPS: usize = 28;

fn held() -> Option<Vec<Vec<Value>>> {
    Some(vec![vec![Value::Integer(1)]])
}

fn step(i: usize, new: &mut RefreshRequest, old: &mut Flags, n: usize) {
    let flag = i % 2 == 1;
    match i {
        0 | 1 => {
            new.ask_by_update(flag);
            old.ask_by_update(flag);
        }
        2 | 3 => {
            let h = if flag { held() } else { None };
            new.set_update_left_view(h.clone());
            old.set_update_left_view(h);
        }
        4 | 5 => {
            new.ask_lookup_left(flag);
            old.ask_lookup_left(flag);
        }
        6 => {
            new.ask_after_action();
            old.ask_after_action();
        }
        7 => {
            new.pinned_run_done();
            old.pinned_run_done();
        }
        8 | 9 => {
            new.ask_after_action_pinned(flag);
            old.ask_after_action_pinned(flag);
        }
        10 | 11 => {
            new.ask_host_reread(flag);
            old.ask_host_reread(flag);
        }
        12 => {
            new.host_reread_done();
            old.host_reread_done();
        }
        13 => {
            new.ask();
            old.ask();
        }
        14 => {
            new.read_without_curr();
            old.read_without_curr();
        }
        15 => {
            new.note_reads_done();
            old.note_reads_done();
        }
        16 => {
            new.land_on_code_inserted_line();
            old.land_on_code_inserted_line();
        }
        17 => {
            new.forget_code_inserted_line();
            old.forget_code_inserted_line();
        }
        18 => {
            new.note_view_left();
            old.note_view_left();
        }
        19 => assert_eq!(new.take_view_left(), old.take_view_left(), "sequence {n}"),
        20 => {
            new.note_save_after_first_read();
            old.note_save_after_first_read();
        }
        21 => assert_eq!(
            new.take_save_after_first_read(),
            old.take_save_after_first_read(),
            "sequence {n}"
        ),
        22 => {
            new.action_starts();
            old.action_starts();
        }
        23 => {
            new.cancel();
            old.cancel();
        }
        24 => {
            new.cancel_update();
            old.cancel_update();
        }
        25..=27 => assert_eq!(new.take(), old.take(), "sequence {n}"),
        _ => unreachable!("STEPS is 28"),
    }
}

fn answers_as(new: &RefreshRequest, old: &Flags, n: usize) {
    assert_eq!(new.asked(), old.refresh, "sequence {n}");
    assert_eq!(
        new.asked_by_update(),
        old.refresh && old.refresh_by_update,
        "sequence {n}"
    );
    assert_eq!(
        new.asked_by_update_false(),
        old.refresh && old.refresh_by_update && old.refresh_update_false,
        "sequence {n}"
    );
    assert_eq!(new.update_false(), old.refresh_update_false, "sequence {n}");
    assert_eq!(
        format!("{:?}", new.update_left_view()),
        format!("{:?}", old.update_left_view.as_ref()),
        "sequence {n}"
    );
    assert_eq!(
        new.starts_from_code_inserted_line(),
        old.refresh_from_code_inserted_line,
        "sequence {n}"
    );
    assert_eq!(new.view_left(), old.view_left, "sequence {n}");
    assert_eq!(new.pinned(), old.refresh_pinned, "sequence {n}");
    assert_eq!(new.keeps_xrec(), old.read_keeps_xrec, "sequence {n}");
    assert_eq!(new.clone().take(), old.clone().take(), "sequence {n}");
    assert_eq!(
        new.clone().take_save_after_first_read(),
        old.clone().take_save_after_first_read(),
        "sequence {n}"
    );
}

#[test]
fn every_sequence_of_four_steps_answers_as_the_old_flags() {
    for n in 0..STEPS.pow(4) {
        let (mut new, mut old) = (RefreshRequest::default(), Flags::default());
        let mut rest = n;
        for _ in 0..4 {
            step(rest % STEPS, &mut new, &mut old, n);
            rest /= STEPS;
            answers_as(&new, &old, n);
        }
    }
}

#[test]
fn a_new_request_asks_nothing() {
    let mut r = RefreshRequest::default();
    assert!(!r.asked() && !r.asked_by_update() && !r.update_false());
    assert_eq!(r.take(), None);
}

#[test]
fn an_update_asks_with_its_argument() {
    let mut r = RefreshRequest::default();
    r.ask_by_update(false);
    assert!(r.asked_by_update_false());
    let t = r.take().unwrap();
    assert!(t.by_update && !t.without_curr && !t.without_parts && !t.from_code_inserted_line);
    assert!(!r.asked());
    // The argument outlives the refresh it asked for, until the next action starts.
    assert!(r.update_false());
    r.action_starts();
    assert!(!r.update_false());
}

#[test]
fn an_ask_after_an_action_is_not_by_update() {
    let mut r = RefreshRequest::default();
    r.ask_by_update(true);
    r.ask_after_action();
    assert!(r.asked() && !r.asked_by_update());
}

#[test]
fn a_plain_ask_keeps_whether_an_update_asked() {
    let mut r = RefreshRequest::default();
    r.ask_by_update(true);
    r.ask();
    assert!(r.asked_by_update());
}

#[test]
fn a_cancel_keeps_the_notes_until_the_next_take() {
    let mut r = RefreshRequest::default();
    r.ask_by_update(false);
    r.read_without_curr();
    r.cancel();
    assert!(!r.asked() && r.by_update_noted() && r.update_false());
    r.ask();
    let t = r.take().unwrap();
    assert!(t.by_update && t.without_curr);
}

#[test]
fn a_cancel_of_the_update_drops_its_argument() {
    let mut r = RefreshRequest::default();
    r.ask_by_update(false);
    r.cancel_update();
    assert!(!r.asked() && !r.by_update_noted() && !r.update_false());
}

#[test]
fn reads_done_end_the_refresh_and_are_taken() {
    let mut r = RefreshRequest::default();
    r.note_reads_done();
    r.ask_after_action();
    assert_eq!(r.take(), None);
    r.ask_after_action();
    assert!(r.take().is_some());
}

#[test]
fn a_lookup_ask_drops_the_left_view_and_notes_the_parts() {
    let mut r = RefreshRequest::default();
    r.ask_by_update(false);
    r.set_update_left_view(held());
    r.ask_lookup_left(false);
    assert!(r.update_left_view().is_none());
    assert!(r.take().unwrap().without_parts);
}

#[test]
fn scoped_runs_end_with_their_done() {
    let mut r = RefreshRequest::default();
    r.ask_after_action_pinned(true);
    assert!(r.pinned());
    r.pinned_run_done();
    assert!(!r.pinned());
    r.ask_host_reread(true);
    assert!(r.keeps_xrec() && !r.asked_by_update());
    assert!(r.clone().take().unwrap().without_parts);
    r.host_reread_done();
    assert!(!r.keeps_xrec());
}

#[test]
fn view_left_and_save_after_first_read_are_taken_once() {
    let mut r = RefreshRequest::default();
    r.note_view_left();
    r.note_save_after_first_read();
    assert!(r.take().is_none());
    assert!(r.view_left() && r.take_view_left() && !r.take_view_left());
    assert!(r.take_save_after_first_read() && !r.take_save_after_first_read());
}

#[test]
fn the_code_inserted_line_is_taken_or_forgotten() {
    let mut r = RefreshRequest::default();
    r.land_on_code_inserted_line();
    r.forget_code_inserted_line();
    assert!(!r.starts_from_code_inserted_line());
    r.ask_by_update(true);
    r.land_on_code_inserted_line();
    assert!(r.take().unwrap().from_code_inserted_line);
    assert!(!r.starts_from_code_inserted_line());
}
