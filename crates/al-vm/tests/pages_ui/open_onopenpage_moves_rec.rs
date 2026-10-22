//! A page without OnFindRecord opened without a record, whose OnOpenPage leaves Rec on a row that is not the first of the
//! view: a Card or List opens on that row, or on the nearest row of the view when Rec is outside the filter OnOpenPage set
//! (corpus pages/lt-qr-onopenpage-moves-rec/QrMovedTests.al:175 QR_Card_OpenView_Get, :292 QR_List_OpenView_Get, :253
//! QR_Card_OpenView_GetThenFilterExcludes, :409 QR_TempCard_OpenView_NoMove). In a key other than the primary key (a
//! SetCurrentKey in OnOpenPage, :345 QR_List_OpenView_SetCurrentKeyGet, or a SourceTableView, the QS_* tests from :856) the
//! page reads Rec's row before it finds from it, and a list reads each row once more. Other page types stay refused.
//! Tests-ERM 134710 EmptySetup reaches it: Payment Registration Setup inserts the user's row in OnOpenPage while another
//! user's row sorts first.

const SRC: &str = r#"
table 69970 "OpMv Setup" {
    fields { field(1; "User ID"; Code[50]) { } field(2; Name; Text[30]) { } }
    keys { key(PK; "User ID") { Clustered = true; } key(ByName; Name) { } }
}
page 69971 "OpMv Card"
{
    PageType = Card; SourceTable = "OpMv Setup"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(UserCtl; Rec."User ID") { } field(NameCtl; Rec.Name) { } } }
    trigger OnOpenPage()
    begin
        case Mode.Get() of
            1: Rec.Get('C');
            7: begin Rec.Get('C'); Rec.SetRange("User ID", 'A', 'B'); end;
        end;
        Mode.Add('O:' + Rec."User ID");
    end;
    trigger OnAfterGetRecord() begin Mode.Add('R:' + Rec."User ID"); end;
    trigger OnAfterGetCurrRecord() begin Mode.Add('C:' + Rec."User ID"); end;
    var Mode: Codeunit OpMvMode;
}
page 69974 "OpMv List"
{
    PageType = List; SourceTable = "OpMv Setup"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(UserCtl; Rec."User ID") { } } } }
    trigger OnOpenPage()
    begin
        case Mode.Get() of
            1: Rec.Get('C');
            7: begin Rec.Get('C'); Rec.SetRange("User ID", 'A', 'B'); end;
            2: Rec.FindLast();
            5: begin Rec.SetCurrentKey(Name); Rec.Get('B'); end;
        end;
        Mode.Add('O:' + Rec."User ID");
    end;
    trigger OnAfterGetRecord() begin Mode.Add('R:' + Rec."User ID"); end;
    trigger OnAfterGetCurrRecord() begin Mode.Add('C:' + Rec."User ID"); end;
    var Mode: Codeunit OpMvMode;
}
page 69975 "OpMv Temp Card"
{
    PageType = Card; SourceTable = "OpMv Setup"; SourceTableTemporary = true; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(UserCtl; Rec."User ID") { } } }
    trigger OnOpenPage()
    begin
        Rec."User ID" := 'A'; Rec.Insert();
        Rec."User ID" := 'B'; Rec.Insert();
    end;
}
page 69976 "OpMv Document"
{
    PageType = Document; SourceTable = "OpMv Setup"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(UserCtl; Rec."User ID") { } } }
    trigger OnOpenPage() begin Rec.Get('C'); end;
}
page 69972 "OpMv Plain"
{
    PageType = Card; SourceTable = "OpMv Setup"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(UserCtl; Rec."User ID") { } } }
}
codeunit 69977 OpMvMode
{
    SingleInstance = true;
    var M: Integer; L: Text;
    procedure Set(N: Integer) begin M := N; L := ''; end;
    procedure Get(): Integer begin exit(M); end;
    procedure Add(T: Text) begin L += T + ';'; end;
    procedure Take(): Text var R: Text; begin R := L; L := ''; exit(R); end;
}
codeunit 69973 C
{
    var Mode: Codeunit OpMvMode;
    local procedure Seed(N: Integer)
    var
        R: Record "OpMv Setup";
    begin
        R.DeleteAll();
        R."User ID" := 'A'; R.Insert();
        R."User ID" := 'B'; R.Insert();
        R."User ID" := 'C'; R.Name := 'c'; R.Insert();
        R."User ID" := 'D'; R.Insert();
        Mode.Set(N);
    end;
    procedure CardGet(): Text
    var
        P: TestPage "OpMv Card";
    begin
        Seed(1);
        P.OpenView();
        exit(P.UserCtl.Value + '/' + P.NameCtl.Value + '|' + Mode.Take());
    end;
    procedure CardOutsideFilter(): Text
    var
        P: TestPage "OpMv Card";
    begin
        Seed(7);
        P.OpenView();
        exit(P.UserCtl.Value + '|' + Mode.Take());
    end;
    procedure ListGet(): Text
    var
        P: TestPage "OpMv List";
    begin
        Seed(1);
        P.OpenView();
        exit(P.UserCtl.Value + '|' + Mode.Take());
    end;
    procedure ListNewLineThenPrevious(): Text
    var
        P: TestPage "OpMv List";
        T: Text;
    begin
        Seed(2);
        P.OpenEdit();
        Mode.Take();
        T := Format(P.Next()) + '|' + P.UserCtl.Value + '|' + Mode.Take();
        exit(T + Format(P.Previous()) + '|' + P.UserCtl.Value + '|' + Mode.Take());
    end;
    procedure ListKeyChanged(): Text
    var
        P: TestPage "OpMv List";
    begin
        Seed(5);
        P.OpenView();
        exit(P.UserCtl.Value + '|' + Mode.Take());
    end;
    procedure TempCard(): Text
    var
        P: TestPage "OpMv Temp Card";
    begin
        Seed(0);
        P.OpenView();
        exit(P.UserCtl.Value);
    end;
    procedure Document(): Text
    var
        P: TestPage "OpMv Document";
    begin
        Seed(0);
        P.OpenView();
        exit(P.UserCtl.Value);
    end;
    procedure OnlyRow(): Text
    var
        R: Record "OpMv Setup";
        P: TestPage "OpMv Card";
    begin
        R.DeleteAll();
        Mode.Set(0);
        R."User ID" := 'ME';
        R.Insert();
        P.OpenEdit();
        exit(P.UserCtl.Value);
    end;
    procedure NoOnOpenPage(): Text
    var
        R: Record "OpMv Setup";
        P: TestPage "OpMv Plain";
    begin
        R."User ID" := 'B';
        R.Insert();
        R."User ID" := 'A';
        R.Insert();
        P.OpenEdit();
        exit(P.UserCtl.Value);
    end;
}
"#;

