//! `Format` of a DateTime shows the year with two digits from 1950 to 2049 and four otherwise, in formats 0 to 3 of the year
//! 9999 too; through a Variant, format 3 does not exist (types/lt-y49-datetime-9999/Y49DateTime9999.al:64-209, corpus master).

const SRC: &str = r#"
codeunit 50769 C {
    procedure Max(): Text
    var DT: DateTime;
    begin
        DT := CreateDateTime(99991231D, 235959.999T);
        exit(Format(DT, 0, 0) + '|' + Format(DT, 0, 1) + '|' + Format(DT, 0, 2) + '|' + Format(DT, 0, 3) + '|' + Format(DT, 0, 9));
    end;
    procedure Width(): Text
    var Y: Integer; Res: Text; Years: List of [Integer];
    begin
        Years.AddRange(1949, 1950, 2049, 2050);
        foreach Y in Years do
            Res += Format(CreateDateTime(DMY2Date(15, 6, Y), 120000T), 0, 2) + ' ' + Format(DMY2Date(15, 6, Y), 0, 1) + '|';
        exit(Res);
    end;
    procedure VariantFormat3(): Text
    var V: Variant;
    begin
        V := CreateDateTime(99991231D, 235959.999T);
        exit(Format(V, 0, 3));
    end;
}
"#;

#[test]
fn a_datetime_in_9999_shows_four_year_digits_in_every_format() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "Max")),
        "12/31/9999 11:59 PM|12/31/9999 11:59:59.999 PM|12/31/9999 11:59 PM|12/31/9999 11:59:59 PM|9999-12-31T23:59:59.999Z"
    );
}

#[test]
fn the_year_has_two_digits_from_1950_to_2049() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "Width")),
        "06/15/1949 12:00 PM 06/15/1949|06/15/50 12:00 PM 06/15/50|06/15/49 12:00 PM 06/15/49|06/15/2050 12:00 PM 06/15/2050|"
    );
}

#[test]
fn format_3_of_a_datetime_in_a_variant_does_not_exist() {
    assert_eq!(
        al_testkit::error_message(al_testkit::run_in(SRC, "C", "VariantFormat3")),
        "Standard format number 3 does not exist for the type 'Variant'."
    );
}

const DF: &str = r#"
codeunit 50779 D {
    procedure Format2(): Text
    var F: DateFormula;
    begin
        Evaluate(F, '<-1W>');
        exit(Format(F, 0, 2));
    end;
    procedure Format1(): Text
    var F: DateFormula;
    begin
        Evaluate(F, '<-1W>');
        exit(Format(F, 0, 1));
    end;
}
"#;

/// Format 0 and 9 of a DateFormula are pinned (misc/lt-system-functions/RandomFormat.al:484-505); format 2 shows the angle
/// brackets and format 1 in English none (types/lt-y62-dateformula-format/Y62DateFormulaFormat.al:DateFormulaFormat_Std1_English,
/// _Std2_English).
#[test]
fn format_2_of_a_date_formula_is_bracketed() {
    assert_eq!(al_testkit::text(al_testkit::run_in(DF, "D", "Format2")), "<-1W>");
}

#[test]
fn format_1_of_a_date_formula_in_english_has_no_brackets() {
    assert_eq!(al_testkit::text(al_testkit::run_in(DF, "D", "Format1")), "-1W");
}
