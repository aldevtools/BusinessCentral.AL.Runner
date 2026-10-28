//! Test data from a restored company (ADR-020 point 3): rows appear on the first touch of a table,
//! keep through the per-codeunit reset, and meet the install baseline in the order a restored
//! database gives them. The provider here is a fake with a handful of rows; the backup reader is
//! tested where it lives (`al-bcdb-std`).

use al_host::testdata::{Cell, TableError, TableProvider, TableRequest, TableRows, TestData, TestSequence};
use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

const DEP_ID: &str = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
const EXT_ID: &str = "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb";

struct Fake {
    company: String,
    tables: HashMap<String, TableRows>,
    asked: Mutex<Vec<String>>,
    sequences: Vec<TestSequence>,
    /// Behaves as a source of untyped values (a configuration package): `wants_fields`.
    untyped: bool,
}

impl Fake {
    fn new(company: &str) -> Fake {
        Fake {
            company: company.into(),
            tables: HashMap::new(),
            asked: Mutex::new(Vec::new()),
            sequences: Vec::new(),
            untyped: false,
        }
    }

    fn untyped(mut self) -> Fake {
        self.untyped = true;
        self
    }

    fn sequence(mut self, name: &str, company_specific: bool, start: i64, current: i64) -> Fake {
        self.sequences.push(TestSequence {
            name: name.into(),
            company_specific,
            start,
            increment: 1,
            current,
        });
        self
    }

    fn table(mut self, name: &str, columns: &[&str], rows: Vec<Vec<Cell>>) -> Fake {
        self.tables.insert(
            name.to_lowercase(),
            TableRows {
                columns: columns.iter().map(|c| c.to_string()).collect(),
                rows,
            },
        );
        self
    }
}

impl TableProvider for Fake {
    fn company(&self) -> &str {
        &self.company
    }
    fn source(&self) -> String {
        "fake.bak".into()
    }
    fn sequences(&self) -> Vec<TestSequence> {
        self.sequences.clone()
    }
    fn wants_fields(&self) -> bool {
        self.untyped
    }
    fn read(&self, t: &TableRequest) -> Result<Arc<TableRows>, TableError> {
        self.asked.lock().unwrap().push(t.name.clone());
        match self.tables.get(&t.name.to_lowercase()) {
            Some(rows) => Ok(Arc::new(rows.clone())),
            None if t.name == "Broken" => Err(TableError::Reader("page 7 is torn".into())),
            None => Err(TableError::Absent),
        }
    }
}

fn text(s: &str) -> Cell {
    Cell::Text(s.into())
}

fn dec(units: i64) -> Cell {
    Cell::Decimal {
        negative: units < 0,
        magnitude: units.unsigned_abs() as u128 * 10u128.pow(19),
        scale: 20,
    }
}

const DEP: &str = r#"
table 50100 "Customer" { fields {
    field(1; "No."; Code[20]) { }
    field(2; Name; Text[50]) { }
    field(3; Balance; Decimal) { }
    field(4; "Reten. Pol."; Boolean) { }
    field(5; Rank; Option) { OptionMembers = Low,Mid,High; }
  } keys { key(PK; "No.") { } } }
table 50101 "Untouched" { fields { field(1; K; Code[10]) { } } keys { key(PK; K) { } } }
table 50102 "Setup" { fields { field(1; "Primary Key"; Code[10]) { } field(2; Value; Text[50]) { } } keys { key(PK; "Primary Key") { } } }
table 50103 "Broken" { fields { field(1; K; Code[10]) { } } keys { key(PK; K) { } } }
table 50104 "Odd" { fields { field(1; K; Code[10]) { } field(2; Ref; RecordId) { } } keys { key(PK; K) { } } }
table 50106 "Log" { fields { field(1; "No."; Integer) { AutoIncrement = true; } field(2; What; Text[30]) { } } keys { key(PK; "No.") { } } }
table 50107 "Dim Val" { fields { field(1; Code; Code[10]) { } field(2; "Value ID"; Integer) { AutoIncrement = true; } } keys { key(PK; Code) { } } }
table 50105 "Shared" { DataPerCompany = false; fields { field(1; K; Code[10]) { } field(2; V; Integer) { } } keys { key(PK; K) { } } }
"#;

