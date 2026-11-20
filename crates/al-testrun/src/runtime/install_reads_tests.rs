//! Install code that could not run and only reads a table (al-runner issue 8238; assumption
//! `table-reads-raise-no-table-event`, corpus PR 1431, open): the subscribers of that table's table events are left out of
//! what it leaves unknown, and the code that reads only what they would have written runs, marked.
use super::app_outcomes_tests::outcomes_and_hits_of_apps;
use super::*;

/// A dependency app with the platform table Company.
const DEP: &str = "table 2000000006 Company { DataPerCompany = false;
        fields { field(1; Name; Text[30]) { } field(2; \"Evaluation Company\"; Boolean) { } field(3; \"Display Name\"; Text[250]) { }
            field(8000; Id; Guid) { } } keys { key(PK; Name) { } } }";

/// The app under test: its install trigger inserts a row of `Install Row`, stops (a group 1 filter on a temporary record,
/// scenario s15 of al-runner issue 8115) and then calls a procedure whose body is `company` on a Company record
/// (`Company`, declared `company_var`). A subscriber of Company's OnAfterDeleteEvent inserts a row of `Open Row`.
/// `extra` adds objects; the tests read `Open Row`, `Install Row` and no table.
fn app(company_var: &str, company: &str, extra: &str) -> String {
    app_vars(&format!("Company: {company_var};"), company, extra)
}

/// [`app`] with the whole variable list `vars` of the procedure the install trigger calls after the stop.
fn app_vars(vars: &str, company: &str, extra: &str) -> String {
    format!(
        "table 60970 \"Install Row\" {{ fields {{ field(1; Code; Code[20]) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}
        table 60971 \"Open Row\" {{ fields {{ field(1; Code; Code[20]) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}
        codeunit 60972 Inst {{ Subtype = Install;
            trigger OnInstallAppPerCompany() var Row: Record \"Install Row\"; T: Record \"Install Row\" temporary;
            begin Row.Code := 'A'; if Row.Insert() then; T.FilterGroup(1); T.SetRange(Code, 'A'); T.FilterGroup(0); Companies(); end;
            local procedure Companies() var {vars} begin {company} end; }}
        codeunit 60977 \"Company Deleted\" {{
            [EventSubscriber(ObjectType::Table, Database::Company, 'OnAfterDeleteEvent', '', false, false)]
            local procedure Deleted(var Rec: Record Company; RunTrigger: Boolean) var R: Record \"Open Row\";
            begin R.Code := 'B'; if R.Insert() then; end; }}
        {extra}
        codeunit 60973 T {{ Subtype = Test;
            [Test] procedure ReadsOpenRow() var R: Record \"Open Row\"; begin if R.Get('B') then; end;
            [Test] procedure ReadsInstallRow() var R: Record \"Install Row\"; begin R.Get('A'); end;
            [Test] procedure NoTable() begin end; }}"
    )
}

const COMPANY: &str = "Record Company";
const IS_EMPTY: &str = "if Company.IsEmpty() then exit;";
const ASSUMED: &str = "table-reads-raise-no-table-event";

fn run(a: &str) -> OutcomesAndHitsByName {
    let (o, h) = outcomes_and_hits_of_apps(&[("dep", &[], DEP), ("a", &["dep"], a)], "T");
    OutcomesAndHitsByName { o, h }
}

