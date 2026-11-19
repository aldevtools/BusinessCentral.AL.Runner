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

/// Runs `F` of the test codeunit `test` in a workspace app whose dependency app holds a SingleInstance log and the
/// CaptionClassTranslate subscriber codeunit `sub`, which logs to it.
fn run_with_dependency_subscriber(
    sub: &str,
    test: &str,
) -> Result<al_builtins::value::Value, al_builtins::native::Signal> {
    use al_sema::workspace::{AppKind, AppSource};
    let app = |id: &str, deps: &[&str], src: &str| AppSource {
        id: id.into(),
        name: id.into(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![(format!("{id}.al"), src.to_string())],
    };
    let lib = format!(
        r#"
codeunit 50220 "BI Log"
{{
    SingleInstance = true;
    var L: Text;
    procedure Add(T: Text) begin L += T + ';'; end;
    procedure Get(): Text begin exit(L); end;
}}
codeunit 50221 "{sub}"
{{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"UI Helper Triggers", 'CaptionClassTranslate', '', false, false)]
    local procedure T(Language: Integer; CaptionExpr: Text[1024]; var Translation: Text[1024])
    var Log: Codeunit "BI Log";
    begin
        Log.Add('CCT(' + CaptionExpr + ')');
    end;
}}
"#
    );
    let (ws, l) = al_sema::lower::compile_apps_with(
        vec![app("lib", &[], &lib), app("test", &["lib"], test)],
        vec![AppKind::Dependency, AppKind::Workspace],
        &Default::default(),
        None,
        Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    al_testkit::Built::new(l).run_in("BI Ui Test", "F")
}

/// The test sees the subscriber's log.
const SEEING_TEST: &str = r#"
codeunit 50901 "BI Ui Test"
{
    procedure F(): Text
    var Log: Codeunit "BI Log"; T: Text;
    begin
        T := CaptionClassTranslate('3,Foo');
        exit(T + ' log=' + Log.Get());
    end;
}
"#;

/// A CaptionClassTranslate subscriber of a dependency app (al-runner issue 8470): what it does is not pinned, so a raise that
/// would reach it is refused instead of leaving it out. The own subscriber of the same event still runs
/// (`the_statement_raises_the_ui_helper_event_once_per_translation`).
#[test]
fn a_dependency_caption_class_translate_subscriber_refuses_the_raise() {
    match run_with_dependency_subscriber("BI Ui Sub", SEEING_TEST) {
        Err(al_builtins::native::Signal::Unsupported(m)) => assert!(m.contains("BI Ui Sub"), "{m}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// The System Application's "Caption Class Impl." raises Caption Class's events, which the runtime raises itself: it is
/// still not run (main's result).
#[test]
fn the_caption_class_impl_subscriber_of_a_dependency_is_not_run() {
    let got = text(run_with_dependency_subscriber("Caption Class Impl.", SEEING_TEST));
    assert_eq!(got, "Foo log=");
}
