//! A failure after a test that was refused before it started is not trusted only when that test could have changed
//! what the failing test reads: a table the refused test writes (its static reach) that the failing test uses, or the
//! `Random` sequence when both can draw. Any earlier refused test counts, not only the one right before.
//! A refusal that does not meet the failing test leaves its failure a failure.

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

const SRC: &str = r#"
table 50310 "Written" { fields { field(1; "No."; Integer) { } } keys { key(PK; "No.") { } } }
table 50311 "Other" { fields { field(1; "No."; Integer) { } } keys { key(PK; "No.") { } } }
report 50301 "R" { ProcessingOnly = true; }

codeunit 50300 "Tables"
{
    Subtype = Test;
    [Test]
    [HandlerFunctions('H')]
    procedure A_RefusedAndWritesWritten()
    var
        W: Record "Written";
    begin
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
    [Test]
    procedure D_ReadsWrittenThroughACallFails()
    begin
        Check();
    end;
    local procedure Check()
    var
        W: Record "Written";
    begin
        if W.IsEmpty() then Error('boom');
    end;
    [Test]
    procedure E_NoRecordFails()
    begin
        Error('boom');
    end;
    [FilterPageHandler]
    procedure H(var Builder: FilterPageBuilder): Boolean
    begin
    end;
}

codeunit 50320 "Reads Only"
{
    Subtype = Test;
    [Test]
    [HandlerFunctions('H')]
    procedure A_RefusedButOnlyReads()
    var
        W: Record "Written";
    begin
        if W.IsEmpty() then;
    end;
    [Test]
    procedure B_ReadsWrittenFails()
    var
        W: Record "Written";
    begin
        if W.IsEmpty() then Error('boom');
    end;
    [FilterPageHandler]
    procedure H(var Builder: FilterPageBuilder): Boolean
    begin
    end;
}

codeunit 50330 "Random"
{
    Subtype = Test;
    [Test]
    [HandlerFunctions('H')]
    procedure A_RefusedWithoutRandom()
    begin
    end;
    [Test]
    procedure B_DrawsAndFails()
    begin
        if Random(10) > 0 then Error('boom');
    end;
    [FilterPageHandler]
    procedure H(var Builder: FilterPageBuilder): Boolean
    begin
    end;
}

codeunit 50350 "Random Seeded"
{
    Subtype = Test;
    [Test]
    [HandlerFunctions('H')]
    procedure A_RefusedThatSeeds()
    begin
        Seed();
    end;
    local procedure Seed()
    begin
        Randomize(5);
    end;
    [Test]
    procedure B_DrawsAndFails()
    begin
        if Random(10) > 0 then Error('boom');
    end;
    [Test]
    procedure C_FailsWithoutDrawing()
    begin
        Error('boom');
    end;
    [FilterPageHandler]
    procedure H(var Builder: FilterPageBuilder): Boolean
    begin
    end;
}

codeunit 50340 "Random Reached"
{
    Subtype = Test;
    [Test]
    [HandlerFunctions('H')]
    procedure A_RefusedThatDraws()
    begin
        Draw();
    end;
    local procedure Draw()
    begin
        if Random(10) = 0 then;
    end;
    [Test]
    procedure B_DrawsAndFails()
    begin
        if Random(10) > 0 then Error('boom');
    end;
    [Test]
    procedure C_FailsWithoutDrawing()
    begin
        Error('boom');
    end;
    [FilterPageHandler]
    procedure H(var Builder: FilterPageBuilder): Boolean
    begin
    end;
}
"#;

fn run() -> CorpusRuntime {
    run_src(SRC)
}

