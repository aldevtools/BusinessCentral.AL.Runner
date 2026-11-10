//! A parameters string with the XML declaration whose `<ReportParameters>` root has no `name` and `id`: its `Field` nodes
//! set the request page's variables as for a named root (reports/layouts-saveas/AMRdlcOutput.al:153, :394-408
//! TextRdlc_SaveAsXml_DoesNotRender `req=Yes`; reports/extensions/Y64XappRunOrder.al:156 for Execute). An unknown `Field`
//! under SaveAs is ignored (al-runner issue 7590, assumption `report-saveas-unknown-field-ignored`). Without the declaration
//! the string stays refused: BC raises an error the corpus has not measured.

use al_builtins::assumptions::{Assumption, clear_hits, take_hits};
use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::Built;

// The reporter's repro (al-runner issue 7590) with two changes: the test codeunit runs here as a plain one, and the data
// item reads a table of its own (the virtual table Integer needs the System app); and the shapes of the reporter's table
// with the second option NoteText. The repro itself, unchanged, was run with al-runner against BC 29's apps.
const SRC: &str = r#"
table 50260 "PX Blob"
{
    TableType = Temporary;

    fields
    {
        field(1; Id; Integer) { }
        field(2; Data; Blob) { }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

table 50267 "PX Row"
{
    fields { field(1; Number; Integer) { } }
    keys { key(PK; Number) { } }
}

report 50261 "PX Param Report"
{
    UsageCategory = None;

    dataset
    {
        dataitem(R; "PX Row")
        {
            column(Num; Number) { }
        }
    }

    requestpage
    {
        layout
        {
            area(Content)
            {
                field(StartCtl; StartDate) { ApplicationArea = All; }
            }
        }
    }

    trigger OnPreReport()
    begin
        if StartDate <> 20260115D then
            Error('StartDate is %1', StartDate);
    end;

    var
        StartDate: Date;
}

codeunit 50263 "PX Param Tests 2"
{
    procedure NoNameNoId()
    var
        B: Record "PX Blob";
        Out: OutStream;
    begin
        B.Data.CreateOutStream(Out);
        Report.SaveAs(Report::"PX Param Report", '<?xml version="1.0" standalone="yes"?><ReportParameters><Options><Field name="StartDate">2026-01-15</Field></Options></ReportParameters>', ReportFormat::Xml, Out);
    end;
}

codeunit 50264 "PX Log"
{
    SingleInstance = true;
    var
        Entries: Text;
    procedure Add(Entry: Text) begin Entries += Entry; end;
    procedure Take(): Text var R: Text; begin R := Entries; Entries := ''; exit(R); end;
}

report 50265 "PX Two Options"
{
    UsageCategory = None;

    dataset
    {
        dataitem(R; "PX Row")
        {
            column(Num; Number) { }
        }
    }

    requestpage
    {
        layout
        {
            area(Content)
            {
                field(StartCtl; StartDate) { ApplicationArea = All; }
                field(NoteCtl; NoteText) { ApplicationArea = All; }
            }
        }
    }

    trigger OnInitReport()
    begin
        NoteText := 'init';
    end;

    trigger OnPreReport()
    begin
        L.Add(Format(StartDate, 0, 9) + '|' + NoteText);
    end;

    var
        StartDate: Date;
        NoteText: Text[30];
        L: Codeunit "PX Log";
}

report 50268 "PX Exec Options"
{
    UsageCategory = None;
    ProcessingOnly = true;

    requestpage
    {
        layout
        {
            area(Content)
            {
                field(StartCtl; StartDate) { ApplicationArea = All; }
                field(NoteCtl; NoteText) { ApplicationArea = All; }
            }
        }
    }

    trigger OnInitReport()
    begin
        NoteText := 'init';
    end;

    trigger OnPreReport()
    begin
        L.Add(Format(StartDate, 0, 9) + '|' + NoteText);
    end;

    var
        StartDate: Date;
        NoteText: Text[30];
        L: Codeunit "PX Log";
}

codeunit 50266 "PX Shapes"
{
    var
        L: Codeunit "PX Log";

    local procedure SaveAs(Parameters: Text): Text
    var
        B: Record "PX Blob";
        Out: OutStream;
    begin
        B.Data.CreateOutStream(Out);
        Report.SaveAs(Report::"PX Two Options", Parameters, ReportFormat::Xml, Out);
        exit(L.Take());
    end;

    procedure SelfClosing(): Text
    begin
        exit(SaveAs('<?xml version="1.0" standalone="yes"?><ReportParameters><Options><Field name="StartDate" /><Field name="NoteText" /></Options></ReportParameters>'));
    end;

    procedure Unknown(): Text
    begin
        exit(SaveAs('<?xml version="1.0" standalone="yes"?><ReportParameters><Options><Field name="StartDate">2026-01-15</Field><Field name="NoSuchOption">x</Field></Options></ReportParameters>'));
    end;

    procedure NoDeclaration(): Text
    begin
        exit(SaveAs('<ReportParameters><Options><Field name="StartDate">2026-01-15</Field></Options></ReportParameters>'));
    end;

    procedure UnknownDataItem(): Text
    begin
        exit(SaveAs('<?xml version="1.0" standalone="yes"?><ReportParameters><Options><Field name="StartDate">2026-01-15</Field></Options><DataItems><DataItem name="NoSuchItem">VERSION(1) SORTING(Field1)</DataItem></DataItems></ReportParameters>'));
    end;

    procedure Execute(): Text
    begin
        Report.Execute(Report::"PX Exec Options", '<?xml version="1.0" standalone="yes"?><ReportParameters><Options><Field name="StartDate">2026-01-15</Field></Options></ReportParameters>');
        exit(L.Take());
    end;
}
"#;

fn run(object: &str, routine: &str) -> (Result<Value, Signal>, Vec<Assumption>) {
    let mut b = Built::from_source(SRC);
    clear_hits();
    let r = b.run_in(object, routine);
    (r, take_hits())
}

fn text(object: &str, routine: &str) -> (String, Vec<Assumption>) {
    let (r, hits) = run(object, routine);
    (al_testkit::text(r), hits)
}

#[test]
fn the_reporters_repro_applies_the_field_of_a_root_without_name_and_id() {
    let (r, hits) = run("PX Param Tests 2", "NoNameNoId");
    assert!(r.is_ok(), "{r:?}");
    assert!(hits.is_empty(), "{hits:?}");
}

// Y68SelfClosingParams.al:263-311 (named root): a self-closing Field blanks the variable.
#[test]
fn a_self_closing_field_under_a_root_without_name_and_id_blanks_the_option() {
    assert_eq!(text("PX Shapes", "SelfClosing"), ("|".to_string(), vec![]));
}

#[test]
fn an_unknown_field_under_saveas_is_ignored_through_the_assumption() {
    assert_eq!(
        text("PX Shapes", "Unknown"),
        (
            "2026-01-15|init".to_string(),
            vec![Assumption::ReportSaveAsUnknownFieldIgnored]
        )
    );
}

#[test]
fn a_string_without_the_xml_declaration_stays_refused() {
    let (r, _) = run("PX Shapes", "NoDeclaration");
    assert!(matches!(&r, Err(Signal::Unsupported(_))), "{r:?}");
}

// Only an Execute skips an unknown data item name (ExecuteParamsTwiceAndViews.al:217).
#[test]
fn an_unknown_data_item_under_saveas_stays_refused() {
    let (r, _) = run("PX Shapes", "UnknownDataItem");
    assert!(
        matches!(&r, Err(Signal::Unsupported(m)) if m.contains("NoSuchItem")),
        "{r:?}"
    );
}

#[test]
fn execute_applies_a_root_without_name_and_id() {
    assert_eq!(text("PX Shapes", "Execute"), ("2026-01-15|init".to_string(), vec![]));
}
