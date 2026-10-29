//! ADR-046 step B: the `--companies` opt-in. The run declares the companies of its database; the per-company install code
//! of the apps under test runs once in each, the session company first, then the per-database part
//! (install/lt-bf-install-probes/BfInstallTests.al:125-140; install/lt-av-install-order/AvInstallOrder.al:56-67). The
//! company list is a setting of the process (`al_testrun::host::set_companies`), so these tests have a binary of their own,
//! and every test in it declares the same list.

use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_vm::Engine;
use std::sync::Arc;

const SESSION: &str = "CRONUS International Ltd.";
const OTHER: &str = "My Company";

/// The app id of Microsoft's Base Application.
const BASE_APP: &str = "437dbf0e-84ff-417a-965d-ed2bb9650972";

const COMPANY_TABLE: &str = "table 2000000006 Company { DataPerCompany = false;
  fields { field(1; Name; Text[30]) { } field(2; \"Evaluation Company\"; Boolean) { } field(3; \"Display Name\"; Text[250]) { } field(8000; Id; Guid) { } } }";

/// The outcomes of the test codeunit of `apps` (id, dependencies, source); the last app holds it.
fn outcomes_of_apps(apps: &[(&str, &[&str], &str)]) -> Vec<(String, Outcome)> {
    outcomes_with_data(apps, None)
}

/// [`outcomes_of_apps`] on a database restored from `data` (the test data of the run).
fn outcomes_with_data(
    apps: &[(&str, &[&str], &str)],
    data: Option<Arc<dyn al_host::testdata::TableProvider>>,
) -> Vec<(String, Outcome)> {
    al_testrun::host::set_companies(al_host::Companies::List(vec![SESSION.into(), OTHER.into()]));
    // The app with id `dep` is a dependency app (ADR-013), the others are under test.
    let (ws, l) = al_sema::lower::compile_apps(
        apps.iter()
            .map(|(id, deps, src)| AppSource {
                id: (*id).into(),
                name: (*id).into(),
                dependencies: deps.iter().map(|d| (*d).to_string()).collect(),
                files: vec![(format!("{id}.al"), (*src).into())],
            })
            .collect(),
        apps.iter()
            .map(|(id, _, _)| {
                if *id == "dep" || *id == BASE_APP {
                    al_sema::AppKind::Dependency
                } else {
                    al_sema::AppKind::Workspace
                }
            })
            .collect(),
        &Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let mut engine = Engine::new(l.program.clone(), blocked);
    Arc::get_mut(&mut engine).expect("a new engine").test_data = data.map(al_host::testdata::TestData::new);
    let oi = l
        .program
        .objects
        .iter()
        .position(|o| {
            o.properties
                .iter()
                .any(|(k, v)| k.eq_ignore_ascii_case("Subtype") && v.eq_ignore_ascii_case("Test"))
        })
        .expect("a test codeunit");
    al_testrun::runtime::run_codeunit(&engine, &l, oi)
}

fn outcomes(src: &str) -> Vec<(String, Outcome)> {
    outcomes_of_apps(&[("a", &[], src)])
}

fn all_pass(o: &[(String, Outcome)]) {
    for (name, x) in o {
        assert_eq!(*x, Outcome::Pass, "{name}: {o:?}");
    }
}

// Step 1 (A6): one Company row per declared company, in key order; the other company's Evaluation Company is No and its
// Display Name empty (BfInstallTests.al:47-64). Without test data the run models the corpus CI database, whose session
// company is an evaluation company (BfInstallTests.al:61, amendment 4).
#[test]
fn the_company_table_holds_the_declared_companies() {
    let o = outcomes(&format!(
        "{COMPANY_TABLE}
        codeunit 60102 T {{ Subtype = Test;
            [Test] procedure Names() var C: Record Company; N: Integer; S: Text; begin
                if C.FindSet() then repeat N += 1; S += C.Name + '|' + C.\"Display Name\" + '|' + Format(C.\"Evaluation Company\") + ';'; until C.Next() = 0;
                if S <> 'CRONUS International Ltd.||Yes;My Company||No;' then Error('%1', S);
            end;
            [Test] procedure ChangeCompanyToTheOther() var C: Record Company; begin
                if not C.ChangeCompany('My Company') then Error('no My Company');
                if C.ChangeCompany('Not Declared') then Error('a company that is not declared');
            end; }}"
    ));
    all_pass(&o);
}

// Step 4a and 4b: the per-company install code of the app under test runs in each declared company, session company
// first, with CompanyName() that company; then the per-database part once, with CompanyName() empty
// (BfInstallTests.al:125-140, AvInstallOrder.al:56-67). A shared table the per-company code writes is answered under the
// opt-in (decision 4c does not apply), and a per-company table of the app holds its row in each company.
#[test]
fn per_company_install_code_runs_in_each_declared_company_then_per_database() {
    let o = outcomes(&format!(
        "{COMPANY_TABLE}
        table 60900 Log {{ DataPerCompany = false; fields {{ field(1; Id; Integer) {{ AutoIncrement = true; }} field(2; Info; Text[50]) {{ }} field(3; Ev; Text[30]) {{ }} }} }}
        table 60901 Seed {{ fields {{ field(1; K; Code[20]) {{ }} field(2; Info; Text[50]) {{ }} }} }}
        codeunit 60905 Subs {{
            [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Extension Triggers\", 'OnInstallAppPerCompany', '', false, false)]
            local procedure PerCompany() var R: Record Log; S: Record Seed; begin
                R.Info := CompanyName(); R.Ev := 'company'; R.Insert();
                S.K := 'COMPANY1'; S.Info := CompanyName(); S.Insert(); end;
            [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Extension Triggers\", 'OnInstallAppPerDatabase', '', false, false)]
            local procedure PerDatabase() var R: Record Log; begin R.Info := CompanyName(); R.Ev := 'database'; R.Insert(); end; }}
        codeunit 60102 T {{ Subtype = Test;
            [Test] procedure LogRows() var R: Record Log; S: Text; begin
                R.SetCurrentKey(Id);
                if R.FindSet() then repeat S += Format(R.Id) + ':' + R.Ev + '@' + R.Info + ';'; until R.Next() = 0;
                if S <> '1:company@CRONUS International Ltd.;2:company@My Company;3:database@;' then Error('%1', S);
            end;
            [Test] procedure SeedInEachCompany() var S: Record Seed; begin
                S.Get('COMPANY1'); if S.Info <> CompanyName() then Error('session %1', S.Info);
                S.ChangeCompany('My Company'); S.Get('COMPANY1'); if S.Info <> 'My Company' then Error('other %1', S.Info);
                if S.Count() <> 1 then Error('rows %1', S.Count());
            end; }}"
    ));
    all_pass(&o);
}

// Decision 5: under the opt-in, dependency apps stay in the session company. A per-company table of a dependency app read
// in the other declared company is refused; the same table in the session company is answered.
#[test]
fn a_dependency_table_in_the_other_company_is_refused() {
    let dep = "table 60800 DepSetup { fields { field(1; K; Code[10]) { } } }";
    let main = format!(
        "{COMPANY_TABLE}
        codeunit 60102 T {{ Subtype = Test;
            [Test] procedure SessionCompany() var R: Record DepSetup; begin R.K := 'D'; R.Insert(); R.Get('D'); end;
            [Test] procedure OtherCompany() var R: Record DepSetup; begin R.ChangeCompany('My Company'); R.K := 'D'; R.Insert(); end; }}"
    );
    let o = outcomes_of_apps(&[("dep", &[], dep), ("main", &["dep"], &main)]);
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
    assert!(
        matches!(&o[1].1, Outcome::NotRun(r) if r.contains("My Company") && r.contains("decision 5")),
        "{o:?}"
    );
}

// Decisions 5 and 6 inside install code: a per-company install routine of the app under test that reads a dependency
// table runs in the session company and stops in the other company. What it would have written there is not known in that
// company only: a per-company table is answered in the session company and refused in the other one; a shared table is
// refused everywhere.
#[test]
fn install_code_that_reads_a_dependency_table_stops_in_the_other_company_only() {
    let dep = "table 60800 DepSetup { fields { field(1; K; Code[10]) { } } }";
    let main = format!(
        "{COMPANY_TABLE}
        table 60901 Seed {{ fields {{ field(1; K; Code[20]) {{ }} field(2; N; Integer) {{ }} }} }}
        table 60902 Shared {{ DataPerCompany = false; fields {{ field(1; K; Code[40]) {{ }} }} }}
        codeunit 60905 Inst {{ Subtype = Install;
            trigger OnInstallAppPerCompany() var D: Record DepSetup; S: Record Seed; Sh: Record Shared;
            begin S.K := 'X'; S.N := D.Count(); S.Insert(); Sh.K := CompanyName(); Sh.Insert(); end; }}
        codeunit 60102 T {{ Subtype = Test;
            [Test] procedure SeedInTheSessionCompany() var S: Record Seed; begin S.Get('X'); if S.Count() <> 1 then Error('rows'); end;
            [Test] procedure SeedInTheOtherCompany() var S: Record Seed; begin S.ChangeCompany('My Company'); if S.IsEmpty() then; end;
            [Test] procedure SharedEverywhere() var Sh: Record Shared; begin if Sh.Count() <> 1 then Error('rows'); end; }}"
    );
    let o = outcomes_of_apps(&[("dep", &[], dep), ("main", &["dep"], &main)]);
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
    for i in [1, 2] {
        assert!(
            matches!(&o[i].1, Outcome::NotRun(r) if r.contains("My Company") && r.contains("decision 6")),
            "{i}: {o:?}"
        );
    }
}

/// Base Application's tables the corpus pins for "My Company" (CDS Connection Setup holds no row there) and one table that
/// holds rows of a content no test pins (Report Selections, 2 rows).
const BASE_TABLES: &str = "table 7200 \"CDS Connection Setup\" { fields { field(1; \"Primary Key\"; Code[20]) { } field(2; \"Server Address\"; Text[250]) { } } }
    table 77 \"Report Selections\" { fields { field(1; Usage; Integer) { } field(2; Sequence; Code[10]) { } } }";

// ADR-046 amendment 4: "My Company" of the corpus CI database holds no CDS Connection Setup row
// (install/lt-bf-install-probes/BfInstallTests.al, SecondCompany_CdsConnectionSetup_ComparedWithSessionCompany, corpus PR
// 804). Under the opt-in, a run without test data answers the table there as empty: what install code and tests write
// there is what it holds. A Base Application table that holds rows of unpinned content there stays refused.
#[test]
fn a_base_table_pinned_empty_in_my_company_starts_empty_there() {
    let main = format!(
        "{COMPANY_TABLE}
        codeunit 60102 T {{ Subtype = Test;
            [Test] procedure CdsInMyCompany() var Cds: Record \"CDS Connection Setup\"; Here: Record \"CDS Connection Setup\"; begin
                Here.\"Server Address\" := 'session'; Here.Insert();
                Cds.ChangeCompany('My Company');
                if not Cds.IsEmpty() then Error('not empty');
                if Cds.Count() <> 0 then Error('count %1', Cds.Count());
                Cds.\"Server Address\" := 'other'; Cds.Insert();
                Cds.Get(); if Cds.\"Server Address\" <> 'other' then Error('other %1', Cds.\"Server Address\");
                Here.Get(); if Here.\"Server Address\" <> 'session' then Error('session %1', Here.\"Server Address\");
            end;
            [Test] procedure ReportSelectionsInMyCompany() var R: Record \"Report Selections\"; begin
                R.ChangeCompany('My Company'); if R.IsEmpty() then;
            end; }}"
    );
    let o = outcomes_of_apps(&[(BASE_APP, &[], BASE_TABLES), ("main", &[BASE_APP], &main)]);
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
    assert!(
        matches!(&o[1].1, Outcome::NotRun(r) if r.contains("My Company") && r.contains("Report Selections")),
        "{o:?}"
    );
}

// The same inside install code: per-company install code of the app under test that reads CDS Connection Setup (as Base
// Application's database trigger setup does on every insert) runs in "My Company" too, and its writes there are answered.
#[test]
fn install_code_that_reads_a_pinned_empty_base_table_runs_in_my_company() {
    let main = format!(
        "{COMPANY_TABLE}
        table 60901 Seed {{ fields {{ field(1; K; Code[20]) {{ }} field(2; N; Integer) {{ }} }} }}
        codeunit 60905 Inst {{ Subtype = Install;
            trigger OnInstallAppPerCompany() var Cds: Record \"CDS Connection Setup\"; S: Record Seed;
            begin S.K := 'X'; S.N := Cds.Count(); S.Insert(); end; }}
        codeunit 60102 T {{ Subtype = Test;
            [Test] procedure SeedInMyCompany() var S: Record Seed; begin
                S.ChangeCompany('My Company'); S.Get('X'); if S.N <> 0 then Error('n %1', S.N);
            end; }}"
    );
    let o = outcomes_of_apps(&[(BASE_APP, &[], BASE_TABLES), ("main", &[BASE_APP], &main)]);
    all_pass(&o);
}

/// A database with two companies: `rows` lists the (company, table) pairs that hold rows.
struct TwoCompanies {
    rows: Vec<(&'static str, &'static str)>,
}

impl al_host::testdata::TableProvider for TwoCompanies {
    fn company(&self) -> &str {
        SESSION
    }
    fn source(&self) -> String {
        "two.bak".into()
    }
    fn read(
        &self,
        _: &al_host::testdata::TableRequest,
    ) -> Result<Arc<al_host::testdata::TableRows>, al_host::testdata::TableError> {
        Err(al_host::testdata::TableError::Absent)
    }
    fn companies(&self) -> Vec<String> {
        vec![SESSION.into(), OTHER.into()]
    }
    fn holds_rows_in(&self, company: &str, table: &al_host::testdata::TableRequest) -> Option<bool> {
        Some(self.rows.iter().any(|(c, t)| *c == company && *t == table.name))
    }
}

// ADR-046 amendment 4, with test data: in a declared company other than the session company, a dependency table the
// restored database holds no row of starts empty there (the database is what BC has); a table that holds rows there stays
// refused, since this runtime does not serve another company's rows.
#[test]
fn with_test_data_a_dependency_table_without_rows_in_the_other_company_starts_empty_there() {
    let main = format!(
        "{COMPANY_TABLE}
        codeunit 60102 T {{ Subtype = Test;
            [Test] procedure CdsInMyCompany() var Cds: Record \"CDS Connection Setup\"; begin
                Cds.ChangeCompany('My Company'); if not Cds.IsEmpty() then Error('not empty');
            end;
            [Test] procedure ReportSelectionsInMyCompany() var R: Record \"Report Selections\"; begin
                R.ChangeCompany('My Company'); if R.IsEmpty() then;
            end; }}"
    );
    let data = TwoCompanies {
        rows: vec![(OTHER, "Report Selections")],
    };
    let o = outcomes_with_data(
        &[(BASE_APP, &[], BASE_TABLES), ("main", &[BASE_APP], &main)],
        Some(Arc::new(data)),
    );
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
    assert!(
        matches!(&o[1].1, Outcome::NotRun(r) if r.contains("My Company") && r.contains("Report Selections")),
        "{o:?}"
    );
}
