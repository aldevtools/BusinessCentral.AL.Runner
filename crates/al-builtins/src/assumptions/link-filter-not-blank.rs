LinkFilterNotBlank => {
    id: "link-filter-not-blank",
    assumed: "A SubPageLink or RunPageLink term written `filter(<> '')` or `filter(<>'')` on a Code or a Text field of the linked page's table is the filter `<>''` on that field (al-sema `pages::NOT_BLANK_TERMS`, `link_entries`): a list part shows, and a list an action opens lists, the rows whose field is not blank, next to a `field(...)` term and alone, and Filter.GetFilter of the field answers `<>''`. The same term on a field of any other type, written with other blank space, or in a SubPageView, RunPageView or another property keeps the text it lowered to before (refused or raising the filter error, as on main).",
    tests: "No Microsoft test: the repro of al-runner issue 8873 (a Card whose ListPart is linked `\"Header No.\" = field(\"No.\"), \"Arrival No.\" = filter(<> '')` on a Code field opens).",
    probe: "Open corpus pull request 1548, tests/al-language/pages/subpagelink-empty-term/SubPageLinkNotBlank.al: the rows and Filter.GetFilter of a list part linked `filter(<> '')` on a Code field next to `field(...)` (:177, and on another host :263), `filter(<>'')` (:200), the term alone (:221), on a Text field (:242), and an action's RunPageLink on a Code (:282) and a Text field (:307).",
    awaiting: Awaiting {
        issues: &[8873],
        files: &["tests/al-language/pages/subpagelink-empty-term/SubPageLinkNotBlank.al"],
        since: "2026-11-23",
    },
}
