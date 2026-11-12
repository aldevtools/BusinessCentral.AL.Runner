ActionEnabledFollowsRowAfterGoToKey => {
    id: "action-enabled-follows-row-after-go-to-key",
    assumed: "TestAction.Enabled() called right after a GoToKey that found its row answers the action's Enabled expression with the page variables as the page's OnAfterGetRecord sets them for that row, not as the last row read left them (al-vm pages/action_enabled_row.rs). Only the reporter's shape: a List page over the virtual table Field with Editable = false, OnAfterGetRecord its only trigger, fields of Rec in a repeater and actions with an Enabled expression of page variables and an OnAction trigger, no page extension and no subscriber, run by the static Page.Run(Page::X, Rec) into a trapped TestPage. Each variable the expression reads is set by OnAfterGetRecord from constants and fields of Rec only, and no other code of the page sets it. Only the first Enabled() after the GoToKey. Every other shape (another page type, table, open, a Page variable's Run, a trigger that reads other state, a variable an action sets, a second Enabled(), Visible, a field control) keeps the variables' current value.",
    tests: "No Microsoft test. An outside report: the reporter's own repro on real BC (al-runner issue 7916, page 50313 and codeunit 50312) gives e2=Yes and e4=Yes after GoToKey onto a field with a relation.",
    probe: "Open corpus PR 1367 (testpage/ActionEnabledAfterGoToKey.al): group G1 passes on every BC major of its CI run 43290818834: :610 the reporter's shape on Field, :430 a trapped filtered Page.Run on an own table; also OpenView :386, OpenEdit :408, an OnAfterGetCurrRecord variable :455, Visible :498, a field control :541 and a Card :574, and that a GoToKey on a list reads the rows again from the first row up to the target.",
    awaiting: Awaiting {
        issues: &[7916],
        files: &["tests/al-language/testpage/ActionEnabledAfterGoToKey.al"],
        since: "2026-11-12",
    },
}
