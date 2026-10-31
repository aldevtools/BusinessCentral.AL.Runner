RunObjectTargetViewWithoutLink => {
    id: "run-object-target-view-without-link",
    assumed: "A List reached by an action's RunObject with RunPageOnRec, or with RunPageView and no RunPageLink, whose page declares SourceTableView, combines the two as RunPageLink does (pages/lt-rz-outside-page-moves/RzRunPageLinkView.al): the view's filters in group 2, the action's RunPageView where filter in group 3, the target's SORTING and ORDER kept; it lists the rows of both and opens on the first. RunPageLink beside the view is pinned by that file.",
    tests: "No Microsoft test: the reporter shape with RunPageLink is pinned; RunPageOnRec and RunPageView alone keep the same rule.",
    probe: "RunPageOnRec, and RunPageView without RunPageLink, to a List with SourceTableView = sorting(...) order(descending), = where(...), and both: the open row, the rows from First/Next and Filter.GetFilter of the view's field.",
}
