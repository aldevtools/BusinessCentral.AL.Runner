//! The rule of `--affected` on hand-made stored results (ADR-050 test plan, the parts that need no compile).

use super::*;

const APP: &str = "a1";

fn header() -> Header {
    Header {
        runtime: "build-1".into(),
        options: "opts".into(),
        environment: vec!["base 27.0.0.0 h1".into(), "bc 27.0".into()],
        apps: vec![(APP.into(), "1.0.0.0".into())],
        manifests: vec!["m1".into()],
    }
}

fn obj(kind: ObjKind, id: u32, hash: &str) -> ObjectFacts {
    let uses = match kind {
        ObjKind::Table => vec![format!("table {id}")],
        ObjKind::Page => vec![format!("page {id}")],
        ObjKind::Enum => vec![format!("enum {id}")],
        _ => Vec::new(),
    };
    ObjectFacts {
        app: APP.into(),
        kind,
        id: Some(id),
        name: format!("O{id}"),
        hash: hash.into(),
        uses,
        subscribes: Vec::new(),
        global_triggers: false,
        install: false,
        test: false,
        refers: Vec::new(),
        unfollowed: Vec::new(),
    }
}

fn test_cu(id: u32, hash: &str) -> ObjectFacts {
    ObjectFacts {
        test: true,
        ..obj(ObjKind::Codeunit, id, hash)
    }
}

fn key(kind: ObjKind, id: u32) -> String {
    format!("{APP}/{} {id}", kind.label())
}

fn stored_cu(id: u32, keys: &[&str], entered: &[String]) -> StoredCodeunit {
    StoredCodeunit {
        app: APP.into(),
        id,
        name: format!("T{id}"),
        keys: keys.iter().map(|k| k.to_string()).collect(),
        entered: entered.to_vec(),
        dotnet: false,
        stubbed: false,
        ended: Ended::Normal,
        tests: vec![("Test1".into(), Outcome::Pass)],
    }
}

/// A workspace of one codeunit 50000, a table 50001, an enum 50002 and the test codeunits 60000 (enters 50000),
/// 60001 (uses table 50001) and 60002 (raises an event of codeunit 50000).
fn base() -> (Stored, Vec<ObjectFacts>) {
    let objects = vec![
        obj(ObjKind::Codeunit, 50000, "c0"),
        obj(ObjKind::Table, 50001, "t1"),
        obj(ObjKind::Enum, 50002, "e2"),
        test_cu(60000, "x0"),
        test_cu(60001, "x1"),
        test_cu(60002, "x2"),
    ];
    let codeunits = vec![
        stored_cu(
            60000,
            &[],
            &[key(ObjKind::Codeunit, 50000), key(ObjKind::Codeunit, 60000)],
        ),
        stored_cu(60001, &["table 50001", "write 50001"], &[key(ObjKind::Codeunit, 60001)]),
        stored_cu(60002, &["event codeunit 50000 onfoo"], &[key(ObjKind::Codeunit, 60002)]),
    ];
    (
        Stored {
            header: header(),
            objects: objects.clone(),
            codeunits,
        },
        objects,
    )
}

fn all() -> Vec<(String, u32)> {
    [60000, 60001, 60002].iter().map(|&i| (APP.to_string(), i)).collect()
}

fn select(stored: &Stored, objects: &[ObjectFacts]) -> Selection {
    select_with(stored, objects, &all(), false, &[])
}

fn select_with(
    stored: &Stored,
    objects: &[ObjectFacts],
    codeunits: &[(String, u32)],
    include_failing: bool,
    stubbed: &[(String, u32)],
) -> Selection {
    let h = header();
    stored.select(&Current {
        header: &h,
        objects,
        codeunits,
        include_failing,
        stubbed,
    })
}

fn ran(s: &Selection) -> Vec<u32> {
    s.run.iter().map(|(_, id)| *id).collect()
}

fn set_hash(objects: &mut [ObjectFacts], id: u32, hash: &str) {
    objects.iter_mut().find(|o| o.id == Some(id)).expect("object").hash = hash.into();
}

#[test]
fn nothing_changed_runs_nothing() {
    // Test plan 6: a comment-only edit that moves no line leaves every hash as it was.
    let (stored, now) = base();
    let s = select(&stored, &now);
    assert_eq!(s.full, None);
    assert!(s.run.is_empty(), "{s:?}");
    assert!(s.changed.is_empty());
}

