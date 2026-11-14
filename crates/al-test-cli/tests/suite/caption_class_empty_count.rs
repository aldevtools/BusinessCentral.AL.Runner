//! How many times a page open runs the CaptionClass expressions of its source table with an empty record (al-runner issue
//! 5870, assumption page-open-caption-class-empty-record-count). BC gives no stable number: the corpus saw 240, 480 and 492
//! calls for the same Card open (record/captions/QdCaptionClassAreas.al:657) and asserts only that such runs
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

/// The shapes writer SX pinned (corpus branch al-runtime-rs-staging-sx, tests/al-language/record/captions/
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

/// al-runner issue 6987: an expression that fills a List local to the routine it calls (also handing it by `var` to an
/// integration event no subscriber listens to) leaves nothing behind, whatever the number of runs; a List the routine takes
/// from outside (a parameter, an assignment) may be shared and is still refused. The tables have three expression fields,
/// so the empty-record count is not the pinned one.
const LOCAL_LIST: &str = r#"
table 50300 "Ll Hierarchy"
{
    fields { field(1; Code; Code[10]) { } }
    keys { key(PK; Code) { } }
    var Kept: List of [Text];
    procedure LevelCaption(ForLevel: Integer): Text
    var Captions: List of [Text];
    begin
        Captions.AddRange('One', 'Two', 'Three');
        OnAfterLevelCaptions(Captions);
        if (ForLevel > 0) and (ForLevel <= Captions.Count) then
            exit('3,' + Captions.Get(ForLevel));
    end;
    procedure SharedCaption(ForLevel: Integer): Text
    var Captions: List of [Text];
    begin
        Captions := Kept;
        Captions.Add('x');
        exit('3,' + Format(Captions.Count));
    end;
    [IntegrationEvent(false, false)]
    local procedure OnAfterLevelCaptions(var Captions: List of [Text]) begin end;
}
table 50301 "Ll Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; L1; Code[20]) { CaptionClass = H.LevelCaption(1); }
        field(3; L2; Code[20]) { CaptionClass = H.LevelCaption(2); }
        field(4; L3; Code[20]) { CaptionClass = H.LevelCaption(3); }
    }
    keys { key(PK; Id) { } }
    var H: Record "Ll Hierarchy";
}
page 50302 "Ll Card"
{
    PageType = Card; SourceTable = "Ll Row";
    layout { area(Content) { field(Id; Rec.Id) { } field(L2; Rec.L2) { } } }
}
table 50303 "Ll Shared Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; L1; Code[20]) { CaptionClass = H.SharedCaption(1); }
        field(3; L2; Code[20]) { CaptionClass = H.SharedCaption(2); }
        field(4; L3; Code[20]) { CaptionClass = H.SharedCaption(3); }
    }
    keys { key(PK; Id) { } }
    var H: Record "Ll Hierarchy";
}
page 50304 "Ll Shared Card"
{
    PageType = Card; SourceTable = "Ll Shared Row";
    layout { area(Content) { field(Id; Rec.Id) { } field(L2; Rec.L2) { } } }
}
codeunit 50305 "Ll Tests"
{
    Subtype = Test;
    [Test]
    procedure LocalList()
    var R: Record "Ll Row"; P: TestPage "Ll Card";
    begin
        R.Id := 1;
        R.Insert();
        P.OpenView();
        if P.L2.Caption() <> 'Two' then Error('caption %1', P.L2.Caption());
    end;
    [Test]
    procedure SharedList()
    var R: Record "Ll Shared Row"; P: TestPage "Ll Shared Card";
    begin
        R.Id := 1;
        R.Insert();
        P.OpenView();
    end;
}
"#;

#[test]
fn a_caption_expression_that_only_fills_a_local_list_still_runs() {
    let outs = run(LOCAL_LIST, "Ll Tests");
    assert_eq!(outcome(&outs, "LocalList"), &Outcome::Pass);
    match outcome(&outs, "SharedList") {
        Outcome::NotRun(why) => assert!(why.contains("List.Add") && why.contains("5870"), "{why}"),
        other => panic!("SharedList: {other:?}"),
    }
}

