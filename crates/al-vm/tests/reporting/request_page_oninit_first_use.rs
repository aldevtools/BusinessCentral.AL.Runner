//! A report variable whose procedure ran before its run: the request page's OnInit runs at that first use, right after
//! OnInitReport, so the values the procedure set survive the run. Assumed (request-page-oninit-at-first-use) for Microsoft's
//! tests, where Library - Manufacturing RefreshProdOrder calls Refresh Production Order's InitializeRequest(CalcLines = false)
//! and the request page OnInit sets CalcLines := true (Tests-SCM 137402 CapacityCostPostedToGLForTwoItemsInOneProductionOrder);
//! the page variable analog is pinned (handlers/TestPageOnInit_Tests.al:188). A run without a procedure before it keeps
//! OnInitReport, then the request page's OnInit (report/lt-ae-reporthandler/AeReportHandler.al:85 Run_UseRequestPageFalse).

const SRC: &str = r#"
table 69953 "RpInit Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
codeunit 69950 "RpInit Log"
{
    SingleInstance = true;
    var T: Text;
    procedure Add(S: Text) begin T += S + ';'; end;
    procedure Take(): Text var R: Text; begin R := T; T := ''; exit(R); end;
}
report 69951 "RpInit Rpt"
{
    ProcessingOnly = true;
    dataset { dataitem(I; "RpInit Row") { } }
    requestpage
    {
        layout { area(Content) { field(CalcLinesCtl; CalcLines) { } } }
        trigger OnInit()
        begin
            CalcLines := true;
            L.Add('RP OnInit');
        end;
    }
    trigger OnInitReport() begin L.Add('OnInitReport'); end;
    trigger OnPreReport() begin L.Add('OnPreReport ' + Format(CalcLines)); end;
    procedure InitializeRequest(NewCalcLines: Boolean)
    begin
        L.Add('InitializeRequest');
        CalcLines := NewCalcLines;
    end;
    var L: Codeunit "RpInit Log"; CalcLines: Boolean;
}
codeunit 69952 C
{
    procedure WithProcedure(): Text
    var
        R: Report "RpInit Rpt";
        L: Codeunit "RpInit Log";
    begin
        R.InitializeRequest(false);
        R.UseRequestPage(false);
        R.RunModal();
        exit(L.Take());
    end;
    procedure WithoutProcedure(): Text
    var
        R: Report "RpInit Rpt";
        L: Codeunit "RpInit Log";
    begin
        R.UseRequestPage(false);
        R.RunModal();
        exit(L.Take());
    end;
}
"#;

#[test]
fn a_procedure_before_the_run_keeps_its_values_over_the_request_page_oninit() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "WithProcedure")),
        "OnInitReport;RP OnInit;InitializeRequest;OnPreReport No;"
    );
}

#[test]
fn without_a_procedure_the_request_page_oninit_runs_with_the_run() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "WithoutProcedure")),
        "OnInitReport;RP OnInit;OnPreReport Yes;"
    );
}
