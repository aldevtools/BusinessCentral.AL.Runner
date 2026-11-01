// The scans of production code stop at the first `#[cfg(test)]` of a file; a test module in a file of its own starts with one.
#[cfg(test)]
use super::*;
use al_sema::workspace::AppSource;

fn outcomes(src: &str) -> Vec<(String, Outcome)> {
    let (ws, l) = al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), src.into())],
        }],
        &Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let engine = Engine::new(l.program.clone(), blocked);
    let oi = l
        .program
        .objects
        .iter()
        .position(|o| {
            o.properties
                .iter()
                .any(|(k, v)| k.eq_ignore_ascii_case("Subtype") && v.eq_ignore_ascii_case("Test"))
        })
        .unwrap();
    run_codeunit(&engine, &l, oi)
}

const SRC: &str = "codeunit 60100 T
{
    Subtype = Test;
    var
        Seen: Integer;

    trigger OnRun()
    begin
        Seen := 41;
    end;

    [Test]
    procedure A_Passes()
    begin
        Seen += 1;
        if Seen <> 42 then Error('OnRun did not run first: %1', Seen);
    end;

    [Test]
    procedure B_Fails()
    begin
        Error('expected failure');
    end;

    [Test]
    procedure C_NotRun()
    begin
        Helper();
    end;

    local procedure Helper()
    begin
        Debugger.Deactivate();
    end;

    procedure NotATest()
    begin
        Error('must not run');
    end;

    [Test]
    [HandlerFunctions('MsgHandler')]
    procedure D_NeedsHandler()
    begin
    end;

    [MessageHandler]
    procedure MsgHandler(Msg: Text)
    begin
    end;
}";

#[test]
fn runs_tests_in_order_after_onrun_on_one_instance() {
    let o = outcomes(SRC);
    let names: Vec<_> = o.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, vec!["A_Passes", "B_Fails", "C_NotRun", "D_NeedsHandler"]);
    assert_eq!(o[0].1, Outcome::Pass);
    assert_eq!(o[1].1, Outcome::Fail("expected failure".into()));
}

#[test]
fn unreachable_code_does_not_block_but_reachable_unsupported_code_does() {
    let o = outcomes(SRC);
    assert!(
        matches!(&o[2].1, Outcome::NotRun(r) if r.contains("Debugger")),
        "{:?}",
        o[2].1
    );
}

fn outcomes_of(src: &str, codeunit: &str) -> Vec<(String, Outcome)> {
    let (ws, l) = al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), src.into())],
        }],
        &Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let engine = Engine::new(l.program.clone(), blocked);
    let oi = l.program.objects.iter().position(|o| o.name == codeunit).unwrap();
    run_codeunit(&engine, &l, oi)
}

fn with_rows(body: &str) -> String {
    format!("{ROWS}\n{body}")
}

const ROWS: &str = "table 60900 S { fields { field(1; K; Code[10]) { } } }
        codeunit 60910 Inserter { trigger OnRun() var R: Record S; begin R.K := 'RUN'; R.Insert(); end; }";

#[test]
fn a_failure_after_a_test_that_was_refused_mid_run_is_not_trusted() {
    // A refused test stops half way: it may have set globals the next tests rely on (an
    // "initialized" flag whose setup was rolled back). A failure that reads what it can change
    // is then not BC's answer, and is reported as not run; passes are kept, and a failure that
    // reads nothing the refused test can change stays a failure.
    let src = "table 60901 T9 { fields { field(1; K; Code[10]) { } } }
            codeunit 60103 T { Subtype = Test; var Flag: Boolean; Count: Integer;
                [Test] procedure T1() var R: Record T9; begin Flag := true; R.LockTable(true, true); Count += 1; end;
                [Test] procedure T2() begin if not Flag then Error('flag lost'); end;
                [Test] procedure T3() begin if Count = 0 then Error('real failure'); end;
                [Test] procedure T4() begin Error('real failure'); end; }
            codeunit 60104 U { Subtype = Test;
                [Test] procedure U1() begin Error('real failure'); end; }";
    let o = outcomes_of(src, "T");
    let by = |n: &str| o.iter().find(|(x, _)| x == n).map(|(_, x)| x.clone()).unwrap();
    assert!(matches!(by("T1"), Outcome::NotRun(_)), "{:?}", by("T1"));
    assert_eq!(by("T2"), Outcome::Pass);
    assert!(
        matches!(by("T3"), Outcome::NotRun(r) if r.contains("earlier test") && r.contains("LockTable") && r.contains("Count")),
        "{:?}",
        by("T3")
    );
    assert_eq!(by("T4"), Outcome::Fail("real failure".into()));
    let u = outcomes_of(src, "U");
    assert_eq!(u[0].1, Outcome::Fail("real failure".into()));
}

/// A test refused while it ran leaves the session language and WorkDate as they were when it started, as its database
/// writes are rolled back: corpus report/lt-bd-format-region/BdRegionTests.al BdSummaryML_1040 sets GlobalLanguage(1040) and
/// is refused before its own `GlobalLanguage(Original)`; the BdSetterLanguage_* tests after it must not run in 1040.
#[test]
fn a_test_refused_while_it_ran_leaves_no_session_language_or_work_date_behind() {
    let src = "table 60902 T8 { fields { field(1; K; Code[10]) { } } }
            codeunit 60106 T { Subtype = Test; var Saved: Date; Lang: Integer;
                [Test] procedure Remember() begin Saved := WorkDate(); Lang := GlobalLanguage(); end;
                [Test] procedure Refused() var R: Record T8; begin GlobalLanguage(1040); WorkDate(Saved + 1); R.LockTable(true, true); end;
                [Test] procedure Reads() begin
                    if GlobalLanguage() <> Lang then Error('language %1', GlobalLanguage());
                    if WorkDate() <> Saved then Error('work date %1', WorkDate());
                end; }";
    let o = outcomes_of(src, "T");
    let by = |n: &str| o.iter().find(|(x, _)| x == n).map(|(_, x)| x.clone()).unwrap();
    assert!(matches!(by("Refused"), Outcome::NotRun(_)), "{:?}", by("Refused"));
    assert_eq!(by("Reads"), Outcome::Pass);
}

/// al-runner issue 5610: a test refused at an HttpClient call can change only what an HTTP server received, which a later
/// test sees only by sending a request itself. A later failure that sends none is trusted.
#[test]
fn a_failure_after_a_test_refused_at_an_http_call_is_trusted_when_it_sends_no_request() {
    // A request no test handler answers is refused (ADR-027): the runtime never opens a connection.
    let src = "codeunit 60105 T { Subtype = Test;
                [Test] procedure HttpGet() var C: HttpClient; R: HttpResponseMessage; begin C.Get('https://example.com/x', R); end;
                [Test] procedure UserIdRead() begin if UserId() = '' then Error('empty user'); end;
                [Test] procedure CompanyNameRead() begin Error('company=%1', CompanyName()); end; }";
    let o = outcomes_of(src, "T");
    let by = |n: &str| o.iter().find(|(x, _)| x == n).map(|(_, x)| x.clone()).unwrap();
    assert!(matches!(by("HttpGet"), Outcome::NotRun(_)), "{:?}", by("HttpGet"));
    assert!(
        matches!(by("CompanyNameRead"), Outcome::Fail(m) if m.starts_with("company=")),
        "{:?}",
        by("CompanyNameRead")
    );
}

// record/TestTransactionModelAutoRollback.al:Test01-Test03: an AutoRollback test's write is
// rolled back when it finishes; the next default-model test's write survives.
#[test]
fn an_auto_rollback_test_is_rolled_back_when_it_finishes() {
    let o = outcomes(&with_rows(
        "            codeunit 60102 T { Subtype = Test;
                [Test] [TransactionModel(TransactionModel::AutoRollback)] procedure T1() var R: Record S; begin
                    R.K := 'A'; R.Insert(); if not R.Get('A') then Error('own write not visible'); end;
                [Test] procedure T2() var R: Record S; begin
                    if R.Get('A') then Error('the AutoRollback write survived');
                    R.K := 'B'; R.Insert(); end;
                [Test] procedure T3() var R: Record S; begin
                    if not R.Get('B') then Error('the default model write did not survive'); end; }",
    ));
    assert_eq!(
        o.iter().map(|(_, x)| x.clone()).collect::<Vec<_>>(),
        vec![Outcome::Pass; 3],
        "{o:?}"
    );
}

// record/TestTransactionModelAutoRollback.al:Test04-Test06
#[test]
fn auto_rollback_refuses_commit_anywhere_beneath_the_test_and_auto_commit_allows_it() {
    let o = outcomes(&with_rows(
            "            codeunit 60102 T { Subtype = Test;
                [Test] [TransactionModel(TransactionModel::AutoRollback)] procedure Direct() begin
                    asserterror Commit();
                    if GetLastErrorText() <> 'Tests cannot call the Commit function if TransactionModel property is set to AutoRollback.' then
                        Error('wrong text: %1', GetLastErrorText()); end;
                [Test] [TransactionModel(TransactionModel::AutoRollback)] procedure Callee() begin
                    asserterror Down();
                    if StrPos(GetLastErrorText(), 'Tests cannot call the Commit function') = 0 then
                        Error('wrong text: %1', GetLastErrorText()); end;
                [Test] [TransactionModel(TransactionModel::AutoCommit)] procedure AutoCommitAllows() var R: Record S; begin
                    R.K := 'C'; R.Insert(); Commit(); asserterror Error('x');
                    if not R.Get('C') then Error('the commit did not stick'); R.Delete(); Commit(); end;
                local procedure Down() begin Commit(); end; }"
        ));
    assert_eq!(
        o.iter().map(|(_, x)| x.clone()).collect::<Vec<_>>(),
        vec![Outcome::Pass; 3],
        "{o:?}"
    );
}

// record/TestTransactionModelAutoRollback.al:Test07 and Test08
#[test]
fn commit_behavior_ignore_exempts_the_auto_rollback_refusal_and_error_does_not_outrank_it() {
    let o = outcomes(&with_rows(
        "            codeunit 60102 T { Subtype = Test;
                [Test] [TransactionModel(TransactionModel::AutoRollback)] procedure Ignored() var R: Record S; begin
                    InsertAndCommitIgnoring();
                    asserterror Error('unrelated');
                    if R.Get('I') then Error('an ignored Commit moved the boundary'); end;
                [Test] [TransactionModel(TransactionModel::AutoRollback)] procedure ErrorBehavior() begin
                    asserterror CommitUnderError();
                    if StrPos(GetLastErrorText(), 'Tests cannot call the Commit function') = 0 then
                        Error('wrong text: %1', GetLastErrorText());
                    if StrPos(GetLastErrorText(), 'Commit is prohibited in the current scope') <> 0 then
                        Error('the CommitBehavior text won'); end;
                [CommitBehavior(CommitBehavior::Ignore)]
                local procedure InsertAndCommitIgnoring() var R: Record S; begin R.K := 'I'; R.Insert(); Commit(); end;
                [CommitBehavior(CommitBehavior::Error)]
                local procedure CommitUnderError() begin Commit(); end; }",
    ));
    assert_eq!(
        o.iter().map(|(_, x)| x.clone()).collect::<Vec<_>>(),
        vec![Outcome::Pass; 2],
        "{o:?}"
    );
}

// codeunit/TestWriteTxTestBoundary.al:Test02-Test06: no write transaction crosses a test
// boundary, and a guarded Codeunit.Run is allowed at the start of the next test.
#[test]
fn no_write_transaction_survives_a_test_boundary() {
    let o = outcomes(&with_rows(
            "            codeunit 60102 T { Subtype = Test;
                [Test] [TransactionModel(TransactionModel::AutoRollback)] procedure RollbackWrites() var R: Record S; begin
                    R.K := 'A'; R.Insert();
                    if not Database.IsInWriteTransaction() then Error('no write transaction after an insert'); end;
                [Test] [TransactionModel(TransactionModel::AutoRollback)] procedure NextStartsClean() var Ok: Boolean; begin
                    if Database.IsInWriteTransaction() then Error('inherited a write transaction');
                    Ok := Codeunit.Run(Codeunit::Inserter);
                    if not Ok then Error('guarded run refused'); end;
                [Test] procedure DefaultWrites() var R: Record S; begin
                    // The guarded run of NextStartsClean committed its row 'RUN': it survives an
                    // AutoRollback test (codeunit/TestCodeunitRunErrorTextsAndModels.al:259,
                    // AutoRollback_Test04_SuccessfulGuardedRun_RowSurvivesTheTestBoundary).
                    R.DeleteAll();
                    R.K := 'D'; R.Insert();
                    if not Database.IsInWriteTransaction() then Error('no write transaction after an insert'); end;
                [Test] procedure GuardedRunAfterDefaultWrite() var R: Record S; Ok: Boolean; begin
                    if Database.IsInWriteTransaction() then Error('the default test left a write transaction');
                    Ok := Codeunit.Run(Codeunit::Inserter);
                    if not (Ok and R.Get('D')) then Error('guarded run refused or D lost'); end; }"
        ));
    assert_eq!(
        o.iter().map(|(_, x)| x.clone()).collect::<Vec<_>>(),
        vec![Outcome::Pass; 4],
        "{o:?}"
    );
}

// codeunit/TestWriteTxTestBoundary.al:Test08-Test10: a None test starts with no
// transaction, cannot write directly, and can write inside a Codeunit.Run.
#[test]
fn a_none_test_has_no_transaction_until_it_runs_a_codeunit() {
    let o = outcomes(&with_rows(
            "            codeunit 60102 T { Subtype = Test;
                [Test] procedure Default() var R: Record S; begin R.K := 'D'; R.Insert(); end;
                [Test] [TransactionModel(TransactionModel::None)] procedure StartsClean() var R: Record S; Ok: Boolean; begin
                    if Database.IsInWriteTransaction() then Error('a None test started inside a write transaction');
                    if not R.Get('D') then Error('the previous test row is gone');
                    Ok := Codeunit.Run(Codeunit::Inserter);
                    if not Ok then Error('guarded run refused'); end;
                [Test] [TransactionModel(TransactionModel::None)] procedure CannotWrite() var R: Record S; begin
                    R.K := 'N';
                    asserterror R.Insert();
                    if GetLastErrorText() <> 'A transaction must be started before changes can be made to the database.' then
                        Error('wrong text: %1', GetLastErrorText());
                    if Database.IsInWriteTransaction() then Error('a refused write left a transaction open');
                    if R.Get('N') then Error('the refused insert wrote'); end;
                [Test] [TransactionModel(TransactionModel::None)] procedure DeleteAllUnderNone() var R: Record S; begin
                    R.DeleteAll();
                    end; }"
        ));
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
    assert_eq!(o[1].1, Outcome::Pass, "{o:?}");
    assert_eq!(o[2].1, Outcome::Pass, "{o:?}");
    // DeleteAll under None is refused with the Insert text
    // (codeunit/TestWriteTxNoneModelContracts.al:91, NoneModel_Test04_DeleteAll_IsRefused).
    assert_eq!(
        o[3].1,
        Outcome::Fail("A transaction must be started before changes can be made to the database.".into()),
        "{o:?}"
    );
}

// codeunit/TestWriteTxTestBoundary.al:Test10_ARunCodeunitMayWriteUnderNone
#[test]
fn a_codeunit_run_from_a_none_test_may_write_and_its_transaction_ends_with_it() {
    let o = outcomes(&with_rows(
        "            codeunit 60102 T { Subtype = Test;
                [Test] [TransactionModel(TransactionModel::None)] procedure Run() var R: Record S; begin
                    Codeunit.Run(Codeunit::Inserter);
                    if not R.Get('RUN') then Error('the run did not write');
                    if Database.IsInWriteTransaction() then Error('the run left a transaction open'); end; }",
    ));
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
}

// codeunit/TestCodeunitRunNestedTestCodeunit.al: Codeunit.Run of a test codeunit from a
// running test raises, guarded or not, in the static and the instance form, and runs nothing.
#[test]
fn running_a_test_codeunit_from_a_test_is_refused() {
    let o = outcomes_of(
            "codeunit 60201 Inner { Subtype = Test;
                [Test] procedure InnerTest() begin Error('the inner test ran'); end; }
            codeunit 60202 Plain { trigger OnRun() begin end; }
            codeunit 60203 Outer { Subtype = Test;
                [Test] procedure Unguarded() begin
                    asserterror Codeunit.Run(Codeunit::Inner);
                    if StrPos(GetLastErrorText(), 'You cannot nest the execution of test codeunits.') = 0 then Error('wrong text: %1', GetLastErrorText());
                    if StrPos(GetLastErrorText(), 'Test codeunit 60201 Inner was called from another test codeunit.') = 0 then Error('wrong text: %1', GetLastErrorText()); end;
                [Test] procedure Guarded() var Ok: Boolean; begin
                    Ok := true;
                    asserterror Ok := Codeunit.Run(Codeunit::Inner);
                    if not Ok then Error('a result was assigned');
                    if StrPos(GetLastErrorText(), 'You cannot nest the execution of test codeunits.') = 0 then Error('wrong text: %1', GetLastErrorText()); end;
                [Test] procedure Instance() var I: Codeunit Inner; begin
                    asserterror I.Run();
                    if StrPos(GetLastErrorText(), 'You cannot nest the execution of test codeunits.') = 0 then Error('wrong text: %1', GetLastErrorText()); end;
                [Test] procedure PlainStillRuns() var Ok: Boolean; begin
                    Ok := Codeunit.Run(Codeunit::Plain);
                    if not Ok then Error('plain codeunit failed'); end; }",
            "Outer",
        );
    assert_eq!(
        o.iter().map(|(_, x)| x.clone()).collect::<Vec<_>>(),
        vec![Outcome::Pass; 4],
        "{o:?}"
    );
}

// The default TransactionModel commits a test's work at the test boundary (corpus
// codeunit/TestWriteTxTestBoundary.al:26; record/TestCalcFormulaFlowFieldValueTests.Codeunit.al:182,
// whose asserterror rolls back to "whatever the previous test committed").
#[test]
fn a_passing_test_commits_so_a_later_asserterror_rolls_back_only_to_that_point() {
    let o = outcomes(
        "table 60900 S { fields { field(1; K; Code[10]) { } } }
            codeunit 60102 T { Subtype = Test;
                [Test] procedure A() var R: Record S; begin R.K := 'A'; R.Insert(); end;
                [Test] procedure B() var R: Record S; begin
                    R.K := 'B'; R.Insert();
                    asserterror Error('x');
                    if not R.Get('A') then Error('the first test was not committed');
                    if R.Get('B') then Error('the second tests own write was not rolled back');
                end; }",
    );
    assert_eq!(o[1].1, Outcome::Pass, "{o:?}");
}

