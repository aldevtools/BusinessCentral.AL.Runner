//! `exit(FR.Value())` from a procedure that returns a Code[20], a Text or a Code[3], for a field of another type (issue
//! 9285): the value converts as `T := FR.Value()` does, its Format text, upper-cased into a Code, StringLengthExceeded
//! past the length. Assumption `fieldref-value-exit-into-text` (corpus PR 1629, merged at 778b8fc1, recordref-fieldref/
//! fieldref-value-convert/FvcTimestampField.al:443, :466, :485 ExitValue_* and :214 Field0_InsertedRow). A Variant
//! variable exited into a Code keeps its TypeConversion error (types/variant/VaTCompare.al:Concat_After_Conversion),
//! and the exit into a number checks the lowered value, so a parenthesized `exit((FR.Value()))` meets the refusal of a
//! Date (issue 9305).

const SRC: &str = r#"
enum 50740 XKind { Extensible = false; value(0; Zero) { } value(1; One) { Caption = 'One Caption'; } }
table 50741 XRow
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; IntF; Integer) { }
        field(3; BigF; BigInteger) { }
        field(4; DecF; Decimal) { }
        field(5; EnumF; Enum XKind) { }
        field(6; OptF; Option) { OptionMembers = Alpha,Beta,Gamma; }
        field(7; BoolF; Boolean) { }
        field(8; DateF; Date) { }
        field(9; DurF; Duration) { }
        field(10; CodeF; Code[20]) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}
codeunit 50742 C
{
    local procedure ExitCode20(var FR: FieldRef): Code[20] begin exit(FR.Value()); end;
    local procedure ExitText(var FR: FieldRef): Text begin exit(FR.Value()); end;
    local procedure ExitCode3(var FR: FieldRef): Code[3] begin exit(FR.Value()); end;
    local procedure ExitParenCode20(var FR: FieldRef): Code[20] begin exit((FR.Value())); end;
    local procedure ExitParenInteger(var FR: FieldRef): Integer begin exit((FR.Value())); end;
    local procedure ExitVariantCode20(V: Variant): Code[20] begin exit(V); end;
    [TryFunction]
    local procedure TryExit(var FR: FieldRef; Target: Text; var Result: Text)
    begin
        case Target of
            'code20': Result := ExitCode20(FR);
            'text': Result := ExitText(FR);
            'code3': Result := ExitCode3(FR);
            'paren': Result := ExitParenCode20(FR);
        end;
    end;
    local procedure Seed(var RecRef: RecordRef)
    var Row: Record XRow;
    begin
        Row.Id := 1; Row.IntF := 5; Row.BigF := 5000000000L; Row.DecF := 1.5; Row.EnumF := Row.EnumF::One;
        Row.OptF := Row.OptF::Beta; Row.BoolF := true; Row.DateF := 20260115D; Row.DurF := 1500; Row.CodeF := 'AB-1';
        Row.Insert();
        RecRef.Open(Database::XRow); RecRef.FindFirst();
    end;
    procedure Cells(FieldNo: Integer): Text
    var RecRef: RecordRef; FR: FieldRef; Target: Text; Result: Text; Out: Text;
    begin
        Seed(RecRef);
        FR := RecRef.Field(FieldNo);
        foreach Target in 'code20,text,code3'.Split(',') do begin
            ClearLastError(); Result := '';
            if TryExit(FR, Target, Result) then
                Out += Target + '=' + Result + ';'
            else
                Out += Target + '=E: ' + GetLastErrorText() + ' | ' + GetLastErrorCode() + ';';
        end;
        exit(Out);
    end;
    procedure Timestamp(): Text
    var RecRef: RecordRef; FR: FieldRef; Result: Text;
    begin
        Seed(RecRef);
        FR := RecRef.Field(0);
        if not TryExit(FR, 'code20', Result) then exit('E: ' + GetLastErrorText());
        exit(Format(Result = Format(FR.Value())) + '|' + Format(Result <> ''));
    end;
    procedure Paren(): Text
    var RecRef: RecordRef; FR: FieldRef; Result: Text;
    begin
        Seed(RecRef);
        FR := RecRef.Field(2);
        TryExit(FR, 'paren', Result);
        exit(Result);
    end;
    procedure ParenDateIntoInteger(): Integer
    var RecRef: RecordRef; FR: FieldRef;
    begin
        Seed(RecRef);
        FR := RecRef.Field(8);
        exit(ExitParenInteger(FR));
    end;
    procedure VariantVariable(): Text
    var V: Variant;
    begin
        V := 7;
        exit(ExitVariantCode20(V));
    end;
}
"#;

fn cells(field: i32) -> String {
    al_testkit::text(
        al_testkit::run_with(SRC, "C", "Cells", vec![al_builtins::value::Value::Integer(field)]).map(|r| r.0),
    )
}

fn too_long(len: usize, max: usize, value: &str) -> String {
    format!(
        "E: The length of the string is {len}, but it must be less than or equal to {max} characters. Value: {value} | StringLengthExceeded"
    )
}

/// FvcTimestampField.al:443 ExitValue_NumberFields, :466 ExitValue_EnumOptionFields and :485 ExitValue_OtherFields
/// (corpus PR 1629).
#[test]
fn a_fieldref_value_exited_as_text_or_code_is_its_format_text() {
    assert_eq!(cells(2), "code20=5;text=5;code3=5;");
    assert_eq!(
        cells(3),
        format!(
            "code20=5000000000;text=5000000000;code3={};",
            too_long(10, 3, "5000000000")
        )
    );
    assert_eq!(cells(4), "code20=1.5;text=1.5;code3=1.5;");
    assert_eq!(
        cells(5),
        format!(
            "code20=ONE CAPTION;text=One Caption;code3={};",
            too_long(11, 3, "One Caption")
        )
    );
    assert_eq!(
        cells(6),
        format!("code20=BETA;text=Beta;code3={};", too_long(4, 3, "Beta"))
    );
    assert_eq!(cells(7), "code20=YES;text=Yes;code3=YES;");
    assert_eq!(
        cells(8),
        format!("code20=01/15/26;text=01/15/26;code3={};", too_long(8, 3, "01/15/26"))
    );
    assert_eq!(
        cells(9),
        format!(
            "code20={};text=1 second 500 milliseconds;code3={};",
            too_long(25, 20, "1 second 500 milliseconds"),
            too_long(25, 3, "1 second 500 milliseconds")
        )
    );
    assert_eq!(
        cells(10),
        format!("code20=AB-1;text=AB-1;code3={};", too_long(4, 3, "AB-1"))
    );
}

/// FvcTimestampField.al:214 Field0_InsertedRow `code_exit`, `code_equals_format` (corpus PR 1629).
#[test]
fn the_timestamp_exited_as_a_code_is_its_format_text() {
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "Timestamp")), "Yes|Yes");
}

#[test]
fn a_parenthesized_exit_converts_as_the_plain_one() {
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "Paren")), "5");
    let r = al_testkit::run_in(SRC, "C", "ParenDateIntoInteger");
    assert!(
        matches!(&r, Err(al_builtins::native::Signal::Unsupported(why)) if why.contains("issue 9265")),
        "{r:?}"
    );
}

#[test]
fn a_variant_variable_exited_as_a_code_keeps_its_type_conversion_error() {
    assert_eq!(
        al_testkit::error_message(al_testkit::run_in(SRC, "C", "VariantVariable")),
        "The expression Microsoft.Dynamics.Nav.Runtime.NavIndirectValue cannot be type-converted to a Microsoft.Dynamics.Nav.Runtime.NavCode value."
    );
}
