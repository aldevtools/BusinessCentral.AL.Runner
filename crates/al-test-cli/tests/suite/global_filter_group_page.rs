//! A list page opened without code of its own on a table whose filter group 1 (the global group) another record set: BC
//! shows the page with that filter (types/lt-code-digits-page-run-view/P145PageRunView.al:232-235; SF_B_NoRecordArgument
//! :273-289 reads `g1=[Qty: >=3]` on the page), the runtime would list every row, so the page's read is refused.

use al_test_cli::report::Outcome;

const SRC: &str = r#"
table 70960 "GF Row"
{
    fields { field(1; No; Integer) { } field(2; Qty; Integer) { } }
    keys { key(PK; No) { } }
}
page 70961 "GF List"
{
    PageType = List; SourceTable = "GF Row";
    layout { area(Content) { repeater(G) { field(NoCtl; Rec.No) { } } } }
}
codeunit 70962 T
{
    Subtype = Test;
    [PageHandler] procedure H(var P: TestPage "GF List") begin P.First(); end;

    [Test]
    procedure SetGlobal()
    var R: Record "GF Row"; I: Integer;
    begin
        for I := 1 to 6 do begin R.No := I; R.Qty := I; R.Insert(); end;
        R.FilterGroup(1);
        R.SetFilter(Qty, '>=3');
        R.FilterGroup(0);
    end;

    [Test] [HandlerFunctions('H')]
    procedure PageWithoutRecord()
    begin
        Page.Run(Page::"GF List");
    end;
}
"#;

#[test]
fn a_page_on_a_table_with_a_global_filter_from_another_record_is_refused() {
    let o = al_testkit::outcomes_in(SRC, "T");
    assert_eq!(o.len(), 2, "{o:?}");
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
    match &o[1].1 {
        Outcome::NotRun(why) => assert!(why.contains("filter group 1"), "{why}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}
