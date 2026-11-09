//! `Report.Execute(Number, '', RecordRef)`: the report runs over the RecordRef's filters, which reach the data item over its
//! table as a view does (reports/W3ReportSaveAsExecute.al:446-459 `Execute_WithRecordRef`: `pre filter=[5..6];row 5;row 6`;
//! al-runner issue 7446). A parameters string together with a RecordRef is not pinned and stays refused.

use al_builtins::native::Signal;
use al_testkit::{run_in, text};

const SRC: &str = r#"
table 69850 "ER Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
codeunit 69851 "ER Log"
{
    SingleInstance = true;
    var
        Entries: Text;
    procedure Add(Entry: Text) begin Entries += Entry + ';'; end;
    procedure Take(): Text var R: Text; begin R := Entries; Entries := ''; exit(R); end;
}
report 69852 "ER Rpt"
{
    ProcessingOnly = true;
    dataset
    {
        dataitem(Row; "ER Row")
        {
            trigger OnPreDataItem()
            begin
                L.Add('pre filter=[' + Row.GetFilter(Id) + ']');
            end;
            trigger OnAfterGetRecord()
            begin
                L.Add('row ' + Format(Row.Id));
            end;
        }
    }
    trigger OnPostReport()
    begin
        L.Add('PostReport');
    end;
    var
        L: Codeunit "ER Log";
}
codeunit 69853 T
{
    var
        L: Codeunit "ER Log";

    local procedure Seed()
    var
        Row: Record "ER Row";
        I: Integer;
    begin
        Row.DeleteAll();
        for I := 1 to 6 do begin
            Row.Id := I;
            Row.Insert();
        end;
        L.Take();
    end;

    procedure Static(): Text
    var
        Row: Record "ER Row";
        RecRef: RecordRef;
    begin
        Seed();
        Row.SetFilter(Id, '5..6');
        RecRef.GetTable(Row);
        Report.Execute(Report::"ER Rpt", '', RecRef);
        exit(L.Take());
    end;

    procedure WithParameters(): Text
    var
        Row: Record "ER Row";
        RecRef: RecordRef;
    begin
        Seed();
        Row.SetFilter(Id, '5..6');
        RecRef.GetTable(Row);
        Report.Execute(Report::"ER Rpt", '<?xml version="1.0" standalone="yes"?><ReportParameters name="ER Rpt" id="69852"></ReportParameters>', RecRef);
        exit(L.Take());
    end;
}
"#;

#[test]
fn static_execute_with_a_recordref_runs_over_its_filters() {
    assert_eq!(
        text(run_in(SRC, "T", "Static")),
        "pre filter=[5..6];row 5;row 6;PostReport;"
    );
}

#[test]
fn execute_with_parameters_and_a_recordref_stays_refused() {
    let r = run_in(SRC, "T", "WithParameters");
    match r {
        Err(Signal::Unsupported(m)) => assert!(m.contains("RecordRef"), "{m}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}