#[test]
fn install_codeunits_run_before_the_tests() {
    // OnInstallAppPerDatabase has no company, so it writes a DataPerCompany = false table; the per-company trigger writes a
    // per-company one.
    let o = outcomes("table 60900 S { DataPerCompany = false; fields { field(1; K; Code[10]) { } } }
            table 60901 C { fields { field(1; K; Code[10]) { } } }
            codeunit 60901 Inst { Subtype = Install;
                trigger OnInstallAppPerCompany() var R: Record C; begin R.K := 'CO'; R.Insert(); end;
                trigger OnInstallAppPerDatabase() var R: Record S; begin R.K := 'DB'; R.Insert(); end; }
            codeunit 60102 T { Subtype = Test;
                [Test] procedure A() var R: Record S; C: Record C; begin if (R.Count() <> 1) or (C.Count() <> 1) then Error('install did not run'); end; }");
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
}

/// The outcomes of the test codeunit `codeunit` of a program of several apps (id, dependencies, source), in order.
fn outcomes_of_apps(apps: &[(&str, &[&str], &str)], codeunit: &str) -> Vec<(String, Outcome)> {
    let (ws, l) = al_sema::lower::compile(
        apps.iter()
            .map(|(id, deps, src)| AppSource {
                id: (*id).into(),
                name: (*id).into(),
                dependencies: deps.iter().map(|d| (*d).to_string()).collect(),
                files: vec![(format!("{id}.al"), (*src).into())],
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
    let engine = Engine::new(l.program.clone(), blocked);
    let oi = l.program.objects.iter().position(|o| o.name == codeunit).unwrap();
    run_codeunit(&engine, &l, oi)
}

// ADR-046 decision 1: an ordinary codeunit's subscribers to "Extension Triggers" run at its app's install
// (events/lt-platform-events-v/PeVFixtures.al:395-407, PeVTests.al:284-285): the per-company one in the session company,
// the per-database one with CompanyName() empty (install/lt-bf-install-probes/BfInstallTests.al:138-140, `blank`).
#[test]
fn an_extension_triggers_subscriber_of_the_app_runs_per_company_then_per_database() {
    let o = outcomes(
        "table 60900 Db { DataPerCompany = false; fields { field(1; K; Code[10]) { } field(2; Info; Text[50]) { } } }
            table 60903 Co { fields { field(1; K; Code[10]) { } field(2; Info; Text[50]) { } } }
            codeunit 60905 Subs {
                [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Extension Triggers\", 'OnInstallAppPerCompany', '', false, false)]
                local procedure PerCompany() var R: Record Co; begin R.K := 'CO'; R.Info := CompanyName(); R.Insert(); end;
                [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Extension Triggers\", 'OnInstallAppPerDatabase', '', false, false)]
                local procedure PerDatabase() var R: Record Db; begin R.K := 'DB'; R.Info := CompanyName(); R.Insert(); end; }
            codeunit 60102 T { Subtype = Test;
                [Test] procedure PerCompanyRow() var R: Record Co; begin R.Get('CO'); if R.Info <> CompanyName() then Error('company %1', R.Info); end;
                [Test] procedure PerDatabaseRow() var R: Record Db; begin R.Get('DB'); if R.Info <> '' then Error('company %1', R.Info); end; }",
    );
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
    assert_eq!(o[1].1, Outcome::Pass, "{o:?}");
}

// ADR-046 amendment 4 (owner, 2026-10-30), replacing decision 4c: the default mode models a database with one company.
// Per-company install code runs once, in that company, so a shared table that code writes holds one write, and every
// read of it is answered.
#[test]
fn a_shared_table_that_per_company_install_code_writes_holds_one_write_in_the_default_mode() {
    let o = outcomes(
        "table 60900 Log { DataPerCompany = false; fields { field(1; K; Code[10]) { } } }
            codeunit 60905 Subs {
                [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Extension Triggers\", 'OnInstallAppPerCompany', '', false, false)]
                local procedure PerCompany() var R: Record Log; begin R.K := 'CO'; R.Insert(); end; }
            codeunit 60102 T { Subtype = Test;
                [Test] procedure Counts() var R: Record Log; begin if R.Count() <> 1 then Error('rows'); end;
                [Test] procedure GetsItsRow() var R: Record Log; begin R.Get('CO'); end;
                [Test] procedure Inserts() var R: Record Log; begin R.K := 'T'; R.Insert(); end; }",
    );
    for (name, x) in &o {
        assert_eq!(*x, Outcome::Pass, "{name}: {o:?}");
    }
}

// ADR-046 amendment 3: install codeunit triggers and Extension Triggers subscribers of one app run as one sequence in
// ascending codeunit id, across the 65536 id boundary (corpus PR 810, writer UR, branch al-runtime-rs-staging-ur 1ac158e1,
// tests/al-language/install/lt-ur-block-order/UrBlockOrderTests.al:59-81, 50300 50301 50302 before 90511 90512 90515).
// A table two such routines write (a), the tables a routine writes that reads what the other writes (b), and those of
// routines sharing a SingleInstance codeunit (c) are answered; until that file is in the pinned corpus it is a flagged pass.
#[test]
fn tables_whose_install_rows_depend_on_the_order_of_the_routines_follow_ascending_id_across_blocks() {
    let main = "table 60900 Both { fields { field(1; K; Code[10]) { } } }
            table 60901 Src { fields { field(1; K; Code[10]) { } } }
            table 60902 Dst { fields { field(1; K; Code[10]) { } } }
            table 60903 ViaState { fields { field(1; K; Code[10]) { } } }
            codeunit 60906 State { SingleInstance = true; var N: Integer; procedure Next(): Integer begin N += 1; exit(N); end; }
            codeunit 70905 Subs {
                [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Extension Triggers\", 'OnInstallAppPerCompany', '', false, false)]
                local procedure PerCompany() var B: Record Both; S: Record Src; D: Record Dst; St: Codeunit State;
                begin B.K := 'S'; B.Insert(); D.K := Format(S.Count()) + '-' + Format(St.Next()); D.Insert(); end; }
            codeunit 60907 Inst { Subtype = Install;
                trigger OnInstallAppPerCompany() var B: Record Both; S: Record Src; V: Record ViaState; St: Codeunit State;
                begin B.K := 'I'; B.Insert(); S.K := 'S'; S.Insert(); V.K := Format(St.Next()); V.Insert(); end; }
            codeunit 60102 T { Subtype = Test;
                [Test] procedure ReadsBoth() var R: Record Both; begin if R.Count() <> 2 then Error('rows'); end;
                [Test] procedure ReadsDst() var R: Record Dst; begin R.FindFirst(); if R.K <> '1-2' then Error('%1', R.K); end;
                [Test] procedure ReadsViaState() var R: Record ViaState; begin R.FindFirst(); if R.K <> '1' then Error('%1', R.K); end; }";
    let (o, hits) = outcomes_and_hits_of_apps(&[("main", &[], main)], "T");
    for (n, r) in &o {
        assert_eq!(*r, Outcome::Pass, "{n}: {o:?}");
    }
    // The order is pinned (install/lt-ur-block-order/UrBlockOrderTests.al:59-81): no pass rests on an assumption.
    for (n, h) in &hits {
        assert!(h.is_empty(), "{n}: {hits:?}");
    }
}

// ADR-046 amendment 3: ids above 99999 could not be published on the sandbox (UrBlockOrderTests.al:9-11), so the order of
// a routine with such an id against another is the same integer comparison, under assumption
// `install-order-ids-above-probed-range`.
#[test]
fn install_routines_with_ids_above_99999_run_in_ascending_id_as_an_assumption() {
    let main = "table 60900 Log { fields { field(1; Id; Integer) { AutoIncrement = true; } field(2; Names; Text[100]) { } } }
            codeunit 60906 State { SingleInstance = true; var N: Text; procedure Add(X: Text): Text begin N += X; exit(N); end; }
            codeunit 130000 High { Subtype = Install;
                trigger OnInstallAppPerCompany() var L: Record Log; S: Codeunit State; begin L.Names := S.Add('H'); L.Insert(); end; }
            codeunit 70000 Mid { Subtype = Install;
                trigger OnInstallAppPerCompany() var L: Record Log; S: Codeunit State; begin L.Names := S.Add('M'); L.Insert(); end; }
            codeunit 60100 Low { Subtype = Install;
                trigger OnInstallAppPerCompany() var L: Record Log; S: Codeunit State; begin L.Names := S.Add('L'); L.Insert(); end; }
            codeunit 60102 T { Subtype = Test;
                [Test] procedure Order() var R: Record Log; begin R.FindLast(); if R.Names <> 'LMH' then Error('%1', R.Names); end; }";
    let (o, hits) = outcomes_and_hits_of_apps(&[("main", &[], main)], "T");
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
    assert!(hits[0].1.contains(&"install-order-ids-above-probed-range"), "{hits:?}");
}

// ADR-046 amendment 3: the order of two subscribers of one codeunit to one install event is not pinned (every pin has
// one subscriber per codeunit), so a table both write stays refused (A2 a).
#[test]
fn a_table_two_install_subscribers_of_one_codeunit_write_is_not_known() {
    let o = outcomes(
        "table 60900 Both { fields { field(1; K; Code[10]) { } } }
            codeunit 70905 Subs {
                [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Extension Triggers\", 'OnInstallAppPerCompany', '', false, false)]
                local procedure One() var B: Record Both; begin B.K := 'A'; B.Insert(); end;
                [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Extension Triggers\", 'OnInstallAppPerCompany', '', false, false)]
                local procedure Two() var B: Record Both; begin B.K := 'B'; B.Insert(); end; }
            codeunit 60102 T { Subtype = Test;
                [Test] procedure ReadsBoth() var R: Record Both; begin if R.Count() <> 2 then Error('rows'); end; }",
    );
    assert!(
        matches!(&o[0].1, Outcome::NotRun(r) if r.contains("subscribers of one codeunit") && r.contains("not pinned")),
        "{o:?}"
    );
}

// ADR-046 amendment 1, A2 d (kept by amendment 3 where the order is not pinned): SystemRowVersion orders rows across tables
// (install/lt-bf-install-probes/BfInstallTests.al:8-9), so in an app with two install routines in one part in different
// blocks of 65536 ids, the row version of a row install code wrote shows their order. A row
// the test wrote itself is answered.
#[test]
fn the_row_version_of_an_install_row_is_refused_when_the_order_of_the_routines_is_not_known() {
    let o = outcomes(
        "table 60900 A { fields { field(1; K; Code[10]) { } } }
            table 60901 B { fields { field(1; K; Code[10]) { } } }
            codeunit 60907 Inst { Subtype = Install;
                trigger OnInstallAppPerCompany() var R: Record A; begin R.K := 'A'; R.Insert(); end; }
            codeunit 70905 Subs {
                [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Extension Triggers\", 'OnInstallAppPerCompany', '', false, false)]
                local procedure PerCompany() var R: Record B; begin R.K := 'B'; R.Insert(); end; }
            codeunit 60102 T { Subtype = Test;
                [Test] procedure GetsTheRow() var R: Record A; begin R.Get('A'); end;
                [Test] procedure ReadsItsVersion() var R: Record A; V: BigInteger; begin R.Get('A'); V := R.SystemRowVersion; end;
                [Test] procedure ReadsOwnRowVersion() var R: Record A; V: BigInteger; begin R.K := 'T'; R.Insert(); V := R.SystemRowVersion; if V = 0 then Error('no version'); end; }",
    );
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
    assert!(
        matches!(&o[1].1, Outcome::NotRun(r) if r.contains("SystemRowVersion") && r.contains("not pinned")),
        "{o:?}"
    );
    assert_eq!(o[2].1, Outcome::Pass, "{o:?}");
}

// ADR-046 amendment 3: install codeunits of one app run in ascending object id in each company and in the per-database
// part, whatever their names and source order, and a SingleInstance codeunit's state carries from one to the next and
// starts again per part (corpus PR 807, writer UM, branch al-runtime-rs-staging-pub-um e3b2eedf,
// tests/al-language/install/lt-um-install-order/UmInstallOrder.al:120-125; fact 5 for subscribers between them,
// install/lt-bf-install-probes/BfInstallTests.al:239). Until that file is in the pinned corpus a test that reads what
// they wrote is a flagged pass.
#[test]
fn install_routines_of_one_app_run_in_ascending_object_id_and_share_single_instance_state() {
    let main = "table 60430 Log { fields { field(1; Id; Integer) { AutoIncrement = true; } field(2; Names; Text[250]) { } field(3; Calls; Integer) { } } }
        table 60429 DbLog { DataPerCompany = false; fields { field(1; Id; Integer) { AutoIncrement = true; } field(2; Names; Text[250]) { } } }
        codeunit 60436 State { SingleInstance = true; var Names: Text; Calls: Integer;
            procedure Append(N: Text): Text begin Calls += 1; Names += N + ';'; exit(Names); end;
            procedure CallCount(): Integer begin exit(Calls); end; }
        codeunit 60437 Writer {
            procedure Co(N: Text) var R: Record Log; S: Codeunit State; begin R.Names := S.Append(N); R.Calls := S.CallCount(); R.Insert(); end;
            procedure Db(N: Text) var R: Record DbLog; S: Codeunit State; begin R.Names := S.Append(N); R.Insert(); end; }
        codeunit 60435 \"Zz Install\" { Subtype = Install;
            trigger OnInstallAppPerCompany() var W: Codeunit Writer; begin W.Co('Zz'); end;
            trigger OnInstallAppPerDatabase() var W: Codeunit Writer; begin W.Db('Zz'); end; }
        codeunit 60433 \"Cu Install\" { Subtype = Install;
            trigger OnInstallAppPerCompany() var W: Codeunit Writer; begin W.Co('Cu'); end;
            trigger OnInstallAppPerDatabase() var W: Codeunit Writer; begin W.Db('Cu'); end; }
        codeunit 60431 \"Mu Install\" { Subtype = Install;
            trigger OnInstallAppPerCompany() var W: Codeunit Writer; begin W.Co('Mu'); end;
            trigger OnInstallAppPerDatabase() var W: Codeunit Writer; begin W.Db('Mu'); end; }
        codeunit 60438 Subs {
            [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Extension Triggers\", 'OnInstallAppPerCompany', '', false, false)]
            local procedure PerCompany() var W: Codeunit Writer; begin W.Co('Sub'); end; }
        codeunit 60434 \"Ay Install\" { Subtype = Install;
            trigger OnInstallAppPerCompany() var W: Codeunit Writer; begin W.Co('Ay'); end;
            trigger OnInstallAppPerDatabase() var W: Codeunit Writer; begin W.Db('Ay'); end; }
        codeunit 60432 \"b Install\" { Subtype = Install;
            trigger OnInstallAppPerCompany() var W: Codeunit Writer; begin W.Co('b'); end;
            trigger OnInstallAppPerDatabase() var W: Codeunit Writer; begin W.Db('b'); end; }
        codeunit 60102 T { Subtype = Test;
            [Test] procedure PerCompanyOrder() var R: Record Log; Seen: Text;
            begin R.SetCurrentKey(Id); if R.FindSet() then repeat Seen += R.Names + '|' + Format(R.Calls) + ' '; until R.Next() = 0;
                if Seen <> 'Mu;|1 Mu;b;|2 Mu;b;Cu;|3 Mu;b;Cu;Ay;|4 Mu;b;Cu;Ay;Zz;|5 Mu;b;Cu;Ay;Zz;Sub;|6 ' then Error('%1', Seen); end;
            [Test] procedure PerDatabaseOrder() var R: Record DbLog;
            begin R.FindLast(); if R.Names <> 'Mu;b;Cu;Ay;Zz;' then Error('%1', R.Names); end; }";
    let (o, hits) = outcomes_and_hits_of_apps(&[("main", &[], main)], "T");
    for (n, r) in &o {
        assert_eq!(*r, Outcome::Pass, "{n}: {o:?}");
    }
    // The order is pinned (install/lt-ur-block-order/UrBlockOrderTests.al:59-81): no pass rests on an assumption.
    for (n, h) in &hits {
        assert!(h.is_empty(), "{n}: {hits:?}");
    }
}

// ADR-046 amendment 1, A4 items 1 and 2: the install runs in a session of its own, not the test session
// (install/lt-bf-install-probes/BfInstallTests.al:142-144), and a SingleInstance codeunit starts again in each company and in
// the per-database part (`counter=1` in rows #1, #4 and #7, BfInstallTests.al:125-140); the test session does not see the
// install run's SingleInstance state (events/lt-platform-events-v/PeVTests.al:297).
#[test]
fn the_install_session_is_its_own_and_single_instance_state_starts_again_per_part() {
    let o = outcomes(
        "table 60900 Db { DataPerCompany = false; fields { field(1; K; Code[10]) { } field(2; N; Integer) { } } }
            table 60901 Co { fields { field(1; K; Code[10]) { } field(2; N; Integer) { } field(3; Sid; Integer) { } } }
            codeunit 60906 State { SingleInstance = true; var N: Integer; procedure Next(): Integer begin N += 1; exit(N); end; }
            codeunit 60907 Inst { Subtype = Install;
                trigger OnInstallAppPerCompany() var R: Record Co; S: Codeunit State; begin R.K := 'CO'; R.N := S.Next(); R.Sid := SessionId(); R.Insert(); end;
                trigger OnInstallAppPerDatabase() var R: Record Db; S: Codeunit State; begin R.K := 'DB'; R.N := S.Next(); R.Insert(); end; }
            codeunit 60102 T { Subtype = Test;
                [Test] procedure PerCompanyCounter() var R: Record Co; begin R.Get('CO'); if R.N <> 1 then Error('counter %1', R.N); end;
                [Test] procedure PerDatabaseCounter() var R: Record Db; begin R.Get('DB'); if R.N <> 1 then Error('counter %1', R.N); end;
                [Test] procedure TestSessionCounter() var S: Codeunit State; begin if S.Next() <> 1 then Error('counter'); end;
                [Test] procedure NotTheTestSession() var R: Record Co; begin R.Get('CO'); if (R.Sid = SessionId()) or (R.Sid <= 0) then Error('session %1', R.Sid); end; }",
    );
    for (n, r) in &o {
        assert_eq!(*r, Outcome::Pass, "{n}: {o:?}");
    }
}

// ADR-046 amendment 1, A3: one app's whole install (per company, then per database) ends before the next app's begins
// (install/lt-bf-install-probes/BfInstallTests.al:239, the fixture app's three rows come first), in dependency order.
#[test]
fn two_apps_under_test_install_one_after_the_other() {
    let dep = "table 60900 DbA { DataPerCompany = false; fields { field(1; K; Code[10]) { } } }
        codeunit 60901 InstA { Subtype = Install;
            trigger OnInstallAppPerDatabase() var R: Record DbA; begin R.K := 'A'; R.Insert(); end; }";
    let main = "table 60910 CoB { fields { field(1; K; Code[10]) { } } }
        codeunit 60911 InstB { Subtype = Install;
            trigger OnInstallAppPerCompany() var A: Record DbA; R: Record CoB; begin if A.Get('A') then R.K := 'SEEN' else R.K := 'NOT'; R.Insert(); end; }
        codeunit 60912 T { Subtype = Test;
            [Test] procedure SawTheFirstApp() var R: Record CoB; begin R.Get('SEEN'); end; }";
    let o = outcomes_of_apps(&[("dep", &[], dep), ("main", &["dep"], main)], "T");
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
}

// ADR-046 amendment 1, A4 item 4: the install session raises Company Triggers OnCompanyOpen and OnCompanyOpenCompleted in
// each company before the install routines run there, and not in the per-database part
// (install/lt-bf-install-probes/BfInstallTests.al:125-140, `company open calls=2` and `=0`, subscribers at
// events/lt-bf-company-open/BfCompanyOpenFixtures.al:51-71).
#[test]
fn company_triggers_run_in_the_install_session_before_the_per_company_routines() {
    let o = outcomes(
        "table 60900 Db { DataPerCompany = false; fields { field(1; K; Code[10]) { } field(2; N; Integer) { } } }
            table 60901 Co { fields { field(1; K; Code[10]) { } field(2; N; Integer) { } } }
            codeunit 60906 Opens { SingleInstance = true; var N: Integer; procedure Add() begin N += 1; end; procedure Calls(): Integer begin exit(N); end; }
            codeunit 60908 OpenSubs {
                [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Company Triggers\", 'OnCompanyOpen', '', false, false)]
                local procedure Open() var S: Codeunit Opens; begin S.Add(); end;
                [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Company Triggers\", 'OnCompanyOpenCompleted', '', false, false)]
                local procedure Completed() var S: Codeunit Opens; begin S.Add(); end; }
            codeunit 60907 Inst { Subtype = Install;
                trigger OnInstallAppPerCompany() var R: Record Co; S: Codeunit Opens; begin R.K := 'CO'; R.N := S.Calls(); R.Insert(); end;
                trigger OnInstallAppPerDatabase() var R: Record Db; S: Codeunit Opens; begin R.K := 'DB'; R.N := S.Calls(); R.Insert(); end; }
            codeunit 60102 T { Subtype = Test;
                [Test] procedure PerCompany() var R: Record Co; begin R.Get('CO'); if R.N <> 2 then Error('calls %1', R.N); end;
                [Test] procedure PerDatabase() var R: Record Db; begin R.Get('DB'); if R.N <> 0 then Error('calls %1', R.N); end; }",
    );
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
    assert_eq!(o[1].1, Outcome::Pass, "{o:?}");
}

/// Per test, the ids of the assumptions it hit.
type TestHits = Vec<(String, Vec<&'static str>)>;

/// The outcomes and the assumption ids each test hit, of the test codeunit `codeunit` of a program of several apps (id,
/// dependencies, source); the app with id `dep` is a dependency app (ADR-013), the others are under test.
fn outcomes_and_hits_of_apps(apps: &[(&str, &[&str], &str)], codeunit: &str) -> (Vec<(String, Outcome)>, TestHits) {
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
                if *id == "dep" {
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
    let engine = Engine::new(l.program.clone(), blocked);
    let oi = l.program.objects.iter().position(|o| o.name == codeunit).unwrap();
    let mut hits = Vec::new();
    let done = run_codeunit_full(
        &engine,
        &l,
        oi,
        None,
        TestIsolation::Codeunit,
        None,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut hits,
        None,
    );
    let hits = hits
        .into_iter()
        .map(|(n, h)| (n, h.iter().map(|a| a.info().id).collect()))
        .collect();
    (done, hits)
}

// ADR-046 amendment 1, A4 item 4, owner decision 2026-10-19 (al-runner issue 6046): the install session of an app raises
// Company Triggers OnCompanyOpen to the subscribers of every installed app, not only its own: an assumption until probe Q3.
// A per-company install routine that reads what another app's subscriber wrote runs, and a test that reads what it wrote
// is a flagged pass; a test that touches none of those tables is a clean pass.
#[test]
fn company_triggers_of_other_installed_apps_run_in_the_install_session_as_an_assumption() {
    let dep = "table 60900 Feature { fields { field(1; K; Code[10]) { } } }
        codeunit 60901 DepOpen {
            [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Company Triggers\", 'OnCompanyOpen', '', false, false)]
            local procedure Open() var F: Record Feature; begin F.K := 'F'; if F.Insert() then; end; }";
    let main = "table 60910 Log { fields { field(1; K; Code[10]) { } field(2; Seen; Boolean) { } } }
        table 60913 Other { fields { field(1; K; Code[10]) { } } }
        codeunit 60911 Inst { Subtype = Install;
            trigger OnInstallAppPerCompany() var F: Record Feature; L: Record Log; begin L.K := 'L'; L.Seen := F.Get('F'); L.Insert(); end; }
        codeunit 60912 T { Subtype = Test;
            [Test] procedure ReadsWhatTheInstallSaw() var L: Record Log; begin L.Get('L'); if not L.Seen then Error('not seen'); end;
            [Test] procedure TouchesNoneOfIt() var O: Record Other; begin O.K := 'X'; O.Insert(); end; }";
    let (o, hits) = outcomes_and_hits_of_apps(&[("dep", &[], dep), ("main", &["dep"], main)], "T");
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
    assert_eq!(o[1].1, Outcome::Pass, "{o:?}");
    let of = |n: &str| hits.iter().find(|(t, _)| t == n).map(|(_, h)| h.clone());
    assert_eq!(
        of("ReadsWhatTheInstallSaw"),
        Some(vec!["install-company-triggers-all-apps"]),
        "{hits:?}"
    );
    assert_eq!(of("TouchesNoneOfIt"), None, "{hits:?}");
}

// ADR-046 amendment 1, A4 item 4: when a Company Triggers subscriber of any installed app cannot run in this runtime, the
// install session stops there as for the app's own subscribers: what the subscribers did is rolled back, the app's
// per-company install routines do not run, and a test that touches what they or the subscribers write is not run. The
// per-database part, which raises no Company Triggers, still runs.
#[test]
fn when_another_apps_company_triggers_subscriber_cannot_run_the_per_company_install_stops() {
    let dep = "table 60900 Feature { fields { field(1; K; Code[10]) { } } }
        codeunit 60901 DepOpen {
            [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Company Triggers\", 'OnCompanyOpen', '', false, false)]
            local procedure Open() var F: Record Feature; begin F.K := 'F'; F.Insert(); end; }
        codeunit 60902 DepOpenLater {
            [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Company Triggers\", 'OnCompanyOpen', '', false, false)]
            local procedure Open() begin Debugger.Deactivate(); end; }";
    let main = "table 60910 Log { fields { field(1; K; Code[10]) { } field(2; Seen; Boolean) { } } }
        table 60913 Other { DataPerCompany = false; fields { field(1; K; Code[10]) { } } }
        codeunit 60911 Inst { Subtype = Install;
            trigger OnInstallAppPerCompany() var L: Record Log; begin L.K := 'L'; L.Insert(); end; }
        codeunit 60914 InstOther { Subtype = Install;
            trigger OnInstallAppPerDatabase() var O: Record Other; begin O.K := 'O'; O.Insert(); end; }
        codeunit 60912 T { Subtype = Test;
            [Test] procedure ReadsWhatTheInstallWrote() var L: Record Log; begin L.FindFirst(); end;
            [Test] procedure ReadsWhatTheSubscriberWrote() var F: Record Feature; begin F.FindFirst(); end;
            [Test] procedure ReadsTheOtherRow() var O: Record Other; begin O.Get('O'); end; }";
    let (o, hits) = outcomes_and_hits_of_apps(&[("dep", &[], dep), ("main", &["dep"], main)], "T");
    assert!(
        matches!(&o[0].1, Outcome::NotRun(r) if r.contains("Company Triggers before it")),
        "{o:?}"
    );
    assert!(
        matches!(&o[1].1, Outcome::NotRun(r) if r.contains("Company Triggers")),
        "{o:?}"
    );
    assert_eq!(o[2].1, Outcome::Pass, "{o:?}");
    assert!(hits.is_empty(), "{hits:?}");
}

// Al-runner issue 5751: install code runs without a client. A plain Message does not stop it, GuiAllowed() is false,
// CurrentClientType() is SOAP, StrMenu, Confirm and Page.Run raise CallbackNotAllowed, Dialog, Hyperlink and
// Notification.Send do nothing (install/lt-outside-install-ui/RlInstallUi.al:210-241; Confirm, Page.RunModal with a write open and
// a Commit in a TryFunction: install/lt-cm-install-leads/CmInstallLeads.al:54-61).
#[test]
fn install_code_runs_without_a_client() {
    let o = outcomes(
        "table 60900 Seed { fields { field(1; K; Code[30]) { } field(2; Txt; Text[2048]) { } } }
            table 60901 SeedDb { DataPerCompany = false; fields { field(1; K; Code[30]) { } field(2; Txt; Text[2048]) { } } }
            page 60903 \"Ui Page\" { PageType = Card; SourceTable = Seed; layout { area(Content) { field(K; Rec.K) { } } } }
            codeunit 60902 Subs {
                [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Extension Triggers\", 'OnInstallAppPerCompany', '', false, false)]
                local procedure PerCompany() var Dlg: Dialog; begin
                    Add('GUI', Format(GuiAllowed())); Add('CLIENT', Format(CurrentClientType()));
                    ClearLastError(); Message('install message %1', 1); Add('AFTER', 'reached'); Add('LASTERR', GetLastErrorText());
                    if GuiAllowed() then Add('GUARD', 'taken') else Add('GUARD', 'skipped');
                    Add('STRMENU', Outcome(TryStrMenu())); Add('CONFIRM', Outcome(TryConfirm()));
                    Add('DIALOG', Outcome(TryDialog(Dlg))); Add('PAGE', Outcome(TryPageRun()));
                    Add('LINK', Outcome(TryLink())); Add('NOTE', Outcome(TryNote())); Add('MODAL', Outcome(TryModal()));
                    Add('COMMIT', Outcome(TryCommit())); Add('AFTERCOMMIT', 'reached'); end;
                [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Extension Triggers\", 'OnInstallAppPerDatabase', '', false, false)]
                local procedure PerDatabase() var R: Record SeedDb; begin
                    Message('per database'); R.K := 'GUI'; R.Txt := Format(GuiAllowed()); R.Insert(); end;
                local procedure Outcome(Ok: Boolean): Text var T: Text; begin
                    if Ok then T := 'ok' else T := 'error: ' + GetLastErrorText() + ' | code: ' + GetLastErrorCode(); ClearLastError(); exit(T); end;
                [TryFunction] local procedure TryStrMenu() begin if StrMenu('one,two', 2, 'pick one') = 0 then; end;
                [TryFunction] local procedure TryConfirm() begin if Confirm('sure?') then; end;
                [TryFunction] local procedure TryDialog(var Dlg: Dialog) begin Dlg.Open('dialog #1####'); Dlg.Update(1, 'x'); Dlg.Close(); end;
                [TryFunction] local procedure TryPageRun() begin Page.Run(Page::\"Ui Page\"); end;
                [TryFunction] local procedure TryCommit() begin Commit(); end;
                [TryFunction] local procedure TryModal() begin Page.RunModal(Page::\"Ui Page\"); end;
                [TryFunction] local procedure TryLink() begin Hyperlink('https://example.com'); end;
                [TryFunction] local procedure TryNote() var N: Notification; begin N.Message('note'); N.Send(); end;
                local procedure Add(K: Code[30]; T: Text) var R: Record Seed; begin R.K := K; R.Txt := T; if R.Insert() then; end; }
            codeunit 60102 T { Subtype = Test;
                local procedure S(K: Code[30]): Text var R: Record Seed; begin if not R.Get(K) then exit('<missing>'); exit(R.Txt); end;
                local procedure Is(K: Code[30]; Expected: Text) begin if S(K) <> Expected then Error('%1: %2', K, S(K)); end;
                [Test] procedure PerCompany() begin
                    Is('GUI', 'No'); Is('CLIENT', 'SOAP'); Is('AFTER', 'reached'); Is('LASTERR', ''); Is('GUARD', 'skipped');
                    Is('STRMENU', 'error: Microsoft Dynamics 365 Business Central Data Services attempted to issue a client callback to show a selection menu: pick one (CodeUnit 60902 Subs). Client callbacks are not supported on Microsoft Dynamics 365 Business Central Data Services. | code: CallbackNotAllowed');
                    Is('CONFIRM', 'error: Microsoft Dynamics 365 Business Central Data Services attempted to issue a client callback to show a confirmation dialog box: sure? (CodeUnit 60902 Subs). Client callbacks are not supported on Microsoft Dynamics 365 Business Central Data Services. | code: CallbackNotAllowed');
                    Is('DIALOG', 'ok');
                    Is('PAGE', 'error: Microsoft Dynamics 365 Business Central Data Services attempted to issue a client callback to run page 60903 Ui Page. Client callbacks are not supported on Microsoft Dynamics 365 Business Central Data Services. | code: CallbackNotAllowed');
                    Is('LINK', 'ok'); Is('NOTE', 'ok');
                    Is('MODAL', 'error: An error occurred and the transaction is stopped. Contact your administrator or partner for further assistance. | code: DB:Ok');
                    Is('COMMIT', 'ok'); Is('AFTERCOMMIT', 'reached'); end;
                [Test] procedure PerDatabase() var R: Record SeedDb; begin R.Get('GUI'); if R.Txt <> 'No' then Error('%1', R.Txt); end;
                [Test] procedure TestHasAClient() begin if not GuiAllowed() then Error('no client'); end; }",
    );
    for (n, r) in &o {
        assert_eq!(*r, Outcome::Pass, "{n}: {o:?}");
    }
}

// The reporter's shape of al-runner issue 5751: an install codeunit writes its setup row, then shows a Message, plain or
// behind `if GuiAllowed()` (pins as above, RlInstallUi.al:214-218).
#[test]
fn an_install_codeunit_that_shows_a_message_keeps_its_setup_row() {
    let o = outcomes(
        "table 60900 Setup { fields { field(1; K; Code[10]) { } field(2; Gui; Boolean) { } } }
            codeunit 60907 Inst { Subtype = Install;
                trigger OnInstallAppPerCompany() var R: Record Setup; begin
                    R.K := 'PLAIN'; R.Gui := GuiAllowed(); R.Insert(); Message('Setup done.');
                    if GuiAllowed() then Message('Guarded.');
                    R.K := 'AFTER'; R.Insert(); end; }
            codeunit 60102 T { Subtype = Test;
                [Test] procedure SetupRowStays() var R: Record Setup; begin R.Get('PLAIN'); R.Get('AFTER'); end;
                [Test] procedure GuiAllowedIsFalseInInstallCode() var R: Record Setup; begin R.Get('PLAIN'); if R.Gui then Error('GuiAllowed'); end; }",
    );
    for (n, r) in &o {
        assert_eq!(*r, Outcome::Pass, "{n}: {o:?}");
    }
}

// Al-runner issue 5743: when the test session opens the company, Base Application's LogInManagement sets WorkDate to
// GetDefaultWorkDate() (the last G/L entry's date in a demo company), so OnRun and the first test see that date
// (session/lt-outside-workdate/RlWorkDateAtStart.al:47-52); install code still sees today (install/lt-outside-install-ui/
// RlInstallUi.al:212). Here codeunit 40 stands in for Base Application's.
#[test]
fn the_test_session_starts_on_the_login_work_date() {
    let o = outcomes(
        "table 60900 Seen { fields { field(1; K; Code[10]) { } field(2; D; Date) { } field(3; IsToday; Boolean) { } } }
            codeunit 40 LogInManagement {
                procedure GetDefaultWorkDate(): Date begin exit(20240115D); end; }
            codeunit 60907 Inst { Subtype = Install;
                trigger OnInstallAppPerCompany() var R: Record Seen; begin R.K := 'INSTALL'; R.IsToday := WorkDate() = Today(); R.Insert(); end; }
            codeunit 60102 T { Subtype = Test;
                var OnRunDate: Date;
                trigger OnRun() begin OnRunDate := WorkDate(); end;
                [Test] procedure OnRunAndTest() begin
                    if OnRunDate <> 20240115D then Error('OnRun %1', OnRunDate);
                    if WorkDate() <> 20240115D then Error('test %1', WorkDate()); end;
                [Test] procedure InstallSawToday() var R: Record Seen; begin R.Get('INSTALL'); if not R.IsToday then Error('install'); end; }",
    );
    for (n, r) in &o {
        assert_eq!(*r, Outcome::Pass, "{n}: {o:?}");
    }
}

// The login code runs once per engine while the tables it reads stay as they were (`install::LoginMemo`): a later test
// session gets the same WorkDate and the same LogInManagement global (read later by Base Application's LogInEnd,
// LogInManagement.Codeunit.al:148) as a session that ran the code. Codeunit 40 stands in for Base Application's.
const LOGIN_SRC: &str = "table 60900 Entry { fields { field(1; No; Integer) { } field(2; D; Date) { } } }
    codeunit 60910 Seed { trigger OnRun() var E: Record Entry; begin E.No := 1; E.D := 20240115D; E.Insert(); end; }
    codeunit 40 LogInManagement { SingleInstance = true;
        var LogInWorkDate: Date;
        procedure GetDefaultWorkDate(): Date var E: Record Entry; begin
            if E.FindLast() then begin LogInWorkDate := NormalDate(E.D); exit(NormalDate(E.D)); end;
            exit(WorkDate()); end;
        procedure LoginDate(): Date begin exit(LogInWorkDate); end; }
    codeunit 60102 T1 { Subtype = Test;
        [Test] procedure SeesTheLoginDate() var L: Codeunit LogInManagement; begin
            if WorkDate() <> 20240115D then Error('WorkDate %1', WorkDate());
            if L.LoginDate() <> 20240115D then Error('LogInWorkDate %1', L.LoginDate()); end; }
    codeunit 60103 T2 { Subtype = Test;
        [Test] procedure SeesTheLoginDate() var L: Codeunit LogInManagement; begin
            if WorkDate() <> 20240115D then Error('WorkDate %1', WorkDate());
            if L.LoginDate() <> 20240115D then Error('LogInWorkDate %1', L.LoginDate()); end; }";

fn login_engine(src: &str) -> (Arc<Engine>, Lowered) {
    let (ws, l) = al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), src.into())],
        }],
        &Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    (Engine::new(l.program.clone(), blocked), l)
}

fn object_named(l: &Lowered, name: &str) -> usize {
    l.program.objects.iter().position(|o| o.name == name).unwrap()
}

fn login_replays(engine: &Engine) -> u64 {
    engine
        .derived(crate::install::LoginMemo::default)
        .replays
        .load(std::sync::atomic::Ordering::Relaxed)
}

/// The baseline of a session that ran codeunit `seed`'s OnRun.
fn seeded_baseline(engine: &Arc<Engine>, l: &Lowered, seed: &str) -> al_vm::Baseline {
    let mut session = al_vm::Session::new(engine.clone());
    let s = object_named(l, seed);
    let on_run = l.program.objects[s]
        .routines
        .iter()
        .position(|r| r.name == "OnRun")
        .unwrap();
    let inst = session.instance(s);
    assert!(session.call(s, on_run, inst, Vec::new()).is_ok());
    session.capture_baseline()
}

#[test]
fn the_login_code_runs_once_while_the_tables_it_reads_stay() {
    let (engine, l) = login_engine(LOGIN_SRC);
    let baseline = seeded_baseline(&engine, &l, "Seed");
    for (i, t) in ["T1", "T2", "T1"].into_iter().enumerate() {
        let o = run_codeunit_from(&engine, &l, object_named(&l, t), Some(&baseline));
        assert_eq!(o, vec![("SeesTheLoginDate".to_string(), Outcome::Pass)], "{t}");
        assert_eq!(login_replays(&engine), i as u64, "{t}");
    }
}

// A session whose tables changed since its baseline runs the login code again: here install code adds a later entry in
// every session, and each session sees the date of that entry.
#[test]
fn the_login_code_runs_again_when_a_table_changed_since_the_baseline() {
    let src = LOGIN_SRC.replace("20240115D", "20240301D").replace(
        "codeunit 60910 Seed { trigger OnRun() var E: Record Entry; begin E.No := 1; E.D := 20240301D; E.Insert(); end; }",
        "codeunit 60910 Seed { trigger OnRun() var E: Record Entry; begin E.No := 1; E.D := 20240115D; E.Insert(); end; }
        codeunit 60911 Inst { Subtype = Install;
            trigger OnInstallAppPerCompany() var E: Record Entry; begin E.No := 2; E.D := 20240301D; E.Insert(); end; }",
    );
    assert!(src.contains("Inst"));
    let (engine, l) = login_engine(&src);
    let baseline = seeded_baseline(&engine, &l, "Seed");
    for t in ["T1", "T2"] {
        let o = run_codeunit_from(&engine, &l, object_named(&l, t), Some(&baseline));
        assert_eq!(o, vec![("SeesTheLoginDate".to_string(), Outcome::Pass)], "{t}");
    }
    assert_eq!(login_replays(&engine), 0);
}

// Login code that writes is run in every session: what it wrote is in each of them.
#[test]
fn login_code_that_writes_is_not_replayed() {
    let src = LOGIN_SRC
        .replace(
            "if E.FindLast() then begin",
            "E.No := 7; E.D := 20240210D; if E.Insert() then; if E.FindLast() then begin",
        )
        .replace("20240115D then Error", "20240210D then Error");
    let (engine, l) = login_engine(&src);
    let baseline = seeded_baseline(&engine, &l, "Seed");
    for t in ["T1", "T2", "T1"] {
        let o = run_codeunit_from(&engine, &l, object_named(&l, t), Some(&baseline));
        assert_eq!(o, vec![("SeesTheLoginDate".to_string(), Outcome::Pass)], "{t}");
    }
    assert_eq!(login_replays(&engine), 0);
}

// Login code that may change a record global is not replayed: the record's state is not a plain value to put back.
#[test]
fn login_code_that_may_change_a_record_global_is_not_replayed() {
    let src = LOGIN_SRC
        .replace(
            "var LogInWorkDate: Date;",
            "var LogInWorkDate: Date; Last: Record Entry;",
        )
        .replace(
            "if E.FindLast() then begin",
            "if Last.FindLast() then; if E.FindLast() then begin",
        );
    let (engine, l) = login_engine(&src);
    let baseline = seeded_baseline(&engine, &l, "Seed");
    for t in ["T1", "T2"] {
        let o = run_codeunit_from(&engine, &l, object_named(&l, t), Some(&baseline));
        assert_eq!(o, vec![("SeesTheLoginDate".to_string(), Outcome::Pass)], "{t}");
    }
    assert_eq!(login_replays(&engine), 0);
}

// ADR-046 amendment 1, A5: in the per-database part, IsEmpty and FindFirst of a per-company table fail with the text BC
// gives (install/lt-bf-install-probes/BfInstallTests.al:138-139; reads in TryFunctions at BfInstallFixtures.al:131-146).
// The code BC gave (DB:Ok) is not pinned yet, so a read of it is refused.
#[test]
fn a_per_company_read_in_the_per_database_part_fails_with_the_pinned_text() {
    let o = outcomes(
        "table 60900 Db { DataPerCompany = false; fields { field(1; K; Code[10]) { } field(2; Info; Text[250]) { } } }
            table 60901 Pc { fields { field(1; K; Code[10]) { } } }
            codeunit 60907 Inst { Subtype = Install;
                trigger OnInstallAppPerDatabase() var R: Record Db; begin R.K := 'DB';
                    if TryIsEmpty() then R.Info := 'ok' else R.Info := CopyStr(GetLastErrorText(), 1, 250);
                    R.Insert();
                    R.K := 'FF';
                    if TryFindFirst() then R.Info := 'ok' else R.Info := CopyStr(GetLastErrorText(), 1, 250);
                    R.Insert(); end;
                [TryFunction] local procedure TryIsEmpty() var P: Record Pc; begin if P.IsEmpty() then; end;
                [TryFunction] local procedure TryFindFirst() var P: Record Pc; begin if P.FindFirst() then; end; }
            codeunit 60102 T { Subtype = Test;
                [Test] procedure Text() var R: Record Db; begin
                    R.Get('DB'); if R.Info <> 'You must choose a company before you can access the \"Pc\" table.' then Error('%1', R.Info);
                    R.Get('FF'); if R.Info <> 'You must choose a company before you can access the \"Pc\" table.' then Error('%1', R.Info); end; }",
    );
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
}

// install/TestInstallSeeder.al: OnInstallAppPerDatabase seeds one table, OnInstallAppPerCompany
// is one the runtime cannot run. The per-database seed ran and stays; only the trigger that
// could not run is skipped, with its tables.
#[test]
fn an_installer_trigger_that_ran_keeps_its_rows_when_the_next_one_cannot_run() {
    let o = outcomes(
            "table 60900 Db { DataPerCompany = false; fields { field(1; K; Code[10]) { } } }
            table 60901 Co { fields { field(1; K; Code[10]) { } } }
            codeunit 60901 Inst { Subtype = Install;
                trigger OnInstallAppPerDatabase() var R: Record Db; begin R.K := 'DB'; R.Insert(); end;
                trigger OnInstallAppPerCompany() var R: Record Co; begin Debugger.Deactivate(); R.K := 'CO'; R.Insert(); end; }
            codeunit 60102 T { Subtype = Test;
                [Test] procedure SeededByTheTriggerThatRan() var R: Record Db; begin if not R.Get('DB') then Error('the per-database seed was rolled back'); end;
                [Test] procedure SeededByTheTriggerThatDidNot() var R: Record Co; begin if R.Count() = 0 then Error('unseeded'); end; }",
        );
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
    assert!(matches!(&o[1].1, Outcome::NotRun(r) if r.contains("install")), "{o:?}");
}

#[test]
fn a_blocked_installer_also_blocks_tests_on_tables_its_event_subscribers_touch() {
    // install/TestInstallEventDispatch_*.al: the installer raises an event whose
    // subscriber seeds a table the installer never names; skipping the installer
    // must not let tests on that table run against an unseeded store.
    let o = outcomes(
        "table 60900 Seed { fields { field(1; K; Code[10]) { } } }
            codeunit 60904 Pub { [IntegrationEvent(false, false)] procedure OnSeed() begin end; }
            codeunit 60905 Sub {
                [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnSeed', '', false, false)]
                local procedure Seed() var R: Record Seed; begin R.Insert(); end; }
            codeunit 60901 Inst { Subtype = Install;
                trigger OnInstallAppPerCompany() var P: Codeunit Pub; begin Debugger.Deactivate(); P.OnSeed(); end; }
            codeunit 60102 T { Subtype = Test;
                [Test] procedure UsesSeed() var R: Record Seed; begin end; }",
    );
    assert!(matches!(&o[0].1, Outcome::NotRun(r) if r.contains("install")), "{o:?}");
}

#[test]
fn a_blocked_installer_also_blocks_tests_on_tables_written_by_table_event_subscribers() {
    // The installer writes to Src, whose Insert event subscriber seeds Seed: skipping the
    // installer must not let tests on Seed run against an unseeded store.
    let o = outcomes(
        "table 60900 Seed { fields { field(1; K; Code[10]) { } } }
            table 60903 Src { fields { field(1; K; Code[10]) { } } }
            codeunit 60905 Sub {
                [EventSubscriber(ObjectType::Table, Database::Src, 'OnAfterInsertEvent', '', false, false)]
                local procedure Seed(var Rec: Record Src) var R: Record Seed; begin R.K := 'X'; R.Insert(); end; }
            codeunit 60901 Inst { Subtype = Install;
                trigger OnInstallAppPerCompany() var S: Record Src; begin Debugger.Deactivate(); S.K := 'A'; S.Insert(); end; }
            codeunit 60102 T { Subtype = Test;
                [Test] procedure UsesSeed() var R: Record Seed; begin end; }",
    );
    assert!(matches!(&o[0].1, Outcome::NotRun(r) if r.contains("install")), "{o:?}");
}

/// What `subscribers_reached` finds: the subscribers of the table events of every table a reached routine has a record
/// of, and of every event a reached routine publishes, followed through what those subscribers call in turn.
#[test]
fn the_subscribers_a_routine_reaches_follow_table_events_published_events_and_calls() {
    let (ws, l) = al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![(
                "f.al".into(),
                "table 60900 Src { fields { field(1; K; Code[10]) { } } }
                table 60901 Seed { fields { field(1; K; Code[10]) { } } }
                table 60902 Other { fields { field(1; K; Code[10]) { } } }
                codeunit 60904 Pub { [IntegrationEvent(false, false)] procedure OnSeed() begin end; }
                codeunit 60905 Sub {
                    [EventSubscriber(ObjectType::Table, Database::Src, 'OnAfterInsertEvent', '', false, false)]
                    local procedure OnSrc(var Rec: Record Src) begin Raise(); end;
                    local procedure Raise() var P: Codeunit Pub; begin P.OnSeed(); end;
                    [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnSeed', '', false, false)]
                    local procedure Seed() var R: Record Seed; begin R.Insert(); end;
                    [EventSubscriber(ObjectType::Table, Database::Seed, 'OnAfterDeleteEvent', '', false, false)]
                    local procedure OnSeedDeleted(var Rec: Record Seed) begin end;
                    [EventSubscriber(ObjectType::Table, Database::Other, 'OnAfterInsertEvent', '', false, false)]
                    local procedure OnOther(var Rec: Record Other) begin end; }
                codeunit 60906 Root { procedure Go() var S: Record Src; begin S.K := 'A'; S.Insert(); end; }"
                    .into(),
            )],
        }],
        &Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let engine = Engine::new(l.program.clone(), Vec::new());
    let objects = &l.program.objects;
    let at = |object: &str, routine: &str| {
        let o = objects.iter().position(|o| o.name == object).unwrap();
        (o, objects[o].routines.iter().position(|r| r.name == routine).unwrap())
    };
    let (o, r) = at("Root", "Go");
    let reached = blocked::subscribers_reached(&engine, &l, o, r);
    let mut want = vec![at("Sub", "OnSrc"), at("Sub", "Seed"), at("Sub", "OnSeedDeleted")];
    want.sort();
    assert_eq!(reached, want);
}

#[test]
fn a_blocked_installer_only_blocks_tests_that_touch_its_tables() {
    let o = outcomes(
        "table 60900 S { fields { field(1; K; Code[10]) { } } }
            table 60903 Other { fields { field(1; K; Code[10]) { } } }
            codeunit 60901 Inst { Subtype = Install;
                trigger OnInstallAppPerCompany() var R: Record S; begin Debugger.Deactivate(); R.Insert(); end; }
            codeunit 60102 T { Subtype = Test;
                [Test] procedure UsesS() var R: Record S; begin end;
                [Test] procedure UsesOther() var R: Record Other; begin end; }",
    );
    assert!(matches!(&o[0].1, Outcome::NotRun(r) if r.contains("install")), "{o:?}");
    assert_eq!(o[1].1, Outcome::Pass, "{o:?}");
}

// ---- test handler functions (docs/probes-needed.md lists what is unpinned) ----

fn outcome_of(o: &[(String, Outcome)], name: &str) -> Outcome {
    o.iter()
        .find(|(n, _)| n == name)
        .unwrap_or_else(|| panic!("{name}: {o:?}"))
        .1
        .clone()
}

const UI: &str = "codeunit 60110 UiT { Subtype = Test;
        var Seen: Text; Fired: Integer; Asked: Text;

        // handlers/TestMessageHandler.al:15 Message_Show_HandlerCaptures
        [Test] [HandlerFunctions('Msg')]
        procedure MessageReachesHandler() begin Message('Hello'); if Seen <> 'Hello' then Error('got %1', Seen); end;

        // handlers/TestMessageHandler.al:23 Message_FormattedString_HandlerGetsFormatted
        [Test] [HandlerFunctions('Msg')]
        procedure MessageArgsAreSubstitutedBeforeTheHandler() begin Message('Value: %1', 42); if Seen <> 'Value: 42' then Error('got %1', Seen); end;

        // handlers/TestMessageHandler.al:31,41 Confirm_Question_HandlerRepliesTrue / False
        [Test] [HandlerFunctions('Yes')]
        procedure ConfirmHandlerRepliesTrue() begin if not Confirm('Sure?') then Error('false'); end;
        [Test] [HandlerFunctions('No')]
        procedure ConfirmHandlerRepliesFalse() begin if Confirm('Sure?', true) then Error('true'); end;

        // handlers/TestConfirmFormatArgs.al:Confirm_SingleFormatArg_HandlerReceivesRawUnsubstitutedQuestion
        [Test] [HandlerFunctions('No')]
        procedure ConfirmHandlerGetsTheRawQuestion() begin if Confirm('%1 items. Go?', true, 2) then; if Asked <> '%1 items. Go?' then Error('got %1', Asked); end;

        // handlers/TestConfirmFormatArgs.al:Confirm_DefaultChoiceHonored_WhenHandlerDoesNotOverrideReply
        [Test] [HandlerFunctions('Silent')]
        procedure ConfirmDefaultSurvivesAHandlerThatLeavesReply()
        begin
            if not Confirm('q', true) then Error('default true lost');
            if Confirm('q', false) then Error('default false lost');
        end;

        // handlers/TestConfirmFormatArgs.al: one handler answers two Confirms
        [Test] [HandlerFunctions('No')]
        procedure OneHandlerServesManyCalls() begin Fired := 0; if Confirm('a', true) then; if Confirm('b', true) then; if Fired <> 2 then Error('fired %1', Fired); end;

        // handlers/TestConfirmNoHandler.al:Confirm_NoHandler_TestBody_IsRefused
        [Test]
        procedure ConfirmWithoutHandlerIsRefused()
        var Reply: Boolean;
        begin
            Reply := false;
            asserterror Reply := Confirm('CNH question', true);
            if StrPos(GetLastErrorText(), 'Unhandled UI') = 0 then Error('no Unhandled UI: %1', GetLastErrorText());
            if StrPos(GetLastErrorText(), 'Confirm') = 0 then Error('no Confirm: %1', GetLastErrorText());
            if StrPos(GetLastErrorText(), 'CNH question') = 0 then Error('no question: %1', GetLastErrorText());
            if Reply then Error('default was assigned');
        end;

        // handlers/TestPageQueryCloseError_Tests.al:75 (Unhandled UI ... Message)
        [Test]
        procedure MessageWithoutHandlerIsRefused()
        begin
            asserterror Message('lost text');
            if StrPos(GetLastErrorText(), 'Unhandled UI: Message') = 0 then Error('got %1', GetLastErrorText());
            if StrPos(GetLastErrorText(), 'lost text') = 0 then Error('text missing: %1', GetLastErrorText());
        end;

        // a Message handler does not answer a Confirm
        [Test] [HandlerFunctions('Msg')]
        procedure HandlerOfAnotherKindDoesNotAnswer()
        begin
            asserterror if Confirm('q') then;
            if StrPos(GetLastErrorText(), 'Unhandled UI: Confirm') = 0 then Error('got %1', GetLastErrorText());
            Message('now used');
        end;

        // handlers/TestPageQueryCloseMessageHandler_Tests.al:51 (UI handlers were not executed)
        [Test] [HandlerFunctions('Msg')]
        procedure DeclaredHandlerThatNeverFiresFailsTheTest() begin end;

        // an error in the test wins over the unexecuted handler
        [Test] [HandlerFunctions('Msg')]
        procedure TestErrorIsReportedBeforeUnexecutedHandlers() begin Error('own error'); end;

        // an error raised in a handler comes out of the UI call
        [Test] [HandlerFunctions('Boom')]
        procedure HandlerErrorPropagates() begin Message('x'); end;

        // handlers of the previous test are not bound any more
        [Test]
        procedure HandlersDoNotLeakIntoTheNextTest()
        begin
            asserterror Message('leak?');
            if StrPos(GetLastErrorText(), 'Unhandled UI') = 0 then Error('got %1', GetLastErrorText());
        end;

        // handlers/TestUnhandledUiAndDialogDefaults.al:164 [StrMenuHandler]
        [Test] [HandlerFunctions('Menu')]
        procedure StrMenuHandlerChoosesAnOption()
        var C: Integer;
        begin
            C := StrMenu('One,Two,Three', 1, 'Pick');
            if C <> 2 then Error('choice %1', C);
            if Asked <> 'One,Two,Three|Pick' then Error('asked %1', Asked);
        end;

        [Test]
        procedure StrMenuWithoutHandlerIsRefused()
        begin
            asserterror if StrMenu('A,B') = 0 then;
            if StrPos(GetLastErrorText(), 'Unhandled UI: StrMenu') = 0 then Error('got %1', GetLastErrorText());
        end;

        [Test] [HandlerFunctions('Nope')]
        procedure UnknownHandlerNameIsRefused() begin end;

        [Test] [HandlerFunctions('Helper')]
        procedure NamingARoutineThatIsNoHandlerIsRefused() begin end;

        [Test] [HandlerFunctions('Blocked')]
        procedure HandlerWithUnsupportedCodeIsRefused() begin Message('x'); end;

        [Test] [HandlerFunctions('Msg,Msg2')]
        procedure TwoHandlersOfOneKindAreRefused() begin end;

        [MessageHandler] procedure Msg(M: Text[1024]) begin Seen := M; end;
        [MessageHandler] procedure Msg2(M: Text[1024]) begin end;
        [MessageHandler] procedure Boom(M: Text[1024]) begin Error('boom in handler'); end;
        [MessageHandler] procedure Blocked(M: Text[1024]) begin Debugger.Deactivate(); end;
        [ConfirmHandler] procedure Yes(Q: Text[1024]; var Reply: Boolean) begin Reply := true; end;
        [ConfirmHandler] procedure No(Q: Text[1024]; var Reply: Boolean) begin Fired += 1; Asked := Q; Reply := false; end;
        [ConfirmHandler] procedure Silent(Q: Text[1024]; var Reply: Boolean) begin end;
        [StrMenuHandler] procedure Menu(Options: Text; var Choice: Integer; Instruction: Text) begin Asked := Options + '|' + Instruction; Choice := 2; end;
        procedure Helper() begin end;
    }";

fn ui(name: &str) -> Outcome {
    outcome_of(&outcomes(UI), name)
}

#[test]
fn message_handler_receives_the_text_and_substituted_arguments() {
    assert_eq!(ui("MessageReachesHandler"), Outcome::Pass);
    assert_eq!(ui("MessageArgsAreSubstitutedBeforeTheHandler"), Outcome::Pass);
}

#[test]
fn confirm_handler_replies_and_receives_the_raw_question() {
    assert_eq!(ui("ConfirmHandlerRepliesTrue"), Outcome::Pass);
    assert_eq!(ui("ConfirmHandlerRepliesFalse"), Outcome::Pass);
    assert_eq!(ui("ConfirmHandlerGetsTheRawQuestion"), Outcome::Pass);
    assert_eq!(ui("ConfirmDefaultSurvivesAHandlerThatLeavesReply"), Outcome::Pass);
    assert_eq!(ui("OneHandlerServesManyCalls"), Outcome::Pass);
}

#[test]
fn ui_without_a_bound_handler_of_its_kind_is_refused_with_unhandled_ui() {
    assert_eq!(ui("ConfirmWithoutHandlerIsRefused"), Outcome::Pass);
    assert_eq!(ui("MessageWithoutHandlerIsRefused"), Outcome::Pass);
    assert_eq!(ui("HandlerOfAnotherKindDoesNotAnswer"), Outcome::Pass);
    assert_eq!(ui("StrMenuWithoutHandlerIsRefused"), Outcome::Pass);
    assert_eq!(ui("HandlersDoNotLeakIntoTheNextTest"), Outcome::Pass);
}

#[test]
fn a_declared_handler_that_never_fires_fails_the_test() {
    assert!(matches!(ui("DeclaredHandlerThatNeverFiresFailsTheTest"),
                Outcome::Fail(m) if m.starts_with("The following UI handlers were not executed") && m.contains("Msg")),);
    assert_eq!(
        ui("TestErrorIsReportedBeforeUnexecutedHandlers"),
        Outcome::Fail("own error".into())
    );
}

#[test]
fn an_error_in_a_handler_comes_out_of_the_ui_call() {
    assert_eq!(ui("HandlerErrorPropagates"), Outcome::Fail("boom in handler".into()));
}

#[test]
fn str_menu_handler_chooses() {
    assert_eq!(ui("StrMenuHandlerChoosesAnOption"), Outcome::Pass);
}

#[test]
fn handlers_the_runtime_cannot_run_are_refused_explicitly() {
    let nr = |n: &str| match ui(n) {
        Outcome::NotRun(r) => r,
        other => panic!("{n}: {other:?}"),
    };
    assert!(nr("UnknownHandlerNameIsRefused").contains("Nope"));
    assert!(nr("NamingARoutineThatIsNoHandlerIsRefused").contains("Helper"));
    assert!(nr("HandlerWithUnsupportedCodeIsRefused").contains("Debugger"));
    assert!(nr("TwoHandlersOfOneKindAreRefused").contains("Msg2"));
}

#[test]
fn page_report_and_other_handlers_are_refused_with_their_kind() {
    let o = outcomes(
        "page 60120 Pg { PageType = Card; }
            report 60123 Rp { }
            codeunit 60121 T { Subtype = Test;
                [Test] [HandlerFunctions('Modal')] procedure A() begin end;
                [Test] [HandlerFunctions('Req')] procedure B() begin end;
                [Test] [HandlerFunctions('Fil')] procedure C() begin end;
                [ModalPageHandler] procedure Modal(var P: TestPage Pg) begin end;
                [RequestPageHandler] procedure Req(var P: TestRequestPage Rp) begin end;
                [FilterPageHandler] procedure Fil(var F: FilterPageBuilder) begin end; }",
    );
    // Modal and request page handlers are supported; one the test never triggers is reported
    // as BC does.
    assert!(
        matches!(&o[0].1, Outcome::Fail(r) if r.contains("not executed")),
        "{o:?}"
    );
    assert!(
        matches!(&o[1].1, Outcome::Fail(r) if r.contains("not executed")),
        "{o:?}"
    );
    assert!(
        // A [FilterPageHandler] takes `var RecordRef` (filterpage/W3FilterPageBuilder.al:160); another shape is refused.
        matches!(&o[2].1, Outcome::NotRun(r) if r.contains("not shaped like a FilterPage handler")),
        "{o:?}"
    );
}

#[test]
fn ui_outside_a_test_method_is_not_run() {
    // OnRun is not a test method: what a UI call does there is not pinned.
    let o = outcomes(
        "codeunit 60122 T { Subtype = Test;
                trigger OnRun() begin Message('x'); end;
                [Test] procedure A() begin end; }",
    );
    assert!(
        matches!(&o[0].1, Outcome::NotRun(r) if r.contains("OnRun") && r.contains("outside a test method")),
        "{o:?}"
    );
}

// ---- page handlers (ADR-012) ----

const PAGES: &str = r#"
table 60700 "PH Row" { fields { field(1; "No."; Code[20]) { } field(2; Descr; Text[50]) { } } keys { key(PK; "No.") { } } }
page 60701 "PH Card" { PageType = Card; SourceTable = "PH Row"; layout { area(Content) { field(NoCtl; Rec."No.") { } field(DescrCtl; Rec.Descr) { } } } }
page 60702 "PH Host"
{
    PageType = Card; SourceTable = "PH Row";
    layout { area(Content) { field(NoCtl; Rec."No.") { } } }
    actions { area(Processing) { action(Pick) { trigger OnAction() var M: Page "PH Card"; R: Record "PH Row"; Res: Action; begin
        R.Init(); R."No." := 'RESULT'; Res := M.RunModal(); R.Descr := Format(Res); R.Insert(); end; } } }
}
table 60706 "PH Nr" { fields { field(1; Id; Guid) { } field(2; Name; Code[50]) { } } keys { key(PK; Id) { } } }
page 60707 "PH NrCard"
{
    PageType = Card; SourceTable = "PH Nr"; DelayedInsert = true;
    layout { area(Content) {
        field(IdField; Rec.Id) { Visible = false; }
        field(NameField; Rec.Name) { trigger OnValidate() begin CurrPage.Update(); end; } } }
    trigger OnInsertRecord(BelowxRec: Boolean): Boolean begin Rec.Id := CreateGuid(); end;
    trigger OnAfterGetRecord() var T: Codeunit PhTrace; begin T.Note('AGR:' + Rec.Name); end;
    trigger OnAfterGetCurrRecord() var T: Codeunit PhTrace; begin T.Note('AGCR'); CurrPage.Update(false); end;
}
codeunit 60708 PhTrace { SingleInstance = true; var Log: Text; procedure Note(T: Text) begin Log += T + ';'; end; procedure Get(): Text begin exit(Log); end; procedure Reset() begin Log := ''; end; }
page 60709 "PH Ro" { PageType = Card; SourceTable = "PH Row"; layout { area(Content) { field(NoCtl; Rec."No.") { } field(NoteCtl; Note) { } } } trigger OnOpenPage() begin CurrPage.Editable(false); end; var Note: Text[20]; }
page 60710 "PH Desc" { PageType = List; SourceTable = "PH Row"; SourceTableView = sorting("No.") order(descending); layout { area(Content) { repeater(R) { field(NoCtl; Rec."No.") { } field(DescrCtl; Rec.Descr) { } } } } }
table 60704 "PH Lk" { LookupPageId = "PH LkCard"; fields { field(1; "No."; Code[20]) { } field(2; Descr; Text[50]) { } } keys { key(PK; "No.") { } } }
// The lookup page of "PH Lk" has to show its table: a record of another table is refused (pages/lt-page-run-overloads-testpart/P106RunErrors.al).
page 60713 "PH LkCard" { PageType = Card; SourceTable = "PH Lk"; layout { area(Content) { field(NoCtl; Rec."No.") { } } } }
page 60705 "PH Saved" { PageType = StandardDialog; SaveValues = true; layout { area(Content) { field(Rem; RemTxt) { } } } var RemTxt: Text[30]; }
page 60711 "PH Saved X" { PageType = StandardDialog; SaveValues = true; SourceTable = "PH Row"; layout { area(Content) { field(RemX; RemTxt) { } } } var RemTxt: Text[30]; }
pageextension 60712 "PH Saved XE" extends "PH Saved X" { layout { addlast(Content) { field(DescX; Rec.Descr) { } } } }
codeunit 60703 PhT
{
    Subtype = Test;
    var Seen: Text;

    // handlers/TestPageModalHandler_Tests.al:43,83 ModalPageHandlerOkReachesTheCallingAl / CancelReaches (lookup)
    [Test] [HandlerFunctions('OkModal')]
    procedure OkReachesTheCaller()
    var R: Record "PH Row"; H: TestPage "PH Host";
    begin
        R."No." := 'A'; R.Insert(); H.OpenEdit(); H.First(); H.Pick.Invoke(); H.Close();
        R.Get('RESULT'); if R.Descr <> 'OK' then Error('got %1', R.Descr);
    end;

    // handlers/TestPageModalHandler_Tests.al:105 ModalPageWithoutAHandlerIsRefused
    [Test]
    procedure NoHandlerIsRefused()
    var R: Record "PH Row"; H: TestPage "PH Host";
    begin
        R."No." := 'N1'; R.Insert(); Commit(); H.OpenEdit(); H.First();
        asserterror H.Pick.Invoke();
        if StrPos(GetLastErrorText(), 'Unhandled UI: ModalPage') = 0 then Error('got %1', GetLastErrorText());
    end;

    // handlers/TestPageRunHandler_Tests.al:60-90: the handler is handed the record Page.Run was given
    [Test] [HandlerFunctions('SeePage')]
    procedure PageRunHandsTheRecord()
    var R: Record "PH Row";
    begin
        R."No." := 'P1'; R.Descr := 'Alpha'; R.Insert(); R."No." := 'P2'; R.Descr := 'Beta'; R.Insert();
        R.Get('P2');
        Page.Run(Page::"PH Card", R);
        if Seen <> 'Beta' then Error('got %1', Seen);
    end;

    // testpageclose + handlers/TestPageModalQueryClose_Tests.al (b): a handler that invokes nothing leaves a plain modal reporting OK
    [Test] [HandlerFunctions('Nothing')]
    procedure UnattendedPlainModalReportsOk()
    var M: Page "PH Card"; Res: Action;
    begin
        Res := M.RunModal();
        if Format(Res) <> 'OK' then Error('got %1', Format(Res));
    end;

    // TestPageModalQueryClose_Tests.al (d)/(e): lookup mode cancels
    [Test] [HandlerFunctions('CancelIt')]
    procedure LookupCancelReportsLookupCancel()
    var M: Page "PH Card"; Res: Action;
    begin
        M.LookupMode(true);
        Res := M.RunModal();
        if Format(Res) <> 'LookupCancel' then Error('got %1', Format(Res));
    end;

    // TestPageModalQueryClose_Tests.al (b2): a plain modal has no built-in Cancel
    [Test] [HandlerFunctions('CancelIt')]
    procedure PlainModalHasNoCancel()
    var M: Page "PH Card"; Res: Action;
    begin
        asserterror Res := M.RunModal();
        if StrPos(GetLastErrorText(), 'The built-in action = Cancel is not found on the page') = 0 then Error('got %1', GetLastErrorText());
    end;

    // handlers/TestPageRunModalLookupPageId_Tests.al:47-55: id 0 with a record resolves LookupPageId (the pin has a
    // row; OK of that lookup on no row is not pinned and is refused)
    [Test] [HandlerFunctions('OkModalLk')]
    procedure IdZeroUsesLookupPageId()
    var K: Record "PH Lk"; R: Record "PH Row"; Res: Action;
    begin
        K."No." := 'L1'; K.Insert();
        Res := Page.RunModal(0, K);
        if Format(Res) <> 'LookupOK' then Error('got %1', Format(Res));
        asserterror Page.RunModal(0, R);
        if StrPos(GetLastErrorText(), 'You tried to invoke the Page object with the ID 0') = 0 then Error('got %1', GetLastErrorText());
    end;

    // handlers/TestPageSaveValues.al: OK stores, the next open shows it; Cancel stores nothing
    [Test] [HandlerFunctions('SaveRem')]
    procedure SaveValuesKeepsWhatOkStored()
    begin
        Seen := 'x'; Page.RunModal(Page::"PH Saved");
        Seen := ''; Page.RunModal(Page::"PH Saved");
        if Seen <> 'kept' then Error('got %1', Seen);
    end;

    // handlers/TestPageSaveValues.al as above, on a SaveValues page that an extension extends with a
    // control bound to a field of Rec (it adds no variable-bound control: docs/probes-needed.md)
    [Test] [HandlerFunctions('SaveRemX')]
    procedure SaveValuesOnAnExtendedPageKeepsWhatOkStored()
    begin
        Seen := 'x'; Page.RunModal(Page::"PH Saved X");
        Seen := ''; Page.RunModal(Page::"PH Saved X");
        if Seen <> 'kept' then Error('got %1', Seen);
    end;

    // handlers/TestPageTrapSharedScope_Tests.al:87: a page still held by the caller keeps the trap
    [Test]
    procedure TrapSurvivesWhileAnotherVariableHoldsThePage()
    var T: TestPage "PH Card"; R: Record "PH Row";
    begin
        R."No." := 'TS'; R.Insert();
        TrapAndAssign(T);
        R.Get('TS');
        Page.Run(Page::"PH Card", R);
        if T.NoCtl.Value() <> 'TS' then Error('got %1', T.NoCtl.Value());
        T.Close();
    end;

    local procedure TrapAndAssign(var Out: TestPage "PH Card")
    var A: TestPage "PH Card"; B: TestPage "PH Card";
    begin
        A.Trap(); B := A; Out := B;
    end;

    // pageupdate/TestPageUpdateNewRow.al: Update() in OnValidate saves a DelayedInsert new row, then the refresh runs AGR for it
    [Test]
    procedure UpdateSavesTheNewRowAndRefreshesIt()
    var C: TestPage "PH NrCard"; R: Record "PH Nr"; T: Codeunit PhTrace;
    begin
        T.Reset(); C.OpenNew();
        if T.Get() <> 'AGCR;' then Error('open: %1', T.Get());
        T.Reset(); C.NameField.SetValue('Nx');
        if StrPos(T.Get(), 'AGR:NX;') = 0 then Error('set: %1', T.Get());
        C.Close();
        R.SetRange(Name, 'Nx'); if R.Count() <> 1 then Error('rows %1', R.Count());
    end;

    // handlers/TestPageControlEditableRoutes.al:548: CurrPage.Editable(false) in OnOpenPage narrows every control
    [Test]
    procedure CurrPageEditableFalseInOnOpenPageNarrowsEverything()
    var C: TestPage "PH Ro"; R: Record "PH Row";
    begin
        R."No." := 'E1'; R.Insert();
        C.OpenEdit();
        if C.Editable() or C.NoCtl.Editable() or C.NoteCtl.Editable() then Error('still editable');
        C.Close();
    end;

    // pagefindrecord/TestPageFindRecord.al:192: SourceTableView sorting descending; Find*Field walks the view
    [Test]
    procedure SourceTableViewOrdersTheRowsAndFindFieldWalksThem()
    var L: TestPage "PH Desc"; R: Record "PH Row";
    begin
        R."No." := 'ZD1'; R.Insert(); R."No." := 'ZD2'; R.Insert(); R."No." := 'ZD3'; R.Insert();
        L.OpenView(); L.First();
        if L.NoCtl.Value() <> 'ZD3' then Error('first %1', L.NoCtl.Value());
        if not L.FindNextField(L.NoCtl, 'ZD1') then Error('next');
        if L.NoCtl.Value() <> 'ZD1' then Error('at %1', L.NoCtl.Value());
        if L.FindFirstField(L.NoCtl, 5) then Error('an Integer must not match a Code control');
        if L.NoCtl.Value() <> 'ZD1' then Error('a miss keeps the row, at %1', L.NoCtl.Value());
        L.Close();
    end;

    [ModalPageHandler] procedure SaveRem(var P: TestPage "PH Saved")
    begin
        if Seen = 'x' then begin P.Rem.SetValue('kept'); P.OK().Invoke(); end
        else begin Seen := P.Rem.Value(); P.Cancel().Invoke(); end;
    end;
    [ModalPageHandler] procedure SaveRemX(var P: TestPage "PH Saved X")
    begin
        if Seen = 'x' then begin P.RemX.SetValue('kept'); P.OK().Invoke(); end
        else begin Seen := P.RemX.Value(); P.Cancel().Invoke(); end;
    end;
    [ModalPageHandler] procedure OkModal(var P: TestPage "PH Card") begin P.OK().Invoke(); end;
    [ModalPageHandler] procedure OkModalLk(var P: TestPage "PH LkCard") begin P.OK().Invoke(); end;
    [ModalPageHandler] procedure Nothing(var P: TestPage "PH Card") begin end;
    [ModalPageHandler] procedure CancelIt(var P: TestPage "PH Card") begin P.Cancel().Invoke(); end;
    [PageHandler] procedure SeePage(var P: TestPage "PH Card") begin Seen := P.DescrCtl.Value(); end;
}
"#;

fn ph(name: &str) -> Outcome {
    outcome_of(&outcomes(PAGES), name)
}

#[test]
fn modal_and_page_handlers_answer_page_opens() {
    for n in [
        "OkReachesTheCaller",
        "NoHandlerIsRefused",
        "PageRunHandsTheRecord",
        "UnattendedPlainModalReportsOk",
        "LookupCancelReportsLookupCancel",
        "PlainModalHasNoCancel",
        "IdZeroUsesLookupPageId",
        "SaveValuesKeepsWhatOkStored",
        "SaveValuesOnAnExtendedPageKeepsWhatOkStored",
        "TrapSurvivesWhileAnotherVariableHoldsThePage",
        "UpdateSavesTheNewRowAndRefreshesIt",
        "CurrPageEditableFalseInOnOpenPageNarrowsEverything",
        "SourceTableViewOrdersTheRowsAndFindFieldWalksThem",
    ] {
        assert_eq!(ph(n), Outcome::Pass, "{n}");
    }
}

// session/TestSessionApplicationAreaTestBoundary.al:67552 (A_FirstTest_StartsWithEveryAreaEnabled_EvenAfterOnRunSetOne,
// C_NextTest_StartsWithEveryAreaEnabled): every test method starts with every application area enabled.
#[test]
fn every_test_starts_with_every_application_area_enabled() {
    let src = "codeunit 60110 T
{
    Subtype = Test;
    trigger OnRun() begin ApplicationArea('#Basic,#FromOnRun'); end;

    [Test]
    procedure A_StartsEmptyEvenAfterOnRunSetOne()
    begin
        if ApplicationArea() <> '' then Error('areas: %1', ApplicationArea());
    end;

    [Test]
    procedure B_SetsAreasForTheRestOfTheTest()
    begin
        ApplicationArea('#Basic,#Probe');
        if ApplicationArea() <> '#Basic,#Probe' then Error('areas: %1', ApplicationArea());
    end;

    [Test]
    procedure C_StartsEmptyAgain()
    begin
        if ApplicationArea() <> '' then Error('areas: %1', ApplicationArea());
    end;
}";
    let o = outcomes_of(src, "T");
    for (n, r) in &o {
        assert_eq!(*r, Outcome::Pass, "{n}");
    }
    assert_eq!(o.len(), 3);
}

#[test]
fn a_page_source_table_view_applies_in_filter_group_2() {
    // corpus handlers/TestPageSourceTableView.al:129-151, reproduced without a dependency app.
    let o = outcomes(
        r#"
enum 50000 "S" { value(0; Open) { } value(1; Active) { } value(2; Closed) { } }
table 50000 "K" { fields { field(1; "Entry No."; Integer) { } field(2; Status; Enum "S") { } } keys { key(PK; "Entry No.") { Clustered = true; } } }
table 50001 "Log" { fields { field(1; "Id"; Integer) { AutoIncrement = true; } field(2; "NewValue"; Text[100]) { } field(3; "OldValue"; Text[100]) { } } keys { key(PK; "Id") { Clustered = true; } } }
page 50000 "P" { PageType = List; SourceTable = "K"; SourceTableView = where(Status = const(Active));
  layout { area(Content) { repeater(Lines) { field("Entry No."; Rec."Entry No.") { } field(Status; Rec.Status) { } } } }
  trigger OnOpenPage() var L: Record "Log"; begin
    L."OldValue" := CopyStr(Rec.GetFilter(Status), 1, 100);
    Rec.FilterGroup(2);
    L."NewValue" := CopyStr(Rec.GetFilter(Status), 1, 100);
    Rec.FilterGroup(0);
    L.Insert();
  end; }
codeunit 50001 "T" { Subtype = Test;
  [Test] procedure Go() var L: Record "Log"; P: TestPage "P"; begin
    P.OpenView(); P.Close();
    L.FindLast();
    if L."NewValue" <> 'Active' then Error('new=[%1] old=[%2]', L."NewValue", L."OldValue");
  end; }
"#,
    );
    assert_eq!(o, vec![("Go".to_string(), Outcome::Pass)]);
}

/// Outcomes of the test codeunit of a workspace app `main` over a dependency app `dep`.
fn outcomes_over_dependency(main: &str, dep: &str) -> Vec<(String, Outcome)> {
    let app = |id: &str, deps: &[&str], src: &str| AppSource {
        id: id.into(),
        name: id.into(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![(format!("{id}.al"), src.into())],
    };
    let (_, l) = al_sema::lower::compile_apps(
        vec![app("main", &["dep"], main), app("dep", &[], dep)],
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        &Default::default(),
    );
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let engine = Engine::new(l.program.clone(), blocked);
    let oi = l
        .program
        .objects
        .iter()
        .position(|o| {
            o.properties
                .iter()
                .any(|(k, v)| k.eq_ignore_ascii_case("Subtype") && v.eq_ignore_ascii_case("Test"))
        })
        .unwrap();
    run_codeunit(&engine, &l, oi)
}

const HIDDEN_FIELD_PAGE: &str = r#"
table 50000 "K" { fields { field(1; "No."; Integer) { } field(2; Name; Text[20]) { } } keys { key(PK; "No.") { Clustered = true; } } }
page 50000 "P" { PageType = List; SourceTable = "K";
  layout { area(Content) { repeater(Lines) { field("No."; Rec."No.") { } field(Name; Rec.Name) { Visible = false; } } } } }
"#;
const SETS_HIDDEN_FIELD: &str = r#"codeunit 50001 "T" { Subtype = Test;
  [Test] procedure Go() var P: TestPage "P"; begin P.OpenView(); P.Name.SetValue('X'); end; }"#;

#[test]
fn a_missing_control_of_a_local_page_is_reported_by_its_generated_id() {
    let o = outcomes(&format!("{HIDDEN_FIELD_PAGE}{SETS_HIDDEN_FIELD}"));
    assert_eq!(
        o,
        vec![(
            "Go".to_string(),
            // The id is FNV-1a of "50000Name" (pages/request.rs `action_id`), checked on its own.
            Outcome::Fail("The field with ID = 1299704023 is not found on the page.".to_string())
        )]
    );
}

#[test]
fn a_missing_control_of_a_dependency_page_is_named_by_the_same_generated_id() {
    // corpus pageapplicationarea/TestPageApplicationAreaPrecompiledControlRemoval.al:92 pins
    // `ID = 301776846` for `Name` of page 329: the id derives from the declaring object's id and
    // the control name alone, so a dependency page needs no special case.
    let o = outcomes_over_dependency(SETS_HIDDEN_FIELD, HIDDEN_FIELD_PAGE);
    assert_eq!(
        o,
        vec![(
            "Go".to_string(),
            Outcome::Fail("The field with ID = 1299704023 is not found on the page.".to_string())
        )]
    );
}

#[test]
fn running_a_dependency_codeunit_nothing_compiled_by_its_id_is_refused_not_run_as_empty() {
    // Nothing names codeunit 50100 but the number: it is not compiled, and running it must not
    // quietly do nothing.
    let o = outcomes_over_dependency(
        "codeunit 50001 \"T\" { Subtype = Test; [Test] procedure Go() begin Codeunit.Run(50100); end; }",
        "codeunit 50100 \"Lib\" { trigger OnRun() begin Error('ran'); end; }",
    );
    let [(name, Outcome::NotRun(why))] = o.as_slice() else {
        panic!("{o:?}")
    };
    assert_eq!(name, "Go");
    assert!(why.contains("50100"), "{why}");
}

#[test]
fn opening_a_dependency_table_nothing_compiled_by_its_id_is_refused() {
    let o = outcomes_over_dependency(
        "codeunit 50001 \"T\" { Subtype = Test; [Test] procedure Go() var R: RecordRef; begin R.Open(50100); end; }",
        "table 50100 \"Lib\" { fields { field(1; K; Code[10]) { } } }",
    );
    let [(_, outcome)] = o.as_slice() else { panic!("{o:?}") };
    assert!(matches!(outcome, Outcome::NotRun(_)), "{o:?}");
}

#[test]
fn running_a_dependency_page_nothing_compiled_by_its_id_is_refused() {
    let o = outcomes_over_dependency(
        "codeunit 50001 \"T\" { Subtype = Test; [Test] procedure Go() begin Page.Run(50100); end; }",
        "table 50100 \"K\" { fields { field(1; \"No.\"; Integer) { } } }\n\
             page 50100 \"Lib\" { PageType = List; SourceTable = \"K\"; layout { area(Content) { repeater(Lines) { field(\"No.\"; Rec.\"No.\") { } } } } }",
    );
    let [(_, outcome)] = o.as_slice() else { panic!("{o:?}") };
    assert!(
        matches!(outcome, Outcome::NotRun(why) if why.contains("50100")),
        "{o:?}"
    );
}

/// The full run (install baseline included) of workspace app `main` over dependency app `dep`:
/// the outcome of test `name` of the test codeunit `50001`.
fn with_baseline(main: &str, dep: &str) -> impl Fn(&str) -> Outcome {
    let app = |id: &str, deps: &[&str], src: &str| AppSource {
        id: id.into(),
        name: id.into(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![(format!("{id}.al"), src.into())],
    };
    let rt = CorpusRuntime::run_for_apps(
        vec![app("main", &["dep"], main), app("dep", &[], dep)],
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        &[],
        &Default::default(),
        None,
        Default::default(),
    );
    move |name: &str| {
        rt.first_outcome(&CaseId {
            codeunit: 50001,
            name: name.into(),
        })
    }
}

const SETUP_DEP: &str = r#"
table 50100 "Setup" { fields { field(1; "Primary Key"; Code[10]) { } field(2; Value; Text[50]) { } } keys { key(PK; "Primary Key") { } } }
table 50101 "Order Log" { fields { field(1; "No."; Integer) { AutoIncrement = true; } field(2; What; Text[30]) { } } keys { key(PK; "No.") { } } }
table 50102 "Shared Log" { DataPerCompany = false; fields { field(1; "No."; Integer) { AutoIncrement = true; } field(2; What; Text[30]) { } } keys { key(PK; "No.") { } } }
codeunit 50110 "Installer A" { Subtype = Install;
  trigger OnInstallAppPerDatabase() var L: Record "Shared Log"; begin L.What := 'A-db'; L.Insert(); end;
  trigger OnInstallAppPerCompany() var L: Record "Order Log"; S: Record "Setup"; begin L.What := 'A-company'; L.Insert(); S."Primary Key" := ''; S.Value := 'installed'; S.Insert(); end; }
codeunit 50111 "Installer B" { Subtype = Install;
  trigger OnInstallAppPerCompany() var L: Record "Order Log"; begin L.What := 'B-company'; L.Insert(); end; }
codeunit 2 "Company-Initialize" {
  trigger OnRun() var S: Record "Setup"; L: Record "Order Log"; begin
    S.Get(); S.Value := S.Value + '+initialized'; S.Modify();
    L.What := 'init'; L.Insert(); end; }
codeunit 50120 "Lib" { procedure ReadSetup(): Text var S: Record "Setup"; begin S.Get(); exit(S.Value); end; }
"#;

#[test]
fn a_shared_log_the_projects_own_install_codeunits_write_in_turn_holds_their_rows_in_order() {
    // Every OnInstallAppPerCompany, then every OnInstallAppPerDatabase (install/lt-bf-install-probes/BfInstallTests.al:239);
    // the two per-company triggers run in ascending id, as their ids lie in one block of 65536 (ADR-046 amendment 3), and
    // the default mode models a database with one company (amendment 4), so the shared log holds one write of each.
    let o = outcomes(
        r#"
table 50101 "Shared Log" { DataPerCompany = false; fields { field(1; "No."; Integer) { AutoIncrement = true; } field(2; What; Text[30]) { } } keys { key(PK; "No.") { } } }
codeunit 50110 "Installer A" { Subtype = Install;
  trigger OnInstallAppPerDatabase() var L: Record "Shared Log"; begin L.What := 'A-db'; L.Insert(); end;
  trigger OnInstallAppPerCompany() var L: Record "Shared Log"; begin L.What := 'A-company'; L.Insert(); end; }
codeunit 50111 "Installer B" { Subtype = Install;
  trigger OnInstallAppPerCompany() var L: Record "Shared Log"; begin L.What := 'B-company'; L.Insert(); end; }
codeunit 50001 "T" { Subtype = Test;
  [Test] procedure Order() var L: Record "Shared Log"; T: Text; begin
    L.FindSet(); repeat T += L.What + ';'; until L.Next() = 0;
    if T <> 'A-company;B-company;A-db;' then Error(T); end; }
"#,
    );
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
}

/// A dependency app whose `Global Triggers` subscribers cannot run (they reach an unimplemented
/// builtin) once the table `Cfg` they read holds a row.
const GT_DEP: &str = r#"
table 50100 "Cfg" { fields { field(1; K; Code[10]) { } } keys { key(PK; K) { } } }
codeunit 50120 "GT Mgt" {
  [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'GetDatabaseTableTriggerSetup', '', false, false)]
  local procedure Setup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean) begin Check(); end;
  [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'OnDatabaseInsert', '', false, false)]
  local procedure Ins(RecRef: RecordRef) begin Check(); end;
  local procedure Check() var C: Record "Cfg"; begin if C.IsEmpty() then exit; Debugger.Deactivate(); end; }
"#;

const GT_WATCHED: &str = r#"
table 50001 "Watched" { fields { field(1; K; Integer) { } } keys { key(PK; K) { } } }
table 50003 "Watched Later" { fields { field(1; K; Integer) { } } keys { key(PK; K) { } } }
"#;

#[test]
fn global_triggers_subscribers_that_cannot_run_are_skipped_only_while_nothing_they_read_is_configured() {
    let main = format!(
        "{GT_WATCHED}codeunit 50001 \"T\" {{ Subtype = Test; \
                 [Test] procedure Plain() var W: Record \"Watched\"; begin W.K := 1; W.Insert(); end; \
                 [Test] procedure Configured() var W: Record \"Watched Later\"; C: Record \"Cfg\"; begin \
                   C.K := 'x'; C.Insert(); W.K := 2; W.Insert(); end; }}"
    );
    let outcome = with_baseline(&main, GT_DEP);
    assert_eq!(outcome("Plain"), Outcome::Pass);
    let Outcome::NotRun(why) = outcome("Configured") else {
        panic!("{:?}", outcome("Configured"))
    };
    assert!(
        why.contains("GT Mgt.Setup") && why.contains("cannot run") && why.contains("Cfg"),
        "{why}"
    );
}

#[test]
fn a_database_event_that_an_unavailable_subscriber_would_receive_is_refused() {
    // The workspace's own subscriber enables OnDatabaseInsert for Watched; BC then runs the
    // dependency's subscriber too, which cannot run here.
    let main = format!(
        "{GT_WATCHED}codeunit 50002 \"Mine\" {{ \
                 [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Global Triggers\", 'GetDatabaseTableTriggerSetup', '', false, false)] \
                 local procedure S(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean) begin \
                   if TableId = Database::Watched then OnDatabaseInsert := true; end; }} \
                 codeunit 50001 \"T\" {{ Subtype = Test; \
                 [Test] procedure Enabled() var W: Record \"Watched\"; begin W.K := 1; W.Insert(); end; }}"
    );
    let outcome = with_baseline(&main, GT_DEP);
    let Outcome::NotRun(why) = outcome("Enabled") else {
        panic!("{:?}", outcome("Enabled"))
    };
    assert!(why.contains("ondatabaseinsert") && why.contains("GT Mgt.Ins"), "{why}");
}

#[test]
fn install_triggers_and_company_initialize_seed_the_rows_every_test_codeunit_starts_from() {
    // ADR-013 decision 5: dependency installs, then Company-Initialize, then every test starts
    // from that state (the Base Application setup rows are read by its own code).
    let outcome = with_baseline(
        "codeunit 50001 \"T\" { Subtype = Test; var L: Codeunit \"Lib\"; \
             [Test] procedure Seeded() begin if L.ReadSetup() <> 'installed+initialized' then Error(L.ReadSetup()); end; }",
        SETUP_DEP,
    );
    assert_eq!(outcome("Seeded"), Outcome::Pass);
}

#[test]
fn per_company_triggers_run_first_in_codeunit_order_then_per_database_with_no_company() {
    // Every OnInstallAppPerCompany, then OnInstallAppPerDatabase, which has no company (a DataPerCompany = false table
    // works there): install/lt-bf-install-probes/BfInstallTests.al:138-140, :239. The order of the dependency app's two
    // per-company triggers is not pinned (ADR-046 amendment 1, A2 a); for dependency apps the refusal is not applied
    // (kept, not pinned: it would lose a baseline case, docs/owner-review.md), so program order is what the test sees.
    let outcome = with_baseline(
        "codeunit 50001 \"T\" { Subtype = Test; \
             [Test] procedure Order() var L: Record \"Order Log\"; Sh: Record \"Shared Log\"; T: Text; begin \
               L.FindSet(); repeat T += L.What + ';'; until L.Next() = 0; \
               Sh.FindFirst(); T += Sh.What; \
               if T <> 'A-company;B-company;init;A-db' then Error(T); end; }",
        SETUP_DEP,
    );
    assert_eq!(outcome("Order"), Outcome::Pass);
}

#[test]
fn a_test_codeunit_starts_from_the_baseline_whatever_an_earlier_one_committed() {
    let main = "codeunit 50001 \"T\" { Subtype = Test; \
             [Test] procedure Changes() var S: Record \"Setup\"; begin S.Get(); S.Value := 'changed'; S.Modify(); end; } \
             codeunit 50002 \"T2\" { Subtype = Test; var L: Codeunit \"Lib\"; \
             [Test] procedure Fresh() begin if L.ReadSetup() <> 'installed+initialized' then Error(L.ReadSetup()); end; }";
    let rt = {
        let app = |id: &str, deps: &[&str], src: &str| AppSource {
            id: id.into(),
            name: id.into(),
            dependencies: deps.iter().map(|d| d.to_string()).collect(),
            files: vec![(format!("{id}.al"), src.into())],
        };
        CorpusRuntime::run_for_apps(
            vec![app("main", &["dep"], main), app("dep", &[], SETUP_DEP)],
            vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
            &[],
            &Default::default(),
            None,
            Default::default(),
        )
    };
    let o = |cu: u32, name: &str| {
        rt.first_outcome(&CaseId {
            codeunit: cu,
            name: name.into(),
        })
    };
    assert_eq!(o(50001, "Changes"), Outcome::Pass);
    assert_eq!(o(50002, "Fresh"), Outcome::Pass);
}

#[test]
fn a_missing_row_is_refused_only_where_the_baseline_did_not_create_it() {
    // The installer cannot run (Debugger.Deactivate is not implemented), so the rows it
    // would create are missing: a Get that finds nothing, from dependency code or from the
    // tests' own, is then not BC's answer. Where the baseline did create rows, the real
    // error is reported.
    let dep = r#"
table 50100 "K" { fields { field(1; "No."; Code[10]) { } } }
table 50103 "Seeded" { fields { field(1; "No."; Code[10]) { } } }
codeunit 50110 "Installer" { Subtype = Install;
  trigger OnInstallAppPerCompany() var S: Record "Seeded"; begin S."No." := 'X'; S.Insert(); Later(); end;
  local procedure Later() var K: Record "K"; begin Debugger.Deactivate(); K.Insert(); end; }
codeunit 50101 "Lib" { procedure ReadK() var S: Record "K"; begin S.Get(); end; procedure ReadSeeded() var S: Record "Seeded"; begin S.Get('NOPE'); end; }
"#;
    let outcome = with_baseline(
        "codeunit 50001 \"T\" { Subtype = Test; var L: Codeunit \"Lib\"; \
             [Test] procedure Own() var S: Record \"K\"; begin asserterror S.Get(); end; \
             [Test] procedure Unseeded() begin L.ReadK(); end; \
             [Test] procedure Seeded() begin asserterror L.ReadSeeded(); end; }",
        dep,
    );
    for name in ["Own", "Unseeded"] {
        let Outcome::NotRun(why) = outcome(name) else {
            panic!("{name}: {:?}", outcome(name))
        };
        assert!(why.contains("row not found"), "{name}: {why}");
    }
    // The installer's rollback leaves `Seeded` empty too, so it is unseeded as well.
    assert!(matches!(outcome("Seeded"), Outcome::NotRun(_) | Outcome::Pass));
}

#[test]
fn reads_of_a_table_the_baseline_could_not_fill_are_refused_everywhere_but_a_get_that_finds_the_row() {
    // The installer cannot run, so BC's rows in "K" are missing: a read whose answer depends on
    // them (Find*, Next, Count, IsEmpty, a FlowField over it, a TableRelation check against
    // it, from the workspace or from the dependency) is not BC's answer. A Get that finds the
    // row is; a table the installer does not write, and a temporary record, are unaffected.
    let dep = r#"
table 50100 "K" { fields { field(1; "No."; Code[10]) { } } keys { key(PK; "No.") { } } }
table 50103 "Other" { fields { field(1; "No."; Code[10]) { } } keys { key(PK; "No.") { } } }
table 50104 "Holder" { fields { field(1; "No."; Code[10]) { } field(2; Cnt; Integer) { FieldClass = FlowField; CalcFormula = count(K); }
  field(3; Ref; Code[10]) { TableRelation = K; } } keys { key(PK; "No.") { } } }
codeunit 50110 "Installer" { Subtype = Install;
  trigger OnInstallAppPerCompany() var S: Record "K"; begin Debugger.Deactivate(); S.Insert(); end; }
codeunit 50101 "Lib" { procedure Count(): Integer var S: Record "K"; begin exit(S.Count()); end; }
"#;
    let main = r#"codeunit 50001 "T" { Subtype = Test; var L: Codeunit "Lib";
  [Test] procedure FindFirstIsRefused() var S: Record "K"; begin if S.FindFirst() then; end;
  [Test] procedure FindSetIsRefused() var S: Record "K"; begin if S.FindSet() then; end;
  [Test] procedure CountIsRefused() var S: Record "K"; begin if S.Count() = 0 then; end;
  [Test] procedure IsEmptyIsRefused() var S: Record "K"; begin if S.IsEmpty() then; end;
  [Test] procedure DependencyCountIsRefused() begin if L.Count() = 0 then; end;
  [Test] procedure FlowFieldIsRefused() var H: Record "Holder"; begin H."No." := 'A'; H.Insert(); H.CalcFields(Cnt); end;
  [Test] procedure TableRelationMissIsRefused() var H: Record "Holder"; begin H."No." := 'B'; H.Insert(); H.Validate(Ref, 'X'); end;
  [Test] procedure GetThatFindsTheRowIsAnswered() var S: Record "K"; begin S."No." := 'W'; S.Insert(); S.Get('W'); end;
  [Test] procedure AnotherTableIsRead() var O: Record "Other"; begin O."No." := 'W'; O.Insert(); if O.Count() <> 1 then Error('x'); end;
  [Test] procedure ATemporaryRecordIsRead() var S: Record "K" temporary; begin S."No." := 'W'; S.Insert(); if S.Count() <> 1 then Error('x'); end; }"#;
    let outcome = with_baseline(main, dep);
    for refused in [
        "FindFirstIsRefused",
        "FindSetIsRefused",
        "CountIsRefused",
        "IsEmptyIsRefused",
        "DependencyCountIsRefused",
        "FlowFieldIsRefused",
        "TableRelationMissIsRefused",
    ] {
        let Outcome::NotRun(why) = outcome(refused) else {
            panic!("{refused}: {:?}", outcome(refused))
        };
        assert!(why.contains("install baseline could not create"), "{refused}: {why}");
    }
    for answered in [
        "GetThatFindsTheRowIsAnswered",
        "AnotherTableIsRead",
        "ATemporaryRecordIsRead",
    ] {
        assert_eq!(outcome(answered), Outcome::Pass, "{answered}");
    }
}

#[test]
fn company_initialize_that_stops_keeps_the_rows_it_wrote_before() {
    let dep = r#"
table 50100 "Setup" { fields { field(1; "Primary Key"; Code[10]) { } } keys { key(PK; "Primary Key") { } } }
table 50104 "Never" { fields { field(1; "Primary Key"; Code[10]) { } } keys { key(PK; "Primary Key") { } } }
codeunit 2 "Company-Initialize" {
  trigger OnRun() var S: Record "Setup"; begin S.Insert(); Later(); end;
  local procedure Later() var N: Record "Never"; begin Debugger.Deactivate(); N.Insert(); end; }
codeunit 50101 "Lib" { procedure SetupCount(): Integer var S: Record "Setup"; begin exit(S.Count()); end;
  procedure ReadNever() var N: Record "Never"; begin N.Get(); end; }
"#;
    let outcome = with_baseline(
        "codeunit 50001 \"T\" { Subtype = Test; var L: Codeunit \"Lib\"; \
             [Test] procedure Kept() begin if L.SetupCount() <> 1 then Error('rows: %1', L.SetupCount()); end; \
             [Test] procedure NotCreated() begin L.ReadNever(); end; }",
        dep,
    );
    assert_eq!(outcome("Kept"), Outcome::Pass);
    assert!(matches!(outcome("NotCreated"), Outcome::NotRun(_)));
}

fn outcomes_isolated(src: &str, isolation: TestIsolation) -> Vec<(String, Outcome)> {
    let (ws, l) = al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), src.into())],
        }],
        &Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let engine = Engine::new(l.program.clone(), blocked);
    let oi = l
        .program
        .objects
        .iter()
        .position(|o| {
            o.properties
                .iter()
                .any(|(k, v)| k.eq_ignore_ascii_case("Subtype") && v.eq_ignore_ascii_case("Test"))
        })
        .unwrap();
    run_codeunit_isolated(&engine, &l, oi, None, isolation)
}

// The isolation probe's fixture (corpus tests/al-language-isolation-probe/src/
// IsolProbeFixture.Codeunit.al:25-114), shortened. Real BC, nightly run on 28.4
// (oracle/28/xunit-isolation-function.xml): all seven pass under TestIsolation = Function;
// oracle/28/xunit-isolation-codeunit.xml: all but T5 pass under Codeunit.
const PROBE: &str = "table 61300 Row { fields { field(1; K; Code[20]) { } } keys { key(PK; K) { Clustered = true; } } }
        codeunit 61303 Single { SingleInstance = true;
            procedure SetMark(M: Integer) begin Mark := M; end;
            procedure GetMark(): Integer begin exit(Mark); end;
            var Mark: Integer; }
        codeunit 61302 Fixture { Subtype = Test;
            var GlobalMark: Integer; CallCount: Integer; OnRunCount: Integer;
            trigger OnRun() var R: Record Row; begin
                OnRunCount += 1;
                if not R.Get('ONRUN') then begin R.K := 'ONRUN'; R.Insert(); end; end;
            [Test] procedure T1() var R: Record Row; S: Codeunit Single; begin
                CallCount += 1; GlobalMark := 4826; S.SetMark(4826);
                R.K := 'MARK'; R.Insert();
                Expect(1, CallCount, 'T1 first on its instance'); end;
            [Test] procedure T2() begin CallCount += 1; Expect(4826, GlobalMark, 'global set by T1'); end;
            [Test] procedure T3() begin CallCount += 1; Expect(3, CallCount, 'counter counts T1 to T3'); end;
            [Test] procedure T4() var S: Codeunit Single; begin CallCount += 1; Expect(4826, S.GetMark(), 'single instance'); end;
            [Test] procedure T5() var R: Record Row; begin
                CallCount += 1;
                if R.Get('MARK') then Error('The row T1 inserted is still there in T5'); end;
            [Test] procedure T6() var R: Record Row; begin
                CallCount += 1;
                if not R.Get('ONRUN') then Error('The row OnRun inserted is gone in T6'); end;
            [Test] procedure T7() begin CallCount += 1; Expect(1, OnRunCount, 'OnRun ran once'); end;
            local procedure Expect(E: Integer; A: Integer; What: Text) begin
                if A <> E then Error('%1: expected %2, observed %3', What, E, A); end; }";

// oracle/28/xunit-isolation-function.xml: T1 to T7 all Pass.
#[test]
fn function_isolation_rolls_the_database_back_per_test_and_keeps_the_instance() {
    let o = outcomes_isolated(PROBE, TestIsolation::Function);
    assert_eq!(
        o.iter().map(|(_, x)| x.clone()).collect::<Vec<_>>(),
        vec![Outcome::Pass; 7],
        "{o:?}"
    );
}

// oracle/28/xunit-isolation-codeunit.xml: T5 fails with the row still there, the rest pass.
#[test]
fn codeunit_isolation_keeps_the_database_between_tests() {
    let o = outcomes_isolated(PROBE, TestIsolation::Codeunit);
    let passes: Vec<_> = o.iter().map(|(n, x)| (n.as_str(), *x == Outcome::Pass)).collect();
    assert_eq!(
        passes,
        vec![
            ("T1", true),
            ("T2", true),
            ("T3", true),
            ("T4", true),
            ("T5", false),
            ("T6", true),
            ("T7", true)
        ]
    );
    assert_eq!(o[4].1, Outcome::Fail("The row T1 inserted is still there in T5".into()));
}

#[test]
fn function_isolation_also_rolls_back_a_failed_test() {
    let o = outcomes_isolated(
            &with_rows(
                "codeunit 60102 T { Subtype = Test;
                    [Test] procedure A() var R: Record S; begin R.K := 'A'; R.Insert(); Error('boom'); end;
                    [Test] procedure B() var R: Record S; begin if R.Count() <> 0 then Error('rows left: %1', R.Count()); end; }",
            ),
            TestIsolation::Function,
        );
    assert_eq!(o[0].1, Outcome::Fail("boom".into()));
    assert_eq!(o[1].1, Outcome::Pass, "{o:?}");
}

// Under a runner's TestIsolation = Disabled the probe's T5 fails, as under Codeunit (corpus PR 735: the rows of runner
// 61304 for "Isol Probe Fixture").
#[test]
fn disabled_isolation_keeps_the_database_between_tests() {
    let o = outcomes_isolated(PROBE, TestIsolation::Disabled);
    let passes: Vec<_> = o.iter().map(|(n, x)| (n.as_str(), *x == Outcome::Pass)).collect();
    assert_eq!(
        passes,
        vec![
            ("T1", true),
            ("T2", true),
            ("T3", true),
            ("T4", true),
            ("T5", false),
            ("T6", true),
            ("T7", true)
        ]
    );
}

// A Commit() inside a test under a runner's TestIsolation = Function is rolled back with the test (corpus PR 735,
// IsolProbeCommit.Codeunit.al C1-C3 under runner 61300).
#[test]
fn a_commit_inside_a_function_isolated_test_is_rolled_back() {
    let src = with_rows(
            "codeunit 60102 T { Subtype = Test;
                [Test] procedure Commits() var R: Record S; begin R.K := 'A'; R.Insert(); Commit(); end;
                [Test] procedure Next() var R: Record S; begin if R.Count() <> 0 then Error('rows left: %1', R.Count()); end; }",
        );
    let o = outcomes_isolated(&src, TestIsolation::Function);
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
    assert_eq!(o[1].1, Outcome::Pass, "{o:?}");
    let o = outcomes_isolated(&src, TestIsolation::Codeunit);
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
}

#[test]
fn retire_drops_the_value_on_another_thread() {
    struct Marker(std::sync::mpsc::Sender<std::thread::ThreadId>);
    impl Drop for Marker {
        fn drop(&mut self) {
            let _ = self.0.send(std::thread::current().id());
        }
    }
    let (tx, rx) = std::sync::mpsc::channel();
    super::retire(Marker(tx));
    let dropped_on = rx
        .recv_timeout(std::time::Duration::from_secs(10))
        .expect("the value is dropped");
    assert_ne!(dropped_on, std::thread::current().id());
}

#[test]
fn retire_at_the_end_leaks_the_value_when_the_process_ends_after_the_run() {
    // The one-shot command lines end the process right after the run: nothing is freed then.
    let held = std::sync::Arc::new(());
    super::retire_at_end(held.clone(), true);
    assert_eq!(std::sync::Arc::strong_count(&held), 2, "never dropped");
    // Any other caller (a server, a test that runs again) still frees it, on another thread.
    super::retire_at_end(held.clone(), false);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while std::sync::Arc::strong_count(&held) != 2 {
        assert!(std::time::Instant::now() < deadline, "the retired value is dropped");
        std::thread::yield_now();
    }
}

#[test]
fn retiring_derefs_to_the_value_and_drops_it_on_another_thread_when_it_goes() {
    struct Marker {
        value: u32,
        tx: std::sync::mpsc::Sender<std::thread::ThreadId>,
    }
    impl Drop for Marker {
        fn drop(&mut self) {
            let _ = self.tx.send(std::thread::current().id());
        }
    }
    let (tx, rx) = std::sync::mpsc::channel();
    let held = super::Retiring::new(Marker { value: 7, tx });
    assert_eq!(held.value, 7, "the value is usable through the wrapper");
    assert!(rx.try_recv().is_err(), "not dropped while held");
    drop(held);
    let dropped_on = rx
        .recv_timeout(std::time::Duration::from_secs(10))
        .expect("the value is dropped");
    assert_ne!(dropped_on, std::thread::current().id());
}

/// A test that never ends is stopped at its time limit and reported as not run, with the limit
/// in the reason. The tests after it still run; a failure after it stays a failure when the
/// stopped test can change nothing the failing test reads (it only counts in a local).
#[test]
fn a_test_over_its_time_limit_is_stopped_and_not_run() {
    let src = "codeunit 61400 Endless { Subtype = Test;
            [Test] procedure Before() begin end;
            [Test] procedure ForEver() var I: Integer; begin while true do I += 1; end;
            [Test] procedure ForEverRepeat() var I: Integer; begin repeat I += 1; until false; end;
            [Test] procedure After() begin end;
            [Test] procedure AfterFails() begin Error('x'); end; }";
    let (ws, l) = al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), src.into())],
        }],
        &Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let engine = Engine::new(l.program.clone(), blocked);
    let oi = l.program.objects.iter().position(|o| o.name == "Endless").unwrap();
    let started = std::time::Instant::now();
    let out = run_codeunit_limited(
        &engine,
        &l,
        oi,
        None,
        TestIsolation::Codeunit,
        Some(std::time::Duration::from_millis(300)),
    );
    assert!(started.elapsed() < std::time::Duration::from_secs(10));
    let get = |n: &str| out.iter().find(|(name, _)| name == n).unwrap().1.clone();
    assert!(matches!(get("Before"), Outcome::Pass));
    for t in ["ForEver", "ForEverRepeat"] {
        match get(t) {
            Outcome::NotRun(r) => assert!(r.contains("time limit"), "{r}"),
            o => panic!("{t}: {o:?}"),
        }
    }
    assert!(matches!(get("After"), Outcome::Pass));
    assert!(matches!(get("AfterFails"), Outcome::Fail(_)), "{:?}", get("AfterFails"));
}

