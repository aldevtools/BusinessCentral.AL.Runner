//! What a failed test leaves for the later tests of its codeunit under al-runner's own runner with TestIsolation =
//! Codeunit: the number sequences and the manual bindings. The cells copy corpus PR 1465 (merged, not yet pinned, "NumberSequence and
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
// the new sequences; an insert in a test with TransactionModel AutoRollback is taken back by a later failure. A failed test
// that called IsolatedStorage without any Commit() is refused too (main's take-back covers no table access and Record
// reads and writes only).
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
    assert!(not_run(&of("B_IsolatedStorageReads"), "'P4IS'"), "{o:?}");
    // Inserted in the failed test after the last transaction end and then a Record write: gone, as in S2E (corpus PR 1572
    // SqFailedSeq.al:2031), where that end is the failed test's own Commit().
    flagged(&o, "C_InsertedInFailedReads", "OBS missing", SEQ);
}

// A failed test whose table access is not a Record write after an earlier Commit() keeps main's refusal: a Record read
// only (no cell of PR 1465 reads after a Commit(); D5 SqCommitFail.al:918 reads without one), a TestPage, a native that
// reads the database. The same for a failed test that inserted, only read and then committed (C1 :31 writes).
const NOT_A_RECORD_WRITE: &str = r#"
page 50790 "P4 Rows"
{
    SourceTable = "Row";
    layout { area(Content) { field(K; Rec.K) { } } }
}
codeunit 50764 "P4 Get After Commit"
{
    Subtype = Test;
    var Rows: Codeunit "Rows";
    [Test] procedure A_InsertsCommits() begin NumberSequence.Insert('P4GA', 1, 1); NumberSequence.Next('P4GA'); Commit(); end;
    [Test] procedure B_GetsFails() begin if Rows.Has('NOPE') then; Error('B deliberate'); end;
    [Test] procedure C_GetReads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4GA')); end;
}
codeunit 50765 "P4 TestPage After Commit"
{
    Subtype = Test;
    [Test] procedure A_InsertsCommits() begin NumberSequence.Insert('P4TP', 1, 1); NumberSequence.Next('P4TP'); Commit(); end;
    [Test] procedure B_PageFails() var P: TestPage "P4 Rows"; begin P.OpenView(); P.Close(); Error('B deliberate'); end;
    [Test] procedure C_PageReads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4TP')); end;
}
codeunit 50766 "P4 Native After Commit"
{
    Subtype = Test;
    [Test] procedure A_InsertsCommits() begin NumberSequence.Insert('P4NA', 1, 1); NumberSequence.Next('P4NA'); Commit(); end;
    [Test] procedure B_NativeFails() begin if CompanyProperty.DisplayName() = '' then; Error('B deliberate'); end;
    [Test] procedure C_NativeReads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4NA')); end;
}
codeunit 50767 "P4 Get Then Commit"
{
    Subtype = Test;
    var Rows: Codeunit "Rows";
    [Test] procedure A_InsertsGetsCommitsFails() begin NumberSequence.Insert('P4GC', 1, 1); NumberSequence.Next('P4GC'); if Rows.Has('NOPE') then; Commit(); Error('A deliberate'); end;
    [Test] procedure B_GetThenCommitReads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4GC')); end;
}
"#;

#[test]
fn a_failed_test_without_a_record_write_after_a_commit_keeps_mains_refusal() {
    let o = cells(NOT_A_RECORD_WRITE, TestIsolation::Codeunit);
    for (test, key) in [
        ("C_GetReads", "'P4GA'"),
        ("C_PageReads", "'P4TP'"),
        ("C_NativeReads", "'P4NA'"),
        ("B_GetThenCommitReads", "'P4GC'"),
    ] {
        assert!(not_run(&row(&o, test).2, key), "{test}: {o:?}");
    }
}

// Every cell of PR 1465 fails by an error raised in the body of a TransactionModel AutoCommit test. A test that fails only
// because a handler it declares did not run keeps main's result: its new sequence is gone, and a binding it left makes a
// later failure not trusted. So does a binding left by a failed AutoRollback test.
const NOT_RAISED: &str = r#"
codeunit 50768 "P4 Unused Handler"
{
    Subtype = Test;
    [Test] [HandlerFunctions('Msg')] procedure A_InsertsHandlerUnused() begin NumberSequence.Insert('P4UH', 1, 1); NumberSequence.Next('P4UH'); end;
    [Test] procedure B_UnusedHandlerReads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4UH')); end;
    [MessageHandler] procedure Msg(Message: Text[1024]) begin end;
}
codeunit 50769 "P5 Events 2"
{
    [IntegrationEvent(false, false)] procedure OnPing(var Hits: Integer) begin end;
    procedure Ping(): Integer var Hits: Integer; begin OnPing(Hits); exit(Hits); end;
}
codeunit 50770 "P5 Sub 2"
{
    EventSubscriberInstance = Manual;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"P5 Events 2", 'OnPing', '', false, false)] local procedure HandlePing(var Hits: Integer) begin Hits += 1; end;
}
codeunit 50771 "P5 Unused Handler Bind"
{
    Subtype = Test;
    var Sub: Codeunit "P5 Sub 2";
    [Test] [HandlerFunctions('Msg')] procedure A_BindsHandlerUnused() begin BindSubscription(Sub); end;
    [Test] procedure B_HandlerBindReads() var Events: Codeunit "P5 Events 2"; begin Error('B hits=%1', Events.Ping()); end;
    [MessageHandler] procedure Msg(Message: Text[1024]) begin end;
}
codeunit 50772 "P5 AutoRollback Bind"
{
    Subtype = Test;
    var Sub: Codeunit "P5 Sub 2";
    [Test] [TransactionModel(TransactionModel::AutoRollback)] procedure A_BindsFails() begin BindSubscription(Sub); Error('A deliberate'); end;
    [Test] procedure B_AutoRollbackBindReads() var Events: Codeunit "P5 Events 2"; begin Error('B hits=%1', Events.Ping()); end;
}
"#;

