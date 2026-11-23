//! The refresh a page waits for: the re-read the client makes once the call that asked for it returns (`CurrPage.Update`,
//! the end of an action, a part's Update that reads its host again), and what was noted for it. `RefreshRequest` owns that
//! state: callers ask, note, cancel and take through its transitions and read through its questions. The rules that read the request stay at their callers, with their corpus citations.
//!
//! Set as a whole: each kind of ask is one transition. Taken as a whole: `take` takes the ask with every note `run_refresh` reads
//! at its start.
//! What is not one `Option`: a cancel (`cancel`) withdraws the ask and keeps the notes, an Update's argument outlives the
//! refresh it asked for, and a reader reads a note while no refresh is asked
//! (`starts_from_code_inserted_line`). Those are main's behavior and stay as they are (al-runner issue 7687).
//!
//! The refresh a page waits for and what was noted for it (`RefreshRequest`: who asked, Update's
//! argument, the rows a part's Update left, the notes `run_refresh` takes at once, the pinned run and
//! the host re-read that keeps xRec). `take` takes every note `run_refresh` reads; a cancel keeps the
//! notes, as main did (al-runner issue 7687). Entry points: `RefreshRequest`, field
//! `PageState::refresh`, taken by `run_refresh` (`actions.rs`).

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
    /// no longer starting with them (`part_update_left_view.rs`). Read only together with `asked_by_update`. `ask_by_update` does not
    /// write it: the caller writes it after `note_view_left` (`pageops.rs` `ask_update_refresh`), so it stays unwritten if that fails;
    /// a plain `ask` keeps the old rows.
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

    /// The refresh that runs now made the reads `note_reads_done` noted itself: the note is spent and does not end the next
    /// refresh (`code_inserted_coded.rs` `coded_refresh`, the page with code only; `validate_view_refresh.rs`
    /// `validate_view_refresh`, the pages of ListValidateFilterRefresh.al only).
    pub fn spend_reads_done(&mut self) {
        self.next.reads_done = false;
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

#[cfg(test)]
mod tests;