#[test]
fn a_card_opens_on_the_row_onopenpage_got() {
    // QrMovedTests.al:175 QR_Card_OpenView_Get `C|bravo`, `O:C;R:C;C:C;`.
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "CardGet")),
        "C/c|O:C;R:C;C:C;"
    );
}

#[test]
fn a_card_whose_rec_is_outside_the_filter_opens_on_the_nearest_row_of_the_view() {
    // QrMovedTests.al:253 QR_Card_OpenView_GetThenFilterExcludes `B|charlie`, `O:C;R:B;C:B;`.
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "CardOutsideFilter")),
        "B|O:C;R:B;C:B;"
    );
}

#[test]
fn a_list_opens_on_the_row_onopenpage_got() {
    // QrMovedTests.al:292 QR_List_OpenView_Get `C|bravo`, `O:C;R:C;R:D;R:C;R:B;R:A;R:C;R:C;C:C;`.
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "ListGet")),
        "C|O:C;R:C;R:D;R:C;R:B;R:A;R:C;R:C;C:C;"
    );
}

#[test]
fn a_list_on_the_moved_rec_goes_back_from_the_new_line_as_a_move() {
    // QrMovedTests.al:615 QR_List_RunModal_FindLast: `next` `Yes|`, `next log` `C:;`, `previous log` `R:D;R:D;C:D;`.
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "ListNewLineThenPrevious")),
        "Yes||C:;Yes|D|R:D;R:D;C:D;"
    );
}