#[test]
fn a_failure_not_raised_by_the_body_of_an_autocommit_test_keeps_mains_result() {
    let o = cells(NOT_RAISED, TestIsolation::Codeunit);
    clean(&o, "B_UnusedHandlerReads", "OBS missing");
    for test in ["B_HandlerBindReads", "B_AutoRollbackBindReads"] {
        assert!(
            not_run(&row(&o, test).2, "left a manual event subscription bound"),
            "{test}: {o:?}"
        );
    }
}

// PR 1465 measured an old sequence changed by the failed test itself (B1-B4, SqCommitFail.al:963-1135) and a kept sequence
// read by the next test, not a second failure. A sequence an earlier passing test restarted, under AutoRollback or
// AutoCommit, and one an earlier failed test kept, keep main's refusal.
const NOT_THE_FAILED_BODY: &str = r#"
codeunit 50773 "P4 Earlier Rollback Restart"
{
    Subtype = Test;
    trigger OnRun() begin if not NumberSequence.Exists('P4ER') then NumberSequence.Insert('P4ER', 1, 1); end;
    [Test] [TransactionModel(TransactionModel::AutoRollback)] procedure A_Restarts() begin NumberSequence.Restart('P4ER', 500); end;
    [Test] procedure B_Fails() begin Error('B deliberate'); end;
    [Test] procedure C_RollbackRestartReads() begin Error('OBS next=%1', NumberSequence.Next('P4ER')); end;
}
codeunit 50774 "P4 Earlier Restart"
{
    Subtype = Test;
    trigger OnRun() begin if not NumberSequence.Exists('P4EC') then NumberSequence.Insert('P4EC', 1, 1); end;
    [Test] procedure A_Restarts() begin NumberSequence.Restart('P4EC', 500); end;
    [Test] procedure B_Fails() begin Error('B deliberate'); end;
    [Test] procedure C_EarlierRestartReads() begin Error('OBS next=%1', NumberSequence.Next('P4EC')); end;
}
codeunit 50775 "P4 Two Fails"
{
    Subtype = Test;
    var Rows: Codeunit "Rows";
    [Test] procedure A_Commits() begin Rows.Put('TF-A'); Commit(); end;
    [Test] procedure B_InsertsFails() begin NumberSequence.Insert('P4TF', 1, 1); NumberSequence.Next('P4TF'); Error('B deliberate'); end;
    [Test] procedure C_WritesFails() begin Rows.Put('TF-C'); Error('C deliberate'); end;
    [Test] procedure D_TwoFailsReads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4TF')); end;
}
codeunit 50776 "P4 Two Bare Fails"
{
    Subtype = Test;
    [Test] procedure A_InsertsFails() begin NumberSequence.Insert('P4TB', 1, 1); NumberSequence.Next('P4TB'); Error('A deliberate'); end;
    [Test] procedure B_Fails() begin Error('B deliberate'); end;
    [Test] procedure C_TwoBareFailsReads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4TB')); end;
}
"#;

#[test]
fn a_sequence_the_failed_body_did_not_change_keeps_mains_refusal() {
    let o = cells(NOT_THE_FAILED_BODY, TestIsolation::Codeunit);
    for (test, key) in [
        ("C_RollbackRestartReads", "'P4ER'"),
        ("C_EarlierRestartReads", "'P4EC'"),
        ("D_TwoFailsReads", "'P4TF'"),
        ("C_TwoBareFailsReads", "'P4TB'"),
    ] {
        assert!(not_run(&row(&o, test).2, key), "{test}: {o:?}");
    }
}

