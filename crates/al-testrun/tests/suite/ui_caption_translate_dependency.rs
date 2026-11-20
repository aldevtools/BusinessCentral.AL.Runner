//! The `CaptionClassTranslate` statement in a test that cannot see a CaptionClassTranslate subscriber of a dependency app
//! (the test holds no variable of its codeunit or of a SingleInstance codeunit it logs to). What the subscriber does is not
//! pinned, so the statement refuses all the same, in the name and the id form; a manual one nobody bound keeps the answer
//! (al-vm `Session::receiving_dependency_subscriber`, al-runner issue 8470; review probes uiwrite, uiid, uiman).

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, Manifest, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

const TEST: &str = r#"
codeunit 50901 "BI Ui Blind Tests"
{
    Subtype = Test;
    [Test] procedure Blind() var T: Text; begin T := CaptionClassTranslate('3,Foo'); if T <> 'Foo' then Error('t=%1', T); end;
}
codeunit 50902 "BI Ui Toucher"
{
    procedure Touch() var S: Codeunit "BI Ui Sub"; begin S.Touch(); end;
}
"#;

/// A dependency app with the subscriber codeunit "BI Ui Sub": `props`, the publisher argument `publisher`, the body `body`.
fn lib(props: &str, publisher: &str, body: &str) -> String {
    format!(
        r#"
codeunit 50220 "BI Log" {{ SingleInstance = true; var L: Text; procedure Add(T: Text) begin L += T; end; }}
codeunit 50221 "BI Ui Sub"
{{
    {props}
    procedure Touch() begin end;
    [EventSubscriber(ObjectType::Codeunit, {publisher}, 'CaptionClassTranslate', '', false, false)]
    local procedure T(Language: Integer; CaptionExpr: Text[1024]; var Translation: Text[1024])
    var Log: Codeunit "BI Log";
    begin
        {body}
        Log.Add(CaptionExpr);
    end;
}}
"#
    )
}

fn blind_outcome(lib_src: &str) -> Outcome {
    let app = |id: &str, deps: &[&str], src: &str| AppSource {
        id: id.into(),
        name: id.into(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![(format!("{id}.al"), src.into())],
    };
    let rt = CorpusRuntime::try_run_suites_with(
        vec![app("test", &["lib"], TEST), app("lib", &[], lib_src)],
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        &[Manifest::default(), Manifest::default()],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions::default(),
    )
    .expect("runs");
    rt.first_outcome(&CaseId {
        codeunit: 50901,
        name: "Blind".into(),
    })
}

const NAME: &str = "Codeunit::\"UI Helper Triggers\"";

#[test]
fn a_blind_test_refuses_for_a_dependency_subscriber_that_writes_the_translation() {
    let o = blind_outcome(&lib("", NAME, "Translation := 'CHG';"));
    assert!(matches!(&o, Outcome::NotRun(m) if m.contains("BI Ui Sub")), "{o:?}");
}

#[test]
fn a_blind_test_refuses_for_a_compiled_dependency_subscriber_written_with_the_id() {
    let o = blind_outcome(&lib("", "2000000004", ""));
    assert!(matches!(&o, Outcome::NotRun(m) if m.contains("BI Ui Sub")), "{o:?}");
}

#[test]
fn a_blind_test_keeps_the_answer_for_a_manual_dependency_subscriber_nobody_bound() {
    let o = blind_outcome(&lib("EventSubscriberInstance = Manual;", NAME, "Translation := 'CHG';"));
    assert!(matches!(o, Outcome::Pass), "{o:?}");
}
