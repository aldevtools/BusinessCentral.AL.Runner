//! Platform page events on pages without a source table (issue 7430). No corpus test pins when BC raises them there
//! (every event subscriber test of the corpus is on a page with SourceTable: events/platform-triggers/PageTriggerEventRows.al,
//! testpage/ABTests.al, events/ClEventLeads.al:530), so a page with such a subscriber is refused when it opens, before any
//! of its code runs, whatever the page type and the event. Probe spec: probe-page-events-no-source.md.

use al_builtins::native::Signal;

fn src(page_type: &str, event: &str) -> String {
    format!(
        r#"
codeunit 69980 "NSE Log"
{{
    SingleInstance = true;
    var Entries: Text;
    procedure Add(T: Text) begin Entries += T + ';'; end;
    procedure Take(): Text var R: Text; begin R := Entries; Entries := ''; exit(R); end;
}}
page 69981 "NSE Page"
{{
    PageType = {page_type}; ApplicationArea = All; UsageCategory = None;
    layout {{ area(Content) {{ field(Txt; TxtVar) {{ }} }} }}
    var TxtVar: Text;
    trigger OnOpenPage() var Log: Codeunit "NSE Log"; begin Log.Add('page.open'); end;
}}
codeunit 69982 "NSE Sub"
{{
    [EventSubscriber(ObjectType::Page, Page::"NSE Page", '{event}', '', false, false)]
    local procedure OnEvt() var Log: Codeunit "NSE Log"; begin Log.Add('sub.{event}'); end;
}}
codeunit 69983 C
{{
    procedure Open(): Text
    var P: TestPage "NSE Page"; Log: Codeunit "NSE Log";
    begin
        Log.Take();
        P.OpenView();
        P.Close();
        exit(Log.Take());
    end;
}}
"#
    )
}

#[test]
fn a_subscriber_on_a_page_without_source_refuses_the_open_before_its_code_runs() {
    for page_type in ["RoleCenter", "Card", "NavigatePage", "Worksheet", "ListPlus"] {
        for event in [
            "OnOpenPageEvent",
            "OnClosePageEvent",
            "OnQueryClosePageEvent",
            "OnAfterGetRecordEvent",
            "OnAfterGetCurrRecordEvent",
            "OnNewRecordEvent",
            "OnInsertRecordEvent",
            "OnModifyRecordEvent",
            "OnDeleteRecordEvent",
        ] {
            let mut b = al_testkit::Built::from_source(&src(page_type, event));
            let (oi, ri) = b.routine_in("C", "Open");
            let r = b.call(oi, ri, vec![]);
            let Err(Signal::Unsupported(why)) = r else {
                panic!("{page_type} {event}: {r:?}")
            };
            assert!(
                why.contains("without a source table") && why.to_lowercase().contains(&event.to_lowercase()),
                "{page_type} {event}: {why}"
            );
            assert!(
                why.to_lowercase().contains(&page_type.to_lowercase()),
                "{page_type} {event}: the reason names the page type: {why}"
            );
        }
    }
}

#[test]
fn nothing_ran_before_the_refusal() {
    let s = src("RoleCenter", "OnOpenPageEvent")
        + "\ncodeunit 69985 D { procedure Seen(): Text var Log: Codeunit \"NSE Log\"; begin exit(Log.Take()); end; }\n";
    let mut b = al_testkit::Built::from_source(&s);
    let (oi, ri) = b.routine_in("C", "Open");
    assert!(matches!(b.call(oi, ri, vec![]), Err(Signal::Unsupported(_))));
    let (oi, ri) = b.routine_in("D", "Seen");
    let (v, _) = b.call(oi, ri, vec![]).unwrap();
    assert_eq!(al_testkit::text(Ok(v)), "", "OnOpenPage must not have run");
}

#[test]
fn a_page_without_a_subscriber_still_opens() {
    // The subscriber listens to another page: the Role Center itself has no subscriber.
    let s = src("RoleCenter", "OnOpenPageEvent").replace(
        "Page::\"NSE Page\", 'OnOpenPageEvent'",
        "Page::\"NSE Other\", 'OnOpenPageEvent'",
    ) + "\npage 69984 \"NSE Other\" { PageType = RoleCenter; ApplicationArea = All; UsageCategory = None; }\n";
    let mut b = al_testkit::Built::from_source(&s);
    let (oi, ri) = b.routine_in("C", "Open");
    let (v, _) = b.call(oi, ri, vec![]).unwrap();
    assert_eq!(al_testkit::text(Ok(v)), "page.open;");
}