/// Runs the test codeunit `name` of `src` with the time limit `limit`; the outcome per test and the wall time used.
fn run_with_limit(src: &str, name: &str, limit: std::time::Duration) -> (Vec<(String, Outcome)>, std::time::Duration) {
    let (ws, l) = al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), src.into())],
        }],
        &Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let engine = Engine::new(l.program.clone(), blocked);
    let oi = l.program.objects.iter().position(|o| o.name == name).unwrap();
    let started = std::time::Instant::now();
    let out = run_codeunit_limited(&engine, &l, oi, None, TestIsolation::Codeunit, Some(limit));
    (out, started.elapsed())
}

/// Al-runner issue 5698: the time limit applies to a test codeunit's `OnRun` trigger too, with a budget of its own (the
/// time of `OnRun` counts toward no test method). An `OnRun` that goes past it stops, and every test of the codeunit
/// gets the timeout reason (the outputs turn it into `Test exceeded <N>s timeout.`), which names the trigger.
#[test]
fn the_time_limit_applies_to_the_on_run_trigger_of_a_test_codeunit() {
    for (src, name) in [
        (
            "codeunit 61402 OnRunSpin { Subtype = Test;
            trigger OnRun() var i: Integer; begin while i < 300000000 do i += 1; end;
            [Test] procedure Noop() begin end; }",
            "OnRunSpin",
        ),
        (
            "codeunit 61403 OnRunSleeps { Subtype = Test;
            trigger OnRun() begin Sleep(600000); end;
            [Test] procedure Noop() begin end; }",
            "OnRunSleeps",
        ),
    ] {
        let (out, _) = run_with_limit(src, name, std::time::Duration::from_secs(1));
        match &out.iter().find(|(t, _)| t == "Noop").unwrap().1 {
            Outcome::NotRun(r) => {
                assert!(
                    r.starts_with("the test used more than the time limit of 1 s"),
                    "{name}: {r}"
                );
                assert!(r.contains("OnRun"), "{name}: {r}");
            }
            o => panic!("{name}: {o:?}"),
        }
    }
}

