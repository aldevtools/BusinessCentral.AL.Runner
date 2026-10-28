//! How many times a page open runs the CaptionClass expressions of its source table with an empty record (al-runner issue
//! 5870, assumption page-open-caption-class-empty-record-count). BC gives no stable number: the corpus saw 240, 480 and 492
//! calls for the same Card open (record/lt-qd-captionclass-areas/QdCaptionClassAreas.al:657) and asserts only that such runs
//! happen (:543 `id0>0=Yes`); the issue's reporter saw 8 per open on one BC 29 container where al-runner runs 20. A test whose
//! outcome depends on the number is refused; a test that only sees that the runs happen, or reads a caption, still runs.

use al_testkit::{blocked, compile};
use al_testrun::Outcome;
use al_testrun::isolation::TestIsolation;

fn run(src: &str, codeunit: &str) -> Vec<(String, Outcome)> {
    let l = compile(src);
    let engine = al_vm::Engine::new(l.program.clone(), blocked(&l));
    let oi = l.program.objects.iter().position(|o| o.name == codeunit).unwrap();
    al_testrun::runtime::run_codeunit_full(
        &engine,
        &l,
        oi,
        None,
        TestIsolation::Codeunit,
        None,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut Vec::new(),
        None,
    )
}

const SRC: &str = r#"
table 50100 "Caption Count Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Amount; Decimal) { Caption = 'Amount'; CaptionClass = CountCaption(); }
    }
    keys { key(PK; Id) { } }
    procedure CountCaption(): Text[80]
    var Log: Codeunit "Caption Count Log";
    begin
        Log.Hit(Id);
        exit('3,Row ' + Format(Id));
    end;
}
codeunit 50101 "Caption Count Log"
{
    SingleInstance = true;
    var EmptyRuns: Integer;
    procedure Hit(RowId: Integer) begin if RowId = 0 then EmptyRuns += 1; end;
    procedure TakeEmpty() Result: Integer begin Result := EmptyRuns; EmptyRuns := 0; end;
}
page 50102 "Caption Count Card"
{
    PageType = Card; SourceTable = "Caption Count Row";
    layout { area(Content) { field(Id; Rec.Id) { } field(Amount; Rec.Amount) { } } }
}
table 50103 "Plain Caption Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Amount; Decimal) { CaptionClass = PlainCaption(); }
    }
    keys { key(PK; Id) { } }
    procedure PlainCaption(): Text[80]
    begin
        exit('3,Row ' + Format(Id));
    end;
}
page 50104 "Plain Caption Card"
{
    PageType = Card; SourceTable = "Plain Caption Row";
    layout { area(Content) { field(Id; Rec.Id) { } field(Amount; Rec.Amount) { } } }
}
codeunit 50105 "Caption Count Tests"
{
    Subtype = Test;
    var Log: Codeunit "Caption Count Log";

    local procedure OpenCount(): Integer
    var R: Record "Caption Count Row"; P: TestPage "Caption Count Card";
    begin
        if R.IsEmpty() then begin R.Id := 1; R.Insert(); end;
        Log.TakeEmpty();
        P.OpenEdit();
        P.Close();
        exit(Log.TakeEmpty());
    end;

    [Test]
    procedure AssertsTheRuntimeCount()
    begin
        if OpenCount() <> 20 then Error('not 20');
    end;

    [Test]
    procedure AssertsBcsCount()
    begin
        if OpenCount() <> 8 then Error('not 8');
    end;

    [Test]
    procedure AssertsOnlyThatTheyRan()
    begin
        if OpenCount() = 0 then Error('none');
    end;

    [Test]
    procedure ReadsACaption()
    var R: Record "Plain Caption Row"; P: TestPage "Plain Caption Card";
    begin
        R.Id := 1;
        R.Insert();
        P.OpenView();
        if P.Amount.Caption() <> 'Row 1' then Error('caption %1', P.Amount.Caption());
    end;
}
"#;

fn outcome<'a>(outs: &'a [(String, Outcome)], name: &str) -> &'a Outcome {
    &outs.iter().find(|(n, _)| n == name).unwrap().1
}

#[test]
fn a_test_whose_outcome_depends_on_the_empty_record_count_is_refused() {
    let outs = run(SRC, "Caption Count Tests");
    for name in ["AssertsTheRuntimeCount", "AssertsBcsCount"] {
        match outcome(&outs, name) {
            Outcome::NotRun(why) => assert!(
                why.contains("with an empty record")
                    && why.contains("5870")
                    && why.contains(if name == "AssertsBcsCount" {
                        "failed after such an open, with: not 8"
                    } else {
                        "changed when they ran once"
                    }),
                "{name}: {why}"
            ),
            other => panic!("{name}: {other:?}"),
        }
    }
}

#[test]
fn a_test_that_does_not_depend_on_the_count_still_runs() {
    let outs = run(SRC, "Caption Count Tests");
    assert_eq!(outcome(&outs, "AssertsOnlyThatTheyRan"), &Outcome::Pass);
    assert_eq!(outcome(&outs, "ReadsACaption"), &Outcome::Pass);
}
