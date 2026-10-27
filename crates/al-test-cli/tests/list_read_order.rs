//! The read order of a list that serves its rows from OnFindRecord and OnNextRecord and has OnAfterGetRecord but no
//! OnAfterGetCurrRecord (pages/lt-f3-pages/F3ListTests.al, F3PartTests.al). AL source → compile → run as a test codeunit.
//! Each test names the corpus test or the Microsoft test it mirrors.

use al_test_cli::report::Outcome;

// A part shaped like Base Application's "Base Calendar Entries Subform" (page 7604): a temporary ListPart whose rows come
// from a position the page keeps itself (`Pos`, Base Application's DateRec of the virtual Date table, which has a row on
// every day). OnFindRecord finds the row at `Pos` and inserts it when missing, OnNextRecord moves `Pos`, OnAfterGetRecord
// puts `Pos` back on the row read. The host card refills the part from its OnAfterGetRecord (`SetSource`: DeleteAll, then
// CurrPage.Update), as "Base Calendar Card" (page 7600) does.
const OBJECTS: &str = r#"
table 62400 "LRO Day"
{
    fields { field(1; "Day No."; Integer) { } field(2; Src; Code[20]) { } field(3; Descr; Text[50]) { } }
    keys { key(PK; "Day No.") { } }
}
table 62401 "LRO Head"
{
    fields { field(1; Code; Code[20]) { } }
    keys { key(PK; Code) { } }
}
codeunit 62402 "LRO Log"
{
    SingleInstance = true;
    var Trail: Text;
    procedure Reset() begin Trail := ''; end;
    procedure Add(T: Text) begin Trail += T + ';'; end;
    procedure Trace(): Text begin exit(Trail); end;
}
page 62403 "LRO Day Part"
{
    PageType = ListPart;
    SourceTable = "LRO Day";
    SourceTableTemporary = true;
    InsertAllowed = false;
    DeleteAllowed = false;
    layout { area(Content) { repeater(Rows) { field(DayCtl; Rec."Day No.") { } field(DescrCtl; Rec.Descr) { } } } }
    var
        Pos: Integer;
        CurSrc: Code[20];
        Log: Codeunit "LRO Log";

    trigger OnFindRecord(Which: Text): Boolean
    begin
        if Pos = 0 then
            Pos := 10;
        Log.Add('F' + Which + '@' + Format(Pos));
        if CurSrc = '' then
            exit(false);
        exit(FindOrInsert());
    end;

    trigger OnNextRecord(Steps: Integer): Integer
    begin
        if Pos + Steps < 1 then
            Steps := 1 - Pos;
        Pos += Steps;
        Log.Add('N' + Format(Steps) + '@' + Format(Pos));
        if (CurSrc = '') or (Steps = 0) then
            exit(0);
        FindOrInsert();
        exit(Steps);
    end;

    trigger OnAfterGetRecord()
    begin
        Pos := Rec."Day No.";
        Log.Add('R' + Format(Rec."Day No.") + '/' + Rec.Descr);
    end;

    local procedure FindOrInsert(): Boolean
    var
        Found: Boolean;
    begin
        Rec.Reset();
        Rec.SetRange("Day No.", Pos);
        Found := Rec.FindFirst();
        Rec.Reset();
        if Found then
            exit(true);
        Rec.Init();
        Rec."Day No." := Pos;
        Rec.Src := CurSrc;
        if Pos = 10 then
            Rec.Descr := CurSrc;
        exit(Rec.Insert());
    end;

    procedure SetSource(Head: Record "LRO Head")
    begin
        Log.Add('Src(' + Head.Code + ')');
        Rec.DeleteAll();
        CurSrc := Head.Code;
        CurrPage.Update();
    end;
}
page 62405 "LRO Pos List"
{
    PageType = List;
    SourceTable = "LRO Day";
    layout { area(Content) { repeater(Rows) { field(DayCtl; Rec."Day No.") { } } } }
    var Log: Codeunit "LRO Log";
    trigger OnOpenPage() begin Rec."Day No." := 4; Rec.SetPosition(Rec.GetPosition()); Rec.Get(4); end;
    trigger OnFindRecord(Which: Text): Boolean var Found: Boolean; begin Found := Rec.Find(Which); Log.Add('F' + Which + '=' + Format(Found)); exit(Found); end;
    trigger OnNextRecord(Steps: Integer): Integer var Moved: Integer; begin Moved := Rec.Next(Steps); Log.Add('N' + Format(Steps) + '=' + Format(Moved)); exit(Moved); end;
    trigger OnAfterGetRecord() begin Log.Add('R' + Format(Rec."Day No.")); end;
}
page 62406 "LRO Pos Part"
{
    PageType = ListPart;
    SourceTable = "LRO Day";
    layout { area(Content) { repeater(Rows) { field(DayCtl; Rec."Day No.") { } } } }
    var Log: Codeunit "LRO Log";
    trigger OnFindRecord(Which: Text): Boolean begin if Rec.Get(4) then; Log.Add('F' + Which); exit(true); end;
    trigger OnNextRecord(Steps: Integer): Integer var Moved: Integer; begin Moved := Rec.Next(Steps); Log.Add('N' + Format(Steps) + '=' + Format(Moved)); exit(Moved); end;
    trigger OnAfterGetRecord() begin Log.Add('R' + Format(Rec."Day No.")); end;
}
page 62407 "LRO Pos Host"
{
    PageType = Card;
    SourceTable = "LRO Head";
    layout { area(Content) { field(CodeCtl; Rec.Code) { } part(Days; "LRO Pos Part") { } } }
}
page 62408 "LRO Param Card"
{
    PageType = Card;
    SourceTable = "LRO Day";
    SourceTableTemporary = true;
    layout { area(Content) { field(DayCtl; Rec."Day No.") { } field(DescrCtl; Rec.Descr) { } } }
    actions { area(Processing) { action(Show) { trigger OnAction() begin Error('action sees <%1>', Rec.Descr); end; } } }
    var Log: Codeunit "LRO Log";
    trigger OnOpenPage() begin Rec.Descr := 'in memory'; end;
    trigger OnFindRecord(Which: Text): Boolean begin Log.Add('F' + Which); exit(true); end;
}
page 62404 "LRO Head Card"
{
    PageType = Card;
    SourceTable = "LRO Head";
    layout { area(Content) { field(CodeCtl; Rec.Code) { } part(Days; "LRO Day Part") { } } }
    trigger OnAfterGetRecord()
    begin
        CurrPage.Days.Page.SetSource(Rec);
    end;
}
"#;

