ViewBlankDateFilter => {
    id: "view-blank-date-filter",
    assumed: "A view property's condition `filter('')` on a Date field (an action's RunPageView, a part's SourceTableView or SubPageView, a report's or XMLport's view) selects the blank date, as a top-level page's SourceTableView does (pages/lt-cd-testpage-leads/CdTestPageLeads.al:842 SH_SourceTableView_DateFilterBlank_Rows) and `filter('')` of a TableRelation or CalcFormula selects the blank value (record/lt-relation-where-empty-term/RelationWhereEmptyTerm.al).",
    tests: "No Microsoft test: a partner app's tests (a list page with where(Date = filter('')) opens on the blank-date row and Next() is false).",
    probe: "An action with RunPageView = where(D = filter('')) over rows with a blank and a set date: the rows the opened list shows; the same view on a ListPart.",
}