#[test]
fn a_changed_routine_runs_the_codeunits_that_entered_it() {
    // Test plan 2 and 5 (a line shift changes the hash just as an edit does).
    let (stored, mut now) = base();
    set_hash(&mut now, 50000, "c0'");
    let s = select(&stored, &now);
    assert_eq!(s.full, None);
    assert_eq!(ran(&s), vec![60000]);
    assert_eq!(s.changed, vec!["codeunit 50000 O50000".to_string()]);
}

#[test]
fn a_changed_table_runs_the_codeunits_that_used_it() {
    // Test plan 3.
    let (stored, mut now) = base();
    set_hash(&mut now, 50001, "t1'");
    assert_eq!(ran(&select(&stored, &now)), vec![60001]);
}

#[test]
fn a_table_extension_runs_the_users_of_its_table() {
    let (mut stored, mut now) = base();
    let mut ext = obj(ObjKind::TableExtension, 50010, "te");
    ext.uses = vec!["table 50001".into()];
    stored.objects.push(ext.clone());
    ext.hash = "te'".into();
    now.push(ext);
    assert_eq!(ran(&select(&stored, &now)), vec![60001]);
}

#[test]
fn a_new_subscriber_runs_the_codeunits_that_raised_its_event() {
    // Test plan 4: also an event nobody subscribed to before (the trace notes every raise).
    let (stored, mut now) = base();
    let o = now.iter_mut().find(|o| o.id == Some(50000)).expect("codeunit");
    o.hash = "c0'".into();
    o.subscribes = vec!["event codeunit 50000 onfoo".into()];
    assert_eq!(ran(&select(&stored, &now)), vec![60000, 60002]);
}

#[test]
fn a_removed_subscriber_runs_the_codeunits_that_raised_its_event() {
    let (mut stored, mut now) = base();
    stored.objects[0].subscribes = vec!["event codeunit 50000 onfoo".into()];
    set_hash(&mut now, 50000, "c0'");
    assert_eq!(ran(&select(&stored, &now)), vec![60000, 60002]);
}

#[test]
fn a_global_triggers_subscriber_runs_every_codeunit_that_wrote_a_table() {
    let (stored, mut now) = base();
    let o = now.iter_mut().find(|o| o.id == Some(50000)).expect("codeunit");
    o.hash = "c0'".into();
    o.global_triggers = true;
    assert_eq!(ran(&select(&stored, &now)), vec![60000, 60001]);
}

#[test]
fn an_object_that_refers_to_a_changed_one_counts_as_changed() {
    // Lowering reads the declarations of the objects referred to (an enum's values, a table's fields).
    let (stored, mut now) = base();
    set_hash(&mut now, 50002, "e2'");
    now[0].refers = vec![key(ObjKind::Enum, 50002)];
    assert_eq!(ran(&select(&stored, &now)), vec![60000]);
}

#[test]
fn an_added_object_is_a_full_run() {
    // Test plan 7, owner decision 4.
    let (stored, mut now) = base();
    now.push(obj(ObjKind::Codeunit, 50005, "n"));
    let s = select(&stored, &now);
    assert_eq!(s.full.as_deref(), Some("an object was added: codeunit 50005 O50005"));
    assert_eq!(ran(&s), vec![60000, 60001, 60002]);
}

#[test]
fn an_added_test_codeunit_runs_alone() {
    let (stored, mut now) = base();
    now.push(test_cu(60009, "n"));
    let mut cus = all();
    cus.push((APP.into(), 60009));
    let s = select_with(&stored, &now, &cus, false, &[]);
    assert_eq!(s.full, None);
    assert_eq!(ran(&s), vec![60009]);
}

#[test]
fn a_removed_object_is_a_full_run() {
    let (stored, mut now) = base();
    now.retain(|o| o.id != Some(50002));
    assert_eq!(
        select(&stored, &now).full.as_deref(),
        Some("an object was removed: enum 50002 O50002")
    );
}

#[test]
fn a_changed_object_of_a_kind_the_trace_does_not_follow_is_a_full_run() {
    let (mut stored, mut now) = base();
    let q = obj(ObjKind::Query, 50020, "q");
    stored.objects.push(q.clone());
    now.push(ObjectFacts { hash: "q'".into(), ..q });
    let s = select(&stored, &now);
    assert_eq!(
        s.full.as_deref(),
        Some("a changed query is not followed by the trace: query 50020 O50020")
    );
}

