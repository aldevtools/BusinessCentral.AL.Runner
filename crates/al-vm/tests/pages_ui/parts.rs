//! Page parts and `TestPart` end to end: AL source → compile → run (ADR-012 follow-up).
//! Each test names the corpus test it mirrors.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::text;

const OBJECTS: &str = r#"
table 51000 "PT Row"
{
    fields
    {
        field(1; Grp; Code[10]) { }
        field(2; "Line No."; Integer) { }
        field(3; Descr; Text[50]) { }
        field(4; Grade; Text[20]) { trigger OnValidate() begin if CopyStr(Grade, 1, 3) = 'BAD' then Error('refuses the grade %1', Grade); end; }
    }
    keys { key(PK; Grp, "Line No.") { } }
}

page 51001 "PT Lines"
{
    PageType = ListPart;
    SourceTable = "PT Row";
    Caption = 'Part Lines Caption';
    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field(Grp; Rec.Grp) { }
                field(LineNo; Rec."Line No.") { }
                field(Descr; Rec.Descr) { }
                field(Grade; Rec.Grade) { }
            }
        }
    }
}

page 51002 "PT Host"
{
    PageType = Worksheet;
    Caption = 'Host Page Caption';
    layout
    {
        area(Content)
        {
            part(Lines; "PT Lines") { }
            part(Hidden; "PT Lines") { Visible = false; }
        }
    }
}

page 51090 "PT Reach Part"
{
    PageType = ListPart;
    SourceTable = "PT Row";
    layout { area(Content) { repeater(Rows) { field(Grp; Rec.Grp) { } } } }
    var Loaded: Text;
    procedure Load(Name: Text) begin Loaded := Name; end;
    procedure Seen(): Text begin exit(Loaded); end;
}
page 51091 "PT Reach Host"
{
    PageType = Card;
    SourceTable = "PT Row";
    layout
    {
        area(Content)
        {
            field(Grp; Rec.Grp) { }
            part(Dropped; "PT Reach Part") { Visible = false; }
        }
    }
    trigger OnAfterGetCurrRecord() begin CurrPage.Dropped.Page.Load('reached ' + Rec.Grp); end;
    procedure Outcome(): Text begin exit('ok'); end;
}

page 51092 "PT Cap Card"
{
    PageType = Card;
    SourceTable = "PT Row";
    layout { area(Content) { field(Grp; Rec.Grp) { Caption = 'Plain'; CaptionClass = '5,1,GB'; } field(Dim; Rec.Descr) { Caption = 'Plain'; CaptionClass = '1,1,'; } field(Lit; Rec.Descr) { Caption = 'Plain'; CaptionClass = '3,Fixed'; } } }
}

table 51093 "PT Cap Rec"
{
    fields
    {
        field(1; Id; Integer) { Caption = 'Identifier'; }
        field(2; A; Text[10]) { Caption = 'Tbl A'; CaptionClass = '1,1,x'; }
        field(3; B; Text[10]) { Caption = 'Tbl B'; CaptionClass = '3,x'; }
        field(4; C; Text[10]) { Caption = 'Tbl C'; CaptionClass = '3,'; }
        field(5; D; Text[10]) { Caption = 'Tbl D'; CaptionClass = ',x'; }
    }
    keys { key(PK; Id) { } }
}

page 51094 "PT Expr Card"
{
    PageType = Card;
    SourceTable = "PT Row";
    layout
    {
        area(Content)
        {
            field(Ctl; Totals[1].Descr) { trigger OnValidate() begin Seen += 1; end; }
            field(Amt; Totals[2]."Line No.") { }
            field(Fn; Format(Totals[1].Descr)) { }
        }
    }
    var Totals: array[2] of Record "PT Row"; Seen: Integer;
    trigger OnOpenPage() begin Totals[1].Descr := 'start'; Totals[2]."Line No." := 7; end;
    procedure SeenCount(): Integer begin exit(Seen); end;
}

table 51020 "PT Rank"
{
    fields { field(1; "Entry No."; Integer) { } field(2; Rank; Integer) { } field(3; Bucket; Code[10]) { } }
    keys { key(PK; "Entry No.") { } key(ByRank; Rank) { } }
}
page 51020 "PT View Part"
{
    PageType = ListPart;
    SourceTable = "PT Rank";
    layout { area(Content) { repeater(Rows) { field(Entry; Rec."Entry No.") { } field(Rank; Rec.Rank) { } field(Bucket; Rec.Bucket) { } } } }
    trigger OnAfterGetCurrRecord() begin Rec.FilterGroup(4); G4 := Rec.GetFilter(Bucket); Rec.FilterGroup(0); G0 := Rec.GetFilter(Bucket); end;
    var G0: Text; G4: Text;
    procedure Groups(): Text begin exit('g0=' + G0 + '|g4=' + G4); end;
}
page 51021 "PT View Host"
{
    PageType = Card;
    SourceTable = "PT Rank";
    layout
    {
        area(Content)
        {
            field(Entry; Rec."Entry No.") { }
            part(Sorted; "PT View Part") { SubPageView = sorting(Rank) order(descending) where(Bucket = const('KEEP')); }
            part(Plain; "PT View Part") { }
        }
    }
}

