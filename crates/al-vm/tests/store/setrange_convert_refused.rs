//! A value of a pair of types the cells of corpus PR 1552 measure (recordref-fieldref/fieldref/SrcSetRangeConvert.al,
//! SrcFilterConvertKinds.al) is refused in a shape or of a size those cells do not measure (al-vm `setrange_convert`); a value
//! whose cell passes on the earlier conversion keeps it in the measured shape, flagged unless a pin covers that call, and in
//! every shape where a pin or another assumption covers it.

use al_builtins::assumptions::{Assumption, clear_hits, take_hits};
use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::run_in;

const SRC: &str = r#"
table 50290 SRC
{
    fields
    {
        field(1; Id; Integer) { }
        field(3; Cod; Code[20]) { }
        field(4; Int; Integer) { }
        field(5; Bool; Boolean) { }
        field(6; D; Date) { }
        field(7; G; Guid) { }
        field(8; Dec; Decimal) { }
        field(9; DT; DateTime) { }
        field(10; Cod1; Code[1]) { }
        field(11; Txt2; Text[2]) { }
        field(12; Cod10; Code[10]) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}
codeunit 50291 T
{
    procedure GuidSetFilterLetters(): Text
    var R: RecordRef; F: FieldRef; X: Code[20];
    begin
        R.Open(Database::SRC);
        F := R.Field(7);
        X := 'ABC';
        F.SetFilter('%1', X);
        exit(F.GetFilter());
    end;

    procedure GuidSetFilterDigits(): Text
    var R: RecordRef; F: FieldRef; X: Code[20];
    begin
        R.Open(Database::SRC);
        F := R.Field(7);
        X := '123';
        F.SetFilter('%1', X);
        exit(F.GetFilter());
    end;

    procedure GuidSetFilterMixed(): Text
    var R: RecordRef; F: FieldRef; X: Code[20];
    begin
        R.Open(Database::SRC);
        F := R.Field(7);
        X := '1A2B-3C';
        F.SetFilter('%1', X);
        exit(F.GetFilter());
    end;


    procedure GuidSetFilterTextOnTemporary(): Text
    var R: RecordRef; F: FieldRef; T: Text;
    begin
        R.Open(Database::SRC, true);
        F := R.Field(7);
        T := '{1F2E3D4C-5B6A-4978-8695-A4B3C2D1E0F9}';
        F.SetFilter('%1', T);
        exit(F.GetFilter());
    end;

    procedure TrueIntoCodeOnTemporary(): Text
    var R: RecordRef; F: FieldRef; B: Boolean;
    begin
        R.Open(Database::SRC, true);
        F := R.Field(3);
        B := true;
        F.SetRange(B);
        exit(F.GetFilter());
    end;

    procedure DecimalTextIntoDecimalOnTemporary(): Text
    var R: RecordRef; F: FieldRef; T: Text;
    begin
        R.Open(Database::SRC, true);
        F := R.Field(8);
        T := '1.5';
        F.SetRange(T);
        exit(F.GetFilter());
    end;

    procedure DateIntoDateTimeOnTemporary(): Text
    var R: RecordRef; F: FieldRef; D: Date;
    begin
        R.Open(Database::SRC, true);
        F := R.Field(9);
        D := 20240101D;
        F.SetRange(D);
        exit(F.GetFilter());
    end;

    procedure BlankOptionIntoDate(): Text
    var R: RecordRef; F: FieldRef; O: Option " ",One;
    begin
        R.Open(Database::SRC);
        F := R.Field(6);
        O := O::" ";
        F.SetRange(O);
        exit(F.GetFilter());
    end;

    procedure RecordTyped(): Text
    var Rec: Record SRC; C: Code[20];
    begin
        C := 'ABC';
        Rec.SetRange(Int, C);
        exit(Rec.GetFilter(Int));
    end;

    procedure RecordTwoVariants(): Text
    var Rec: Record SRC; V: Variant;
    begin
        V := 'ABC';
        Rec.SetRange(Int, V, V);
        exit(Rec.GetFilter(Int));
    end;

    procedure RecordSetFilterVariant(): Text
    var Rec: Record SRC; V: Variant;
    begin
        V := 'ABC';
        Rec.SetFilter(Int, '%1', V);
        exit(Rec.GetFilter(Int));
    end;

    procedure RecordSetFilterIntegerIntoCode(): Text
    var Rec: Record SRC; I: Integer;
    begin
        I := 5;
        Rec.SetFilter(Cod, '%1', I);
        exit(Rec.GetFilter(Cod));
    end;

    procedure Temporary(): Text
    var R: RecordRef; F: FieldRef; C: Code[20];
    begin
        R.Open(Database::SRC, true);
        F := R.Field(4);
        C := 'ABC';
        F.SetRange(C);
        exit(F.GetFilter());
    end;

    procedure FilterGroupTwo(): Text
    var R: RecordRef; F: FieldRef; C: Code[20];
    begin
        R.Open(Database::SRC);
        R.FilterGroup(2);
        F := R.Field(4);
        C := 'ABC';
        F.SetRange(C);
        exit(F.GetFilter());
    end;

    procedure OtherFilterText(): Text
    var R: RecordRef; F: FieldRef; C: Code[20];
    begin
        R.Open(Database::SRC);
        F := R.Field(4);
        C := 'ABC';
        F.SetFilter('<>%1', C);
        exit(F.GetFilter());
    end;

    procedure KeyRefField(): Text
    var R: RecordRef; K: KeyRef; F: FieldRef; C: Code[20];
    begin
        R.Open(Database::SRC);
        K := R.KeyIndex(1);
        F := K.FieldIndex(1);
        C := 'ABC';
        F.SetRange(C);
        exit(F.GetFilter());
    end;

    procedure IntegerTwoIntoBoolean(): Text
    var R: RecordRef; F: FieldRef; I: Integer;
    begin
        R.Open(Database::SRC);
        F := R.Field(5);
        I := 2;
        F.SetRange(I);
        exit(F.GetFilter());
    end;

    procedure DecimalThreePlacesIntoCode(): Text
    var R: RecordRef; F: FieldRef; X: Decimal;
    begin
        R.Open(Database::SRC);
        F := R.Field(3);
        X := 1.125;
        F.SetRange(X);
        exit(F.GetFilter());
    end;

    procedure EmptyTextIntoGuid(): Text
    var R: RecordRef; F: FieldRef; T: Text;
    begin
        R.Open(Database::SRC);
        F := R.Field(7);
        T := '';
        F.SetRange(T);
        exit(F.GetFilter());
    end;

    procedure NegativeIntegerIntoDate(): Text
    var R: RecordRef; F: FieldRef; I: Integer;
    begin
        R.Open(Database::SRC);
        F := R.Field(6);
        I := -3;
        F.SetRange(I);
        exit(F.GetFilter());
    end;

    procedure DigitsIntoIntegerOnTemporary(): Text
    var R: RecordRef; F: FieldRef; C: Code[20];
    begin
        R.Open(Database::SRC, true);
        F := R.Field(4);
        C := '123';
        F.SetRange(C);
        exit(F.GetFilter());
    end;

    procedure TextIntoCodeOnRecord(): Text
    var Rec: Record SRC; T: Text;
    begin
        T := 'abc';
        Rec.SetRange(Cod, T);
        exit(Rec.GetFilter(Cod));
    end;
    procedure DigitsIntoDecimalOnTemporary(): Text
    var R: RecordRef; F: FieldRef; T: Text;
    begin
        R.Open(Database::SRC, true);
        F := R.Field(8);
        T := '5';
        F.SetRange(T);
        exit(F.GetFilter());
    end;

    procedure GuidSetFilterHexNoHyphens(): Text
    var R: RecordRef; F: FieldRef; T: Text;
    begin
        R.Open(Database::SRC);
        F := R.Field(7);
        T := 'ABCDEFABCDEFABCDEFABCDEFABCDEFAB';
        F.SetFilter('%1', T);
        exit(F.GetFilter());
    end;

    procedure FalseIntoCode(): Text
    var R: RecordRef; F: FieldRef; B: Boolean;
    begin
        R.Open(Database::SRC);
        F := R.Field(3);
        B := false;
        F.SetFilter('%1', B);
        exit(F.GetFilter());
    end;

    procedure FalseIntoCode1(): Text
    var R: RecordRef; F: FieldRef; B: Boolean;
    begin
        R.Open(Database::SRC);
        F := R.Field(10);
        B := false;
        F.SetRange(B);
        exit(F.GetFilter());
    end;

    procedure TrueIntoCode1(): Text
    var R: RecordRef; F: FieldRef; B: Boolean;
    begin
        R.Open(Database::SRC);
        F := R.Field(10);
        B := true;
        F.SetRange(B);
        exit(F.GetFilter());
    end;

    procedure FalseIntoCodeOnTemporary(): Text
    var R: RecordRef; F: FieldRef; B: Boolean;
    begin
        R.Open(Database::SRC, true);
        F := R.Field(3);
        B := false;
        F.SetRange(B);
        exit(F.GetFilter());
    end;

    procedure TrueIntoText2(): Text
    var R: RecordRef; F: FieldRef; B: Boolean;
    begin
        R.Open(Database::SRC);
        F := R.Field(11);
        B := true;
        F.SetRange(B);
        exit(F.GetFilter());
    end;

    procedure FalseIntoCode10(): Text
    var R: RecordRef; F: FieldRef; B: Boolean;
    begin
        R.Open(Database::SRC);
        F := R.Field(12);
        B := false;
        F.SetRange(B);
        exit(F.GetFilter());
    end;

    procedure TrueIntoCode(): Text
    var R: RecordRef; F: FieldRef; B: Boolean;
    begin
        R.Open(Database::SRC);
        F := R.Field(3);
        B := true;
        F.SetRange(B);
        exit(F.GetFilter());
    end;

    procedure DecimalTextVariant(): Text
    var R: RecordRef; F: FieldRef; V: Variant;
    begin
        R.Open(Database::SRC);
        F := R.Field(8);
        V := '1.5';
        F.SetRange(V);
        exit(F.GetFilter());
    end;

    procedure DecimalCodeVariant(): Text
    var R: RecordRef; F: FieldRef; V: Variant; C: Code[10];
    begin
        R.Open(Database::SRC);
        F := R.Field(8);
        C := '1.5';
        V := C;
        F.SetRange(V);
        exit(F.GetFilter());
    end;

    procedure DecimalOtherTextVariant(): Text
    var R: RecordRef; F: FieldRef; V: Variant;
    begin
        R.Open(Database::SRC);
        F := R.Field(8);
        V := '2.25';
        F.SetRange(V);
        exit(F.GetFilter());
    end;

    procedure DecimalTextVariantOnTemporary(): Text
    var R: RecordRef; F: FieldRef; V: Variant;
    begin
        R.Open(Database::SRC, true);
        F := R.Field(8);
        V := '1.5';
        F.SetRange(V);
        exit(F.GetFilter());
    end;
}
"#;

fn refused(routine: &str) {
    match run_in(SRC, "T", routine) {
        Err(Signal::Unsupported(reason)) => {
            assert!(reason.contains("in a shape no cell measures"), "{routine}: {reason}")
        }
        other => panic!("{routine}: expected a refusal, got {other:?}"),
    }
}

fn filter(routine: &str) -> String {
    match run_in(SRC, "T", routine) {
        Ok(Value::Text(s)) => s.to_string(),
        other => panic!("{routine}: expected a filter, got {other:?}"),
    }
}

#[test]
fn a_measured_pair_in_another_shape_is_refused() {
    for routine in [
        "RecordTyped",
        "RecordTwoVariants",
        "RecordSetFilterVariant",
        "RecordSetFilterIntegerIntoCode",
        "Temporary",
        "FilterGroupTwo",
        "OtherFilterText",
        "KeyRefField",
    ] {
        refused(routine);
    }
}

#[test]
fn a_value_no_cell_measures_is_refused() {
    for routine in [
        "IntegerTwoIntoBoolean",
        "DecimalThreePlacesIntoCode",
        "EmptyTextIntoGuid",
        "NegativeIntegerIntoDate",
    ] {
        refused(routine);
    }
}

#[test]
fn a_cell_that_passes_on_the_earlier_conversion_keeps_it() {
    // Code123_Int: digits into an Integer field are that number, also on a temporary table, through the assumption
    // text-digits-to-integer (the cover `Cell::MainEveryShape` names for this arm).
    clear_hits();
    assert_eq!(filter("DigitsIntoIntegerOnTemporary"), "123");
    assert!(take_hits().contains(&Assumption::TextDigitsToInteger));
    // A Text into a Code field is no pair the cells measure.
    assert_eq!(filter("TextIntoCodeOnRecord"), "ABC");
}

/// SrcFilterConvertKinds.al:1730-1819 (CodeABC_G, Code123_G, TxtXY_G) route e: `FieldRef.SetFilter('%1', X)` of a typed text that
/// is no Guid raises the Guid format error itself, without the filter error around it.
#[test]
fn a_typed_text_that_is_no_guid_in_setfilter_raises_the_guid_format_error() {
    for x in ["GuidSetFilterLetters", "GuidSetFilterDigits"] {
        match run_in(SRC, "T", x) {
            Err(Signal::Error(e)) => {
                assert_eq!(e.code, "InvalidGuidFormat", "{x}");
                assert_eq!(e.message, al_builtins::native::INVALID_GUID_FORMAT, "{x}");
            }
            other => panic!("{x}: expected the Guid format error, got {other:?}"),
        }
    }
    // A text of another form than letters or digits (here: hex digits and a hyphen) is not measured, nor are 32 hexadecimal
    // digits without hyphens (letters only, main's Guid for an assignment).
    refused("GuidSetFilterMixed");
    refused("GuidSetFilterHexNoHyphens");
}

/// The cells that pass on the earlier conversion are measured on a table that is not temporary only.
#[test]
fn a_cell_of_the_earlier_conversion_in_another_shape_is_refused() {
    for routine in [
        "GuidSetFilterTextOnTemporary",
        "TrueIntoCodeOnTemporary",
        "FalseIntoCodeOnTemporary",
        "DecimalTextIntoDecimalOnTemporary",
        "DateIntoDateTimeOnTemporary",
        "DigitsIntoDecimalOnTemporary",
        "DecimalTextVariantOnTemporary",
    ] {
        refused(routine);
    }
}

/// In the measured shape a cell that passes on the earlier conversion runs through the entry setrange-other-type-convert
/// (BoolTrue_Cod, open corpus PR 1552), unless a pin covers that call: `1.5` in a Variant given to FieldRef.SetRange
/// (FieldRefFilterMatrix.al:819,822).
#[test]
fn a_cell_of_the_earlier_conversion_in_the_measured_shape_is_flagged_unless_pinned() {
    clear_hits();
    assert_eq!(filter("TrueIntoCode"), "YES");
    assert!(take_hits().contains(&Assumption::SetRangeOtherTypeConvert));
    clear_hits();
    assert_eq!(filter("DecimalTextVariant"), "1.5");
    assert!(!take_hits().contains(&Assumption::SetRangeOtherTypeConvert));
    // The pin measures the Text '1.5' only: a Code, or another Decimal text, in the same call runs through the entry.
    for (routine, expected) in [("DecimalCodeVariant", "1.5"), ("DecimalOtherTextVariant", "2.25")] {
        clear_hits();
        assert_eq!(filter(routine), expected, "{routine}");
        assert!(take_hits().contains(&Assumption::SetRangeOtherTypeConvert), "{routine}");
    }
}

/// SrcSetRangeConvert.al Opt_D_* measures the Option member `One`; a blank member is not measured.
#[test]
fn a_blank_option_member_into_a_date_is_refused() {
    refused("BlankOptionIntoDate");
}

/// False into a Code field is main's answer, `NO` (SrcFilterBoolIntoText.al:178-192 BoolFalse_Cod, corpus PR 1575, al-runner issue 9026),
/// in the measured shape through the entry setrange-bool-into-text; refused on a temporary table
/// (`a_cell_of_the_earlier_conversion_in_another_shape_is_refused`).
#[test]
fn false_into_a_code_field_is_its_format_text_through_its_own_entry() {
    clear_hits();
    assert_eq!(filter("FalseIntoCode"), "NO");
    let hits = take_hits();
    assert!(hits.contains(&Assumption::SetRangeBoolIntoText), "{hits:?}");
    assert!(!hits.contains(&Assumption::SetRangeOtherTypeConvert), "{hits:?}");
}

/// No cell measures true into a Text[2] field or false into a Code[10] field (corpus PR 1575 measures lengths 1, 2, 20 and 50): both
/// refuse, and the message names the entry setrange-bool-into-text. Main ran true into Text[2] and answered `Yes`.
#[test]
fn a_boolean_into_a_length_no_cell_measures_is_refused_naming_its_entry() {
    for routine in ["TrueIntoText2", "FalseIntoCode10"] {
        match run_in(SRC, "T", routine) {
            Err(Signal::Unsupported(reason)) => {
                assert!(reason.contains("in a shape no cell measures"), "{routine}: {reason}");
                assert!(
                    reason.contains("(assumption setrange-bool-into-text)"),
                    "{routine}: {reason}"
                );
            }
            other => panic!("{routine}: expected a refusal, got {other:?}"),
        }
    }
}

/// SrcFilterBoolIntoText.al:226-240 (BoolFalse_Cod1) and 274-288 (BoolTrue_Cod1, corpus PR 1575): a Boolean into a Code[1] field
/// raises the string-length error with the Format text of the value (`No`, `Yes`, not upper case), through the entry
/// setrange-bool-into-text.
#[test]
fn a_boolean_longer_than_a_code_field_raises_the_length_error_with_its_format_text() {
    for (routine, message) in [
        (
            "FalseIntoCode1",
            "The length of the string is 2, but it must be less than or equal to 1 characters. Value: No",
        ),
        (
            "TrueIntoCode1",
            "The length of the string is 3, but it must be less than or equal to 1 characters. Value: Yes",
        ),
    ] {
        clear_hits();
        match run_in(SRC, "T", routine) {
            Err(Signal::Error(e)) => {
                assert_eq!(e.code, "StringLengthExceeded", "{routine}");
                assert_eq!(e.message, message, "{routine}");
            }
            other => panic!("{routine}: expected the length error, got {other:?}"),
        }
        let hits = take_hits();
        assert!(hits.contains(&Assumption::SetRangeBoolIntoText), "{routine}: {hits:?}");
        assert!(
            !hits.contains(&Assumption::SetRangeOtherTypeConvert),
            "{routine}: {hits:?}"
        );
    }
}
