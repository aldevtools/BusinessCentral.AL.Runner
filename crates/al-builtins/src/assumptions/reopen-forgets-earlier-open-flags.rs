ReopenForgetsEarlierOpenFlags => {
    id: "reopen-forgets-earlier-open-flags",
    assumed: "A Card the test opened with OpenEdit, closed and opened again with OpenEdit through the same TestPage variable starts its second open without one state of its first open: that the client's save before a step failed (al-vm pages/open/page_open.rs `forget_earlier_open_flags`). Marked only on such a second open where that state was set. Every other reopen (another page type or mode, an open the test did not make) keeps it, as on main. Every other state of the first open keeps carrying on every reopen, as on main: that an action ran, that a control bound to an expression of a page without a source table was set, a save that ModifyAllowed = false denied (`save_aborted`) and a read-only temporary list that lost its only row (`gone_temp_list`).",
    tests: "No Microsoft test.",
    probe: "Open corpus PR 1367 (testpage/lifecycle/ReopenSameVariableState.al, group G8): :335 after a failed table OnModify on Next, nothing carries into the second open of the same variable. Its other cells set no flag whose clearing would show: :303 an action ran (the flag has no reader on a Card), :386 a save aborted by OnModifyRecord, :437 a variable control set (not an expression control), :469 a temporary list row deleted under the cursor. Measured on one major locally, green on every major of its CI runs 43290818834 and 43307199491.",
    awaiting: Awaiting {
        issues: &[7645],
        files: &["tests/al-language/testpage/lifecycle/ReopenSameVariableState.al"],
        since: "2026-11-12",
    },
}
