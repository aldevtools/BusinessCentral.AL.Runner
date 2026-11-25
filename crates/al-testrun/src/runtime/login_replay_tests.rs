// The scans of production code stop at the first `#[cfg(test)]` of a file; a test module in a file of its own starts with one.
#![cfg(test)]
//! The check of the raised login (`login_replay.rs`, al-runner issue 8211) on real runs over the stand-in of Microsoft's
//! login (`login_tests::DEP`): sessions of one key hold the same after the login, and an input the key holds apart (the
//! test codeunit, an entry's resource text) changes what the login leaves, so a key without it would hand a session
//! another input's store.

use super::*;
use crate::login_replay::{Check, first_difference};

/// An app under test whose `OnRegisterManualSetup` subscriber (the issue's shape; the event raises the login, `login.rs`
/// `RAISE_ONLY`) registers a row whose id depends on the text of the app's resource `rpl.txt`, and keeps a copy in a temporary record of
/// a SingleInstance codeunit, shared with a second record (`Copy(x, true)`), as Base Application's "Change Log Management"
/// keeps temporary records after the login on BC 28.5.
pub(super) const MAIN: &str = r#"
codeunit 77400 "RPL Registration"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Guided Experience", 'OnRegisterManualSetup', '', false, false)]
    local procedure RegisterManualSetup()
    var G: Codeunit "Guided Experience"; Cache: Codeunit "RPL Cache";
    begin
        G.InsertAssistedSetup(77400 + StrLen(NavApp.GetResourceAsText('rpl.txt')));
        Cache.Keep(77400 + StrLen(NavApp.GetResourceAsText('rpl.txt')));
    end;
}
codeunit 77405 "RPL Cache"
{
    SingleInstance = true;
    var Temp: Record "Guided Experience Item" temporary; Shared: Record "Guided Experience Item" temporary;
        Helper: Codeunit "RPL Helper"; Other: Codeunit "RPL Helper"; Same: Codeunit "RPL Helper";
    procedure Keep(Id: Integer)
    begin
        Temp."Object ID" := Id; Temp.Insert(); Shared.Copy(Temp, true);
        Helper.SetX(Id); Same := Helper; Other.SetX(1);
    end;
}
codeunit 77406 "RPL Helper"
{
    var X: Integer; Buffer: Record "Guided Experience Item" temporary;
    procedure SetX(V: Integer) begin X := V; Buffer."Object ID" := V; Buffer.Insert(); end;
}
codeunit 77402 "RPL Tests A"
{
    Subtype = Test;
    [Test]
    procedure SeesTheRow() var I: Record "Guided Experience Item"; begin if I.IsEmpty() then Error('no row'); end;
}
codeunit 77403 "RPL Tests B"
{
    Subtype = Test;
    [Test]
    procedure SeesTheRow() var I: Record "Guided Experience Item"; begin if I.IsEmpty() then Error('no row'); end;
}
"#;

/// The check line of `engine`'s report: the report's first line (the keys line, `login replay keys: ...`, and the image
/// line, `login image: ...`, follow it; the tests of `login_image_tests.rs` read the image line).
fn check_line(engine: &Engine) -> Option<String> {
    crate::login_replay::report(engine).map(|r| r.lines().next().unwrap_or_default().to_string())
}

/// The compiled apps (the main app has the resource `rpl.txt` holding `a`) and an engine for them.
pub(super) fn compiled() -> (Arc<Engine>, al_sema::lower::Lowered) {
    compiled_with(MAIN)
}

/// [`compiled`] with the app under test's source `main`.
pub(super) fn compiled_with(main: &str) -> (Arc<Engine>, al_sema::lower::Lowered) {
    let (ws, mut l) = al_sema::lower::compile_apps(
        [
            ("main", vec!["dep".to_string()], main),
            ("dep", vec![], super::login_tests::DEP),
        ]
        .into_iter()
        .map(|(id, dependencies, src)| al_sema::workspace::AppSource {
            id: id.into(),
            name: id.into(),
            dependencies,
            files: vec![(format!("{id}.al"), src.into())],
        })
        .collect(),
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        &Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let main = l.program.apps.iter().position(|a| a.name == "main").unwrap();
    Arc::make_mut(&mut l.program).apps[main].resources = vec![("rpl.txt".into(), b"a".to_vec())];
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().map(|r| r.to_string())).collect())
        .collect();
    (Engine::new(l.program.clone(), blocked), l)
}

/// Runs test codeunit `name` once on `engine` with the check on; every test must pass.
pub(super) fn run(engine: &Arc<Engine>, l: &al_sema::lower::Lowered, name: &str) {
    run_with(engine, l, name, RunOptions::default());
}

/// [`run`] with the session flags of `opts` (`SessionFlags::of`).
pub(super) fn run_with(engine: &Arc<Engine>, l: &al_sema::lower::Lowered, name: &str, opts: RunOptions) {
    let o = run_as(
        engine,
        l,
        name,
        RunOptions {
            login_replay_check: true,
            ..opts
        },
    );
    assert_eq!(o.outcomes, vec![("SeesTheRow".to_string(), Outcome::Pass)]);
}

/// What a run of a test codeunit gives: each test's outcome, its error stack and its assumption marks.
#[derive(Debug, PartialEq)]
pub(super) struct Ran {
    pub(super) outcomes: Vec<(String, Outcome)>,
    stacks: Vec<(String, String)>,
    marks: Vec<(String, Vec<Assumption>)>,
}

