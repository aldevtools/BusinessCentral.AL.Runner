//! al-runner issue 6924 (waldo, 156 tests): an event a page extension publishes with IncludeSender, whose subscriber asks for
//! the sender as a `Page` parameter, was handed the extension's Rec and failed with "conversion of Record of table object N to
//! Page". The sender is the page instance (events/extension-events/PageExtensionSender.al).

use al_testkit::Built;

const SRC: &str = r#"
table 52700 "LD Snd Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }
page 52701 "LD Snd Host" { PageType = Card; SourceTable = "LD Snd Row"; layout { area(Content) { field(Id; Rec.Id) { } } } }
pageextension 52702 "LD Snd Ext" extends "LD Snd Host"
{
    trigger OnAfterGetRecord() begin OnWithSender(Rec.Id); OnWithoutSender(Rec.Id); end;
    [IntegrationEvent(true, false)] local procedure OnWithSender(Number: Integer) begin end;
    [IntegrationEvent(false, false)] local procedure OnWithoutSender(Number: Integer) begin end;
}
codeunit 52703 "LD Snd Sub"
{
    SingleInstance = true;
    var Seen: Integer;
    [EventSubscriber(ObjectType::Page, Page::"LD Snd Host", 'OnWithSender', '', false, false)]
    local procedure TakesSender(var Sender: Page "LD Snd Host"; Number: Integer) begin Seen := -1; end;
    [EventSubscriber(ObjectType::Page, Page::"LD Snd Host", 'OnWithoutSender', '', false, false)]
    local procedure TakesNumber(Number: Integer) begin Seen := Number; end;
    procedure GetSeen(): Integer begin exit(Seen); end;
}
codeunit 52704 "LD Snd Run"
{
    procedure Open(): Integer var H: TestPage "LD Snd Host"; R: Record "LD Snd Row"; S: Codeunit "LD Snd Sub"; begin
        R.Id := 7; R.Insert(); H.OpenView(); H.Close(); exit(S.GetSeen()); end;
}
"#;

// The sender is the page the extension runs on: the subscriber takes it and the call goes through. The event without a sender is
// raised first, so `Seen` stays -1 only when the sender subscriber ran last.
#[test]
fn a_subscriber_that_asks_for_the_sender_of_a_page_extension_event_gets_the_page() {
    let mut b = Built::from_source(&SRC.replace(
        "OnWithSender(Rec.Id); OnWithoutSender(Rec.Id);",
        "OnWithoutSender(Rec.Id); OnWithSender(Rec.Id);",
    ));
    let r = b.run_in("LD Snd Run", "Open");
    assert!(matches!(r, Ok(al_builtins::value::Value::Integer(-1))), "{r:?}");
}
