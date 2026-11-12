//! `UseRequestPage()` read in a trigger answers from the run only for the runs and triggers reports/bare-report-member/
//! UseRequestPageReadInTriggers.al:244-475 pins: the report's own run, data item and request page OnOpenPage triggers of
//! the run that is running now. Everywhere else it keeps the answer it had before that pin (what the next Run does): the
//! request page's OnInit and OnClosePage, also when the run starts inside another report's trigger or a page's OnOpenPage,
//! a reportextension's triggers, and a Print run as an Execute (review of runner PR 7815; none of these is pinned).

use al_testkit::{run_in, text};

const SRC: &str = r#"
table 69878 "UO Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
codeunit 69870 "UO Log"
{
    SingleInstance = true;
    var
        Entries: Text;
    procedure Add(Entry: Text) begin Entries += Entry + ';'; end;
    procedure Take(): Text var R: Text; begin R := Entries; Entries := ''; exit(R); end;
}
report 69871 "UO Target"
{
    ProcessingOnly = true;
    dataset { dataitem(One; "UO Row") { } }
    requestpage
    {
        layout { area(Content) { field(OptCtl; Opt) { ApplicationArea = All; } } }
        trigger OnInit() begin L.Add('rp init ' + Format(CurrReport.UseRequestPage())); end;
        trigger OnClosePage() begin L.Add('rp close ' + Format(CurrReport.UseRequestPage())); end;
    }
    trigger OnPreReport() begin L.Add('pre ' + Format(CurrReport.UseRequestPage())); end;
    var
        Opt: Integer;
        L: Codeunit "UO Log";
}
report 69872 "UO Outer"
{
    ProcessingOnly = true;
    UseRequestPage = false;
    dataset { dataitem(One; "UO Row") { } }
    trigger OnPreReport()
    var
        C: Codeunit "UO Caller";
    begin
        Report.Execute(Report::"UO Target", C.Params());
    end;
}
page 69873 "UO Page"
{
    PageType = Card;
    layout { area(Content) { field(OptCtl; Opt) { ApplicationArea = All; } } }
    trigger OnOpenPage()
    var
        C: Codeunit "UO Caller";
    begin
        Report.Execute(Report::"UO Target", C.Params());
    end;
    var
        Opt: Integer;
}
report 69874 "UO Ext Base"
{
    ProcessingOnly = true;
    dataset { dataitem(One; "UO Row") { } }
    requestpage { layout { area(Content) { field(OptCtl; Opt) { ApplicationArea = All; } } } }
    trigger OnPreReport() begin L.Add('base pre ' + Format(CurrReport.UseRequestPage())); end;
    var
        Opt: Integer;
        L: Codeunit "UO Log";
}
reportextension 69875 "UO Ext" extends "UO Ext Base"
{
    dataset
    {
        modify(One)
        {
            trigger OnAfterAfterGetRecord() begin L.Add('ext row ' + Format(CurrReport.UseRequestPage())); end;
        }
    }
    trigger OnPreReport() begin L.Add('ext pre ' + Format(CurrReport.UseRequestPage())); end;
    trigger OnPostReport() begin L.Add('ext post ' + Format(CurrReport.UseRequestPage())); end;
    var
        L: Codeunit "UO Log";
}
report 69876 "UO Print"
{
    UseSystemPrinter = true;
    ProcessingOnly = true;
    dataset
    {
        dataitem(One; "UO Row")
        {
            trigger OnAfterGetRecord() begin L.Add('row ' + Format(CurrReport.UseRequestPage())); end;
        }
    }
    requestpage { layout { area(Content) { field(OptCtl; Opt) { ApplicationArea = All; } } } }
    trigger OnPreReport() begin L.Add('pre ' + Format(CurrReport.UseRequestPage())); end;
    trigger OnPostReport() begin L.Add('post ' + Format(CurrReport.UseRequestPage())); end;
    var
        Opt: Integer;
        L: Codeunit "UO Log";
}
codeunit 69877 "UO Caller"
{
    var
        L: Codeunit "UO Log";

    procedure Params(): Text
    begin
        exit('<?xml version="1.0" standalone="yes"?><ReportParameters name="UO Target" id="69871"><Options><Field name="Opt">2</Field></Options></ReportParameters>');
    end;

    local procedure Seed()
    var
        Row: Record "UO Row";
    begin
        Row.DeleteAll();
        Row.Id := 1;
        Row.Insert();
        L.Take();
    end;

    procedure TopLevel(): Text
    begin
        Seed();
        Report.Execute(Report::"UO Target", Params());
        exit(L.Take());
    end;

    procedure FromReportTrigger(): Text
    begin
        Seed();
        Report.Execute(Report::"UO Outer", '');
        exit(L.Take());
    end;

    procedure FromPageOpen(): Text
    var
        P: TestPage "UO Page";
    begin
        Seed();
        P.OpenView();
        P.Close();
        exit(L.Take());
    end;

    procedure Extension(): Text
    begin
        Seed();
        Report.Execute(Report::"UO Ext Base", '');
        exit(L.Take());
    end;

    procedure Print(): Text
    begin
        Seed();
        Report.Print(Report::"UO Print", '');
        exit(L.Take());
    end;
}
"#;

/// The request page's OnInit and OnClosePage of an Execute with parameters keep their answer, wherever the Execute starts.
#[test]
fn request_page_init_and_close_answer_the_same_wherever_the_execute_starts() {
    let top = text(run_in(SRC, "UO Caller", "TopLevel"));
    assert_eq!(top, "rp init Yes;rp close Yes;pre No;");
    assert_eq!(text(run_in(SRC, "UO Caller", "FromReportTrigger")), top);
    assert_eq!(text(run_in(SRC, "UO Caller", "FromPageOpen")), top);
}

/// A reportextension's triggers keep the answer they had before the pin, all three alike; the base report's OnPreReport
/// answers from the run (UseRequestPageReadInTriggers.al:396-410, Execute with empty parameters).
#[test]
fn reportextension_triggers_answer_alike() {
    assert_eq!(
        text(run_in(SRC, "UO Caller", "Extension")),
        "base pre No;ext pre Yes;ext row Yes;ext post Yes;"
    );
}

/// A Print run as an Execute is not pinned: its triggers keep the instance's answer.
#[test]
fn a_print_run_as_execute_keeps_the_instance_answer() {
    assert_eq!(text(run_in(SRC, "UO Caller", "Print")), "pre Yes;row Yes;post Yes;");
}

/// The bare read is pinned in the report's own triggers, its data item triggers and its request page's OnOpenPage; a request
/// page trigger with the name of a data item trigger (OnAfterGetRecord) is none of them and stays refused.
#[test]
fn a_bare_read_in_a_request_page_on_after_get_record_is_refused() {
    let src = r#"
table 69879 "UO Row2" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
report 69880 "UO RpGet"
{
    ProcessingOnly = true;
    dataset { dataitem(One; "UO Row2") { } }
    requestpage
    {
        layout { area(Content) { field(OptCtl; Opt) { ApplicationArea = All; } } }
        trigger OnAfterGetRecord() begin if UseRequestPage() then Opt := 1; end;
    }
    var
        Opt: Integer;
}
"#;
    let reasons = al_testkit::blocked(&al_testkit::compile(src));
    assert!(
        reasons
            .iter()
            .flatten()
            .flatten()
            .any(|why| why.contains("a bare `UseRequestPage`")),
        "{reasons:?}"
    );
}
