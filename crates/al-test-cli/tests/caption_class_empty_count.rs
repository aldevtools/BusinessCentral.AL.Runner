//! How many times a page open runs the CaptionClass expressions of its source table with an empty record (al-runner issue
//! 5870, assumption page-open-caption-class-empty-record-count). BC gives no stable number: the corpus saw 240, 480 and 492
//! calls for the same Card open (record/lt-qd-captionclass-areas/QdCaptionClassAreas.al:657) and asserts only that such runs
//! happen and give the same count per field and per open (:1227 SF_Count_CardOpenViewFiveTimes); the issue's reporter saw 8
//! per open on one BC 29 container where al-runner ran 20. Writer SX pinned the count for a Card or List with one or two
//! expression fields (the last test here). For any other shape the number can only matter when the expression leaves something
//! behind (the tables here have three expression fields). A page open whose expression writes the database (also through an event subscriber) is refused. An expression
//! that leaves nothing behind (a caption built from the row, a read of a setup record into a variable of the record) still
//! runs. A count kept in a SingleInstance codeunit is kept, not pinned: refusing it costs 33 baseline cases of codeunit 86504.

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
table 50106 "Caption Setup"
{
    fields { field(1; Code; Code[10]) { } field(2; Prefix; Text[20]) { } }
    keys { key(PK; Code) { } }
}
table 50107 "Setup Caption Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Amount; Decimal) { CaptionClass = SetupCaption(); }
    }
    keys { key(PK; Id) { } }
    var Setup: Record "Caption Setup"; SetupRead: Boolean;
    procedure SetupCaption(): Text[80]
    begin
        if not SetupRead then begin if Setup.Get() then; SetupRead := true; end;
        OnSetupCaption(Id);
        exit('3,' + Setup.Prefix + Format(Id));
    end;
    [IntegrationEvent(false, false)]
    local procedure OnSetupCaption(RowId: Integer) begin end;
}
page 50108 "Setup Caption Card"
{
    PageType = Card; SourceTable = "Setup Caption Row";
    layout { area(Content) { field(Id; Rec.Id) { } field(Amount; Rec.Amount) { } } }
}
codeunit 50109 "Setup Caption Listener"
{
    var Seen: Integer;
    [EventSubscriber(ObjectType::Table, Database::"Setup Caption Row", 'OnSetupCaption', '', false, false)]
    local procedure Count(RowId: Integer) begin Seen += 1; end;
}
table 50110 "Caption Run"
{
    fields { field(1; No; Integer) { AutoIncrement = true; } field(2; RowId; Integer) { } }
    keys { key(PK; No) { } }
}
table 50111 "Logged Caption Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Amount; Decimal) { CaptionClass = LoggedCaption(); }
        field(3; Quantity; Decimal) { CaptionClass = LoggedCaption(); }
        field(4; Price; Decimal) { CaptionClass = LoggedCaption(); }
    }
    keys { key(PK; Id) { } }
    procedure LoggedCaption(): Text[80]
    var Run: Record "Caption Run";
    begin
        Run.RowId := Id;
        Run.Insert();
        exit('3,Row');
    end;
}
page 50112 "Logged Caption Card"
{
    PageType = Card; SourceTable = "Logged Caption Row";
    layout { area(Content) { field(Id; Rec.Id) { } field(Amount; Rec.Amount) { } } }
}
table 50113 "Event Caption Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Amount; Decimal) { CaptionClass = EventCaption(); }
        field(3; Quantity; Decimal) { CaptionClass = EventCaption(); }
        field(4; Price; Decimal) { CaptionClass = EventCaption(); }
    }
    keys { key(PK; Id) { } }
    procedure EventCaption(): Text[80]
    begin
        OnEventCaption(Id);
        exit('3,Row');
    end;
    [IntegrationEvent(false, false)]
    local procedure OnEventCaption(RowId: Integer) begin end;
}
page 50114 "Event Caption Card"
{
    PageType = Card; SourceTable = "Event Caption Row";
    layout { area(Content) { field(Id; Rec.Id) { } field(Amount; Rec.Amount) { } } }
}
codeunit 50115 "Event Caption Listener"
{
    [EventSubscriber(ObjectType::Table, Database::"Event Caption Row", 'OnEventCaption', '', false, false)]
    local procedure Count(RowId: Integer)
    var Run: Record "Caption Run";
    begin
        Run.RowId := RowId;
        Run.Insert();
    end;
}
codeunit 50105 "Caption Count Tests"
{
    Subtype = Test;
    var Log: Codeunit "Caption Count Log";

    [Test]
    procedure CountsInASingleInstanceCodeunit()
    var R: Record "Caption Count Row"; P: TestPage "Caption Count Card";
    begin
        R.Id := 1;
        R.Insert();
        Log.TakeEmpty();
        P.OpenEdit();
        P.Close();
        if Log.TakeEmpty() = 0 then Error('none');
    end;

    [Test]
    procedure CountsInATable()
    var R: Record "Logged Caption Row"; Run: Record "Caption Run"; P: TestPage "Logged Caption Card";
    begin
        R.Id := 1;
        R.Insert();
        P.OpenEdit();
        P.Close();
        Run.SetRange(RowId, 0);
        if Run.Count() <> 8 then Error('not 8: %1', Run.Count());
    end;

    [Test]
    procedure CountsThroughASubscriber()
    var R: Record "Event Caption Row"; Run: Record "Caption Run"; P: TestPage "Event Caption Card";
    begin
        R.Id := 1;
        R.Insert();
        P.OpenView();
        Run.SetRange(RowId, 0);
        if Run.IsEmpty() then Error('none');
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

    [Test]
    procedure ReadsASetupCaption()
    var S: Record "Caption Setup"; R: Record "Setup Caption Row"; P: TestPage "Setup Caption Card";
    begin
        S.Prefix := 'No. ';
        S.Insert();
        R.Id := 1;
        R.Insert();
        P.OpenView();
        if P.Amount.Caption() <> 'No. 1' then Error('caption %1', P.Amount.Caption());
    end;
}
"#;

