PageextAnchorOnSeparator => {
    id: "pageext-anchor-on-separator",
    assumed: "A page extension's addafter or addbefore that names a separator of the base page's actions compiles, and the actions it adds sit next to that separator, are members of the TestPage and run on Invoke. The separator itself stays no control of the page. modify, addfirst, addlast, moveafter, movebefore, movefirst and movelast on a separator keep their refusal.",
    tests: "No Microsoft test. Outside reporter's measurement (al-runner issue 8078): alc compiles a page extension with addafter on separator Action134 of Base Application's Purchase Return Order and on Action132 of Purchase Credit Memo; whether the added action runs on real BC was not tried.",
    probe: "Corpus probe PR pending (not yet opened), tests/al-language/pages/extensions/PageExtSeparatorAnchor.al: addafter and addbefore on a separator in an action group and in an area; Invoke of the added action; the order and type rows of the Page Action virtual table with the extension (record/TestPageActionVirtualTable.al:22 notes a separator action type).",
    awaiting: Awaiting {
        issues: &[8078],
        files: &["tests/al-language/pages/extensions/PageExtSeparatorAnchor.al"],
        since: "2026-11-18",
    },
}
