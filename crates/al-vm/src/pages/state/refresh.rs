#![deny(clippy::wildcard_enum_match_arm)]
//! The refresh a page waits for: the re-read the client makes once the call that asked for it returns (`CurrPage.Update`,
//! the end of an action, a part's Update that reads its host again), and what was noted for it. One owner for what used to be
//! twelve free flags on `PageState`: callers ask, note, cancel and take through the transitions below and read through the
//! questions. The rules that read the request stay at their callers, with their corpus citations.
//!
//! Set as a whole: each kind of ask is one transition. Taken as a whole: `take` takes the ask with every note `run_refresh` reads
//! at its start.
//! What is not one `Option`: a cancel (`cancel`) withdraws the ask and keeps the notes, an Update's argument outlives the
//! refresh it asked for, and two readers read a note while no refresh is asked (`by_update_noted`,
//! `starts_from_code_inserted_line`). Those are main's behavior and stay as they are (al-runner issue 7687).

use al_builtins::value::Value;

/// The refresh request of one page and what was noted for it.
#[derive(Debug, Clone, Default)]
pub struct RefreshRequest {
    /// A refresh waits.
    asked: bool,
    /// The notes `take` takes with the ask.
    next: NextRefresh,
    /// The last `CurrPage.Update` was `Update(false)` (it saves nothing). Cleared when an action starts (`action_starts`) and by
    /// `cancel_update`, not by `take`: readers inside the refresh still see it.
    update_false: bool,
    /// The keys of the rows the client held when the part's own `CurrPage.Update`, which asked for the refresh, found its view
    /// no longer starting with them (`part_update_left_view.rs`). Read only together with `asked_by_update`: every ask by
    /// Update writes it.
    update_left_view: Option<Vec<Vec<Value>>>,
    /// CurrPage.Update ran while the page's row was stored but outside the view the action had just set (list refresh,
    /// `note_view_left`); taken by the refresh of a list with OnFindRecord.
    view_left: bool,
    /// The action changed the row a list shows: it is saved after the first read of the refresh (misc/lt-probes-needed-7/
    /// PnGRefresh.al:503).
    save_after_first_read: bool,
    /// The refresh that runs now follows an action on a card with OnFindRecord, without CurrPage.Update: pinned
    /// (pages/triggers/BeCardOnFindRecord.al), so it marks no assumption. Set only while that refresh runs.
    pinned: bool,
    /// The row's read keeps xRec as it was while its triggers run (a host re-read after a part's Update). Set only while that
    /// refresh runs.
    keeps_xrec: bool,
}

/// What `take` takes with the ask.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct NextRefresh {
    /// The refresh was asked for by `CurrPage.Update` (not by an action that saved an edit).
    by_update: bool,
    /// The refresh reads the row (OnAfterGetRecord) without OnAfterGetCurrRecord: a card action that only saved.
    without_curr: bool,
    /// The refresh does not make the parts read their rows again (they just did).
    without_parts: bool,
    /// The reads after an action on a list without OnAfterGetCurrRecord ended the refresh: `CurrPage.Update` reads nothing
    /// more (pages/list-order/AiListOrderTests.al:233 Keep_Plain).
    reads_done: bool,
    /// The refresh starts from a new line the table code inserted inside the validation (`code_inserted_new_line.rs`).
    from_code_inserted_line: bool,
}

/// What `run_refresh` runs with: the notes of a refresh that was asked for and whose reads are not done.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TakenRefresh {
    /// Asked for by `CurrPage.Update`.
    pub by_update: bool,
    /// Read the row without OnAfterGetCurrRecord.
    pub without_curr: bool,
    /// The parts do not read their rows again.
    pub without_parts: bool,
    /// Starts from a new line the table code inserted inside the validation.
    pub from_code_inserted_line: bool,
}

impl RefreshRequest {
    /// `CurrPage.Update(save)` asked for the refresh.
    pub fn ask_by_update(&mut self, save: bool) {
        self.asked = true;
        self.next.by_update = true;
        self.update_false = !save;
    }

    /// The rows a part held when its own Update was asked, when its view no longer starts with them (`None` otherwise).
    pub fn set_update_left_view(&mut self, held: Option<Vec<Vec<Value>>>) {
        self.update_left_view = held;
    }

    /// The refresh of `CurrPage.Update(false)` a TestField.Lookup left for later is asked again; `with_parts`: the parts
    /// read their rows again after it.
    pub fn ask_lookup_left(&mut self, with_parts: bool) {
        self.asked = true;
        self.next.by_update = true;
        self.update_left_view = None;
        self.next.without_parts = !with_parts;
    }

    /// An action asks for the refresh, not by Update (`actions.rs`, `dirty_step.rs`).
    pub fn ask_after_action(&mut self) {
        self.asked = true;
        self.next.by_update = false;
    }

    /// The same, for the refresh that runs at once after an action on a card with OnFindRecord; `pinned` when a corpus case
    /// pins it. `pinned_run_done` ends it.
    pub fn ask_after_action_pinned(&mut self, pinned: bool) {
        self.ask_after_action();
        self.pinned = pinned;
    }

    /// The pinned refresh of `ask_after_action_pinned` ended.
    pub fn pinned_run_done(&mut self) {
        self.pinned = false;
    }

