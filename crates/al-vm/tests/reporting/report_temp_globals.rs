//! A temporary record that is a global of a report belongs to the report instance: a second report variable starts
//! with an empty one (Base Application's "Get Demand To Reserve" collects lines into such records and the Reservation
//! Worksheet reads them back through getters; corpus reports/extensions/TestPrecompiledReportExtensionDataItems.al).

use al_testkit::text;

const SRC: &str = r#"
table 69950 "RTG Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
report 69951 "RTG Rpt"
{
    ProcessingOnly = true; UseRequestPage = false;
    dataset { dataitem(Item; "RTG Row") { } }
    var
        Collected: Record "RTG Row" temporary;
        Mode: Integer;
    trigger OnPreReport()
    begin
        if Mode = 1 then begin
            Collected.Id := 1;
            Collected.Insert();
        end;
    end;
    procedure SetMode(M: Integer) begin Mode := M; end;
    procedure CountCollected(): Integer begin exit(Collected.Count()); end;
}
reportextension 69953 "RTG Ext" extends "RTG Rpt"
{
    var
        ExtCollected: Record "RTG Row" temporary;
    procedure FillExt() begin ExtCollected.Id := 7; ExtCollected.Insert(); end;
    procedure GetExt(var Dest: Record "RTG Row" temporary) begin Dest.Copy(ExtCollected, true); end;
}
codeunit 69954 C2
{
    procedure G(): Text
    var A: Report "RTG Rpt"; Perm: Record "RTG Row"; Dest: Record "RTG Row" temporary;
    begin
        Perm.Id := 5; Perm.Insert();
        A.FillExt();
        A.GetExt(Dest);
        exit(Format(Dest.Count()) + Format(Dest.IsTemporary()));
    end;
}
codeunit 69952 C
{
    procedure F(): Text
    var A: Report "RTG Rpt"; B: Report "RTG Rpt"; T: Text;
    begin
        A.SetMode(1);
        A.RunModal();
        T += Format(A.CountCollected());
        B.RunModal();
        T += Format(B.CountCollected());
        exit(T);
    end;
}
"#;

#[test]
fn a_second_report_variable_starts_with_an_empty_temporary_global() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "F")), "10");
}

#[test]
fn a_temporary_global_of_a_report_extension_stays_temporary_when_a_caller_copies_it() {
    // The permanent row must not show up through the shared table.
    assert_eq!(text(al_testkit::run_in(SRC, "C2", "G")), "1Yes");
}
