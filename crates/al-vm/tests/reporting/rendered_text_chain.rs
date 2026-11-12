//! The shape of an outside reporter's e-mail body chain (al-runner issue 6924), with minimal stand-in AL, not Microsoft's code:
//! a Word layout report saved as Html into a temporary Blob, copied into the e-mail item's body, read whole as text, joined
//! with a signature in a BigText by a subscriber and formatted, stored in a message Blob, and read back from a page field. ADR-040 amendment 2026-11-12, A12: the chain passes (flagged), its `Contains`, comparison and `StrLen`
//! twins are not run.

use al_builtins::assumptions::{self, Assumption};
use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::Built;

const CHAIN: &str = r#"
table 50995 "RC Message"
{
    fields { field(1; Id; Integer) { } field(2; Body; Blob) { } }
    keys { key(PK; Id) { } }
}

// Stand-in for Email Item: SetBody copies the saved Html, GetBodyText reads it whole.
codeunit 50996 "RC Item"
{
    var Body: Record "RC Message" temporary;
    procedure SetBody(var Source: Record "RT Row" temporary)
    var I: InStream; O: OutStream;
    begin
        Body.Id := 1;
        Source.Data.CreateInStream(I);
        Body.Body.CreateOutStream(O);
        CopyStream(O, I);
        Body.Insert();
    end;
    procedure GetBodyText(): Text
    var I: InStream; T: Text;
    begin
        Body.Get(1);
        Body.Body.CreateInStream(I);
        I.Read(T);
        exit(T);
    end;
}

// Stand-in for Email Message: Create raises the subscriber's event, stores the body in a Blob, GetBody reads it back.
codeunit 50997 "RC Message"
{
    SingleInstance = true;
    var Stored: Record "RC Message" temporary;
    procedure Create(Body: Text)
    var O: OutStream; Handled: Boolean;
    begin
        OnBeforeCreate(Body, Handled);
        Stored.DeleteAll();
        Stored.Id := 1;
        Stored.Body.CreateOutStream(O, TextEncoding::UTF8);
        O.WriteText(Body);
        Stored.Insert();
    end;
    procedure GetBody(): Text
    var I: InStream; T: Text;
    begin
        Stored.Get(1);
        Stored.Body.CreateInStream(I, TextEncoding::UTF8);
        I.Read(T);
        exit(T);
    end;
    [IntegrationEvent(false, false)]
    procedure OnBeforeCreate(var Body: Text; var Handled: Boolean) begin end;
}

// The partner subscriber: BodyText.AddText(Body) + a signature, then Format(BodyText).
codeunit 50998 "RC Signature"
{
    EventSubscriberInstance = StaticAutomatic;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RC Message", 'OnBeforeCreate', '', false, false)]
    local procedure AddSignature(var Body: Text; var Handled: Boolean)
    var BodyText: BigText;
    begin
        BodyText.AddText(Body);
        BodyText.AddText('<p>Signature</p>');
        Body := Format(BodyText);
    end;
}

// Stand-in for the e-mail editor: a field over a Text variable.
page 50999 "RC Editor"
{
    PageType = Card;
    layout { area(Content) { field(BodyField; BodyText) { ApplicationArea = All; } } }
    var BodyText: Text;
    trigger OnOpenPage() var Msg: Codeunit "RC Message"; begin BodyText := Msg.GetBody(); end;
}

page 51001 "RC Rich Editor"
{
    PageType = Card;
    layout { area(Content) { field(BodyField; BodyText) { ApplicationArea = All; MultiLine = true; } } }
    var BodyText: Text;
    trigger OnOpenPage() var Msg: Codeunit "RC Message"; begin BodyText := Msg.GetBody(); end;
}

codeunit 51000 "RC Tests"
{
    Subtype = Test;
    var Kept: Text;

    local procedure SendBody(): Text
    var Tmp: Record "RT Row" temporary; R: Record "RT Row"; Item: Codeunit "RC Item"; Msg: Codeunit "RC Message"; Out: OutStream;
    begin
        R.DeleteAll();
        R.Id := 1; R.Insert();
        Tmp.Id := 1;
        Tmp.Data.CreateOutStream(Out);
        Report.SaveAs(Report::"RT Word", '', ReportFormat::Html, Out);
        Tmp.Insert();
        Item.SetBody(Tmp);
        Msg.Create(Item.GetBodyText());
        exit(Msg.GetBody());
    end;

    procedure PassThrough(): Text var Body: Text; begin Body := SendBody(); exit(Format(Body <> '')); end;
    procedure AssertNotEmpty() var Body: Text; begin Body := SendBody(); if Body = '' then Error('empty body'); end;
    procedure BodyContains(): Boolean begin exit(SendBody().Contains('NLB')); end;
    procedure BodyCompared(): Boolean begin exit(SendBody() = '<html>'); end;
    procedure BodyLength(): Integer begin exit(StrLen(SendBody())); end;

    // The editor shows the body the message holds; the test reads the field (the reporter's handler reads
    // EmailEditor.BodyField.Value()).
    procedure ThroughTheEditor(): Text
    var Editor: TestPage "RC Editor";
    begin
        SendBody();
        Editor.OpenView();
        Kept := Editor.BodyField.Value();
        exit(Format(Kept <> ''));
    end;
    procedure ThroughTheRichEditor(): Text
    var Editor: TestPage "RC Rich Editor";
    begin
        SendBody();
        Editor.OpenView();
        Kept := Editor.BodyField.Value();
        exit(Format(Kept <> ''));
    end;
}
"#;

fn src() -> String {
    format!("{}{CHAIN}", crate::rendered_text::FIXTURE)
}

fn run(proc: &str) -> Result<Value, Signal> {
    Built::from_source(&src()).run_in("RC Tests", proc)
}

#[test]
fn rendered_text_chain_the_body_passes_through_and_is_not_empty() {
    assumptions::clear_hits();
    assert_eq!(al_testkit::text(run("PassThrough")), "Yes");
    assert!(assumptions::take_hits().contains(&Assumption::ReportHtmlRenderedText));
    run("AssertNotEmpty").expect("AreNotEqual('', Body) passes");
}

#[test]
fn rendered_text_chain_contains_compare_and_strlen_of_the_body_refuse() {
    for p in ["BodyContains", "BodyCompared", "BodyLength"] {
        match run(p) {
            Err(Signal::Unsupported(why)) => assert!(why.contains("rendered content of report RT Word"), "{p}: {why}"),
            other => panic!("{p}: expected a refusal, got {other:?}"),
        }
    }
}

#[test]
fn rendered_text_chain_a_plain_page_field_over_the_body_shows_it() {
    // P11: a field over a Text variable with no property that changes what it shows.
    assumptions::clear_hits();
    assert_eq!(al_testkit::text(run("ThroughTheEditor")), "Yes");
    assert!(assumptions::take_hits().contains(&Assumption::ReportHtmlRenderedText));
}

#[test]
fn rendered_text_chain_a_multiline_page_field_over_the_body_refuses_until_measured() {
    match run("ThroughTheRichEditor") {
        Err(Signal::Unsupported(why)) => assert!(why.contains("rendered content"), "{why}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}
