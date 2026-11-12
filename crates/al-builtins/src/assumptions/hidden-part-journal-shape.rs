HiddenPartJournalShape => {
    id: "hidden-part-journal-shape",
    assumed: "A Worksheet or List page the test opened with OpenView, with OnOpenPage, OnAfterGetCurrRecord and OnClosePage as its only triggers, shows one unlinked ListPart (Editable = false, temporary or stored, a repeater of fields, OnOpenPage, OnAfterGetRecord, OnAfterGetCurrRecord and OnClosePage) hidden by a Visible expression that the host's code sets from a procedure of the part page (al-vm pages/hidden_part_journal.rs). An action whose body turns it visible, on the row the host stands on: the part reads its rows before the host's refresh and in it, and a temporary part once more after it on the majors that do (al-version `journal_temp_part_reads_after_refresh`). A GoToRecord that stays: the part reads nothing. Next that leaves it hidden with no row: nothing, or OnAfterGetCurrRecord on the blank row of a temporary part on the majors that do. Previous that shows it with one row: it reads its row. Every other page, part, state or major is refused at the turn and at the move; a GoToKey and First or Last keep the earlier path.",
    tests: "No Microsoft test (the journals of Base Application and the Sustainability app show the Workflow Status FactBox so, with more triggers and properties, which stay refused).",
    probe: "Open corpus PR 1367 (testpage/parts/HiddenPartJournalShape.al, group G2): :537 Worksheet and :589 List by value, :563 and :615 by var, :642 the request before the open, :672 the stored part. Green on every major of its CI run 43307199491, with a version guard on the temporary part's send and next.",
    awaiting: Awaiting {
        issues: &[7922],
        files: &["tests/al-language/testpage/parts/HiddenPartJournalShape.al"],
        since: "2026-11-12",
    },
}
