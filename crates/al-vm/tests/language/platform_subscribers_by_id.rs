//! A subscriber that names a platform codeunit by its id instead of its name (al-runner issue 7644). The runtime raises the
//! events of `Filter Helper Triggers`, `UI Helper Triggers` and `Reporting Triggers` to the subscribers that name the
//! codeunit. A static subscriber of the app under test to the one event of each that corpus PR 1466 measured by id
//! (MakeDateFilter, CaptionClassTranslate, SubstituteReport; SbiSubscriberByIdTests.al:245, :171, :231) is handled as the
//! name form is and flags the run (`subscriber-publisher-by-id`). Every other by-id subscriber to these codeunits refuses the
//! raise of the event it names, a manual one only while an instance is bound. A subscriber written with the id of an app's
//! own codeunit, even one with a platform codeunit's name, keeps main's result.

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
    run_with_dependency_app("dep", dep, ws, (object, proc))
}

/// [`run_with_dependency`] with the dependency app's id `dep_id`.
fn run_with_dependency_app(
    dep_id: &str,
    dep: &str,
    ws: &str,
    (object, proc): (&str, &str),
) -> Result<al_builtins::value::Value, Signal> {
    use al_sema::workspace::{AppKind, AppSource};
    let app = |id: &str, deps: &[&str], src: &str| AppSource {
        id: id.into(),
        name: id.into(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![(format!("{id}.al"), src.to_string())],
    };
    let (w, l) = al_sema::lower::compile_apps_with(
        vec![app(dep_id, &[], dep), app("ws", &[dep_id], ws)],
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
    assert!(m.contains("\"FSub\""), "{m}");
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

/// SbiSubscriberByIdTests.al:171: the by-id subscriber is called as the name form is, so the run gives the name form's
/// result (here its refusal of a changed Translation, the same text) and is flagged.
#[test]
fn a_caption_class_translate_subscriber_written_with_the_codeunit_id_gets_the_name_forms_result() {
    use al_builtins::assumptions::{Assumption, take_hits};
    let _ = take_hits();
    let by_id = refused(run_in(UI_HELPER, "T", "F"));
    assert!(take_hits().contains(&Assumption::SubscriberPublisherById));
    let by_name = refused(run_in(
        &UI_HELPER.replace("2000000004", "Codeunit::\"UI Helper Triggers\""),
        "T",
        "F",
    ));
    assert_eq!(by_id, by_name);
    assert!(take_hits().is_empty());
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

/// SbiSubscriberByIdTests.al:231: the by-id subscriber is called as the name form is (its NewReportId is the substitute) and
/// the run is flagged.
#[test]
fn a_substitute_report_subscriber_written_with_the_codeunit_id_runs_flagged() {
    use al_builtins::assumptions::{Assumption, take_hits};
    let _ = take_hits();
    assert_eq!(text(run_in(REPORTING, "U", "Id")), "50509");
    assert!(take_hits().contains(&Assumption::SubscriberPublisherById));
    let by_name = REPORTING.replace("2000000005", "Codeunit::\"Reporting Triggers\"");
    assert_eq!(text(run_in(&by_name, "U", "Id")), "50509");
    assert!(take_hits().is_empty());
}

/// A Date filter text with a word no filter reads, handed to `MakeDateFilter` (events/platform-triggers/PtObserve.al:157).
const DATE_FILTER: &str = r#"
table 50220 DRow
{
    fields { field(1; Id; Integer) { } field(2; D; Date) { } }
    keys { key(PK; Id) { Clustered = true; } }
}
codeunit 50221 DC
{
    procedure Word(): Text
    var R: Record DRow;
    begin
        R.SetFilter(D, 'abc');
        exit(R.GetFilter(D));
    end;
}
"#;

const BY_ID_DATE_SUB: &str = r#"
codeunit 50222 DSub
{
    [EventSubscriber(ObjectType::Codeunit, 2000000007, 'MakeDateFilter', '', false, false)]
    local procedure MakeDateFilter(var DateFilterText: Text)
    begin
        if DateFilterText = 'abc' then
            DateFilterText := '2020-12-31';
    end;
}
"#;

/// SbiSubscriberByIdTests.al:245: the by-id subscriber repairs the text as the name form does, and the run is flagged.
#[test]
fn a_make_date_filter_subscriber_written_with_the_codeunit_id_runs_flagged() {
    use al_builtins::assumptions::{Assumption, take_hits};
    let _ = take_hits();
    let by_id = text(run_in(&format!("{DATE_FILTER}{BY_ID_DATE_SUB}"), "DC", "Word"));
    assert!(take_hits().contains(&Assumption::SubscriberPublisherById));
    let by_name = BY_ID_DATE_SUB.replace("2000000007", "Codeunit::\"Filter Helper Triggers\"");
    assert_eq!(by_id, text(run_in(&format!("{DATE_FILTER}{by_name}"), "DC", "Word")));
    assert!(take_hits().is_empty());
}

/// The same subscriber in a dependency app: a dependency app's by-id subscriber is not measured, so the raise refuses.
#[test]
fn a_dependency_make_date_filter_subscriber_written_with_the_codeunit_id_refuses_the_raise() {
    let m = refused(run_with_dependency(BY_ID_DATE_SUB, DATE_FILTER, "DC", "Word"));
    assert!(m.contains("\"DSub\""), "{m}");
}

/// The same subscriber in a manual codeunit: not measured, so the raise refuses while an instance is bound, and an unbound
/// one receives nothing (the filter error stands, main's result).
#[test]
fn a_manual_make_date_filter_subscriber_written_with_the_codeunit_id_refuses_only_while_bound() {
    let manual = BY_ID_DATE_SUB.replace(
        "codeunit 50222 DSub\n{",
        "codeunit 50222 DSub\n{\n    EventSubscriberInstance = Manual;",
    );
    assert!(manual.contains("Manual"));
    let binder = r#"
codeunit 50223 DBinder
{
    procedure Bound(): Text
    var S: Codeunit DSub; C: Codeunit DC;
    begin
        BindSubscription(S);
        exit(C.Word());
    end;
}
"#;
    let src = format!("{DATE_FILTER}{manual}{binder}");
    let m = refused(run_in(&src, "DBinder", "Bound"));
    assert!(m.contains("by its id 2000000007"), "{m}");
    let m = error_message(run_in(&src, "DC", "Word"));
    assert!(m.contains("abc"), "{m}");
}

/// A dependency app's by-id SubstituteReport subscriber is not measured: the raise refuses as on main.
#[test]
fn a_substitute_report_subscriber_of_a_dependency_written_with_the_codeunit_id_refuses_the_raise() {
    let dep = r#"
codeunit 50502 S
{
    [EventSubscriber(ObjectType::Codeunit, 2000000005, 'SubstituteReport', '', false, false)]
    local procedure OnSubstitute(ReportId: Integer; RunMode: Option Normal,ParametersOnly,Execute,Print,SaveAs,RunModal; RequestPageXml: Text; RecordRef: RecordRef; var NewReportId: Integer)
    begin
        NewReportId := 50509;
    end;
}
"#;
    let ws = r#"
report 50500 R { ProcessingOnly = true; UseRequestPage = false; dataset { dataitem(I; Integer) { } } }
codeunit 50503 U
{
    procedure Id(): Text
    begin
        exit(Format(Report.GetSubstituteReportId(Report::R)));
    end;
}
"#;
    let m = refused(run_with_dependency(dep, ws, "U", "Id"));
    assert!(m.contains("by its id 2000000005"), "{m}");
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

/// A table of the subscriber's own app and its OnAfterInsertEvent, the subscriber written with the table's bare id: bound as the
/// name form is, and the run is flagged (`subscriber-publisher-by-id`, corpus PR 1466 SbiSubscriberByIdTests.al:78).
const OWN_TABLE: &str = r#"
table 50300 BRow
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}
codeunit 50301 BLog
{
    SingleInstance = true;
    var L: Text;
    procedure Add(T: Text) begin L += T + ';'; end;
    procedure Get(): Text begin exit(L); end;
}
codeunit 50302 BSub
{
    [EventSubscriber(ObjectType::Table, 50300, 'OnAfterInsertEvent', '', false, false)]
    local procedure AfterInsert(var Rec: Record BRow) var Log: Codeunit BLog; begin Log.Add('ins'); end;
}
"#;

const INSERT: &str = r#"
codeunit 50303 C
{
    procedure Go(): Text
    var R: Record BRow; Log: Codeunit BLog;
    begin
        R.Id := 1;
        R.Insert();
        exit(Log.Get());
    end;
}
"#;

#[test]
fn an_own_table_subscriber_written_with_the_bare_id_runs_flagged() {
    use al_builtins::assumptions::{Assumption, take_hits};
    let _ = take_hits();
    assert_eq!(text(run_in(&format!("{OWN_TABLE}{INSERT}"), "C", "Go")), "ins;");
    assert!(take_hits().contains(&Assumption::SubscriberPublisherById));
}

/// The same subscriber in an app that depends on the table's app: a publisher of another app is not measured (a Microsoft
/// table such as 483 is one), so the insert refuses.
#[test]
fn a_bare_id_subscriber_to_a_table_of_another_app_refuses_the_raise() {
    let (table, rest) = OWN_TABLE.split_at(OWN_TABLE.find("codeunit 50301").unwrap());
    let m = refused(run_with_dependency(table, &format!("{rest}{INSERT}"), "C", "Go"));
    assert!(m.contains("by the id 50300"), "{m}");
}

/// A table event the probe did not measure (OnAfterRenameEvent) by the bare id: refused.
#[test]
fn a_bare_id_subscriber_to_an_unmeasured_table_event_refuses_the_raise() {
    let src = OWN_TABLE.replace("'OnAfterInsertEvent'", "'OnBeforeDeleteEvent'");
    let go = INSERT.replace("R.Insert();", "R.Insert(); R.Delete(true);");
    let m = refused(run_in(&format!("{src}{go}"), "C", "Go"));
    assert!(m.contains("by the id 50300"), "{m}");
}

/// A manual subscriber in an unmeasured by-id shape refuses only while bound: unbound it receives nothing, main's answer.
#[test]
fn an_unmeasured_manual_bare_id_subscriber_refuses_only_while_bound() {
    let src = OWN_TABLE
        .replace("'OnAfterInsertEvent'", "'OnBeforeDeleteEvent'")
        .replace(
            "codeunit 50302 BSub\n{",
            "codeunit 50302 BSub\n{\n    EventSubscriberInstance = Manual;",
        );
    let go = INSERT.replace("R.Insert();", "R.Insert(); R.Delete(true);");
    assert_eq!(text(run_in(&format!("{src}{go}"), "C", "Go")), "");
    let bound = go
        .replace("R.Id := 1;", "BindSubscription(S); R.Id := 1;")
        .replace("Log: Codeunit BLog;", "Log: Codeunit BLog; S: Codeunit BSub;");
    let m = refused(run_in(&format!("{src}{bound}"), "C", "Go"));
    assert!(m.contains("by the id 50300"), "{m}");
}

/// A manual Filter Helper Triggers subscriber of a dependency app nobody bound receives nothing: the filter keeps the answer
/// it has without the subscriber. Bound, the raise refuses (`receiving_dependency_subscriber`).
#[test]
fn a_manual_dependency_filter_helper_subscriber_refuses_only_while_bound() {
    let manual = r#"
codeunit 50212 FMan
{
    EventSubscriberInstance = Manual;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Filter Helper Triggers", 'MakeIntFilter', '', false, false)]
    local procedure MakeIntFilter(var IntFilterText: Text) begin IntFilterText := '5'; end;
}
"#;
    let (table, c) = FILTER_HELPER.split_at(FILTER_HELPER.find("codeunit 50100").unwrap());
    let without = format!("{:?}", run_with_dependency(table, c, "C", "Word"));
    let with = format!("{:?}", run_with_dependency(&format!("{table}{manual}"), c, "C", "Word"));
    assert_eq!(with, without);
    let bound = c
        .replace("var R: Record FRow;", "var R: Record FRow; M: Codeunit FMan;")
        .replace("R.SetFilter", "BindSubscription(M); R.SetFilter");
    let m = refused(run_with_dependency(&format!("{table}{manual}"), &bound, "C", "Word"));
    assert!(m.contains("FMan"), "{m}");
}

/// Subscriber order with both forms mixed: alphabetical by method name within a codeunit, ascending codeunit id across
/// codeunits, whatever the form (corpus PR 1466 SbiSubscriberByIdTests.al:23, :36).
#[test]
fn bare_id_and_name_subscribers_run_in_one_order() {
    let src = r#"
codeunit 50320 OLog
{
    SingleInstance = true;
    var L: Text;
    procedure Add(T: Text) begin L += T + ';'; end;
    procedure Get(): Text begin exit(L); end;
}
codeunit 50321 OPub
{
    procedure Fire(): Text var Log: Codeunit OLog; begin OnX(); exit(Log.Get()); end;
    [IntegrationEvent(false, false)] local procedure OnX() begin end;
}
codeunit 50323 OSubHigh
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::OPub, 'OnX', '', false, false)]
    local procedure B(); var Log: Codeunit OLog; begin Log.Add('high.B'); end;
    [EventSubscriber(ObjectType::Codeunit, 50321, 'OnX', '', false, false)]
    local procedure A(); var Log: Codeunit OLog; begin Log.Add('high.A'); end;
}
codeunit 50322 OSubLow
{
    [EventSubscriber(ObjectType::Codeunit, 50321, 'OnX', '', false, false)]
    local procedure Z(); var Log: Codeunit OLog; begin Log.Add('low.Z'); end;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::OPub, 'OnX', '', false, false)]
    local procedure Y(); var Log: Codeunit OLog; begin Log.Add('low.Y'); end;
}
"#;
    assert_eq!(text(run_in(src, "OPub", "Fire")), "low.Y;low.Z;high.A;high.B;");
}