/// Runs test codeunit `name` once on `engine` under the session flags of `opts`, the check on or off as `opts` says.
pub(super) fn run_as(engine: &Arc<Engine>, l: &al_sema::lower::Lowered, name: &str, opts: RunOptions) -> Ran {
    let mut scratch = Scratch {
        flags: super::session_flags::SessionFlags::of(&opts),
        ..Scratch::new(crate::host::RunClock::begin())
    };
    let oi = l.program.objects.iter().position(|o| o.name == name).unwrap();
    let (mut stacks, mut marks) = (Vec::new(), Vec::new());
    let outcomes = run_codeunit_core(
        engine,
        l,
        oi,
        None,
        TestIsolation::Codeunit,
        None,
        &mut stacks,
        &mut Vec::new(),
        &mut marks,
        &mut scratch,
        None,
        false,
    );
    Ran {
        outcomes,
        stacks,
        marks,
    }
}

/// The text `rpl.txt` of the main app reads as in this pass (a serve entry's `resources`, `Engine::resource_overrides`).
pub(super) fn set_resource(engine: &Engine, l: &al_sema::lower::Lowered, text: Option<&str>) {
    let main = l.program.apps.iter().position(|a| a.name == "main").unwrap();
    let mut o = engine.resource_overrides.write().unwrap();
    o.clear();
    if let Some(t) = text {
        o.insert(main, Arc::new(vec![("rpl.txt".to_string(), t.as_bytes().to_vec())]));
    }
}

#[test]
fn sessions_of_one_key_hold_the_same_after_the_raised_login() {
    let (engine, l) = compiled();
    for _ in 0..3 {
        run(&engine, &l, "RPL Tests A");
    }
    assert_eq!(
        check_line(&engine).as_deref(),
        Some(
            "login replay check: 3 sessions over 1 keys (a key seen before: 2 sessions, 66%), 2 compared, 0 differ; across test codeunits: 0 compared, 0 differ"
        )
    );
    // What the login wrote is in the state compared: the subscriber's row (77400 + 1).
    let firsts = crate::login_replay::check(&engine).firsts();
    assert!(
        firsts[0].1.rows().iter().any(|r| r.holds("Integer(77401)")),
        "{:?}",
        firsts[0].1.rows()
    );
}

#[test]
fn another_test_codeunit_is_another_key() {
    let (engine, l) = compiled();
    run(&engine, &l, "RPL Tests A");
    run(&engine, &l, "RPL Tests B");
    assert_eq!(
        check_line(&engine).as_deref(),
        Some(
            "login replay check: 2 sessions over 2 keys (a key seen before: 0 sessions, 0%), 0 compared, 0 differ; across test codeunits: 1 compared, 0 differ"
        )
    );
}

// Two serve entries with other `resources`: the login reads the text, so what it leaves differs. The key holds the two
// apart; under one key the comparator reports the difference (the planted case).
#[test]
fn an_entrys_resource_text_is_part_of_the_key_and_changes_what_the_login_leaves() {
    let (engine, l) = compiled();
    run(&engine, &l, "RPL Tests A");
    set_resource(&engine, &l, Some("abcd"));
    run(&engine, &l, "RPL Tests A");
    set_resource(&engine, &l, None);
    run(&engine, &l, "RPL Tests A");
    assert_eq!(
        check_line(&engine).as_deref(),
        Some(
            "login replay check: 3 sessions over 2 keys (a key seen before: 1 sessions, 33%), 1 compared, 0 differ; across test codeunits: 0 compared, 0 differ"
        )
    );
    let firsts = crate::login_replay::check(&engine).firsts();
    // Named by where it differs, never by the value (77401 against 77404).
    let difference = first_difference(&firsts[0].1, &firsts[1].1);
    assert!(
        difference
            .as_deref()
            .is_some_and(|d| d.starts_with("row ") && !d.contains("7740")),
        "{difference:?}"
    );
    assert!(firsts[1].1.rows().iter().any(|r| r.holds("Integer(77404)")));
    // The temporary table of the SingleInstance cache holds the other row too.
    let temp_row = |k: usize, id: &str| {
        (firsts[k].1.state_lines().lines.iter()).any(|l| l.starts_with("temp table 0 row ") && l.contains(id))
    };
    assert!(temp_row(0, "Integer(77401)") && temp_row(1, "Integer(77404)"));
    let planted = Check::default();
    planted.note(firsts[0].0.clone(), firsts[0].1.clone());
    planted.note(firsts[0].0.clone(), firsts[1].1.clone());
    assert!(planted.line().is_some_and(|l| l.contains("1 compared, 1 differ")));
}

// What the login leaves in a SingleInstance codeunit's temporary records ("Change Log Management" on BC 28.5) compares by
// the table's rows and everything each record carries: equal sessions are equal, not "not comparable"; two records that
// share one table (`Copy(x, true)`) name it once, as one table.
#[test]
fn temporary_records_the_login_leaves_compare_by_their_rows_and_view_and_a_shared_table_is_one_table() {
    let (engine, l) = compiled();
    run(&engine, &l, "RPL Tests A");
    run(&engine, &l, "RPL Tests A");
    let line = crate::login_replay::report(&engine).unwrap();
    assert!(line.contains(", 1 compared, 0 differ;"), "{line}");
    let firsts = crate::login_replay::check(&engine).firsts();
    let state = firsts[0].1.state_lines();
    assert!(state.opaque.is_empty(), "{:?}", state.opaque);
    let rows: Vec<&String> = state
        .lines
        .iter()
        .filter(|l| l.starts_with("temp table 0 row "))
        .collect();
    assert_eq!(rows.len(), 1, "{:?}", state.lines);
    assert!(rows[0].contains("Integer(77401)"), "{rows:?}");
    let records = state
        .lines
        .iter()
        .filter(|l| l.contains(" Record temp table 0: "))
        .count();
    assert_eq!(records, 2, "{:?}", state.lines);
}