const EXT: &str = r#"
tableextension 50200 "Customer Ext" extends "Customer" { fields { field(50200; Extra; Text[20]) { } } }
"#;

fn run(main: &str, dep: &str, data: Option<&Arc<Fake>>) -> (CorpusRuntime, Option<TestData>) {
    let app = |id: &str, deps: &[&str], src: &str| AppSource {
        id: id.into(),
        name: id.into(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![(format!("{id}.al"), src.into())],
    };
    let test_data = data.map(|f| TestData::new(f.clone()));
    let rt = CorpusRuntime::run_suites_with(
        vec![
            app("main", &[DEP_ID, EXT_ID], main),
            app(DEP_ID, &[], dep),
            app(EXT_ID, &[DEP_ID], EXT),
        ],
        vec![
            al_sema::AppKind::Workspace,
            al_sema::AppKind::Dependency,
            al_sema::AppKind::Dependency,
        ],
        &[],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions {
            test_data: test_data.clone(),
            ..Default::default()
        },
    );
    (rt, test_data)
}

fn outcome(rt: &CorpusRuntime, codeunit: u32, name: &str) -> Outcome {
    rt.first_outcome(&CaseId {
        codeunit,
        name: name.into(),
    })
}

fn customers() -> Fake {
    Fake::new("CRONUS International Ltd.").table(
        "Customer",
        &[
            "No_",
            "Name",
            "Balance",
            "Reten_ Pol_",
            "Rank",
            "Extra$bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb",
            "Gone$cccccccc-cccc-cccc-cccc-cccccccccccc",
            "Ghost",
        ],
        vec![
            vec![
                text("10000"),
                text("Adatum"),
                dec(125),
                Cell::Int(1),
                Cell::Int(2),
                text("x1"),
                text("g"),
                text("?"),
            ],
            vec![
                text("20000"),
                text("Trey"),
                dec(-3),
                Cell::Int(0),
                Cell::Int(0),
                Cell::Null,
                text("g"),
                text("?"),
            ],
        ],
    )
}

#[test]
fn a_touched_table_holds_the_restored_rows_and_the_session_runs_in_the_restored_company() {
    let fake = Arc::new(customers());
    let (rt, data) = run(
        r#"codeunit 50001 "T" { Subtype = Test;
  [Test] procedure Rows() var C: Record "Customer"; begin
    if C.Count() <> 2 then Error('count %1', C.Count());
    C.Get('10000');
    if C.Name <> 'Adatum' then Error('name %1', C.Name);
    if C.Balance <> 12.5 then Error('balance %1', C.Balance);
    if not C."Reten. Pol." then Error('boolean');
    if C.Rank <> C.Rank::High then Error('option');
    if C.Extra <> 'x1' then Error('extension field %1', C.Extra);
    C.Get('20000');
    if C.Balance <> -0.3 then Error('negative %1', C.Balance);
  end;
  [Test] procedure Company() begin
    if CompanyName() <> 'CRONUS International Ltd.' then Error(CompanyName());
  end; }"#,
        DEP,
        Some(&fake),
    );
    assert_eq!(outcome(&rt, 50001, "Rows"), Outcome::Pass);
    assert_eq!(outcome(&rt, 50001, "Company"), Outcome::Pass);
    let s = data.unwrap().stats.summary();
    assert_eq!((s.tables, s.rows), (1, 2));
    assert_eq!(
        s.dropped_uninstalled_app, 1,
        "the column of an app the run does not install"
    );
    assert_eq!(s.dropped_no_such_field, 1, "the column no field has");
}

fn counted(rt: &CorpusRuntime, codeunit: u32, name: &str) -> Outcome {
    outcome(rt, codeunit, name)
}

