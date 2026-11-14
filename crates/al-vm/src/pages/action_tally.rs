//! What the AL code of one action did, counted while it ran, for the one gate that needs it
//! (`empty_list_one_row.rs`): an action on a List that stood on its new-row line over an empty view. The tally only counts;
//! it decides nothing. It is armed around that action's trigger (`actions.rs`), and the session's hooks (`record_op`,
//! `commit_builtin`, `codeunit_run`, `report_run`) add to it while it is armed.

use al_builtins::record_op::RecordOp;

/// What an action did while its trigger ran.
#[derive(Default)]
pub(crate) struct ActionTally {
    /// Every write method of a record (any table): the table object, the method and whether it had no argument.
    writes: Vec<(usize, RecordOp, bool)>,
    commits: u32,
    codeunit_runs: u32,
    /// Every report run: whether it ran with its request page.
    report_runs: Vec<bool>,
    /// Another action started while this one ran.
    nested_action: bool,
}

impl ActionTally {
    pub(crate) fn note_record_op(&mut self, method: RecordOp, table: usize, no_args: bool) {
        if method.writes() {
            self.writes.push((table, method, no_args));
        }
    }

    pub(crate) fn note_commit(&mut self) {
        self.commits += 1;
    }

    pub(crate) fn note_codeunit_run(&mut self) {
        self.codeunit_runs += 1;
    }

    /// The action did what pages/actions/ActionReportInsertsOneRowOnEmptyList.al's does and nothing else: it ran one report
    /// with its request page (:81 `Pg.AddByReport.Invoke()`, handler :232-:237), which inserted one row into the page's table
    /// with a statement-form `Insert()` (ActionReportInsertsRowsOnEmptyList.al:57 `Line.Insert()`), with no other write, no
    /// Commit and no Codeunit.Run.
    pub(crate) fn is_one_report_insert(&self, table: usize) -> bool {
        matches!(self.writes.as_slice(), [(t, RecordOp::Insert, true)] if *t == table)
            && self.report_runs == [true]
            && self.commits == 0
            && self.codeunit_runs == 0
            && !self.nested_action
    }
}

impl crate::Session {
    /// Before an action's trigger runs: arms a tally when `armed`, and taints the tally of an action this one runs inside. Returns
    /// that outer tally for `end_action_tally`.
    pub(crate) fn begin_action_tally(&mut self, armed: bool) -> Option<ActionTally> {
        let mut outer = self.pages.action_tally.take();
        if let Some(t) = outer.as_mut() {
            t.nested_action = true;
        }
        self.pages.action_tally = armed.then(ActionTally::default);
        outer
    }

    /// After the trigger: this action's tally, with the outer one back in place.
    pub(crate) fn end_action_tally(&mut self, outer: Option<ActionTally>) -> Option<ActionTally> {
        std::mem::replace(&mut self.pages.action_tally, outer)
    }

    /// A report starts (`report_run_entered`): counts it when a tally is armed. Answers whether its request page opens.
    pub(crate) fn note_report_start(&mut self, request_page: bool) -> bool {
        if let Some(t) = self.pages.action_tally.as_mut() {
            t.report_runs.push(request_page);
        }
        request_page
    }
}