fn run_src(src: &str) -> CorpusRuntime {
    CorpusRuntime::run_suites_with(
        vec![AppSource {
            id: "main".into(),
            name: "main".into(),
            dependencies: vec![],
            files: vec![("main.al".into(), src.into())],
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

fn not_trusted(o: Outcome) -> bool {
    matches!(o, Outcome::NotRun(m) if m.contains("this failure is not trusted"))
}

#[test]
fn a_failure_reading_a_table_the_refused_test_writes_is_not_trusted() {
    let rt = run();
    assert!(matches!(
        outcome(&rt, 50300, "A_RefusedAndWritesWritten"),
        Outcome::NotRun(_)
    ));
    assert!(not_trusted(outcome(&rt, 50300, "C_ReadsWrittenFails")));
}

#[test]
fn the_refused_test_does_not_have_to_be_the_neighbor_nor_the_read_direct() {
    let rt = run();
    // B and C sit between the refused test and D.
    assert!(not_trusted(outcome(&rt, 50300, "D_ReadsWrittenThroughACallFails")));
}

#[test]
fn a_failure_that_reads_no_table_the_refused_test_writes_stays_a_failure() {
    let rt = run();
    assert!(matches!(outcome(&rt, 50300, "B_ReadsOtherFails"), Outcome::Fail(_)));
    assert!(matches!(outcome(&rt, 50300, "E_NoRecordFails"), Outcome::Fail(_)));
}

#[test]
fn a_refused_test_that_only_reads_makes_no_later_failure_doubtful() {
    let rt = run();
    assert!(matches!(
        outcome(&rt, 50320, "A_RefusedButOnlyReads"),
        Outcome::NotRun(_)
    ));
    assert!(matches!(outcome(&rt, 50320, "B_ReadsWrittenFails"), Outcome::Fail(_)));
}

#[test]
fn a_refused_test_that_cannot_draw_random_numbers_does_not_distrust_a_drawing_failure() {
    let rt = run();
    assert!(matches!(
        outcome(&rt, 50330, "A_RefusedWithoutRandom"),
        Outcome::NotRun(_)
    ));
    assert!(matches!(outcome(&rt, 50330, "B_DrawsAndFails"), Outcome::Fail(_)));
}

/// ADR-061: each test restarts `Random` from the run seed, so what an earlier refused test would have drawn does not
/// change a later test's numbers.
#[test]
fn a_refused_test_that_only_draws_random_numbers_does_not_distrust_a_drawing_failure() {
    let rt = run();
    assert!(matches!(outcome(&rt, 50340, "B_DrawsAndFails"), Outcome::Fail(_)));
    assert!(matches!(outcome(&rt, 50340, "C_FailsWithoutDrawing"), Outcome::Fail(_)));
}

/// A refused test that can call `Randomize(n)` may have left a sequence that goes on into the next test
/// (types/TestRandomSeededContracts.al:111-131): a later failure that draws is not trusted, one that does not draw is.
#[test]
fn a_refused_test_that_can_seed_random_distrusts_a_drawing_failure_only() {
    let rt = run();
    assert!(not_trusted(outcome(&rt, 50350, "B_DrawsAndFails")));
    assert!(matches!(outcome(&rt, 50350, "C_FailsWithoutDrawing"), Outcome::Fail(_)));
}

// The handler functions a test names are part of its reach on both sides (review of al-runner PR 8363, probe trust): a
// failure that reads the table only in its handler, after a refused test that writes it, is not trusted; so is a failure
// after a refused test that writes the table only in its handler. A refused test whose handlers write nothing leaves the
// failure a failure.
const HANDLERS: &str = r#"
table 50335 "Trust Row" { fields { field(1; K; Code[10]) { } } keys { key(PK; K) { } } }
codeunit 50336 "Later Handler Reads"
{
    Subtype = Test;
    var Seen: Integer;
    [Test] [HandlerFunctions('H')] procedure T1_RefusedWritesInBody() var R: Record "Trust Row"; begin R.K := 'T1'; R.Insert(); end;
    [Test] [HandlerFunctions('MsgRead')] procedure T2_ReadsInHandlerAndFails() begin Message('x'); if Seen = 0 then Error('T2 sees no row'); end;
    [MessageHandler] procedure MsgRead(T: Text[1024]) var R: Record "Trust Row"; begin Seen := R.Count(); end;
    [FilterPageHandler] procedure H(var Builder: FilterPageBuilder): Boolean begin end;
}
codeunit 50337 "Earlier Handler Writes"
{
    Subtype = Test;
    [Test] [HandlerFunctions('H,MsgW')] procedure U1_RefusedWritesInHandler() begin Message('x'); end;
    [Test] procedure U2_ReadsInBodyAndFails() var R: Record "Trust Row"; begin if R.IsEmpty() then Error('U2 sees no row'); end;
    [MessageHandler] procedure MsgW(T: Text[1024]) var R: Record "Trust Row"; begin R.K := 'U1'; R.Insert(); end;
    [FilterPageHandler] procedure H(var Builder: FilterPageBuilder): Boolean begin end;
}
codeunit 50338 "Handler Writes Nothing"
{
    Subtype = Test;
    [Test] [HandlerFunctions('H,Msg')] procedure V1_RefusedWritesNothing() begin Message('x'); end;
    [Test] procedure V2_ReadsInBodyAndFails() var R: Record "Trust Row"; begin if R.IsEmpty() then Error('V2 sees no row'); end;
    [MessageHandler] procedure Msg(T: Text[1024]) begin end;
    [FilterPageHandler] procedure H(var Builder: FilterPageBuilder): Boolean begin end;
}
"#;

#[test]
fn the_handler_functions_a_test_names_count_on_both_sides_of_the_trust_check() {
    let rt = run_src(HANDLERS);
    for (codeunit, test) in [(50336, "T1_RefusedWritesInBody"), (50337, "U1_RefusedWritesInHandler")] {
        assert!(matches!(outcome(&rt, codeunit, test), Outcome::NotRun(_)), "{test}");
    }
    assert!(not_trusted(outcome(&rt, 50336, "T2_ReadsInHandlerAndFails")));
    assert!(not_trusted(outcome(&rt, 50337, "U2_ReadsInBodyAndFails")));
    assert_eq!(
        outcome(&rt, 50338, "V2_ReadsInBodyAndFails"),
        Outcome::Fail("V2 sees no row".into())
    );
}
