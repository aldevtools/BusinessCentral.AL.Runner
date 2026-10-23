//! al-runner issue 5692: a modal page whose OnQueryClosePage returns false. Mirrors
//! pages/lt-modal-query-close-veto/PageModalQueryCloseVeto.al (codeunit 70867).

use al_test_cli::report::Outcome;

const OBJECTS: &str = r#"
page 61400 "MV Modal"
{
    PageType = Card;
    layout { area(Content) { field(Marker; MarkerVar) { } } }
    var Probe: Codeunit "MV Probe"; MarkerVar: Text[10];
    trigger OnOpenPage() begin MarkerVar := 'OPENED'; end;
    trigger OnQueryClosePage(CloseAction: Action): Boolean begin exit(Probe.Answer(CloseAction)); end;
    trigger OnClosePage() begin Probe.NoteClosePage(); end;
}
codeunit 61401 "MV Probe"
{
    SingleInstance = true;
    var Mode: Integer; QueryCount: Integer; CloseCount: Integer; Seen: Text;
    procedure Reset(NewMode: Integer) begin Mode := NewMode; QueryCount := 0; CloseCount := 0; Seen := ''; end;
    procedure Answer(CloseAction: Action): Boolean
    begin
        QueryCount += 1;
        Seen += Format(CloseAction) + ';';
        if Mode = 1 then exit(false);
        exit(true);
    end;
    procedure NoteClosePage() begin CloseCount += 1; end;
    procedure Summary(): Text begin exit('query=' + Format(QueryCount) + ' close=' + Format(CloseCount) + ' seen=' + Seen); end;
}
"#;

const HELPERS: &str = "var Probe: Codeunit \"MV Probe\"; LookupFlag: Boolean;
[TryFunction] local procedure TryRunModal(var Result: Text) var Modal: Page \"MV Modal\"; begin
    Modal.LookupMode(LookupFlag); Result := Format(Modal.RunModal()); end;
local procedure Run(): Text var Result: Text; Ok: Boolean; begin
    ClearLastError(); Ok := TryRunModal(Result);
    exit(Format(Ok) + '|' + Result + '|' + GetLastErrorText() + '|' + GetLastErrorCode() + '|' + Probe.Summary()); end;
local procedure Check(Expected: Text; Actual: Text) begin if Expected <> Actual then Error('expected <%1> got <%2>', Expected, Actual); end;
[ModalPageHandler] procedure OkHandler(var Modal: TestPage \"MV Modal\") begin Modal.OK().Invoke(); end;
[ModalPageHandler] procedure CancelHandler(var Modal: TestPage \"MV Modal\") begin Modal.Cancel().Invoke(); end;
[ModalPageHandler] procedure NoInvokeHandler(var Modal: TestPage \"MV Modal\") begin end;
[ModalPageHandler] procedure CloseHandler(var Modal: TestPage \"MV Modal\") begin Modal.Close(); end;
[ModalPageHandler] procedure OkThenCloseHandler(var Modal: TestPage \"MV Modal\") begin
    Modal.OK().Invoke(); if Modal.Marker.Value() <> 'OPENED' then Error('marker'); Modal.Close(); end;";

fn outcomes(tests: &str) -> Vec<(String, Outcome)> {
    let src = format!("{OBJECTS}\ncodeunit 61410 T {{ Subtype = Test; {HELPERS} {tests} }}");
    al_testkit::outcomes_in(&src, "T")
}

fn passes(tests: &str) {
    let o = outcomes(tests);
    assert!(!o.is_empty());
    for (n, r) in &o {
        assert_eq!(r, &Outcome::Pass, "{n}");
    }
}

const VETOED: &str = "The page cannot be closed at this time.|FormCloseNotAllowed";

// PageModalQueryCloseVeto.al:275 Modal_HandlerDoesNothing_VetoAlways, :129 Modal_HandlerOk_VetoAlways,
// :148 Modal_HandlerCancel_VetoAlways, :255 Modal_Lookup_HandlerLookupOk_VetoAlways, :309 Modal_AfterVetoedRun_NextRunIsClean.
#[test]
fn a_vetoed_close_of_a_modal_page_answers_none_and_leaves_the_last_error() {
    passes(&format!(
        "[Test] [HandlerFunctions('NoInvokeHandler')] procedure NoInvoke() begin
            Probe.Reset(1); Check('Yes|None|{VETOED}|query=1 close=0 seen=OK;', Run()); end;
        [Test] [HandlerFunctions('OkHandler')] procedure OkTwice() begin
            Probe.Reset(1); Check('Yes|None|{VETOED}|query=2 close=0 seen=OK;OK;', Run());
            Probe.Reset(0); Check('Yes|OK|||query=1 close=1 seen=OK;', Run()); end;
        [Test] [HandlerFunctions('CancelHandler')] procedure LookupCancel() begin
            Probe.Reset(1); LookupFlag := true;
            Check('Yes|None|{VETOED}|query=2 close=0 seen=LookupCancel;LookupCancel;', Run()); end;
        [Test] [HandlerFunctions('OkHandler')] procedure LookupOk() begin
            Probe.Reset(1); LookupFlag := true;
            Check('Yes|None|{VETOED}|query=2 close=0 seen=LookupOK;LookupCancel;', Run()); end;"
    ));
}

// PageModalQueryCloseVeto.al:167 Modal_HandlerClose_VetoAlways, :236 Modal_HandlerOkThenClose_VetoAlways: a TestPage.Close()
// the page refuses closes the TestPage all the same, and RunModal then raises.
#[test]
fn a_vetoed_test_page_close_in_the_handler_makes_run_modal_raise() {
    let closed = "The RunModal procedure could not close the page 61400 as it has already been closed.|TestCodeUnit";
    passes(&format!(
        "[Test] [HandlerFunctions('CloseHandler')] procedure CloseOnce() begin
            Probe.Reset(1); Check('No||{closed}|query=1 close=0 seen=OK;', Run()); end;
        [Test] [HandlerFunctions('OkThenCloseHandler')] procedure OkThenClose() begin
            Probe.Reset(1); Check('No||{closed}|query=2 close=0 seen=OK;OK;', Run()); end;"
    ));
}
