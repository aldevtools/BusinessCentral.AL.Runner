//! A new Card without parts: a field outside the key typed first stores the row with a blank key, the key typed next is dropped,
//! and Close saves the pending edit (pages/lt-part-link-unsaved-host/PagePartLinkUnsavedHost.al:482 PYB_NoPart_DescrFirstThenKey).

const SRC: &str = r#"
table 51900 "NDK Host" { fields { field(1; Code; Code[10]) { } field(2; Descr; Text[30]) { } } keys { key(PK; Code) { Clustered = true; } } }
page 51901 "NDK Card" { PageType = Card; SourceTable = "NDK Host";
    layout { area(Content) { field(CodeCtl; Rec.Code) { } field(DescrCtl; Rec.Descr) { } } } }
table 51903 "NDK Line" { fields { field(1; "Host Code"; Code[10]) { } field(2; "Line No."; Integer) { } } keys { key(PK; "Host Code", "Line No.") { Clustered = true; } } }
page 51904 "NDK Lines" { PageType = ListPart; SourceTable = "NDK Line";
    layout { area(Content) { repeater(Rows) { field(NoCtl; Rec."Line No.") { } } } } }
page 51905 "NDK Card Part" { PageType = Card; SourceTable = "NDK Host";
    layout { area(Content) { field(CodeCtl; Rec.Code) { } field(DescrCtl; Rec.Descr) { }
        part(LinesPart; "NDK Lines") { SubPageLink = "Host Code" = field(Code); } } } }
codeunit 51902 C
{
    procedure Rows(): Text
    var H: Record "NDK Host"; R: Text;
    begin
        if H.FindSet() then repeat R += '[' + H.Code + '|' + H.Descr + ']'; until H.Next() = 0;
        exit(Format(H.Count()) + ' ' + R);
    end;
    procedure DescrThenKeyWithPart(): Text
    var P: TestPage "NDK Card Part"; s: Text;
    begin
        P.OpenNew();
        P.DescrCtl.SetValue('d1');
        s := Rows();
        P.CodeCtl.SetValue('G1');
        s += '|' + Rows() + '|[' + P.CodeCtl.Value() + ']';
        P.Close();
        exit(s + '|' + Rows());
    end;
    procedure DescrThenKey(): Text
    var P: TestPage "NDK Card"; s: Text;
    begin
        P.OpenNew();
        P.DescrCtl.SetValue('d1');
        s := Rows();
        P.CodeCtl.SetValue('G1');
        s += '|' + Rows() + '|[' + P.CodeCtl.Value() + ']';
        P.Close();
        exit(s + '|' + Rows());
    end;
}
"#;

// PYB_NoPart_DescrFirstThenKey: `db_after_descr` `1 [|]`, `db_after_code` `1 [|]`, `code_value` `[]`, `db_after_close` `1 [|d1]`.
#[test]
fn the_key_typed_after_a_field_outside_it_is_dropped_and_the_edit_is_saved_at_close() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "DescrThenKey")),
        "1 [|]|1 [|]|[]|1 [|d1]"
    );
}

// PYB_TwoParts_DescrFirstThenKey (the same with parts linked on the key): the key is dropped there too.
#[test]
fn the_key_typed_on_a_stored_row_of_a_card_with_a_part_is_dropped_too() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "DescrThenKeyWithPart")),
        "1 [|]|1 [|]|[]|1 [|d1]"
    );
}
