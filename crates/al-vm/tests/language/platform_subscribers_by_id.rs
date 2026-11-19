//! A subscriber that names a platform codeunit by its id instead of its name (al-runner issue 7644). The runtime raises the
//! events of `Filter Helper Triggers`, `UI Helper Triggers` and `Reporting Triggers` to the subscribers that name the
//! codeunit; whether BC treats the id form the same is not pinned, so raising the event such a subscriber names refuses, for a
//! manual subscriber only while an instance is bound. A subscriber written with the id of an app's own codeunit, even one
//! with a platform codeunit's name, keeps main's result.

use al_builtins::native::Signal;
use al_testkit::{error_message, run_in, text};

fn refused(r: Result<al_builtins::value::Value, Signal>) -> String {
    match r {
        Err(Signal::Unsupported(m)) => m.to_string(),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// A record filter whose Integer text the runtime hands to `MakeIntFilter` of the platform codeunit Filter Helper Triggers.
/// No source declares the platform codeunit: System.app is not loaded (a `--no-deps` run), the subscriber is decided by
/// the id it writes.
const FILTER_HELPER: &str = r#"
table 50210 FRow
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}
codeunit 50100 C
{
    procedure Word(): Text
    var R: Record FRow;
    begin
        R.SetFilter(Id, 'abc');
        exit(R.GetFilter(Id));
    end;
}
"#;

const BY_ID_FILTER_SUB: &str = r#"
codeunit 50211 FSub
{
    [EventSubscriber(ObjectType::Codeunit, 2000000007, 'MakeIntFilter', '', false, false)]
    local procedure MakeIntFilter(var IntFilterText: Text)
    begin
        if IntFilterText = 'abc' then
            IntFilterText := '5';
    end;
}
"#;

#[test]
fn a_make_int_filter_subscriber_written_with_the_codeunit_id_refuses_the_raise() {
    let m = refused(run_in(&format!("{FILTER_HELPER}{BY_ID_FILTER_SUB}"), "C", "Word"));
    assert!(m.contains("by its id"), "{m}");
}

/// Runs `proc` of `object` in a workspace app whose dependency app holds `dep`.
fn run_with_dependency(dep: &str, ws: &str, object: &str, proc: &str) -> Result<al_builtins::value::Value, Signal> {
    use al_sema::workspace::{AppKind, AppSource};
    let app = |id: &str, deps: &[&str], src: &str| AppSource {
        id: id.into(),
        name: id.into(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![(format!("{id}.al"), src.to_string())],
    };
    let (w, l) = al_sema::lower::compile_apps_with(
        vec![app("dep", &[], dep), app("ws", &["dep"], ws)],
        vec![AppKind::Dependency, AppKind::Workspace],
        &Default::default(),
        None,
        Default::default(),
    );
    assert!(w.diagnostics.is_empty(), "{:?}", w.diagnostics);
    al_testkit::Built::new(l).run_in(object, proc)
}

/// al-runner issue 8466: a dependency app's subscriber written with the bare id is found as one written in the app under test
/// is, so raising the event refuses where main left the subscriber out.
#[test]
fn a_dependency_subscriber_written_with_the_codeunit_id_refuses_the_raise() {
    let m = refused(run_with_dependency(BY_ID_FILTER_SUB, FILTER_HELPER, "C", "Word"));
    assert!(m.contains("by its id"), "{m}");
}

const UI_HELPER: &str = r#"
codeunit 50462 Subs
{
    [EventSubscriber(ObjectType::Codeunit, 2000000004, 'CaptionClassTranslate', '', false, false)]
    local procedure T(Language: Integer; CaptionExpr: Text[1024]; var Translation: Text[1024])
    begin
        Translation := 'changed';
    end;
}
codeunit 50469 T
{
    procedure F(): Text
    begin
        exit(CaptionClassTranslate('3,Foo'));
    end;
}
"#;

#[test]
fn a_caption_class_translate_subscriber_written_with_the_codeunit_id_refuses_the_raise() {
    let m = refused(run_in(UI_HELPER, "T", "F"));
    assert!(m.contains("by its id"), "{m}");
}

const REPORTING: &str = r#"
report 50500 R { ProcessingOnly = true; UseRequestPage = false; dataset { dataitem(I; Integer) { } } }
codeunit 50502 S
{
    [EventSubscriber(ObjectType::Codeunit, 2000000005, 'SubstituteReport', '', false, false)]
    local procedure OnSubstitute(ReportId: Integer; RunMode: Option Normal,ParametersOnly,Execute,Print,SaveAs,RunModal; RequestPageXml: Text; RecordRef: RecordRef; var NewReportId: Integer)
    begin
        NewReportId := 50509;
    end;
}
codeunit 50503 U
{
    procedure Id(): Text
    begin
        exit(Format(Report.GetSubstituteReportId(Report::R)));
    end;
}
"#;

#[test]
fn a_substitute_report_subscriber_written_with_the_codeunit_id_refuses_the_raise() {
    let m = refused(run_in(REPORTING, "U", "Id"));
    assert!(m.contains("by its id"), "{m}");
}

/// Main's result: a subscriber written with the id of an app's own publisher runs (the raise reaches it).
#[test]
fn a_subscriber_written_with_the_id_of_an_own_codeunit_keeps_running() {
    let src = r#"
codeunit 50200 Pub
{
    [IntegrationEvent(false, false)] procedure OnWork(var Log: Text) begin end;
    procedure Go(): Text var L: Text; begin OnWork(L); exit(L); end;
}
codeunit 50201 Sub
{
    [EventSubscriber(ObjectType::Codeunit, 50200, 'OnWork', '', false, false)]
    local procedure W(var Log: Text) begin Log += 'ran'; end;
}
codeunit 50100 C
{
    procedure F(): Text var P: Codeunit Pub; begin exit(P.Go()); end;
}
"#;
    assert_eq!(text(run_in(src, "C", "F")), "ran");
}

/// A by-id subscriber of a manual codeunit that no instance is bound to receives nothing: the raise keeps main's result. The
/// same subscriber with a bound instance refuses.
const MANUAL_FILTER_SUB: &str = r#"
codeunit 50211 FSub
{
    EventSubscriberInstance = Manual;

    [EventSubscriber(ObjectType::Codeunit, 2000000007, 'MakeIntFilter', '', false, false)]
    local procedure MakeIntFilter(var IntFilterText: Text)
    begin
        if IntFilterText = 'abc' then
            IntFilterText := '5';
    end;
}
codeunit 50212 Binder
{
    procedure Bound(): Text
    var S: Codeunit FSub; C: Codeunit C;
    begin
        BindSubscription(S);
        exit(C.Word());
    end;
}
"#;

#[test]
fn a_manual_by_id_subscriber_never_bound_keeps_the_evaluate_error() {
    let m = error_message(run_in(&format!("{FILTER_HELPER}{MANUAL_FILTER_SUB}"), "C", "Word"));
    assert!(m.contains("abc"), "{m}");
}

#[test]
fn a_manual_by_id_subscriber_with_a_bound_instance_refuses_the_raise() {
    let m = refused(run_in(
        &format!("{FILTER_HELPER}{MANUAL_FILTER_SUB}"),
        "Binder",
        "Bound",
    ));
    assert!(m.contains("by its id 2000000007"), "{m}");
}

#[test]
fn a_manual_by_id_substitute_report_subscriber_never_bound_keeps_the_report() {
    let src = REPORTING.replace(
        "codeunit 50502 S\n{",
        "codeunit 50502 S\n{\n    EventSubscriberInstance = Manual;",
    );
    assert!(src.contains("Manual"));
    assert_eq!(text(run_in(&src, "U", "Id")), "50500");
}

/// An app's own codeunit with a platform codeunit's name, subscribed by its own id: not the platform codeunit, main's result
/// (the evaluate error of the filter, and its own event reaches the subscriber).
const OWN_NAMED: &str = r#"
codeunit 50300 "Filter Helper Triggers"
{
    [IntegrationEvent(false, false)] procedure MakeIntFilter(var IntFilterText: Text) begin end;
    procedure Raise(): Text var T: Text; begin T := 'abc'; MakeIntFilter(T); exit(T); end;
}
codeunit 50211 FSub
{
    [EventSubscriber(ObjectType::Codeunit, 50300, 'MakeIntFilter', '', false, false)]
    local procedure MakeIntFilter(var IntFilterText: Text)
    begin
        if IntFilterText = 'abc' then
            IntFilterText := '5';
    end;
}
codeunit 50213 Own
{
    procedure Raise(): Text var F: Codeunit "Filter Helper Triggers"; begin exit(F.Raise()); end;
}
"#;

#[test]
fn an_own_codeunit_named_like_a_platform_codeunit_subscribed_by_its_id_keeps_main() {
    let src = format!("{FILTER_HELPER}{OWN_NAMED}");
    let m = error_message(run_in(&src, "C", "Word"));
    assert!(m.contains("abc"), "{m}");
    assert_eq!(text(run_in(&src, "Own", "Raise")), "5");
}
