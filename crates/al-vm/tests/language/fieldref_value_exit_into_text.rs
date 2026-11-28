//! `exit(FR.Value())` from a procedure that returns a Code[20], a Text, a Text[3], a Text[30] or a Code[3], for a field
//! of another type (issue 9285): the value's Format text, as FieldRef.Validate converts it, upper-cased into a Code,
//! StringLengthExceeded past the length. Assumption `fieldref-value-exit-into-text` (corpus PR 1629, merged at
//! 778b8fc1, recordref-fieldref/fieldref-value-convert/FvcTimestampField.al:443, :466, :485 ExitValue_* and :214
//! Field0_InsertedRow; the follow-up corpus probe, FvcExitValueMore.al:142-280 ExitValue_*). Outside an en-US session a
//! number or an Enum refuses. A Variant variable exited into a Code keeps its TypeConversion error
//! (types/variant/VaTCompare.al:Concat_After_Conversion), and the exit into a number checks the lowered value, so a
//! parenthesized `exit((FR.Value()))` meets the refusal of a Date (issue 9305).

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
    procedure CellsGerman(FieldNo: Integer): Text
    var RecRef: RecordRef; FR: FieldRef;
    begin
        Seed(RecRef);
        FR := RecRef.Field(FieldNo);
        GlobalLanguage(1031);
        exit(ExitCode20(FR));
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

fn german(field: i32) -> Result<al_builtins::value::Value, al_builtins::native::Signal> {
    al_testkit::run_with(SRC, "C", "CellsGerman", vec![al_builtins::value::Value::Integer(field)]).map(|r| r.0)
}

/// Outside an en-US session no cell is measured: a number or an Enum refuses (main raised the Variant error, BC gives a
/// text), the kinds main already answered as BC does keep that answer.
#[test]
fn outside_an_en_us_session_a_number_or_enum_exited_as_a_code_is_refused() {
    for field in [2, 3, 4, 5] {
        let r = german(field);
        assert!(
            matches!(&r, Err(al_builtins::native::Signal::Unsupported(why)) if why.contains("en-US")),
            "field {field}: {r:?}"
        );
    }
    assert_eq!(al_testkit::text(german(10)), "AB-1");
    assert!(
        !matches!(german(8), Err(al_builtins::native::Signal::Unsupported(_))),
        "a Date keeps its answer"
    );
}

const MORE: &str = r#"
enum 50743 MKind { Extensible = false; value(0; Plain) { } value(1; Fancy) { Caption = 'Fancy Caption'; } }
table 50744 MRow
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; IntF; Integer) { }
        field(3; DecF; Decimal) { }
        field(4; DecNegF; Decimal) { }
        field(5; DecBigF; Decimal) { }
        field(6; OptF; Option) { OptionMembers = Alpha,Beta,Gamma; OptionCaption = 'First,Second,Third'; }
        field(7; EnumPlainF; Enum MKind) { }
        field(8; EnumFancyF; Enum MKind) { }
        field(9; BoolF; Boolean) { }
        field(10; DateF; Date) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}
codeunit 50745 M
{
    local procedure ExitCode20(var FR: FieldRef): Code[20] begin exit(FR.Value()); end;
    local procedure ExitText(var FR: FieldRef): Text begin exit(FR.Value()); end;
    local procedure ExitText3(var FR: FieldRef): Text[3] begin exit(FR.Value()); end;
    local procedure ExitText30(var FR: FieldRef): Text[30] begin exit(FR.Value()); end;
    [TryFunction]
    local procedure TryExit(var FR: FieldRef; Target: Text; var Result: Text)
    begin
        case Target of
            'code20': Result := ExitCode20(FR);
            'text': Result := ExitText(FR);
            'text3': Result := ExitText3(FR);
            'text30': Result := ExitText30(FR);
        end;
    end;
    procedure Cells(FieldNo: Integer): Text
    var Row: Record MRow; RecRef: RecordRef; FR: FieldRef; Target: Text; Result: Text; Out: Text;
    begin
        Row.Id := 1; Row.IntF := 5; Row.DecF := 1234.5; Row.DecNegF := -1234.5; Row.DecBigF := 1234567.89;
        Row.OptF := Row.OptF::Beta; Row.EnumPlainF := Row.EnumPlainF::Plain; Row.EnumFancyF := Row.EnumFancyF::Fancy;
        Row.BoolF := true; Row.DateF := 20260115D; Row.Insert();
        RecRef.Open(Database::MRow); RecRef.FindFirst();
        FR := RecRef.Field(FieldNo);
        foreach Target in 'code20,text,text3,text30'.Split(',') do begin
            ClearLastError(); Result := '';
            if TryExit(FR, Target, Result) then
                Out += Target + '=' + Result + ';'
            else
                Out += Target + '=E: ' + GetLastErrorText() + ' | ' + GetLastErrorCode() + ';';
        end;
        exit(Out);
    end;
}
"#;

fn more(field: i32) -> String {
    al_testkit::text(
        al_testkit::run_with(MORE, "M", "Cells", vec![al_builtins::value::Value::Integer(field)]).map(|r| r.0),
    )
}

/// The follow-up corpus probe, recordref-fieldref/fieldref-value-convert/FvcExitValueMore.al ExitValue_*: a Decimal
/// keeps its thousands separator, an Option gives its OptionCaption, an Enum member without a Caption its name, and a
/// bounded Text return takes the text when it fits.
#[test]
fn more_values_and_bounded_text_returns_follow_the_format_text() {
    assert_eq!(more(2), "code20=5;text=5;text3=5;text30=5;");
    assert_eq!(
        more(3),
        format!(
            "code20=1,234.5;text=1,234.5;text3={};text30=1,234.5;",
            too_long(7, 3, "1,234.5")
        )
    );
    assert_eq!(
        more(4),
        format!(
            "code20=-1,234.5;text=-1,234.5;text3={};text30=-1,234.5;",
            too_long(8, 3, "-1,234.5")
        )
    );
    assert_eq!(
        more(5),
        format!(
            "code20=1,234,567.89;text=1,234,567.89;text3={};text30=1,234,567.89;",
            too_long(12, 3, "1,234,567.89")
        )
    );
    assert_eq!(
        more(6),
        format!(
            "code20=SECOND;text=Second;text3={};text30=Second;",
            too_long(6, 3, "Second")
        )
    );
    assert_eq!(
        more(7),
        format!(
            "code20=PLAIN;text=Plain;text3={};text30=Plain;",
            too_long(5, 3, "Plain")
        )
    );
    assert_eq!(
        more(8),
        format!(
            "code20=FANCY CAPTION;text=Fancy Caption;text3={};text30=Fancy Caption;",
            too_long(13, 3, "Fancy Caption")
        )
    );
    assert_eq!(more(9), "code20=YES;text=Yes;text3=Yes;text30=Yes;");
    assert_eq!(
        more(10),
        format!(
            "code20=01/15/26;text=01/15/26;text3={};text30=01/15/26;",
            too_long(8, 3, "01/15/26")
        )
    );
}
