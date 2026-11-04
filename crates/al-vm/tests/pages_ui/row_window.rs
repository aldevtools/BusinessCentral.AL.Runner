//! The row window of a list page and moves inside it (pages/rows.rs). Each test names the corpus test that pins what BC does.

/// A log table and its codeunit, shared by the pages below.
const LOG: &str = r#"
table 69900 "RW Log" { fields { field(1; Id; Integer) { } field(2; Txt; Text[250]) { } } keys { key(PK; Id) { } } }
codeunit 69901 "RW Log Mgt"
{
    procedure Add(Value: Text)
    var L: Record "RW Log"; N: Integer;
    begin
        if L.FindLast() then N := L.Id;
        L.Init(); L.Id := N + 1; L.Txt := CopyStr(Value, 1, 250); L.Insert();
    end;
    procedure Take(): Text
    var L: Record "RW Log"; R: Text;
    begin
        if L.FindSet() then
            repeat
                if R <> '' then R += ';';
                R += L.Txt;
            until L.Next() = 0;
        L.DeleteAll();
        exit(R);
    end;
}
table 69902 "RW Row" { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } field(3; Amount; Decimal) { } } keys { key(PK; Id) { } } }
"#;

fn with_log(src: &str) -> String {
    format!("{LOG}\n{src}")
}

/// pages/W1Update.al:811-843 (List_BufferLeftByOnAfterGetRecord_AfterTheStoredRowChanged): a GoToKey onto a row the client
/// holds reads that row from the table, so a change made elsewhere since the open shows (`name_row2 = changed`).
#[test]
fn go_to_key_onto_a_held_row_reads_the_stored_row_again() {
    let src = with_log(
        r#"
page 69903 "RW Buffer List"
{
    PageType = List; SourceTable = "RW Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(Id; Rec.Id) { } field(Name; Rec.Name) { } field(Amount; Rec.Amount) { } } } }
    var Log: Codeunit "RW Log Mgt";
    trigger OnAfterGetRecord() begin Rec.Amount := Rec.Id * 100; Log.Add('r' + Format(Rec.Id)); end;
    trigger OnAfterGetCurrRecord() begin Log.Add('c' + Format(Rec.Id) + ':x.name=' + xRec.Name); end;
}
codeunit 69904 C
{
    procedure F(): Text
    var Row: Record "RW Row"; Pg: TestPage "RW Buffer List"; Log: Codeunit "RW Log Mgt"; i: Integer; R: Text;
    begin
        for i := 1 to 4 do begin Row.Init(); Row.Id := i; Row.Name := 'row' + Format(i); Row.Insert(); end;
        Pg.OpenView();
        Log.Take();
        Row.Get(2); Row.Name := 'changed'; Row.Amount := 5; Row.Modify();
        Pg.GoToKey(2);
        R := Log.Take() + '|' + Pg.Name.Value() + '|' + Pg.Amount.Value();
        Pg.Next();
        R += '|' + Log.Take();
        Pg.Previous();
        R += '|' + Log.Take() + '|' + Pg.Name.Value();
        exit(R);
    end;
}
"#,
    );
    assert_eq!(
        al_testkit::text(al_testkit::run_in(&src, "C", "F")),
        "r2;r2;c2:x.name=row1|changed|200.00|r3;r3;c3:x.name=changed|r2;r2;c2:x.name=row3|changed"
    );
}

