//! What a failed test leaves for the later tests of its codeunit under al-runner's own runner with TestIsolation =
//! Codeunit: the number sequences and the manual bindings. The cells copy corpus PR 1465 (draft, "NumberSequence and
//! manual bindings after a test that commits and fails"), tests/al-language-isolation-probe/src/SqCommitFail.al and
//! SqBindFailed.al, measured on BC 27.5, 28.5 and 29.0 under runners 61301 and 130450 (assertions in SqReader.al). Until a
//! corpus bump pins them, a test that reads what such a failure left is a flagged pass.

use super::runner_isolation::{not_run, results};
use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::isolation::TestIsolation;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};

const SEQ: &str = "sequence-after-failed-test";
const BIND: &str = "binding-after-failed-test";

/// The reading of a sequence: `missing`, or its current value and the next number.
const READ: &str = r#"
codeunit 50739 "P4 Read"
{
    procedure Of(Name: Text): Text
    begin
        if not NumberSequence.Exists(Name) then
            exit('missing');
        exit(StrSubstNo('cur=%1 next=%2', NumberSequence.Current(Name), NumberSequence.Next(Name)));
    end;
}
"#;

type Row = (u32, String, Outcome, Vec<&'static str>);

fn cells(src: &str, isolation: TestIsolation) -> Vec<Row> {
    results(&format!("{READ}\n{src}"), isolation)
}

fn row<'a>(o: &'a [Row], test: &str) -> &'a Row {
    o.iter()
        .find(|r| r.1 == test)
        .unwrap_or_else(|| panic!("no {test} in {o:?}"))
}

/// `test` failed with `seen` and passed through `id`.
fn flagged(o: &[Row], test: &str, seen: &str, id: &str) {
    let r = row(o, test);
    assert_eq!(r.2, Outcome::Fail(seen.into()), "{test}: {o:?}");
    assert!(r.3.contains(&id), "{test} is not flagged {id}: {r:?}");
}

/// `test` failed with `seen` and passed through no assumption.
fn clean(o: &[Row], test: &str, seen: &str) {
    let r = row(o, test);
    assert_eq!(r.2, Outcome::Fail(seen.into()), "{test}: {o:?}");
    assert!(r.3.is_empty(), "{test}: {r:?}");
}

// A failed test with no table access keeps the sequences: C13 (SqCommitFail.al:541, SqReader.al:138), C3 (:100, :68),
// D6 (:1225, :201), D1 (:758, :166), and an old sequence it restarted, B3 (:1051, :229).
const NO_ACCESS: &str = r#"
codeunit 50741 "P4 C13"
{
    Subtype = Test;
    [Test] procedure A_InsertsFails() begin NumberSequence.Insert('P4C13', 1, 1); NumberSequence.Next('P4C13'); Error('A deliberate'); end;
    [Test] procedure B_C13Reads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4C13')); end;
}
codeunit 50742 "P4 C3"
{
    Subtype = Test;
    [Test] procedure A_CommitsInsertsFails() begin Commit(); NumberSequence.Insert('P4C3', 1, 1); NumberSequence.Next('P4C3'); Error('A deliberate'); end;
    [Test] procedure B_C3Reads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4C3')); end;
}
codeunit 50743 "P4 D6"
{
    Subtype = Test;
    [Test] procedure A_InsertsCommits() begin NumberSequence.Insert('P4D6', 1, 1); NumberSequence.Next('P4D6'); Commit(); end;
    [Test] procedure B_Fails() begin Error('B deliberate'); end;
    [Test] procedure C_D6Reads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4D6')); end;
}
codeunit 50744 "P4 D1"
{
    Subtype = Test;
    var Rows: Codeunit "Rows";
    [Test] procedure A_WritesRow() begin Rows.Put('D1-ROW'); end;
    [Test] procedure B_Inserts() begin NumberSequence.Insert('P4D1', 1, 1); NumberSequence.Next('P4D1'); end;
    [Test] procedure C_Fails() begin Error('C deliberate'); end;
    [Test] procedure D_D1Reads() var R: Codeunit "P4 Read"; begin Error('OBS %1 row=%2', R.Of('P4D1'), Rows.Has('D1-ROW')); end;
}
codeunit 50745 "P4 B3"
{
    Subtype = Test;
    trigger OnRun() begin if not NumberSequence.Exists('P4B3') then NumberSequence.Insert('P4B3', 1, 1); end;
    [Test] procedure A_RestartsFails() begin NumberSequence.Restart('P4B3', 500); Error('A deliberate'); end;
    [Test] procedure B_B3Reads() begin Error('OBS next=%1', NumberSequence.Next('P4B3')); end;
}
"#;

