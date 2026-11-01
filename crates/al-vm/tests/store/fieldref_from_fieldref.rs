//! `FieldRef.Validate(OtherFieldRef)` and `FieldRef.Value := OtherFieldRef` (al-runner issue 6591): the source field's value
//! goes in, converted as a value of its type would be; Validate runs the target's OnValidate, Value does not
//! (recordref/lt-y47-validate-fieldref/Y47ValidateFieldRef.al:231-402).

const SRC: &str = r#"
table 69995 "FRF Doc"
{
    fields
    {
        field(1; "No."; Code[10]) { }
        field(2; Description; Text[20]) { trigger OnValidate() begin Validated += 1; end; }
        field(3; Amount; Decimal) { }
        field(4; Num; Integer) { }
        field(5; Short; Code[20]) { }
        field(6; Validated; Integer) { }
    }
    keys { key(PK; "No.") { } }
}
codeunit 69996 C
{
    local procedure Copy(FieldNo: Integer; TargetNo: Integer; Validate: Boolean): Text
    var
        Source: Record "FRF Doc";
        Target: Record "FRF Doc";
        SourceRef: RecordRef;
        TargetRef: RecordRef;
        SourceField: FieldRef;
        TargetField: FieldRef;
    begin
        Source."No." := 'S';
        Source.Description := 'Hello';
        Source.Amount := 12.5;
        Source.Num := 42;
        Source.Insert();
        Target."No." := 'T';
        Target.Insert();
        SourceRef.GetTable(Source);
        TargetRef.GetTable(Target);
        SourceField := SourceRef.Field(FieldNo);
        TargetField := TargetRef.Field(TargetNo);
        if Validate then
            TargetField.Validate(SourceField)
        else
            TargetField.Value := SourceField;
        TargetRef.Modify();
        Target.Get('T');
        exit(Format(Target.Description) + '/' + Format(Target.Amount) + '/' + Target.Short + '/' + Format(Target.Validated));
    end;

    procedure F(): Text
    begin
        exit(Copy(2, 2, true));
    end;

    procedure Dec(): Text
    begin
        exit(Copy(3, 3, true));
    end;

    procedure ToCode(): Text
    begin
        exit(Copy(2, 5, true));
    end;

    procedure TextToInt(): Text
    begin
        exit(Copy(2, 4, true));
    end;

    procedure Assign(): Text
    begin
        exit(Copy(2, 2, false));
    end;
}
"#;

#[test]
fn validate_with_a_fieldref_takes_the_source_value_and_runs_onvalidate() {
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "F")), "Hello/0//1");
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "Dec")), "/12.5//0");
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "ToCode")), "/0/HELLO/0");
}

#[test]
fn value_from_a_fieldref_takes_the_source_value_without_onvalidate() {
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "Assign")), "Hello/0//0");
}

const MORE: &str = r#"
table 69997 "FRF Row"
{
    fields
    {
        field(1; "No."; Code[10]) { }
        field(2; Description; Text[20]) { }
        field(3; Num; Integer) { }
        field(4; Flag; Boolean) { }
    }
    keys { key(PK; "No.") { } }
}
codeunit 69998 D
{
    procedure F(): Text
    var
        R: Record "FRF Row";
        SourceRef: RecordRef;
        TargetRef: RecordRef;
        TargetField: FieldRef;
        S: Text;
    begin
        R."No." := 'S'; R.Description := 'Hello'; R.Num := 42; R.Flag := true; R.Insert();
        R."No." := 'T'; R.Description := 'Other'; R.Num := 7; R.Flag := false; R.Insert();
        R.Get('S');
        SourceRef.GetTable(R);
        TargetRef.Open(Database::"FRF Row");
        TargetField := TargetRef.Field(2);
        TargetField.SetRange(SourceRef.Field(2));
        S := TargetField.GetFilter() + '|' + Format(TargetRef.Count());
        TargetField := TargetRef.Field(3);
        TargetField.SetFilter('>%1', SourceRef.Field(3));
        S += '|' + TargetField.GetFilter();
        TargetRef.Reset();
        TargetRef.Get(R.RecordId());
        TargetField := TargetRef.Field(3);
        TargetField.Validate(SourceRef.Field(4));
        S += '|' + Format(TargetField.Value);
        TargetField := TargetRef.Field(2);
        TargetField.Validate(SourceRef.Field(3));
        S += '|' + Format(TargetField.Value);
        exit(S);
    end;
}
"#;

/// Y47ValidateFieldRef.al Y47_SetRangeFieldRef (filter `Hello`, count 1, `>42`) and Y47_ValidateFieldRef_Matrix `bool_to_int` and `int_to_text`.
#[test]
fn a_fieldref_as_a_filter_value_and_a_boolean_into_an_integer() {
    assert_eq!(al_testkit::text(al_testkit::run_in(MORE, "D", "F")), "Hello|1|>42|1|42");
}

/// Y47_ValidateFieldRef_Matrix `text_to_int`: the Evaluate error of the Value setter, not the Variant's TypeConversion.
#[test]
fn validate_of_a_text_fieldref_into_an_integer_raises_the_evaluate_error() {
    assert_eq!(
        al_testkit::error_message(al_testkit::run_in(SRC, "C", "TextToInt")),
        "The value \"Hello\" can't be evaluated into type Integer."
    );
}
