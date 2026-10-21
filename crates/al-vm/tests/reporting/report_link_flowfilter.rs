//! A DataItemLink whose parent field is a FlowFilter. With no filter on it, the child gets no filter from that link: assumed
//! (report-link-flowfilter-without-filter) for Tests-ERM 134404 and 134429 TestDirectDebitCollectionEntryIsNotCreatedTwice
//! (report 1200 links "Currency Code" = field("Currency Filter") of a Customer with no Currency Filter, and a EUR invoice is
//! collected on real BC). A FlowFilter that has a filter is not pinned and is refused.

use al_builtins::native::Signal;
use al_testkit::text;

const SRC: &str = r#"
table 69975 "RLFF Parent" { fields { field(1; Id; Integer) { } field(2; CurFilter; Code[10]) { FieldClass = FlowFilter; } } keys { key(PK; Id) { } } }
table 69976 "RLFF Child" { fields { field(1; Id; Integer) { } field(2; Cur; Code[10]) { } } keys { key(PK; Id) { } } }
codeunit 69977 "RLFF Log"
{
    SingleInstance = true;
    var T: Text;
    procedure Add(S: Text) begin T += S + ';'; end;
    procedure Take(): Text var R: Text; begin R := T; T := ''; exit(R); end;
}
report 69978 "RLFF Rpt"
{
    ProcessingOnly = true; UseRequestPage = false;
    dataset
    {
        dataitem(P; "RLFF Parent")
        {
            dataitem(C; "RLFF Child")
            {
                DataItemLink = Cur = field(CurFilter);
                trigger OnAfterGetRecord() begin L.Add('p' + Format(P.Id) + 'c' + Format(C.Id)); end;
            }
        }
    }
    var L: Codeunit "RLFF Log";
}
codeunit 69979 C
{
    local procedure Seed()
    var P: Record "RLFF Parent"; C: Record "RLFF Child";
    begin
        P.Id := 1; P.Insert();
        C.Id := 10; C.Cur := ''; C.Insert(); C.Id := 11; C.Cur := 'EUR'; C.Insert();
    end;

    procedure NoFilter(): Text
    var L: Codeunit "RLFF Log";
    begin
        Seed();
        Report.Run(Report::"RLFF Rpt");
        exit(L.Take());
    end;

    procedure WithFilter(): Text
    var P: Record "RLFF Parent"; R: Report "RLFF Rpt"; L: Codeunit "RLFF Log";
    begin
        Seed();
        P.SetRange(CurFilter, 'EUR');
        R.SetTableView(P);
        R.UseRequestPage(false);
        R.Run();
        exit(L.Take());
    end;
}
"#;

#[test]
fn a_flowfilter_without_a_filter_links_nothing() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "NoFilter")), "p1c10;p1c11;");
}

#[test]
fn a_flowfilter_with_a_filter_is_refused() {
    match al_testkit::run_in(SRC, "C", "WithFilter") {
        Err(Signal::Unsupported(why)) => assert!(why.contains("FlowFilter"), "{why}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}
