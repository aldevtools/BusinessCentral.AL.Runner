//! A data item whose own filter (DataItemTableView) is on a field its DataItemLink joins: both apply, the child shows the rows that
//! satisfy the view and the link (reports/F3ReportTests.al:68 LinkFilter_InItemView, :116 LinkFilter_CallerViewOverItemView).
//! Microsoft's Assembly Header item of Get Source Documents has `where("Document Type" = const(Order))` and a link on "Document Type".

use al_testkit::text;

const SRC: &str = r#"
table 69970 "RLF Parent" { fields { field(1; Id; Integer) { } field(2; G; Integer) { } } keys { key(PK; Id) { } } }
table 69971 "RLF Child" { fields { field(1; Id; Integer) { } field(2; G; Integer) { } } keys { key(PK; Id) { } } }
codeunit 69972 "RLF Log"
{
    SingleInstance = true;
    var T: Text;
    procedure Add(S: Text) begin T += S + ';'; end;
    procedure Take(): Text var R: Text; begin R := T; T := ''; exit(R); end;
}
report 69973 "RLF Rpt"
{
    ProcessingOnly = true; UseRequestPage = false;
    dataset
    {
        dataitem(P; "RLF Parent")
        {
            dataitem(C; "RLF Child")
            {
                DataItemLink = G = field(G);
                DataItemTableView = sorting(Id) where(G = filter(1 | 2));
                trigger OnAfterGetRecord() begin L.Add('p' + Format(P.Id) + 'c' + Format(C.Id)); end;
            }
        }
    }
    var L: Codeunit "RLF Log";
}
codeunit 69974 C
{
    procedure F(): Text
    var P: Record "RLF Parent"; C: Record "RLF Child"; L: Codeunit "RLF Log";
    begin
        P.Id := 1; P.G := 1; P.Insert(); P.Id := 2; P.G := 2; P.Insert(); P.Id := 3; P.G := 3; P.Insert();
        C.Id := 10; C.G := 1; C.Insert(); C.Id := 11; C.G := 2; C.Insert(); C.Id := 12; C.G := 3; C.Insert();
        Report.Run(Report::"RLF Rpt");
        exit(L.Take());
    end;
}
"#;

#[test]
fn the_link_and_a_view_filter_on_the_field_it_joins_both_apply() {
    // The view admits G 1 and 2: parent 3 finds no child.
    assert_eq!(text(al_testkit::run_in(SRC, "C", "F")), "p1c10;p2c11;");
}
