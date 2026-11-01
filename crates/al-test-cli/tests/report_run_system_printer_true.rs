//! `Report.Run` / `Report.RunModal(Id, RequestWindow, SystemPrinter = true, Rec)` runs as with `false`: the request page
//! opens with the same handler, OK runs the report with the page's filters and fields, Cancel runs no trigger, and with no
//! request page the report runs at once (report/lt-report-run-system-printer/Y43SystemPrinterRequestPage.al, corpus PR 966;
//! ReportRunSystemPrinter.al:110-150).

use al_testkit::outcomes_in;
use al_testrun::Outcome;

const SRC: &str = r#"
table 69985 "SPT Row"
{
    fields { field(1; "No."; Code[20]) { } }
    keys { key(PK; "No.") { Clustered = true; } }
}
codeunit 69986 "SPT Log"
{
    SingleInstance = true;
    var L: Text;
    procedure Add(T: Text) begin L += T + ';'; end;
    procedure Take(): Text var T: Text; begin T := L; L := ''; exit(T); end;
}
report 69987 "SPT Report"
{
    ProcessingOnly = true;
    dataset
    {
        dataitem(D; "SPT Row")
        {
            RequestFilterFields = "No.";
            trigger OnPreDataItem() begin Log.Add('pre-di ' + D.GetFilters()); end;
            trigger OnAfterGetRecord() begin Log.Add('row ' + D."No."); end;
        }
    }
    requestpage
    {
        layout { area(Content) { field(FlagCtl; Flag) { ApplicationArea = All; } } }
    }
    trigger OnPreReport() begin Log.Add('pre'); Log.Add('flag ' + Format(Flag)); end;
    trigger OnPostReport() begin Log.Add('post'); end;
    var
        Log: Codeunit "SPT Log";
        Flag: Boolean;
}
codeunit 69988 "SPT Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;
    var
        Log: Codeunit "SPT Log";

    local procedure Seed()
    var
        R: Record "SPT Row";
    begin
        R.DeleteAll();
        R."No." := 'A'; R.Insert();
        R."No." := 'B'; R.Insert();
        R."No." := 'C'; R.Insert();
        Commit();
        Log.Take();
    end;

    local procedure Expect(Want: Text)
    var
        Got: Text;
    begin
        Got := Log.Take();
        if Got <> Want then
            Error('log %1', Got);
    end;

    [Test]
    [HandlerFunctions('RpOk')]
    procedure RunModalOk()
    var
        R: Record "SPT Row";
    begin
        Seed();
        Report.RunModal(Report::"SPT Report", true, true, R);
        Expect('rp;pre;flag Yes;pre-di No.: A|B;row A;row B;post;');
    end;

    [Test]
    [HandlerFunctions('RpOk')]
    procedure RunOk()
    var
        R: Record "SPT Row";
    begin
        Seed();
        Report.Run(Report::"SPT Report", true, true, R);
        Expect('rp;pre;flag Yes;pre-di No.: A|B;row A;row B;post;');
    end;

    [Test]
    [HandlerFunctions('RpCancel')]
    procedure RunModalCancel()
    var
        R: Record "SPT Row";
    begin
        Seed();
        Report.RunModal(Report::"SPT Report", true, true, R);
        Expect('rp;');
    end;

    [Test]
    procedure RunNoPage()
    var
        R: Record "SPT Row";
    begin
        Seed();
        Report.Run(Report::"SPT Report", false, true, R);
        Expect('pre;flag No;pre-di ;row A;row B;row C;post;');
    end;

    [RequestPageHandler]
    procedure RpOk(var Page: TestRequestPage "SPT Report")
    begin
        Log.Add('rp');
        Page.D.SetFilter("No.", 'A|B');
        Page.FlagCtl.SetValue(true);
        Page.OK().Invoke();
    end;

    [RequestPageHandler]
    procedure RpCancel(var Page: TestRequestPage "SPT Report")
    begin
        Log.Add('rp');
        Page.Cancel().Invoke();
    end;
}
"#;

#[test]
fn system_printer_true_runs_as_false() {
    let o = outcomes_in(SRC, "SPT Tests");
    for t in ["RunModalOk", "RunOk", "RunModalCancel", "RunNoPage"] {
        let got = o.iter().find(|(n, _)| n == t).map(|(_, o)| o.clone());
        assert_eq!(got, Some(Outcome::Pass), "{t}: {o:?}");
    }
}
