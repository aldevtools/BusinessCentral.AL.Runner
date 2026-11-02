//! The check of the rule that skips the install baseline (`RunOptions::check_baseline_rule`, `tools/baseline_rule_check.sh`)
//! runs each test codeunit the rule calls free of the baseline on an empty store. Under runner 130450 a codeunit that joins
//! the carry chain only because its tests can take numbers (`numbers_taken.rs`) must not hand that empty store to the later
//! codeunits: one that reads what the install code wrote still sees it, as in a normal run (the reader is in the chain too,
//! as its tests can change a number sequence). Nor may a codeunit with RequiredTestIsolation = Disabled on the TestRunnerHub,
//! which hands its whole store to the later codeunits.

use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, Manifest, RunOptions, SuiteRun};

const BASE: &str = r#"
table 50120 Setting { fields { field(1; Code; Code[20]) { } } keys { key(PK; Code) { } } }
codeunit 50123 SettingSubscriber
{
    [EventSubscriber(ObjectType::Table, Database::Setting, 'OnAfterInsertEvent', '', false, false)]
    local procedure TakesNumber(var Rec: Record Setting) begin NumberSequence.Insert('RULECARRY', 1, 1, false); end;
}
codeunit 50121 Installer
{
    Subtype = Install;
    trigger OnInstallAppPerCompany() var S: Record Setting; begin S.Code := 'SEEDED'; S.Insert(); end;
}
"#;

const MAIN: &str = r#"
codeunit 60120 DisabledPure
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Disabled;
    [Test] procedure DisabledAdds() var A: Integer; begin A := 1 + 1; if A <> 2 then Error('add'); end;
}
codeunit 60121 InMemoryRecord
{
    Subtype = Test;
    [Test] procedure MakesOne() var S: Record Setting; begin S.Code := 'INMEMORY'; if S.Code <> 'INMEMORY' then Error('field'); end;
}
codeunit 60122 ReadsInstallRow
{
    Subtype = Test;
    [Test] procedure SeesTheInstallRowAfterTheChain()
    var S: Record Setting;
    begin
        if not S.Get('SEEDED') then Error('no row');
        if NumberSequence.Exists('READER', false) then NumberSequence.Delete('READER', false);
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

fn run(check_baseline_rule: bool, required_isolation: bool) -> CorpusRuntime {
    let manifest = |p: &str| Manifest {
        publisher: p.into(),
        version: "1.0.0.0".into(),
        resources: vec![],
        include_source: false,
        target: String::new(),
        logo: None,
        description: None,
        layout_files: Vec::new(),
        id_ranges: Vec::new(),
        translations: false,
        translation_files: Vec::new(),
        runtime: None,
        app_json_keys: Vec::new(),
        symbols: vec![],
    };
    CorpusRuntime::try_run_suites_with(
        vec![app("main", &["base"], MAIN), app("base", &[], BASE)],
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        &[manifest("Me"), manifest("Microsoft")],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions {
            check_baseline_rule,
            required_isolation,
            ..Default::default()
        },
    )
    .expect("the run starts")
}

fn outcome(rt: &CorpusRuntime, test: &str) -> Outcome {
    rt.results()
        .iter()
        .find(|r| r.test == test)
        .unwrap_or_else(|| panic!("{test} did not run"))
        .outcome
        .clone()
}

#[test]
fn a_codeunit_in_the_chain_for_its_numbers_does_not_carry_the_empty_store_of_the_rule_check() {
    for check in [false, true] {
        let rt = run(check, false);
        assert_eq!(outcome(&rt, "MakesOne"), Outcome::Pass, "check {check}");
        assert_eq!(
            outcome(&rt, "SeesTheInstallRowAfterTheChain"),
            Outcome::Pass,
            "check {check}"
        );
    }
}

#[test]
fn a_disabled_codeunit_on_the_hub_does_not_carry_the_empty_store_of_the_rule_check() {
    for check in [false, true] {
        let rt = run(check, true);
        assert_eq!(outcome(&rt, "DisabledAdds"), Outcome::Pass, "check {check}");
        assert_eq!(
            outcome(&rt, "SeesTheInstallRowAfterTheChain"),
            Outcome::Pass,
            "check {check}"
        );
    }
}
