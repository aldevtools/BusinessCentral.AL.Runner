PageControlRecMethodSource => {
    id: "page-control-rec-method-source",
    assumed: "On the own layout of a List or Card page over a table (not a page extension, a report's request page or another page type), a field control whose source is `Rec.Count`, `Rec.CountApprox`, `Rec.GetFilters`, `Rec.IsEmpty`, `Rec.TableCaption` or `Rec.TableName`, written without parentheses and unquoted (and `Rec.Count()` or a bare `Count`, when no routine of the page or the table is named Count), in a repeater or outside it, shows the value of that Record method for the page's rows as a call in the page's code does, so a TestPage filter and the SourceTableView count (also after TestPage.Last on a page without parts; Rec.Count 3 on every row of a table of 3 rows, 2 after a TestPage filter Id 2..3 and under a SourceTableView of 2 rows; GetFilters empty, then the TestPage filter; IsEmpty No; TableCaption and TableName the table's names). A field of the table with that name is that field (Rec.Count with a field Count shows 10, 20, 30). The control cannot be written. A table with a procedure of that name, `CountApprox` under a filter, a Record-level SetRange before Page.Run and every other method, page type, object kind or source form keep the refusal.",
    tests: "No Microsoft test. al-runner issue 7875 (the reporter's repro: a control with the source `Rec.Count`).",
    probe: "Open corpus pull request 1525, tests/al-language/pages/controls/RecCountControl.al, green on all three pinned majors of a local Business Central with no version difference (corpus CI not run yet).",
    awaiting: Awaiting {
        issues: &[7875],
        files: &["tests/al-language/pages/controls/RecCountControl.al"],
        since: "2026-11-22",
    },
}
