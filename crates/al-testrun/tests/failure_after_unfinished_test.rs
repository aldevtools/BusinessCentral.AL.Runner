//! A failure after a test that did not run to its end is not trusted only when that test could have changed what the
//! failing test reads: a table, a global of the test codeunit or of a single-instance codeunit, session state such as
//! `WorkDate` or a number sequence, or the `Random` sequence the failing test drew from. A test refused while it ran
//! counts with its whole static reach (its partial run is a part of it), unless a statement of its own body refused it:
//! then with what it wrote before the refusal and the reach of the statements from that one on. A reach that is not
//! known may change anything. The triggers of the tables a test uses are part of its reach.

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

const SRC: &str = r#"
table 50410 "Written" { fields { field(1; "No."; Integer) { } } keys { key(PK; "No.") { } } }
table 50411 "Other" { fields { field(1; "No."; Integer) { } } keys { key(PK; "No.") { } } }
table 50412 "Trigger Source"
{
    fields { field(1; "No."; Integer) { } }
    keys { key(PK; "No.") { } }
    trigger OnInsert()
    var
        T: Record "Trigger Target";
    begin
        T."No." := Rec."No.";
        T.Insert();
    end;
}
table 50413 "Trigger Target" { fields { field(1; "No."; Integer) { } } keys { key(PK; "No.") { } } }
report 50401 "R" { ProcessingOnly = true; }

codeunit 50420 "SI State"
{
    SingleInstance = true;
    var
        Flag: Boolean;
    procedure SetFlag()
    begin
        Flag := true;
    end;
    procedure GetFlag(): Boolean
    begin
        exit(Flag);
    end;
}

codeunit 50400 "MidRun Tables"
{
    Subtype = Test;
    [Test]
    procedure A_RefusedMidRunWritesWritten()
    var
        W: Record "Written";
    begin
        Sleep(-1);
        W.Insert();
    end;
    [Test]
    procedure B_ReadsOtherFails()
    var
        O: Record "Other";
    begin
        if O.IsEmpty() then Error('boom');
    end;
    [Test]
    procedure C_ReadsWrittenFails()
    var
        W: Record "Written";
    begin
        if W.IsEmpty() then Error('boom');
    end;
}

codeunit 50401 "MidRun Globals"
{
    Subtype = Test;
    var
        Counter: Integer;
    [Test]
    procedure A_RefusedMidRunCounts()
    begin
        Sleep(-1);
        Counter += 1;
    end;
    [Test]
    procedure B_FailsWithoutReadingIt()
    begin
        Error('boom');
    end;
    [Test]
    procedure C_ReadsTheCounterFails()
    begin
        if Counter = 0 then Error('boom');
    end;
}

codeunit 50402 "MidRun SingleInstance"
{
    Subtype = Test;
    [Test]
    procedure A_RefusedMidRunSetsTheFlag()
    var
        S: Codeunit "SI State";
    begin
        Sleep(-1);
        S.SetFlag();
    end;
    [Test]
    procedure B_FailsWithoutReadingIt()
    begin
        Error('boom');
    end;
    [Test]
    procedure C_ReadsTheFlagFails()
    var
        S: Codeunit "SI State";
    begin
        if not S.GetFlag() then Error('boom');
    end;
}

codeunit 50403 "MidRun WorkDate"
{
    Subtype = Test;
    [Test]
    procedure A_RefusedMidRunSetsWorkDate()
    begin
        Sleep(-1);
        WorkDate(20240101D);
    end;
    [Test]
    procedure B_Fails()
    begin
        Error('boom');
    end;
}

codeunit 50404 "MidRun NumberSequence"
{
    Subtype = Test;
    [Test]
    procedure A_RefusedMidRunTakesANumber()
    begin
        Sleep(-1);
        if NumberSequence.Next('UFSEQ') = 0 then;
    end;
    [Test]
    procedure B_FailsWithoutASequence()
    begin
        Error('boom');
    end;
    [Test]
    procedure C_ReadsTheSequenceFails()
    begin
        if not NumberSequence.Exists('UFSEQ') then Error('boom');
    end;
}