#[test]
fn only_the_touched_tables_are_read_and_each_exclusion_is_counted() {
    let fake = Arc::new(
        customers()
            .table("Untouched", &["K"], vec![vec![text("U1")]])
            .table(
                "Odd",
                &["K", "Ref"],
                vec![vec![text("O1"), Cell::Binary(vec![1, 2, 3])]],
            )
            .table("Shared", &["K", "V"], vec![vec![text("S1"), Cell::Int(5)]]),
    );
    let (rt, data) = run(
        r#"codeunit 50001 "T" { Subtype = Test;
  [Test] procedure Absent() var S: Record "Setup"; begin if S.Count() <> 0 then Error('rows'); end;
  [Test] procedure BrokenReader() var B: Record "Broken"; begin if B.Count() <> 0 then Error('rows'); end;
  [Test] procedure UnsupportedValue() var O: Record "Odd"; begin if O.Count() <> 0 then Error('rows'); end;
  [Test] procedure NotPerCompany() var S: Record "Shared"; begin S.Get('S1'); if S.V <> 5 then Error('v'); end; }"#,
        DEP,
        Some(&fake),
    );
    for t in ["Absent", "BrokenReader", "UnsupportedValue", "NotPerCompany"] {
        assert_eq!(counted(&rt, 50001, t), Outcome::Pass, "{t}");
    }
    let mut asked = fake.asked.lock().unwrap().clone();
    asked.sort();
    assert_eq!(
        asked,
        ["Broken", "Odd", "Setup", "Shared"],
        "Untouched and Customer were never read"
    );
    let s = data.unwrap().stats.summary();
    assert_eq!(s.tables, 1, "Shared");
    assert_eq!(s.empty, 1, "Setup has no rows in the database");
    assert_eq!(s.reader.len(), 1);
    assert!(s.reader[0].contains("page 7 is torn"), "{:?}", s.reader);
    assert_eq!(s.unsupported.len(), 1);
    assert!(
        s.unsupported[0].contains("Ref") && s.unsupported[0].contains("not rebuilt"),
        "{:?}",
        s.unsupported
    );
}

#[test]
fn restored_rows_survive_the_reset_between_test_codeunits_and_are_read_once() {
    let fake = Arc::new(customers());
    let codeunit = |id: u32, body: &str| {
        format!(
            "codeunit {id} \"T{id}\" {{ Subtype = Test; [Test] procedure Go() var C: Record \"Customer\"; begin {body} end; }}\n"
        )
    };
    let mut main = codeunit(
        50001,
        "if C.Count() <> 2 then Error('first sees %1', C.Count()); C.DeleteAll(); C.Init(); C.\"No.\" := '30000'; C.Insert(); Commit();",
    );
    for id in 50002..=50009 {
        main += &codeunit(
            id,
            "if C.Count() <> 2 then Error('after reset %1', C.Count()); C.Get('10000');",
        );
    }
    let (rt, _) = run(&main, DEP, Some(&fake));
    for id in 50001..=50009 {
        assert_eq!(counted(&rt, id, "Go"), Outcome::Pass, "codeunit {id}");
    }
    let asked = fake.asked.lock().unwrap().iter().filter(|t| *t == "Customer").count();
    assert_eq!(asked, 1, "nine codeunits on several threads read the table once");
}

const INSTALLERS: &str = r#"
codeunit 50110 "Installer A" { Subtype = Install;
  trigger OnInstallAppPerCompany() var S: Record "Setup"; begin
    if not S.Get('') then begin S.Value := 'installed'; S.Insert(); end;
    S.Init(); S."Primary Key" := 'NEW'; S.Value := 'new-from-install'; S.Insert();
  end; }
codeunit 50111 "Installer B" { Subtype = Install;
  trigger OnInstallAppPerCompany() var S: Record "Setup"; begin
    S.Get(''); S.Value := 'overwritten'; S.Modify();
  end; }
codeunit 50112 "Installer C" { Subtype = Install;
  trigger OnInstallAppPerCompany() var S: Record "Setup"; begin
    S.Init(); S."Primary Key" := ''; S.Value := 'duplicate'; S.Insert();
  end; }
"#;

#[test]
fn install_code_sees_the_restored_rows_may_change_them_and_cannot_duplicate_them() {
    let fake = Arc::new(Fake::new("CRONUS International Ltd.").table(
        "Setup",
        &["Primary Key", "Value"],
        vec![vec![text(""), text("restored")]],
    ));
    let (rt, _) = run(
        r#"codeunit 50001 "T" { Subtype = Test;
  [Test] procedure Restored() var S: Record "Setup"; begin
    S.Get('');
    // Installer B rewrote the restored row (ADR-020 amendment: install code may change restored
    // rows); Installer C's insert of the restored key failed with 'already exists' and left no row.
    if S.Value <> 'overwritten' then Error('value %1', S.Value);
    if S.Count() <> 2 then Error('count %1', S.Count());
    S.Get('NEW');
    if S.Value <> 'new-from-install' then Error('new row %1', S.Value);
  end; }"#,
        &format!("{DEP}{INSTALLERS}"),
        Some(&fake),
    );
    assert_eq!(outcome(&rt, 50001, "Restored"), Outcome::Pass);
}

