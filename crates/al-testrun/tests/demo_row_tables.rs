//! Without test data the tests run against the corpus database's company, which holds demo rows of Accounting Period and
//! Company Information, and its General Ledger Setup row, that the runtime does not have (session/lt-company-information/
//! CompanyInformationValues.al:45, :89; record/lt-qd-captionclass-areas/QdCaptionClassAreas.al:804 QF_GLSetup_CorpusRow;
//! ADR-047 rejected). A Count or IsEmpty of one of those dependency tables before the test wrote to it is refused, also when
//! install code wrote a row there; once the test wrote to it, it is answered, and other reads stay answered.

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, Manifest, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

const BASE: &str = r#"
table 50 "Accounting Period" { fields { field(1; "Starting Date"; Date) { } } keys { key(PK; "Starting Date") { } } }
table 79 "Company Information" { fields { field(1; "Primary Key"; Code[10]) { } field(2; Name; Text[100]) { } } keys { key(PK; "Primary Key") { } } }
table 98 "General Ledger Setup" { fields { field(1; "Primary Key"; Code[10]) { } } keys { key(PK; "Primary Key") { } } }
codeunit 51 "Base Install"
{
    Subtype = Install;
    trigger OnInstallAppPerCompany() var C: Record "Company Information"; begin C.Insert(); end;
}
"#;

const MAIN: &str = r#"
table 50300 "Own Row" { fields { field(1; K; Integer) { } } keys { key(PK; K) { } } }
codeunit 50301 DemoTests
{
    Subtype = Test;
    [Test] procedure GetsTheInstalledRow() var C: Record "Company Information"; begin if not C.Get() then Error('no row'); end;
    [Test] procedure CountsCompanyInformation() var C: Record "Company Information"; begin if C.Count() <> 1 then Error('count %1', C.Count()); end;
    [Test] procedure ScansAccountingPeriods() var P: Record "Accounting Period"; begin if P.IsEmpty() then Error('empty'); end;
    [Test] procedure CountsTheRowItWrote() var C: Record "Company Information"; begin C.Get(); C.Name := 'mine'; C.Modify(); if C.Count() <> 1 then Error('count %1', C.Count()); end;
    [Test] procedure CountsGeneralLedgerSetup() var G: Record "General Ledger Setup"; begin if G.Count() <> 1 then Error('count %1', G.Count()); end;
    [Test] procedure WritesGeneralLedgerSetup() var G: Record "General Ledger Setup"; begin G.Insert(); G.Delete(); end;
    [Test] procedure CountsGeneralLedgerSetupAfterAnEarlierTestWroteIt() var G: Record "General Ledger Setup"; begin if G.Count() <> 1 then Error('count %1', G.Count()); end;
    [Test] procedure ScansAnOtherTable() var R: Record "Own Row"; begin if not R.IsEmpty() then Error('rows'); end;
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

fn outcome(rt: &CorpusRuntime, name: &str) -> Outcome {
    rt.first_outcome(&CaseId {
        codeunit: 50301,
        name: name.into(),
    })
}

#[test]
fn a_count_of_an_empty_corpus_demo_table_is_refused() {
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
    let rt = CorpusRuntime::try_run_suites_with(
        vec![app("main", &["base"], MAIN), app("base", &[], BASE)],
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        &[manifest("Me"), manifest("Microsoft")],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions::default(),
    )
    .expect("runs");
    // What an earlier test wrote does not count: the corpus misc/lt-qk-setup-rows/QkSetupRows.al writers put the demo values back,
    // the runtime then holds the install-code row (record/lt-qd-captionclass-areas/QdCaptionClassAreas.al:798 QF_GLSetup_CorpusRow
    // runs after Literals_FieldCaption wrote and restored the row).
    for name in [
        "CountsCompanyInformation",
        "ScansAccountingPeriods",
        "CountsGeneralLedgerSetup",
        "CountsGeneralLedgerSetupAfterAnEarlierTestWroteIt",
    ] {
        let o = outcome(&rt, name);
        assert!(matches!(o, Outcome::NotRun(_)), "{name}: {o:?}");
    }
    for name in [
        "CountsTheRowItWrote",
        "GetsTheInstalledRow",
        "ScansAnOtherTable",
        "WritesGeneralLedgerSetup",
    ] {
        let o = outcome(&rt, name);
        assert!(matches!(o, Outcome::Pass), "{name}: {o:?}");
    }
}