// A serve request's session flags and license reach the login code (a permission error, an unhandled UI call) while the
// engine and its check outlive the request: on one engine, a session with another flag or another license has a key of
// its own, it is not compared with the sessions before.
#[test]
fn another_session_flag_or_license_is_another_key() {
    let (mut engine, l) = compiled();
    run(&engine, &l, "RPL Tests A");
    let flag = |o: RunOptions| run_with(&engine, &l, "RPL Tests A", o);
    flag(RunOptions {
        ignore_permission_mock: true,
        ..Default::default()
    });
    flag(RunOptions {
        dev_endpoint_ui: true,
        ..Default::default()
    });
    let everything: &[(i64, i64, u32)] = &[(0, 2_000_000_000, 0b1_1111)];
    let license = al_host::license::parse(&al_host::license::synthetic::file(
        "X",
        "1/1/2040",
        &[
            ("TableData", everything),
            ("Codeunit", everything),
            ("Page", everything),
        ],
    ))
    .unwrap();
    Arc::get_mut(&mut engine).expect("no session holds the engine").license = Some(Arc::new(license));
    run(&engine, &l, "RPL Tests A");
    run(&engine, &l, "RPL Tests A");
    assert_eq!(
        check_line(&engine).as_deref(),
        Some(
            "login replay check: 5 sessions over 4 keys (a key seen before: 1 sessions, 20%), 1 compared, 0 differ; \
             across test codeunits: 0 compared, 0 differ"
        )
    );
}

// Codeunit instances a SingleInstance codeunit holds after the login (on a real app one global of a SingleInstance
// codeunit holds one): compared by identity (one number per instance, by first sight: two globals that hold one
// instance name the same number, another instance another) and by their own state (the codeunit and its globals).
#[test]
fn codeunit_instances_the_login_leaves_compare_by_identity_and_their_state() {
    let (engine, l) = compiled();
    run(&engine, &l, "RPL Tests A");
    run(&engine, &l, "RPL Tests A");
    let line = crate::login_replay::report(&engine).unwrap();
    assert!(line.contains(", 1 compared, 0 differ"), "{line}");
    let firsts = crate::login_replay::check(&engine).firsts();
    let state = firsts[0].1.state_lines();
    assert!(state.opaque.is_empty(), "{:?}", state.opaque);
    let global = |g: usize| {
        let prefix = format!("single 77405 global {g} ");
        let line = state.lines.iter().find(|l| l.starts_with(&prefix));
        line.unwrap_or_else(|| panic!("{:?}", state.lines)).clone()
    };
    // Helper and Same hold one instance, Other another.
    assert!(global(2).ends_with(" instance 0"), "{}", global(2));
    assert!(global(4).ends_with(" instance 0"), "{}", global(4));
    assert!(global(3).ends_with(" instance 1"), "{}", global(3));
    for n in 0..2 {
        let own = format!("instance {n} codeunit 77406 globals [");
        assert!(state.lines.iter().any(|l| l.starts_with(&own)), "{:?}", state.lines);
    }
    // The resource text the login read reaches the instance's own global: another text, another instance state.
    set_resource(&engine, &l, Some("abcd"));
    run(&engine, &l, "RPL Tests A");
    let firsts = crate::login_replay::check(&engine).firsts();
    let own = |k: usize| {
        let lines = &firsts[k].1.state_lines().lines;
        lines
            .iter()
            .find(|l| l.starts_with("instance 0 codeunit 77406"))
            .cloned()
            .unwrap()
    };
    assert_ne!(own(0), own(1));
}

/// [`MAIN`] with a second login subscriber that leaves, in a SingleInstance codeunit, an instance whose own global holds
/// an HttpClient with a base address.
pub(super) const OPAQUE_INSIDE: &str = r#"
codeunit 77410 "RPL Deep Registration"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Guided Experience", 'OnRegisterManualSetup', '', false, false)]
    local procedure RegisterManualSetup() var Holder: Codeunit "RPL Holder"; begin Holder.Keep(); end;
}
codeunit 77411 "RPL Holder"
{
    SingleInstance = true;
    var Inner: Codeunit "RPL Ref Holder";
    procedure Keep() begin Inner.Open(); end;
}
codeunit 77412 "RPL Ref Holder"
{
    var Count: Integer; Client: HttpClient;
    procedure Open() begin Count := 1; Client.SetBaseAddress('http://localhost/'); end;
}
"#;

// A value with no comparable form inside a codeunit instance is reported by the SingleInstance global that holds it (the
// path before the first `: `); what follows (the instances inside and the kind of value, which can describe a value) is
// not printed (login_replay/difference.rs).
#[test]
fn a_value_with_no_comparable_form_inside_an_instance_is_named_with_its_path() {
    let (engine, l) = compiled_with(&format!("{MAIN}{OPAQUE_INSIDE}"));
    run(&engine, &l, "RPL Tests A");
    run(&engine, &l, "RPL Tests A");
    let line = crate::login_replay::report(&engine).unwrap();
    assert!(
        line.contains("first: not comparable: codeunit 77411 global 0); across"),
        "{line}"
    );
}