/// Al-runner issue 5605: `Sleep` does not wait, but the time it asks for counts toward the test's time limit, summed
/// over the test. A test whose Sleeps go past the limit is stopped and reported like a test over its CPU limit (the
/// reason starts with "the test used more than the time limit of N s", which the outputs turn into v2's timeout
/// row); one whose Sleeps stay under it passes. The sum starts again for each test.
#[test]
fn sleep_time_counts_toward_the_time_limit() {
    let src = "codeunit 61401 Sleepy { Subtype = Test;
            [Test] procedure RetryForEver() var Done: Boolean; Tries: Integer; begin
                repeat Sleep(400); Tries += 1; if Tries > 1000000 then Done := true; until Done; end;
            [Test] procedure OneLongSleep() begin Sleep(600000); Error('after the sleep'); end;
            [Test] procedure JustUnder() begin Sleep(1000); Sleep(999); end;
            [Test] procedure JustUnderAgain() begin Sleep(1999); end; }";
    let (out, wall) = run_with_limit(src, "Sleepy", std::time::Duration::from_secs(2));
    let get = |n: &str| out.iter().find(|(name, _)| name == n).unwrap().1.clone();
    for t in ["RetryForEver", "OneLongSleep"] {
        match get(t) {
            Outcome::NotRun(r) => {
                assert!(
                    r.starts_with("the test used more than the time limit of 2 s"),
                    "{t}: {r}"
                );
                assert!(r.contains("Sleep"), "{t}: {r}");
            }
            o => panic!("{t}: {o:?}"),
        }
    }
    assert!(matches!(get("JustUnder"), Outcome::Pass), "{:?}", get("JustUnder"));
    assert!(
        matches!(get("JustUnderAgain"), Outcome::Pass),
        "{:?}",
        get("JustUnderAgain")
    );
    // Sleep never waits: the whole codeunit takes far less than one limit.
    assert!(wall < std::time::Duration::from_secs(2), "{wall:?}");
}

