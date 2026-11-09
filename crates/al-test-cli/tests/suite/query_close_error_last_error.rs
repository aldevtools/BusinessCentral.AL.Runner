//! An error of OnQueryClosePage at TestPage.Close() with a [MessageHandler]: the handler gets the error text, Close() returns
//! and the error is the last error (GetLastErrorText, code TestField), whether the row is clean, dirty from another field, or
//! dirty from a field whose OnValidate asked a Confirm; without a MessageHandler Close() raises "Unhandled UI: Message <text>"
//! (corpus master text/TestPageQueryCloseErrorMessageHandler_Tests.al; al-runner issue 7457).

use al_testkit::outcomes_in;
use al_testrun::Outcome;

const SRC: &str = r#"
table 69980 "QCM IC"
{
    fields { field(1; Code; Code[10]) { } field(2; Active; Boolean) { } field(3; InboxType; Integer) { } field(4; InboxDetails; Text[30]) { } }
    keys { key(PK; Code) { Clustered = true; } }
    procedure CheckSetup()
    begin
        TestField(InboxDetails);
    end;
}
page 69981 "QCM IC Card"
{
    PageType = Card; SourceTable = "QCM IC"; UsageCategory = None;
    layout
    {
        area(Content)
        {
            field(Code; Rec.Code) { ApplicationArea = All; }
            field(InboxType; Rec.InboxType) { ApplicationArea = All; }
            field(InboxDetails; Rec.InboxDetails) { ApplicationArea = All; }
        }
    }
}
pageextension 69982 "QCM IC Card Ext" extends "QCM IC Card"
{
    layout
    {
        addlast(Content)
        {
            field(MyActive; Rec.Active)
            {
                ApplicationArea = All;
                trigger OnValidate()
                begin
                    if Rec.Active then begin
                        Rec.TestField(InboxType);
                        if not Confirm('confirm?') then
                            Error('not confirmed');
                    end;
                end;
            }
        }
    }
    trigger OnQueryClosePage(CloseAction: Action): Boolean
    begin
        if Rec.Active then
            Rec.CheckSetup();
    end;
}
codeunit 69983 T
{
    Subtype = Test;
    var
        Seen: Text;
    local procedure Seed(ActiveNow: Boolean)
    var Row: Record "QCM IC";
    begin
        Row.DeleteAll();
        Row.Init(); Row.Code := 'GU5'; Row.InboxType := 1; Row.Active := ActiveNow; Row.Insert();
    end;
    local procedure Check()
    var Expected: Text;
    begin
        Expected := 'InboxDetails must have a value in QCM IC: Code=GU5. It cannot be zero or empty.';
        if GetLastErrorText() <> Expected then Error('last error: %1', GetLastErrorText());
        if GetLastErrorCode() <> 'TestField' then Error('last code: %1', GetLastErrorCode());
        if Seen <> Expected then Error('message: %1', Seen);
    end;
    [ConfirmHandler] procedure YesHandler(Question: Text; var Reply: Boolean) begin Reply := true; end;
    [MessageHandler] procedure MessageSink(Msg: Text) begin Seen := Msg; end;

    [Test] [HandlerFunctions('YesHandler,MessageSink')]
    procedure AfterConfirmedActivation()
    var Row: Record "QCM IC"; Card: TestPage "QCM IC Card";
    begin
        Seed(false); Row.Get('GU5');
        Card.OpenEdit(); Card.GoToRecord(Row); Card.MyActive.SetValue(true); Card.Close();
        Check();
    end;
    [Test] [HandlerFunctions('MessageSink')]
    procedure CleanRow()
    var Row: Record "QCM IC"; Card: TestPage "QCM IC Card";
    begin
        Seed(true); Row.Get('GU5');
        Card.OpenEdit(); Card.GoToRecord(Row); Card.Close();
        Check();
    end;
    [Test] [HandlerFunctions('MessageSink')]
    procedure AfterEditingAnotherField()
    var Row: Record "QCM IC"; Card: TestPage "QCM IC Card";
    begin
        Seed(true); Row.Get('GU5');
        Card.OpenEdit(); Card.GoToRecord(Row); Card.InboxType.SetValue(2); Card.Close();
        Check();
    end;
    [Test] [HandlerFunctions('YesHandler')]
    procedure WithoutMessageHandler()
    var Row: Record "QCM IC"; Card: TestPage "QCM IC Card";
    begin
        Seed(false); Row.Get('GU5');
        Card.OpenEdit(); Card.GoToRecord(Row); Card.MyActive.SetValue(true);
        asserterror Card.Close();
        if GetLastErrorText() <> 'Unhandled UI: Message InboxDetails must have a value in QCM IC: Code=GU5. It cannot be zero or empty.' then
            Error('text: %1', GetLastErrorText());
        if GetLastErrorCode() <> 'MissingUIHandler' then Error('code: %1', GetLastErrorCode());
    end;
}
"#;

#[test]
fn an_error_of_on_query_close_page_is_the_last_error_after_a_consumed_close() {
    let out = outcomes_in(SRC, "T");
    let mut got: Vec<(String, Outcome)> = out;
    got.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(
        got,
        vec![
            ("AfterConfirmedActivation".to_string(), Outcome::Pass),
            ("AfterEditingAnotherField".to_string(), Outcome::Pass),
            ("CleanRow".to_string(), Outcome::Pass),
            ("WithoutMessageHandler".to_string(), Outcome::Pass),
        ]
    );
}