/// [`MAIN`] with a login subscriber that leaves, in a SingleInstance codeunit, an instance holding an array of plain
/// values (one element from the resource text) and an array of codeunit instances (each holding a temporary record).
const ARRAYS_INSIDE: &str = r#"
codeunit 77420 "RPL Array Registration"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Guided Experience", 'OnRegisterManualSetup', '', false, false)]
    local procedure RegisterManualSetup() var Holder: Codeunit "RPL Array Holder";
    begin Holder.Keep(StrLen(NavApp.GetResourceAsText('rpl.txt'))); end;
}
codeunit 77421 "RPL Array Holder"
{
    SingleInstance = true;
    var Inner: Codeunit "RPL Array Inner";
    procedure Keep(N: Integer) begin Inner.Fill(N); end;
}
codeunit 77422 "RPL Array Inner"
{
    var Values: array[5] of Integer; Helpers: array[2] of Codeunit "RPL Helper";
    procedure Fill(N: Integer) begin Values[1] := 7; Values[2] := 7; Values[3] := N; Helpers[1].SetX(N); end;
}
"#;

// An array a login leaves inside an instance compares element by element (plain values by value, instances by number,
// runs of equal elements collapsed): equal sessions are equal, and another element is a session state difference that
// names the array.
#[test]
fn arrays_the_login_leaves_compare_element_by_element() {
    let (engine, l) = compiled_with(&format!("{MAIN}{ARRAYS_INSIDE}"));
    run(&engine, &l, "RPL Tests A");
    run(&engine, &l, "RPL Tests A");
    let line = crate::login_replay::report(&engine).unwrap();
    assert!(line.contains(", 1 compared, 0 differ"), "{line}");
    set_resource(&engine, &l, Some("abcd"));
    run(&engine, &l, "RPL Tests A");
    let firsts = crate::login_replay::check(&engine).firsts();
    let differences = crate::login_replay::differences(&firsts[0].1, &firsts[1].1);
    assert!(differences.iter().any(|(kind, _)| *kind == 2), "{differences:?}");
    // The instance of "RPL Array Inner" in each session: element [2] differs (the resource's length), the rest is equal
    // and its equal runs are collapsed.
    let inner = |k: usize| {
        let lines = &firsts[k].1.state_lines().lines;
        let line = lines
            .iter()
            .find(|l| l.starts_with("instance ") && l.contains(" codeunit 77422 "));
        line.unwrap_or_else(|| panic!("{lines:?}")).clone()
    };
    let (a, b) = (inner(0), inner(1));
    assert!(a.contains("array of 5 elements [[0..1]: "), "{a}");
    assert!(a.contains(", [2]: ") && a.contains(", [3..4]: "), "{a}");
    assert_ne!(a, b);
}

/// [`MAIN`] with a login subscriber that leaves, in a SingleInstance codeunit, one instance per kind of value, each filled
/// from the resource text's length `N`: RecordRefs never opened (one alone, an array of three, one opened when `N` > 1), a
/// RecordRef on a temporary table with the record it was taken from, a RecordRef on a real table with a FieldRef that
/// filters it and a KeyRef, a List and a Dictionary of codeunit instances, Json, a TextBuilder, Xml, a Notification, an
/// ErrorInfo, a Version and an HttpContent.
pub(super) const KINDS_INSIDE: &str = r#"
codeunit 77430 "RPL Kinds Registration"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Guided Experience", 'OnRegisterManualSetup', '', false, false)]
    local procedure RegisterManualSetup() var Holder: Codeunit "RPL Kinds Holder";
    begin Holder.Keep(StrLen(NavApp.GetResourceAsText('rpl.txt'))); end;
}
codeunit 77431 "RPL Kinds Holder"
{
    SingleInstance = true;
    var Closed: Codeunit "RPL K Closed"; TempRef: Codeunit "RPL K TempRef"; RealRef: Codeunit "RPL K RealRef";
        Lists: Codeunit "RPL K List"; Dict: Codeunit "RPL K Dict"; Json: Codeunit "RPL K Json"; Text: Codeunit "RPL K Text";
        Xml: Codeunit "RPL K Xml"; Misc: Codeunit "RPL K Misc"; Http: Codeunit "RPL K Http";
    procedure Keep(N: Integer)
    begin
        Closed.Fill(N); TempRef.Fill(N); RealRef.Fill(N); Lists.Fill(N); Dict.Fill(N); Json.Fill(N); Text.Fill(N);
        Xml.Fill(N); Misc.Fill(N); Http.Fill(N);
    end;
}
codeunit 77432 "RPL K Closed"
{
    var Never: RecordRef; Refs: array[3] of RecordRef;
    procedure Fill(N: Integer) begin if N > 1 then Refs[2].Open(Database::"Guided Experience Item"); end;
}
codeunit 77433 "RPL K TempRef"
{
    var T: Record "Guided Experience Item" temporary; R: RecordRef;
    procedure Fill(N: Integer) begin T."Object ID" := N; T.Insert(); R.GetTable(T); if R.FindFirst() then; end;
}
codeunit 77434 "RPL K RealRef"
{
    var R: RecordRef; F: FieldRef; K: KeyRef;
    procedure Fill(N: Integer)
    begin
        R.Open(Database::"Guided Experience Item"); F := R.Field(1); F.SetRange(77400 + N); K := R.KeyIndex(1);
        if R.FindFirst() then;
    end;
}
codeunit 77435 "RPL K List"
{
    var L: List of [Codeunit "RPL Helper"]; Names: List of [Text];
    procedure Fill(N: Integer) var H: Codeunit "RPL Helper"; begin H.SetX(N); L.Add(H); L.Add(H); Names.Add(Format(N)); end;
}
codeunit 77436 "RPL K Dict"
{
    var D: Dictionary of [Integer, Codeunit "RPL Helper"];
    procedure Fill(N: Integer) var H: Codeunit "RPL Helper"; begin H.SetX(N); D.Add(N, H); end;
}
codeunit 77437 "RPL K Json"
{
    var J: JsonObject; A: JsonArray;
    procedure Fill(N: Integer) begin A.Add(N); J.Add('n', N); J.Add('a', A); end;
}
codeunit 77438 "RPL K Text"
{
    var B: TextBuilder;
    procedure Fill(N: Integer) begin B.Append(Format(N)); end;
}
codeunit 77439 "RPL K Xml"
{
    var X: XmlDocument;
    procedure Fill(N: Integer) begin XmlDocument.ReadFrom('<a>' + Format(N) + '</a>', X); end;
}
codeunit 77440 "RPL K Misc"
{
    var Notif: Notification; Info: ErrorInfo; V: Version;
    procedure Fill(N: Integer) begin Notif.Message(Format(N)); Info.Message(Format(N)); V := Version.Create(N, 0); end;
}
codeunit 77441 "RPL K Http"
{
    var C: HttpContent;
    procedure Fill(N: Integer) begin C.WriteFrom(Format(N)); end;
}
"#;