fn outcomes(tests: &str) -> Vec<(String, Outcome)> {
    let src = format!("{OBJECTS}\ncodeunit 62410 T {{ Subtype = Test; {tests} }}");
    al_testkit::outcomes_in(&src, "T")
}

fn passes(tests: &str) {
    let o = outcomes(tests);
    assert!(!o.is_empty());
    for (n, r) in &o {
        assert_eq!(r, &Outcome::Pass, "{n}");
    }
}

const HELPERS: &str = "procedure Check(Expected: Text; Actual: Text) begin if Expected <> Actual then Error('expected <%1> got <%2>', Expected, Actual); end;";

// The shape of Microsoft's Tests-ERM "ERM Base Calendar" (codeunit 134233) BaseCalendarEntriesShowCorrespondingRecords: a part
// that keeps its position in a global its OnAfterGetRecord sets, refilled by the host's OnAfterGetRecord. Its open read from day 10
// fills the first window and steps back over every row above it, so the position ends on day 1 (al-runner issue 5887, corpus
// branch al-runtime-rs-staging-sf misc/lt-probes-needed-6/PnF06FindListMoves.al SF_DayHost_RunWithFilter_OpensPart; for a page of
// its own pages/lt-ai-list-order/AiListOrderTests.al:1090). After the host moves, the refill opens on day 1, which shows no Descr.
#[test]
fn a_part_refilled_by_its_host_opens_on_the_row_its_step_back_reached_after_the_host_moves() {
    passes(&format!(
        "{HELPERS}
        [Test]
        procedure HostMoves() var H: Record \"LRO Head\"; C: TestPage \"LRO Head Card\"; Log: Codeunit \"LRO Log\"; begin
            H.DeleteAll(); H.Code := 'A'; H.Insert(); H.Code := 'B'; H.Insert();
            Log.Reset();
            C.OpenView();
            Check('A', C.Days.DescrCtl.Value());
            C.Next();
            if C.Days.DescrCtl.Value() <> '' then
                Error('after next: <%1> trace %2', C.Days.DescrCtl.Value(), Log.Trace());
            C.Close(); end;"
    ));
}