#[test]
fn a_failed_test_without_table_access_keeps_the_sequences() {
    let o = cells(NO_ACCESS, TestIsolation::Codeunit);
    flagged(&o, "B_C13Reads", "OBS cur=1 next=2", SEQ);
    flagged(&o, "B_C3Reads", "OBS cur=1 next=2", SEQ);
    flagged(&o, "C_D6Reads", "OBS cur=1 next=2", SEQ);
    flagged(&o, "D_D1Reads", "OBS cur=1 next=2 row=Yes", SEQ);
    flagged(&o, "B_B3Reads", "OBS next=500", SEQ);
}

// A failed test that wrote a row and did not commit takes back a sequence inserted since the codeunit began, also after a
// Commit() of an earlier test: C2 (SqCommitFail.al:64, SqReader.al:61), C14 (:570, :145). Without any Commit() in the
// codeunit that is main's result, pinned for a Get (IsolProbeAfterRollback.Codeunit.al:72-79 S4, :117-123 S8): D7 (:1260,
// :208) passes through no assumption.
const ROW_NO_COMMIT: &str = r#"
codeunit 50746 "P4 C2"
{
    Subtype = Test;
    var Rows: Codeunit "Rows";
    [Test] procedure A_InsertsCommits() begin NumberSequence.Insert('P4C2', 1, 1); NumberSequence.Next('P4C2'); Commit(); end;
    [Test] procedure B_WritesFails() begin Rows.Put('C2-FAILED'); Error('B deliberate'); end;
    [Test] procedure C_C2Reads() var R: Codeunit "P4 Read"; begin Error('OBS %1 row=%2', R.Of('P4C2'), Rows.Has('C2-FAILED')); end;
}
codeunit 50747 "P4 C14"
{
    Subtype = Test;
    var Rows: Codeunit "Rows"; IsInitialized: Boolean;
    [Test] procedure A_Initializes() begin Initialize(); end;
    [Test] procedure B_TakesNumber() begin Initialize(); if GetNextNo() <> 1 then Error('B first number'); end;
    [Test] procedure C_WritesFails() begin Initialize(); Rows.Put('C14-FAILED'); Error('C deliberate'); end;
    [Test] procedure D_C14Reads() var R: Codeunit "P4 Read"; begin Initialize(); Error('OBS %1', R.Of('P4C14')); end;
    local procedure Initialize() begin if IsInitialized then exit; Rows.Put('C14-INIT'); IsInitialized := true; Commit(); end;
    local procedure GetNextNo(): BigInteger
    begin
        if not NumberSequence.Exists('P4C14') then begin
            NumberSequence.Insert('P4C14', 0, 1);
            if NumberSequence.Next('P4C14') = 0 then;
        end;
        exit(NumberSequence.Next('P4C14'));
    end;
}
codeunit 50748 "P4 D7"
{
    Subtype = Test;
    var Rows: Codeunit "Rows";
    [Test] procedure A_Inserts() begin NumberSequence.Insert('P4D7', 1, 1); NumberSequence.Next('P4D7'); end;
    [Test] procedure B_WritesFails() begin Rows.Put('D7-FAILED'); Error('B deliberate'); end;
    [Test] procedure C_D7Reads() var R: Codeunit "P4 Read"; begin Error('OBS %1 row=%2', R.Of('P4D7'), Rows.Has('D7-FAILED')); end;
}
"#;

#[test]
fn a_failed_test_that_wrote_without_a_commit_takes_the_new_sequences_back() {
    let o = cells(ROW_NO_COMMIT, TestIsolation::Codeunit);
    flagged(&o, "C_C2Reads", "OBS missing row=No", SEQ);
    flagged(&o, "D_C14Reads", "OBS missing", SEQ);
    clean(&o, "C_D7Reads", "OBS missing row=No");
}