codeunit 50405 "Refused Trigger Writes"
{
    Subtype = Test;
    [Test]
    [HandlerFunctions('H')]
    procedure A_RefusedInsertsWithTrigger()
    var
        S: Record "Trigger Source";
    begin
        S."No." := 1;
        S.Insert(true);
    end;
    [Test]
    procedure B_ReadsTheTriggerTargetFails()
    var
        T: Record "Trigger Target";
    begin
        if T.IsEmpty() then Error('boom');
    end;
    [FilterPageHandler]
    procedure H(var Builder: FilterPageBuilder): Boolean
    begin
    end;
}

codeunit 50406 "Random After Failure"
{
    Subtype = Test;
    [Test]
    procedure A_DrawsAndFails()
    begin
        if Random(10) > 0 then Error('first');
    end;
    [Test]
    procedure B_DrawsFromTheSequenceItFoundAndFails()
    begin
        if Random(10) > 0 then Error('boom');
    end;
    [Test]
    procedure C_ReseedsThenDrawsAndFails()
    begin
        Randomize(7);
        if Random(10) > 0 then Error('boom');
    end;
}

codeunit 50407 "MidRun Unknown"
{
    Subtype = Test;
    [Test]
    procedure A_RefusedMidRunRunsByAComputedId()
    var
        Id: Integer;
    begin
        Sleep(-1);
        Id := 50420;
        Codeunit.Run(Id);
    end;
    [Test]
    procedure B_Fails()
    begin
        Error('boom');
    end;
}

codeunit 50408 "Settled Flag"
{
    Subtype = Test;
    var
        IsInitialized: Boolean;
    local procedure Initialize()
    begin
        if IsInitialized then exit;
        IsInitialized := true;
    end;
    [Test]
    procedure A_InitializesThenIsRefused()
    begin
        Initialize();
        Sleep(-1);
        Initialize();
    end;
    [Test]
    procedure B_InitializesAndFails()
    begin
        Initialize();
        Error('boom');
    end;
}

codeunit 50409 "Unsettled Flag"
{
    Subtype = Test;
    var
        IsInitialized: Boolean;
    local procedure Initialize()
    begin
        if IsInitialized then exit;
        IsInitialized := true;
    end;
    [Test]
    procedure A_RefusedBeforeItInitializes()
    begin
        Sleep(-1);
        Initialize();
    end;
    [Test]
    procedure B_InitializesAndFails()
    begin
        Initialize();
        Error('boom');
    end;
}
"#;

fn run() -> CorpusRuntime {
    CorpusRuntime::run_suites_with(
        vec![AppSource {
            id: "main".into(),
            name: "main".into(),
            dependencies: vec![],
            files: vec![("main.al".into(), SRC.into())],
        }],
        vec![al_sema::AppKind::Workspace],
        &[],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions::default(),
    )
}

fn outcome(rt: &CorpusRuntime, codeunit: u32, name: &str) -> Outcome {
    rt.first_outcome(&CaseId {
        codeunit,
        name: name.into(),
    })
}

fn not_trusted(o: &Outcome) -> bool {
    matches!(o, Outcome::NotRun(m) if m.contains("this failure is not trusted"))
}

fn failed(o: &Outcome) -> bool {
    matches!(o, Outcome::Fail(_))
}

#[test]
fn a_refusal_mid_run_distrusts_only_a_failure_that_reads_a_table_it_can_write() {
    let rt = run();
    let a = outcome(&rt, 50400, "A_RefusedMidRunWritesWritten");
    assert!(matches!(&a, Outcome::NotRun(m) if m.contains("Sleep")), "{a:?}");
    let b = outcome(&rt, 50400, "B_ReadsOtherFails");
    assert!(failed(&b), "{b:?}");
    let c = outcome(&rt, 50400, "C_ReadsWrittenFails");
    assert!(not_trusted(&c), "{c:?}");
    assert!(
        matches!(&c, Outcome::NotRun(m) if m.contains("refused while it ran") && m.contains("Written")),
        "{c:?}"
    );
}

