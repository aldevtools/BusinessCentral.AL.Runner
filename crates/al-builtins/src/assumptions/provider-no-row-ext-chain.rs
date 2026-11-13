ProviderNoRowExtChain => {
    id: "provider-no-row-ext-chain",
    assumed: "On a Card page the test opened with OpenView and moved with GoToRecord, or opened with OpenEdit and moved with GoToKey, as its first call, onto a stored header whose Provider part shows no row, a ListPart that a page extension added outside FactBoxes with Provider = that part shows no row too (First, Last and Next answer No, a value reads empty). Only in the probe's shape: the Card with no property but the plain ones, no trigger, no subscriber, one page extension that adds only that part, fields of Rec and exactly two parts; the Provider part a ListPart linked by one field(..), the linked part linked by two field(..) entries to the Provider's row; both part pages ListParts with Editable = false, no trigger, no extension and no subscriber; the three tables without code, extension or subscriber; and no row of the linked part's table holds the blank link values. After that landing only value reads, First, Last and Next of either part, in any order, and Close run; every other call is refused.",
    tests: "No Microsoft test: al-runner issue 7392 (the reporter's smaller shape passed on real BC 29.1).",
    probe: "Corpus PR 1384 (merged as 2a2e2e4e, not pinned yet), testpage/parts/ProviderNoRowFirst.al: :192 ExtOutside_NoLines_OpenView_First, :240 _DetailsFirstOnly, :259 _NoMove, :278 _LastNext, :301 ExtOutside_NoLines_OpenEditGoToKey_First, measured alike on BC 27.5, 28.5 and 29.0.",
    awaiting: Awaiting {
        issues: &[7392],
        files: &["tests/al-language/testpage/parts/ProviderNoRowFirst.al"],
        since: "2026-11-13",
    },
}