// A failed test that inserted a sequence, wrote and then committed keeps it: C1 (SqCommitFail.al:31, SqReader.al:54), a
// Commit() in a MessageHandler C5 (:248, :82), a guarded Codeunit.Run C6 (:289, :89).
const ROW_COMMIT: &str = r#"
codeunit 50749 "P4 C1"
{
    Subtype = Test;
    var Rows: Codeunit "Rows";
    [Test] procedure A_InsertsCommitsFails() begin NumberSequence.Insert('P4C1', 1, 1); NumberSequence.Next('P4C1'); Rows.Put('C1-BEFORE'); Commit(); Rows.Put('C1-AFTER'); Error('A deliberate'); end;
    [Test] procedure B_C1Reads() var R: Codeunit "P4 Read"; begin Error('OBS %1 rows=%2 %3', R.Of('P4C1'), Rows.Has('C1-BEFORE'), Rows.Has('C1-AFTER')); end;
}
codeunit 50750 "P4 C5"
{
    Subtype = Test;
    var Rows: Codeunit "Rows";
    [Test] [HandlerFunctions('MessageCommits')] procedure A_InsertsMessageFails() begin NumberSequence.Insert('P4C5', 1, 1); NumberSequence.Next('P4C5'); Rows.Put('C5-BEFORE'); Message('C5'); Error('A deliberate'); end;
    [Test] procedure B_C5Reads() var R: Codeunit "P4 Read"; begin Error('OBS %1 rows=%2 %3', R.Of('P4C5'), Rows.Has('C5-BEFORE'), Rows.Has('C5-HANDLER')); end;
    [MessageHandler] procedure MessageCommits(Msg: Text[1024]) begin Rows.Put('C5-HANDLER'); Commit(); end;
}
codeunit 50751 "P4 C6 Run"
{
    trigger OnRun() var Rows: Codeunit "Rows"; begin Rows.Put('C6-RUN'); end;
}
codeunit 50752 "P4 C6"
{
    Subtype = Test;
    var Rows: Codeunit "Rows";
    [Test] procedure A_InsertsRunsFails() begin NumberSequence.Insert('P4C6', 1, 1); NumberSequence.Next('P4C6'); if not Codeunit.Run(Codeunit::"P4 C6 Run") then Error('C6 run'); Error('A deliberate'); end;
    [Test] procedure B_C6Reads() var R: Codeunit "P4 Read"; begin Error('OBS %1 row=%2', R.Of('P4C6'), Rows.Has('C6-RUN')); end;
}
"#;

#[test]
fn a_failed_test_that_inserted_wrote_and_committed_keeps_the_sequence() {
    let o = cells(ROW_COMMIT, TestIsolation::Codeunit);
    flagged(&o, "B_C1Reads", "OBS cur=1 next=2 rows=Yes No", SEQ);
    flagged(&o, "B_C5Reads", "OBS cur=1 next=2 rows=Yes Yes", SEQ);
    flagged(&o, "B_C6Reads", "OBS cur=1 next=2 row=Yes", SEQ);
}

