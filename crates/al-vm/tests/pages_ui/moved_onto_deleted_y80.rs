//! A move of a plain list onto a row that code deleted after the list read it (corpus pages/deleted-rows/Y80GoneRows.al,
//! the Y80_Moved_All_* and Y80_Moved_NoNew_* tests; the values below are the BC 29 `Expect` lines, the table has no OnDelete here).
//! BC 27 and BC 28 values are checked by the corpus run (`--bc-version`).

const SRC: &str = r#"
table 51700 "Y8M Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
codeunit 51701 "Y8M Log"
{
    SingleInstance = true;
    var E: Text;
    procedure Add(T: Text) begin if E <> '' then E += ';'; E += T; end;
    procedure Take() R: Text begin R := E; E := ''; end;
}
page 51702 "Y8M All" { PageType = List; SourceTable = "Y8M Row";
    layout { area(Content) { repeater(Rows) { field(IdCtl; Rec.Id) { } } } }
    trigger OnAfterGetRecord() begin L.Add('R:' + Format(Rec.Id)); end;
    trigger OnAfterGetCurrRecord() begin L.Add('C:' + Format(Rec.Id)); end;
    trigger OnNewRecord(BelowxRec: Boolean) begin L.Add('N:' + Format(Rec.Id)); end;
    var L: Codeunit "Y8M Log"; }
page 51703 "Y8M NoNew" { PageType = List; SourceTable = "Y8M Row"; InsertAllowed = false;
    layout { area(Content) { repeater(Rows) { field(IdCtl; Rec.Id) { } } } }
    trigger OnAfterGetRecord() begin L.Add('R:' + Format(Rec.Id)); end;
    trigger OnAfterGetCurrRecord() begin L.Add('C:' + Format(Rec.Id)); end;
    var L: Codeunit "Y8M Log"; }
page 51704 "Y8M ReadOnly" { PageType = List; SourceTable = "Y8M Row"; Editable = false;
    layout { area(Content) { repeater(Rows) { field(IdCtl; Rec.Id) { } } } }
    trigger OnAfterGetRecord() begin L.Add('R:' + Format(Rec.Id)); end;
    trigger OnAfterGetCurrRecord() begin L.Add('C:' + Format(Rec.Id)); end;
    var L: Codeunit "Y8M Log"; }
codeunit 51705 C
{
    procedure Seed()
    var R: Record "Y8M Row"; i: Integer;
    begin
        for i := 1 to 4 do begin R.Id := i * 10; R.Insert(); end;
    end;
    // Move `Which` on the list "All" after row `Gone` was deleted; `First` rows of moves lead to the cursor row.
    procedure All(Lead: Text; Gone: Integer; Which: Text): Text
    var R: Record "Y8M Row"; L: Codeunit "Y8M Log"; P: TestPage "Y8M All"; s: Text; ok: Boolean;
    begin
        Seed();
        P.OpenEdit();
        if Lead = 'last' then P.Last();
        if Lead = 'lastprevious' then begin P.Last(); P.Previous(); end;
        L.Take();
        R.Get(Gone); R.Delete();
        case Which of
            'next': ok := P.Next();
            'previous': ok := P.Previous();
            'first': ok := P.First();
            'last': ok := P.Last();
            'goto': ok := P.GoToKey(Gone);
        end;
        s := Format(ok) + '|' + L.Take() + '|' + P.IdCtl.Value();
        ok := P.Next();
        s += '|' + Format(ok) + '|' + L.Take() + '|' + P.IdCtl.Value();
        exit(s);
    end;
    procedure NoNew(Lead: Text; Gone: Integer; Which: Text): Text
    var R: Record "Y8M Row"; L: Codeunit "Y8M Log"; P: TestPage "Y8M NoNew"; s: Text; ok: Boolean;
    begin
        Seed();
        P.OpenEdit();
        if Lead = 'lastprevious' then begin P.Last(); P.Previous(); end;
        L.Take();
        R.Get(Gone); R.Delete();
        case Which of
            'next': ok := P.Next();
            'goto': ok := P.GoToKey(Gone);
        end;
        s := Format(ok) + '|' + L.Take() + '|' + P.IdCtl.Value();
        exit(s);
    end;
    procedure ReadOnly(): Text
    var R: Record "Y8M Row"; P: TestPage "Y8M ReadOnly";
    begin
        Seed();
        P.OpenEdit();
        R.Get(20); R.Delete();
        exit(Format(P.Next()));
    end;
}
"#;