#[test]
fn tests_are_reset_to_the_state_after_install_code_not_to_the_raw_backup() {
    // The model is AL Runner v2's (ADR-020 amendment): install code runs on top of the restored
    // rows and may change them; the state after it is what every test codeunit starts from. A table
    // no install code touched is first loaded by the test, as the backup holds it.
    let fake = Arc::new(customers().table(
        "Setup",
        &["Primary Key", "Value"],
        vec![vec![text(""), text("restored")]],
    ));
    let dep = format!(
        r#"{DEP}
codeunit 50111 "Installer B" {{ Subtype = Install;
  trigger OnInstallAppPerCompany() var S: Record "Setup"; begin
    S.Get(''); S.Value := 'after-install'; S.Modify();
  end; }}
"#
    );
    let main = r#"
codeunit 50001 "T1" { Subtype = Test; [Test] procedure Go() var S: Record "Setup"; C: Record "Customer"; begin
  S.Get(''); if S.Value <> 'after-install' then Error('first sees %1', S.Value);
  if C.Count() <> 2 then Error('customers %1', C.Count());
  S.Value := 'by-test-1'; S.Modify(); C.DeleteAll(); Commit(); end; }
codeunit 50002 "T2" { Subtype = Test; [Test] procedure Go() var S: Record "Setup"; C: Record "Customer"; begin
  S.Get(''); if S.Value <> 'after-install' then Error('second sees %1', S.Value);
  if C.Count() <> 2 then Error('customers after reset %1', C.Count()); end; }
"#;
    let (rt, _) = run(main, &dep, Some(&fake));
    assert_eq!(counted(&rt, 50001, "Go"), Outcome::Pass);
    assert_eq!(counted(&rt, 50002, "Go"), Outcome::Pass);
}

#[test]
fn an_auto_increment_key_continues_after_the_restored_rows() {
    let fake = Arc::new(Fake::new("CRONUS International Ltd.").table(
        "Log",
        &["No_", "What"],
        vec![
            vec![Cell::Int(1), text("a")],
            vec![Cell::Int(7), text("b")],
            vec![Cell::Int(3), text("c")],
        ],
    ));
    let (rt, _) = run(
        r#"codeunit 50001 "T" { Subtype = Test;
  [Test] procedure Next() var L: Record "Log"; begin
    L.What := 'new'; L.Insert();
    if L."No." <> 8 then Error('first new key %1', L."No.");
    Clear(L); L.What := 'newer'; L.Insert();
    if L."No." <> 9 then Error('second new key %1', L."No.");
  end;
  [Test] procedure AnExplicitRestoredKeyIsStillTaken() var L: Record "Log"; begin
    L.Init(); L."No." := 3; L.What := 'x';
    if L.Insert() then Error('3 is a restored key');
  end; }"#,
        DEP,
        Some(&fake),
    );
    assert_eq!(outcome(&rt, 50001, "Next"), Outcome::Pass);
    assert_eq!(outcome(&rt, 50001, "AnExplicitRestoredKeyIsStillTaken"), Outcome::Pass);
}

/// A configuration package holds no value for an AutoIncrement field (Dimension Value."Dimension Value ID" is not in the
/// shipped package); the database numbers such rows 1, 2, 3 in the order they were imported. Microsoft's Sales-Calc.
/// Discount Test and ERM Sales Quotes need a nonzero id there (unprobed: docs/probes-needed.md, registry entry
/// `package-auto-increment-from-one`). A backup, which holds the column, keeps its own values.
#[test]
fn an_auto_increment_field_the_package_does_not_hold_is_numbered_in_row_order() {
    let fake = Arc::new(Fake::new("CRONUS International Ltd.").untyped().table(
        "Dim Val",
        &["Code"],
        vec![vec![text("B")], vec![text("A")], vec![text("C")]],
    ));
    let (rt, _) = run(
        r#"codeunit 50001 "T" { Subtype = Test;
  [Test] procedure Go() var D: Record "Dim Val"; begin
    D.Get('B'); if D."Value ID" <> 1 then Error('B %1', D."Value ID");
    D.Get('A'); if D."Value ID" <> 2 then Error('A %1', D."Value ID");
    D.Get('C'); if D."Value ID" <> 3 then Error('C %1', D."Value ID");
    Clear(D); D.Code := 'N'; D.Insert();
    if D."Value ID" <> 4 then Error('new %1', D."Value ID");
  end; }"#,
        DEP,
        Some(&fake),
    );
    assert_eq!(outcome(&rt, 50001, "Go"), Outcome::Pass);
}