// Shapes PR 1465 does not measure keep main's result: a failed test that reached a table through a RecordRef after a
// Commit() in the codeunit, and one that inserted a second sequence after its last Commit(), are refused for the readers of
// the new sequences; an insert in a test with TransactionModel AutoRollback is taken back by a later failure.
const NEIGHBORS: &str = r#"
codeunit 50753 "P4 RecordRef"
{
    Subtype = Test;
    [Test] procedure A_InsertsCommits() begin NumberSequence.Insert('P4RR', 1, 1); NumberSequence.Next('P4RR'); Commit(); end;
    [Test] procedure B_RecordRefFails() var RR: RecordRef; begin RR.Open(50700); RR.Init(); RR.Field(1).Value := 'RR'; RR.Insert(); Error('B deliberate'); end;
    [Test] procedure C_RecordRefReads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4RR')); end;
}
codeunit 50754 "P4 Second Insert"
{
    Subtype = Test;
    var Rows: Codeunit "Rows";
    [Test] procedure A_InsertsCommitsInsertsFails() begin NumberSequence.Insert('P4S1', 1, 1); Rows.Put('S-BEFORE'); Commit(); NumberSequence.Insert('P4S2', 1, 1); Error('A deliberate'); end;
    [Test] procedure B_FirstReads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4S1')); end;
}
codeunit 50756 "P4 IsolatedStorage"
{
    Subtype = Test;
    [Test] procedure A_InsertsStoresFails() begin NumberSequence.Insert('P4IS', 1, 1); IsolatedStorage.Set('P4IS', 'x'); Error('A deliberate'); end;
    [Test] procedure B_IsolatedStorageReads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4IS')); end;
}
codeunit 50757 "P4 Insert In Failed"
{
    Subtype = Test;
    var Rows: Codeunit "Rows";
    [Test] procedure A_Commits() begin Rows.Put('P4IF-A'); Commit(); end;
    [Test] procedure B_InsertsWritesFails() begin NumberSequence.Insert('P4IF', 1, 1); Rows.Put('P4IF-B'); Error('B deliberate'); end;
    [Test] procedure C_InsertedInFailedReads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4IF')); end;
}
codeunit 50755 "P4 AutoRollback"
{
    Subtype = Test;
    [Test] [TransactionModel(TransactionModel::AutoRollback)] procedure A_InsertsAutoRollback() begin NumberSequence.Insert('P4AR', 1, 1); NumberSequence.Next('P4AR'); end;
    [Test] procedure B_Fails() begin Error('B deliberate'); end;
    [Test] procedure C_AutoRollbackReads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4AR')); end;
}
"#;

#[test]
fn shapes_the_probe_does_not_measure_keep_mains_result() {
    let o = cells(NEIGHBORS, TestIsolation::Codeunit);
    let plain: Vec<_> = o.iter().map(|r| (r.0, r.1.clone(), r.2.clone())).collect();
    let of = |t: &str| super::runner_isolation::outcome(&plain, t).clone();
    assert!(not_run(&of("C_RecordRefReads"), "'P4RR'"), "{o:?}");
    assert!(not_run(&of("B_FirstReads"), "'P4S1'"), "{o:?}");
    clean(&o, "C_AutoRollbackReads", "OBS missing");
    // IsolatedStorage is not measured in the failed test (only in an earlier one, D4 SqCommitFail.al:878).
    clean(&o, "B_IsolatedStorageReads", "OBS missing");
    // C2 and C14 measured a sequence an earlier test inserted; one the failed test inserted after an earlier Commit() is not.
    assert!(not_run(&of("C_InsertedInFailedReads"), "'P4IF'"), "{o:?}");
}

// Under a runner with TestIsolation = Function every new sequence is gone after each test (IsolProbeAfterRollback.Codeunit.al
// S3, S8 under runner 61300): the cells of PR 1465 do not apply.
#[test]
fn under_a_function_runner_a_failed_test_without_table_access_keeps_mains_result() {
    let o = cells(NO_ACCESS, TestIsolation::Function);
    clean(&o, "B_C13Reads", "OBS missing");
}

// al-runner issue 8337, the reporter's repro unchanged: a failed test leaves the binding of a codeunit global for the later
// tests; a second BindSubscription raises "already been bound". PR 1465 P5a (SqBindFailed.al:36, SqReader.al:257).
const REPRO_8337: &str = r#"
codeunit 91506 "P161 Events"
{
    [IntegrationEvent(false, false)]
    procedure OnPing(var Hits: Integer)
    begin
    end;

    procedure Ping(): Integer
    var
        Hits: Integer;
    begin
        OnPing(Hits);
        exit(Hits);
    end;
}

codeunit 91507 "P161 Manual Sub"
{
    EventSubscriberInstance = Manual;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"P161 Events", 'OnPing', '', false, false)]
    local procedure HandlePing(var Hits: Integer)
    begin
        Hits += 1;
    end;
}

