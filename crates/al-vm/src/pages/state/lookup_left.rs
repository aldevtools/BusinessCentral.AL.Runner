//! What a TestField.Lookup left for the next TestPage calls (`LookupLeft`: the refresh left for later
//! and who ran it, a written or cancelled value, a part a call reached, the host that read its
//! unlinked ListPart's row). Flags behind methods, because they do not exclude each other (al-runner
//! issue 7686). Entry points: `LookupLeft`, field `PageState::lookup_left`.
#![deny(clippy::wildcard_enum_match_arm)]
//! What a TestField.Lookup left on its page for the next TestPage calls: the refresh of `CurrPage.Update(false)` that BC runs
//! later on a host with row triggers whose ListPart links two or three fields (`Session::defer_lookup_refresh`), who ran it,
//! whether the Lookup wrote a value or was cancelled, and what the host read for its unlinked ListPart. The rules that read it
//! are at the callers in `pages/lookup_refresh.rs`, `pages/lookup_next.rs`, `pages/fn_controls.rs`, `pages/host_follows.rs` and
//! `pages/shown.rs`, with their corpus citations.
//!
//! The flags stay flags: they do not exclude each other. A new Lookup leaves its refresh for later without ending what the
//! last one's refresh left (`refresh_used` stays), and a refresh handed to the host's next move stays when another call comes
//! first.

/// What a Lookup left on a page. Nothing is set (`Default`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LookupLeft {
    /// The Lookup left its refresh for later: reads and Close run no trigger; any other TestPage call on its tree is refused
    /// until Close.
    deferred: bool,
    /// The first TestPart move after the Lookup ran that refresh (`Session::run_lookup_refresh_left`); the part's later moves
    /// and the host's Next are pinned until another call.
    used: bool,
    /// The host's Next is the first call after the Lookup: the move runs that refresh too (`Session::lookup_next_refresh`).
    next_refresh: bool,
    /// The host's move is the first call after the Lookup: the refresh runs after the move (`Session::lookup_move_refresh_after`).
    move_refresh: bool,
    /// The host's OnAfterGetCurrRecord raised an error while its move carried the refresh (`Session::lookup_move_stopped`).
    curr_record_raised: bool,
    /// The refresh the host's Next ran was on a host without OnAfterGetCurrRecord: its reads differ.
    next_plain: bool,
    /// The TestField.Lookup running on this page wrote a value (its OnLookup returned true); false after a cancelled Lookup.
    value_set: bool,
    /// The last TestField.Lookup on this page was cancelled and only reads and part moves followed (`note_lookup_cancelled`).
    cancelled: bool,
    /// A TestPage call reached this page as a part of its host (`lookup_refresh_deferred_call`).
    part_touched: bool,
    /// The host read the row of its unlinked ListPart after that part moved, and has not moved since
    /// (`Session::host_follows_part`).
    host_followed_part: bool,
}

impl LookupLeft {
    /// A TestField.Lookup starts on this page: it has written nothing yet.
    pub fn lookup_started(&mut self) {
        self.value_set = false;
    }

    /// The Lookup wrote the text its OnLookup returned.
    pub fn value_written(&mut self) {
        self.value_set = true;
    }

    /// The Lookup left its refresh for later.
    pub fn refresh_deferred(&mut self) {
        self.deferred = true;
    }

    /// The refresh left for later no longer waits: a call ran it, dropped it, or handed it on.
    pub fn refresh_taken(&mut self) {
        self.deferred = false;
    }

    /// A part's call ran the refresh left for later.
    pub fn refresh_used(&mut self) {
        self.used = true;
    }

    /// What the part's run of the refresh pinned no longer holds.
    pub fn used_ended(&mut self) {
        self.used = false;
    }

    /// The host's Next takes the refresh left for later and runs it within its move.
    pub fn refresh_to_host_next(&mut self) {
        self.deferred = false;
        self.next_refresh = true;
    }

    /// The host's move runs the refresh after it moved.
    pub fn refresh_after_host_move(&mut self) {
        self.move_refresh = true;
    }

    /// Whether the host's Next carries the refresh; reading it clears it.
    pub fn take_next_refresh(&mut self) -> bool {
        std::mem::take(&mut self.next_refresh)
    }

    /// Whether the host's move carries the refresh; reading it clears it.
    pub fn take_move_refresh(&mut self) -> bool {
        std::mem::take(&mut self.move_refresh)
    }

    /// The host's OnAfterGetCurrRecord raised an error; noted only while its move carries the refresh.
    pub fn curr_record_raised(&mut self) {
        if self.move_refresh {
            self.curr_record_raised = true;
        }
    }

    /// Whether the host's OnAfterGetCurrRecord raised since this was last read; reading it clears it.
    pub fn take_curr_record_raised(&mut self) -> bool {
        std::mem::take(&mut self.curr_record_raised)
    }

    /// The host's Next runs the refresh; `plain` when the host has no OnAfterGetCurrRecord.
    pub fn next_refresh_runs(&mut self, plain: bool) {
        self.next_plain = plain;
    }

    /// The host's step that ran the refresh is done.
    pub fn host_step_done(&mut self) {
        self.next_plain = false;
    }

    /// A TestField.Lookup on this page ended; `cancelled` when it was cancelled and left nothing.
    pub fn lookup_ended(&mut self, cancelled: bool) {
        self.cancelled = cancelled;
    }

