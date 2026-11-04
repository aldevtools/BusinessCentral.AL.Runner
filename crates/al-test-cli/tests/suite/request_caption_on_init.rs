//! `RequestOptionsPage.Caption := Text` in the request page's OnInit is the caption a request page handler reads; OnOpenPage wins
//! over OnInit; a set in a control's OnValidate changes what the report's code reads back and not what the handler reads
//! (reports/request-page/Y86RequestCaption.al).

use al_testrun::Outcome;

const SRC: &str = r#"
table 61504 "RC Row" { fields { field(1; Number; Integer) { } } keys { key(PK; Number) { } } }
report 61501 "RC OnInit"
{
    Caption = 'RC report caption';
    ProcessingOnly = true;
    dataset { dataitem(Row; "RC Row") { } }
    requestpage
    {
        layout { area(Content) { field(Ctl; Ctl) { ApplicationArea = All; } } }
        trigger OnInit() begin RequestOptionsPage.Caption := 'set in OnInit'; end;
    }
    var Ctl: Integer;
}
report 61502 "RC Both"
{
    ProcessingOnly = true;
    dataset { dataitem(Row; "RC Row") { } }
    requestpage
    {
        layout { area(Content) { field(Ctl; Ctl) { ApplicationArea = All; } } }
        trigger OnInit() begin RequestOptionsPage.Caption := 'set in OnInit'; end;
        trigger OnOpenPage() begin RequestOptionsPage.Caption := 'set in OnOpenPage'; end;
    }
    var Ctl: Integer;
}
codeunit 61503 T
{
    Subtype = Test;
    var Seen: Text;
    local procedure Check(Expected: Text; Actual: Text) begin if Expected <> Actual then Error('expected [%1] got [%2]', Expected, Actual); end;

    [Test] [HandlerFunctions('InitHandler')]
    procedure OnInitCaptionIsTheHandlers() begin Seen := ''; Report.Run(Report::"RC OnInit"); Check('set in OnInit', Seen); end;
    [RequestPageHandler] procedure InitHandler(var RP: TestRequestPage "RC OnInit") begin Seen := RP.Caption(); RP.Cancel().Invoke(); end;

    [Test] [HandlerFunctions('BothHandler')]
    procedure OnOpenPageWinsOverOnInit() begin Seen := ''; Report.Run(Report::"RC Both"); Check('set in OnOpenPage', Seen); end;
    [RequestPageHandler] procedure BothHandler(var RP: TestRequestPage "RC Both") begin Seen := RP.Caption(); RP.Cancel().Invoke(); end;
}
"#;

#[test]
fn a_caption_set_in_on_init_is_what_the_handler_reads() {
    al_builtins::assumptions::clear_hits();
    let o = al_testkit::outcomes_in(SRC, "T");
    assert_eq!(o.len(), 2, "{o:?}");
    for (n, r) in &o {
        assert_eq!(r, &Outcome::Pass, "{n}: {r:?}");
    }
    assert!(al_builtins::assumptions::take_hits().is_empty());
}
