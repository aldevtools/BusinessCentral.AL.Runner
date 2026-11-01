PageextImplicitPromotedAnchors => {
    id: "pageext-implicit-promoted-anchors",
    assumed: "A page extension that adds to or modifies (Caption, ShowAs) an implicit group Category_Category4 to Category_Category20 (other than 7 and 12) the page does not declare, or anchors addafter/addbefore on the implicit <Action>_Promoted actionref of a legacy promoted action, opens the TestPage and its actions and actionrefs run as with a declared anchor (alc compiles every form: corpus writer Y64's compile facts; the run is pinned for addafter/addlast of Category_Category7 and 12 by pages/lt-up-page-ext/UpPromotedCategories.al, and for a group a legacy promoted action of the page names by pages/lt-page-ext-promoted/PageExtPromotedCategoryRisky.al).",
    tests: "No Microsoft test: al-runner issues 6646 and 6647, an ISV app suite whose tests pass on real BC.",
    probe: "Corpus writer Y64: pages/lt-up-page-ext/Y64PromotedImplicit.al (modify and addlast of Category_Category5 and 6, addafter/addbefore of <Action>_Promoted, a page without actions).",
    awaiting: Awaiting {
        issues: &[6646, 6647],
        files: &["tests/al-language/pages/lt-up-page-ext/Y64PromotedImplicit.al"],
        since: "2026-11-01",
    },
}