#[test]
fn changed_install_code_is_a_full_run() {
    let (mut stored, mut now) = base();
    stored.objects[0].install = true;
    set_hash(&mut now, 50000, "c0'");
    assert_eq!(
        select(&stored, &now).full.as_deref(),
        Some("install code changed: codeunit 50000 O50000")
    );
}

#[test]
fn a_change_after_a_dotnet_run_is_a_full_run() {
    let (mut stored, mut now) = base();
    stored.codeunits[1].dotnet = true;
    assert_eq!(select(&stored, &now).full, None, "no change: nothing to vouch for");
    set_hash(&mut now, 50001, "t1'");
    assert!(select(&stored, &now).full.is_some());
}

#[test]
fn a_header_difference_is_a_full_run_with_its_reason() {
    // Test plan 8 and 12: app.json, the environment, the runtime build, a run option.
    let (stored, now) = base();
    type Change = Box<dyn Fn(&mut Header)>;
    let cases: Vec<(Change, &str)> = vec![
        (
            Box::new(|h| h.runtime = "build-2".into()),
            "the stored result was written by another runtime build",
        ),
        (
            Box::new(|h| h.apps[0].1 = "1.0.0.1".into()),
            "the workspace apps (ids or versions) changed",
        ),
        (Box::new(|h| h.manifests[0] = "m2".into()), "app.json changed"),
        (Box::new(|h| h.options = "other".into()), "a run option changed"),
        (
            Box::new(|h| h.environment[1] = "bc 27.1".into()),
            "the dependency apps or the BC build changed",
        ),
    ];
    for (change, reason) in cases {
        let mut h = header();
        change(&mut h);
        let s = stored.select(&Current {
            header: &h,
            objects: &now,
            codeunits: &all(),
            include_failing: false,
            stubbed: &[],
        });
        assert_eq!(s.full.as_deref(), Some(reason));
        assert_eq!(s.run.len(), 3);
    }
}

#[test]
fn include_failing_runs_a_failing_codeunit_the_change_does_not_reach() {
    // Test plan 9.
    let (mut stored, now) = base();
    stored.codeunits[2].tests[0].1 = Outcome::Fail("boom".into());
    assert!(select(&stored, &now).run.is_empty());
    assert_eq!(ran(&select_with(&stored, &now, &all(), true, &[])), vec![60002]);
}

#[test]
fn a_cut_or_refused_run_runs_again() {
    // Test plan 10.
    let (mut stored, now) = base();
    stored.codeunits[0].ended = Ended::Cut;
    stored.codeunits[1].tests[0].1 = Outcome::NotRun("refused".into());
    assert_eq!(ran(&select(&stored, &now)), vec![60000, 60001]);
}

#[test]
fn a_metadata_table_reader_runs_on_any_change() {
    // Test plan 11.
    let (mut stored, mut now) = base();
    stored.codeunits[2].keys.push("table 2000000038".into());
    assert!(select(&stored, &now).run.is_empty());
    set_hash(&mut now, 50001, "t1'");
    assert_eq!(ran(&select(&stored, &now)), vec![60001, 60002]);
}

#[test]
fn a_changed_test_codeunit_runs() {
    let (stored, mut now) = base();
    set_hash(&mut now, 60001, "x1'");
    assert_eq!(ran(&select(&stored, &now)), vec![60001]);
}

#[test]
fn stubbed_codeunits_run_every_time() {
    // Test plan 16 (decision 8).
    let (stored, now) = base();
    let stubbed = vec![(APP.to_string(), 60001)];
    assert_eq!(ran(&select_with(&stored, &now, &all(), false, &stubbed)), vec![60001]);
}

#[test]
fn the_stubbed_codeunits_of_the_stored_run() {
    let (mut stored, _) = base();
    stored.codeunits[1].stubbed = true;
    assert_eq!(stored.stubbed(), vec![(APP.to_string(), 60001)]);
}

#[test]
fn an_unrecorded_codeunit_runs() {
    let (mut stored, now) = base();
    stored.codeunits.remove(0);
    assert_eq!(ran(&select(&stored, &now)), vec![60000]);
}

#[test]
fn the_stored_result_survives_json() {
    let (mut stored, _) = base();
    stored.codeunits[1]
        .tests
        .push(("Test2".into(), Outcome::Fail("a \"quoted\" message".into())));
    stored.codeunits[2].ended = Ended::Cut;
    stored.codeunits[2].dotnet = true;
    stored.codeunits[0].stubbed = true;
    let back = Stored::from_json(&stored.to_json()).expect("readable");
    assert_eq!(back, stored);
}