// Every kind of value a login can leave inside an instance has an exact form: sessions of one input are equal, not "not
// comparable", and another input is a session state difference in the instance of each kind.
#[test]
fn every_kind_of_value_the_login_leaves_compares_exactly() {
    let (engine, l) = compiled_with(&format!("{MAIN}{KINDS_INSIDE}"));
    run(&engine, &l, "RPL Tests A");
    run(&engine, &l, "RPL Tests A");
    let line = crate::login_replay::report(&engine).unwrap();
    assert!(line.contains(", 1 compared, 0 differ;"), "{line}");
    let firsts = crate::login_replay::check(&engine).firsts();
    let state = firsts[0].1.state_lines();
    assert!(state.opaque.is_empty(), "{:?}", state.opaque);
    // The line of the instance of codeunit `id`.
    fn of(firsts: &[(crate::login_replay::Key, crate::login_replay::LoginState)], k: usize, id: u32) -> String {
        let lines = &firsts[k].1.state_lines().lines;
        let line =
            (lines.iter()).find(|l| l.starts_with("instance ") && l.contains(&format!(" codeunit {id} globals")));
        line.unwrap_or_else(|| panic!("{id}: {lines:?}")).clone()
    }
    // The instance line of codeunit `id` with the lines of the shared handles it names (`shared <n>`).
    fn held(firsts: &[(crate::login_replay::Key, crate::login_replay::LoginState)], k: usize, id: u32) -> String {
        let line = of(firsts, k, id);
        let shared = (firsts[k].1.state_lines().lines.iter()).filter(|l| {
            let n = l.strip_prefix("shared ").and_then(|t| t.split(' ').next());
            n.is_some_and(|n| line.contains(&format!("shared {n},")) || line.contains(&format!("shared {n}]")))
        });
        shared.fold(line.clone(), |all, l| format!("{all}\n{l}"))
    }
    // RecordRefs never opened render as that, and an array of them collapses into one run.
    let closed = of(&firsts, 0, 77432);
    assert!(
        closed.contains("[RecordRef (not open), array of 3 elements [[0..2]: RecordRef (not open)]]"),
        "{closed}"
    );
    // A RecordRef taken from a temporary record names the record's table.
    let temp = of(&firsts, 0, 77433);
    let table = (temp.split("Record temp table ").nth(1)).and_then(|t| t.split(':').next());
    assert!(
        table.is_some_and(|n| temp.contains(&format!(", RecordRef temp table {n}: "))),
        "{temp}"
    );
    set_resource(&engine, &l, Some("abcd"));
    run(&engine, &l, "RPL Tests A");
    let firsts = crate::login_replay::check(&engine).firsts();
    let differences = crate::login_replay::differences(&firsts[0].1, &firsts[1].1);
    assert!(differences.iter().all(|(kind, _)| *kind != 0), "{differences:?}");
    assert!(differences.iter().any(|(kind, _)| *kind == 2), "{differences:?}");
    for id in 77432..=77441 {
        assert_ne!(held(&firsts, 0, id), held(&firsts, 1, id), "{id}");
    }
}

// A backing that does not say who it is (`Backing::identity` `None`, the trait's default) may offer other rows than
// another such backing: if the two gave one key, a replay could hand one session the store another backing's login made.
// Each session over such a backing is a key of its own (maintainer's review of al-runner PR 8447).
#[test]
fn two_backings_that_do_not_say_who_they_are_are_never_one_key() {
    struct Unnamed;
    impl al_data::memory::Backing for Unnamed {
        fn company(&self) -> &str {
            ""
        }
        fn rows(&self, _: &str, _: u32) -> Option<al_data::memory::Rows> {
            None
        }
    }
    let (engine, _) = compiled();
    let key = || {
        let mut s = Session::new(engine.clone());
        let store = std::mem::take(s.store_mut());
        *s.store_mut() = store.with_backing(std::rc::Rc::new(Unnamed));
        // At rest, as a test session is before its login: SQL counters not at rest have no comparable form (no key).
        s.commit_transaction();
        crate::login_replay::Key::of(&mut s, (TestIsolation::Codeunit, false), Default::default(), &[]).expect("a key")
    };
    assert_ne!(key(), key());
}

// The check measures only: the same codeunit, with the switch off and on, gives the same outcomes, error stacks and
// assumption marks (maintainer's review of al-runner PR 8447). Two shapes: the raised login, which the check compares
// (only the run with the switch on reports a line), with a passing and a failing test; and the apps under test's own
// login alone (`Login::Own`), which the check keys and does not compare, whose tests carry the mark
// `login-own-registration-reads-alone` (`login_tests.rs` shows the mark on this shape).
const FAILS_AFTER_LOGIN: &str = r#"
codeunit 77404 "RPL Tests C"
{
    Subtype = Test;
    [Test]
    procedure SeesTheRow() var I: Record "Guided Experience Item"; begin if I.IsEmpty() then Error('no row'); end;
    [Test]
    procedure FailsOnTheRow() var I: Record "Guided Experience Item"; begin if not I.IsEmpty() then Error('a row'); end;
}
"#;

