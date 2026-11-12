ActionEnabledFollowsRowAfterGoToKey => {
    id: "action-enabled-follows-row-after-go-to-key",
    assumed: "TestAction.Enabled() called right after a GoToKey that found its row answers the action's Enabled expression with the page variables as the page's OnAfterGetRecord left them for that row, not as the last row read left them (al-vm pages/action_enabled_row.rs). Only the reporter's shape: a List page over the virtual table Field with Editable = false, OnAfterGetRecord its only trigger, fields of Rec in a repeater and actions with an Enabled expression of page variables and an OnAction trigger, no page extension and no subscriber, run by Page.Run into a trapped TestPage. Every other shape (another page type, table, open, call before Enabled, Visible, a field control) keeps the variables' current value.",
    tests: "No Microsoft test. An outside report: the reporter's own repro on BC 29 (al-runner issue 7916, page 50313 and codeunit 50312) gives e2=Yes and e4=Yes after GoToKey onto a field with a relation.",
    probe: "Corpus branch al-runtime-rs-staging-pages-6, group G1: the same on own tables, OpenView, OpenEdit and a trapped Page.Run, after GoToKey, GoToRecord, Next, Previous, First and Last, with the variable set in OnAfterGetCurrRecord, with Visible, and with a field control's Enabled and Editable.",
    awaiting: Awaiting {
        issues: &[7916],
        files: &["tests/al-language/testpage/ActionEnabledAfterGoToKey.al"],
        since: "2026-11-12",
    },
}
