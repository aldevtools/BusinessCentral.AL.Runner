ProviderNoRowExtChain => {
    id: "provider-no-row-ext-chain",
    assumed: "On a Card page the test opened with OpenView and moved with GoToRecord, or opened with OpenEdit and moved with GoToKey, as its first call, onto a stored header whose Provider part shows no row, a ListPart that a page extension added outside FactBoxes with Provider = that part shows no row too (First, Last and Next answer No, a value reads empty). Only in the probe's shape: the Card with no property but the plain ones, no trigger, routine, variable or subscriber, its fields of Rec with only ApplicationArea, and exactly two parts; one page extension with no property, trigger, routine or variable that adds only that part, right after the Provider part, and modifies nothing; the Provider part linked by one field(..) entry, the linked part by two field(..) entries to the Provider's row, each entry joining the leading primary key fields in order, two fields of one type; both part pages ListParts with Editable = false, no other property, no trigger, routine, variable, extension or subscriber, one repeater of fields of Rec with only ApplicationArea and no control trigger; the three tables with no property, code, extension or subscriber; and no row of the linked part's table holds the blank link values. After that landing only value reads of a part's control, First, Last and Next of either part in any order (the probe measured First on both, First on the linked part alone, Last, Last, Next, Next, and no move), and Close run, while the test writes nothing to the store; every other call is refused.",
    tests: "No Microsoft test: al-runner issue 7392 (the reporter's smaller shape passed on real BC).",
    probe: "Corpus PR 1384 (merged as 2a2e2e4e, not pinned yet), testpage/parts/ProviderNoRowFirst.al: :192 ExtOutside_NoLines_OpenView_First, :240 _DetailsFirstOnly, :259 _NoMove, :278 _LastNext, :301 ExtOutside_NoLines_OpenEditGoToKey_First, measured alike on every major the corpus CI runs.",
    awaiting: Awaiting {
        issues: &[7392],
        files: &["tests/al-language/testpage/parts/ProviderNoRowFirst.al"],
        since: "2026-11-13",
    },
}