fn call(routine: &str, args: Vec<al_builtins::value::Value>) -> String {
    al_testkit::text(al_testkit::run_with(SRC, "C", routine, args).map(|(v, _)| v))
}

fn t(s: &str) -> al_builtins::value::Value {
    al_builtins::value::Value::Text(s.into())
}

fn i(n: i32) -> al_builtins::value::Value {
    al_builtins::value::Value::Integer(n)
}

// Y80_Moved_All_NextOntoDeleted (:1677): the cursor stands on 10, 20 is gone.
#[test]
fn next_onto_a_deleted_row_reads_the_rows_around_the_cursor_row_again() {
    assert_eq!(
        call("All", vec![t(""), i(20), t("next")]),
        "Yes|R:10;C:10;R:10;R:30;R:40;R:10;R:10;C:10|10|Yes|R:30;R:30;C:30|30"
    );
}

// Y80_Moved_All_PreviousOntoDeleted (:1715): the cursor stands on 30, 20 is gone.
#[test]
fn previous_onto_a_deleted_row_reads_the_rows_above_and_below_the_cursor_row() {
    assert_eq!(
        call("All", vec![t("lastprevious"), i(20), t("previous")]),
        "Yes|R:30;C:30;R:30;R:40;R:30;R:10;R:30;R:30;C:30|30|Yes|R:40;R:40;C:40|40"
    );
}

// Y80_Moved_All_FirstOntoDeleted (:1785): the cursor stands on the last row 40, 10 is gone; Next then reaches the blank line.
#[test]
fn first_onto_a_deleted_row_keeps_the_cursor_and_the_blank_line_follows() {
    assert_eq!(
        call("All", vec![t("last"), i(10), t("first")]),
        "Yes|R:40;C:40;R:40;R:30;R:20;R:40;R:40;C:40|40|Yes|N:0;C:0|0"
    );
}

// Y80_Moved_All_LastOntoDeleted (:1854): the blank line is reached first, then the cursor row is read again.
#[test]
fn last_onto_a_deleted_row_visits_the_blank_line_first() {
    assert_eq!(
        call("All", vec![t(""), i(40), t("last")]),
        "Yes|N:0;C:0;R:10;R:20;R:30;R:10;R:10;C:10|10|Yes|R:20;R:20;C:20|20"
    );
}

// Y80_Moved_All_GoToKeyDeleted (:1922): the scan goes on over the rows held afterwards and ends on the blank line.
#[test]
fn go_to_key_on_a_deleted_key_scans_the_rows_held_afterwards() {
    assert_eq!(
        call("All", vec![t(""), i(20), t("goto")]),
        "No|R:10;C:10;R:10;R:30;R:40;R:10;R:10;C:10;R:30;R:30;C:30;R:40;R:40;C:40;N:0;C:0;R:10;R:10;C:10|10|Yes|R:30;R:30;C:30|30"
    );
}

// Y80_Moved_NoNew_NextOntoDeleted (:1960) and _GoToKeyDeleted (:2205): the same without the blank line.
#[test]
fn the_list_without_the_blank_line_moves_the_same_way() {
    assert_eq!(
        call("NoNew", vec![t(""), i(20), t("next")]),
        "Yes|R:10;C:10;R:10;R:30;R:40;R:10;R:10;C:10|10"
    );
    assert_eq!(
        call("NoNew", vec![t(""), i(20), t("goto")]),
        "No|R:10;C:10;R:10;R:30;R:40;R:10;R:10;C:10;R:30;R:30;C:30;R:40;R:40;C:40;R:10;R:10;C:10|10"
    );
}

// Only the shape of those pages is pinned: an Editable = false list keeps the refusal.
#[test]
fn a_read_only_list_moving_onto_a_deleted_row_is_still_refused() {
    let r = al_testkit::run_in(SRC, "C", "ReadOnly");
    assert!(matches!(&r, Err(al_builtins::native::Signal::Unsupported(_))), "{r:?}");
}
