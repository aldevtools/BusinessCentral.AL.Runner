//! The impact index's record of a run (ADR-043 step 2): what each test raised and touched, as keys by object id, what
//! the install baseline raised and touched, the narrowing of the run after its compile, and the impact points of an
//! extension.

use al_sema::workspace::AppSource;
use al_testrun::impact_trace::{Matcher, impact_points};
use al_testrun::runtime::{CorpusRuntime, Manifest, Narrow, RunOptions, SuiteRun};
use std::sync::Arc;

const BASE: &str = r#"
table 50970 "IX Row"
{
    fields { field(1; Id; Integer) { } field(2; V; Integer) { } }
    keys { key(PK; Id) { } }
}
table 50971 "IX Setup"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { } }
}
codeunit 50972 "IX Posting"
{
    procedure Post()
    begin
        OnBeforePost();
    end;

    [IntegrationEvent(false, false)]
    local procedure OnBeforePost()
    begin
    end;

    [IntegrationEvent(false, false)]
    procedure OnInstallSetup()
    begin
    end;
}
codeunit 50973 "IX Install"
{
    Subtype = Install;
    trigger OnInstallAppPerCompany()
    var
        S: Record "IX Setup";
        P: Codeunit "IX Posting";
    begin
        S.Id := 1;
        S.Insert();
        P.OnInstallSetup();
    end;
}
"#;

const TESTS: &str = r#"
codeunit 50974 "IX Tests"
{
    Subtype = Test;

    [Test]
    procedure PostsAndWrites()
    var
        P: Codeunit "IX Posting";
        Ref: RecordRef;
        F: FieldRef;
    begin
        P.Post();
        Ref.Open(Database::"IX Row");
        F := Ref.Field(1);
        F.Value := 5;
        Ref.Insert();
    end;

    [Test]
    procedure ReadsOnly()
    var
        S: Record "IX Setup";
    begin
        if S.Get(1) then;
    end;
}
codeunit 50975 "IX Other Tests"
{
    Subtype = Test;

    [Test]
    procedure Nothing()
    begin
    end;
}
"#;

fn app(id: &str, deps: &[&str], src: &str) -> AppSource {
    AppSource {
        id: id.into(),
        name: id.into(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![(format!("{id}.al"), src.into())],
    }
}

fn run(opts: &RunOptions) -> CorpusRuntime {
    let manifest = |p: &str| Manifest {
        publisher: p.into(),
        version: "1.0.0.0".into(),
        resources: vec![],
        include_source: false,
        target: String::new(),
        layout_files: Vec::new(),
        id_ranges: Vec::new(),
        symbols: vec![],
    };
    CorpusRuntime::try_run_suites_with(
        vec![app("tests", &["base"], TESTS), app("base", &[], BASE)],
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        &[manifest("Microsoft"), manifest("Microsoft")],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        opts,
    )
    .expect("the run starts")
}

fn keys_of(rt: &CorpusRuntime, test: &str) -> Vec<String> {
    let r = rt.results().iter().find(|r| r.test == test).unwrap();
    r.impact.iter().map(|&k| rt.impact_keys()[k as usize].clone()).collect()
}

#[test]
fn a_traced_run_keeps_what_each_test_and_the_install_baseline_raised_and_touched() {
    let rt = run(&RunOptions {
        impact_trace: true,
        ..Default::default()
    });
    let outcomes: Vec<_> = rt
        .results()
        .iter()
        .map(|r| (r.test.clone(), r.outcome.clone()))
        .collect();
    assert!(
        outcomes.iter().all(|(_, o)| matches!(o, al_testrun::Outcome::Pass)),
        "{outcomes:?}"
    );
    let posts = keys_of(&rt, "PostsAndWrites");
    // A raise nobody subscribes to, a write through a RecordRef, the table events of that write.
    for k in [
        "event codeunit 50972 onbeforepost",
        "table 50970",
        "write 50970",
        "event table 50970 onbeforeinsertevent",
    ] {
        assert!(posts.contains(&k.to_string()), "{k}: {posts:?}");
    }
    let reads = keys_of(&rt, "ReadsOnly");
    assert_eq!(reads, ["table 50971"], "{reads:?}");
    assert!(keys_of(&rt, "Nothing").is_empty());
    // The install code's raise and write are the baseline's, not a test's.
    let baseline: Vec<&str> = rt
        .impact_baseline()
        .expect("the baseline ran")
        .iter()
        .map(|&k| rt.impact_keys()[k as usize].as_str())
        .collect();
    assert!(
        baseline.contains(&"event codeunit 50972 oninstallsetup"),
        "{baseline:?}"
    );
    assert!(baseline.contains(&"write 50971"), "{baseline:?}");
    assert!(!posts.iter().any(|k| k.contains("oninstallsetup")), "{posts:?}");

    // Off: nothing is kept.
    let rt = run(&RunOptions::default());
    assert!(rt.results().iter().all(|r| r.impact.is_empty()));
    assert!(rt.impact_baseline().is_none());
}

#[test]
fn a_run_can_be_narrowed_after_its_compile() {
    let rt = run(&RunOptions {
        narrow: Some(Narrow(Arc::new(|input| {
            // The program knows both test codeunits; the run keeps one.
            assert!(input.program.objects.iter().any(|o| o.id == Some(50975)));
            Some(vec![(0, 50974)])
        }))),
        ..Default::default()
    });
    let ran: Vec<&str> = rt.results().iter().map(|r| r.test.as_str()).collect();
    assert_eq!(ran, ["PostsAndWrites", "ReadsOnly"]);
}

const EXT: &str = r#"
codeunit 50976 "IX Ext Sub"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"IX Posting", 'OnBeforePost', '', false, false)]
    local procedure Check()
    begin
    end;

    [EventSubscriber(ObjectType::Table, Database::"IX Row", 'OnAfterValidateEvent', 'V', false, false)]
    local procedure AfterV()
    begin
    end;
}
tableextension 50977 "IX Row Ext" extends "IX Row"
{
    fields { field(50977; Extra; Integer) { InitValue = 3; } }
}
enumextension 50978 "IX Ext Enum" extends "IX Kind"
{
    value(50978; Partner) { }
}
"#;

#[test]
fn the_impact_points_of_an_extension_are_listed_from_the_compiled_program() {
    let base = format!("{BASE}\nenum 50979 \"IX Kind\" {{ Extensible = true; value(0; None) {{ }} }}");
    let (ws, lowered) = al_sema::lower::compile(
        vec![app("base", &[], &base), app("ext", &["base"], EXT)],
        &Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let program = &lowered.program;
    let ext_app = program.apps.iter().position(|a| a.id == "ext").unwrap() as u32;
    let points = impact_points(&lowered, |oi| program.objects[oi].app == ext_app);
    let matchers: Vec<&Matcher> = points.iter().map(|p| &p.matcher).collect();
    assert!(
        matchers.contains(&&Matcher::Key("event codeunit 50972 onbeforepost".into())),
        "{points:?}"
    );
    assert!(
        matchers.contains(&&Matcher::Key("event table 50970 onaftervalidateevent/v".into())),
        "{points:?}"
    );
    assert!(matchers.contains(&&Matcher::Key("table 50970".into())), "{points:?}");
    // An enum extension selects the tests that read its enum's values (ADR-043 step 5).
    assert!(matchers.contains(&&Matcher::Key("enum 50979".into())), "{points:?}");
    // A table extension with fields only runs no code.
    let t = points
        .iter()
        .find(|p| p.matcher == Matcher::Key("table 50970".into()))
        .unwrap();
    assert!(!t.runs_code);
}