#[test]
fn a_global_of_the_test_codeunit_the_refused_test_can_change_distrusts_a_failure_that_reads_it() {
    let rt = run();
    let b = outcome(&rt, 50401, "B_FailsWithoutReadingIt");
    assert!(failed(&b), "{b:?}");
    let c = outcome(&rt, 50401, "C_ReadsTheCounterFails");
    assert!(
        matches!(&c, Outcome::NotRun(m) if m.contains("not trusted") && m.contains("Counter")),
        "{c:?}"
    );
}

#[test]
fn a_global_of_a_single_instance_codeunit_counts_like_one_of_the_test_codeunit() {
    let rt = run();
    let b = outcome(&rt, 50402, "B_FailsWithoutReadingIt");
    assert!(failed(&b), "{b:?}");
    let c = outcome(&rt, 50402, "C_ReadsTheFlagFails");
    assert!(
        matches!(&c, Outcome::NotRun(m) if m.contains("not trusted") && m.contains("Flag")),
        "{c:?}"
    );
}

#[test]
fn work_date_is_read_by_every_test() {
    let rt = run();
    let b = outcome(&rt, 50403, "B_Fails");
    assert!(
        matches!(&b, Outcome::NotRun(m) if m.contains("not trusted") && m.contains("WorkDate")),
        "{b:?}"
    );
}

#[test]
fn a_number_sequence_counts_for_a_test_that_uses_one() {
    let rt = run();
    let b = outcome(&rt, 50404, "B_FailsWithoutASequence");
    assert!(failed(&b), "{b:?}");
    let c = outcome(&rt, 50404, "C_ReadsTheSequenceFails");
    assert!(not_trusted(&c), "{c:?}");
}

#[test]
fn what_the_triggers_of_a_table_write_is_part_of_the_reach() {
    let rt = run();
    let a = outcome(&rt, 50405, "A_RefusedInsertsWithTrigger");
    assert!(matches!(a, Outcome::NotRun(_)), "{a:?}");
    let b = outcome(&rt, 50405, "B_ReadsTheTriggerTargetFails");
    assert!(not_trusted(&b), "{b:?}");
}

#[test]
fn a_test_that_reseeds_before_it_draws_does_not_depend_on_earlier_draws() {
    let rt = run();
    let a = outcome(&rt, 50406, "A_DrawsAndFails");
    assert!(failed(&a), "{a:?}");
    let b = outcome(&rt, 50406, "B_DrawsFromTheSequenceItFoundAndFails");
    assert!(not_trusted(&b), "{b:?}");
    let c = outcome(&rt, 50406, "C_ReseedsThenDrawsAndFails");
    assert!(failed(&c), "{c:?}");
}

#[test]
fn a_reach_that_is_not_known_may_change_anything() {
    let rt = run();
    let b = outcome(&rt, 50407, "B_Fails");
    assert!(
        matches!(&b, Outcome::NotRun(m) if m.contains("not trusted") && m.contains("not known")),
        "{b:?}"
    );
}

#[test]
fn a_flag_that_already_holds_the_only_value_any_test_sets_does_not_count() {
    let rt = run();
    let b = outcome(&rt, 50408, "B_InitializesAndFails");
    assert!(failed(&b), "{b:?}");
    let b = outcome(&rt, 50409, "B_InitializesAndFails");
    assert!(
        matches!(&b, Outcome::NotRun(m) if m.contains("not trusted") && m.contains("IsInitialized")),
        "{b:?}"
    );
}

// al-runner issue 5961: a test refused while it ran counts with what it wrote while it ran and the static reach of the
// statements it did not reach, not with its whole static reach. Here every table write reaches a `Global Triggers`
// subscriber whose reach is not known, as Office Host Provider's RunOnClient variable is in Microsoft's Base App.
const REST_SRC: &str = r#"
table 50450 "Rest Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
table 50451 "Rest Other" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
page 50450 "Rest Row List"
{
    PageType = List;
    SourceTable = "Rest Row";
    layout { area(Content) { repeater(Rows) { field(Id; Rec.Id) { } } } }
}