/// Pages over "RW Row" that log their row reads: `Plain` (OnAfterGetRecord), `Nav` (OnFindRecord, OnNextRecord,
/// OnAfterGetRecord), `NavCurr` (`Nav` and OnAfterGetCurrRecord).
const LISTS: &str = r#"
page 69905 "RW Plain"
{
    PageType = List; SourceTable = "RW Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(Id; Rec.Id) { } } } }
    var Log: Codeunit "RW Log Mgt";
    trigger OnAfterGetRecord() begin Log.Add('R:' + Format(Rec.Id)); end;
}
page 69906 "RW Nav"
{
    PageType = List; SourceTable = "RW Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(Id; Rec.Id) { } } } }
    var Log: Codeunit "RW Log Mgt";
    trigger OnFindRecord(Which: Text): Boolean var Found: Boolean; begin Found := Rec.Find(Which); Log.Add('F:' + Which + '=' + Format(Found)); exit(Found); end;
    trigger OnNextRecord(Steps: Integer): Integer var Moved: Integer; begin Moved := Rec.Next(Steps); Log.Add('N:' + Format(Steps) + '=' + Format(Moved)); exit(Moved); end;
    trigger OnAfterGetRecord() begin Log.Add('R:' + Format(Rec.Id)); end;
}
page 69907 "RW NavCurr"
{
    PageType = List; SourceTable = "RW Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(Id; Rec.Id) { } } } }
    var Log: Codeunit "RW Log Mgt";
    trigger OnFindRecord(Which: Text): Boolean var Found: Boolean; begin Found := Rec.Find(Which); Log.Add('F:' + Which + '=' + Format(Found)); exit(Found); end;
    trigger OnNextRecord(Steps: Integer): Integer var Moved: Integer; begin Moved := Rec.Next(Steps); Log.Add('N:' + Format(Steps) + '=' + Format(Moved)); exit(Moved); end;
    trigger OnAfterGetRecord() begin Log.Add('R:' + Format(Rec.Id)); end;
    trigger OnAfterGetCurrRecord() begin Log.Add('C:' + Format(Rec.Id)); end;
}
codeunit 69908 Seed
{
    procedure Rows(Count: Integer)
    var Row: Record "RW Row"; i: Integer; Log: Codeunit "RW Log Mgt";
    begin
        for i := 1 to Count do begin Row.Init(); Row.Id := i; Row.Insert(); end;
        Log.Take();
    end;
}
"#;

fn lists(src: &str) -> String {
    with_log(&format!("{LISTS}\n{src}"))
}

/// The reads of opening a list of `rows` rows, then two Next() calls: `open|next1 log|next2 log|Id`.
fn open_then_next_twice(page: &str, rows: i32) -> String {
    let src = lists(&format!(
        r#"
codeunit 69909 C
{{
    procedure F(): Text
    var P: TestPage "{page}"; Log: Codeunit "RW Log Mgt"; S: Codeunit Seed; R: Text;
    begin
        S.Rows({rows});
        P.OpenView();
        R := Log.Take();
        R += '|' + Format(P.Next()) + ':' + Log.Take();
        R += '|' + Format(P.Next()) + ':' + Log.Take();
        exit(R + '|' + P.Id.Value());
    end;
}}
"#
    ));
    al_testkit::text(al_testkit::run_in(&src, "C", "F"))
}

/// pages/list-order/AiListOrderTests.al:655 (Next_OneRow_Plain): a list with OnAfterGetRecord alone and one row reads it
/// once when it opens.
#[test]
fn a_plain_list_of_one_row_reads_it_once_at_open() {
    assert_eq!(open_then_next_twice("RW Plain", 1), "R:1|No:|No:|1");
}

/// pages/list-order/AiListOrderTests.al:535, :565 (Next_OneRow_Nav, Next_OneRow_NavCurr); testpage/
/// AnTestPageTests.al:405: a find list of one row does not read it again before OnNextRecord(-1).
#[test]
fn a_find_list_of_one_row_reads_it_once_before_stepping_back() {
    assert_eq!(
        open_then_next_twice("RW Nav", 1),
        "F:=><=Yes;R:1;N:1=0;N:-1=0|No:|No:|1"
    );
    assert_eq!(
        open_then_next_twice("RW NavCurr", 1),
        "F:=><=Yes;R:1;N:1=0;N:-1=0;F:==Yes;R:1;C:1|No:|No:|1"
    );
}