codeunit 91558 "P161 Binding"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Sub: Codeunit "P161 Manual Sub";

    [Test]
    procedure BindInFirst()
    var
        Events: Codeunit "P161 Events";
    begin
        BindSubscription(Sub);
        Error('MEASURED hits=%1', Events.Ping());
    end;

    [Test]
    procedure StillBoundInSecond()
    var
        Events: Codeunit "P161 Events";
    begin
        Error('MEASURED hits=%1', Events.Ping());
    end;

    [Test]
    procedure BindTwice()
    var
        Events: Codeunit "P161 Events";
        Sub2: Codeunit "P161 Manual Sub";
    begin
        BindSubscription(Sub2);
        asserterror BindSubscription(Sub2);
        Error('MEASURED hits=%1 err=%2', Events.Ping(), GetLastErrorText());
    end;
}
"#;

fn repro_8337(isolation: TestIsolation) -> Vec<Row> {
    let opts = RunOptions {
        isolation: Some(isolation),
        ..Default::default()
    };
    let rt = CorpusRuntime::run_suites_with(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("main.al".into(), REPRO_8337.into())],
        }],
        vec![al_sema::AppKind::Workspace],
        &[],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &opts,
    );
    rt.results()
        .iter()
        .map(|r| {
            (
                r.codeunit,
                r.test.clone(),
                r.outcome.clone(),
                r.assumptions.iter().map(|a| a.info().id).collect(),
            )
        })
        .collect()
}

#[test]
fn issue_8337_a_failed_test_leaves_the_binding_of_a_codeunit_global() {
    let o = repro_8337(TestIsolation::Codeunit);
    clean(&o, "BindInFirst", "MEASURED hits=1");
    flagged(&o, "StillBoundInSecond", "MEASURED hits=1", BIND);
    flagged(
        &o,
        "BindTwice",
        "MEASURED hits=2 err=The binding of codeunit 91507 was unsuccessful. The codeunit has already been bound.",
        BIND,
    );
}

// P5b (SqBindFailed.al:85, SqReader.al:264): a local binding ends with its failed test. P5c (:114, :271): the next test
// unbinds the global.
const BIND_NEIGHBORS: &str = r#"
codeunit 50760 "P5 Events"
{
    [IntegrationEvent(false, false)] procedure OnPing(var Hits: Integer) begin end;
    procedure Ping(): Integer var Hits: Integer; begin OnPing(Hits); exit(Hits); end;
}
codeunit 50761 "P5 Sub"
{
    EventSubscriberInstance = Manual;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"P5 Events", 'OnPing', '', false, false)] local procedure HandlePing(var Hits: Integer) begin Hits += 1; end;
}
codeunit 50762 "P5 Local"
{
    Subtype = Test;
    [Test] procedure A_BindsLocalFails() var Sub: Codeunit "P5 Sub"; Events: Codeunit "P5 Events"; begin BindSubscription(Sub); Error('A hits=%1', Events.Ping()); end;
    [Test] procedure B_LocalReads() var Events: Codeunit "P5 Events"; begin Error('B hits=%1', Events.Ping()); end;
}
codeunit 50763 "P5 Unbind"
{
    Subtype = Test;
    var Sub: Codeunit "P5 Sub";
    [Test] procedure A_BindsFails() var Events: Codeunit "P5 Events"; begin BindSubscription(Sub); Error('A hits=%1', Events.Ping()); end;
    [Test] procedure B_Unbinds() var Events: Codeunit "P5 Events"; begin Error('B unbind=%1 hits=%2', UnbindSubscription(Sub), Events.Ping()); end;
}
"#;

#[test]
fn a_local_binding_ends_with_its_failed_test_and_the_next_test_can_unbind_a_global() {
    let o = cells(BIND_NEIGHBORS, TestIsolation::Codeunit);
    clean(&o, "B_LocalReads", "B hits=0");
    flagged(&o, "B_Unbinds", "B unbind=Yes hits=0", BIND);
}

// Under TestIsolation = Function the binding PR 1465 observed is not asserted: a later failure stays not trusted.
#[test]
fn under_a_function_runner_a_failure_after_a_left_binding_is_still_not_trusted() {
    let o = repro_8337(TestIsolation::Function);
    let r = row(&o, "StillBoundInSecond");
    assert!(not_run(&r.2, "left a manual event subscription bound"), "{o:?}");
}