// Without any Commit() main's rule drops a new sequence only after a failed test that reached no table, or reached tables
// through Record reads and writes (pinned for a Get, IsolProbeAfterRollback.Codeunit.al:72-79 S4, :117-123 S8; D5 and D7
// SqCommitFail.al:918, :1260). Any other access of the failed test is not measured: a temporary record, Init, a RecordRef,
// IsolatedStorage, a native that reads the database. Its readers are refused.
const OTHER_ACCESS_NO_COMMIT: &str = r#"
codeunit 50777 "P4 Temp"
{
    Subtype = Test;
    [Test] procedure A_InsertsTempFails() var T: Record "Row" temporary; begin NumberSequence.Insert('P4TMP', 1, 1); T.K := 'X'; T.Insert(); Error('A deliberate'); end;
    [Test] procedure B_TempReads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4TMP')); end;
}
codeunit 50778 "P4 Init"
{
    Subtype = Test;
    [Test] procedure A_InsertsInitFails() var T: Record "Row"; begin NumberSequence.Insert('P4INI', 1, 1); T.Init(); Error('A deliberate'); end;
    [Test] procedure B_InitReads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4INI')); end;
}
codeunit 50779 "P4 RecordRef No Commit"
{
    Subtype = Test;
    [Test] procedure A_InsertsRecordRefFails() var RR: RecordRef; begin NumberSequence.Insert('P4RRN', 1, 1); RR.Open(50700); if RR.IsEmpty() then; RR.Close(); Error('A deliberate'); end;
    [Test] procedure B_RecordRefNoCommitReads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4RRN')); end;
}
codeunit 50780 "P4 Native No Commit"
{
    Subtype = Test;
    [Test] procedure A_InsertsNativeFails() begin NumberSequence.Insert('P4NN', 1, 1); if CompanyProperty.DisplayName() = '' then; Error('A deliberate'); end;
    [Test] procedure B_NativeNoCommitReads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4NN')); end;
}
codeunit 50781 "P4 Get No Commit"
{
    Subtype = Test;
    var Rows: Codeunit "Rows";
    [Test] procedure A_InsertsGetsFails() begin NumberSequence.Insert('P4GN', 1, 1); if Rows.Has('NOPE') then; Error('A deliberate'); end;
    [Test] procedure B_GetNoCommitReads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4GN')); end;
}
"#;

#[test]
fn without_a_commit_only_a_record_read_or_write_drops_the_new_sequence() {
    let o = cells(OTHER_ACCESS_NO_COMMIT, TestIsolation::Codeunit);
    for (test, key) in [
        ("B_TempReads", "'P4TMP'"),
        ("B_InitReads", "'P4INI'"),
        ("B_RecordRefNoCommitReads", "'P4RRN'"),
        ("B_NativeNoCommitReads", "'P4NN'"),
    ] {
        assert!(not_run(&row(&o, test).2, key), "{test}: {o:?}");
    }
    clean(&o, "B_GetNoCommitReads", "OBS missing");
}

// A Record write whose table runs a subscriber of Microsoft's app that calls a session builtin (as the first insert into a
// table runs the System Application's global triggers, reviewer's neighbor K09 of PR 8512): the subscriber's calls count
// with the Record call, and main's rule drops the sequence. The same calls from a subscriber of the app under test are
// other access, refused.
const MS_TRIGGERS: &str = r#"
table 50796 "P4 MS Row" { fields { field(1; K; Code[20]) { } } keys { key(PK; K) { } } }
table 50797 "P4 MS Row Two" { fields { field(1; K; Code[20]) { } } keys { key(PK; K) { } } }
codeunit 50798 "P4 MS Subscriber"
{
    [EventSubscriber(ObjectType::Table, Database::"P4 MS Row", 'OnAfterInsertEvent', '', false, false)]
    local procedure Stamp(var Rec: Record "P4 MS Row")
    begin
        if CurrentDateTime() = 0DT then;
        if CompanyName() = '' then;
    end;
}
"#;

const OWN_TRIGGERS: &str = r#"
codeunit 50799 "P4 Own Subscriber"
{
    [EventSubscriber(ObjectType::Table, Database::"P4 MS Row Two", 'OnAfterInsertEvent', '', false, false)]
    local procedure Stamp(var Rec: Record "P4 MS Row Two")
    begin
        if CurrentDateTime() = 0DT then;
    end;
}
codeunit 50782 "P4 MS Trigger"
{
    Subtype = Test;
    [Test] procedure A_Inserts() begin NumberSequence.Insert('P4MST', 1, 1); end;
    [Test] procedure B_WritesFails() var T: Record "P4 MS Row"; begin T.K := 'X'; T.Insert(); Error('B deliberate'); end;
    [Test] procedure C_MsTriggerReads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4MST')); end;
}
codeunit 50783 "P4 Own Trigger"
{
    Subtype = Test;
    [Test] procedure A_Inserts() begin NumberSequence.Insert('P4OWT', 1, 1); end;
    [Test] procedure B_WritesFails() var T: Record "P4 MS Row Two"; begin T.K := 'X'; T.Insert(); Error('B deliberate'); end;
    [Test] procedure C_OwnTriggerReads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4OWT')); end;
}
"#;

