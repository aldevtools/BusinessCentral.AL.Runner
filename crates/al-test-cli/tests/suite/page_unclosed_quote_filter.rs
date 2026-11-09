//! A page opened on a record whose filter has an unclosed quote after `@` (al-runner issue 7462): which rows the filter keeps
//! is not pinned, so the page's read of its rows is refused, not answered as an empty page.

use al_testkit::outcomes_in;
use al_testrun::Outcome;

const SRC: &str = r#"
table 69992 "PUQ Row" { fields { field(1; Code; Code[10]) { } field(2; Name; Text[50]) { } } keys { key(PK; Code) { Clustered = true; } } }
page 69993 "PUQ List"
{
    PageType = List; SourceTable = "PUQ Row"; UsageCategory = None;
    layout { area(Content) { repeater(R) { field(Code; Rec.Code) { ApplicationArea = All; } field(Name; Rec.Name) { ApplicationArea = All; } } } }
}
codeunit 69994 T
{
    Subtype = Test;
    [Test]
    procedure ListOnFilteredRecord()
    var R: Record "PUQ Row"; L: TestPage "PUQ List";
    begin
        R.Code := 'A'; R.Name := ''''; R.Insert();
        R.SetFilter(Name, '@''');
        L.Trap();
        Page.Run(Page::"PUQ List", R);
        if L.First() then;
    end;
    [Test]
    procedure TestPageFilter()
    var R: Record "PUQ Row"; L: TestPage "PUQ List";
    begin
        R.Code := 'A'; R.Name := ''''; R.Insert();
        L.OpenView();
        L.Filter.SetFilter(Name, '@''');
        if L.First() then;
    end;
}
"#;

#[test]
fn a_page_read_through_an_unclosed_quote_filter_is_refused() {
    for (name, outcome) in outcomes_in(SRC, "T") {
        assert!(
            matches!(&outcome, Outcome::NotRun(why) if why.contains("matching not pinned")),
            "{name}: {outcome:?}"
        );
    }
}
