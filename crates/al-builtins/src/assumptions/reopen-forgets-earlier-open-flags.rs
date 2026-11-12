ReopenForgetsEarlierOpenFlags => {
    id: "reopen-forgets-earlier-open-flags",
    assumed: "A Card the test opened with OpenEdit, closed and opened again with OpenEdit through the same TestPage variable starts its second open without three states of its first open: that an action ran, that the client's save before a step failed, and that a control bound to an expression of a page without a source table was set (al-vm pages/open/page_open.rs `forget_earlier_open_flags`). Marked only on such a second open where one of them was set. Every other reopen (another page type or mode, an open the test did not make) keeps them, as on main; so do a save that ModifyAllowed = false denied (`save_aborted`) and a read-only temporary list that lost its only row (`gone_temp_list`) on every reopen.",
    tests: "No Microsoft test.",
    probe: "Open corpus PR 1367 (testpage/lifecycle/ReopenSameVariableState.al, group G8): nothing carries into the second open of the same variable: :303 an action ran, :335 a failed table OnModify on Next, :437 a no-source variable control set; :386 a save aborted by OnModifyRecord and :469 a temporary list row deleted under the cursor set none of the flags. Measured on one major locally, green on every major of its CI runs 43290818834 and 43307199491.",
    awaiting: Awaiting {
        issues: &[7645],
        files: &["tests/al-language/testpage/lifecycle/ReopenSameVariableState.al"],
        since: "2026-11-12",
    },
}