pub(super) const OWN_LOGIN: &str = r#"
codeunit 77408 "RPL Own Registration"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Guided Experience", 'OnRegisterAssistedSetup', '', false, false)]
    local procedure RegisterAssistedSetup()
    var G: Codeunit "Guided Experience";
    begin
        G.InsertAssistedSetup(77408);
    end;
}
codeunit 77409 "RPL Own Tests"
{
    Subtype = Test;
    [Test]
    procedure ReadsTheRowOfTheLogin()
    var G: Codeunit "Guided Experience";
    begin
        if not G.AssistedSetupExistsAndIsNotComplete(77408) then Error('no row');
    end;
    [Test]
    procedure InsertsTheKeyOfTheLoginRow()
    var G: Codeunit "Guided Experience";
    begin
        G.InsertAssistedSetup(77408);
    end;
}
"#;

#[test]
fn the_check_off_and_on_gives_the_same_outcomes_and_marks() {
    let raised = format!("{MAIN}{FAILS_AFTER_LOGIN}");
    // Two runs of codeunit `name` of app `main` on one engine, with the check on or off, and the check's line.
    let ran = |main: &str, name: &str, check: bool| {
        let (engine, l) = compiled_with(main);
        let opts = RunOptions {
            login_replay_check: check,
            ..Default::default()
        };
        let ran = (0..2)
            .map(|_| run_as(&engine, &l, name, opts.clone()))
            .collect::<Vec<_>>();
        (ran, crate::login_replay::report(&engine))
    };
    let (off, off_line) = ran(&raised, "RPL Tests C", false);
    let (on, on_line) = ran(&raised, "RPL Tests C", true);
    assert_eq!(off, on);
    let passed: Vec<_> = (off[0].outcomes.iter())
        .map(|(t, o)| (t.as_str(), matches!(o, Outcome::Pass)))
        .collect();
    assert_eq!(passed, [("SeesTheRow", true), ("FailsOnTheRow", false)], "{:?}", off[0]);
    assert_eq!(off_line, None);
    assert!(on_line.is_some_and(|l| l.starts_with("login replay check: 2 sessions over 1 keys")));

    let (off, _) = ran(OWN_LOGIN, "RPL Own Tests", false);
    let (on, on_line) = ran(OWN_LOGIN, "RPL Own Tests", true);
    assert_eq!(off, on);
    assert_eq!(on_line, None, "the own login alone is not raised: nothing compared");
    let marked = al_builtins::assumptions::by_id("login-own-registration-reads-alone").unwrap();
    assert!(
        off[0].marks.len() == 2 && off[0].marks.iter().all(|(_, m)| m.contains(&marked)),
        "{:?}",
        off[0]
    );
}

/// The key of a new session on `engine` that holds, uncommitted, a "Guided Experience Item" row with id `id`, and the
/// paths of its values with no comparable form.
fn key_with_uncommitted_row(engine: &Arc<Engine>, id: i32) -> (Option<crate::login_replay::Key>, Vec<String>) {
    use al_data::Store;
    let mut session = al_vm::Session::new(engine.clone());
    session.commit_transaction();
    let at = engine
        .program
        .objects
        .iter()
        .position(|o| o.name == "Guided Experience Item")
        .unwrap();
    let t = engine.tables[at].clone().unwrap();
    let mut row: Vec<al_builtins::value::Value> = (t.fields.iter())
        .map(|f| al_builtins::value::Value::default_for(&f.ty))
        .collect();
    row[0] = al_builtins::value::Value::Integer(id);
    let cx = al_data::AccessContext {
        company: session.access_company().into(),
        user: session.user_id().into(),
    };
    assert!(session.store_mut().insert(&cx, &t, row).is_ok());
    assert!(session.store_mut().has_uncommitted());
    let flags = super::session_flags::SessionFlags::of(&RunOptions::default());
    let opaque = session.state_lines().opaque.iter().map(|o| o.path.clone()).collect();
    (
        crate::login_replay::Key::of(&mut session, Default::default(), flags, &[]).ok(),
        opaque,
    )
}

// The rows a session holds uncommitted before the login are an input of the login (it reads them). A write since the
// last Commit leaves the SQL counters with no comparable form (al-vm `state_facts.rs`), so such a session has no key
// and runs its login: two sessions whose uncommitted rows differ never share one.
#[test]
fn a_session_with_uncommitted_rows_before_the_login_has_no_key() {
    let (engine, _) = compiled();
    for id in [1, 2] {
        let (key, opaque) = key_with_uncommitted_row(&engine, id);
        assert_eq!(opaque, ["SQL counters"], "GUARD: the counters are why");
        assert_eq!(key, None);
    }
}

/// A raised login whose subscriber registers one fixed row and calls `Plant` of codeunit "RPL Gap" (77462) with the length
/// of the resource text: each test of a state the check must compare adds an "RPL Gap" that plants that state.
pub(super) const GAP: &str = r#"
codeunit 77460 "RPL Gap Registration"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Guided Experience", 'OnRegisterManualSetup', '', false, false)]
    local procedure RegisterManualSetup() var G: Codeunit "Guided Experience"; Gap: Codeunit "RPL Gap";
    begin G.InsertAssistedSetup(77460); Gap.Plant(StrLen(NavApp.GetResourceAsText('rpl.txt'))); end;
}
codeunit 77461 "RPL Gap Tests"
{
    Subtype = Test;
    [Test]
    procedure SeesTheRow() var I: Record "Guided Experience Item"; begin if I.IsEmpty() then Error('no row'); end;
}
"#;

