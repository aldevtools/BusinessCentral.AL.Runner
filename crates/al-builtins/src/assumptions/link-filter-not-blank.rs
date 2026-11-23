LinkFilterNotBlank => {
    id: "link-filter-not-blank",
    assumed: "A SubPageLink or RunPageLink term written `filter(<> '')` or `filter(<>'')` on a Code or a Text field of the linked page's table is the filter `<>''` on that field (al-sema `pages::NOT_BLANK_TERMS`, `link_entries`): a list part shows, and a list an action opens lists, the rows whose field is not blank, next to a `field(...)` term and alone, and Filter.GetFilter of the field answers `<>''`. The same term on a field of any other type, written with other blank space, or in a SubPageView, RunPageView or another property keeps the text it lowered to before (refused or raising the filter error, as on main).",
    tests: "No Microsoft test: the repro of al-runner issue 8873 (a Card whose ListPart is linked `\"Header No.\" = field(\"No.\"), \"Arrival No.\" = filter(<> '')` on a Code field opens).",
    probe: "Corpus PR (al-runner issue 8873), tests/al-language/pages/subpagelink-empty-term/SubPageLinkNotBlank.al: the rows and Filter.GetFilter of a list part linked `filter(<> '')` on a Code field next to `field(...)` (SubPageLink_CodeSpaced_Rows, and on another host SubPageLink_CodeSpaced_OtherHost_Rows), `filter(<>'')` (SubPageLink_CodeTight_Rows), the term alone (SubPageLink_CodeAlone_Rows), on a Text field (SubPageLink_TextSpaced_Rows), and an action's RunPageLink on a Code and a Text field (RunPageLink_Code_Rows, RunPageLink_Text_Rows).",
    awaiting: Awaiting {
        issues: &[8873],
        files: &["tests/al-language/pages/subpagelink-empty-term/SubPageLinkNotBlank.al"],
        since: "2026-11-23",
    },
}
