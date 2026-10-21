//! A DataItemLink whose parent field is a FlowFilter. The child gets the parent's filter on the FlowFilter as its own
//! filter: none, a single value, a range or an or-filter, and the child's GetFilter does not show it
//! (report/lt-report-requestpage/RptFilters.al QP_DataItemLink_FlowFilter_* tests). Tests-ERM 134404 and 134429
//! TestDirectDebitCollectionEntryIsNotCreatedTwice reach it (report 1200 links "Currency Code" = field("Currency Filter")).

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
                trigger OnPreDataItem() begin L.Add('f[' + C.GetFilter(Cur) + ']'); end;
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
        C.Id := 10; C.Cur := ''; C.Insert(); C.Id := 11; C.Cur := 'EUR'; C.Insert(); C.Id := 12; C.Cur := 'USD'; C.Insert(); C.Id := 13; C.Cur := 'GBP'; C.Insert();
    end;

    procedure NoFilter(): Text
    var L: Codeunit "RLFF Log";
    begin
        Seed();
        Report.Run(Report::"RLFF Rpt");
        exit(L.Take());
    end;

    procedure WithFilter(F: Text): Text
    var P: Record "RLFF Parent"; R: Report "RLFF Rpt"; L: Codeunit "RLFF Log";
    begin
        Seed();
        P.SetFilter(CurFilter, F);
        R.SetTableView(P);
        R.UseRequestPage(false);
        R.Run();
        exit(L.Take());
    end;
}
"#;

#[test]
fn a_flowfilter_without_a_filter_links_nothing() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "NoFilter")), "f[];p1c10;p1c11;p1c12;p1c13;");
}

/// RptFilters.al QP_DataItemLink_FlowFilter_SingleValue, _Range ('E..G' leaves GBP out) and _OrFilter.
#[test]
fn a_flowfilter_with_a_filter_filters_the_child() {
    for (f, want) in [
        ("EUR", "f[];p1c11;"),
        ("E..G", "f[];p1c11;"),
        ("EUR|USD", "f[];p1c11;p1c12;"),
    ] {
        let r = al_testkit::run_with(SRC, "C", "WithFilter", vec![al_builtins::value::Value::Text(f.into())]).map(|(v, _)| v);
        assert_eq!(text(r), want, "{f}");
    }
}
