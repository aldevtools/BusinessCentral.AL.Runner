//! The `CalcFields` property of a report data item: the FlowFields it lists are calculated for each row before the row's
//! triggers (corpus reports/PnH3Link.al:CalcFields_Property_CalculatesTheFlowField, a top-level item with one field).
//! Nested items and lists of several fields are read the same way on an assumption (report-dataitem-calcfields-nested-or-list).

use al_testkit::text;

const SRC: &str = r#"
table 69960 "RCF Parent" { fields { field(1; Id; Integer) { } field(2; Cnt; Integer) { FieldClass = FlowField; CalcFormula = count("RCF Child" where(ParentId = field(Id))); } field(3; Tot; Decimal) { FieldClass = FlowField; CalcFormula = sum("RCF Child".Amount where(ParentId = field(Id))); } } keys { key(PK; Id) { } } }
table 69961 "RCF Child" { fields { field(1; Id; Integer) { } field(2; ParentId; Integer) { } field(3; Amount; Decimal) { } } keys { key(PK; Id) { } } }
codeunit 69962 "RCF Log"
{
    SingleInstance = true;
    var T: Text;
    procedure Add(S: Text) begin T += S + ';'; end;
    procedure Take(): Text var R: Text; begin R := T; T := ''; exit(R); end;
}
report 69963 "RCF Top"
{
    ProcessingOnly = true; UseRequestPage = false;
    dataset { dataitem(P; "RCF Parent") { CalcFields = Cnt; trigger OnAfterGetRecord() begin L.Add('p' + Format(P.Id) + ' cnt=' + Format(P.Cnt) + ' tot=' + Format(P.Tot)); end; } }
    var L: Codeunit "RCF Log";
}
report 69964 "RCF Plain"
{
    ProcessingOnly = true; UseRequestPage = false;
    dataset { dataitem(P; "RCF Parent") { trigger OnAfterGetRecord() begin L.Add('p' + Format(P.Id) + ' cnt=' + Format(P.Cnt)); end; } }
    var L: Codeunit "RCF Log";
}
report 69965 "RCF Nested"
{
    ProcessingOnly = true; UseRequestPage = false;
    dataset { dataitem(C; "RCF Child") { dataitem(P; "RCF Parent") { DataItemLink = Id = field(ParentId); CalcFields = Cnt, Tot; trigger OnAfterGetRecord() begin L.Add('p' + Format(P.Id) + ' cnt=' + Format(P.Cnt) + ' tot=' + Format(P.Tot)); end; } } }
    var L: Codeunit "RCF Log";
}
codeunit 69966 C
{
    procedure Seed()
    var P: Record "RCF Parent"; C: Record "RCF Child";
    begin
        P.Id := 1; P.Insert(); P.Id := 2; P.Insert();
        C.Id := 1; C.ParentId := 1; C.Amount := 5; C.Insert();
        C.Id := 2; C.ParentId := 1; C.Amount := 7; C.Insert();
        C.Id := 3; C.ParentId := 2; C.Amount := 1; C.Insert();
    end;
    procedure Top(): Text var L: Codeunit "RCF Log"; begin Seed(); Report.Run(Report::"RCF Top"); exit(L.Take()); end;
    procedure Plain(): Text var L: Codeunit "RCF Log"; begin Seed(); Report.Run(Report::"RCF Plain"); exit(L.Take()); end;
    procedure Nested(): Text var L: Codeunit "RCF Log"; begin Seed(); Report.Run(Report::"RCF Nested"); exit(L.Take()); end;
}
"#;

fn run(routine: &str) -> String {
    text(al_testkit::run_in(SRC, "C", routine))
}

#[test]
fn a_listed_flowfield_is_calculated_for_each_row_and_an_unlisted_one_is_not() {
    assert_eq!(run("Top"), "p1 cnt=2 tot=0;p2 cnt=1 tot=0;");
    assert_eq!(run("Plain"), "p1 cnt=0;p2 cnt=0;");
}

#[test]
fn a_nested_item_with_two_fields_calculates_both() {
    assert_eq!(run("Nested"), "p1 cnt=2 tot=12;p1 cnt=2 tot=12;p2 cnt=1 tot=1;");
}