    /// A part's Update reads its host again, the host's triggers seeing xRec as it was; `without_parts`: the host's parts
    /// do not read their rows with it. `host_reread_done` ends it.
    pub fn ask_host_reread(&mut self, without_parts: bool) {
        self.ask_after_action();
        self.next.without_parts = without_parts;
        self.keeps_xrec = true;
    }

    /// The host re-read of `ask_host_reread` ended.
    pub fn host_reread_done(&mut self) {
        self.keeps_xrec = false;
    }

    /// A refresh is asked for and every note stays as it is (also whether an Update asked for it).
    pub fn ask(&mut self) {
        self.asked = true;
    }

    /// The refresh reads the row without OnAfterGetCurrRecord.
    pub fn read_without_curr(&mut self) {
        self.next.without_curr = true;
    }

    /// The reads after an action ended the refresh: it reads nothing more.
    pub fn note_reads_done(&mut self) {
        self.next.reads_done = true;
    }

    /// The page left a new line for the row the table code inserted inside the validation.
    pub fn land_on_code_inserted_line(&mut self) {
        self.next.from_code_inserted_line = true;
    }

    /// What `land_on_code_inserted_line` noted is gone.
    pub fn forget_code_inserted_line(&mut self) {
        self.next.from_code_inserted_line = false;
    }

    /// Update ran while the stored row stood outside the view the action had just set.
    pub fn note_view_left(&mut self) {
        self.view_left = true;
    }

    /// Takes `note_view_left`.
    pub fn take_view_left(&mut self) -> bool {
        std::mem::take(&mut self.view_left)
    }

    /// The action changed the shown row of a list: it is saved after the first read of the refresh.
    pub fn note_save_after_first_read(&mut self) {
        self.save_after_first_read = true;
    }

    /// Takes `note_save_after_first_read`.
    pub fn take_save_after_first_read(&mut self) -> bool {
        std::mem::take(&mut self.save_after_first_read)
    }

    /// An action's OnAction starts: an Update(false) before it no longer counts.
    pub fn action_starts(&mut self) {
        self.update_false = false;
    }

    /// The refresh is withdrawn (its caller ran it its own way, or an error ended the call). The notes stay until `take`.
    pub fn cancel(&mut self) {
        self.asked = false;
    }

    /// The refresh is withdrawn and the Update that asked for it no longer counts.
    pub fn cancel_update(&mut self) {
        self.asked = false;
        self.next.by_update = false;
        self.update_false = false;
    }

    /// Takes the ask and every note `run_refresh` reads: `Some` when a refresh was asked for and its reads are not done.
    pub fn take(&mut self) -> Option<TakenRefresh> {
        let next = std::mem::take(&mut self.next);
        let asked = std::mem::take(&mut self.asked);
        (asked && !next.reads_done).then_some(TakenRefresh {
            by_update: next.by_update,
            without_curr: next.without_curr,
            without_parts: next.without_parts,
            from_code_inserted_line: next.from_code_inserted_line,
        })
    }

    /// A refresh waits.
    pub fn asked(&self) -> bool {
        self.asked
    }

    /// A refresh waits and `CurrPage.Update` asked for it.
    pub fn asked_by_update(&self) -> bool {
        self.asked && self.next.by_update
    }

    /// A refresh waits and `CurrPage.Update(false)` asked for it.
    pub fn asked_by_update_false(&self) -> bool {
        self.asked_by_update() && self.update_false
    }

    /// An Update asked for the next refresh, also when the ask was withdrawn since (`cancel` keeps the note).
    pub fn by_update_noted(&self) -> bool {
        self.next.by_update
    }

    /// The last `CurrPage.Update` was `Update(false)`, and no action start or `cancel_update` came since.
    pub fn update_false(&self) -> bool {
        self.update_false
    }

    /// The rows of `set_update_left_view`; read it only together with `asked_by_update`.
    pub fn update_left_view(&self) -> Option<&Vec<Vec<Value>>> {
        self.update_left_view.as_ref()
    }

    /// The page landed on the row the table code inserted, and no `take` or `forget_code_inserted_line` came since.
    pub fn starts_from_code_inserted_line(&self) -> bool {
        self.next.from_code_inserted_line
    }

    /// Update ran while the stored row stood outside the view the action had just set.
    pub fn view_left(&self) -> bool {
        self.view_left
    }

    /// The refresh that runs now is pinned (`ask_after_action_pinned`).
    pub fn pinned(&self) -> bool {
        self.pinned
    }

    /// The read that runs now keeps xRec (`ask_host_reread`).
    pub fn keeps_xrec(&self) -> bool {
        self.keeps_xrec
    }
}

/// The twelve flags `RefreshRequest` replaced, with the bodies the call sites had, for the bounded comparison. Delete it when a
/// probed behavior change makes the old flags wrong.
#[cfg(test)]
mod model {
    use super::TakenRefresh;
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

#[cfg(test)]
mod tests {
    use super::{RefreshRequest, model::Flags};
    use al_builtins::value::Value;

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
        assert_eq!(new.by_update_noted(), old.refresh_by_update, "sequence {n}");
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
        assert!(!r.asked() && !r.by_update_noted());
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
}
