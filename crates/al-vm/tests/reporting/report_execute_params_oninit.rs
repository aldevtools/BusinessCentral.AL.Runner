//! `Report.Execute(Parameters)` on a report whose request page has an OnInit: OnInitReport, the request page's OnInit, then the
//! parameter values, then the page's OnOpenPage and OnClosePage (no OnQueryClosePage), then OnPreReport. A control the
//! parameters do not name keeps the value OnInit gave it; with an empty string no page trigger after OnInit runs
//! (report/lt-report-requestpage/Y12ExecuteOnInit.al:117-156, corpus branch al-runtime-rs-staging-y12, al-runner issue 6446;
//! RptRequestPage.al:878-893 RunRequestPage_ThenExecute_AppliesValues pins the order for parameters that name every control).

const SRC: &str = r#"
table 91554 "RPI Row" { fields { field(1; No; Integer) { } } keys { key(PK; No) { } } }
report 91555 "RPI Report"
{
    ProcessingOnly = true;
    dataset
    {
        dataitem(R; "RPI Row")
        {
            trigger OnPreDataItem() begin Log += 'OnPreDataItem(' + R.GetFilters() + ');'; end;
            trigger OnAfterGetRecord() begin Log += 'row ' + Format(R.No) + ';'; end;
        }
    }
    requestpage
    {
        layout { area(Content) { field(ValControl; Val) { } field(OtherControl; Other) { } } }
        trigger OnInit()
        begin
            SeenInInit := Val;
            Val := 'INIT';
            Other := 'INIT';
            Log += 'OnInit;';
        end;
        trigger OnOpenPage() begin Log += 'OnOpenPage(' + Val + '|' + Other + ');'; end;
        trigger OnQueryClosePage(CloseAction: Action): Boolean begin Log += 'OnQueryClosePage;'; end;
        trigger OnClosePage() begin Log += 'OnClosePage(' + Val + ');'; end;
    }
    trigger OnInitReport() begin Log += 'OnInitReport(' + Val + ');'; end;
    trigger OnPreReport()
    begin
        Log += 'OnPreReport;';
        AtPre := Val;
        OtherAtPre := Other;
    end;
    var
        Val, Other, SeenInInit, AtPre, OtherAtPre : Text[30];
        Log: Text;
    procedure Result(): Text
    begin
        exit(StrSubstNo('seenInInit=[%1] atPre=[%2] otherAtPre=[%3] log=%4', SeenInInit, AtPre, OtherAtPre, Log));
    end;
}
codeunit 91556 C
{
    local procedure Seed()
    var
        Row: Record "RPI Row";
    begin
        Row.DeleteAll();
        Row.No := 1;
        Row.Insert();
        Row.No := 2;
        Row.Insert();
    end;
    local procedure Params(): Text
    begin
        exit('<?xml version="1.0" standalone="yes"?><ReportParameters name="RPI Report" id="91555"><Options><Field name="Val">PARAM</Field></Options></ReportParameters>');
    end;
    procedure Plain(): Text
    var
        A: Report "RPI Report";
    begin
        Seed();
        A.Execute(Params());
        exit(A.Result());
    end;
    procedure WithView(): Text
    var
        Row: Record "RPI Row";
        B: Report "RPI Report";
    begin
        Seed();
        Row.SetRange(No, 2);
        B.SetTableView(Row);
        B.Execute(Params());
        exit(B.Result());
    end;
    procedure Empty(): Text
    var
        C: Report "RPI Report";
    begin
        Seed();
        C.Execute('');
        exit(C.Result());
    end;
}
"#;

fn run(routine: &str) -> String {
    match al_testkit::run_in(SRC, "C", routine) {
        Ok(v) => al_testkit::text(Ok(v)),
        Err(e) => panic!("{routine}: {e:?}"),
    }
}

#[test]
fn execute_parameters_land_after_the_request_page_oninit() {
    assert_eq!(
        run("Plain"),
        "seenInInit=[] atPre=[PARAM] otherAtPre=[INIT] log=OnInitReport();OnInit;OnOpenPage(PARAM|INIT);OnClosePage(PARAM);OnPreReport;OnPreDataItem();row 1;row 2;"
    );
    assert_eq!(
        run("WithView"),
        "seenInInit=[] atPre=[PARAM] otherAtPre=[INIT] log=OnInitReport();OnInit;OnOpenPage(PARAM|INIT);OnClosePage(PARAM);OnPreReport;OnPreDataItem(No: 2);row 2;"
    );
    assert_eq!(
        run("Empty"),
        "seenInInit=[] atPre=[INIT] otherAtPre=[INIT] log=OnInitReport();OnInit;OnPreReport;OnPreDataItem();row 1;row 2;"
    );
}