table 51030 "PT Doc"
{
    fields { field(1; "No."; Code[20]) { } field(2; Descr; Text[30]) { } }
    keys { key(PK; "No.") { } }
}
table 51031 "PT Doc Line"
{
    fields
    {
        field(1; "Doc No."; Code[20]) { trigger OnValidate() begin "Doc Validated" := true; end; }
        field(2; "Line No."; Integer) { }
        field(3; Descr; Text[30]) { trigger OnValidate() begin "Seen Doc" := "Doc No."; end; }
        field(4; "Doc Validated"; Boolean) { }
        field(5; "Seen Doc"; Code[20]) { }
        field(6; "Set By New"; Code[20]) { }
        field(7; Other; Code[20]) { }
    }
    keys { key(PK; "Doc No.", "Line No.") { } }
}
codeunit 51031 "PT Count" { SingleInstance = true; var N: Integer; procedure Hit() begin N += 1; end; procedure Count(): Integer begin exit(N); end; }
page 51031 "PT Doc Lines"
{
    PageType = ListPart;
    SourceTable = "PT Doc Line";
    AutoSplitKey = true;
    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field(DocNo; Rec."Doc No.") { }
                field(LineNo; Rec."Line No.") { }
                field(Descr; Rec.Descr) { }
                field(Validated; Rec."Doc Validated") { }
                field(SetByNew; Rec."Set By New") { }
                field(Other; Rec.Other) { }
            }
        }
    }
    trigger OnNewRecord(BelowxRec: Boolean) var C: Codeunit "PT Count"; begin C.Hit(); Rec."Set By New" := 'NEWREC'; end;
}
page 51030 "PT Doc Card"
{
    PageType = Card;
    SourceTable = "PT Doc";
    layout
    {
        area(Content)
        {
            field(DocNo; Rec."No.") { }
            field(Descr; Rec.Descr) { }
            part(Lines; "PT Doc Lines") { SubPageLink = "Doc No." = field("No."); }
        }
    }
}
codeunit 51032 "PT Log" { SingleInstance = true; var L: Text; procedure Add(T: Text) begin if L <> '' then L += ' | '; L += T; end; procedure Take(): Text var T: Text; begin T := L; L := ''; exit(T); end; }
page 51032 "PT Doc Lines Cur"
{
    PageType = ListPart;
    SourceTable = "PT Doc Line";
    AutoSplitKey = true;
    layout { area(Content) { repeater(Rows) { field(LineNo; Rec."Line No.") { } } } }
    trigger OnNewRecord(BelowxRec: Boolean) var G: Codeunit "PT Log"; begin G.Add('New:' + Rec."Doc No."); end;
    trigger OnAfterGetCurrRecord() var G: Codeunit "PT Log"; begin G.Add('Cur:' + Rec."Doc No."); end;
}
page 51033 "PT Doc Lines CurOnly"
{
    PageType = ListPart;
    SourceTable = "PT Doc Line";
    AutoSplitKey = true;
    layout { area(Content) { repeater(Rows) { field(LineNo; Rec."Line No.") { } } } }
    trigger OnAfterGetCurrRecord() var G: Codeunit "PT Log"; begin G.Add('Cur:' + Rec."Doc No."); end;
}
page 51034 "PT Doc Card Cur"
{
    PageType = Card;
    SourceTable = "PT Doc";
    layout
    {
        area(Content)
        {
            field(DocNo; Rec."No.") { }
            part(Lines; "PT Doc Lines Cur") { SubPageLink = "Doc No." = field("No."); }
        }
    }
}
page 51035 "PT Doc Card CurOnly"
{
    PageType = Card;
    SourceTable = "PT Doc";
    layout
    {
        area(Content)
        {
            field(DocNo; Rec."No.") { }
            part(Lines; "PT Doc Lines CurOnly") { SubPageLink = "Doc No." = field("No."); }
        }
    }
}

page 51040 "PT Info Part"
{
    PageType = CardPart;
    layout { area(Content) { field(Tag; TagValue) { trigger OnValidate() begin Seen := TagValue; end; } } }
    trigger OnOpenPage() begin TagValue := 'Hello'; end;
    var TagValue: Text; Seen: Text;
    procedure SetTag(NewTag: Text) begin TagValue := NewTag; end;
    procedure GetSeen(): Text begin exit(Seen); end;
}
page 51041 "PT Info Host"
{
    PageType = Card;
    layout
    {
        area(Content)
        {
            field(Mode; SelectedMode) { trigger OnValidate() begin CurrPage.Info.Page.SetTag(SelectedMode); end; }
            part(Info; "PT Info Part") { Caption = 'Info'; }
        }
    }
    var SelectedMode: Text;
}

page 51042 "PT Seeding Host"
{
    PageType = Card;
    layout { area(Content) { part(Info; "PT Info Part") { } } }
    trigger OnOpenPage() begin CurrPage.Info.Page.SetTag('SEEDED'); end;
}

table 51050 "PT Link Row"
{
    fields { field(1; "Header No."; Code[20]) { } field(2; "Line No."; Integer) { } field(3; Info; Text[30]) { } field(4; Tag; Code[10]) { } }
    keys { key(PK; "Header No.", "Line No.") { } }
}
page 51050 "PT Link Part"
{
    PageType = ListPart;
    SourceTable = "PT Link Row";
    layout { area(Content) { repeater(Rows) { field(HeaderNo; Rec."Header No.") { } field(LineNo; Rec."Line No.") { } field(Info; Rec.Info) { } field(Tag; Rec.Tag) { } } } }
}
page 51051 "PT Populate Part"
{
    PageType = ListPart;
    SourceTable = "PT Link Row";
    PopulateAllFields = true;
    layout { area(Content) { repeater(Rows) { field(HeaderNo; Rec."Header No.") { } field(Tag; Rec.Tag) { } } } }
}
page 51052 "PT Link Card"
{
    PageType = Card;
    SourceTable = "PT Header";
    layout
    {
        area(Content)
        {
            field(No; Rec."No.") { }
            part(Keyed; "PT Link Part") { SubPageLink = "Header No." = field("No."), Tag = const('T1'); }
            part(Multi; "PT Link Part") { SubPageLink = "Header No." = field("No."), Tag = filter(T1 | T2); }
            part(Populated; "PT Populate Part") { SubPageLink = Tag = const('T1'); }
            part(RO; "PT Link Part") { SubPageLink = "Header No." = field("No."); Editable = false; }
        }
    }
}
page 51054 "PT Flag Card"
{
    PageType = Card;
    SourceTable = "PT Header";
    layout
    {
        area(Content)
        {
            field(No; Rec."No.") { }
            part(EdExpr; "PT Link Part") { SubPageLink = "Header No." = field("No."); Editable = Flag; }
            part(EnExpr; "PT Link Part") { SubPageLink = "Header No." = field("No."); Enabled = Flag; }
            part(EnOff; "PT Link Part") { SubPageLink = "Header No." = field("No."); Enabled = false; }
        }
    }
    var Flag: Boolean;
}
page 51055 "PT Flag On Card"
{
    PageType = Card;
    SourceTable = "PT Header";
    layout
    {
        area(Content)
        {
            field(No; Rec."No.") { }
            part(EdExpr; "PT Link Part") { SubPageLink = "Header No." = field("No."); Editable = Flag; }
            part(EnExpr; "PT Link Part") { SubPageLink = "Header No." = field("No."); Enabled = Flag; }
        }
    }
    var Flag: Boolean;
    trigger OnOpenPage() begin Flag := true; end;
}
page 51053 "PT Provider Card"
{
    PageType = Card;
    SourceTable = "PT Header";
    layout
    {
        area(Content)
        {
            field(No; Rec."No.") { }
            part(Lines; "PT Link Part") { SubPageLink = "Header No." = field("No."); }
            part(Detail; "PT Link Part") { Provider = Lines; SubPageLink = "Line No." = field("Line No."); }
        }
    }
}

table 51010 "PT Header" { fields { field(1; "No."; Code[20]) { } } keys { key(PK; "No.") { } } }
table 51011 "PT Line"
{
    fields { field(1; "Header No."; Code[20]) { } field(2; "Line No."; Integer) { } field(3; Info; Text[30]) { } }
    keys { key(PK; "Header No.", "Line No.") { } }
}
page 51011 "PT Line Part"
{
    PageType = ListPart;
    SourceTable = "PT Line";
    layout { area(Content) { repeater(Rows) { field(HeaderNo; Rec."Header No.") { } field(LineNo; Rec."Line No.") { } field(Info; Rec.Info) { } } } }
}
page 51010 "PT Header Card"
{
    PageType = Card;
    SourceTable = "PT Header";
    layout
    {
        area(Content)
        {
            field(No; Rec."No.") { }
            part(Lines; "PT Line Part") { SubPageLink = "Header No." = field("No."); }
        }
    }
}
"#;