fn seed_days() -> &'static str {
    "procedure SeedDays() var D: Record \"LRO Day\"; I: Integer; begin D.DeleteAll(); for I := 1 to 6 do begin D.\"Day No.\" := I; D.Insert(); end; end;"
}

// pages/lt-ai-list-order/AiListOrderTests.al:777 OpenOnRow_SetPositionInOnOpenPage: a list whose OnOpenPage puts the cursor on
// row 4 of 6 reads on to the end, then back from row 3 to the first row, each row once, until OnNextRecord(-1) answers 0 (the
// corpus log loses its first entry, the OnFindRecord call: its first Add writes Id 0, which Dump leaves out); the
// cursor stays on row 4 and First() reads nothing.
#[test]
fn a_list_opened_on_a_later_row_reads_back_to_the_first_row() {
    passes(&format!(
        "{HELPERS} {}
        [Test]
        procedure OpenOnRow() var P: TestPage \"LRO Pos List\"; Log: Codeunit \"LRO Log\"; begin
            SeedDays(); Log.Reset();
            P.OpenView();
            Check('F=><=Yes;R4;N1=1;R5;N1=1;R6;N1=0;R4;N-1=-1;R3;N-1=-1;R2;N-1=-1;R1;N-1=0;', Log.Trace());
            Check('4', P.DayCtl.Value());
            Log.Reset();
            if not P.First() then Error('First');
            Check('', Log.Trace());
            Check('1', P.DayCtl.Value());
            P.Close(); end;",
        seed_days()
    ));
}

// A part in the same shape is not pinned (pages/lt-f3-pages/F3PartTests.al pins parts whose OnNextRecord(-1) answers 0): refused.
#[test]
fn a_part_with_rows_before_the_first_one_read_is_refused() {
    let o = outcomes(&format!(
        "{HELPERS} {}
        [Test]
        procedure PartOpen() var H: Record \"LRO Head\"; C: TestPage \"LRO Pos Host\"; begin
            SeedDays(); H.DeleteAll(); H.Code := 'A'; H.Insert();
            C.OpenView(); C.Close(); end;",
        seed_days()
    ));
    assert!(
        matches!(&o[0].1, Outcome::NotRun(r) if r.contains("rows before the first one read")),
        "{o:?}"
    );
}

// Microsoft's Tests-ERM "ERM Accounts" (codeunit 134020) AnalysisViewWithUpdate: "Analysis by Dimensions" (page 554), a card over a
// temporary table run with Page.Run(id, TempRec), sets Rec."Account Filter" in OnOpenPage without Modify; its ShowMatrix action
// must see it. The card is asked OnFindRecord('=') for the record as it holds it (pages/lt-be-card-onfindrecord/
// BeCardOnFindRecord.al:292 CardTempFind_PageRun_TempRec `F:=:1/passed/in memory`).
#[test]
fn a_card_run_with_a_record_keeps_what_on_open_page_set_on_rec() {
    let o = outcomes(&format!(
        "{HELPERS}
        [Test] [HandlerFunctions('H')]
        procedure Run() var D: Record \"LRO Day\" temporary; Log: Codeunit \"LRO Log\"; begin
            D.\"Day No.\" := 7; D.Insert(); Log.Reset();
            Page.Run(Page::\"LRO Param Card\", D); end;
        [PageHandler] procedure H(var C: TestPage \"LRO Param Card\") var Log: Codeunit \"LRO Log\"; begin
            Check('F=;', Log.Trace());
            Check('in memory', C.DescrCtl.Value());
            C.Show.Invoke(); end;"
    ));
    assert_eq!(o[0].1, Outcome::Fail("action sees <in memory>".into()), "{o:?}");
}
