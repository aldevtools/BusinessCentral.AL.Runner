//! The page event OnAfterGetRecordEvent, raised by the test client after the page's OnAfterGetRecord trigger and the
//! extensions' (events/ClEventLeads.al:513-532, PageTriggers_ExtensionsAndSubscribers_Order: `page.OnAfterGetRecord;
//! extA.OnAfterGetRecord;extB.OnAfterGetRecord;sub.OnAfterGetRecordEvent;page.OnAfterGetCurrRecord;...`).

const SRC: &str = r#"
codeunit 69980 "AE Log"
{
    SingleInstance = true;
    var Entries: Text;
    procedure Add(T: Text) begin Entries += T + ';'; end;
    procedure Take(): Text var R: Text; begin R := Entries; Entries := ''; exit(R); end;
}
table 69981 "AE Row" { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } } keys { key(PK; Id) { } } }
page 69982 "AE Card"
{
    PageType = Card; SourceTable = "AE Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(IdCtl; Rec.Id) { } } }
    trigger OnOpenPage() var Log: Codeunit "AE Log"; begin Log.Add('page.OnOpenPage'); end;
    trigger OnAfterGetRecord() var Log: Codeunit "AE Log"; begin Log.Add('page.OnAfterGetRecord'); end;
    trigger OnAfterGetCurrRecord() var Log: Codeunit "AE Log"; begin Log.Add('page.OnAfterGetCurrRecord'); end;
}
pageextension 69983 "AE Card A" extends "AE Card"
{
    trigger OnAfterGetRecord() var Log: Codeunit "AE Log"; begin Log.Add('extA.OnAfterGetRecord'); end;
}
page 69984 "AE Plain"
{
    PageType = Card; SourceTable = "AE Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(IdCtl; Rec.Id) { } } }
}
codeunit 69985 "AE Subs"
{
    EventSubscriberInstance = Manual;
    [EventSubscriber(ObjectType::Page, Page::"AE Card", 'OnAfterGetRecordEvent', '', false, false)]
    local procedure OnAgr(var Rec: Record "AE Row") var Log: Codeunit "AE Log"; begin Log.Add('sub.OnAfterGetRecordEvent ' + Format(Rec.Id)); end;
    [EventSubscriber(ObjectType::Page, Page::"AE Plain", 'OnAfterGetRecordEvent', '', false, false)]
    local procedure OnAgrPlain(var Rec: Record "AE Row") var Log: Codeunit "AE Log"; begin Log.Add('sub.plain ' + Format(Rec.Id)); end;
}
codeunit 69986 C
{
    procedure Open(): Text
    var R: Record "AE Row"; P: TestPage "AE Card"; S: Codeunit "AE Subs"; Log: Codeunit "AE Log"; T: Text;
    begin
        R.Id := 1; R.Insert();
        BindSubscription(S);
        P.OpenView();
        T := Log.Take();
        P.Close();
        UnbindSubscription(S);
        exit(T);
    end;

    procedure OpenPlain(): Text
    var R: Record "AE Row"; P: TestPage "AE Plain"; S: Codeunit "AE Subs";
    begin
        R.Id := 1; R.Insert();
        BindSubscription(S);
        P.OpenView();
        P.Close();
        exit('opened');
    end;
}
"#;

#[test]
fn the_event_follows_the_page_and_extension_triggers() {
    let mut b = al_testkit::Built::from_source(SRC);
    let (oi, ri) = b.routine_in("C", "Open");
    let (v, _) = b.call(oi, ri, vec![]).unwrap();
    assert_eq!(
        al_testkit::text(Ok(v)),
        "page.OnOpenPage;page.OnAfterGetRecord;extA.OnAfterGetRecord;sub.OnAfterGetRecordEvent 1;page.OnAfterGetCurrRecord;"
    );
}

// A page that does not declare OnAfterGetRecord: when the client raises the event is not pinned, so the open refuses.
#[test]
fn a_page_without_the_trigger_refuses_the_open() {
    let mut b = al_testkit::Built::from_source(SRC);
    let (oi, ri) = b.routine_in("C", "OpenPlain");
    let r = b.call(oi, ri, vec![]);
    let Err(al_builtins::native::Signal::Unsupported(why)) = r else {
        panic!("{r:?}")
    };
    assert!(why.contains("onaftergetrecordevent"), "{why}");
}