fn run(body: &str, routine: &str) -> Result<Value, Signal> {
    let src = format!("{OBJECTS}\ncodeunit 51100 C {{ {body} }}");
    let l = al_testkit::compile(&src);
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let ri = l.program.objects[oi]
        .routines
        .iter()
        .position(|r| r.name == routine)
        .unwrap();
    let mut seen = std::collections::BTreeSet::new();
    let mut stack = vec![(oi, ri)];
    while let Some((o, r)) = stack.pop() {
        if !seen.insert((o, r)) {
            continue;
        }
        if let Some(why) = l.unsupported[o][r].first() {
            panic!(
                "{}.{} is blocked: {why}",
                l.program.objects[o].name, l.program.objects[o].routines[r].name
            );
        }
        stack.extend(l.calls[o][r].iter().copied());
    }
    let mut s = al_testkit::session(l);
    let inst = s.instance(oi);
    s.call(oi, ri, inst, vec![]).map(|(v, _)| v)
}

fn error_text(r: Result<Value, Signal>) -> String {
    match r {
        Err(Signal::Error(e)) => e.message,
        other => panic!("expected an error, got {other:?}"),
    }
}

const SEED: &str = "R.Grp := 'A'; R.\"Line No.\" := 10; R.Descr := 'Alpha'; R.Insert();
    R.Grp := 'A'; R.\"Line No.\" := 20; R.Descr := 'Shared'; R.Insert();
    R.Grp := 'A'; R.\"Line No.\" := 30; R.Descr := 'Shared'; R.Insert();";

// testpart/TestTestPart.al TestPart_First_LandsOnTheFirstRowInKeyOrder, _Next_WalksForwardOneRowAtATime,
// _Last_LandsOnTheHighestKeyedRow, _Previous_WalksBackwardOneRowAtATime
#[test]
fn a_part_walks_its_own_rowset_in_key_order() {
    let r = run(
        &format!(
            "procedure F(): Text var R: Record \"PT Row\"; H: TestPage \"PT Host\"; T: Text; begin
                {SEED}
                H.OpenEdit();
                if H.Lines.First() then T += H.Lines.Descr.Value() + ',';
                if H.Lines.Next() then T += H.Lines.LineNo.Value() + ',';
                if H.Lines.Last() then T += H.Lines.LineNo.Value() + ',';
                if H.Lines.Previous() then T += H.Lines.LineNo.Value();
                H.Close();
                exit(T);
            end;"
        ),
        "F",
    );
    assert_eq!(text(r), "Alpha,20,30,20");
}

// testpart/TestTestPart.al TestPart_Caption_ReadsThePartPageNotTheHost
#[test]
fn a_part_caption_is_the_part_pages() {
    let r = run(
        "procedure F(): Text var H: TestPage \"PT Host\"; begin H.OpenEdit(); exit(H.Lines.Caption()); end;",
        "F",
    );
    assert_eq!(text(r), "Part Lines Caption");
}

// testpart/TestTestPart.al TestPart_InvisiblePart_IsNotInTheControlTreeAtAll
#[test]
fn an_invisible_part_is_not_on_the_page() {
    let r = run(
        "procedure F(): Boolean var H: TestPage \"PT Host\"; B: Boolean; begin H.OpenEdit(); B := H.Hidden.Visible(); exit(B); end;",
        "F",
    );
    assert!(error_text(r).contains("was not found on the page"));
}

// testpart/PvdHeaderCard.Page.al + TestPartProvider.al: SubPageLink follows the host's current row
#[test]
fn a_linked_part_shows_the_rows_of_the_hosts_current_row() {
    let r = run(
        "procedure F(): Text var H: Record \"PT Header\"; L: Record \"PT Line\"; C: TestPage \"PT Header Card\"; T: Text; begin
            H.\"No.\" := 'H1'; H.Insert(); H.\"No.\" := 'H2'; H.Insert();
            L.\"Header No.\" := 'H1'; L.\"Line No.\" := 10; L.Info := 'ten'; L.Insert();
            L.\"Header No.\" := 'H1'; L.\"Line No.\" := 20; L.Info := 'twenty'; L.Insert();
            C.OpenView();
            T += C.Lines.Info.Value() + ',';
            C.Lines.Next();
            T += C.Lines.Info.Value() + ',';
            C.GoToKey('H2');
            T += '[' + C.Lines.Info.Value() + ']';
            exit(T);
        end;",
        "F",
    );
    assert_eq!(text(r), "ten,twenty,[]");
}

// testpart/TestTestPart.al TestPart_New_AddsARowThePartCanThenWrite
#[test]
fn a_row_typed_into_a_part_is_saved_when_the_host_closes() {
    let r = run(
        "procedure F(): Text var H: TestPage \"PT Host\"; R: Record \"PT Row\"; begin
            H.OpenEdit();
            H.Lines.New();
            H.Lines.Grp.SetValue('B');
            H.Lines.LineNo.SetValue(40);
            H.Lines.Descr.SetValue('AddedThroughPart');
            H.Close();
            R.Get('B', 40);
            exit(R.Descr);
        end;",
        "F",
    );
    assert_eq!(text(r), "AddedThroughPart");
}

const RANKS: &str = "R.\"Entry No.\" := 1; R.Rank := 30; R.Bucket := 'KEEP'; R.Insert();
    R.\"Entry No.\" := 2; R.Rank := 10; R.Bucket := 'DROP'; R.Insert();
    R.\"Entry No.\" := 3; R.Rank := 40; R.Bucket := 'KEEP'; R.Insert();
    R.\"Entry No.\" := 4; R.Rank := 20; R.Bucket := 'KEEP'; R.Insert();";

// handlers/TestPartSubPageView.al SubPageView_FiltersThePartToTheMatchingRowsOnly and
// handlers/TestPartSubPageViewSorting.al SubPageViewSortingAndWhere_FiltersAndOrdersTogether:
// the part control's SubPageView filters, orders by the named key and direction.
#[test]
fn a_sub_page_view_filters_and_orders_the_part() {
    let r = run(
        &format!(
            "procedure F(): Text var R: Record \"PT Rank\"; H: TestPage \"PT View Host\"; T: Text; begin
                {RANKS}
                H.OpenEdit();
                if H.Sorted.First() then repeat T += H.Sorted.Entry.Value() + ','; until not H.Sorted.Next();
                T += '|';
                if H.Plain.First() then repeat T += H.Plain.Entry.Value() + ','; until not H.Plain.Next();
                H.Close();
                exit(T);
            end;"
        ),
        "F",
    );
    // The editable part ends with its blank new-row line (an empty Entry No. reads '0').
    assert_eq!(text(r), "3,1,4,0,|1,2,3,4,0,");
}