#[test]
fn another_format_is_not_read() {
    let (stored, _) = base();
    let mut v = stored.to_json();
    v["format"] = json!(FORMAT + 1);
    assert!(Stored::from_json(&v).unwrap_err().contains("format"));
}

#[test]
fn a_narrowed_run_keeps_the_codeunits_it_did_not_run() {
    let (stored, now) = base();
    let mut again = stored_cu(60000, &["table 1"], &[]);
    again.tests[0].1 = Outcome::Fail("x".into());
    let existing: Vec<(String, u32)> = vec![(APP.into(), 60000), (APP.into(), 60001)];
    let merged = stored
        .clone()
        .merge(header(), now.clone(), &existing, vec![again.clone()], false);
    // 60002 no longer exists; 60001 is kept; 60000 is the new run.
    assert_eq!(merged.codeunits.len(), 2);
    assert_eq!(merged.codeunits[0], again);
    assert_eq!(merged.codeunits[1], stored.codeunits[1]);
    let full = stored.merge(header(), now, &existing, vec![again.clone()], true);
    assert_eq!(full.codeunits, vec![again]);
}

#[test]
fn a_changed_object_the_trace_cannot_follow_is_a_full_run() {
    let (stored, mut now) = base();
    let o = now.iter_mut().find(|o| o.id == Some(50000)).expect("codeunit");
    o.hash = "c0'".into();
    o.unfollowed = vec!["subscriber X to codeunit 1 onlogin: its raises are not noted".into()];
    assert_eq!(
        select(&stored, &now).full.as_deref(),
        Some(
            "a change of codeunit 50000 O50000 is not followed by the trace: subscriber X to codeunit 1 onlogin: its raises are not noted"
        )
    );
}

// The facts of a compiled workspace (`facts`).
const FACTS_SRC: &str = r#"
table 50300 "F Tab" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
tableextension 50301 "F TabExt" extends "F Tab" { fields { field(50000; X; Integer) { } } }
enum 50302 "F Enum" { value(0; A) { } }
codeunit 50303 "F Pub"
{
    [IntegrationEvent(false, false)]
    procedure OnFoo() begin end;
}
codeunit 50304 "F Sub"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"F Pub", 'OnFoo', '', false, false)]
    local procedure Handle()
    var
        E: Enum "F Enum";
    begin
    end;

    [EventSubscriber(ObjectType::Table, Database::"F Tab", 'OnAfterInsertEvent', '', false, false)]
    local procedure AfterInsert(var Rec: Record "F Tab"; RunTrigger: Boolean) begin end;
}
codeunit 50305 "F Install"
{
    Subtype = Install;
    trigger OnInstallAppPerCompany() begin end;
}
codeunit 50306 "F Test"
{
    Subtype = Test;
    [Test]
    procedure T() begin end;
}
"#;

#[test]
fn the_facts_of_a_compiled_workspace() {
    let (_ws, lowered) = al_sema::lower::compile_apps_with(
        vec![al_sema::workspace::AppSource {
            id: "A1".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), FACTS_SRC.into())],
        }],
        vec![al_sema::workspace::AppKind::Workspace],
        &Default::default(),
        None,
        al_sema::check::Roots {
            object_hashes: true,
            ..Default::default()
        },
    );
    let facts = facts(&lowered);
    let f = |id: u32| {
        facts
            .iter()
            .find(|o| o.id == Some(id))
            .unwrap_or_else(|| panic!("{id}: {facts:?}"))
    };
    assert!(facts.iter().all(|o| o.app == "a1" && o.hash.len() == 64), "{facts:?}");
    assert_eq!(f(50300).uses, vec!["table 50300".to_string()]);
    assert_eq!(f(50301).uses, vec!["table 50300".to_string()]);
    assert_eq!(f(50302).uses, vec!["enum 50302".to_string()]);
    let mut subs = f(50304).subscribes.clone();
    subs.sort();
    assert_eq!(
        subs,
        vec![
            "event codeunit 50303 onfoo".to_string(),
            "event table 50300 onafterinsertevent".to_string()
        ]
    );
    assert!(
        f(50304).refers.contains(&"a1/enum 50302".to_string()),
        "{:?}",
        f(50304).refers
    );
    assert!(f(50305).install && !f(50304).install);
    assert!(f(50306).test && !f(50303).test);
    assert!(
        facts.iter().all(|o| o.unfollowed.is_empty() && !o.global_triggers),
        "{facts:?}"
    );
}