/// al-runner issues 7424, 7467 and 7391 (record/captions/CaptionClassStateFreeCalls.al): an expression that calls
/// UserSecurityId, RecordRef.GetTable on a local RecordRef, or reaches Message or Error without taking that branch with the
/// empty record runs in an open whose count is not pinned (a List with three expression fields). A Message that a run of that
/// open raises (an empty-record run or the row run) refuses; an error of an empty-record run fails the open; a GetTable on a
/// global RecordRef keeps main's refusal.
const STATE_FREE: &str = r#"
table 50190 "Sf Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; A; Decimal) { CaptionClass = UserCc(); }
        field(3; B; Decimal) { CaptionClass = RefCc(); }
        field(4; C; Decimal) { CaptionClass = BranchCc(); }
    }
    keys { key(PK; Id) { } }
    local procedure UserCc(): Text
    begin
        if IsNullGuid(UserSecurityId()) then exit('3,None');
        exit('3,User');
    end;
    local procedure RefCc(): Text
    var RecRef: RecordRef;
    begin
        RecRef.GetTable(Rec);
        exit('3,Ref');
    end;
    local procedure BranchCc(): Text
    begin
        if Id = 7 then Message('taken');
        if Id = 9 then Error('taken error');
        exit('3,Branch');
    end;
}
table 50191 "Sf Message Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; A; Decimal) { CaptionClass = MsgCc(); }
        field(3; B; Decimal) { CaptionClass = MsgCc(); }
        field(4; C; Decimal) { CaptionClass = ErrCc(); }
    }
    keys { key(PK; Id) { } }
    local procedure MsgCc(): Text
    begin
        if Id = 0 then Message('empty');
        exit('3,M');
    end;
    local procedure ErrCc(): Text
    begin
        exit('3,E');
    end;
}
table 50192 "Sf Error Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; A; Decimal) { CaptionClass = ErrCc(); }
        field(3; B; Decimal) { CaptionClass = ErrCc(); }
        field(4; C; Decimal) { CaptionClass = PlainCc(); }
    }
    keys { key(PK; Id) { } }
    local procedure ErrCc(): Text
    begin
        if Id = 0 then Error('empty error');
        exit('3,E');
    end;
    local procedure PlainCc(): Text
    begin
        exit('3,P');
    end;
}
table 50193 "Sf Global Ref Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; A; Decimal) { CaptionClass = RefCc(); }
        field(3; B; Decimal) { CaptionClass = PlainCc(); }
        field(4; C; Decimal) { CaptionClass = '3,C'; }
    }
    keys { key(PK; Id) { } }
    var Kept: Codeunit "Sf Holder";
    local procedure RefCc(): Text
    begin
        Kept.Take(Rec);
        exit('3,R');
    end;
    local procedure PlainCc(): Text
    begin
        exit('3,P');
    end;
}
codeunit 50194 "Sf Holder"
{
    var RecRef: RecordRef;
    procedure Take(var Row: Record "Sf Global Ref Row")
    begin
        RecRef.GetTable(Row);
    end;
}
page 50190 "Sf List" { PageType = List; SourceTable = "Sf Row"; layout { area(Content) { repeater(R) { field(Id; Rec.Id) { } field(A; Rec.A) { } field(B; Rec.B) { } field(C; Rec.C) { } } } } }
page 50194 "Sf Card" { PageType = Card; SourceTable = "Sf Row"; layout { area(Content) { field(Id; Rec.Id) { } field(C; Rec.C) { } } } }
page 50191 "Sf Message List" { PageType = List; SourceTable = "Sf Message Row"; layout { area(Content) { repeater(R) { field(Id; Rec.Id) { } field(A; Rec.A) { } } } } }
page 50192 "Sf Error List" { PageType = List; SourceTable = "Sf Error Row"; layout { area(Content) { repeater(R) { field(Id; Rec.Id) { } field(A; Rec.A) { } } } } }
page 50193 "Sf Global Ref List" { PageType = List; SourceTable = "Sf Global Ref Row"; layout { area(Content) { repeater(R) { field(Id; Rec.Id) { } field(A; Rec.A) { } } } } }
codeunit 50195 "Sf Tests"
{
    Subtype = Test;
    [Test]
    procedure StateFree()
    var R: Record "Sf Row"; P: TestPage "Sf List";
    begin
        R.Id := 1;
        R.Insert();
        P.OpenView();
        if P.A.Caption() <> 'User' then Error('A %1', P.A.Caption());
        if P.C.Caption() <> 'Branch' then Error('C %1', P.C.Caption());
    end;
    [Test]
    [HandlerFunctions('Msg')]
    procedure MessageInRowRun()
    var R: Record "Sf Row"; P: TestPage "Sf Card";
    begin
        R.DeleteAll();
        R.Id := 7;
        R.Insert();
        P.OpenEdit();
    end;
    [Test]
    [HandlerFunctions('Msg')]
    procedure MessageInEmptyRun()
    var R: Record "Sf Message Row"; P: TestPage "Sf Message List";
    begin
        R.Id := 1;
        R.Insert();
        P.OpenView();
    end;
    [Test]
    procedure ErrorInEmptyRun()
    var R: Record "Sf Error Row"; P: TestPage "Sf Error List";
    begin
        R.Id := 1;
        R.Insert();
        asserterror P.OpenView();
    end;
    [Test]
    procedure GlobalRecordRef()
    var R: Record "Sf Global Ref Row"; P: TestPage "Sf Global Ref List";
    begin
        R.Id := 1;
        R.Insert();
        P.OpenView();
    end;
    [MessageHandler]
    procedure Msg(Text: Text[1024])
    begin
    end;
}
"#;

#[test]
fn state_free_calls_in_a_caption_expression_run_and_a_raised_dialog_or_error_refuses() {
    let outs = run(STATE_FREE, "Sf Tests");
    assert_eq!(outcome(&outs, "StateFree"), &Outcome::Pass);
    match outcome(&outs, "MessageInEmptyRun") {
        Outcome::NotRun(why) => assert!(
            why.contains("Message raised by a CaptionClass expression on a page"),
            "{why}"
        ),
        other => panic!("MessageInEmptyRun: {other:?}"),
    }
    match outcome(&outs, "MessageInRowRun") {
        Outcome::NotRun(why) => assert!(
            why.contains("Message raised by a CaptionClass expression on a page"),
            "{why}"
        ),
        other => panic!("MessageInRowRun: {other:?}"),
    }
    // An error ends the open at the first run whatever the number of runs: the open fails, as on main.
    assert_eq!(outcome(&outs, "ErrorInEmptyRun"), &Outcome::Pass);
    match outcome(&outs, "GlobalRecordRef") {
        Outcome::NotRun(why) => assert!(why.contains("RecordRef.GetTable") && why.contains("5870"), "{why}"),
        other => panic!("GlobalRecordRef: {other:?}"),
    }
}