/// The limit counts the CPU time of the test thread: a thread that waits is not stopped at the
/// limit (the machine's load must not decide an outcome), one that computes is.
#[cfg(unix)]
#[test]
fn the_time_limit_counts_cpu_time_not_wall_time() {
    let limit = std::time::Duration::from_millis(200);
    let flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let w = Watchdog::start(flag.clone(), limit);
    std::thread::sleep(limit * 3);
    assert!(!w.finish(), "a thread that waited was stopped");
    assert!(!flag.load(std::sync::atomic::Ordering::Relaxed));

    let flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let w = Watchdog::start(flag.clone(), limit);
    let started = std::time::Instant::now();
    let mut x = 0u64;
    while !flag.load(std::sync::atomic::Ordering::Relaxed) && started.elapsed() < std::time::Duration::from_secs(10) {
        x = x.wrapping_add(std::hint::black_box(1));
    }
    assert!(w.finish() && x > 0, "a thread that computed was not stopped");
}

/// The limit is per test: the CPU time a thread used before the test started does not count.
/// Three tests on one thread, each using 60 % of the limit, all stay under it.
#[cfg(any(unix, windows))]
#[test]
fn the_cpu_limit_is_measured_from_the_start_of_each_test() {
    let limit = std::time::Duration::from_millis(500);
    for n in 0..3 {
        let flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let w = Watchdog::start(flag.clone(), limit);
        let clock = crate::cpuclock::ThreadClock::current().expect("a CPU clock");
        let from = clock.read().expect("a reading");
        let mut x = 0u64;
        while clock.read().expect("a reading") - from < limit * 6 / 10 {
            x = x.wrapping_add(std::hint::black_box(1));
        }
        // Let the watchdog poll once more after the work.
        std::thread::sleep(WATCHDOG_POLL * 3);
        assert!(!w.finish() && x > 0, "test {n} was stopped under 60 % of the limit");
    }
}