codeunit 50452 "Rest Global Triggers"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'OnDatabaseInsert', '', false, false)]
    local procedure OnInsert(RecRef: RecordRef)
    var
        Id: Integer;
    begin
        if RecRef.Number = 0 then
            Codeunit.Run(Id);
    end;
}

codeunit 50450 "Rest Refused On Page"
{
    Subtype = Test;
    [Test]
    procedure A_RefusedAtTheEndOnAnOwnPage()
    var
        Row: Record "Rest Row";
        RowList: TestPage "Rest Row List";
    begin
        Row.SetRange(Id, 1, 2);
        RowList.Trap();
        Page.Run(Page::"Rest Row List", Row);
        Sleep(StrLen(RowList.Filter.GetFilter(Id)) - 10);
    end;
    [Test]
    procedure B_FailsReadingNothing()
    begin
        Error('boom');
    end;
}

codeunit 50451 "Rest Writes After"
{
    Subtype = Test;
    [Test]
    procedure A_RefusedBeforeAPageWrite()
    var
        RowList: TestPage "Rest Row List";
    begin
        RowList.OpenNew();
        Sleep(-1);
        RowList.Id.SetValue(5);
        RowList.Close();
    end;
    [Test]
    procedure B_FailsReadingNothing()
    begin
        Error('boom');
    end;
}

codeunit 50453 "Rest Wrote Before"
{
    Subtype = Test;
    [Test]
    procedure A_WritesThenIsRefused()
    var
        O: Record "Rest Other";
    begin
        O.Id := 1;
        O.Insert();
        Sleep(-1);
    end;
    [Test]
    procedure B_ReadsRowFails()
    var
        R: Record "Rest Row";
    begin
        if R.IsEmpty() then Error('boom');
    end;
    [Test]
    procedure C_ReadsWhatItWroteFails()
    var
        O: Record "Rest Other";
    begin
        if O.IsEmpty() then Error('boom');
    end;
}
"#;

fn run_rest() -> CorpusRuntime {
    CorpusRuntime::run_suites_with(
        vec![AppSource {
            id: "main".into(),
            name: "main".into(),
            dependencies: vec![],
            files: vec![("main.al".into(), REST_SRC.into())],
        }],
        vec![al_sema::AppKind::Workspace],
        &[],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions::default(),
    )
}

#[test]
fn a_test_refused_at_its_last_statement_counts_only_what_it_did_not_run() {
    let rt = run_rest();
    let a = outcome(&rt, 50450, "A_RefusedAtTheEndOnAnOwnPage");
    assert!(matches!(&a, Outcome::NotRun(m) if m.contains("Sleep")), "{a:?}");
    let b = outcome(&rt, 50450, "B_FailsReadingNothing");
    assert!(failed(&b), "{b:?}");
}

#[test]
fn a_page_write_after_the_refusal_still_counts_with_its_reach() {
    let rt = run_rest();
    let a = outcome(&rt, 50451, "A_RefusedBeforeAPageWrite");
    assert!(matches!(&a, Outcome::NotRun(m) if m.contains("Sleep")), "{a:?}");
    let b = outcome(&rt, 50451, "B_FailsReadingNothing");
    assert!(
        matches!(&b, Outcome::NotRun(m) if m.contains("not trusted") && m.contains("not known")),
        "{b:?}"
    );
}

#[test]
fn what_a_refused_test_wrote_before_the_refusal_counts() {
    let rt = run_rest();
    let b = outcome(&rt, 50453, "B_ReadsRowFails");
    assert!(failed(&b), "{b:?}");
    let c = outcome(&rt, 50453, "C_ReadsWhatItWroteFails");
    assert!(
        matches!(&c, Outcome::NotRun(m) if m.contains("not trusted") && m.contains("Rest Other")),
        "{c:?}"
    );
}
