//! al-runner issue 6924: a selection (`--explain`, `--codeunit`, `--test`) runs under the same installed state as the full
//! run. The install code of another app may run a test codeunit by a computed id; before the fix the selection left that
//! codeunit out of the compile ("not compiled: no code reached it while compiling the workspace's dependencies").

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, Manifest, RunOptions};
use al_testrun::{CaseId, Outcome};

const DEP_ID: &str = "11111111-1111-1111-1111-111111169240";
const TEST_ID: &str = "22222222-2222-2222-2222-222222269240";

const DEP: &str = r#"
table 50400 "SI Setting"
{
    fields { field(1; Code; Code[20]) { } field(2; Value; Text[30]) { } }
    keys { key(PK; Code) { Clustered = true; } }
}

codeunit 50400 "SI Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        S: Record "SI Setting";
    begin
        S.Code := 'SEEDED';
        S.Value := 'yes';
        S.Insert();
    end;
}
"#;

const TESTS: &str = r#"
codeunit 50410 "SI Tests A"
{
    Subtype = Test;

    [Test]
    procedure ReadsInstalledRow()
    var
        S: Record "SI Setting";
    begin
        S.Get('SEEDED');
        if S.Value <> 'yes' then
            Error('value %1', S.Value);
    end;
}

codeunit 50411 "SI Tests B"
{
    Subtype = Test;

    [Test]
    procedure AlsoReadsInstalledRow()
    var
        S: Record "SI Setting";
    begin
        if not S.Get('SEEDED') then
            Error('not installed');
    end;
}
"#;

fn run(only: Option<Vec<(usize, u32)>>, test: &str, codeunit: u32) -> Outcome {
    let apps = vec![
        AppSource {
            id: TEST_ID.into(),
            name: "SI Tests".into(),
            dependencies: vec![DEP_ID.into()],
            files: vec![("t.al".into(), TESTS.into())],
        },
        AppSource {
            id: DEP_ID.into(),
            name: "SI Dep".into(),
            dependencies: Vec::new(),
            files: vec![("d.al".into(), DEP.into())],
        },
    ];
    let kinds = vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency];
    let opts = RunOptions {
        only,
        ..Default::default()
    };
    let rt = CorpusRuntime::run_suites_with(
        apps,
        kinds,
        &[Manifest::default(), Manifest::default()],
        &Default::default(),
        None,
        Default::default(),
        &[al_testrun::runtime::SuiteRun::everything()],
        &opts,
    );
    rt.first_outcome(&CaseId {
        codeunit,
        name: test.into(),
    })
}

#[test]
fn the_full_run_reads_the_install_row() {
    assert_eq!(run(None, "ReadsInstalledRow", 50410), Outcome::Pass);
}

/// The selection must give the same outcome as the full run.
#[test]
fn a_selected_codeunit_reads_the_install_row_of_a_dependency_app() {
    assert_eq!(run(Some(vec![(0, 50410)]), "ReadsInstalledRow", 50410), Outcome::Pass);
    assert_eq!(
        run(Some(vec![(0, 50411)]), "AlsoReadsInstalledRow", 50411),
        Outcome::Pass
    );
}

const DYNAMIC_INSTALL: &str = r#"
codeunit 50421 "SI Dynamic Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Id: Integer;
    begin
        // The id is computed: no name or literal in the text says which codeunit runs.
        Id := 50420 + 3;
        Codeunit.Run(Id);
    end;
}

codeunit 50423 "SI Seeding Tests"
{
    Subtype = Test;

    trigger OnRun()
    var
        S: Record "SI Setting";
    begin
        S.Code := 'SEEDED';
        S.Value := 'yes';
        S.Insert();
    end;

    [Test]
    procedure Mine()
    begin
    end;
}
"#;

/// The install code of a second workspace app runs a test codeunit of that app by a computed id. A selection in the
/// first app compiles that codeunit, as the full run does: the outcome must not depend on which tests are selected
/// (al-runner issue 6924).
#[test]
fn a_selection_compiles_the_test_codeunit_an_install_routine_runs_by_a_computed_id() {
    let outcome = |only: Option<Vec<(usize, u32)>>| {
        let apps = vec![
            AppSource {
                id: TEST_ID.into(),
                name: "SI Tests".into(),
                dependencies: vec!["33333333-3333-3333-3333-333333369240".into()],
                files: vec![(
                    "t.al".into(),
                    format!("{}{TESTS}", &DEP[..DEP.find("codeunit 50400").unwrap()]),
                )],
            },
            AppSource {
                id: DEP_ID.into(),
                name: "SI Dynamic".into(),
                dependencies: vec![TEST_ID.into()],
                files: vec![("d.al".into(), DYNAMIC_INSTALL.to_string())],
            },
            // A dependency app makes the compile demand-driven, as Microsoft's apps do.
            AppSource {
                id: "33333333-3333-3333-3333-333333369240".into(),
                name: "SI Lib".into(),
                dependencies: Vec::new(),
                files: vec![(
                    "l.al".into(),
                    "codeunit 50300 \"SI Lib\" { procedure A(): Integer begin exit(1); end; }".into(),
                )],
            },
        ];
        let kinds = vec![
            al_sema::AppKind::Workspace,
            al_sema::AppKind::Workspace,
            al_sema::AppKind::Dependency,
        ];
        let opts = RunOptions {
            only,
            ..Default::default()
        };
        let rt = CorpusRuntime::run_suites_with(
            apps,
            kinds,
            &vec![Manifest::default(); 3],
            &Default::default(),
            None,
            Default::default(),
            &[al_testrun::runtime::SuiteRun::everything()],
            &opts,
        );
        rt.first_outcome(&CaseId {
            codeunit: 50410,
            name: "ReadsInstalledRow".into(),
        })
    };
    assert_eq!(outcome(None), Outcome::Pass, "full run");
    assert_eq!(outcome(Some(vec![(0, 50410)])), Outcome::Pass, "selected codeunit");
}