/// The test names with the unprobed assumptions each ran through, from the first test codeunit of `src`.
fn assumptions_of(src: &str) -> Vec<(String, Vec<&'static str>)> {
    let (ws, l) = al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), src.into())],
        }],
        &Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let engine = Engine::new(l.program.clone(), Vec::new().into_iter().collect());
    let oi = l
        .program
        .objects
        .iter()
        .position(|o| {
            o.properties
                .iter()
                .any(|(k, v)| k.eq_ignore_ascii_case("Subtype") && v.eq_ignore_ascii_case("Test"))
        })
        .unwrap();
    let mut hits = Vec::new();
    let done = run_codeunit_full(
        &engine,
        &l,
        oi,
        None,
        TestIsolation::Codeunit,
        None,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut hits,
        None,
    );
    for (name, outcome) in &done {
        assert_eq!(*outcome, Outcome::Pass, "{name}");
    }
    hits.into_iter()
        .map(|(n, h)| (n, h.iter().map(|a| a.info().id).collect()))
        .collect()
}

const FLAGGED: &str = "codeunit 60150 T
{
    Subtype = Test;

    [Test]
    procedure RunsAnAssumption()
    var
        V: Variant;
        I: Integer;
    begin
        V := '12';
        I := V;
    end;

    [Test]
    procedure RunsNone()
    var
        X: Integer;
    begin
        X := 1;
    end;

    [Test]
    procedure RunsNoneAfterTheOneThatDid()
    var
        X: Integer;
    begin
        X := 2;
    end;
}";

#[test]
fn a_test_that_runs_an_assumed_path_is_marked_and_the_marks_do_not_leak_to_the_next_test() {
    let hits = assumptions_of(FLAGGED);
    let of = |n: &str| hits.iter().find(|(t, _)| t == n).map(|(_, h)| h.clone());
    assert_eq!(of("RunsAnAssumption"), Some(vec!["text-digits-to-integer"]), "{hits:?}");
    assert_eq!(of("RunsNone"), None, "{hits:?}");
    assert_eq!(of("RunsNoneAfterTheOneThatDid"), None, "{hits:?}");
}

const QUERY_AFTER_LAST_ROW: &str = r#"
table 60160 "QA Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }
query 60160 "QA Query" { elements { dataitem(R; "QA Row") { column(Id; Id) { } } } }
codeunit 60161 T
{
    Subtype = Test;
    [Test]
    procedure FilterAfterTheEndThenRead()
    var Row: Record "QA Row"; Q: Query "QA Query";
    begin
        Row.Id := 1; Row.Insert();
        Q.Open();
        while Q.Read() do;
        Q.SetRange(Id, 1);
        asserterror if Q.Read() then;
    end;
    [Test]
    procedure FilterAfterTheEndThenOpen()
    var Row: Record "QA Row"; Q: Query "QA Query";
    begin
        Row.Id := 1; Row.Insert();
        Q.Open();
        while Q.Read() do;
        Q.SetRange(Id, 1);
        Q.Open();
        if not Q.Read() then Error('no row');
    end;
}"#;

// A filter set after the last row and the Read that raises are pinned (errors/lt-ms-refusals-131/
// P131QuerySetRangeAfterRead.al:154-170), and so is the Open after the last row (query/lt-query-read-to-end/QueryReadToEnd.al:88-200):
// neither test is flagged.
#[test]
fn an_open_after_the_last_row_is_not_flagged() {
    let hits = assumptions_of(QUERY_AFTER_LAST_ROW);
    let of = |n: &str| hits.iter().find(|(t, _)| t == n).map(|(_, h)| h.clone());
    assert_eq!(of("FilterAfterTheEndThenRead"), None, "{hits:?}");
    assert_eq!(of("FilterAfterTheEndThenOpen"), None, "{hits:?}");
}