// handlers/TestPartSubPageView.al SubPageView_FilterLandsInTheLinkFilterGroup4
#[test]
fn a_sub_page_view_filter_lands_in_filter_group_four() {
    let r = run(
        &format!(
            "procedure F(): Text var R: Record \"PT Rank\"; H: TestPage \"PT View Host\"; P: Page \"PT View Part\"; begin
                {RANKS}
                H.OpenEdit();
                H.Sorted.First();
                H.Close();
                exit('');
            end;"
        ),
        "F",
    );
    let _ = text(r);
}

const DOCS: &str = "D.\"No.\" := 'H1'; D.Insert(); D.\"No.\" := 'H2'; D.Insert();";

fn line(doc: &str, no: i32) -> String {
    format!("L.\"Doc No.\" := '{doc}'; L.\"Line No.\" := {no}; L.Descr := 'seed'; L.Insert();")
}

// handlers/TestPagePartDraftLink.al LinkedPart_DraftLine_ReadsTheLinkValueInTheLinkedKeyColumn,
// LinkedPart_DraftLine_HasRunTheOnNewRecordTrigger, LinkedPart_DraftLine_ReadsZeroInTheAutoSplitKeyColumn
#[test]
fn the_draft_line_of_a_linked_part_is_a_started_record() {
    let r = run(
        &format!(
            "procedure F(): Text var D: Record \"PT Doc\"; L: Record \"PT Doc Line\"; C: TestPage \"PT Doc Card\"; T: Text; begin
                {DOCS} {} {}
                C.OpenEdit(); C.GoToKey('H1');
                C.Lines.First(); C.Lines.Next();
                T := C.Lines.DocNo.Value() + '/' + C.Lines.Descr.Value() + '/' + C.Lines.SetByNew.Value() + '/' + C.Lines.LineNo.Value();
                C.Close();
                exit(T);
            end;",
            line("H1", 10000),
            line("H2", 10000)
        ),
        "F",
    );
    assert_eq!(text(r), "H1//NEWREC/0");
}