#[test]
fn a_list_whose_key_onopenpage_changed_reads_each_row_once_more() {
    // QrMovedTests.al:345 QR_List_OpenView_SetCurrentKeyGet `B|charlie`, `O:B;R:B;R:B;R:A;R:B;R:C;R:D;R:B;R:B;R:B;C:B;`
    // (here the Name order is A, B, C, D: C and D are named c).
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "ListKeyChanged")),
        "B|O:B;R:B;R:B;R:C;R:D;R:B;R:A;R:B;R:B;R:B;C:B;"
    );
}

#[test]
fn a_temporary_source_filled_in_onopenpage_opens_on_the_last_insert() {
    // QrMovedTests.al:409 QR_TempCard_OpenView_NoMove `D|alpha` (Rec left on the last row inserted).
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "TempCard")), "B");
}

#[test]
fn another_page_type_whose_onopenpage_moves_rec_is_refused() {
    match al_testkit::run_in(SRC, "C", "Document") {
        Err(al_builtins::native::Signal::Unsupported(m)) => assert!(m.contains("OnOpenPage left Rec"), "{m}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn onopenpage_that_inserts_the_only_row_opens_on_it() {
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "OnlyRow")), "ME");
}

#[test]
fn a_page_without_onopenpage_opens_on_the_first_row() {
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "NoOnOpenPage")), "A");
}