// The lowering can mark a statement as resting on an assumption (`Lowerer::assume`, `Stmt::Assume`): the test is flagged only when the
// statement runs. A Code[30] true branch against a Label false branch rests on `ternary-label-branch` (only Code[20] and Text[10] are
// pinned, types/TestTernaryContracts.al TI_Ternary_*); a Code[20] true branch does not.
const TERNARY_LABEL: &str = "codeunit 60170 T
{
    Subtype = Test;

    var
        Lbl: Label 'abc';

    [Test]
    procedure UnpinnedLength()
    var
        C30: Code[30];
        R: Code[30];
    begin
        R := C30 = '' ? C30 : Lbl;
        R := C30 <> '' ? C30 : Lbl;
        if R <> 'ABC' then Error('R %1', R);
    end;

    [Test]
    procedure PinnedLength()
    var
        C20: Code[20];
        R: Code[20];
    begin
        R := C20 <> '' ? C20 : Lbl;
    end;

    [Test]
    procedure NotRun()
    var
        C30: Code[30];
        R: Code[30];
        B: Boolean;
    begin
        if B then
            R := C30 <> '' ? C30 : Lbl;
    end;
}";

#[test]
fn an_unpinned_label_branch_length_is_flagged_only_when_its_statement_runs() {
    let hits = assumptions_of(TERNARY_LABEL);
    let of = |n: &str| hits.iter().find(|(t, _)| t == n).map(|(_, h)| h.clone());
    assert_eq!(of("UnpinnedLength"), Some(vec!["ternary-label-branch"]), "{hits:?}");
    assert_eq!(of("PinnedLength"), None, "{hits:?}");
    assert_eq!(of("NotRun"), None, "{hits:?}");
}

const TODAY: &str = "codeunit 60160 T
{
    Subtype = Test;

    [Test]
    procedure TodayIsTheDayTheOracleRan()
    begin
        if Format(Today, 0, 9) <> '2026-10-15' then Error('Today %1', Format(Today, 0, 9));
        if Format(DT2Date(CurrentDateTime), 0, 9) <> '2026-10-15' then Error('CurrentDateTime %1', Format(CurrentDateTime, 0, 9));
        if Format(WorkDate, 0, 9) <> '2026-10-15' then Error('WorkDate %1', Format(WorkDate, 0, 9));
    end;
}";

// The corpus oracle recorded values derived from Today on the day real BC ran (22:00 UTC on 2026-10-15); a run on
// another day gave "expected 10/15/26 but was 10/16/26". The run clock of a codeunit starts at the oracle's own
// timestamp of that codeunit's suite.
#[test]
fn a_codeunit_run_with_a_clock_start_sees_that_day_as_today() {
    let (ws, l) = al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), TODAY.into())],
        }],
        &Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let engine = Engine::new(l.program.clone(), Vec::new().into_iter().collect());
    let oi = l.program.objects.iter().position(|o| o.id == Some(60160)).unwrap();
    let mut scratch = Scratch {
        // 2026-10-15T22:00:23Z
        clock_start: Some(1_792_101_623_000),
        ..Default::default()
    };
    let done = run_codeunit_core(
        &engine,
        &l,
        oi,
        None,
        TestIsolation::Codeunit,
        None,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut Vec::new(),
        &mut scratch,
        None,
        false,
    );
    assert_eq!(done, vec![("TodayIsTheDayTheOracleRan".to_string(), Outcome::Pass)]);
}

const HIT_IN_ONRUN: &str = "codeunit 60152 T
{
    Subtype = Test;

    trigger OnRun()
    var
        V: Variant;
        I: Integer;
    begin
        V := '12';
        I := V;
    end;

    [Test]
    procedure FirstTestRunsNone()
    var
        X: Integer;
    begin
        X := 1;
    end;

    [Test]
    procedure SecondTestRunsAnAssumption()
    var
        V: Variant;
        I: Integer;
    begin
        V := '12';
        I := V;
    end;
}";

// Rework counts determinism as observable tooling quality: the marks of a test are those of the code that test ran,
// whatever ran on the thread before it and however often the same selection runs. (A run of the whole ERM bucket
// three times gave the same assumption lists for all 9333 tests once main's address-keyed state fixes were in; before
// them a page event mark moved between the tests of one codeunit from run to run.)
#[test]
fn the_marks_of_a_test_do_not_depend_on_what_ran_before_it_or_on_the_run() {
    let first = assumptions_of(HIT_IN_ONRUN);
    let of = |hits: &[(String, Vec<&'static str>)], n: &str| hits.iter().find(|(t, _)| t == n).map(|(_, h)| h.clone());
    assert_eq!(
        of(&first, "FirstTestRunsNone"),
        None,
        "a mark of OnRun leaked into the first test: {first:?}"
    );
    assert_eq!(
        of(&first, "SecondTestRunsAnAssumption"),
        Some(vec!["text-digits-to-integer"]),
        "{first:?}"
    );
    for _ in 0..5 {
        assert_eq!(
            assumptions_of(HIT_IN_ONRUN),
            first,
            "the same selection gave other marks"
        );
    }
}

/// A test refused before it starts also leaves out what BC would have committed (a cleanup, a setup the later tests
/// rely on; Tests-ERM "Discount Setup Notifications": a refused test that would have reset the posting setups, then a
/// test that failed on the rows the test before it left). Here the refused test's reach holds the construct it is refused
/// for (a Label true branch into a Variant), so what it would have done is not known: it may change anything, the shared globals and session
/// state included, so every later failure is not trusted, under Codeunit and Function isolation alike (coordinator's
/// decision 2026-10-19, owner's priority: no false results).
#[test]
fn a_failure_after_a_test_refused_before_it_started_for_a_construct_in_its_reach_is_not_trusted() {
    let src = "table 61412 Row { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
        codeunit 61411 Shared { Subtype = Test;
            [Test] procedure Refused() var V: Variant; T: Text; L: Label 'abc', Locked = true; B: Boolean; R: Record Row;
            begin R.DeleteAll(); V := B ? L : T; end;
            [Test] procedure Fails() var R: Record Row; begin R.Id := 1; R.Insert(); Error('x'); end;
            [Test] procedure FailsOther() begin Error('y'); end; }";
    let (ws, l) = al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), src.into())],
        }],
        &Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let engine = Engine::new(l.program.clone(), blocked);
    let oi = l.program.objects.iter().position(|o| o.name == "Shared").unwrap();
    for isolation in [TestIsolation::Codeunit, TestIsolation::Function] {
        let out = run_codeunit_limited(&engine, &l, oi, None, isolation, None);
        let get = |n: &str| out.iter().find(|(name, _)| name == n).unwrap().1.clone();
        assert!(matches!(get("Refused"), Outcome::NotRun(_)), "{:?}", get("Refused"));
        for later in ["Fails", "FailsOther"] {
            match get(later) {
                Outcome::NotRun(r) => assert!(
                    r.contains("not trusted") && r.contains("not known"),
                    "{isolation:?} {later}: {r}"
                ),
                o => panic!("{isolation:?} {later}: {o:?}"),
            }
        }
    }
}

// ADR-046 amendment 1, A4 item 10: the test session raises Company Triggers OnCompanyOpen then OnCompanyOpenCompleted to the
// app's subscribers once, before the first test of the codeunit, and not again for the second test
// (events/lt-bf-company-open/BfCompanyOpenTests.al:19-38, :40-50). What the runner does with the writes of a subscriber is not
// pinned: a test whose reach can read a table one wrote is not run, the other tests run (al-runner issue 5606: the whole
// codeunit was refused).
#[test]
fn company_triggers_run_once_in_the_test_session_before_the_first_test() {
    let o = outcomes(
        "codeunit 60906 State { SingleInstance = true; var L: Text; procedure Add(T: Text) begin L += T + ';'; end; procedure Get(): Text begin exit(L); end; }
            codeunit 60907 Subs {
                [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Company Triggers\", 'OnCompanyOpenCompleted', '', false, false)]
                local procedure Completed() var S: Codeunit State; begin S.Add('Completed'); end;
                [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Company Triggers\", 'OnCompanyOpen', '', false, false)]
                local procedure Open() var S: Codeunit State; begin S.Add('Open'); end; }
            codeunit 60102 T { Subtype = Test;
                [Test] procedure First() var S: Codeunit State; begin if S.Get() <> 'Open;Completed;' then Error('first %1', S.Get()); end;
                [Test] procedure Second() var S: Codeunit State; begin if S.Get() <> 'Open;Completed;' then Error('second %1', S.Get()); end; }",
    );
    for (n, r) in &o {
        assert_eq!(*r, Outcome::Pass, "{n}: {o:?}");
    }
    let o = outcomes(
        "table 60901 Co { fields { field(1; K; Code[10]) { } } }
            codeunit 60907 Subs {
                [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Company Triggers\", 'OnCompanyOpen', '', false, false)]
                local procedure Open() var R: Record Co; begin R.K := 'X'; if R.Insert() then; end; }
            table 60902 Other { fields { field(1; K; Code[10]) { } } }
            codeunit 60903 Helper { procedure Count(): Integer var R: Record Co; begin exit(R.Count()); end; }
            codeunit 60102 T { Subtype = Test;
                [Test] procedure First() begin end;
                [Test] procedure ReadsCo() var R: Record Co; begin if R.IsEmpty() then Error('empty'); end;
                [Test] procedure ReadsCoThroughAHelper() var H: Codeunit Helper; begin if H.Count() = 0 then Error('empty'); end;
                [Test] procedure WritesOther() var R: Record Other; begin R.K := 'Y'; R.Insert(); end; }",
    );
    let of = |n: &str| o.iter().find(|(t, _)| t == n).map(|(_, r)| r.clone()).unwrap();
    assert_eq!(of("First"), Outcome::Pass, "{o:?}");
    assert_eq!(of("WritesOther"), Outcome::Pass, "{o:?}");
    for n in ["ReadsCo", "ReadsCoThroughAHelper"] {
        assert!(
            matches!(&of(n), Outcome::NotRun(r) if r.contains("OnCompanyOpen") && r.contains("Co")),
            "{n}: {o:?}"
        );
    }
    // A subscriber whose reach is not known (a codeunit run by an id known only at run time) may write any table.
    let o = outcomes(
        "table 60901 Co { fields { field(1; K; Code[10]) { } } }
            codeunit 60907 Subs {
                [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Company Triggers\", 'OnCompanyOpen', '', false, false)]
                local procedure Open() var R: Record Co; Id: Integer; begin R.K := 'X'; if R.Insert() then; Id := R.Count(); Codeunit.Run(Id); end; }
            codeunit 60102 T { Subtype = Test;
                [Test] procedure First() begin end; }",
    );
    assert!(
        matches!(&o[0].1, Outcome::NotRun(r) if r.contains("OnCompanyOpen")),
        "{o:?}"
    );
}

/// [`with_baseline`] whose baseline also runs a company data codeunit (`--company-data demotool`, `company_data.rs`).
fn with_company_data(main: &str, dep: &str, company: crate::company_data::CompanyData) -> impl Fn(&str) -> Outcome {
    with_company_data_apps(main, &[("dep", &[], dep)], company)
}

/// [`with_company_data`] with several dependency apps: (name, the apps it depends on, source). The test app depends on
/// every one of them.
fn with_company_data_apps(
    main: &str,
    deps: &[(&str, &[&str], &str)],
    company: crate::company_data::CompanyData,
) -> impl Fn(&str) -> Outcome + use<> {
    let rt = company_data_runtime(main, deps, company);
    move |name: &str| {
        rt.first_outcome(&CaseId {
            codeunit: 50001,
            name: name.into(),
        })
    }
}

/// The run of [`with_company_data_apps`].
fn company_data_runtime(
    main: &str,
    deps: &[(&str, &[&str], &str)],
    company: crate::company_data::CompanyData,
) -> CorpusRuntime {
    let app = |id: &str, deps: &[&str], src: &str| AppSource {
        id: id.into(),
        name: id.into(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![(format!("{id}.al"), src.into())],
    };
    let opts = RunOptions {
        company_data: Some(company),
        ..Default::default()
    };
    let names: Vec<&str> = deps.iter().map(|d| d.0).collect();
    let mut apps = vec![app("main", &names, main)];
    let mut kinds = vec![al_sema::AppKind::Workspace];
    for (name, on, src) in deps {
        apps.push(app(name, on, src));
        kinds.push(al_sema::AppKind::Dependency);
    }
    CorpusRuntime::run_suites_with(
        apps,
        kinds,
        &[],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &opts,
    )
}

fn demo_company(resources: Option<std::path::PathBuf>) -> crate::company_data::CompanyData {
    crate::company_data::CompanyData {
        codeunit: 50130,
        name: "Build Company".into(),
        resources,
        identity: "test".into(),
        environment: None,
        installed_first: None,
        method: None,
        more_steps: Vec::new(),
        no_client: false,
        evaluation_company: false,
    }
}

// Microsoft's Legacy lane (BCApps build/scripts/ImportTestDataInBcContainer.ps1:907-923): only the base apps are installed
// when the company is created and the DemoTool runs; every other app is installed after, so its install code runs after
// the company data codeunit and its subscribers do not run before that. The company is built on an on-prem server.
#[test]
fn only_the_apps_installed_first_take_part_in_building_the_company_and_the_rest_install_after_it() {
    let dep = format!(
        "{SETUP_DEP}
codeunit 50130 \"Build Company\" {{
  trigger OnRun() var L: Record \"Order Log\"; S: Record \"Setup\"; H: DotNet NavTenantSettingsHelper; begin
    L.What := 'demo'; L.Insert(); OnBuilt();
    S.Get(); S.Value := S.Value + '+sandbox:' + Format(H.IsSandbox()); S.Modify(); end;
  [IntegrationEvent(false, false)] procedure OnBuilt() begin end; }}
dotnet {{ assembly(\"Microsoft.Dynamics.Nav.Ncl\") {{ type(\"Microsoft.Dynamics.Nav.NavUserAccount.NavTenantSettingsHelper\"; \"NavTenantSettingsHelper\") {{ }} }} }}"
    );
    let late = r#"
codeunit 50140 "Late Installer" { Subtype = Install;
  trigger OnInstallAppPerCompany() var L: Record "Order Log"; begin L.What := 'late-company'; L.Insert(); end; }
codeunit 50141 "Late Subs" {
  [EventSubscriber(ObjectType::Codeunit, Codeunit::"Build Company", 'OnBuilt', '', false, false)]
  local procedure OnBuilt() var L: Record "Order Log"; begin L.What := 'late-sub'; L.Insert(); end; }
"#;
    let main = "codeunit 50001 \"T\" { Subtype = Test; var L: Codeunit \"Lib\"; \
         [Test] procedure Order() var O: Record \"Order Log\"; T: Text; begin \
           O.FindSet(); repeat T += O.What + ';'; until O.Next() = 0; \
           if T <> 'A-company;B-company;init;demo;late-company;' then Error(T); end; \
         [Test] procedure OnPrem() begin if L.ReadSetup() <> 'installed+initialized+sandbox:No' then Error(L.ReadSetup()); end; }";
    let mut company = demo_company(None);
    company.installed_first = Some(vec!["DEP".into()]);
    company.environment = Some(al_host::Environment::OnPrem);
    let outcome = with_company_data_apps(main, &[("dep", &[], &dep), ("late", &["dep"], late)], company);
    assert_eq!(outcome("Order"), Outcome::Pass);
    assert_eq!(outcome("OnPrem"), Outcome::Pass);
}

#[test]
fn a_company_data_codeunit_runs_after_company_initialize_and_every_test_starts_from_what_it_wrote() {
    // BCApps' CI for the Legacy lane: Company-Initialize, then the DemoTool's action (`Invoke-LegacyDemoDataTool`). The
    // codeunit reads its resource file through `File`, by a relative path, matched without case.
    let dir = std::env::temp_dir().join(format!("alr-company-data-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("Config.XML"), b"from-file").unwrap();
    let dep = format!(
        "{SETUP_DEP}
codeunit 50130 \"Build Company\" {{
  trigger OnRun() var S: Record \"Setup\"; L: Record \"Order Log\"; F: File; I: InStream; T: Text; begin
    F.Open('config.xml'); F.CreateInStream(I); I.ReadText(T); F.Close();
    S.Get(); S.Value := S.Value + '+' + T; S.Modify();
    L.What := 'demo'; L.Insert(); end; }}"
    );
    let outcome = with_company_data(
        "codeunit 50001 \"T\" { Subtype = Test; var L: Codeunit \"Lib\"; \
             [Test] procedure Built() begin if L.ReadSetup() <> 'installed+initialized+from-file' then Error(L.ReadSetup()); end; \
             [Test] procedure Order() var O: Record \"Order Log\"; T: Text; begin \
               O.FindSet(); repeat T += O.What + ';'; until O.Next() = 0; \
               if T <> 'A-company;B-company;init;demo;' then Error(T); end; }",
        &dep,
        demo_company(Some(dir.clone())),
    );
    assert_eq!(outcome("Built"), Outcome::Pass);
    assert_eq!(outcome("Order"), Outcome::Pass);
    let _ = std::fs::remove_dir_all(&dir);
}

// Orchestrator decision (2026-10-22): BCApps' CI runs the DemoTool's codeunit 101899 without a client
// (Invoke-LegacyDemoDataTool), and 101900 ends with a Message (CreateDemonstrationData.Codeunit.al:133). While the company
// is built, a Message is accepted and written to the run log; Confirm and StrMenu stay refused.
#[test]
fn a_message_while_the_company_is_built_is_accepted_and_a_confirm_is_not() {
    let main = "codeunit 50001 \"T\" { Subtype = Test; [Test] procedure Plain() begin end; }";
    let dep = format!(
        "{SETUP_DEP}
codeunit 50130 \"Build Company\" {{
  trigger OnRun() var L: Record \"Order Log\"; begin L.What := 'demo'; L.Insert(); Message('Built %1', 1); end; }}"
    );
    let outcome = with_company_data(main, &dep, demo_company(None));
    assert_eq!(outcome("Plain"), Outcome::Pass);
    for ui in ["if Confirm('Sure?') then;", "if StrMenu('A,B') = 0 then;"] {
        let dep = format!(
            "{SETUP_DEP}
codeunit 50130 \"Build Company\" {{
  trigger OnRun() begin {ui} end; }}"
        );
        let Outcome::NotRun(why) = with_company_data(main, &dep, demo_company(None))("Plain") else {
            panic!("{ui}")
        };
        assert!(why.starts_with(crate::company_data::BLOCKED_NOTE), "{ui}: {why}");
    }
}

/// A company built in steps, as BCApps builds the Contoso company (`CompanyData::contoso`): a method of the first
/// codeunit, then the `OnRun` of the next, each committed on its own, without a client, in an evaluation company.
fn stepped_company() -> crate::company_data::CompanyData {
    let mut c = demo_company(None);
    c.method = Some("Setup".into());
    c.more_steps = vec![crate::company_data::CompanyStep {
        codeunit: 50131,
        name: "Tenant Step".into(),
        method: None,
    }];
    c.no_client = true;
    c.evaluation_company = true;
    c
}

const COMPANY_TABLE: &str = r#"
table 2000000006 Company { DataPerCompany = false;
  fields { field(1; Name; Text[30]) { } field(2; "Evaluation Company"; Boolean) { } field(3; "Display Name"; Text[250]) { } field(8000; Id; Guid) { } }
  keys { key(PK; Name) { } } }
"#;

// BCApps build/scripts/ImportTestDataInBcContainer.ps1 (releases/29.0) :307-326: Company-Initialize, then each Contoso step
// through Invoke-NavContainerCodeunit, a session without a client (GuiAllowed false; owner decision 2026-10-23) that
// commits at its end; the Uncategorized company is created with -EvaluationCompany (:903).
#[test]
fn a_company_built_in_steps_runs_each_step_without_a_client_in_an_evaluation_company() {
    let dep = format!(
        "{SETUP_DEP}{COMPANY_TABLE}
codeunit 50130 \"Build Company\" {{
  trigger OnRun() var L: Record \"Order Log\"; begin L.What := 'onrun'; L.Insert(); end;
  procedure Setup() var L: Record \"Order Log\"; C: Record Company; begin
    L.What := 'setup gui:' + Format(GuiAllowed()); L.Insert();
    C.Get(CompanyName()); L.\"No.\" := 0; L.What := 'eval:' + Format(C.\"Evaluation Company\"); L.Insert(); end; }}
codeunit 50131 \"Tenant Step\" {{
  trigger OnRun() var L: Record \"Order Log\"; begin L.What := 'tenant gui:' + Format(GuiAllowed()); L.Insert(); end; }}"
    );
    let main = "codeunit 50001 \"T\" { Subtype = Test; \
         [Test] procedure Order() var O: Record \"Order Log\"; T: Text; begin \
           O.FindSet(); repeat T += O.What + ';'; until O.Next() = 0; \
           if T <> 'A-company;B-company;init;setup gui:No;eval:Yes;tenant gui:No;' then Error(T); end; \
         [Test] procedure TestsHaveAClient() begin if not GuiAllowed() then Error('no client'); end; \
         [Test] procedure Evaluation() var C: Record Company; begin C.Get(CompanyName()); if not C.\"Evaluation Company\" then Error('not evaluation'); end; }";
    let rt = company_data_runtime(main, &[("dep", &[], &dep)], stepped_company());
    let outcome = |n: &str| {
        rt.first_outcome(&CaseId {
            codeunit: 50001,
            name: n.into(),
        })
    };
    assert_eq!(outcome("Order"), Outcome::Pass);
    assert_eq!(outcome("TestsHaveAClient"), Outcome::Pass);
    assert_eq!(outcome("Evaluation"), Outcome::Pass);
    // The build relied on the session without a client: the run lists the assumption with the baseline.
    assert!(
        rt.phases
            .baseline_assumptions
            .iter()
            .any(|a| a == "contoso-build-no-client"),
        "{:?}",
        rt.phases.baseline_assumptions
    );
}

#[test]
fn a_step_that_does_not_run_to_its_end_blocks_the_company() {
    let dep = format!(
        "{SETUP_DEP}
table 50103 \"Tenant Log\" {{ DataPerCompany = false; fields {{ field(1; K; Integer) {{ }} }} keys {{ key(PK; K) {{ }} }} }}
codeunit 50130 \"Build Company\" {{
  procedure Setup() var L: Record \"Order Log\"; begin L.What := 'setup'; L.Insert(); Debugger.Deactivate(); end; }}
codeunit 50131 \"Tenant Step\" {{
  trigger OnRun() var T: Record \"Tenant Log\"; begin T.K := 1; T.Insert(); end; }}"
    );
    let main = "codeunit 50001 \"T\" { Subtype = Test; [Test] procedure Plain() begin end; }";
    let mut company = stepped_company();
    company.evaluation_company = false;
    let Outcome::NotRun(why) = with_company_data(main, &dep, company.clone())("Plain") else {
        panic!("not blocked")
    };
    assert!(
        why.starts_with(crate::company_data::BLOCKED_NOTE) && why.contains("Build Company.Setup"),
        "{why}"
    );
    // A step's method the codeunit does not have blocks the company too.
    let dep = format!(
        "{SETUP_DEP}
codeunit 50130 \"Build Company\" {{ procedure Other() begin end; }}
codeunit 50131 \"Tenant Step\" {{ trigger OnRun() begin end; }}"
    );
    let Outcome::NotRun(why) = with_company_data(main, &dep, company)("Plain") else {
        panic!("not blocked")
    };
    assert!(why.contains("has no method Setup"), "{why}");
}

#[test]
fn a_company_data_codeunit_that_does_not_run_to_its_end_leaves_every_test_not_run() {
    let dep = format!(
        "{SETUP_DEP}
codeunit 50130 \"Build Company\" {{
  trigger OnRun() var L: Record \"Order Log\"; begin L.What := 'demo'; L.Insert(); Debugger.Deactivate(); end; }}"
    );
    let main = "codeunit 50001 \"T\" { Subtype = Test; [Test] procedure Plain() begin end; }";
    let outcome = with_company_data(main, &dep, demo_company(None));
    let Outcome::NotRun(why) = outcome("Plain") else {
        panic!("{:?}", outcome("Plain"))
    };
    assert!(
        why.starts_with(crate::company_data::BLOCKED_NOTE) && why.contains("Build Company.OnRun"),
        "{why}"
    );
    // A codeunit the program does not have blocks the company too.
    let outcome = with_company_data(main, SETUP_DEP, demo_company(None));
    let Outcome::NotRun(why) = outcome("Plain") else {
        panic!("{:?}", outcome("Plain"))
    };
    assert!(why.contains("no dependency app has codeunit 50130"), "{why}");
}

// al-runner issue 5641: code in a company-open subscriber's static reach that does not run (a routine refused at lowering
// for a `[RunOnClient]` variable, a run by an id known only at run time, behind a branch that is not taken) refused every
// test of the run. The subscriber runs to its end before the first test, so what it wrote is known: the tables it wrote at
// run time decide. A test whose static reach reads one is not run; a test whose static reach is not known runs, and is not
// run when it touches a written table while it runs.
#[test]
fn a_company_open_subscriber_refuses_only_tests_that_touch_what_it_wrote_at_run_time() {
    let o = outcomes(
        "dotnet { assembly(\"System.Drawing\") { type(\"System.Drawing.Printing.PrinterSettings\"; \"PrinterSettings\") { } } }
            table 60901 Co { fields { field(1; K; Code[10]) { } } }
            table 60902 Other { fields { field(1; K; Code[10]) { } } }
            codeunit 60907 Subs {
                [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Company Triggers\", 'OnCompanyOpenCompleted', '', false, false)]
                local procedure Open() var R: Record Co; Id: Integer;
                begin
                    R.K := 'OPENED'; if R.Insert() then;
                    if R.K = 'NEVER' then begin ClientPrinter(); Id := R.Count(); Codeunit.Run(Id); end;
                end;
                local procedure ClientPrinter(): Text var [RunOnClient] Settings: DotNet PrinterSettings;
                begin Settings := Settings.PrinterSettings(); exit(Settings.PrinterName); end; }
            codeunit 60903 ReadsCo { trigger OnRun() var R: Record Co; begin if R.IsEmpty() then Error('empty'); end; }
            codeunit 60904 NoTable { trigger OnRun() begin end; }
            codeunit 60102 T { Subtype = Test;
                [Test] procedure Unrelated() begin end;
                [Test] procedure ReadsOtherTable() var R: Record Other; begin if not R.IsEmpty() then Error('not empty'); end;
                [Test] procedure ReadsWrittenTable() var R: Record Co; begin if R.IsEmpty() then Error('empty'); end;
                [Test] procedure UnknownReachTouchesNothing() var Id: Integer; begin Id := 60904; Codeunit.Run(Id); end;
                [Test] procedure UnknownReachReadsWrittenTable() var Id: Integer; begin Id := 60903; Codeunit.Run(Id); end; }",
    );
    let of = |n: &str| o.iter().find(|(t, _)| t == n).map(|(_, r)| r.clone()).unwrap();
    for n in ["Unrelated", "ReadsOtherTable", "UnknownReachTouchesNothing"] {
        assert_eq!(of(n), Outcome::Pass, "{n}: {o:?}");
    }
    for n in ["ReadsWrittenTable", "UnknownReachReadsWrittenTable"] {
        assert!(
            matches!(&of(n), Outcome::NotRun(r) if r.contains("OnCompanyOpen") && r.contains("table Co")),
            "{n}: {o:?}"
        );
    }
    // A subscriber whose reach is not known and that wrote nothing when it ran refuses no test.
    let o = outcomes(
        "table 60901 Co { fields { field(1; K; Code[10]) { } } }
            codeunit 60907 Subs {
                [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Company Triggers\", 'OnCompanyOpen', '', false, false)]
                local procedure Open() var R: Record Co; Id: Integer; begin if R.K = 'NEVER' then begin R.Insert(); Codeunit.Run(Id); end; end; }
            codeunit 60102 T { Subtype = Test;
                [Test] procedure ReadsCo() var R: Record Co; begin if not R.IsEmpty() then Error('not empty'); end; }",
    );
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
}

// A test's outcome must not depend on which other test codeunits are selected (ADR-013, demand-driven compile): a
// selected test that calls a procedure of a test codeunit the selection leaves out (a test library that is itself a test
// codeunit) runs that procedure, and what it reaches in a dependency app, as it does when both codeunits are selected.
// Found on NP Retail: codeunit 85020 calls `NPR EFT Tests.GenericEFTPaymentSuccess` of test codeunit 85004.
#[test]
fn a_test_calling_an_unselected_test_codeunit_has_the_outcome_of_a_run_that_selects_both() {
    let dep = "codeunit 50200 \"Dep Lib\" { procedure Two(): Integer begin exit(2); end; }";
    let main = "codeunit 50001 \"Caller Test\" { Subtype = Test; var B: Codeunit \"Callee Test\"; \
         [Test] procedure UsesOther() begin if B.Helper() <> 3 then Error('wrong'); end; }
codeunit 50002 \"Callee Test\" { Subtype = Test; var L: Codeunit \"Dep Lib\"; \
         procedure Helper(): Integer begin exit(L.Two() + 1); end; \
         [Test] procedure Own() begin if Helper() <> 3 then Error('wrong'); end; }";
    let app = |id: &str, deps: &[&str], src: &str| AppSource {
        id: id.into(),
        name: id.into(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![(format!("{id}.al"), src.into())],
    };
    let run = |only: Option<Vec<(usize, u32)>>| {
        let rt = CorpusRuntime::run_suites_with(
            vec![app("main", &["dep"], main), app("dep", &[], dep)],
            vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
            &[],
            &Default::default(),
            None,
            Default::default(),
            &[SuiteRun::everything()],
            &RunOptions {
                only,
                ..Default::default()
            },
        );
        rt.first_outcome(&CaseId {
            codeunit: 50001,
            name: "UsesOther".into(),
        })
    };
    let both = run(Some(vec![(0, 50001), (0, 50002)]));
    assert!(matches!(both, Outcome::Pass), "{both:?}");
    assert_eq!(run(Some(vec![(0, 50001)])), both, "selected alone");
    assert_eq!(run(None), both, "everything");
}

// ADR-058 (stability mode): a tiny app with an order-dependent test, a work-date-dependent test, a test that reads
// the date of the run clock and one that reads `Library - Random`'s seed after the runner's reset step. Codeunits
// 130301 and 130440 stand in for Microsoft's (same ids, same calls), so the runner finds them as it finds the real ones.
const STABILITY_APP: &str = "codeunit 130440 \"Library - Random\"
{
    SingleInstance = true;
    var
        Seed: Integer;
    procedure SetSeed(Val: Integer): Integer
    begin
        Seed := Val;
        exit(Seed);
    end;
    procedure GetSeed(): Integer
    begin
        exit(Seed);
    end;
}
codeunit 130301 \"Reset State Before Test Run\"
{
    SingleInstance = true;
    trigger OnRun()
    var
        LibraryRandom: Codeunit \"Library - Random\";
    begin
        LibraryRandom.SetSeed(1);
    end;
}
codeunit 60990 StabilityProbe
{
    Subtype = Test;
    var
        Counter: Integer;

    [Test]
    procedure First()
    begin
        Counter += 1;
    end;

    [Test]
    procedure SecondNeedsFirst()
    begin
        if Counter <> 1 then
            Error('order: counter is %1', Counter);
    end;

    [Test]
    procedure WorkDateIsToday()
    begin
        if WorkDate() <> Today() then
            Error('work date %1 is not today %2', WorkDate(), Today());
    end;

    [Test]
    procedure NotOnLeapDay()
    begin
        if (Date2DMY(Today(), 1) = 29) and (Date2DMY(Today(), 2) = 2) then
            Error('today is a leap day');
    end;

    [Test]
    procedure SeedIsOne()
    var
        LibraryRandom: Codeunit \"Library - Random\";
    begin
        if LibraryRandom.GetSeed() <> 1 then
            Error('seed %1', LibraryRandom.GetSeed());
    end;
}";

fn stability_outcomes(perturb: crate::stability::Perturb) -> Vec<(String, &'static str)> {
    let opts = RunOptions {
        perturb,
        ..Default::default()
    };
    let rt = CorpusRuntime::run_suites_with(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), STABILITY_APP.into())],
        }],
        vec![al_sema::AppKind::Workspace],
        &[],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &opts,
    );
    let mut v: Vec<(String, &'static str)> = rt
        .results()
        .iter()
        .filter(|r| r.codeunit == 60990)
        .map(|r| {
            let k = match &r.outcome {
                Outcome::Pass => "pass",
                Outcome::Fail(_) => "fail",
                Outcome::NotRun(why) => panic!("{} not run: {why}", r.test),
            };
            (r.test.clone(), k)
        })
        .collect();
    v.sort();
    v
}

fn failing(perturb: crate::stability::Perturb) -> Vec<String> {
    stability_outcomes(perturb)
        .into_iter()
        .filter(|(_, k)| *k == "fail")
        .map(|(t, _)| t)
        .collect()
}

#[test]
fn stability_variants_change_only_what_they_name() {
    use crate::stability::{DateShift, MethodOrder, Perturb};
    let base = stability_outcomes(Perturb::default());
    assert_eq!(base.len(), 5);
    assert!(base.iter().all(|(_, k)| *k == "pass"), "{base:?}");
    let reverse = Perturb {
        method_order: MethodOrder::Reverse,
        ..Default::default()
    };
    assert_eq!(failing(reverse), ["SecondNeedsFirst"]);
    let alone = Perturb {
        alone: true,
        ..Default::default()
    };
    assert_eq!(failing(alone), ["SecondNeedsFirst"]);
    let work_date = Perturb {
        work_date: DateShift::parse("+1Y", false).ok(),
        ..Default::default()
    };
    assert_eq!(failing(work_date), ["WorkDateIsToday"]);
    let seed = Perturb {
        library_seed: Some(811_203),
        ..Default::default()
    };
    assert_eq!(failing(seed), ["SeedIsOne"]);
    // The clock moves Today and the default work date together.
    let leap = Perturb {
        clock_date: DateShift::parse("2028-02-29", true).ok(),
        ..Default::default()
    };
    assert_eq!(failing(leap), ["NotOnLeapDay"]);
}

/// Phase 2 of ADR-058: each test fails with the host value it reads, so a variant shows which values it moved.
const STABILITY_HOST_APP: &str = "codeunit 60991 StabilityHostProbe
{
    Subtype = Test;

    [Test]
    procedure HostRandom()
    begin
        Error('%1', Random(1000000));
    end;

    [Test]
    procedure NewGuid()
    begin
        Error('%1', CreateGuid());
    end;

    [Test]
    procedure TimeOfDay()
    begin
        Error('%1', Format(Time(), 0, '<Hours24,2>:<Minutes,2>'));
    end;

    [Test]
    procedure ClockStep()
    var
        A: DateTime;
        B: DateTime;
    begin
        A := CurrentDateTime();
        B := CurrentDateTime();
        Error('%1', B - A);
    end;

    [Test]
    procedure Identity()
    begin
        Error('%1|%2|%3|%4|%5', UserId(), UserSecurityId(), SessionId(), ServiceInstanceId(), TenantId());
    end;

    [Test]
    procedure Company()
    begin
        Error('%1', CompanyName());
    end;
}
codeunit 60992 StabilityWriter
{
    Subtype = Test;

    [Test]
    procedure Writes()
    var
        R: Record StabilityRow;
    begin
        R.Id := 1;
        R.Insert();
    end;
}
codeunit 60993 StabilityReader
{
    Subtype = Test;

    [Test]
    procedure SeesTheWriter()
    var
        R: Record StabilityRow;
    begin
        if R.IsEmpty() then
            Error('the writer has not run');
    end;
}
table 60990 StabilityRow
{
    fields
    {
        field(1; Id; Integer) { }
    }
}";

fn stability_host_messages(perturb: crate::stability::Perturb, disabled: bool) -> Vec<(String, String)> {
    let opts = RunOptions {
        perturb,
        isolation: disabled.then_some(TestIsolation::Disabled),
        ..Default::default()
    };
    let rt = CorpusRuntime::run_suites_with(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), STABILITY_HOST_APP.into())],
        }],
        vec![al_sema::AppKind::Workspace],
        &[],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &opts,
    );
    let mut v: Vec<(String, String)> = rt
        .results()
        .iter()
        .map(|r| {
            let m = match &r.outcome {
                Outcome::Pass => "pass".to_string(),
                Outcome::Fail(m) => m.clone(),
                Outcome::NotRun(why) => panic!("{} not run: {why}", r.test),
            };
            (r.test.clone(), m)
        })
        .collect();
    v.sort();
    v
}

