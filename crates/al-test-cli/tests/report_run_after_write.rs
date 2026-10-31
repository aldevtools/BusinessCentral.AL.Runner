//! `Report.Run(Id, true)` after an uncommitted insert in the test raises `DB:Ok` "An error occurred and the transaction is
//! stopped. ..." before the request page opens: the handler is not called. Without a write before it the page opens and the
//! handler runs (corpus report/lt-y36-bare-report-member/Y36BareReportMember.al:251-264, the comment of
//! Y36_BareGetter_InsideRun_RequestWindow, measured by writer Y36 on the CI containers of BC 27.5, 28.5 and 29.0).

use al_testkit::outcomes_in;
use al_testrun::Outcome;

const SRC: &str = r#"
table 69995 "RW Row"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}
report 69996 "RW Report"
{
    ProcessingOnly = true;
    dataset { dataitem(Row; "RW Row") { } }
    requestpage
    {
        layout { area(Content) { field(FlagCtl; Flag) { ApplicationArea = All; } } }
    }
    var
        Flag: Boolean;
}
codeunit 69997 "RW Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;
    var
        HandlerCalls: Integer;

    // No handler: the page never opens (with the handler declared the test fails as BC's does: "The following UI handlers
    // were not executed").
    [Test]
    procedure AfterInsert()
    var
        R: Record "RW Row";
    begin
        R.Id := 1;
        R.Insert();
        asserterror Report.Run(Report::"RW Report", true);
        if GetLastErrorCode() <> 'DB:Ok' then
            Error('code %1', GetLastErrorCode());
        if GetLastErrorText() <> 'An error occurred and the transaction is stopped. Contact your administrator or partner for further assistance.' then
            Error('text %1', GetLastErrorText());
        if HandlerCalls <> 0 then
            Error('handler ran %1', HandlerCalls);
    end;

    [Test]
    [HandlerFunctions('RP')]
    procedure WithoutWrite()
    begin
        Report.Run(Report::"RW Report", true);
        if HandlerCalls <> 1 then
            Error('handler ran %1', HandlerCalls);
    end;

    [RequestPageHandler]
    procedure RP(var Page: TestRequestPage "RW Report")
    begin
        HandlerCalls += 1;
        Page.OK().Invoke();
    end;
}
"#;

#[test]
fn a_report_run_with_its_request_page_after_an_uncommitted_insert_raises_before_the_page() {
    let o = outcomes_in(SRC, "RW Tests");
    for t in ["AfterInsert", "WithoutWrite"] {
        let got = o.iter().find(|(n, _)| n == t).map(|(_, o)| o.clone());
        assert_eq!(got, Some(Outcome::Pass), "{t}: {o:?}");
    }
}