/// pages/list-order/AiListOrderTests.al:600, :630 (Next_TwoRows_Nav, Next_TwoRows_NavCurr): after the open read reached
/// the end of the view, Next on the last row answers No without asking OnNextRecord.
#[test]
fn next_on_the_last_row_after_an_open_that_read_the_whole_view_asks_nothing() {
    assert_eq!(
        open_then_next_twice("RW Nav", 2),
        "F:=><=Yes;R:1;N:1=1;R:2;N:1=0;R:1;N:-1=0|Yes:|No:|2"
    );
    assert_eq!(
        open_then_next_twice("RW NavCurr", 2),
        "F:=><=Yes;R:1;N:1=1;R:2;N:1=0;R:1;N:-1=0;F:==Yes;R:1;C:1|Yes:R:2;F:==Yes;R:2;C:2|No:|2"
    );
}

/// testpage/AnTestPageTests.al:147, :168 (An02_Part_ConstLink_OneRow, _FiveRows): a part with OnAfterGetRecord
/// and OnAfterGetCurrRecord reads its first row twice more after a view that ends inside the window, once more after a view
/// of one row.
#[test]
fn a_part_with_the_current_record_trigger_reads_its_first_row_twice_more() {
    let src = with_log(
        r#"
table 69910 "RW Line" { fields { field(1; Doc; Code[10]) { } field(2; "Line No."; Integer) { } } keys { key(PK; Doc, "Line No.") { } } }
table 69911 "RW Head" { fields { field(1; "Code"; Code[10]) { } } keys { key(PK; "Code") { } } }
page 69912 "RW Part"
{
    PageType = ListPart; SourceTable = "RW Line"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field("Line No."; Rec."Line No.") { } } } }
    var Log: Codeunit "RW Log Mgt";
    trigger OnAfterGetRecord() begin Log.Add('R' + Format(Rec."Line No.")); end;
    trigger OnAfterGetCurrRecord() begin Log.Add('C' + Format(Rec."Line No.")); end;
}
page 69913 "RW Host"
{
    PageType = Card; SourceTable = "RW Head"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field("Code"; Rec."Code") { } part(Lines; "RW Part") { SubPageLink = Doc = field("Code"); } } }
}
codeunit 69914 C
{
    procedure F(): Text
    var H: Record "RW Head"; L: Record "RW Line"; Host: TestPage "RW Host"; Log: Codeunit "RW Log Mgt"; i: Integer; R: Text;
    begin
        H."Code" := 'ONE'; H.Insert(); H."Code" := 'FIVE'; H.Insert();
        L.Doc := 'ONE'; L."Line No." := 10; L.Insert();
        for i := 1 to 5 do begin L.Doc := 'FIVE'; L."Line No." := i * 10; L.Insert(); end;
        Log.Take();
        Host.Trap();
        H.Get('ONE');
        Page.Run(Page::"RW Host", H);
        R := Log.Take();
        Host.Close();
        Host.Trap();
        H.Get('FIVE');
        Page.Run(Page::"RW Host", H);
        R += '|' + Log.Take();
        Host.Close();
        exit(R);
    end;
}
"#,
    );
    assert_eq!(
        al_testkit::text(al_testkit::run_in(&src, "C", "F")),
        "R10;R10;C10|R10;R20;R30;R40;R50;R10;R10;C10"
    );
}