/// A planted session: the check's line, the differences, and the first session state line of the base that differs from
/// the planted session's (the difference text names only its place).
type Planted = (String, Vec<(usize, String)>, Option<String>);

/// Runs the login of [`GAP`] with `plant` twice with the resource text `base` and once with each text of `others`. Asserts
/// the two sessions of `base` compare equal; returns, per text of `others`, the check's line when its session is noted
/// under the key of `base` (a planted difference: what a key that missed the text would compare), the differences and
/// the first base state line that differs.
fn planted(plant: &str, base: &str, others: &[&str]) -> Vec<Planted> {
    let (engine, l) = compiled_with(&format!("{GAP}{plant}"));
    for text in std::iter::repeat_n(base, 2).chain(others.iter().copied()) {
        set_resource(&engine, &l, Some(text));
        run(&engine, &l, "RPL Gap Tests");
    }
    let line = crate::login_replay::report(&engine).unwrap();
    assert!(line.contains(", 1 compared, 0 differ;"), "{line}");
    let firsts = crate::login_replay::check(&engine).firsts();
    assert_eq!(firsts.len(), 1 + others.len());
    (firsts[1..].iter())
        .map(|(_, state)| {
            let check = Check::default();
            check.note(firsts[0].0.clone(), firsts[0].1.clone());
            check.note(firsts[0].0.clone(), state.clone());
            let line = check.line().unwrap();
            let (base, other) = (&firsts[0].1.state_lines().lines, &state.state_lines().lines);
            let first = (base.iter().zip(other)).find(|(a, b)| a != b).map(|(a, _)| a.clone());
            (line, crate::login_replay::differences(&firsts[0].1, state), first)
        })
        .collect()
}

/// Asserts that each planted session differs from the base in the session state only, that the difference names the
/// first differing line by its place, and that the line starts with `what`.
fn assert_planted_state(planted: &[Planted], what: &str) {
    for (line, differences, first) in planted {
        assert!(
            line.contains("1 compared, 1 differ (not comparable 0, rows 0, session state 1, marks 0, open writes 0;"),
            "{line}"
        );
        let state = differences.iter().find(|(kind, _)| *kind == 2);
        assert!(
            state.is_some_and(|(_, d)| d.starts_with("session state ") && !d.contains(':')),
            "{differences:?}"
        );
        assert!(first.as_deref().is_some_and(|l| l.starts_with(what)), "{first:?}");
    }
}

/// "RPL Gap" holds an instance it may call, and binds one of two instances of a manual subscriber (it calls both, so both
/// are initialized): text length 1 binds the first, 2 also calls the held instance (it is then initialized), 3 binds the
/// second.
pub(super) const GAP_INSTANCES: &str = r#"
codeunit 77462 "RPL Gap"
{
    SingleInstance = true;
    var Inner: Codeunit "RPL Gap Inner"; A: Codeunit "RPL Gap Bound"; B: Codeunit "RPL Gap Bound";
    procedure Plant(N: Integer)
    begin
        A.Touch(); B.Touch();
        if N = 3 then BindSubscription(B) else BindSubscription(A);
        if N = 2 then Inner.Touch();
    end;
}
codeunit 77463 "RPL Gap Inner" { procedure Touch() begin end; }
codeunit 77464 "RPL Gap Bound"
{
    EventSubscriberInstance = Manual;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Guided Experience", 'OnRegisterAssistedSetup', '', false, false)]
    local procedure OnAssisted() begin end;
    procedure Touch() begin end;
}
"#;

// Whether a held instance was initialized, and which instance a manual binding holds (not only its codeunit), are
// compared.
#[test]
fn an_instance_s_own_facts_and_the_instance_a_binding_holds_are_compared() {
    let planted = planted(GAP_INSTANCES, "a", &["ab", "abc"]);
    assert_planted_state(&planted[..1], "instance ");
    assert_planted_state(&planted[1..], "bindings ");
}

/// "RPL Gap" writes a row of its table; its `GetDatabaseTableTriggerSetup` subscriber enables OnDatabaseModify for the
/// table when the text is longer than 1 (no Modify follows: only the setup BC asked differs).
pub(super) const GAP_GLOBAL_SETUPS: &str = r#"
table 77465 "RPL Gap Row" { fields { field(1; K; Integer) { } } keys { key(PK; K) { Clustered = true; } } }
codeunit 77462 "RPL Gap"
{
    procedure Plant(N: Integer) var R: Record "RPL Gap Row"; begin R.K := 1; R.Insert(); end;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'GetDatabaseTableTriggerSetup', '', false, false)]
    local procedure Setup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean)
    begin
        if TableId = Database::"RPL Gap Row" then OnDatabaseModify := StrLen(NavApp.GetResourceAsText('rpl.txt')) > 1;
    end;
}
"#;

// The Global Triggers setup BC asked per table (a later first write asks only for a table not asked yet) is compared.
#[test]
fn the_global_trigger_setups_asked_are_compared() {
    assert_planted_state(&planted(GAP_GLOBAL_SETUPS, "a", &["ab"]), "global setups ");
}

/// "RPL Gap" raises its own event once per character of the text; a subscriber handles it.
pub(super) const GAP_EVENT_CALLS: &str = r#"
codeunit 77462 "RPL Gap"
{
    procedure Plant(N: Integer) var I: Integer; begin for I := 1 to N do OnGap(); end;
    [IntegrationEvent(false, false)] local procedure OnGap() begin end;
}
codeunit 77467 "RPL Gap Listener"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"RPL Gap", 'OnGap', '', false, false)]
    local procedure Heard() begin end;
}
"#;