/// An own codeunit publisher and its bare-id subscriber, both in one app.
const OWN_PUB_SUB: &str = r#"
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
"#;

const CALL_GO: &str = r#"
codeunit 50100 C
{
    procedure F(): Text var P: Codeunit Pub; begin exit(P.Go()); end;
}
"#;

/// The probe measured publisher and bare-id subscriber in one app, the app under test: there it runs, flagged.
#[test]
fn a_bare_id_subscriber_to_its_own_app_publisher_in_the_app_under_test_runs_flagged() {
    use al_builtins::assumptions::{Assumption, take_hits};
    let _ = take_hits();
    assert_eq!(
        text(run_with_dependency("", &format!("{OWN_PUB_SUB}{CALL_GO}"), "C", "F")),
        "ran"
    );
    assert!(take_hits().contains(&Assumption::SubscriberPublisherById));
}

/// The same pair in a dependency app is not measured: raising the event refuses.
#[test]
fn a_bare_id_subscriber_to_its_own_app_publisher_in_a_dependency_refuses_the_raise() {
    let m = refused(run_with_dependency(OWN_PUB_SUB, CALL_GO, "C", "F"));
    assert!(m.contains("by the id 50200"), "{m}");
}

/// A stand-in for the System Application's codeunit 45 "Auto Format" in a dependency app, with the event it raises.
const AUTO_FORMAT: &str = r#"
codeunit 45 "Auto Format"
{
    [IntegrationEvent(false, false)] procedure OnResolveAutoFormat(var Log: Text) begin end;
    procedure Go(): Text var L: Text; begin OnResolveAutoFormat(L); exit(L); end;
}
"#;