/// pages/list-order/AiListOrderTests.al:769 (GoToRecord_BeforeAnyNavigation_NavCurr): on a find list with
/// OnAfterGetCurrRecord that holds its whole view, GoToRecord from the first row steps through the rows one move at a time.
/// Without OnAfterGetCurrRecord nothing is read (:739 GoToRecord_BeforeAnyNavigation_Nav).
#[test]
fn go_to_record_on_a_find_list_with_the_current_record_trigger_steps_row_by_row() {
    let go = |page: &str| {
        let src = lists(&format!(
            r#"
codeunit 69909 C
{{
    procedure F(): Text
    var P: TestPage "{page}"; Log: Codeunit "RW Log Mgt"; S: Codeunit Seed; Row: Record "RW Row"; R: Text;
    begin
        S.Rows(6);
        P.OpenView();
        Log.Take();
        Row.Get(4);
        R := Format(P.GoToRecord(Row)) + ':' + Log.Take() + '|' + P.Id.Value();
        R += '|' + Format(P.Next()) + ':' + Log.Take();
        exit(R);
    end;
}}
"#
        ));
        al_testkit::text(al_testkit::run_in(&src, "C", "F"))
    };
    assert_eq!(
        go("RW NavCurr"),
        "Yes:R:2;F:==Yes;R:2;C:2;R:3;F:==Yes;R:3;C:3;R:4;F:==Yes;R:4;C:4|4|Yes:R:5;F:==Yes;R:5;C:5"
    );
    assert_eq!(go("RW Nav"), "Yes:|4|Yes:");
}