struct OutcomesAndHitsByName {
    o: Vec<(String, Outcome)>,
    h: Vec<(String, Vec<&'static str>)>,
}

impl OutcomesAndHitsByName {
    fn outcome(&self, name: &str) -> &Outcome {
        &self
            .o
            .iter()
            .find(|(n, _)| n == name)
            .unwrap_or_else(|| panic!("{name}: {:?}", self.o))
            .1
    }
    fn marked(&self, name: &str) -> bool {
        self.h.iter().any(|(n, h)| n == name && h.contains(&ASSUMED))
    }
}

fn not_run(o: &Outcome) -> bool {
    matches!(o, Outcome::NotRun(_))
}

// The mechanism of al-runner issue 8238 (a stand-in, not the reporter's repro): the stopped install routine only calls
// Company.IsEmpty() (TiCompanyReadEventTests.al:26-70 of corpus PR 1431: no table event). A test that reads a table only
// the Company subscriber's reach writes runs, marked; one that reads the routine's own table is not run; one that reads
// no table runs unmarked.
#[test]
fn a_test_that_reads_only_what_a_table_event_subscriber_would_write_runs_marked() {
    let r = run(&app(COMPANY, IS_EMPTY, ""));
    assert_eq!(r.outcome("ReadsOpenRow"), &Outcome::Pass, "{:?}", r.o);
    assert!(r.marked("ReadsOpenRow"), "{:?}", r.h);
    assert!(not_run(r.outcome("ReadsInstallRow")), "{:?}", r.o);
    assert_eq!(r.outcome("NoTable"), &Outcome::Pass, "{:?}", r.o);
    assert!(!r.marked("NoTable"), "{:?}", r.h);
}

// Neighbors that keep main's result: a Company call the probe did not measure (Init), a temporary Company record, and a
// write that raises the table event (Delete) leave the subscriber in the reach, so a test that reads `Open Row` is not run.
#[test]
fn an_unmeasured_use_of_the_table_keeps_the_full_reach() {
    for (var, body) in [
        (COMPANY, "Company.Init();"),
        ("Record Company temporary", IS_EMPTY),
        (COMPANY, "if Company.Get('X') then Company.Delete();"),
    ] {
        let r = run(&app(var, body, ""));
        assert!(not_run(r.outcome("ReadsOpenRow")), "{var} {body}: {:?}", r.o);
        assert_eq!(r.outcome("NoTable"), &Outcome::Pass, "{var} {body}: {:?}", r.o);
    }
}

// The test codeunit's OnRun counts toward no test: its read of a table only the left-out subscriber writes refuses the
// codeunit, as on main.
#[test]
fn an_on_run_that_reads_an_assumed_table_is_refused_as_before() {
    let a = app(COMPANY, IS_EMPTY, "").replace(
        "codeunit 60973 T { Subtype = Test;",
        "codeunit 60973 T { Subtype = Test; trigger OnRun() var R: Record \"Open Row\"; begin if R.Get('B') then; end;",
    );
    let r = run(&a);
    assert!(r.o.iter().all(|(_, o)| not_run(o)), "{:?}", r.o);
}

// The code that runs when the test session opens the company reads a table only the left-out subscriber writes: the
// codeunit runs, and every test of the session is marked (`run.rs` `open_company`).
#[test]
fn a_company_open_subscriber_that_reads_an_assumed_table_marks_every_test() {
    let open = "codeunit 60974 Opens {
        [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Company Triggers\", 'OnCompanyOpenCompleted', '', false, false)]
        local procedure Opened() var R: Record \"Open Row\"; begin if R.Get('B') then; end; }";
    let r = run(&app(COMPANY, IS_EMPTY, open));
    assert_eq!(r.outcome("NoTable"), &Outcome::Pass, "{:?}", r.o);
    assert!(r.marked("NoTable"), "{:?}", r.h);
    assert!(not_run(r.outcome("ReadsInstallRow")), "{:?}", r.o);
}

// Install code that runs after the stopped routine and reads a table only the left-out subscriber writes is refused, as
// on main: its marks would count for no test.
#[test]
fn later_install_code_that_reads_an_assumed_table_is_refused_as_before() {
    let later = "table 60975 Seen { fields { field(1; K; Code[10]) { } field(2; Found; Boolean) { } } keys { key(PK; K) { } } }
        codeunit 60976 Inst2 { Subtype = Install;
            trigger OnInstallAppPerCompany() var R: Record \"Open Row\"; S: Record Seen; begin S.Found := R.Get('B'); if S.Insert() then; end; }";
    let a = app(COMPANY, IS_EMPTY, later).replace(
        "[Test] procedure NoTable() begin end;",
        "[Test] procedure NoTable() begin end; [Test] procedure ReadsSeen() var S: Record Seen; begin if S.Get() then; end;",
    );
    let r = run(&a);
    assert!(not_run(r.outcome("ReadsSeen")), "{:?}", r.o);
    assert_eq!(r.outcome("NoTable"), &Outcome::Pass, "{:?}", r.o);
}

/// The stopped routine reads Company with a measured call (`Company.IsEmpty()`) and changes Company another way: the
/// full reach stays (main's result), so the test that reads what the Company delete subscriber writes is not run, and the
/// test that reads no table passes. `vars`: the locals of the procedure after the stop; `extra`: more objects.
fn assert_full_reach(vars: &str, body: &str, extra: &str) {
    let r = run(&app_vars(
        &format!("Company: Record Company; {vars}"),
        &format!("{IS_EMPTY_THEN} {body}"),
        extra,
    ));
    assert!(not_run(r.outcome("ReadsOpenRow")), "{vars} {body}: {:?}", r.o);
    assert_eq!(r.outcome("NoTable"), &Outcome::Pass, "{vars} {body}: {:?}", r.o);
}

const IS_EMPTY_THEN: &str = "if Company.IsEmpty() then;";

// Review of PR 8275, shape d_array_insert: a record of an array of Company records is deleted.
#[test]
fn an_array_of_the_table_s_records_keeps_the_full_reach() {
    assert_full_reach("Rows: array[2] of Record Company;", "if Rows[1].Delete() then;", "");
}

// Review of PR 8275, shapes e_recref_assert and co_company_recref_delete: a RecordRef opened on Company deletes.
#[test]
fn a_recordref_opened_on_the_table_keeps_the_full_reach() {
    assert_full_reach(
        "RecRef: RecordRef;",
        "RecRef.Open(Database::Company); if RecRef.Delete() then;",
        "",
    );
}

// Review of PR 8275, shape e2_recref_const_callee: the RecordRef that deletes is in a callee with no Company record.
#[test]
fn a_recordref_opened_on_the_table_in_a_callee_keeps_the_full_reach() {
    let callee = "codeunit 60978 Del { procedure Go() var RecRef: RecordRef; begin RecRef.Open(Database::Company); if RecRef.Delete() then; end; }";
    assert_full_reach("D: Codeunit Del;", "D.Go();", callee);
}

// Review of PR 8275, shape e3_fieldref_validate: a FieldRef of a RecordRef opened on Company validates (no write).
#[test]
fn a_fieldref_validate_on_the_table_keeps_the_full_reach() {
    assert_full_reach(
        "RecRef: RecordRef; FldRef: FieldRef;",
        "RecRef.Open(Database::Company); FldRef := RecRef.Field(3); FldRef.Validate('X');",
        "",
    );
}

// Review of PR 8275, shapes c2_return_assert and c_named_return_delete: a record a callee returns is deleted; the callee
// only reads its own record.
#[test]
fn a_returned_record_of_the_table_keeps_the_full_reach() {
    let callee = "codeunit 60978 Src { procedure GetRow() R: Record Company; begin if R.Get('X') then; end; }";
    assert_full_reach("S: Codeunit Src;", "if S.GetRow().Delete() then;", callee);
}

// Review of PR 8275, shape r_codeunit_run_assert: a codeunit with TableNo = Company, run by its id, deletes Rec.
#[test]
fn a_table_no_codeunit_run_by_id_keeps_the_full_reach() {
    let w = "codeunit 60978 W { TableNo = Company; trigger OnRun() begin if Rec.Delete() then; end; }";
    assert_full_reach("", "Codeunit.Run(Codeunit::W);", w);
}