/// Pages with a SourceTableView whose OnOpenPage moves Rec, as the QS fixtures of QrMovedTests.al (codeunit 87266, from :856):
/// rows A 'delta', B 'charlie', C 'bravo', D 'alpha'; KeyAsc = sorting(Name), KeyWhere = sorting(Name) where("No." = A..C),
/// Where = where("No." = B..D).
const SRC_VIEW: &str = r#"
table 69980 "OpMv Row" {
    fields { field(1; "No."; Code[10]) { } field(2; Name; Text[30]) { } }
    keys { key(PK; "No.") { Clustered = true; } key(ByName; Name) { } }
}
page 69981 "OpMv KeyAsc Card"
{
    PageType = Card; SourceTable = "OpMv Row"; SourceTableView = sorting(Name); ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(NoCtl; Rec."No.") { } field(NameCtl; Rec.Name) { } } }
    trigger OnOpenPage() begin Log.Move(Rec); Log.Add('O:' + Rec."No."); end;
    trigger OnAfterGetRecord() begin Log.Add('R:' + Rec."No."); end;
    trigger OnAfterGetCurrRecord() begin Log.Add('C:' + Rec."No."); end;
    var Log: Codeunit "OpMv View Log";
}
page 69982 "OpMv KeyAsc List"
{
    PageType = List; SourceTable = "OpMv Row"; SourceTableView = sorting(Name); ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(NoCtl; Rec."No.") { } field(NameCtl; Rec.Name) { } } } }
    trigger OnOpenPage() begin Log.Move(Rec); Log.Add('O:' + Rec."No."); end;
    trigger OnAfterGetRecord() begin Log.Add('R:' + Rec."No."); end;
    trigger OnAfterGetCurrRecord() begin Log.Add('C:' + Rec."No."); end;
    var Log: Codeunit "OpMv View Log";
}
page 69983 "OpMv KeyWhere Card"
{
    PageType = Card; SourceTable = "OpMv Row"; SourceTableView = sorting(Name) where("No." = filter('A..C'));
    ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(NoCtl; Rec."No.") { } field(NameCtl; Rec.Name) { } } }
    trigger OnOpenPage() begin Log.Move(Rec); Log.Add('O:' + Rec."No."); end;
    trigger OnAfterGetRecord() begin Log.Add('R:' + Rec."No."); end;
    trigger OnAfterGetCurrRecord() begin Log.Add('C:' + Rec."No."); end;
    var Log: Codeunit "OpMv View Log";
}
page 69984 "OpMv Where List"
{
    PageType = List; SourceTable = "OpMv Row"; SourceTableView = where("No." = filter('B..D')); ApplicationArea = All;
    UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(NoCtl; Rec."No.") { } field(NameCtl; Rec.Name) { } } } }
    trigger OnOpenPage() begin Log.Move(Rec); Log.Add('O:' + Rec."No."); end;
    trigger OnAfterGetRecord() begin Log.Add('R:' + Rec."No."); end;
    trigger OnAfterGetCurrRecord() begin Log.Add('C:' + Rec."No."); end;
    var Log: Codeunit "OpMv View Log";
}
codeunit 69985 "OpMv View Log"
{
    SingleInstance = true;
    var M: Integer; L: Text;
    procedure Set(N: Integer) begin M := N; L := ''; end;
    procedure Add(T: Text) begin L += T + ';'; end;
    procedure Take(): Text var R: Text; begin R := L; L := ''; exit(R); end;
    procedure Move(var Rec: Record "OpMv Row")
    begin
        case M of
            1: Rec.Get('C');
            4: Rec.FindFirst();
            5: Rec."No." := 'C';
            6: Rec.Get('D');
            7: Rec.Get('A');
        end;
    end;
}
codeunit 69986 V
{
    var Log: Codeunit "OpMv View Log";
    local procedure Seed(N: Integer)
    var
        R: Record "OpMv Row";
    begin
        R.DeleteAll();
        R."No." := 'A'; R.Name := 'delta'; R.Insert();
        R."No." := 'B'; R.Name := 'charlie'; R.Insert();
        R."No." := 'C'; R.Name := 'bravo'; R.Insert();
        R."No." := 'D'; R.Name := 'alpha'; R.Insert();
        Log.Set(N);
    end;
    procedure KeyAscCard(N: Integer): Text
    var
        P: TestPage "OpMv KeyAsc Card";
        T: Text;
    begin
        Seed(N);
        P.OpenView();
        T := P.NoCtl.Value + '/' + P.NameCtl.Value + '|' + Log.Take();
        exit(T + Format(P.Next()) + '|' + P.NoCtl.Value + '|' + Log.Take());
    end;
    procedure KeyAscList(N: Integer): Text
    var
        P: TestPage "OpMv KeyAsc List";
        T: Text;
    begin
        Seed(N);
        P.OpenView();
        T := P.NoCtl.Value + '/' + P.NameCtl.Value + '|' + Log.Take();
        T += Format(P.Next()) + '|' + P.NoCtl.Value + '|' + Log.Take();
        exit(T + Format(P.Previous()) + '|' + P.NoCtl.Value + '|' + Log.Take());
    end;
    procedure KeyWhereCard(N: Integer): Text
    var
        P: TestPage "OpMv KeyWhere Card";
    begin
        Seed(N);
        P.OpenView();
        exit(P.NoCtl.Value + '/' + P.NameCtl.Value + '|' + Log.Take());
    end;
    procedure WhereList(N: Integer): Text
    var
        P: TestPage "OpMv Where List";
    begin
        Seed(N);
        P.OpenView();
        exit(P.NoCtl.Value + '/' + P.NameCtl.Value + '|' + Log.Take());
    end;
}
"#;

fn view(routine: &str, mode: i64) -> String {
    let r = al_testkit::run_with(
        SRC_VIEW,
        "V",
        routine,
        vec![al_builtins::value::Value::Integer(mode as i32)],
    );
    al_testkit::text(r.map(|(v, _)| v))
}

#[test]
fn a_card_in_a_view_key_reads_the_row_onopenpage_got_before_it_opens_on_it() {
    // QrMovedTests.al QS_KeyAsc_Card_GetC: `C|bravo`, `O:C;R:C;R:C;C:C;`, `next` `Yes|B`, `R:B;C:B;`.
    assert_eq!(view("KeyAscCard", 1), "C/bravo|O:C;R:C;R:C;C:C;Yes|B|R:B;C:B;");
}

#[test]
fn a_card_in_a_view_key_finds_a_key_assigned_without_a_read_by_its_stored_row() {
    // QrMovedTests.al QS_KeyAsc_Card_KeyAssigned: "No." := 'C' without a read opens on `C|bravo`, `O:C;R:C;R:C;C:C;`.
    assert_eq!(view("KeyAscCard", 5), "C/bravo|O:C;R:C;R:C;C:C;Yes|B|R:B;C:B;");
}