/// pages/triggers/P84ListTemp.al:387-401 (TestPage_Real_OpenNew) and pages/opennew-findrecord/
/// OpenNewFindRecord.al:210-231 (List_OpenNew_TableWithRows): on a list opened with OpenNew over a table with rows, First and
/// Next answer No, run no trigger, and the cursor stays on the new row.
#[test]
fn moves_on_a_list_opened_with_open_new_answer_no() {
    let src = lists(
        r#"
codeunit 69909 C
{
    procedure F(): Text
    var P: TestPage "RW Plain"; Log: Codeunit "RW Log Mgt"; S: Codeunit Seed; R: Text;
    begin
        S.Rows(3);
        P.OpenNew();
        Log.Take();
        R := Format(P.First()) + ':' + Log.Take() + ':' + P.Id.Value();
        R += '|' + Format(P.Next()) + ':' + Log.Take() + ':' + P.Id.Value();
        exit(R);
    end;
    procedure G(): Text
    var P: TestPage "RW Plain"; S: Codeunit Seed;
    begin
        S.Rows(3);
        P.OpenNew();
        exit(Format(P.Last()));
    end;
}
"#,
    );
    assert_eq!(al_testkit::text(al_testkit::run_in(&src, "C", "F")), "No::0|No::0");
    match al_testkit::run_in(&src, "C", "G") {
        Err(al_builtins::native::Signal::Unsupported(why)) => assert!(why.contains("OpenNew"), "{why}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// record/load-fields/P81PageTempSource.al:238-242 (RunModal_TempRecord_NewRow): a row saved by New()
/// below T1 with key N1 stays below T1 in the client's order (`page_row1 = T1`, `page_row2 = N1`). The client keeps key order,
/// so a move other than Next after such a save is refused; a key that follows its anchor in key order is not.
#[test]
fn a_move_after_a_new_row_saved_out_of_key_order_is_refused() {
    let src = with_log(
        r#"
table 69915 "RW Code Row" { fields { field(1; "Code"; Code[10]) { } field(2; Name; Text[30]) { } } keys { key(PK; "Code") { } } }
page 69916 "RW Code List"
{
    PageType = List; SourceTable = "RW Code Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field("Code"; Rec."Code") { } field(Name; Rec.Name) { } } } }
}
codeunit 69909 C
{
    local procedure Seed()
    var R: Record "RW Code Row";
    begin
        R."Code" := 'T1'; R.Insert(); R."Code" := 'T2'; R.Insert(); R."Code" := 'T3'; R.Insert();
    end;
    procedure F(): Text
    var P: TestPage "RW Code List";
    begin
        Seed();
        P.OpenEdit();
        P.First();
        P.New();
        P."Code".SetValue('N1');
        P.First();
        exit(P."Code".Value());
    end;
    procedure G(): Text
    var P: TestPage "RW Code List";
    begin
        Seed();
        P.OpenEdit();
        P.First();
        P.New();
        P."Code".SetValue('T15');
        P.First();
        exit(P."Code".Value());
    end;
}
"#,
    );
    match al_testkit::run_in(&src, "C", "F") {
        Err(al_builtins::native::Signal::Unsupported(why)) => assert!(why.contains("key order"), "{why}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
    assert_eq!(al_testkit::text(al_testkit::run_in(&src, "C", "G")), "T1");
}

/// testpage/PnF06FindListMoves.al:299-325 (FindList_Descending_3Rows, _25Rows): a find list with
/// OnAfterGetCurrRecord over a descending view opens on its last row; the client reads up from there, at most a window of rows,
/// and moves inside the rows it holds. First past them reads the rest of the view upwards.
#[test]
fn a_descending_find_list_with_the_current_record_trigger_reads_up_from_its_last_row() {
    let moves = |rows: i32| {
        let src = with_log(&format!(
            r#"
page 69917 "RW Desc"
{{
    PageType = List; SourceTable = "RW Row"; ApplicationArea = All; UsageCategory = None;
    layout {{ area(Content) {{ repeater(Rows) {{ field(Id; Rec.Id) {{ }} }} }} }}
    var Log: Codeunit "RW Log Mgt";
    trigger OnOpenPage() begin Rec.Ascending(false); end;
    trigger OnFindRecord(Which: Text): Boolean begin Log.Add('F(' + Which + ')'); exit(Rec.Find(Which)); end;
    trigger OnNextRecord(Steps: Integer): Integer begin Log.Add('N(' + Format(Steps) + ')'); exit(Rec.Next(Steps)); end;
    trigger OnAfterGetRecord() begin Log.Add('R' + Format(Rec.Id)); end;
    trigger OnAfterGetCurrRecord() begin Log.Add('C'); end;
}}
codeunit 69909 C
{{
    procedure F(): Text
    var P: TestPage "RW Desc"; Log: Codeunit "RW Log Mgt"; Row: Record "RW Row"; i: Integer; R: Text;
    begin
        for i := 1 to {rows} do begin Row.Init(); Row.Id := i; Row.Insert(); end;
        Log.Take();
        P.OpenView();
        R := Log.Take();
        P.Next(); R += '|' + Log.Take();
        P.Last(); R += '|' + Log.Take();
        P.Previous(); R += '|' + Log.Take();
        P.First(); R += '|' + Log.Take() + '|' + P.Id.Value();
        exit(R);
    end;
}}
"#
        ));
        al_testkit::text(al_testkit::run_in(&src, "C", "F"))
    };
    assert_eq!(
        moves(3),
        "F(=><);R1;N(1);N(-1);R2;N(-1);R3;N(-1);R1;F(=);R1;C|||R2;F(=);R2;C|R3;F(=);R3;C|3"
    );
    let up: String = (2..=20).map(|i| format!("R{i};N(-1);")).collect();
    let rest: String = (22..=25).map(|i| format!("N(-1);R{i};")).collect();
    assert_eq!(
        moves(25),
        format!("F(=><);R1;N(1);N(-1);{up}R21;R1;F(=);R1;C|||R2;F(=);R2;C|R21;{rest}N(-1);F(=);R25;C|25")
    );
}

/// pages/list-order/AiListOrderTests.al:999, :1020 (Paging_GoToKey_Nav, Paging_GoToKeyThenWalk_Nav): GoToKey past a full
/// window on a find list without OnAfterGetCurrRecord reads the last two rows held again, then the rest of the view with
/// OnNextRecord; the walk to the end afterwards asks nothing.
#[test]
fn go_to_key_past_a_full_window_of_a_find_list_reads_on_to_the_end() {
    let src = lists(
        r#"
codeunit 69909 C
{
    procedure F(): Text
    var P: TestPage "RW Nav"; Log: Codeunit "RW Log Mgt"; S: Codeunit Seed; R: Text;
    begin
        S.Rows(25);
        P.OpenView();
        Log.Take();
        R := Format(P.GoToKey(21)) + ':' + Log.Take() + '|' + P.Id.Value();
        while P.Next() do
            R += '|' + P.Id.Value() + ':' + Log.Take();
        exit(R + '|' + Log.Take() + '|' + P.Id.Value());
    end;
}
"#,
    );
    let on: String = (21..=24).map(|i| format!("R:{i};N:1=1;")).collect();
    assert_eq!(
        al_testkit::text(al_testkit::run_in(&src, "C", "F")),
        format!("Yes:R:19;R:20;N:1=1;{on}R:25;N:1=0|21|22:|23:|24:|25:||25")
    );
}

/// testpage/TestPageNewRowLine_Tests.al:64-87: Next() past the last data row of an editable, insert-allowed page lands on the
/// blank new-row line and answers true; the next Next() answers false. A list that declares OnAfterGetRecord and holds its whole
/// view in the client walks onto the line as well, without asking the page (pages/triggers/QcFindViewTests.al:505
/// DocPart_OpenEdit_LinesOfH1 `part_walk = 10000,20000,0`, a part with OnFindRecord; pages/list-order/AiListOrderTests.al:600
/// Next_TwoRows_Nav `next2_log` empty: after an open that read the view to its end, Next on the last row asks the page nothing).
/// Tests-ERM 134344 SI_MoveNextLineAfterQuantityValidateCalcInvDiscount.
#[test]
fn next_past_the_last_row_of_an_editable_list_with_on_after_get_record_lands_on_the_new_line() {
    let src = with_log(
        r#"
page 69905 "RW Edit List"
{
    PageType = List; SourceTable = "RW Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(Id; Rec.Id) { } field(Name; Rec.Name) { } } } }
    trigger OnAfterGetRecord() begin Rec.Amount := Rec.Id; end;
    trigger OnAfterGetCurrRecord() begin Rec.Amount := Rec.Id; end;
    trigger OnFindRecord(Which: Text): Boolean begin exit(Rec.Find(Which)); end;
}
codeunit 69906 C
{
    procedure F(): Text
    var Row: Record "RW Row"; Pg: TestPage "RW Edit List"; R: Text;
    begin
        Row.Id := 1; Row.Name := 'one'; Row.Insert();
        Pg.OpenEdit();
        R := Format(Pg.Next()) + '|[' + Pg.Name.Value() + ']|' + Format(Pg.Next());
        exit(R);
    end;
    procedure G(): Text
    var Row: Record "RW Row"; Log: Record "RW Log"; Pg: TestPage "RW Host Card"; R: Text;
    begin
        Log.Id := 1; Log.Insert();
        Row.Id := 1; Row.Name := 'one'; Row.Insert();
        Pg.OpenEdit();
        Pg.Lines.Name.SetValue('uno');
        R := Format(Pg.Lines.Next()) + '|[' + Pg.Lines.Name.Value() + ']|' + Format(Pg.Lines.Next());
        exit(R);
    end;
}
page 69907 "RW Lines Part"
{
    PageType = ListPart; SourceTable = "RW Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(Id; Rec.Id) { } field(Name; Rec.Name) { } } } }
    trigger OnAfterGetRecord() begin Rec.Amount := Rec.Id; end;
    trigger OnAfterGetCurrRecord() begin Rec.Amount := Rec.Id; end;
    trigger OnFindRecord(Which: Text): Boolean begin exit(Rec.Find(Which)); end;
}
page 69908 "RW Host Card"
{
    PageType = Document; SourceTable = "RW Log"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(Id; Rec.Id) { } part(Lines; "RW Lines Part") { } } }
}
"#,
    );
    assert_eq!(al_testkit::text(al_testkit::run_in(&src, "C", "F")), "Yes|[]|No");
    assert_eq!(al_testkit::text(al_testkit::run_in(&src, "C", "G")), "Yes|[]|No");
}
