//! CurrReport.Quit in OnInitReport of a processing-only report with a request page, then each way to start the report
//! (al-runner issue 7964 item 3; corpus PR 1382, reports/runtime/ReportQuitInOnInit.al:235-355, not pinned yet, assumption
//! `report-quit-in-oninit-request-page`). The probe's objects; its test codeunit's RunForm is the driver's `Form`, which
//! answers `Yes|result|log`, or `No|error text`.

use al_builtins::assumptions::{Assumption, take_hits};
use al_builtins::value::Value;

const SRC: &str = r#"
table 2000000026 Integer { fields { field(1; Number; Integer) { } } keys { key(PK; Number) { } } }

table 71901 "LT RqI Blob"
{
    DataClassification = SystemMetadata;
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

report 71900 "LT RqI Quit Proc"
{
    ProcessingOnly = true;
    UsageCategory = None;

    dataset
    {
        dataitem(I; Integer)
        {
            DataItemTableView = where(Number = const(1));

            trigger OnPreDataItem()
            begin
                Log.Add('OnPreDataItem');
            end;

            trigger OnAfterGetRecord()
            begin
                Log.Add('OnAfterGetRecord');
            end;
        }
    }

    requestpage
    {
        layout
        {
            area(Content)
            {
                field(OptionField; OptionValue)
                {
                    ApplicationArea = All;
                    Caption = 'Option value';
                }
            }
        }

        trigger OnOpenPage()
        begin
            Log.Add('RequestPage.OnOpenPage');
        end;

        trigger OnQueryClosePage(CloseAction: Action): Boolean
        begin
            Log.Add('RequestPage.OnQueryClosePage');
            exit(true);
        end;
    }

    trigger OnInitReport()
    begin
        Log.Add('OnInitReport');
        CurrReport.Quit();
        Log.Add('after Quit');
    end;

    trigger OnPreReport()
    begin
        Log.Add('OnPreReport');
    end;

    trigger OnPostReport()
    begin
        Log.Add('OnPostReport');
    end;

    var
        Log: Codeunit "LT RqI Log";
        OptionValue: Text[30];
}

codeunit 71899 "LT RqI Log"
{
    SingleInstance = true;

    var
        Entries: Text;

    procedure Reset()
    begin
        Entries := '';
    end;

    procedure Add(Entry: Text)
    begin
        Entries += Entry + ';';
    end;

    procedure Get(): Text
    begin
        exit(Entries);
    end;
}

codeunit 71902 "LT RqI Runner"
{
    var
        Mode: Integer;
        Result: Text;

    procedure SetMode(NewMode: Integer)
    begin
        Mode := NewMode;
        Result := '';
    end;

    procedure GetResult(): Text
    begin
        exit(Result);
    end;

    trigger OnRun()
    var
        Rep: Report "LT RqI Quit Proc";
        Buf: Record "LT RqI Blob";
        OutS: OutStream;
        Ok: Boolean;
    begin
        case Mode of
            1:
                Result := Rep.RunRequestPage();
            2:
                Result := Report.RunRequestPage(Report::"LT RqI Quit Proc");
            3:
                begin
                    Rep.UseRequestPage(false);
                    Rep.Run();
                end;
            4:
                Rep.Run();
            5:
                Report.Run(Report::"LT RqI Quit Proc");
            6:
                Report.Run(Report::"LT RqI Quit Proc", false);
            7:
                begin
                    Rep.UseRequestPage(false);
                    Rep.RunModal();
                end;
            8:
                Rep.RunModal();
            9:
                begin
                    Buf.Id := 1;
                    Buf.Insert();
                    Buf.Data.CreateOutStream(OutS);
                    Ok := Rep.SaveAs('', ReportFormat::Xml, OutS);
                    Buf.Modify();
                    Buf.CalcFields(Data);
                    Result := 'ok=' + Format(Ok) + ' length=' + Format(Buf.Data.Length());
                end;
            10:
                begin
                    Buf.Id := 1;
                    Buf.Insert();
                    Buf.Data.CreateOutStream(OutS);
                    Ok := Report.SaveAs(Report::"LT RqI Quit Proc", '', ReportFormat::Xml, OutS);
                    Buf.Modify();
                    Buf.CalcFields(Data);
                    Result := 'ok=' + Format(Ok) + ' length=' + Format(Buf.Data.Length());
                end;
            11:
                Report.Execute(Report::"LT RqI Quit Proc", '');
            12:
                begin
                    Rep.UseRequestPage(false);
                    Rep.Run();
                    Rep.Run();
                end;
            13:
                Result := Report.RunRequestPage(Report::"LT RqI Quit Proc", '');
        end;
    end;
}

codeunit 71904 "LT RqI Driver"
{
    procedure Form(Mode: Integer): Text
    var
        Log: Codeunit "LT RqI Log";
        Runner: Codeunit "LT RqI Runner";
        Ok: Boolean;
    begin
        Log.Reset();
        ClearLastError();
        Runner.SetMode(Mode);
        Ok := Runner.Run();
        if not Ok then
            exit('No|' + GetLastErrorText());
        exit('Yes|' + Runner.GetResult() + '|' + Log.Get());
    end;
}
"#;

fn form(b: &mut al_testkit::Built, mode: i32) -> String {
    let (oi, ri) = b.routine_in("LT RqI Driver", "Form");
    match b.call(oi, ri, vec![Value::Integer(mode)]) {
        Ok((Value::Text(t), _)) => t.to_string(),
        other => format!("{other:?}"),
    }
}

/// The same objects with the report's `ProcessingOnly` taken out (a report that is not processing-only: not measured).
fn not_processing_only() -> String {
    SRC.replace("ProcessingOnly = true;", "")
}

/// The probe's runner with three more forms: RunRequestPage after `UseRequestPage(false)` (14), SaveAs with the answer dropped
/// (15), UseRequestPage(false) then SaveAs (16), and UseRequestPage(false), Clear, Run (17).
fn with_neighbor_forms(src: &str) -> String {
    src.replace(
        "            13:\n",
        "            14:\n                begin\n                    Rep.UseRequestPage(false);\n                    Result := Rep.RunRequestPage();\n                end;\n            15:\n                begin\n                    Buf.Id := 1;\n                    Buf.Insert();\n                    Buf.Data.CreateOutStream(OutS);\n                    Rep.SaveAs('', ReportFormat::Xml, OutS);\n                end;\n            16:\n                begin\n                    Rep.UseRequestPage(false);\n                    Buf.Id := 1;\n                    Buf.Insert();\n                    Buf.Data.CreateOutStream(OutS);\n                    Ok := Rep.SaveAs('', ReportFormat::Xml, OutS);\n                end;\n            17:\n                begin\n                    Rep.UseRequestPage(false);\n                    Clear(Rep);\n                    Rep.Run();\n                end;\n            13:\n",
    )
}

#[test]
fn the_forms_that_refused_now_run_only_oninitreport_flagged() {
    let mut b = al_testkit::Built::from_source(SRC);
    // :235 variable RunRequestPage, :245 static, :255 static with parameters (mode 13), :265 UseRequestPage(false) + Run,
    // :305 UseRequestPage(false) + RunModal, :355 UseRequestPage(false) + Run twice.
    for (mode, expected) in [
        (1, "Yes||OnInitReport;"),
        (2, "Yes||OnInitReport;"),
        (13, "Yes||OnInitReport;"),
        (3, "Yes||OnInitReport;"),
        (7, "Yes||OnInitReport;"),
        (12, "Yes||OnInitReport;OnInitReport;"),
        // :325 variable SaveAs, :335 static SaveAs: false, nothing written.
        (9, "Yes|ok=No length=0|OnInitReport;"),
        (10, "Yes|ok=No length=0|OnInitReport;"),
    ] {
        take_hits();
        assert_eq!(form(&mut b, mode), expected, "mode {mode}");
        assert!(take_hits().contains(&Assumption::ReportQuitInOninitRequestPage), "mode {mode} must mark");
    }
}

#[test]
fn the_forms_main_ran_keep_their_result_unflagged() {
    let mut b = al_testkit::Built::from_source(SRC);
    // :275 variable Run, :285 static Run, :295 static Run without request page, :315 variable RunModal, :345 Execute: the run
    // itself runs OnInitReport and stops there, as on main.
    for mode in [4, 5, 6, 8, 11] {
        take_hits();
        assert_eq!(form(&mut b, mode), "Yes||OnInitReport;", "mode {mode}");
        assert!(!take_hits().contains(&Assumption::ReportQuitInOninitRequestPage), "mode {mode}");
    }
}

#[test]
fn shapes_outside_the_probe_keep_mains_refusal() {
    let refused = |b: &mut al_testkit::Built, mode: i32| {
        let r = form(b, mode);
        assert!(r.contains("Unsupported") && r.contains("CurrReport.Quit"), "mode {mode}: {r}");
    };
    // A report that is not processing-only.
    let mut b = al_testkit::Built::from_source(&not_processing_only());
    for mode in [1, 2, 3, 7] {
        refused(&mut b, mode);
    }
    // Another method after the quit UseRequestPage(false) kept, and a SaveAs whose answer is dropped.
    let mut b = al_testkit::Built::from_source(&with_neighbor_forms(SRC));
    for mode in [14, 15, 16] {
        refused(&mut b, mode);
    }
}

#[test]
fn clear_drops_the_kept_quit_and_the_next_run_is_a_first_run() {
    // Clear gives the variable a new instance (as for Execute, reports/request-page/ExecuteParamsTwiceAndViews.al:146-183),
    // so the quit UseRequestPage(false) kept stays with the old one: the Run runs OnInitReport again, which quits, as mode 4.
    let mut b = al_testkit::Built::from_source(&with_neighbor_forms(SRC));
    take_hits();
    assert_eq!(form(&mut b, 17), "Yes||OnInitReport;OnInitReport;");
    assert!(take_hits().contains(&Assumption::ReportQuitInOninitRequestPage));
}
