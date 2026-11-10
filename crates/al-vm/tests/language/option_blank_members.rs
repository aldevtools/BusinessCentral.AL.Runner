//! Blank option members: `""` and an unquoted empty member are the same nameless member and format as their ordinal at any
//! position, `" "` is a member of its own and formats as a space (corpus format-evaluate/format-matrix/FMOptionBlankVar.al,
//! FMOptionBlankEnum.al, FMOptionBlankField.al; the nameless member of a `Base Application` option, `OptionMembers = ,Customer`).

use al_testkit::text;

const SRC: &str = r#"
enum 69801 "OB E MQ" { value(0; A) { } value(1; "") { } value(2; B) { } }
enum 69802 "OB E MQ Cap" { value(0; A) { } value(1; "") { Caption = 'Cap1'; } value(2; B) { } }
enum 69803 "OB E FS" { value(0; " ") { } value(1; A) { } }
codeunit 69800 C
{
    procedure Unquoted(): Text
    var O: Option A,,B;
    begin
        O := 1;
        exit('[' + Format(O) + ']');
    end;

    procedure Quoted(): Text
    var O: Option A,"",B;
    begin
        O := 1;
        exit('[' + Format(O) + '][' + Format(O, 0, 9) + ']');
    end;

    procedure Spaced(): Text
    var O: Option A," ",B;
    begin
        O := 1;
        exit('[' + Format(O) + '][' + Format(O, 0, 9) + ']');
    end;

    procedure EvalBlankIntoSpaceMember(): Text
    var O: Option A," ",B; R: Boolean;
    begin
        O := 2;
        R := Evaluate(O, '');
        exit(Format(R) + '|' + Format(O, 0, 2));
    end;

    procedure EvalBlankIntoNamelessMember(): Text
    var O: Option A,"",B; R: Boolean;
    begin
        O := 2;
        R := Evaluate(O, ' ');
        exit(Format(R) + '|' + Format(O, 0, 2));
    end;

    procedure EvalStatementError(): Text
    var O: Option A,"",B;
    begin
        O := 2;
        asserterror Evaluate(O, 'zzz');
        exit(GetLastErrorText());
    end;

    procedure EvalNulError(): Text
    var O: Option A,,B; C: Char; T: Text;
    begin
        C := 0;
        T += C;
        asserterror Evaluate(O, T);
        exit(GetLastErrorText());
    end;

    procedure EnumNameless(): Text
    var E: Enum "OB E MQ";
    begin
        E := Enum::"OB E MQ".FromInteger(1);
        exit('[' + Format(E) + '][' + Format(E, 0, 2) + ']');
    end;

    procedure EnumNamelessCaption(): Text
    var E: Enum "OB E MQ Cap"; R: Boolean;
    begin
        E := Enum::"OB E MQ Cap".FromInteger(1);
        exit('[' + Format(E) + ']');
    end;

    procedure EnumNamelessCaptionEval(): Text
    var E: Enum "OB E MQ Cap"; R: Boolean;
    begin
        E := Enum::"OB E MQ Cap".FromInteger(0);
        R := Evaluate(E, '');
        exit(Format(R) + '|' + Format(E, 0, 2));
    end;

    procedure EnumNamelessEval(): Text
    var E: Enum "OB E MQ"; R: Boolean;
    begin
        E := Enum::"OB E MQ".FromInteger(0);
        R := Evaluate(E, '', 2);
        exit(Format(R) + '|' + Format(E, 0, 2));
    end;
}
"#;

#[test]
fn an_unquoted_empty_member_formats_as_its_ordinal() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "Unquoted")), "[1]");
}

#[test]
fn a_quoted_empty_member_is_the_same_nameless_member() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "Quoted")), "[1][1]");
}

#[test]
fn a_space_member_formats_as_a_space_and_as_its_ordinal_in_format_9() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "Spaced")), "[ ][1]");
}

#[test]
fn blank_text_reads_the_space_member_at_any_place() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "EvalBlankIntoSpaceMember")), "Yes|1");
}

#[test]
fn blank_text_does_not_read_the_nameless_member() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "EvalBlankIntoNamelessMember")),
        "No|2"
    );
}

#[test]
fn the_error_list_shows_a_nameless_member_as_an_empty_slot() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "EvalStatementError")),
        "'zzz' is not an option. The existing options are: A,,B"
    );
}

#[test]
fn a_nul_text_shows_as_u0000_in_the_error() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "EvalNulError")),
        "'\\u0000' is not an option. The existing options are: A,,B"
    );
}

#[test]
fn an_enum_value_named_empty_formats_as_its_ordinal_at_any_place() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "EnumNameless")), "[1][1]");
}

#[test]
fn a_caption_replaces_the_nameless_enum_value_in_format() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "EnumNamelessCaption")), "[Cap1]");
}

#[test]
fn a_caption_on_the_nameless_enum_value_stops_the_blank_text_from_reading_it() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "EnumNamelessCaptionEval")), "No|0");
}

#[test]
fn an_enum_reads_format_number_2_as_0() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "EnumNamelessEval")), "Yes|1");
}