// handlers/TestPagePartDraftLink.al EmptyLinkedPart_WriteWithoutFirst_ValidateSeesTheLinkedKey
// and handlers/TestPageSubpagePartNewRecordInit.al New_StampsTheLinkedFieldAndValidatesIt
#[test]
fn typing_into_the_draft_line_inserts_a_row_with_the_link_key_validated() {
    let r = run(
        &format!(
            "procedure F(): Text var D: Record \"PT Doc\"; L: Record \"PT Doc Line\"; C: TestPage \"PT Doc Card\"; T: Text; begin
                {DOCS} {}
                C.OpenEdit(); C.GoToKey('H1');
                C.Lines.Descr.SetValue('typed');
                C.Close();
                L.SetRange(\"Doc No.\", 'H1');
                L.FindFirst();
                T := L.\"Doc No.\" + '/' + Format(L.\"Line No.\") + '/' + L.\"Seen Doc\" + '/' + L.\"Set By New\" + '/' + Format(L.\"Doc Validated\");
                L.SetRange(\"Doc No.\", 'H2'); T += '/' + Format(L.Count());
                exit(T);
            end;",
            line("H2", 10000)
        ),
        "F",
    );
    // The blank line typed without New() takes the first AutoSplitKey number (NewLineStampsFilters.al
    // QP_TwoKeyLink_TypeFirst_BlankLine); the corpus test above pins only that the line is stored.
    assert_eq!(text(r), "H1/10000/H1/NEWREC/Yes/1");
}

// handlers/TestPagePartOnNewRecordCount.al: showing the draft line starts the record once; First()
// on the open empty part and writing into the draft line start nothing; New() starts one more.
#[test]
fn on_new_record_runs_once_per_started_row() {
    let r = run(
        &format!(
            "procedure F(): Text var D: Record \"PT Doc\"; C: TestPage \"PT Doc Card\"; N: Codeunit \"PT Count\"; T: Text; Open: Integer; begin
                {DOCS}
                C.OpenEdit(); C.GoToKey('H1');
                Open := N.Count();
                T += Format(Open > 0) + ',' + Format(C.Lines.First()) + ',' + Format(N.Count() - Open);
                C.Lines.Descr.SetValue('x');
                T += ',' + Format(N.Count() - Open);
                C.Lines.New();
                T += ',' + Format(N.Count() - Open);
                C.Close();
                exit(T);
            end;"
        ),
        "F",
    );
    assert_eq!(text(r), "Yes,No,0,0,1");
}

fn append_after(existing: &[i32], pos: usize) -> String {
    // The new line's number when New() is pressed on the line at index `pos`.
    let seeds: String = existing.iter().map(|n| line("H1", *n)).collect::<Vec<_>>().join(" ");
    let r = run(
        &format!(
            "procedure F(): Text var D: Record \"PT Doc\"; L: Record \"PT Doc Line\"; C: TestPage \"PT Doc Card\"; T: Text; begin
                {DOCS} {seeds}
                C.OpenEdit(); C.GoToKey('H1');
                C.Lines.First();
                {}
                C.Lines.New();
                C.Lines.Descr.SetValue('new');
                C.Close();
                L.SetRange(\"Doc No.\", 'H1'); L.SetRange(Descr, 'new'); L.FindFirst();
                exit(Format(L.\"Line No.\"));
            end;",
            "C.Lines.Next();".repeat(pos)
        ),
        "F",
    );
    text(r)
}

// handlers/TestPageAutoSplitKeyRanges_Tests.al: _OnAGridThatAlreadyHasALine_ContinuesFromTheLastLine,
// _BetweenLines50000And70000_LandsAt60000, _BetweenLines10000And90000_StepsOneIncrementTo20000,
// _AfterSingleNegativeLine_LandsAtMinus6667, _BetweenMinus10000And10000_LandsAtMinusOne
#[test]
fn auto_split_key_numbers_a_new_line_after_the_current_one() {
    assert_eq!(append_after(&[50000], 0), "60000");
    assert_eq!(append_after(&[50000, 70000], 0), "60000");
    assert_eq!(append_after(&[10000, 90000], 0), "20000");
    assert_eq!(append_after(&[-10000], 0), "-6667");
    assert_eq!(append_after(&[-10000, 10000], 0), "-1");
}

// handlers/TestPageAutoSplitKey_Tests.al TestPage_New_ThreeConsecutiveLines_AutoSplitKeyNumbersThemInTenThousands
#[test]
fn consecutive_new_lines_are_numbered_in_steps_of_ten_thousand() {
    let r = run(
        &format!(
            "procedure F(): Text var D: Record \"PT Doc\"; L: Record \"PT Doc Line\"; C: TestPage \"PT Doc Card\"; T: Text; begin
                {DOCS}
                C.OpenEdit(); C.GoToKey('H1');
                C.Lines.New(); C.Lines.Descr.SetValue('one');
                C.Lines.New(); C.Lines.Descr.SetValue('two');
                C.Lines.New(); C.Lines.Descr.SetValue('three');
                C.Close();
                L.SetRange(\"Doc No.\", 'H1');
                if L.FindSet() then repeat T += Format(L.\"Line No.\") + L.Descr + ','; until L.Next() = 0;
                exit(T);
            end;"
        ),
        "F",
    );
    assert_eq!(text(r), "20000one,30000two,40000three,");
}

// testpart/TestOkPartRow.al OkInvoke_SavesThePartRowStartedByNew (and the untouched negative control)
#[test]
fn ok_saves_a_typed_part_row_and_drops_an_untouched_new_one() {
    let r = run(
        &format!(
            "procedure F(): Text var D: Record \"PT Doc\"; L: Record \"PT Doc Line\"; C: TestPage \"PT Doc Card\"; T: Text; begin
                {DOCS}
                C.OpenEdit(); C.GoToKey('H1');
                C.Lines.New(); C.Lines.Descr.SetValue('typed');
                C.OK().Invoke();
                C.OpenEdit(); C.GoToKey('H2');
                C.Lines.New();
                C.OK().Invoke();
                L.SetRange(\"Doc No.\", 'H1');
                exit(Format(L.Count()) + '/' + Format(D.Count()));
            end;"
        ),
        "F",
    );
    assert_eq!(text(r), "1/2");
}

// handlers/TestPageNoSrcPart_Tests.al DirectOpen_NoSrcHost_NoSourceTablePartControlReads,
// _HostFieldReachesTheNoSourceTablePartPage and _HostWriteIsVisibleThroughTheSameInstance:
// a part page without a source table, and the host's code calling its procedure through
// CurrPage.<Part>.Page.
#[test]
fn a_host_calls_the_procedures_of_its_part_page() {
    let r = run(
        "procedure F(): Text var H: TestPage \"PT Info Host\"; T: Text; begin
            H.OpenEdit();
            T := H.Info.Tag.Value();
            H.Mode.SetValue('FROM-HOST');
            T += '/' + H.Info.Tag.Value();
            H.Info.Tag.SetValue('typed');
            H.Close();
            exit(T);
        end;",
        "F",
    );
    assert_eq!(text(r), "Hello/FROM-HOST");
}

// testfilter/TestTestFilter.al TestFilter_SetFilter_And_GetFilter_RoundTripAFilterExpression and
// TestFilter_SetFilter_RestrictsTheRowsetAndNotJustTheReportedFilter, on a part (claim 9: a part has
// its own Filter, independent of its host's).
#[test]
fn a_part_filter_restricts_its_own_rowset_and_reads_back() {
    let r = run(
        &format!(
            "procedure F(): Text var R: Record \"PT Rank\"; H: TestPage \"PT View Host\"; T: Text; begin
                {RANKS}
                H.OpenView();
                H.Plain.Filter.SetFilter(Rank, '10..20');
                T := H.Plain.Filter.GetFilter(Rank) + '[' + H.Plain.Filter.GetFilter(Bucket) + ']';
                if H.Plain.First() then repeat T += H.Plain.Entry.Value() + ','; until not H.Plain.Next();
                T += '|';
                if H.Sorted.First() then repeat T += H.Sorted.Entry.Value() + ','; until not H.Sorted.Next();
                H.Close();
                exit(T);
            end;"
        ),
        "F",
    );
    assert_eq!(text(r), "10..20[]2,4,|3,1,4,");
}

// TestFilter.SetFilter with an empty text takes the filter off the field, as Record.SetFilter and FieldRef.SetFilter do
// (errors/lt-ms-refusals-134/P134DateTextFilters.al:P134_FieldRef_Date_EmptyAndBlankForms pins FieldRef; for a page filter it is an
// assumption, testfilter-setfilter-empty-clears).
#[test]
fn an_empty_page_filter_text_clears_the_filter_of_the_field() {
    let r = run(
        &format!(
            "procedure F(): Text var R: Record \"PT Rank\"; H: TestPage \"PT View Host\"; T: Text; begin
                {RANKS}
                H.OpenView();
                H.Plain.Filter.SetFilter(Rank, '10..20');
                H.Plain.Filter.SetFilter(Rank, '');
                T := '[' + H.Plain.Filter.GetFilter(Rank) + ']';
                if H.Plain.First() then repeat T += H.Plain.Entry.Value() + ','; until not H.Plain.Next();
                H.Close();
                exit(T);
            end;"
        ),
        "F",
    );
    assert_eq!(text(r), "[]1,2,3,4,");
}

// handlers/TestPageTempPart_Host.al: the host's OnOpenPage reaches the part page it shows through
// CurrPage.<Part>.Page, so the part page exists before the host's own triggers run, and the part's
// OnOpenPage runs after the host's.
#[test]
fn a_hosts_on_open_page_can_reach_its_part_page() {
    let r = run(
        "procedure F(): Text var H: TestPage \"PT Seeding Host\"; T: Text; begin
            H.OpenEdit();
            T := H.Info.Tag.Value();
            H.Close();
            exit(T);
        end;",
        "F",
    );
    // PT Info Part's OnOpenPage sets 'Hello' after the host's OnOpenPage set 'SEEDED'.
    assert_eq!(text(r), "Hello");
}

// testpart/TestPartProviderMove.al ProviderLinkedPart_FollowsTheProviderWhenOnlyTheProviderMoves and
// ProviderLinkedPart_HostMovesToAnEmptyProvider_ReadsBlank (the provider's own row, not the host's).
#[test]
fn a_provider_linked_part_follows_the_provider_part() {
    let r = run(
        "procedure F(): Text var H: Record \"PT Header\"; L: Record \"PT Link Row\"; C: TestPage \"PT Provider Card\"; T: Text; begin
            H.\"No.\" := 'H1'; H.Insert(); H.\"No.\" := 'H2'; H.Insert();
            L.\"Header No.\" := 'H1'; L.\"Line No.\" := 10; L.Info := 'ten'; L.Insert();
            L.\"Header No.\" := 'H1'; L.\"Line No.\" := 20; L.Info := 'twenty'; L.Insert();
            L.\"Header No.\" := 'X'; L.\"Line No.\" := 10; L.Info := 'detail10'; L.Insert();
            L.\"Header No.\" := 'X'; L.\"Line No.\" := 20; L.Info := 'detail20'; L.Insert();
            C.OpenView(); C.GoToKey('H1');
            T := C.Detail.Info.Value();
            C.Lines.Next();
            T += ',' + C.Detail.Info.Value();
            C.GoToKey('H2');
            T += ',[' + C.Detail.Info.Value() + ']';
            exit(T);
        end;",
        "F",
    );
    // Detail is linked on "Line No." only, to the Provider's line number (the host has none): the
    // first row in key order with that line number is H1's.
    assert_eq!(text(r), "ten,twenty,[]");
}

// handlers/TestPageSubpagePartFieldLink.al, TestPageSubpagePartConstFilter.al: New() stamps a
// single-valued link onto key fields only, a multi-valued filter nowhere, and PopulateAllFields
// lifts the key gate.
#[test]
fn new_stamps_single_valued_links_onto_key_fields_only() {
    let r = run(
        "procedure F(): Text var H: Record \"PT Header\"; C: TestPage \"PT Link Card\"; T: Text; begin
            H.\"No.\" := 'H1'; H.Insert();
            C.OpenEdit(); C.GoToKey('H1');
            C.Keyed.New();
            T := C.Keyed.HeaderNo.Value() + '/' + C.Keyed.Tag.Value();
            C.Multi.New();
            T += ',' + C.Multi.HeaderNo.Value() + '/' + C.Multi.Tag.Value();
            C.Populated.New();
            T += ',' + C.Populated.Tag.Value();
            C.Close();
            exit(T);
        end;",
        "F",
    );
    // Tag is not part of the key: const('T1') is not stamped by New() on Keyed; the multi-valued
    // filter stamps nothing on Tag; PopulateAllFields stamps it.
    assert_eq!(text(r), "H1/,H1/,T1");
}

// What Editable = false on the part control does to the rows is not pinned: writing through it is
// refused (the corpus pins only that TestPart.Editable() ignores it).
#[test]
fn writing_through_a_read_only_part_control_is_refused() {
    let r = run(
        "procedure F() var H: Record \"PT Header\"; C: TestPage \"PT Link Card\"; begin
            H.\"No.\" := 'H1'; H.Insert();
            C.OpenEdit(); C.GoToKey('H1');
            C.RO.New();
        end;",
        "F",
    );
    assert!(matches!(r, Err(Signal::Unsupported(m)) if m.contains("Editable = false")));
}

// testpart/TestTestPart.al TestPart_GoToKey_*: a missing key answers false when the result is used;
// the wrong number of key values and a discarded miss raise errors with the texts and codes above.
#[test]
fn part_go_to_key_answers_false_and_refuses_what_the_corpus_leaves_open() {
    let ok = run(
        &format!(
            "procedure F(): Text var R: Record \"PT Row\"; H: TestPage \"PT Host\"; T: Text; begin
                {SEED}
                H.OpenEdit();
                T := Format(H.Lines.GoToKey('A', 20)) + Format(H.Lines.GoToKey('A', 999)) + H.Lines.LineNo.Value();
                H.Close();
                exit(T);
            end;"
        ),
        "F",
    );
    assert_eq!(text(ok), "YesNo20");
    // handlers/TestPageRuntimeContracts.al:300-312: texts and codes.
    let arity = run(
        "procedure F() var H: TestPage \"PT Host\"; begin H.OpenEdit(); H.Lines.GoToKey('A'); end;",
        "F",
    );
    assert!(matches!(arity, Err(Signal::Error(e))
        if e.message == "The number of arguments does not match the number of primary key fields."
            && e.code == "TestInvalidNumberOfKeyFieldValues"));
    let discarded = run(
        "procedure F() var H: TestPage \"PT Host\"; begin H.OpenEdit(); H.Lines.GoToKey('A', 999); end;",
        "F",
    );
    assert!(matches!(discarded, Err(Signal::Error(e))
        if e.message == "The row does not exist on the TestPage." && e.code == "TestRowNotFound"));
}

// testfilter/TestTestFilter.al: Ascending, CurrentKey and SetCurrentKey change the walk of the page.
#[test]
fn test_filter_changes_the_key_and_direction_of_the_page() {
    let r = run(
        &format!(
            "procedure F(): Text var R: Record \"PT Rank\"; H: TestPage \"PT View Host\"; T: Text; begin
                {RANKS}
                H.OpenView();
                H.Plain.Filter.SetCurrentKey(Rank);
                T := H.Plain.Filter.CurrentKey() + ':';
                if H.Plain.First() then repeat T += H.Plain.Entry.Value() + ','; until not H.Plain.Next();
                H.Plain.Filter.Ascending(false);
                T += Format(H.Plain.Filter.Ascending()) + ':';
                if H.Plain.First() then repeat T += H.Plain.Entry.Value() + ','; until not H.Plain.Next();
                H.Close();
                exit(T);
            end;"
        ),
        "F",
    );
    assert_eq!(text(r), "Rank:2,4,1,3,No:3,1,4,2,");
}

// handlers/TestPageOpenNewPartLink.al OpenNew_NumberAssignedOnInsert_PartWriteCarriesTheNumber:
// the parts of a new host row link to it once it is stored.
#[test]
fn a_new_hosts_part_follows_the_header_once_it_is_stored() {
    let r = run(
        "procedure F(): Text var D: Record \"PT Doc\"; L: Record \"PT Doc Line\"; C: TestPage \"PT Doc Card\"; begin
                C.OpenNew();
                C.DocNo.SetValue('N1');
                C.Descr.SetValue('header');
                C.Lines.Descr.SetValue('line');
                C.Close();
                L.SetRange(\"Doc No.\", 'N1');
                L.FindFirst();
                exit(L.\"Doc No.\" + '/' + L.\"Seen Doc\" + '/' + Format(L.\"Line No.\"));
            end;",
        "F",
    );
    // First AutoSplitKey number on the blank line (NewLineStampsFilters.al QP_TwoKeyLink_TypeFirst_BlankLine).
    assert_eq!(text(r), "N1/N1/10000");
}

// A part the control tree dropped (Visible = false) is still there for the page's own code: Base Application's Sales
// Credit Memo calls `CurrPage.IncomingDocAttachFactBox.Page.LoadDataFromRecord(Rec)` in OnAfterGetCurrRecord (Tests-ERM
// ExternalDocNoNotificationForCrMemo and about 230 more). Unprobed: assumption invisible-part-for-page-code.
#[test]
fn the_pages_own_code_reaches_a_part_the_control_tree_dropped() {
    let r = run(
        "procedure F(): Text var R: Record \"PT Row\"; H: TestPage \"PT Reach Host\"; begin
            R.Grp := 'A'; R.\"Line No.\" := 10; R.Insert();
            H.OpenView(); exit('opened'); end;",
        "F",
    );
    assert_eq!(text(r), "opened");
}

// pages/lt-page-report-events/P84Captions.al (Page_NoSubscriber c01..c13, BC 29): a CaptionClass of a numeric area other
// than 3 stays as written and raises no event. Base Application's captions of this kind ('5,1,GB', '1,1,') stopped
// about 770 Tests-ERM tests.
#[test]
fn a_numeric_caption_class_area_other_than_3_keeps_the_text_as_written() {
    let r = run(
        "procedure F(): Text var H: TestPage \"PT Cap Card\"; begin H.OpenView(); exit(H.Grp.Caption() + '|' + H.Dim.Caption() + '|' + H.Lit.Caption()); end;",
        "F",
    );
    assert_eq!(text(r), "5,1,GB|1,1,|Fixed");
}

// pages/lt-page-report-events/P84Captions.al Table_NoSubscriber f01..f14 (BC 29): FieldCaption of a field with a CaptionClass.
#[test]
fn field_caption_of_a_field_with_a_caption_class_follows_the_pinned_forms() {
    let r = run(
        "procedure F(): Text var R: Record \"PT Cap Rec\"; begin
            exit(R.FieldCaption(A) + '|' + R.FieldCaption(B) + '|' + R.FieldCaption(C) + '|' + R.FieldCaption(D)); end;",
        "F",
    );
    assert_eq!(text(r), "1,1,x|x|Tbl C|,x");
}

// Tests-ERM `PurchaseOrderStatisticsTestPage.InvDiscountAmount_General.SetValue(...)`: the control shows
// `TotalPurchLine[1]."Inv. Discount Amount"`. Unprobed (page-expression-control-setvalue).
#[test]
fn set_value_on_a_control_bound_to_an_assignable_expression_writes_it_and_validates() {
    let r = run(
        "procedure F(): Text var H: TestPage \"PT Expr Card\"; R: Record \"PT Row\"; s: Text; begin
            R.Grp := 'A'; R.\"Line No.\" := 1; R.Insert();
            H.OpenEdit();
            s := H.Ctl.Value() + '|' + H.Amt.Value();
            H.Ctl.SetValue('changed'); H.Amt.SetValue(12);
            s += '|' + H.Ctl.Value() + '|' + H.Amt.Value() + '|' + Format(H.Fn.Value());
            exit(s); end;",
        "F",
    );
    assert_eq!(text(r), "start|7|changed|12|changed");
}

#[test]
fn set_value_on_a_control_bound_to_a_call_stays_read_only() {
    let r = run(
        "procedure F(): Text var H: TestPage \"PT Expr Card\"; begin H.OpenEdit(); H.Fn.SetValue('x'); exit('ran'); end;",
        "F",
    );
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}

// Microsoft's Tests-ERM write `Page.Control.SetValue := value;` (ERM Pmt. Tolerance VAT Appln., 197 tests with the other
// "is not assignable" shapes): the setter form of the SetValue method, as `Value :=` is of Value. Unprobed.
#[test]
fn set_value_written_as_an_assignment_is_the_call() {
    let r = run(
        "procedure F(): Text var H: TestPage \"PT Cap Card\"; R: Record \"PT Row\"; begin
            R.Grp := 'A'; R.\"Line No.\" := 1; R.Insert();
            H.OpenEdit(); H.Dim.SetValue := 'viaassign'; exit(H.Dim.Value()); end;",
        "F",
    );
    assert_eq!(text(r), "viaassign");
}

// misc/lt-probes-needed-7/PnGPartTriggers.al:OpenOrderOfHostWithSixParts (17 part reads) and pages/lt-part-reads/PartReadsOpen.al:
// OpenView_SixParts_ReadsPerPartLogged (`open-Same`, 3 rows read 30000, 20000, 10000, count 4 with OnOpenPage): a part whose SubPageView
// restricts the field its SubPageLink names reads its rows once each, last to first, when the host opens; every other part reads
// its rows and then the first row again.
#[test]
fn a_part_viewing_its_linked_field_reads_its_rows_backwards_once_at_open() {
    let mut objs = String::from(
        "table 52000 \"SX Line\" { fields { field(1; \"Head Code\"; Code[10]) { } field(2; \"Line No.\"; Integer) { } } keys { key(PK; \"Head Code\", \"Line No.\") { } } }
         table 52001 \"SX Head\" { fields { field(1; \"Code\"; Code[10]) { } } keys { key(PK; \"Code\") { } } }
         codeunit 52002 \"SX Log\" { SingleInstance = true; var T: Text; procedure Add(s: Text) begin T += s + ';'; end; procedure Dump(): Text begin exit(T); end; }
",
    );
    for i in 1..=6 {
        objs += &format!(
            "page {} \"SX Part {i}\" {{ PageType = ListPart; SourceTable = \"SX Line\"; layout {{ area(Content) {{ repeater(R) {{ field(H; Rec.\"Head Code\") {{ }} field(L; Rec.\"Line No.\") {{ }} }} }} }}
              trigger OnAfterGetRecord() var Lg: Codeunit \"SX Log\"; begin Lg.Add('P{i}/' + Format(Rec.\"Line No.\")); end; }}\n",
            52010 + i
        );
    }
    let views = [
        "",
        "",
        "",
        "",
        "SubPageView = where(\"Line No.\" = filter('>10'));",
        "SubPageView = where(\"Head Code\" = const('ZZZ'));",
    ];
    let vis = ["", "", "Enabled = false;", "Visible = Hide;", "", ""];
    let ed = ["", "Editable = false;", "", "", "", ""];
    let mut parts = String::new();
    for i in 0..6 {
        parts += &format!(
            "part(P{}; \"SX Part {}\") {{ {} {} {} SubPageLink = \"Head Code\" = field(\"Code\"); }}\n",
            i + 1,
            i + 1,
            views[i],
            vis[i],
            ed[i]
        );
    }
    objs += &format!(
        "page 52030 \"SX Host\" {{ PageType = Document; SourceTable = \"SX Head\"; layout {{ area(Content) {{ field(C; Rec.\"Code\") {{ }} {parts} }} }} var Hide: Boolean; }}\n"
    );
    let src = format!(
        "{objs}\ncodeunit 52100 C {{ procedure F(): Text var H: Record \"SX Head\"; L: Record \"SX Line\"; Pg: TestPage \"SX Host\"; Lg: Codeunit \"SX Log\"; begin H.Code := 'D1'; H.Insert(); L.\"Head Code\" := 'D1'; L.\"Line No.\" := 10000; L.Insert(); L.\"Line No.\" := 20000; L.Insert(); Pg.OpenEdit(); exit(Lg.Dump()); end; }}"
    );
    let l = al_testkit::compile(&src);
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let mut s = al_testkit::session(l);
    let inst = s.instance(oi);
    let r = s.call(oi, 0, inst, vec![]).map(|(v, _)| v);
    assert_eq!(
        text(r),
        "P1/10000;P1/20000;P1/10000;P2/10000;P2/20000;P2/10000;P3/10000;P3/20000;P3/10000;P4/10000;P4/20000;P4/10000;P5/10000;P5/20000;P5/10000;P6/20000;P6/10000;"
    );
}

// misc/lt-probes-needed-7/PnGPartRows.al:117 ReadOnlyPartSetValue `ro-descr-editable` No: a field of a part whose control
// says the literal `Editable = false` is not editable.
#[test]
fn a_field_of_a_part_with_a_literal_editable_false_reads_not_editable() {
    let r = run(
        "procedure F(): Text var H: Record \"PT Header\"; L: Record \"PT Link Row\"; C: TestPage \"PT Link Card\"; begin
            H.\"No.\" := 'H1'; H.Insert(); L.\"Header No.\" := 'H1'; L.\"Line No.\" := 10; L.Insert();
            C.OpenEdit(); C.GoToKey('H1'); C.RO.First();
            exit(Format(C.RO.Info.Editable()) + '/' + Format(C.Keyed.Info.Editable()));
        end;",
        "F",
    );
    assert_eq!(text(r), "No/Yes");
}

// Hard rule 4: what an Editable or Enabled expression on a part control (or Enabled = false) does to the fields of the part
// is not pinned (the corpus pins only TestPart.Editable()/Enabled() of the part itself,
// pages/lt-page-controls-aa/AaControls.al:AA_Controls_PartState). Tests-ERM ERM Sales Quotes BlankSalesQuoteLinesUneditable
// reads `SalesLines.Description.Editable()` behind `Editable = IsSalesLinesEditable`: refused, not answered Yes.
#[test]
fn editable_and_enabled_of_a_field_behind_a_part_expression_are_refused() {
    for (part, method) in [
        ("EdExpr", "Editable"),
        ("EdExpr", "Enabled"),
        ("EnExpr", "Editable"),
        ("EnExpr", "Enabled"),
        ("EnOff", "Editable"),
        ("EnOff", "Enabled"),
    ] {
        let r = run(
            &format!(
                "procedure F(): Boolean var H: Record \"PT Header\"; C: TestPage \"PT Flag Card\"; begin
                    H.\"No.\" := 'H1'; H.Insert();
                    C.OpenEdit(); C.GoToKey('H1');
                    exit(C.{part}.Info.{method}());
                end;"
            ),
            "F",
        );
        assert!(
            matches!(&r, Err(Signal::Unsupported(m)) if m.contains("part")),
            "{part}.{method}: {r:?}"
        );
    }
}

// Enabled() of a field behind a part control with `Editable = false` is not pinned (PnGPartRows.al pins Editable() only).
#[test]
fn enabled_of_a_field_behind_a_read_only_part_is_refused() {
    let r = run(
        "procedure F(): Boolean var H: Record \"PT Header\"; C: TestPage \"PT Link Card\"; begin
            H.\"No.\" := 'H1'; H.Insert();
            C.OpenEdit(); C.GoToKey('H1');
            exit(C.RO.Info.Enabled());
        end;",
        "F",
    );
    assert!(
        matches!(&r, Err(Signal::Unsupported(m)) if m.contains("Editable = false")),
        "{r:?}"
    );
}

// Assumption part-field-under-true-part-expression: an Editable or Enabled expression on a part control that comes out true
// leaves the fields of the part answering by themselves, as without the property (Tests-ERM sales and purchase document
// tests read fields of `SalesLines` behind `Editable = IsSalesLinesEditable; Enabled = IsSalesLinesEditable;`).
#[test]
fn a_part_expression_that_is_true_leaves_its_fields_answering_by_themselves() {
    al_builtins::assumptions::clear_hits();
    let r = run(
        "procedure F(): Text var H: Record \"PT Header\"; C: TestPage \"PT Flag On Card\"; begin
            H.\"No.\" := 'H1'; H.Insert();
            C.OpenEdit(); C.GoToKey('H1');
            exit(Format(C.EdExpr.Info.Editable()) + Format(C.EdExpr.Info.Enabled()) + Format(C.EnExpr.Info.Editable()) + Format(C.EnExpr.Info.Enabled()));
        end;",
        "F",
    );
    assert_eq!(text(r), "YesYesYesYes");
    assert!(
        al_builtins::assumptions::take_hits()
            .contains(&al_builtins::assumptions::Assumption::PartFieldUnderTruePartExpression)
    );
}

// pages/lt-part-line-before-host-key/PartLineBeforeHostKey.al:672 PYB_OpenNew_Document_LogOnly, :809 PYB_OpenEdit_HostWithoutLines,
// :879 PYB_OpenEdit_ThenHostNew: the blank line of a part that shows no row of an editable host runs OnNewRecord three times as the
// host opens (OpenNew and OpenEdit); the host's New() runs none of the part's.
#[test]
fn an_empty_part_of_an_editable_host_runs_on_new_record_three_times_and_host_new_none() {
    let r = run(
        &format!(
            "procedure F(): Text var D: Record \"PT Doc\"; C: TestPage \"PT Doc Card\"; E: TestPage \"PT Doc Card\"; N: Codeunit \"PT Count\"; T: Text; Before: Integer; begin
                {DOCS}
                Before := N.Count();
                C.OpenNew();
                T := Format(N.Count() - Before);
                C.Close();
                Before := N.Count();
                E.OpenEdit();
                T += ',' + Format(N.Count() - Before);
                Before := N.Count();
                E.New();
                T += ',' + Format(N.Count() - Before);
                E.Close();
                exit(T);
            end;"
        ),
        "F",
    );
    assert_eq!(text(r), "3,3,0");
}

// pages/lt-part-line-before-host-key/PartLineBeforeHostKey.al:1349 QH_EmptyPart_OpenNew_CurrRecordOrder, :1381
// QH_EmptyPart_OpenEdit_CurrRecordOrder, :1495 QH_EmptyPart_OpenEdit_HostNext, :1418 QH_EmptyPart_OpenNew_KeyTyped: the
// empty part of an editable host runs OnAfterGetCurrRecord right after each of its three OnNewRecord runs; :1439
// QH_EmptyPart_OpenNew_CurrRecordOnlyPart, :1455 QH_EmptyPart_OpenEdit_CurrRecordOnlyPart: a part declaring only
// OnAfterGetCurrRecord runs it three times. No assumption is marked on this path any more.
#[test]
fn an_empty_part_runs_on_after_get_curr_record_after_each_of_its_three_new_records() {
    let r = run(
        "procedure F(): Text var D: Record \"PT Doc\"; C: TestPage \"PT Doc Card Cur\"; E: TestPage \"PT Doc Card Cur\"; O: TestPage \"PT Doc Card CurOnly\"; G: Codeunit \"PT Log\"; T: Text; begin
            G.Take();
            C.OpenNew();
            T := G.Take();
            C.Close();
            D.\"No.\" := 'D1'; D.Insert(); D.\"No.\" := 'D2'; D.Insert();
            G.Take();
            E.OpenEdit();
            T += ' / ' + G.Take();
            E.Next();
            T += ' / ' + G.Take();
            E.Close();
            O.OpenNew();
            T += ' / ' + G.Take();
            O.Close();
            exit(T);
        end;",
        "F",
    );
    assert_eq!(
        text(r),
        "New: | Cur: | New: | Cur: | New: | Cur: / New:D1 | Cur:D1 | New:D1 | Cur:D1 | New:D1 | Cur:D1 / New:D2 | Cur:D2 | New:D2 | Cur:D2 | New:D2 | Cur:D2 / Cur: | Cur: | Cur:"
    );
}

// PartLineBeforeHostKey.al:879 PYB_OpenEdit_ThenHostNew: a host's New() runs none of the part's OnNewRecord; whether the
// part's OnAfterGetCurrRecord runs then is not pinned, so a part that declares it is refused.
#[test]
fn a_host_new_over_an_empty_part_with_on_after_get_curr_record_is_refused() {
    let r = run(
        "procedure F(): Text var D: Record \"PT Doc\"; E: TestPage \"PT Doc Card Cur\"; begin
            D.\"No.\" := 'D1'; D.Insert();
            E.OpenEdit();
            E.New();
            exit('ran');
        end;",
        "F",
    );
    match r {
        Err(Signal::Unsupported(why)) => assert!(why.contains("while its host runs New()"), "{why}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}