fn outcome<'a>(outs: &'a [(String, Outcome)], name: &str) -> &'a Outcome {
    &outs.iter().find(|(n, _)| n == name).unwrap().1
}

#[test]
fn a_page_open_whose_caption_expression_writes_the_database_is_refused() {
    let outs = run(SRC, "Caption Count Tests");
    for (name, what) in [
        ("CountsInATable", "Logged Caption Row"),
        ("CountsThroughASubscriber", "Event Caption Row"),
    ] {
        match outcome(&outs, name) {
            Outcome::NotRun(why) => assert!(
                why.contains("CaptionClass expression")
                    && why.contains(what)
                    && why.contains("Record.Insert")
                    && why.contains("5870"),
                "{name}: {why}"
            ),
            other => panic!("{name}: {other:?}"),
        }
    }
}

#[test]
fn a_page_open_whose_caption_expression_leaves_nothing_behind_still_runs() {
    let outs = run(SRC, "Caption Count Tests");
    assert_eq!(outcome(&outs, "ReadsACaption"), &Outcome::Pass);
    assert_eq!(outcome(&outs, "ReadsASetupCaption"), &Outcome::Pass);
}

#[test]
fn a_count_in_a_single_instance_codeunit_still_runs() {
    // A Card with one expression field: the count is the pinned one (see the last test). In other shapes such a count is kept,
    // not pinned: the corpus keeps its count in a SingleInstance log (QdCaptionClassAreas.al:1227).
    let outs = run(SRC, "Caption Count Tests");
    assert_eq!(outcome(&outs, "CountsInASingleInstanceCodeunit"), &Outcome::Pass);
}