#[test]
fn a_card_in_a_view_key_on_the_first_row_reads_it_once_more() {
    // QrMovedTests.al QS_KeyAsc_Card_GetD: D is the first row of sorting(Name): `D|alpha`, `O:D;R:D;R:D;C:D;`.
    assert_eq!(view("KeyAscCard", 6), "D/alpha|O:D;R:D;R:D;C:D;Yes|C|R:C;C:C;");
}

#[test]
fn a_card_whose_rec_is_outside_the_view_where_reads_it_then_opens_on_the_nearest_row() {
    // QrMovedTests.al QS_KeyWhere_Card_GetD: D is outside A..C: `C|bravo`, `O:D;R:D;R:C;C:C;`.
    assert_eq!(view("KeyWhereCard", 6), "C/bravo|O:D;R:D;R:C;C:C;");
}

#[test]
fn a_list_in_a_view_key_opens_on_the_row_onopenpage_got() {
    // QrMovedTests.al QS_KeyAsc_List_GetC: `O:C;R:C;R:C;R:B;R:A;R:C;R:D;R:C;R:C;R:C;C:C;`, `next log` `R:B;R:B;R:B;C:B;`,
    // `previous log` `R:C;R:C;R:C;C:C;`.
    assert_eq!(
        view("KeyAscList", 1),
        "C/bravo|O:C;R:C;R:C;R:B;R:A;R:C;R:D;R:C;R:C;R:C;C:C;Yes|B|R:B;R:B;R:B;C:B;Yes|C|R:C;R:C;R:C;C:C;"
    );
}

#[test]
fn a_list_in_a_view_key_that_onopenpage_left_alone_reads_its_first_row_once_more() {
    // QrMovedTests.al QS_KeyAsc_List_NoMove: `O:;R:D;R:C;R:B;R:A;R:D;R:D;R:D;C:D;`, `next log` `R:C;R:C;R:C;C:C;`.
    assert_eq!(
        view("KeyAscList", 0),
        "D/alpha|O:;R:D;R:C;R:B;R:A;R:D;R:D;R:D;C:D;Yes|C|R:C;R:C;R:C;C:C;Yes|D|R:D;R:D;R:D;C:D;"
    );
}

#[test]
fn a_list_in_a_view_key_on_its_first_row_reads_it_first() {
    // QrMovedTests.al QS_KeyAsc_List_FindFirst: `O:D;R:D;R:D;R:C;R:B;R:A;R:D;R:D;R:D;C:D;`.
    assert_eq!(
        view("KeyAscList", 4),
        "D/alpha|O:D;R:D;R:D;R:C;R:B;R:A;R:D;R:D;R:D;C:D;Yes|C|R:C;R:C;R:C;C:C;Yes|D|R:D;R:D;R:D;C:D;"
    );
}

#[test]
fn a_list_whose_rec_is_outside_the_view_where_opens_on_its_first_row() {
    // QrMovedTests.al QS_Where_List_GetA: A is outside B..D: `B|charlie`, `O:A;R:B;R:C;R:D;R:B;R:B;C:B;`.
    assert_eq!(view("WhereList", 7), "B/charlie|O:A;R:B;R:C;R:D;R:B;R:B;C:B;");
}

#[test]
fn a_list_in_a_view_key_on_its_last_row_reads_the_rows_above_and_once_more() {
    // QrMovedTests.al QS_KeyAsc_List_GetA (A is the last row of sorting(Name)): `O:A;R:A;R:A;R:B;R:C;R:D;R:A;R:A;R:A;C:A;`,
    // `next` `No|A` with an empty log, `previous log` `R:B;R:B;R:B;C:B;`.
    assert_eq!(
        view("KeyAscList", 7),
        "A/delta|O:A;R:A;R:A;R:B;R:C;R:D;R:A;R:A;R:A;C:A;No|A|Yes|B|R:B;R:B;R:B;C:B;"
    );
}
