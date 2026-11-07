//! al-runner issue 6924 (waldo, 156 tests): an event a page extension publishes with IncludeSender, whose subscriber asks for
//! the sender as a `Page` parameter, was handed the extension's Rec and failed with "conversion of Record of table object N to
//! Page". What BC passes as the sender of a page extension's event is not pinned by the corpus (events/extension-events pins a
//! table extension's and a report extension's sender, and a page extension's event without a sender), so the subscriber that
//! asks for it is refused with that reason.

use al_builtins::native::Signal;
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

// The extension's procedure raises the event on the open page; a subscriber that takes the sender is refused, naming the reason.
#[test]
fn a_subscriber_that_asks_for_the_sender_of_a_page_extension_event_is_refused() {
    let mut b = Built::from_source(SRC);
    let r = b.run_in("LD Snd Run", "Open");
    let Err(Signal::Unsupported(why)) = r else {
        panic!("{r:?}")
    };
    assert!(why.contains("IncludeSender on a pageextension"), "{why}");
}