    /// A call other than a read or a part move came after a cancelled Lookup.
    pub fn cancel_ended(&mut self) {
        self.cancelled = false;
    }

    /// A TestPage call reached this page as a part of its host.
    pub fn touched_as_part(&mut self) {
        self.part_touched = true;
    }

    /// The host read the row of its unlinked ListPart.
    pub fn host_followed_part(&mut self) {
        self.host_followed_part = true;
    }

    /// The host refreshed its parts: it no longer stands on the row it read for its unlinked ListPart.
    pub fn host_refreshed_parts(&mut self) {
        self.host_followed_part = false;
    }

    /// Whether the host read the row of its unlinked ListPart; reading it clears it.
    pub fn take_host_followed(&mut self) -> bool {
        std::mem::take(&mut self.host_followed_part)
    }

    /// The refresh left for later still waits.
    pub fn deferred(&self) -> bool {
        self.deferred
    }

    /// A part's call ran the refresh and what it pinned still holds.
    pub fn used(&self) -> bool {
        self.used
    }

    /// The host's move carries the refresh.
    pub fn move_refresh(&self) -> bool {
        self.move_refresh
    }

    /// The refresh the host's Next runs is on a host without OnAfterGetCurrRecord.
    pub fn next_plain(&self) -> bool {
        self.next_plain
    }

    /// The running Lookup wrote a value.
    pub fn value_set(&self) -> bool {
        self.value_set
    }

    /// The last Lookup was cancelled and only reads and part moves followed.
    pub fn cancelled(&self) -> bool {
        self.cancelled
    }

    /// The refresh waits, a part ran it, or the last Lookup was cancelled.
    pub fn deferred_used_or_cancelled(&self) -> bool {
        self.deferred || self.used || self.cancelled
    }

    /// A TestPage call reached this page as a part of its host.
    pub fn part_touched(&self) -> bool {
        self.part_touched
    }

    /// The host read the row of its unlinked ListPart and has not moved since.
    pub fn host_followed(&self) -> bool {
        self.host_followed_part
    }
}

#[cfg(test)]
mod tests {
    use super::LookupLeft;

    #[test]
    fn a_new_page_has_nothing_left() {
        let l = LookupLeft::default();
        assert!(!l.deferred() && !l.used() && !l.move_refresh() && !l.next_plain() && !l.value_set());
        assert!(!l.cancelled() && !l.deferred_used_or_cancelled() && !l.part_touched() && !l.host_followed());
    }

    #[test]
    fn a_new_lookup_forgets_the_written_value() {
        let mut l = LookupLeft::default();
        l.value_written();
        assert!(l.value_set());
        l.lookup_started();
        assert!(!l.value_set());
    }

    #[test]
    fn a_deferred_refresh_waits_until_taken() {
        let mut l = LookupLeft::default();
        l.refresh_deferred();
        assert!(l.deferred() && l.deferred_used_or_cancelled());
        l.refresh_taken();
        assert!(!l.deferred() && !l.deferred_used_or_cancelled());
    }

    #[test]
    fn a_new_deferral_keeps_what_the_last_refresh_used() {
        let mut l = LookupLeft::default();
        l.refresh_used();
        l.refresh_deferred();
        assert!(l.used() && l.deferred());
        l.used_ended();
        assert!(!l.used() && l.deferred());
    }

    #[test]
    fn the_host_next_takes_the_refresh_once() {
        let mut l = LookupLeft::default();
        l.refresh_deferred();
        l.refresh_to_host_next();
        assert!(!l.deferred());
        assert!(l.take_next_refresh());
        assert!(!l.take_next_refresh());
        l.next_refresh_runs(true);
        assert!(l.next_plain());
        l.host_step_done();
        assert!(!l.next_plain());
    }

    #[test]
    fn the_host_move_carries_the_refresh_once() {
        let mut l = LookupLeft::default();
        l.refresh_after_host_move();
        assert!(l.move_refresh());
        assert!(l.take_move_refresh());
        assert!(!l.move_refresh() && !l.take_move_refresh());
    }

    #[test]
    fn a_curr_record_error_is_noted_only_while_the_move_carries_the_refresh() {
        let mut l = LookupLeft::default();
        l.curr_record_raised();
        assert!(!l.take_curr_record_raised());
        l.refresh_after_host_move();
        l.curr_record_raised();
        assert!(l.take_curr_record_raised());
        assert!(!l.take_curr_record_raised());
    }

    #[test]
    fn a_cancelled_lookup_holds_until_another_call() {
        let mut l = LookupLeft::default();
        l.lookup_ended(true);
        assert!(l.cancelled() && l.deferred_used_or_cancelled());
        l.cancel_ended();
        assert!(!l.cancelled());
        l.lookup_ended(true);
        l.lookup_ended(false);
        assert!(!l.cancelled());
    }

    #[test]
    fn a_part_touch_stays() {
        let mut l = LookupLeft::default();
        l.touched_as_part();
        l.refresh_taken();
        assert!(l.part_touched());
    }

    #[test]
    fn the_host_follows_its_part_until_it_refreshes_or_closes() {
        let mut l = LookupLeft::default();
        l.host_followed_part();
        assert!(l.host_followed());
        l.host_refreshed_parts();
        assert!(!l.host_followed());
        l.host_followed_part();
        assert!(l.take_host_followed());
        assert!(!l.take_host_followed() && !l.host_followed());
    }
}