// How often each subscriber was called (the Event Subscription table's "Number of Calls") is compared.
#[test]
fn the_event_subscriber_call_counts_are_compared() {
    assert_planted_state(&planted(GAP_EVENT_CALLS, "a", &["ab"]), "event calls ");
}

/// "RPL Gap" imports the text's length, as text, into the Media field of a temporary record: the record is gone after,
/// the media object stays in the session's media store.
pub(super) const GAP_MEDIA: &str = r#"
table 77466 "RPL Gap Pic"
{
    fields { field(1; K; Integer) { } field(2; B; Blob) { } field(3; Pic; Media) { } }
    keys { key(PK; K) { Clustered = true; } }
}
codeunit 77462 "RPL Gap"
{
    procedure Plant(N: Integer)
    var T: Record "RPL Gap Pic" temporary; O: OutStream; I: InStream;
    begin
        T.B.CreateOutStream(O); O.WriteText(Format(N)); T.B.CreateInStream(I); T.Pic.ImportStream(I, 'x');
    end;
}
"#;

// The session's media objects (a row holds only the media id; the content lives in the session) are compared.
#[test]
fn the_media_the_login_imports_are_compared() {
    assert_planted_state(&planted(GAP_MEDIA, "a", &["ab"]), "media ");
}

/// "RPL Gap" seeds `Random` with the text's length: no number is drawn, the sequence differs.
pub(super) const GAP_RANDOM: &str = r#"
codeunit 77462 "RPL Gap"
{
    procedure Plant(N: Integer) begin Randomize(N); end;
}
"#;

// The state of the session's `Random` sequence (not only how many numbers it drew) is compared.
#[test]
fn the_random_sequence_the_login_leaves_is_compared() {
    assert_planted_state(&planted(GAP_RANDOM, "a", &["ab"]), "random ");
}

/// "RPL Gap" looks for a row of its empty table when the text is longer than 1: a find that found nothing while the SQL
/// counters were inactive is kept for a later read of `SessionInformation.SqlStatementsExecuted`.
pub(super) const GAP_SQL: &str = r#"
table 77465 "RPL Gap Row" { fields { field(1; K; Integer) { } } keys { key(PK; K) { Clustered = true; } } }
codeunit 77462 "RPL Gap"
{
    procedure Plant(N: Integer) var R: Record "RPL Gap Row"; begin if N > 1 then if R.FindFirst() then; end;
}
"#;

// What the SQL counters keep while inactive (a later counter read reads it) is compared.
#[test]
fn the_sql_counters_the_login_leaves_are_compared() {
    assert_planted_state(&planted(GAP_SQL, "a", &["ab"]), "sql ");
}

/// "RPL Gap" reads `SessionInformation.SqlStatementsExecuted`: the counters log from then on.
const GAP_SQL_READ: &str = r#"
codeunit 77462 "RPL Gap"
{
    procedure Plant(N: Integer) var C: BigInteger; begin C := SessionInformation.SqlStatementsExecuted(); end;
}
"#;

// SQL counters a login read have no comparable form: what a later read counts depends on the session's own log.
#[test]
fn sql_counters_the_login_read_are_not_comparable() {
    let (engine, l) = compiled_with(&format!("{GAP}{GAP_SQL_READ}"));
    for _ in 0..2 {
        set_resource(&engine, &l, Some("a"));
        run(&engine, &l, "RPL Gap Tests");
    }
    let line = crate::login_replay::report(&engine).unwrap();
    assert!(
        line.contains("1 differ (not comparable 1, ") && line.contains("first: not comparable: SQL counters)"),
        "{line}"
    );
}

/// "RPL Gap" commits when the text is longer than 1 (it wrote nothing: only the session's count of commits moves).
pub(super) const GAP_COMMIT: &str = r#"
codeunit 77462 "RPL Gap"
{
    procedure Plant(N: Integer) begin if N > 1 then Commit(); end;
}
"#;

// How often the session committed and the facts of its last commit (a later test reads them) are compared.
#[test]
fn the_commits_and_transaction_facts_the_login_leaves_are_compared() {
    assert_planted_state(&planted(GAP_COMMIT, "a", &["ab"]), "transaction ");
}

/// A Company Triggers subscriber of the app under test inserts and deletes a row when the text is longer than 1: the rows
/// are the same after, the table it wrote is noted for the tests of the codeunit (`install::CompanyOpenWrites`).
const GAP_OPEN_WRITES: &str = r#"
table 77465 "RPL Gap Row" { fields { field(1; K; Integer) { } } keys { key(PK; K) { Clustered = true; } } }
codeunit 77462 "RPL Gap"
{
    procedure Plant(N: Integer) begin end;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Company Triggers", 'OnCompanyOpenCompleted', '', false, false)]
    local procedure Opened() var R: Record "RPL Gap Row";
    begin
        if StrLen(NavApp.GetResourceAsText('rpl.txt')) > 1 then begin R.K := 1; R.Insert(); R.Delete(); end;
    end;
}
"#;

// What the company open noted for the tests of the codeunit (the tables its subscribers wrote) is compared, as a kind
// of its own.
#[test]
fn what_the_company_open_noted_for_the_tests_is_compared() {
    for (line, differences, _) in planted(GAP_OPEN_WRITES, "a", &["ab"]) {
        assert!(line.contains(", open writes 1;"), "{line}");
        assert!(
            differences
                .iter()
                .any(|(kind, d)| (*kind, d.as_str()) == (4, "open writes")),
            "{differences:?}"
        );
    }
}
