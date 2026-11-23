//! A value of a pair of types the cells of corpus PR 1552 measure (recordref-fieldref/fieldref/SrcSetRangeConvert.al,
//! SrcFilterConvertKinds.al) is refused in a shape or of a size those cells do not measure (al-vm `setrange_convert`); a value
//! whose cell passes on the earlier conversion keeps it in every shape.

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
    }
    keys { key(PK; Id) { Clustered = true; } }
}
codeunit 50291 T
{
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
    // Code123_Int: digits into an Integer field are that number, also on a temporary table.
    assert_eq!(filter("DigitsIntoIntegerOnTemporary"), "123");
    // A Text into a Code field is no pair the cells measure.
    assert_eq!(filter("TextIntoCodeOnRecord"), "ABC");
}
