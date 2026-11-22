ListNewLineCodedFactboxesRepro => {
    id: "list-new-line-coded-factboxes-repro",
    assumed: "Page 97383 \"LT NFR Lines\" of ListNewLineFactboxesRepro.al (the reporter's repro of al-runner issue 8434), line for line, with the same lines in its table 97375 \"LT NFR Line\", its three CardPart FactBoxes 97378 \"LT NFR Rules\", 97379 \"LT NFR Picture\" and 97380 \"LT NFR Where-Used\", the tables 97372 \"LT NFR Header\", 97373 \"LT NFR Setup\", 97374 \"LT NFR Kind\", 97376 \"LT NFR Value\" and 97377 \"LT NFR Change\", the calculation codeunit 97382 and the background task codeunit 97381 the part enqueues; objects may carry other names, every procedure the code calls is one of these objects' or Environment Information.IsSaaSInfrastructure, nothing subscribes to the parts, the other tables or the codeunits, and the page and its table have manual subscribers nobody binds at most. Opened as the repro opens it, by a Page.RunModal statement on a record with one SetRange on the key's Code field, on an empty view, with the parts shown and their records filtered by their links only, and driven as it drives it, New(), a SetValue of Attribute Name whose validation lands, one SetValue of Value and OK(): it behaves as the probe's page of list-new-line-coded-factboxes with the same calls does. The background task's read is refused when the test session changed the table it reads and has not committed. A second Value, OK() before the Value, and every call, open and view list-new-line-coded-factboxes refuses are refused; every other page and every other code keeps the earlier behavior.",
    tests: "No Microsoft test: al-runner issue 8434 (the reporter's repro, 1 test, green on real Business Central 29.1, 3/3).",
    probe: "Corpus pull request 1517 (open), tests/al-language/pages/currpage/ListNewLineFactboxesRepro.al: the repro's objects :18-597 and its test :611-683, as filed with object names and ids changed only.",
    awaiting: Awaiting {
        issues: &[8434],
        files: &["tests/al-language/pages/currpage/ListNewLineFactboxesRepro.al"],
        since: "2026-11-22",
    },
}
