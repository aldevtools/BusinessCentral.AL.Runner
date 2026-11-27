//! Issue 9244: install code renames the session company, in a run with test data and without `--companies` (the
//! default of one company, which `companies_opt_in` cannot declare: that binary fixes the company list of the process).
//! No corpus test pins what BC does when the current company is renamed, so the Rename is refused, and every copy of
//! the install baseline answers alike: the install code stops at the Rename.

use al_sema::workspace::AppSource;
use std::sync::Arc;

const BASE_APP: &str = "437dbf0e-84ff-417a-965d-ed2bb9650972";
const SESSION: &str = "CRONUS International Ltd.";

/// A database whose session company holds one row of table "DC Row", Id 7, and its own Company row.
struct SessionRow;

impl al_host::testdata::TableProvider for SessionRow {
    fn company(&self) -> &str {
        SESSION
    }
    fn companies(&self) -> Vec<String> {
        vec![SESSION.into()]
    }
    fn source(&self) -> String {
        "session-row.bak".into()
    }
    fn read(
        &self,
        t: &al_host::testdata::TableRequest,
    ) -> Result<Arc<al_host::testdata::TableRows>, al_host::testdata::TableError> {
        let (column, cell) = match t.name.as_str() {
            "DC Row" => ("Id", al_host::testdata::Cell::Int(7)),
            "Company" => ("Name", al_host::testdata::Cell::Text(SESSION.into())),
            _ => return Err(al_host::testdata::TableError::Absent),
        };
        Ok(Arc::new(al_host::testdata::TableRows {
            columns: vec![column.into()],
            rows: vec![vec![cell]],
        }))
    }
}

const BASE: &str = "table 2000000006 Company { DataPerCompany = false;
      fields { field(1; Name; Text[30]) { } field(2; \"Evaluation Company\"; Boolean) { } field(3; \"Display Name\"; Text[250]) { } field(8000; Id; Guid) { } } }
    table 70100 \"DC Row\" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
    codeunit 70101 \"DC Installer\" { Subtype = Install;
        trigger OnInstallAppPerDatabase() var Co: Record Company; begin Co.Get('CRONUS International Ltd.'); Co.Rename('Renamed Co'); end; }";

const TESTS: &str = "codeunit 70104 \"DC Renamed Tests\" { Subtype = Test;
    [Test] procedure ReadsTheRenamedCompany() var R: Record \"DC Row\"; begin
        if not R.ChangeCompany('Renamed Co') then Error('ChangeCompany answered false');
        if R.FindFirst() then;
        Error('count=%1 id=%2', R.Count(), R.Id);
    end; }";

#[test]
fn install_code_that_renames_the_session_company_is_refused_on_every_baseline_copy() {
    let apps = [(BASE_APP, vec![], BASE), ("main", vec![BASE_APP.to_string()], TESTS)];
    let (ws, l) = al_sema::lower::compile_apps_with(
        apps.iter()
            .map(|(id, deps, src)| AppSource {
                id: (*id).into(),
                name: (*id).into(),
                dependencies: deps.clone(),
                files: vec![(format!("{id}.al"), (*src).into())],
            })
            .collect(),
        vec![al_sema::AppKind::Dependency, al_sema::AppKind::Workspace],
        &Default::default(),
        None,
        al_sema::check::Roots {
            baseline: true,
            ..Default::default()
        },
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let mut engine = al_vm::Engine::new(l.program.clone(), Default::default());
    Arc::get_mut(&mut engine).expect("a new engine").test_data =
        Some(al_host::testdata::TestData::new(Arc::new(SessionRow)));
    let run = al_testrun::baseline::run(&engine, &l, None, al_testrun::host::RunClock::begin());
    let run = run.expect("the install baseline");
    let notes = format!("{:?}", run.notes);
    let captured = run.baseline;
    let thawed = captured.freeze().expect("frozen").thaw(Some(&engine));
    let read_back = captured
        .to_bytes()
        .map(|bytes| al_vm::Baseline::from_bytes(&bytes).expect("read back").backed_by(&engine));
    let oi = l
        .program
        .objects
        .iter()
        .position(|o| o.name.eq_ignore_ascii_case("DC Renamed Tests"))
        .expect("the test codeunit");
    let answer = |b| format!("{:?}", al_testrun::runtime::run_codeunit_from(&engine, &l, oi, Some(b)));
    let first = answer(&captured);
    assert_eq!(answer(&thawed), first, "thawed");
    if let Some(read_back) = &read_back {
        assert_eq!(answer(read_back), first, "read back");
    }
    assert!(notes.contains("renaming the session company"), "{notes}");
    assert!(first.contains("ChangeCompany answered false"), "{first}");
}
