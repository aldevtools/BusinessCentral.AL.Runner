//! `Report.Execute(Number, '', RecordRef)`: the report runs over the RecordRef's filters, which reach the data item over its
//! table as a view does (reports/W3ReportSaveAsExecute.al:446-459 `Execute_WithRecordRef`: `pre filter=[5..6];row 5;row 6`;
//! al-runner issue 7446). With a parameters string the shapes reports/runtime/ExecuteParametersWithRecordRef.al pins run, and
//! the others are refused before the run (`reports/execute_args.rs`, `record_ref_with_parameters`).

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
    requestpage
    {
        layout { area(Content) { field(OptCtl; Opt) { ApplicationArea = All; } } }
    }
    trigger OnPostReport()
    begin
        L.Add('PostReport');
    end;
    var
        Opt: Integer;
        L: Codeunit "ER Log";
}
report 69854 "ER Two"
{
    ProcessingOnly = true;
    dataset
    {
        dataitem(First; "ER Row") { }
        dataitem(Second; "ER Row") { }
    }
    requestpage
    {
        layout { area(Content) { field(OptCtl; Opt) { ApplicationArea = All; } } }
    }
    var
        Opt: Integer;
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
        Report.Execute(Report::"ER Rpt", Params('ER Rpt', 69852), RecRef);
        exit(L.Take());
    end;

    procedure TwoItemsOverTheTable()
    var
        Row: Record "ER Row";
        RecRef: RecordRef;
    begin
        Seed();
        RecRef.GetTable(Row);
        Report.Execute(Report::"ER Two", Params('ER Two', 69854), RecRef);
    end;

    procedure VariableNotOpened()
    var
        Rpt: Report "ER Rpt";
        RecRef: RecordRef;
    begin
        Seed();
        Rpt.Execute(Params('ER Rpt', 69852), RecRef);
    end;

    procedure VariableEmptyParameters()
    var
        Rpt: Report "ER Rpt";
        Row: Record "ER Row";
        RecRef: RecordRef;
    begin
        Seed();
        RecRef.GetTable(Row);
        Rpt.Execute('', RecRef);
    end;

    local procedure Params(Name: Text; Id: Integer): Text
    begin
        exit('<?xml version="1.0" standalone="yes"?><ReportParameters name="' + Name + '" id="' + Format(Id) +
            '"><Options><Field name="Opt">3</Field></Options></ReportParameters>');
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
fn execute_with_parameters_and_a_recordref_runs_over_the_recordref() {
    assert_eq!(
        text(run_in(SRC, "T", "WithParameters")),
        "pre filter=[5..6];row 5;row 6;PostReport;"
    );
}

fn refused(method: &str, needle: &str) {
    match run_in(SRC, "T", method) {
        Err(Signal::Unsupported(m)) => assert!(m.contains(needle), "{m}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn a_recordref_over_a_table_two_data_items_read_is_refused_with_parameters() {
    refused("TwoItemsOverTheTable", "two data items");
}

#[test]
fn an_unopened_recordref_of_a_report_variable_is_refused_with_parameters() {
    refused("VariableNotOpened", "not open");
}

#[test]
fn a_report_variable_with_empty_parameters_and_a_recordref_is_refused() {
    refused("VariableEmptyParameters", "empty parameters and a RecordRef");
}