#[test]
fn session_builtins_of_microsofts_subscribers_count_with_the_record_call() {
    let manifest = |p: &str| al_testrun::runtime::Manifest {
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
        package_data: false,
        package_data_files: Vec::new(),
        translation_files: Vec::new(),
        translation_source: None,
        runtime: None,
        app_json_keys: Vec::new(),
        platform_internals_visible_to: Vec::new(),
        symbols: vec![],
    };
    let app = |id: &str, deps: &[&str], src: String| AppSource {
        id: id.into(),
        name: id.into(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![(format!("{id}.al"), src)],
    };
    let rt = CorpusRuntime::run_suites_with(
        vec![
            app("main", &["ms"], format!("{READ}\n{OWN_TRIGGERS}")),
            app("ms", &[], MS_TRIGGERS.into()),
        ],
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        &[manifest("Me"), manifest("Microsoft")],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions {
            isolation: Some(TestIsolation::Codeunit),
            ..Default::default()
        },
    );
    let o: Vec<Row> = rt
        .results()
        .iter()
        .map(|r| {
            (
                r.codeunit,
                r.test.clone(),
                r.outcome.clone(),
                r.assumptions.iter().map(|a| a.info().id).collect(),
            )
        })
        .collect();
    clean(&o, "C_MsTriggerReads", "OBS missing");
    assert!(not_run(&row(&o, "C_OwnTriggerReads").2, "'P4OWT'"), "{o:?}");
}

// RecordRef access in the failed test counts as table access, as Record access does: corpus PR 1468 (draft,
// tests/al-language-isolation-probe/src/SqRecordRefFail.al, assertions in SqRecordRefReader.al), measured on BC 27.5, 28.5
// and 29.0 under runner 61301 and on 28.5 under 130450. C2 shape (insert and Commit() in an earlier test) and C14 shape (an
// earlier test's Commit(), then a test that inserts): a RecordRef read, write, the Library - Utility.GetNewRecNo copy and a
// Record passed to a RecordRef parameter take the sequence away; RecordRef.Open and Close alone keep it.
const REF_LINE: &str = r#"
table 50856 "P4 Line" { fields { field(1; Doc; Code[20]) { } field(2; "Line No."; Integer) { } } keys { key(PK; Doc, "Line No.") { Clustered = true; } } }
codeunit 50857 "P4 Ref Access"
{
    procedure GetNewRecNo(RecVariant: Variant; FieldNo: Integer): Integer
    var
        RecRef: RecordRef;
    begin
        RecRef.GetTable(RecVariant);
        exit(GetNewLineNo(RecRef, FieldNo));
    end;

    procedure GetNewLineNo(RecRef: RecordRef; FieldNo: Integer): Integer
    var
        RecRef2: RecordRef;
        FieldRef: FieldRef;
        FieldRef2: FieldRef;
        KeyRef: KeyRef;
        FieldCount: Integer;
    begin
        RecRef2.Open(RecRef.Number, false, CompanyName);
        KeyRef := RecRef.KeyIndex(1);
        for FieldCount := 1 to KeyRef.FieldCount do begin
            FieldRef := KeyRef.FieldIndex(FieldCount);
            if FieldRef.Number <> FieldNo then begin
                FieldRef2 := RecRef2.Field(FieldRef.Number);
                FieldRef2.SetRange(FieldRef.Value);
            end;
        end;
        if RecRef2.FindLast() then begin
            FieldRef := RecRef2.Field(FieldNo);
            FieldCount := FieldRef.Value();
        end else
            FieldCount := 0;
        exit(FieldCount + 10000);
    end;

    procedure MockInsert(Doc: Text)
    var
        Line: Record "P4 Line";
    begin
        Line.Init();
        Line.Doc := CopyStr(Doc, 1, 20);
        Line."Line No." := GetNewRecNo(Line, Line.FieldNo("Line No."));
        Line.Insert();
    end;

    procedure LastByRef(RecRef: RecordRef)
    begin
        if RecRef.FindLast() then;
    end;
}
"#;

const REF_ACCESS: &str = r#"
codeunit 50850 "P4 R2O"
{
    Subtype = Test;
    [Test] procedure A_InsertsCommits() begin NumberSequence.Insert('P4R2O', 1, 1); NumberSequence.Next('P4R2O'); Commit(); end;
    [Test] procedure B_RefReadFails() var RR: RecordRef; FR: FieldRef; begin RR.Open(Database::"P4 Line"); FR := RR.Field(1); FR.SetRange('X'); if RR.FindLast() then; Error('B deliberate'); end;
    [Test] procedure C_R2OReads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4R2O')); end;
}
codeunit 50851 "P4 R14M"
{
    Subtype = Test;
    var Rows: Codeunit "Rows"; Ref: Codeunit "P4 Ref Access";
    [Test] procedure A_Commits() begin Rows.Put('R14M'); Commit(); end;
    [Test] procedure B_Inserts() begin NumberSequence.Insert('P4R14M', 1, 1); NumberSequence.Next('P4R14M'); end;
    [Test] procedure C_MockFails() begin Ref.MockInsert('R14M'); Error('C deliberate'); end;
    [Test] procedure D_R14MReads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4R14M')); end;
}
codeunit 50852 "P4 R14W"
{
    Subtype = Test;
    var Rows: Codeunit "Rows";
    [Test] procedure A_Commits() begin Rows.Put('R14W'); Commit(); end;
    [Test] procedure B_Inserts() begin NumberSequence.Insert('P4R14W', 1, 1); NumberSequence.Next('P4R14W'); end;
    [Test] procedure C_RefInsertFails() var RR: RecordRef; FR: FieldRef; begin RR.Open(Database::"P4 Line"); FR := RR.Field(1); FR.Value := 'W'; FR := RR.Field(2); FR.Value := 10000; RR.Insert(); Error('C deliberate'); end;
    [Test] procedure D_R14WReads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4R14W')); end;
}
codeunit 50853 "P4 R14N"
{
    Subtype = Test;
    var Rows: Codeunit "Rows";
    [Test] procedure A_Commits() begin Rows.Put('R14N'); Commit(); end;
    [Test] procedure B_Inserts() begin NumberSequence.Insert('P4R14N', 1, 1); NumberSequence.Next('P4R14N'); end;
    [Test] procedure C_OpenOnlyFails() var RR: RecordRef; begin RR.Open(Database::"P4 Line"); RR.Close(); Error('C deliberate'); end;
    [Test] procedure D_R14NReads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4R14N')); end;
}
codeunit 50854 "P4 R2N"
{
    Subtype = Test;
    [Test] procedure A_InsertsCommits() begin NumberSequence.Insert('P4R2N', 1, 1); NumberSequence.Next('P4R2N'); Commit(); end;
    [Test] procedure B_OpenOnlyFails() var RR: RecordRef; begin RR.Open(Database::"P4 Line"); RR.Close(); Error('B deliberate'); end;
    [Test] procedure C_R2NReads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4R2N')); end;
}
codeunit 50855 "P4 R2P"
{
    Subtype = Test;
    var Ref: Codeunit "P4 Ref Access";
    [Test] procedure A_InsertsCommits() begin NumberSequence.Insert('P4R2P', 1, 1); NumberSequence.Next('P4R2P'); Commit(); end;
    [Test] procedure B_RecordAsRefFails() var Line: Record "P4 Line"; begin Ref.LastByRef(Line); Error('B deliberate'); end;
    [Test] procedure C_R2PReads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4R2P')); end;
}
"#;

#[test]
fn recordref_access_in_a_failed_test_is_table_access() {
    let o = cells(&format!("{REF_LINE}\n{REF_ACCESS}"), TestIsolation::Codeunit);
    flagged(&o, "C_R2OReads", "OBS missing", SEQ);
    flagged(&o, "D_R14MReads", "OBS missing", SEQ);
    flagged(&o, "D_R14WReads", "OBS missing", SEQ);
    flagged(&o, "D_R14NReads", "OBS cur=1 next=2", SEQ);
    flagged(&o, "C_R2NReads", "OBS cur=1 next=2", SEQ);
    flagged(&o, "C_R2PReads", "OBS missing", SEQ);
}

// Shapes corpus PR 1468 did not measure keep main's refusal: a temporary RecordRef, a RecordRef method the cells do not call
// (Count), a RecordRef of another company, RecordRef access in a failed test that commits (C1 shape), and RecordRef access without any Commit().
const REF_UNMEASURED: &str = r#"
codeunit 50858 "P4 Ref Temp"
{
    Subtype = Test;
    [Test] procedure A_InsertsCommits() begin NumberSequence.Insert('P4RT', 1, 1); NumberSequence.Next('P4RT'); Commit(); end;
    [Test] procedure B_TempRefFails() var RR: RecordRef; begin RR.Open(Database::"P4 Line", true); if RR.FindLast() then; Error('B deliberate'); end;
    [Test] procedure C_TempRefReads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4RT')); end;
}
codeunit 50859 "P4 Ref Count"
{
    Subtype = Test;
    [Test] procedure A_InsertsCommits() begin NumberSequence.Insert('P4RC', 1, 1); NumberSequence.Next('P4RC'); Commit(); end;
    [Test] procedure B_RefCountFails() var RR: RecordRef; begin RR.Open(Database::"P4 Line"); if RR.Count() = 0 then; Error('B deliberate'); end;
    [Test] procedure C_RefCountReads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4RC')); end;
}
codeunit 50860 "P4 Ref C1"
{
    Subtype = Test;
    var Rows: Codeunit "Rows";
    [Test] procedure A_InsertsWritesCommitsFails() var RR: RecordRef; begin NumberSequence.Insert('P4R1', 1, 1); NumberSequence.Next('P4R1'); Rows.Put('R1'); RR.Open(Database::"P4 Line"); if RR.FindLast() then; Commit(); Error('A deliberate'); end;
    [Test] procedure B_RefC1Reads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4R1')); end;
}
codeunit 50861 "P4 Ref No Commit"
{
    Subtype = Test;
    [Test] procedure A_Inserts() begin NumberSequence.Insert('P4RN', 1, 1); NumberSequence.Next('P4RN'); end;
    [Test] procedure B_RefReadFails() var RR: RecordRef; begin RR.Open(Database::"P4 Line"); if RR.FindLast() then; Error('B deliberate'); end;
    [Test] procedure C_RefNoCommitReads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4RN')); end;
}
codeunit 50863 "P4 Ref Other Company"
{
    Subtype = Test;
    [Test] procedure A_InsertsCommits() begin NumberSequence.Insert('P4RO', 1, 1); NumberSequence.Next('P4RO'); Commit(); end;
    [Test] procedure B_OtherCompanyFails() var RR: RecordRef; begin RR.Open(Database::"P4 Line", false, 'P4 Other'); if RR.FindLast() then; Error('B deliberate'); end;
    [Test] procedure C_OtherCompanyReads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4RO')); end;
}
codeunit 50862 "P4 Open Only No Commit"
{
    Subtype = Test;
    [Test] procedure A_Inserts() begin NumberSequence.Insert('P4RON', 1, 1); NumberSequence.Next('P4RON'); end;
    [Test] procedure B_OpenOnlyFails() var RR: RecordRef; begin RR.Open(Database::"P4 Line"); RR.Close(); Error('B deliberate'); end;
    [Test] procedure C_OpenOnlyNoCommitReads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4RON')); end;
}
"#;

#[test]
fn recordref_shapes_the_probe_did_not_measure_are_refused() {
    let o = cells(&format!("{REF_LINE}\n{REF_UNMEASURED}"), TestIsolation::Codeunit);
    for (test, key) in [
        ("C_TempRefReads", "'P4RT'"),
        ("C_RefCountReads", "'P4RC'"),
        ("B_RefC1Reads", "'P4R1'"),
        ("C_RefNoCommitReads", "'P4RN'"),
        ("C_OpenOnlyNoCommitReads", "'P4RON'"),
        ("C_OtherCompanyReads", "'P4RO'"),
    ] {
        assert!(not_run(&row(&o, test).2, key), "{test}: {o:?}");
    }
}

// RecordRef table calls that read no row (review cells of draft PR 8512, round 2: Q03 KeyIndex and FieldCount, Q04
// FieldRef.SetRange, Q05 GetTable and Number, Q18 a FieldRef.Value assignment) and RecordRef.Open and Close next to a Record
// insert (Q14), or a temporary record next to a Record insert (Q17), in the C2 shape: corpus PR 1468 measures Open and Close
// alone (R14N, R2N) and a Record insert only beside a RecordRef read (R14M, R2M), so these keep main's refusal.
const REF_NO_READ: &str = r#"
codeunit 50864 "P4 Ref Key Only"
{
    Subtype = Test;
    [Test] procedure A_InsertsCommits() begin NumberSequence.Insert('P4KO', 1, 1); NumberSequence.Next('P4KO'); Commit(); end;
    [Test] procedure B_KeyOnlyFails() var RR: RecordRef; KR: KeyRef; N: Integer; begin RR.Open(Database::"P4 Line"); KR := RR.KeyIndex(1); N := KR.FieldCount; RR.Close(); Error('B deliberate'); end;
    [Test] procedure C_KeyOnlyReads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4KO')); end;
}
codeunit 50865 "P4 Ref SetRange Only"
{
    Subtype = Test;
    [Test] procedure A_InsertsCommits() begin NumberSequence.Insert('P4SO', 1, 1); NumberSequence.Next('P4SO'); Commit(); end;
    [Test] procedure B_SetRangeOnlyFails() var RR: RecordRef; FR: FieldRef; begin RR.Open(Database::"P4 Line"); FR := RR.Field(1); FR.SetRange('X'); RR.Close(); Error('B deliberate'); end;
    [Test] procedure C_SetRangeOnlyReads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4SO')); end;
}
codeunit 50866 "P4 Ref GetTable Only"
{
    Subtype = Test;
    [Test] procedure A_InsertsCommits() begin NumberSequence.Insert('P4GT', 1, 1); NumberSequence.Next('P4GT'); Commit(); end;
    [Test] procedure B_GetTableOnlyFails() var RR: RecordRef; Line: Record "P4 Line"; N: Integer; begin RR.GetTable(Line); N := RR.Number; Error('B deliberate'); end;
    [Test] procedure C_GetTableOnlyReads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4GT')); end;
}
codeunit 50867 "P4 Ref Value Only"
{
    Subtype = Test;
    [Test] procedure A_InsertsCommits() begin NumberSequence.Insert('P4VO', 1, 1); NumberSequence.Next('P4VO'); Commit(); end;
    [Test] procedure B_ValueOnlyFails() var RR: RecordRef; FR: FieldRef; begin RR.Open(Database::"P4 Line"); FR := RR.Field(1); FR.Value := 'X'; RR.Close(); Error('B deliberate'); end;
    [Test] procedure C_ValueOnlyReads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4VO')); end;
}
codeunit 50868 "P4 Open Then Insert"
{
    Subtype = Test;
    [Test] procedure A_InsertsCommits() begin NumberSequence.Insert('P4OI', 1, 1); NumberSequence.Next('P4OI'); Commit(); end;
    [Test] procedure B_OpenThenInsertFails() var RR: RecordRef; Line: Record "P4 Line"; begin RR.Open(Database::"P4 Line"); RR.Close(); Line.Doc := 'OI'; Line."Line No." := 1; Line.Insert(); Error('B deliberate'); end;
    [Test] procedure C_OpenThenInsertReads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4OI')); end;
}
codeunit 50869 "P4 Temp Then Insert"
{
    Subtype = Test;
    [Test] procedure A_InsertsCommits() begin NumberSequence.Insert('P4TI', 1, 1); NumberSequence.Next('P4TI'); Commit(); end;
    [Test] procedure B_TempThenInsertFails() var TempLine: Record "P4 Line" temporary; Line: Record "P4 Line"; begin TempLine.Doc := 'T'; TempLine.Insert(); if TempLine.Count() = 0 then; Line.Doc := 'TI'; Line."Line No." := 1; Line.Insert(); Error('B deliberate'); end;
    [Test] procedure C_TempThenInsertReads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4TI')); end;
}
"#;

#[test]
fn table_calls_no_cell_measures_without_a_read_are_refused() {
    let o = cells(&format!("{REF_LINE}\n{REF_NO_READ}"), TestIsolation::Codeunit);
    for (test, key) in [
        ("C_KeyOnlyReads", "'P4KO'"),
        ("C_SetRangeOnlyReads", "'P4SO'"),
        ("C_GetTableOnlyReads", "'P4GT'"),
        ("C_ValueOnlyReads", "'P4VO'"),
        ("C_OpenThenInsertReads", "'P4OI'"),
        ("C_TempThenInsertReads", "'P4TI'"),
    ] {
        assert!(not_run(&row(&o, test).2, key), "{test}: {o:?}");
    }
}

// A failed test whose only table access is Record.SetRange and Record.FindLast after an earlier test's Commit(): corpus PR
// 1468 (draft) R2R SqRecordRefFail.al:913 (no row found, SqRecordRefReader.al:135) and R14R :597 (a row found, :86), each
// with LastLineByRecord :118: the new sequence is gone. Other Record reads after a Commit() keep main's refusal: Count,
// FindSet, IsEmpty, and FindLast beside a Get (no cell measures them; Get alone: C_GetReads above).
const RECORD_READS: &str = r#"
codeunit 50870 "P4 R2R"
{
    Subtype = Test;
    [Test] procedure A_InsertsCommits() begin NumberSequence.Insert('P4R2R', 1, 1); NumberSequence.Next('P4R2R'); Commit(); end;
    [Test] procedure B_FindLastFails() var Line: Record "P4 Line"; begin Line.SetRange(Doc, 'R2R'); if Line.FindLast() then; Error('B deliberate'); end;
    [Test] procedure C_R2RReads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4R2R')); end;
}
codeunit 50871 "P4 R14R"
{
    Subtype = Test;
    [Test] procedure A_WritesCommits() var Line: Record "P4 Line"; begin Line.Doc := 'R14R'; Line."Line No." := 10000; Line.Insert(); Commit(); end;
    [Test] procedure B_Inserts() begin NumberSequence.Insert('P4R14R', 1, 1); NumberSequence.Next('P4R14R'); end;
    [Test] procedure C_FindLastFails() var Line: Record "P4 Line"; begin Line.SetRange(Doc, 'R14R'); if Line.FindLast() then; Error('C deliberate %1', Line."Line No."); end;
    [Test] procedure D_R14RReads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4R14R')); end;
}
codeunit 50872 "P4 Record Count"
{
    Subtype = Test;
    [Test] procedure A_InsertsCommits() begin NumberSequence.Insert('P4RCT', 1, 1); NumberSequence.Next('P4RCT'); Commit(); end;
    [Test] procedure B_CountFails() var Line: Record "P4 Line"; begin if Line.Count() = 0 then; Error('B deliberate'); end;
    [Test] procedure C_RecordCountReads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4RCT')); end;
}
codeunit 50873 "P4 Record FindSet"
{
    Subtype = Test;
    [Test] procedure A_InsertsCommits() begin NumberSequence.Insert('P4RFS', 1, 1); NumberSequence.Next('P4RFS'); Commit(); end;
    [Test] procedure B_FindSetFails() var Line: Record "P4 Line"; begin if Line.FindSet() then; Error('B deliberate'); end;
    [Test] procedure C_RecordFindSetReads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4RFS')); end;
}
codeunit 50874 "P4 Record IsEmpty"
{
    Subtype = Test;
    [Test] procedure A_InsertsCommits() begin NumberSequence.Insert('P4RIE', 1, 1); NumberSequence.Next('P4RIE'); Commit(); end;
    [Test] procedure B_IsEmptyFails() var Line: Record "P4 Line"; begin if Line.IsEmpty() then; Error('B deliberate'); end;
    [Test] procedure C_RecordIsEmptyReads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4RIE')); end;
}
codeunit 50875 "P4 Record FindLast And Get"
{
    Subtype = Test;
    [Test] procedure A_InsertsCommits() begin NumberSequence.Insert('P4RFG', 1, 1); NumberSequence.Next('P4RFG'); Commit(); end;
    [Test] procedure B_FindLastGetFails() var Line: Record "P4 Line"; begin if Line.FindLast() then; if Line.Get('X', 1) then; Error('B deliberate'); end;
    [Test] procedure C_RecordFindLastGetReads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4RFG')); end;
}
"#;

#[test]
fn a_failed_test_that_only_ran_record_findlast_after_a_commit_takes_the_sequence_away() {
    let o = cells(&format!("{REF_LINE}\n{RECORD_READS}"), TestIsolation::Codeunit);
    flagged(&o, "C_R2RReads", "OBS missing", SEQ);
    flagged(&o, "D_R14RReads", "OBS missing", SEQ);
    for (test, key) in [
        ("C_RecordCountReads", "'P4RCT'"),
        ("C_RecordFindSetReads", "'P4RFS'"),
        ("C_RecordIsEmptyReads", "'P4RIE'"),
        ("C_RecordFindLastGetReads", "'P4RFG'"),
    ] {
        assert!(not_run(&row(&o, test).2, key), "{test}: {o:?}");
    }
}

// Neighbors of R14R and R2R that no cell measures (review probe of PR 8512, round 3, cells F03, F06, F08, F09, F12, F17,
// F19): two FindLast calls, no filter, SetAutoCalcFields, FindLast raising, LockTable, SetLoadFields, FilterGroup. Main's
// refusal, except for F08, whose earlier test commits after a Record write.
const RECORD_READ_NEIGHBORS: &str = r#"
table 50877 "P4 Calc Line" { fields { field(1; Doc; Code[20]) { } field(2; "Line No."; Integer) { } field(3; Lines; Integer) { FieldClass = FlowField; CalcFormula = count("P4 Calc Line" where(Doc = field(Doc))); } } keys { key(PK; Doc, "Line No.") { Clustered = true; } } }
codeunit 50878 "P4 F03"
{
    Subtype = Test;
    [Test] procedure A_InsertsCommits() begin NumberSequence.Insert('P4F03', 1, 1); NumberSequence.Next('P4F03'); Commit(); end;
    [Test] procedure B_Fails() var Line: Record "P4 Line"; begin Line.SetRange(Doc, 'F03'); if Line.FindLast() then; if Line.FindLast() then; Error('B deliberate'); end;
    [Test] procedure C_F03Reads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4F03')); end;
}
codeunit 50879 "P4 F06"
{
    Subtype = Test;
    [Test] procedure A_InsertsCommits() begin NumberSequence.Insert('P4F06', 1, 1); NumberSequence.Next('P4F06'); Commit(); end;
    [Test] procedure B_Fails() var Line: Record "P4 Line"; begin if Line.FindLast() then; Error('B deliberate'); end;
    [Test] procedure C_F06Reads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4F06')); end;
}
codeunit 50880 "P4 F08"
{
    Subtype = Test;
    [Test] procedure A_InsertsCommits() var Line: Record "P4 Calc Line"; begin Line.Doc := 'F08'; Line."Line No." := 10000; Line.Insert(); NumberSequence.Insert('P4F08', 1, 1); NumberSequence.Next('P4F08'); Commit(); end;
    [Test] procedure B_Fails() var Line: Record "P4 Calc Line"; begin Line.SetRange(Doc, 'F08'); Line.SetAutoCalcFields(Lines); if Line.FindLast() then; Error('B deliberate'); end;
    [Test] procedure C_F08Reads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4F08')); end;
}
codeunit 50881 "P4 F09"
{
    Subtype = Test;
    [Test] procedure A_InsertsCommits() begin NumberSequence.Insert('P4F09', 1, 1); NumberSequence.Next('P4F09'); Commit(); end;
    [Test] procedure B_Fails() var Line: Record "P4 Line"; begin Line.SetRange(Doc, 'F09'); Line.FindLast(); Error('B deliberate'); end;
    [Test] procedure C_F09Reads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4F09')); end;
}
codeunit 50882 "P4 F12"
{
    Subtype = Test;
    [Test] procedure A_InsertsCommits() begin NumberSequence.Insert('P4F12', 1, 1); NumberSequence.Next('P4F12'); Commit(); end;
    [Test] procedure B_Fails() var Line: Record "P4 Line"; begin Line.LockTable(); Line.SetRange(Doc, 'F12'); if Line.FindLast() then; Error('B deliberate'); end;
    [Test] procedure C_F12Reads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4F12')); end;
}
codeunit 50883 "P4 F17"
{
    Subtype = Test;
    [Test] procedure A_InsertsCommits() begin NumberSequence.Insert('P4F17', 1, 1); NumberSequence.Next('P4F17'); Commit(); end;
    [Test] procedure B_Fails() var Line: Record "P4 Line"; begin Line.SetLoadFields(Doc); Line.SetRange(Doc, 'F17'); if Line.FindLast() then; Error('B deliberate'); end;
    [Test] procedure C_F17Reads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4F17')); end;
}
codeunit 50884 "P4 F19"
{
    Subtype = Test;
    [Test] procedure A_InsertsCommits() begin NumberSequence.Insert('P4F19', 1, 1); NumberSequence.Next('P4F19'); Commit(); end;
    [Test] procedure B_Fails() var Line: Record "P4 Line"; begin Line.FilterGroup(2); Line.SetRange(Doc, 'F19'); if Line.FindLast() then; Error('B deliberate'); end;
    [Test] procedure C_F19Reads() var R: Codeunit "P4 Read"; begin Error('OBS %1', R.Of('P4F19')); end;
}
"#;

#[test]
fn record_findlast_shapes_beside_the_cells_keep_mains_refusal() {
    let o = cells(&format!("{REF_LINE}\n{RECORD_READ_NEIGHBORS}"), TestIsolation::Codeunit);
    // F08 inserts after a Record write and commits: the commit keeps the sequence, whatever the failed test does (corpus
    // PR 1572 S1U SqFailedSeq.al:2166, S1F :662).
    flagged(&o, "C_F08Reads", "OBS cur=1 next=2", SEQ);
    for cell in ["F03", "F06", "F09", "F12", "F17", "F19"] {
        let test = format!("C_{cell}Reads");
        assert!(not_run(&row(&o, &test).2, &format!("'P4{cell}'")), "{test}: {o:?}");
    }
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