/// The shapes writer SX pinned (corpus branch al-runtime-rs-staging-sx, tests/al-language/record/lt-sx-captionclass-count/
/// SxCaptionClassCount.al:190-369, not on master yet): a Card or List opened with OpenView or OpenEdit over a table with one
/// or two CaptionClass expression fields (the same expression), three cycles of open, GoToRecord and Close.
const SX: &str = r#"
table 50200 "Sx One"
{
    fields { field(1; Id; Integer) { } field(2; Amount; Decimal) { CaptionClass = Cc(); } }
    keys { key(PK; Id) { } }
    procedure Cc(): Text[80]
    var Log: Codeunit "Sx Log";
    begin Log.Hit(Id); exit('3,Row ' + Format(Id)); end;
}
table 50201 "Sx Two"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Amount; Decimal) { CaptionClass = Cc(); }
        field(3; Quantity; Decimal) { CaptionClass = Cc(); }
    }
    keys { key(PK; Id) { } }
    procedure Cc(): Text[80]
    var Log: Codeunit "Sx Log";
    begin Log.Hit(Id); exit('3,Row ' + Format(Id)); end;
}
codeunit 50202 "Sx Log"
{
    SingleInstance = true;
    var EmptyRuns: Integer; RowRuns: Integer;
    procedure Hit(RowId: Integer) begin if RowId = 0 then EmptyRuns += 1 else RowRuns += 1; end;
    procedure Take() Result: Text
    begin
        Result := 'empty=' + Format(EmptyRuns) + ' rows=' + Format(RowRuns);
        EmptyRuns := 0;
        RowRuns := 0;
    end;
}
page 50203 "Sx Card One" { PageType = Card; SourceTable = "Sx One"; layout { area(Content) { field(Id; Rec.Id) { } field(Amount; Rec.Amount) { } } } }
page 50204 "Sx Card Two" { PageType = Card; SourceTable = "Sx Two"; layout { area(Content) { field(Id; Rec.Id) { } field(Amount; Rec.Amount) { } field(Quantity; Rec.Quantity) { } } } }
page 50205 "Sx List One" { PageType = List; SourceTable = "Sx One"; layout { area(Content) { repeater(R) { field(Id; Rec.Id) { } field(Amount; Rec.Amount) { } } } } }
page 50206 "Sx List Two" { PageType = List; SourceTable = "Sx Two"; layout { area(Content) { repeater(R) { field(Id; Rec.Id) { } field(Amount; Rec.Amount) { } field(Quantity; Rec.Quantity) { } } } } }
codeunit 50207 "Sx Tests"
{
    Subtype = Test;
    var Log: Codeunit "Sx Log";

    local procedure Seed()
    var One: Record "Sx One"; Two: Record "Sx Two"; i: Integer;
    begin
        if not One.IsEmpty() then exit;
        for i := 1 to 3 do begin
            One.Id := i; One.Insert();
            Two.Id := i; Two.Insert();
        end;
    end;

    local procedure Check(Got: Text; Want: Text)
    begin
        if Got <> Want then Error('got %1', Got);
    end;

    [Test]
    procedure CardOne()
    var P: TestPage "Sx Card One"; R: Record "Sx One"; Got: Text; i: Integer;
    begin
        Seed();
        R.Get(2);
        for i := 1 to 3 do begin
            Log.Take();
            P.OpenEdit(); Got += 'open ' + Log.Take();
            P.GoToRecord(R); Got += ' goto ' + Log.Take();
            P.Close(); Got += ' close ' + Log.Take() + '|';
        end;
        Check(Got, 'open empty=8 rows=1 goto empty=0 rows=1 close empty=0 rows=0|open empty=8 rows=1 goto empty=0 rows=1 close empty=0 rows=0|open empty=8 rows=1 goto empty=0 rows=1 close empty=0 rows=0|');
    end;

    [Test]
    procedure CardTwo()
    var P: TestPage "Sx Card Two"; R: Record "Sx Two"; Got: Text;
    begin
        Seed();
        R.Get(2);
        Log.Take();
        P.OpenView(); Got += 'open ' + Log.Take();
        P.GoToRecord(R); Got += ' goto ' + Log.Take();
        P.Close(); Got += ' close ' + Log.Take();
        Check(Got, 'open empty=18 rows=2 goto empty=0 rows=2 close empty=0 rows=0');
    end;

    [Test]
    procedure ListOne()
    var P: TestPage "Sx List One"; R: Record "Sx One"; Got: Text;
    begin
        Seed();
        R.Get(2);
        Log.Take();
        P.OpenView(); Got += 'open ' + Log.Take();
        P.GoToRecord(R); Got += ' goto ' + Log.Take();
        P.Close(); Got += ' close ' + Log.Take();
        Check(Got, 'open empty=7 rows=1 goto empty=0 rows=0 close empty=0 rows=0');
    end;

    [Test]
    procedure ListTwo()
    var P: TestPage "Sx List Two"; R: Record "Sx Two"; Got: Text;
    begin
        Seed();
        R.Get(2);
        Log.Take();
        P.OpenEdit(); Got += 'open ' + Log.Take();
        P.GoToRecord(R); Got += ' goto ' + Log.Take();
        P.Close(); Got += ' close ' + Log.Take();
        Check(Got, 'open empty=16 rows=2 goto empty=0 rows=0 close empty=0 rows=0');
    end;
}
"#;

#[test]
fn the_shapes_writer_sx_pinned_run_the_counts_bc_gave() {
    let outs = run(SX, "Sx Tests");
    for name in ["CardOne", "CardTwo", "ListOne", "ListTwo"] {
        assert_eq!(outcome(&outs, name), &Outcome::Pass, "{name}");
    }
}