#[test]
fn an_auto_increment_field_the_backup_holds_keeps_its_values() {
    let fake = Arc::new(Fake::new("CRONUS International Ltd.").table(
        "Dim Val",
        &["Code", "Value ID"],
        vec![vec![text("B"), Cell::Int(40)], vec![text("A"), Cell::Int(5)]],
    ));
    let (rt, _) = run(
        r#"codeunit 50001 "T" { Subtype = Test;
  [Test] procedure Go() var D: Record "Dim Val"; begin
    D.Get('B'); if D."Value ID" <> 40 then Error('B %1', D."Value ID");
    Clear(D); D.Code := 'N'; D.Insert();
    if D."Value ID" <> 41 then Error('new %1', D."Value ID");
  end; }"#,
        DEP,
        Some(&fake),
    );
    assert_eq!(outcome(&rt, 50001, "Go"), Outcome::Pass);
}

#[test]
fn a_table_the_backup_holds_rows_for_is_seeded_even_when_the_root_that_writes_it_fails() {
    // Company-Initialize stops before it writes Setup (Debugger.Deactivate is not implemented).
    // Without test data Setup is "unseeded" and a Get that finds nothing in dependency code is
    // refused; with the backup's rows it is an ordinary missing row.
    let dep = format!(
        r#"{DEP}
codeunit 2 "Company-Initialize" {{ trigger OnRun() var S: Record "Setup"; begin
  Debugger.Deactivate(); S."Primary Key" := ''; S.Insert(); end; }}
codeunit 50120 "Lib" {{ procedure Missing() var S: Record "Setup"; begin S.Get('NOPE'); end; }}
"#
    );
    let main = r#"codeunit 50001 "T" { Subtype = Test;
  [Test] procedure Missing() var L: Codeunit "Lib"; begin L.Missing(); end; }"#;
    let plain = run(main, &dep, None).0;
    assert!(
        matches!(outcome(&plain, 50001, "Missing"), Outcome::NotRun(_)),
        "{:?}",
        outcome(&plain, 50001, "Missing")
    );
    let fake = Arc::new(Fake::new("CRONUS International Ltd.").table(
        "Setup",
        &["Primary Key", "Value"],
        vec![vec![text(""), text("restored")]],
    ));
    let (rt, _) = run(main, &dep, Some(&fake));
    match outcome(&rt, 50001, "Missing") {
        Outcome::Fail(m) => assert!(m.contains("does not exist"), "{m}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_company_independent_table_the_backup_holds_rows_for_is_seeded_too() {
    // Application Area Setup is such a table (DataPerCompany = false): the backup keeps its rows
    // under no company, and the session works in the restored company. The check for restored
    // rows must ask for the table's own slot, or 563 Tests-ERM tests stay not run.
    let dep = format!(
        r#"{DEP}
codeunit 2 "Company-Initialize" {{ trigger OnRun() var S: Record "Shared"; begin
  Debugger.Deactivate(); S.K := 'A'; S.Insert(); end; }}
codeunit 50120 "Lib" {{ procedure Missing() var S: Record "Shared"; begin S.Get('NOPE'); end; }}
"#
    );
    let main = r#"codeunit 50001 "T" { Subtype = Test;
  [Test] procedure Missing() var L: Codeunit "Lib"; begin L.Missing(); end; }"#;
    let fake = Arc::new(Fake::new("CRONUS International Ltd.").table(
        "Shared",
        &["K", "V"],
        vec![vec![text("A"), Cell::Int(1)]],
    ));
    let (rt, _) = run(main, &dep, Some(&fake));
    match outcome(&rt, 50001, "Missing") {
        Outcome::Fail(m) => assert!(m.contains("does not exist"), "{m}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn the_install_code_of_the_app_under_test_may_change_the_restored_rows() {
    // ADR-020 (decided without owner review, 2026-10-14): the guard that keeps install code from
    // rewriting the restored company protects the dependency apps' install roots; the tested app's
    // own install codeunit is the project's fixture (a real project's seeds the setup tables of
    // the demo company), and publishing it on a real database may change those rows.
    let fake = Arc::new(Fake::new("CRONUS International Ltd.").table(
        "Setup",
        &["Primary Key", "Value"],
        vec![vec![text(""), text("restored")]],
    ));
    let (rt, _) = run(
        r#"codeunit 50001 "T" { Subtype = Test;
  [Test] procedure Changed() var S: Record "Setup"; begin
    S.Get('');
    if S.Value <> 'fixture' then Error('value %1', S.Value);
  end; }
codeunit 50002 "Fixture" { Subtype = Install;
  trigger OnInstallAppPerCompany() var S: Record "Setup"; begin
    S.Get(''); S.Value := 'fixture'; S.Modify();
  end; }"#,
        DEP,
        Some(&fake),
    );
    assert_eq!(outcome(&rt, 50001, "Changed"), Outcome::Pass);
}

#[test]
fn a_get_in_the_tests_own_code_on_a_table_the_baseline_could_not_seed_is_refused_too() {
    // The same unseeded table as above, read by the test code itself (a real project's tests read
    // the rows its install code creates): "does not exist" would be a wrong failure, not BC's answer.
    let dep = format!(
        r#"{DEP}
codeunit 2 "Company-Initialize" {{ trigger OnRun() var S: Record "Setup"; begin
  Debugger.Deactivate(); S."Primary Key" := ''; S.Insert(); end; }}
"#
    );
    let main = r#"codeunit 50001 "T" { Subtype = Test;
  [Test] procedure Own() var S: Record "Setup"; begin S.Get('NOPE'); end;
  [Test] procedure OwnWithResult() var S: Record "Setup"; begin if S.Get('NOPE') then Error('found'); end; }"#;
    let (rt, _) = run(main, &dep, None);
    for name in ["Own", "OwnWithResult"] {
        assert!(
            matches!(outcome(&rt, 50001, name), Outcome::NotRun(_)),
            "{name}: {:?}",
            outcome(&rt, 50001, name)
        );
    }
}

