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

/// Runs `F` of codeunit `test_object` (in `test`) in a workspace app whose dependency app (id `lib_id`) holds a
/// SingleInstance log, the CaptionClassTranslate subscriber codeunit `sub` with the properties `props` and the body `body`
/// that names its publisher `publisher`, and `extra`. The dependency is compiled on demand.
fn run_dep(
    lib_id: &str,
    (sub, props, body): (&str, &str, &str),
    publisher: &str,
    extra: &str,
    (test, test_object): (&str, &str),
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
    {props}
    [EventSubscriber(ObjectType::Codeunit, {publisher}, 'CaptionClassTranslate', '', false, false)]
    local procedure T(Language: Integer; CaptionExpr: Text[1024]; var Translation: Text[1024])
    var Log: Codeunit "BI Log";
    begin
        {body}
        Log.Add('CCT(' + CaptionExpr + ')');
    end;
}}
{extra}
"#
    );
    let (ws, l) = al_sema::lower::compile_apps_with(
        vec![app(lib_id, &[], &lib), app("test", &[lib_id], test)],
        vec![AppKind::Dependency, AppKind::Workspace],
        &Default::default(),
        None,
        Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    al_testkit::Built::new(l).run_in(test_object, "F")
}

/// The publisher argument written with the codeunit's name.
const NAME: &str = "Codeunit::\"UI Helper Triggers\"";

fn run_with_dependency_subscriber(
    sub: &str,
    test: &str,
) -> Result<al_builtins::value::Value, al_builtins::native::Signal> {
    run_dep("lib", (sub, "", ""), NAME, "", (test, "BI Ui Test"))
}

fn refused(r: Result<al_builtins::value::Value, al_builtins::native::Signal>) -> String {
    match r {
        Err(al_builtins::native::Signal::Unsupported(m)) => m.to_string(),
        other => panic!("expected a refusal, got {other:?}"),
    }
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
    let m = refused(run_with_dependency_subscriber("BI Ui Sub", SEEING_TEST));
    assert!(m.contains("BI Ui Sub"), "{m}");
}

/// The System Application's "Caption Class Impl." (its name and the System Application's app id) raises Caption Class's
/// events, which the runtime raises itself: it is still not run (main's result).
#[test]
fn the_caption_class_impl_subscriber_of_the_system_application_is_not_run() {
    let got = text(run_dep(
        al_ir::SYSTEM_APPLICATION_APP_ID,
        ("Caption Class Impl.", "", ""),
        NAME,
        "",
        (SEEING_TEST, "BI Ui Test"),
    ));
    assert_eq!(got, "Foo log=");
}

/// A codeunit named "Caption Class Impl." in another app is not the System Application's: refused (review probe uiimpl).
#[test]
fn a_caption_class_impl_twin_of_another_app_refuses_the_raise() {
    let m = refused(run_dep(
        "lib",
        ("Caption Class Impl.", "", "Translation := 'CHG';"),
        NAME,
        "",
        (SEEING_TEST, "BI Ui Test"),
    ));
    assert!(m.contains("Caption Class Impl."), "{m}");
}

/// A manual dependency subscriber nobody bound receives nothing: main's answer stays (review probe uiman D1). Bound, it
/// refuses (D3).
#[test]
fn a_manual_dependency_subscriber_refuses_only_while_bound() {
    let manual = ("BI Ui Man Sub", "EventSubscriberInstance = Manual;", "");
    assert_eq!(
        text(run_dep("lib", manual, NAME, "", (SEEING_TEST, "BI Ui Test"))),
        "Foo log="
    );
    let bound = r#"
codeunit 50903 "BI Ui Bound"
{
    procedure F(): Text
    var Log: Codeunit "BI Log"; M: Codeunit "BI Ui Man Sub"; T: Text;
    begin
        BindSubscription(M);
        T := CaptionClassTranslate('3,Foo');
        exit(T + ' log=' + Log.Get());
    end;
}
"#;
    let m = refused(run_dep("lib", manual, NAME, "", (bound, "BI Ui Bound")));
    assert!(m.contains("BI Ui Man Sub"), "{m}");
}

/// The statement in dependency code, the subscriber in a dependency, compiled on demand: refused (review probe uidepstmt).
#[test]
fn the_statement_in_dependency_code_refuses_for_a_dependency_subscriber() {
    let tr = "codeunit 50222 \"BI Tr\" { procedure Tr(): Text begin exit(CaptionClassTranslate('3,Foo')); end; }";
    let test = r#"
codeunit 50901 "BI Ui Test"
{
    procedure F(): Text
    var Log: Codeunit "BI Log"; Tr: Codeunit "BI Tr";
    begin
        exit(Tr.Tr() + ' log=' + Log.Get());
    end;
}
"#;
    let m = refused(run_dep("lib", ("BI Ui Sub", "", ""), NAME, tr, (test, "BI Ui Test")));
    assert!(m.contains("BI Ui Sub"), "{m}");
}
