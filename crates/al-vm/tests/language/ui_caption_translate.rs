//! CaptionClassTranslate of the platform codeunit `UI Helper Triggers`: raised by the `CaptionClassTranslate` statement once per
//! text, silent for FieldCaption of a table field (events/platform-triggers/P114GlobalTriggerEvents.al:309-330 and
//! PtObserve.al:135-150 `Captions`, `fieldcap` log=[]).

use al_testkit::text;

const OBJECTS: &str = r#"
codeunit 42 "Caption Class"
{
    [IntegrationEvent(false, false)] procedure OnAfterCaptionClassResolve(Language: Integer; CaptionExpression: Text; var Caption: Text[1024]) begin end;
}
codeunit 50461 Log
{
    SingleInstance = true;
    var Lines: Text;
    procedure Add(L: Text) begin if Lines <> '' then Lines += ' | '; Lines += L; end;
    procedure Take(): Text var T: Text; begin T := Lines; Lines := ''; exit(T); end;
}
codeunit 50462 Subs
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"UI Helper Triggers", 'CaptionClassTranslate', '', false, false)]
    local procedure T(Language: Integer; CaptionExpr: Text[1024]; var Translation: Text[1024])
    var L: Codeunit Log;
    begin
        L.Add('ui(session=' + Format(Language = GlobalLanguage()) + ',expr=' + CaptionExpr + ',in=' + Translation + ')');
        if CaptionExpr = 'CHANGE,x' then Translation := 'changed';
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Caption Class", 'OnAfterCaptionClassResolve', '', false, false)]
    local procedure After(Language: Integer; CaptionExpression: Text; var Caption: Text[1024])
    begin
    end;
}
table 50463 Row
{
    fields { field(1; Id; Integer) { } field(2; Cap; Code[20]) { CaptionClass = '3,Caption three'; } }
    keys { key(PK; Id) { } }
}
"#;

fn run(body: &str) -> Result<al_builtins::value::Value, al_builtins::native::Signal> {
    let src = format!("{OBJECTS}\ncodeunit 50469 T\n{{\n{body}\n}}\n");
    al_testkit::run_in(&src, "T", "F")
}

/// P114GlobalTriggerEvents.al:309-330: area 3 raises the event with the text after the comma as the translation, an unclaimed
/// area and a text with no comma raise it with the whole text.
#[test]
fn the_statement_raises_the_ui_helper_event_once_per_translation() {
    let got = text(run(r#"
    procedure F(): Text
    var L: Codeunit Log; S: Text;
    begin
        S := CaptionClassTranslate('3,Foo') + ' / ' + L.Take() + ' || ';
        S += CaptionClassTranslate('P114ZZ,Foo') + ' / ' + L.Take() + ' || ';
        S += CaptionClassTranslate('P114 no comma') + ' / ' + L.Take();
        exit(S);
    end;"#));
    assert_eq!(
        got,
        "Foo / ui(session=Yes,expr=3,Foo,in=Foo) || P114ZZ,Foo / ui(session=Yes,expr=P114ZZ,Foo,in=P114ZZ,Foo) || \
         P114 no comma / ui(session=Yes,expr=P114 no comma,in=P114 no comma)"
    );
}

/// A subscriber that changes Translation has an effect no case pins: refused.
#[test]
fn a_ui_helper_subscriber_that_changes_the_translation_is_refused() {
    let r = run(r#"procedure F(): Text begin exit(CaptionClassTranslate('CHANGE,x')); end;"#);
    let e = format!("{r:?}");
    assert!(e.contains("Unsupported") && e.contains("CaptionClassTranslate"), "{e}");
}

/// PtObserve.al:135-150 `Captions`: FieldCaption of a field with CaptionClass '3,Caption three' raises no UI Helper event, with
/// an OnAfterCaptionClassResolve subscriber present.
#[test]
fn field_caption_of_an_area_3_class_raises_no_ui_helper_event() {
    let got = text(run(r#"
    procedure F(): Text
    var L: Codeunit Log; R: Record Row;
    begin
        exit(R.FieldCaption(Cap) + ' / ' + L.Take());
    end;"#));
    assert_eq!(got, "Caption three / ");
}
