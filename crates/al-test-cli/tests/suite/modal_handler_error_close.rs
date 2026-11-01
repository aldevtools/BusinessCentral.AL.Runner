//! A ModalPageHandler that raised an error left a modal page with an open part: the platform asks the part (None), then the
//! page (its own action); when both allow the close, the page closes and RunModal raises the handler's error
//! (pages/lt-wb-6291/WbModalHandlerCloseTests.al:9-39, al-runner issue 6291). Outside lookup mode it is not pinned.

use al_testkit::outcomes_in;

const SRC: &str = r#"
table 50100 "MW Row" { fields { field(1; "Entry No."; Integer) { } } keys { key(PK; "Entry No.") { } } }
codeunit 50101 "MW Trace"
{
    SingleInstance = true;
    var Trace: Text; AllowClose: Boolean;
    procedure Reset(Allow: Boolean) begin Trace := ''; AllowClose := Allow; end;
    procedure Add(Source: Text; CloseAction: Action) begin if Trace <> '' then Trace += '|'; Trace += Source + ':' + Format(CloseAction, 0, 9); end;
    procedure GetTrace(): Text begin exit(Trace); end;
    procedure IsCloseAllowed(): Boolean begin exit(AllowClose); end;
}
page 50102 "MW Part"
{
    PageType = ListPart; SourceTable = "MW Row"; SourceTableTemporary = true;
    layout { area(Content) { repeater(Lines) { field("Entry No."; Rec."Entry No.") { } } } }
    trigger OnOpenPage() begin Rec."Entry No." := 1; Rec.Insert(); end;
    trigger OnQueryClosePage(CloseAction: Action): Boolean var T: Codeunit "MW Trace"; begin T.Add('Part', CloseAction); exit(T.IsCloseAllowed()); end;
}
page 50103 "MW Worksheet"
{
    PageType = Worksheet; UsageCategory = None;
    layout { area(Content) { part(Lines; "MW Part") { } } }
    actions { area(Processing) { action(RaisePageError) { trigger OnAction() begin Error('Page failure'); end; } } }
    trigger OnQueryClosePage(CloseAction: Action): Boolean var T: Codeunit "MW Trace"; begin T.Add('Page', CloseAction); exit(T.IsCloseAllowed()); end;
}
codeunit 50104 T
{
    Subtype = Test;
    [Test] [HandlerFunctions('LeaveOpenHandler')]
    procedure HandlerError()
    var T: Codeunit "MW Trace";
    begin
        T.Reset(true);
        asserterror RunModalWorksheet(true);
        Check('Handler failure', T.GetTrace());
    end;
    [Test] [HandlerFunctions('PageErrorHandler')]
    procedure PageActionError()
    var T: Codeunit "MW Trace";
    begin
        T.Reset(true);
        asserterror RunModalWorksheet(true);
        Check('Page failure', T.GetTrace());
    end;
    [Test] [HandlerFunctions('LeaveOpenHandler')]
    procedure NotLookupModeIsRefused()
    var T: Codeunit "MW Trace";
    begin
        T.Reset(true);
        asserterror RunModalWorksheet(false);
    end;
    local procedure Check(Expected: Text; Trace: Text)
    begin
        if GetLastErrorText() <> Expected then Error('error %1', GetLastErrorText());
        if Trace <> 'Part:None|Page:LookupCancel' then Error('trace %1', Trace);
    end;
    local procedure RunModalWorksheet(Lookup: Boolean): Action
    var W: Page "MW Worksheet";
    begin
        W.LookupMode(Lookup);
        exit(W.RunModal());
    end;
    [ModalPageHandler] procedure LeaveOpenHandler(var W: TestPage "MW Worksheet") begin Error('Handler failure'); end;
    [ModalPageHandler] procedure PageErrorHandler(var W: TestPage "MW Worksheet") begin W.RaisePageError.Invoke(); end;
}
"#;

#[test]
fn a_handler_error_closes_a_modal_page_with_a_part_whose_close_is_allowed() {
    let o = outcomes_in(SRC, "T");
    let get = |n: &str| format!("{:?}", o.iter().find(|(name, _)| name == n).map(|(_, x)| x));
    assert_eq!(get("HandlerError"), "Some(Pass)");
    assert_eq!(get("PageActionError"), "Some(Pass)");
    let g = get("NotLookupModeIsRefused");
    assert!(g.contains("NotRun") && g.contains("whose close is allowed"), "{g}");
}
