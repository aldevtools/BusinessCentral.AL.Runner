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
report 69855 "ER Nest"
{
    ProcessingOnly = true;
    dataset
    {
        dataitem(Row; "ER Row")
        {
            trigger OnAfterGetRecord()
            begin
                L.Add('row ' + Format(Row.Id));
            end;
        }
    }
    requestpage
    {
        layout { area(Content) { field(OptCtl; Opt) { ApplicationArea = All; } } }
        trigger OnInit()
        begin
            case Nested.Mode() of
                1:
                    Report.Execute(Report::"ER Other", '');
                2:
                    Report.Execute(Report::"ER Other", '<?xml version="1.0" standalone="yes"?><ReportParameters name="ER Other" id="69857"><Options><Field name="Opt">1</Field></Options></ReportParameters>');
            end;
        end;
    }
    var
        Opt: Integer;
        L: Codeunit "ER Log";
        Nested: Codeunit "ER Mode";
}
codeunit 69856 "ER Mode"
{
    SingleInstance = true;
    var
        M: Integer;
    procedure SetMode(NewMode: Integer) begin M := NewMode; end;
    procedure Mode(): Integer begin exit(M); end;
}
table 69858 "ER Other Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
report 69857 "ER Other"
{
    ProcessingOnly = true;
    dataset
    {
        dataitem(OtherRow; "ER Other Row") { }
    }
    requestpage
    {
        layout { area(Content) { field(OptCtl; Opt) { ApplicationArea = All; } } }
    }
    var
        Opt: Integer;
}
table 69859 "ER Buf" { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } keys { key(PK; Id) { } } }
report 69860 "ER Data"
{
    dataset
    {
        dataitem(Row; "ER Row") { column(Id; Id) { } }
    }
    requestpage
    {
        layout { area(Content) { field(OptCtl; Opt) { ApplicationArea = All; } } }
        trigger OnOpenPage() begin L.Add('opened'); end;
    }
    var
        Opt: Integer;
        L: Codeunit "ER Log";
}
report 69861 "ER Data Close"
{
    dataset
    {
        dataitem(Row; "ER Row") { column(Id; Id) { } }
    }
    requestpage
    {
        layout { area(Content) { field(OptCtl; Opt) { ApplicationArea = All; } } }
        trigger OnClosePage() begin L.Add('closed'); end;
    }
    var
        Opt: Integer;
        L: Codeunit "ER Log";
}
report 69862 "ER Mixed"
{
    ProcessingOnly = true;
    dataset
    {
        dataitem(Row; "ER Row") { }
        dataitem(OtherRow; "ER Other Row") { }
    }
    requestpage
    {
        layout { area(Content) { field(OptCtl; Opt) { ApplicationArea = All; } } }
    }
    var
        Opt: Integer;
}
table 69863 "ER Ext Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
reportextension 69864 "ER Rpt Ext" extends "ER Rpt"
{
    dataset
    {
        addlast(Row) { dataitem(ExtRow; "ER Ext Row") { } }
    }
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

    procedure NestedEmpty(): Text
    begin
        exit(Nested(1));
    end;

    procedure NestedWithParameters(): Text
    begin
        exit(Nested(2));
    end;

    local procedure Nested(Mode: Integer): Text
    var
        Row: Record "ER Row";
        RecRef: RecordRef;
        M: Codeunit "ER Mode";
    begin
        Seed();
        M.SetMode(Mode);
        Row.SetFilter(Id, '5..6');
        RecRef.GetTable(Row);
        Report.Execute(Report::"ER Nest", Params('ER Nest', 69855), RecRef);
        M.SetMode(0);
        exit(L.Take());
    end;

    procedure SaveAsWithoutRecordRef(): Text
    var
        Buf: Record "ER Buf";
        OutStr: OutStream;
    begin
        Seed();
        Buf.Data.CreateOutStream(OutStr);
        Report.SaveAs(Report::"ER Data", Params('ER Data', 69860), ReportFormat::Xml, OutStr);
        exit(L.Take());
    end;

    procedure SaveAsWithRecordRef(): Text
    var
        Buf: Record "ER Buf";
        Row: Record "ER Row";
        RecRef: RecordRef;
        OutStr: OutStream;
    begin
        Seed();
        Row.SetFilter(Id, '5..6');
        RecRef.GetTable(Row);
        Buf.Data.CreateOutStream(OutStr);
        Report.SaveAs(Report::"ER Data", Params('ER Data', 69860), ReportFormat::Xml, OutStr, RecRef);
        exit(L.Take());
    end;

    procedure SaveAsClosePageWithRecordRef()
    var
        Buf: Record "ER Buf";
        Row: Record "ER Row";
        RecRef: RecordRef;
        OutStr: OutStream;
    begin
        Seed();
        Row.SetFilter(Id, '5..6');
        RecRef.GetTable(Row);
        Buf.Data.CreateOutStream(OutStr);
        Report.SaveAs(Report::"ER Data Close", Params('ER Data Close', 69861), ReportFormat::Xml, OutStr, RecRef);
    end;

    procedure VariableDescending()
    var
        Rpt: Report "ER Rpt";
        Row: Record "ER Row";
        RecRef: RecordRef;
    begin
        Seed();
        Row.SetFilter(Id, '2..5');
        Row.Ascending(false);
        RecRef.GetTable(Row);
        Rpt.Execute(Params('ER Rpt', 69852), RecRef);
    end;

    procedure VariableSecondItem()
    var
        Rpt: Report "ER Mixed";
        Other: Record "ER Other Row";
        RecRef: RecordRef;
    begin
        Seed();
        Other.SetFilter(Id, '1..2');
        RecRef.GetTable(Other);
        Rpt.Execute(Params('ER Mixed', 69862), RecRef);
    end;

    procedure TableOfAnExtensionItem()
    var
        ExtRow: Record "ER Ext Row";
        RecRef: RecordRef;
    begin
        Seed();
        ExtRow.SetFilter(Id, '1..2');
        RecRef.GetTable(ExtRow);
        Report.Execute(Report::"ER Rpt", Params('ER Rpt', 69852), RecRef);
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

/// The RecordRef belongs to its own run: a report its request page's OnInit runs does not take it or drop it (review of
/// runner PR 7815: an Execute with empty parameters inside the OnInit cleared it, and one with parameters took it).
#[test]
fn a_nested_execute_with_empty_parameters_leaves_the_recordref_of_the_outer_run() {
    assert_eq!(text(run_in(SRC, "T", "NestedEmpty")), "row 5;row 6;");
}

#[test]
fn a_nested_execute_with_parameters_does_not_take_the_recordref_of_the_outer_run() {
    assert_eq!(text(run_in(SRC, "T", "NestedWithParameters")), "row 5;row 6;");
}

/// A SaveAs with parameters and no RecordRef runs no request page trigger (only the form with a RecordRef runs OnOpenPage,
/// ExecuteParametersWithRecordRef.al:556-617); with a RecordRef it does.
#[test]
fn a_saveas_with_parameters_runs_on_open_page_only_with_a_recordref() {
    assert_eq!(text(run_in(SRC, "T", "SaveAsWithoutRecordRef")), "");
    assert_eq!(text(run_in(SRC, "T", "SaveAsWithRecordRef")), "opened;");
}

#[test]
fn on_close_page_in_a_saveas_with_parameters_and_a_recordref_is_refused() {
    refused("SaveAsClosePageWithRecordRef", "OnClosePage");
}

#[test]
fn a_descending_view_through_a_report_variable_is_refused_with_parameters() {
    refused("VariableDescending", "a view other than a filter");
}

#[test]
fn a_recordref_over_a_non_first_data_item_through_a_report_variable_is_refused() {
    refused("VariableSecondItem", "other than the first");
}

#[test]
fn a_recordref_over_a_table_only_a_reportextension_reads_is_refused() {
    refused("TableOfAnExtensionItem", "reportextensions");
}