/// The tests whose message differs from the base variant's.
fn moved(perturb: crate::stability::Perturb, disabled: bool) -> Vec<String> {
    let base = stability_host_messages(Default::default(), disabled);
    let other = stability_host_messages(perturb, disabled);
    assert_eq!(base.len(), other.len());
    base.iter()
        .zip(&other)
        .filter(|(a, b)| a.1 != b.1)
        .map(|(a, _)| a.0.clone())
        .collect()
}

#[test]
fn stability_host_value_variants_change_only_what_they_name() {
    use crate::stability::{Perturb, apply};
    let of = |name: &str, v: serde_json::Value| {
        let mut p = Perturb::default();
        apply(&mut p, name, &v).unwrap();
        p
    };
    // The base run is repeatable.
    assert_eq!(moved(Perturb::default(), false), Vec::<String>::new());
    assert_eq!(moved(of("random.hostSeed", 7.into()), false), ["HostRandom"]);
    assert_eq!(moved(of("guid.seed", 7.into()), false), ["NewGuid"]);
    assert_eq!(
        moved(of("clock.timeOfDay", "23:59:59.000".into()), false),
        ["TimeOfDay"]
    );
    assert_eq!(moved(of("clock.stepMs", 250.into()), false), ["ClockStep"]);
    assert_eq!(moved(of("identity.userId", "OTHERUSER".into()), false), ["Identity"]);
    assert_eq!(
        moved(
            of(
                "identity.userSecurityId",
                "{0F1E2D3C-4B5A-6978-8796-A5B4C3D2E1F0}".into()
            ),
            false
        ),
        ["Identity"]
    );
    assert_eq!(moved(of("identity.sessionId", 42.into()), false), ["Identity"]);
    assert_eq!(moved(of("identity.serviceInstanceId", 3.into()), false), ["Identity"]);
    assert_eq!(moved(of("identity.tenantId", "contoso".into()), false), ["Identity"]);
    assert_eq!(moved(of("company.name", "Other Company".into()), false), ["Company"]);
    // The values a test reads are the variant's.
    let msgs = stability_host_messages(of("company.name", "Other Company".into()), false);
    assert!(msgs.contains(&("Company".into(), "Other Company".into())), "{msgs:?}");
    let msgs = stability_host_messages(of("clock.timeOfDay", "23:59:59.000".into()), false);
    assert!(msgs.contains(&("TimeOfDay".into(), "23:59".into())), "{msgs:?}");
    // Codeunit order matters only when codeunits share the database (--isolation disabled).
    assert_eq!(
        moved(of("order.codeunits", "reverse".into()), false),
        Vec::<String>::new()
    );
    let base = stability_host_messages(Default::default(), true);
    assert!(base.contains(&("SeesTheWriter".into(), "pass".into())), "{base:?}");
    // Reversed, the reader runs before the writer. (The probe codeunit runs at another place in the chain too, so its
    // Guid moves with the session's counter; only the pass/fail outcomes are compared here.)
    let rev = stability_host_messages(of("order.codeunits", "reverse".into()), true);
    assert!(
        rev.contains(&("SeesTheWriter".into(), "the writer has not run".into())),
        "{rev:?}"
    );
    let passing = |v: &[(String, String)]| v.iter().filter(|(_, m)| m == "pass").count();
    assert_eq!(passing(&base) - passing(&rev), 1, "{rev:?}");
}

/// ADR-061: an install codeunit that keeps what it drew, and a test that reports it next to its own draws.
const RUN_SEED_INSTALL_APP: &str = "table 60995 SeedRow
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Drawn; Integer) { }
        field(3; G; Guid) { }
    }
}
codeunit 60996 SeedInstall
{
    Subtype = Install;
    trigger OnInstallAppPerCompany()
    var
        R: Record SeedRow;
    begin
        R.Id := 1;
        R.Drawn := Random(1000000);
        R.G := CreateGuid();
        R.Insert();
    end;
}
codeunit 60997 SeedInstallProbe
{
    Subtype = Test;

    [Test]
    procedure InstallDrew()
    var
        R: Record SeedRow;
    begin
        R.Get(1);
        Error('%1 %2', R.Drawn, R.G);
    end;
}";

fn seeded_messages(app: &str, run_seed: crate::seed::RunSeed, only: Option<(u32, &str)>) -> Vec<(String, String)> {
    let opts = RunOptions {
        run_seed,
        run_only: only.map(|(c, t)| Arc::new(std::collections::BTreeSet::from([(c, t.to_lowercase())]))),
        ..Default::default()
    };
    let rt = CorpusRuntime::run_suites_with(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), app.into())],
        }],
        vec![al_sema::AppKind::Workspace],
        &[],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &opts,
    );
    let mut v: Vec<(String, String)> = rt
        .results()
        .iter()
        .filter_map(|r| match &r.outcome {
            Outcome::Pass => Some((r.test.clone(), "pass".to_string())),
            Outcome::Fail(m) => Some((r.test.clone(), m.clone())),
            Outcome::NotRun(_) => None,
        })
        .collect();
    v.sort();
    v
}

fn seed(n: i32) -> crate::seed::RunSeed {
    crate::seed::RunSeed::new(n, crate::seed::SeedSource::Option)
}

fn message<'a>(v: &'a [(String, String)], test: &str) -> &'a str {
    &v.iter()
        .find(|(t, _)| t == test)
        .unwrap_or_else(|| panic!("{test} in {v:?}"))
        .1
}

#[test]
fn the_run_seed_drives_random_and_guids_per_test_method() {
    let a = seeded_messages(STABILITY_HOST_APP, seed(4271833), None);
    let again = seeded_messages(STABILITY_HOST_APP, seed(4271833), None);
    assert_eq!(a, again, "the same seed gives the same values");
    let b = seeded_messages(STABILITY_HOST_APP, seed(4271834), None);
    assert_ne!(message(&a, "HostRandom"), message(&b, "HostRandom"));
    assert_ne!(message(&a, "NewGuid"), message(&b, "NewGuid"));
    // Only what the seed drives moves.
    for t in ["TimeOfDay", "ClockStep", "Identity", "Company"] {
        assert_eq!(message(&a, t), message(&b, t), "{t}");
    }
    // `--seed N --test NewGuid`: the test alone sees what it saw in the full run.
    let alone = seeded_messages(STABILITY_HOST_APP, seed(4271833), Some((60991, "NewGuid")));
    assert_eq!(message(&alone, "NewGuid"), message(&a, "NewGuid"));
    let alone = seeded_messages(STABILITY_HOST_APP, seed(4271833), Some((60991, "HostRandom")));
    assert_eq!(message(&alone, "HostRandom"), message(&a, "HostRandom"));
}

#[test]
fn install_code_keeps_the_fixed_seed_whatever_the_run_seed() {
    let a = seeded_messages(RUN_SEED_INSTALL_APP, seed(4271833), None);
    let b = seeded_messages(RUN_SEED_INSTALL_APP, seed(17), None);
    let fixed = seeded_messages(RUN_SEED_INSTALL_APP, Default::default(), None);
    assert_eq!(message(&a, "InstallDrew"), message(&b, "InstallDrew"));
    assert_eq!(message(&a, "InstallDrew"), message(&fixed, "InstallDrew"));
}

#[test]
fn seed_library_hands_library_random_the_derived_seed() {
    let opts = |library| {
        let mut s = seed(4271833);
        s.library = library;
        s
    };
    let off = seeded_messages(STABILITY_APP, opts(false), None);
    assert_eq!(
        message(&off, "SeedIsOne"),
        "pass",
        "by default the reset step's SetSeed(1) stays"
    );
    let on = seeded_messages(STABILITY_APP, opts(true), None);
    let want = al_host::seed::derive_library(4271833, 60990, "SeedIsOne");
    assert_eq!(message(&on, "SeedIsOne"), format!("seed {want}"));
}

/// The work ratchet (`al-test-cli`, `metrics/<major>/work.json`): a run counts what it compiled, the rows its install
/// baseline holds and the tests it ran.
#[test]
fn a_run_counts_its_work() {
    let dep = "table 60900 Setup { fields { field(1; K; Code[10]) { } } }
        codeunit 60901 Inst { Subtype = Install;
            trigger OnInstallAppPerCompany() var R: Record Setup; begin R.K := 'A'; R.Insert(); R.K := 'B'; R.Insert(); end; }
        codeunit 60902 Unused { procedure P() begin end; }";
    let main = "codeunit 60990 T { Subtype = Test;
        [Test] procedure Reads() var R: Record Setup; begin if R.Count() <> 2 then Error('rows %1', R.Count()); end;
        [Test] procedure Other() begin end; }";
    let app = |id: &str, deps: &[&str], src: &str| AppSource {
        id: id.into(),
        name: id.into(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![(format!("{id}.al"), src.into())],
    };
    let rt = CorpusRuntime::run_suites_with(
        vec![app("main", &["dep"], main), app("dep", &[], dep)],
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        &[],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions::default(),
    );
    assert!(
        rt.results().iter().all(|r| r.outcome == Outcome::Pass),
        "{:?}",
        rt.results()
    );
    let w = rt.phases.work;
    assert_eq!(w.compiles, 1);
    assert_eq!(w.dependency_apps, 1);
    assert_eq!(w.dependency_apps_from_cache, 0);
    assert_eq!((w.cache_layers_hit, w.cache_layers_built), (0, 0));
    assert_eq!(w.lower.objects_lowered, 1);
    assert_eq!(w.lower.dependency_objects_lowered, 2, "{w:?}");
    assert_eq!(w.baseline_rows, Some(2), "{w:?}");
    assert_eq!(w.tests_run, 2);
}