const AUTO_FORMAT_SUB: &str = r#"
codeunit 50201 Sub
{
    [EventSubscriber(ObjectType::Codeunit, 45, 'OnResolveAutoFormat', '', false, false)]
    local procedure W(var Log: Text) begin Log += 'ran'; end;
}
"#;

const CALL_AUTO_FORMAT: &str = r#"
codeunit 50100 C
{
    procedure F(): Text var P: Codeunit "Auto Format"; begin exit(P.Go()); end;
}
"#;

/// A subscriber of the app under test to the System Application's Auto Format 45 by the bare id: measured
/// (SbiSubscriberByIdTests.al:201), runs flagged.
#[test]
fn a_bare_id_auto_format_subscriber_of_the_app_under_test_runs_flagged() {
    use al_builtins::assumptions::{Assumption, take_hits};
    let _ = take_hits();
    let ws = format!("{AUTO_FORMAT_SUB}{CALL_AUTO_FORMAT}");
    let got = run_with_dependency_app(al_ir::SYSTEM_APPLICATION_APP_ID, AUTO_FORMAT, &ws, ("C", "F"));
    assert_eq!(text(got), "ran");
    assert!(take_hits().contains(&Assumption::SubscriberPublisherById));
}

/// A codeunit 45 "Auto Format" of an app other than the System Application is not measured: the raise refuses.
#[test]
fn a_bare_id_subscriber_to_another_apps_auto_format_45_refuses_the_raise() {
    let ws = format!("{AUTO_FORMAT_SUB}{CALL_AUTO_FORMAT}");
    let m = refused(run_with_dependency(AUTO_FORMAT, &ws, "C", "F"));
    assert!(m.contains("by the id 45"), "{m}");
}

/// The same subscriber in the dependency app: not measured, the raise refuses.
#[test]
fn a_bare_id_auto_format_subscriber_of_a_dependency_refuses_the_raise() {
    let dep = format!("{AUTO_FORMAT}{AUTO_FORMAT_SUB}");
    let m = refused(run_with_dependency(&dep, CALL_AUTO_FORMAT, "C", "F"));
    assert!(m.contains("by the id 45"), "{m}");
}