/// BC 29 keeps the number series of a restored database as SQL sequences (No. Series lines with the Sequence
/// implementation). A run on the backup must continue where the backup left them: the next contact number is the one
/// after the last contact (Tests-Marketing, Test Contact Lookup, CheckBillToContactWhenUpdateBillToContactToAnotherContact
/// failed with "The record in table Contact already exists. No.='CT000001'").
#[test]
fn number_sequences_of_the_backup_continue_where_it_left_them() {
    let fake = Arc::new(
        customers()
            .sequence("SEQ-A", true, 0, 24)
            .sequence("SEQ-UNUSED", true, 5, 5)
            .sequence("SEQ-GLOBAL", false, 0, 7),
    );
    let (rt, _) = run(
        r#"codeunit 50001 "T" { Subtype = Test;
  [Test] procedure Continues() begin
    if not NumberSequence.Exists('SEQ-A') then Error('exists');
    if NumberSequence.Current('SEQ-A') <> 24 then Error('current %1', NumberSequence.Current('SEQ-A'));
    if NumberSequence.Next('SEQ-A') <> 25 then Error('next');
  end;
  [Test] procedure Unused() begin
    if NumberSequence.Next('SEQ-UNUSED') <> 5 then Error('first value is the start');
  end;
  [Test] procedure Global() begin
    if not NumberSequence.Exists('SEQ-GLOBAL', false) then Error('global exists');
    if NumberSequence.Exists('SEQ-GLOBAL') then Error('not company specific');
    if NumberSequence.Next('SEQ-GLOBAL', false) <> 8 then Error('global next');
  end;
  [Test] procedure Missing() begin
    if NumberSequence.Exists('NOPE') then Error('exists');
  end; }"#,
        DEP,
        Some(&fake),
    );
    for t in ["Continues", "Unused", "Global", "Missing"] {
        assert_eq!(counted(&rt, 50001, t), Outcome::Pass, "{t}");
    }
}
