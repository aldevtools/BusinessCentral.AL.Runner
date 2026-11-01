//! `FieldRef.Value()` of an Enum into a Text or Code variable or by-value parameter is its caption; a number passed straight as a
//! Text argument is its Format text; `TextBuilder.Append(FieldRef.Value())` appends the value of every field type as the corpus
//! shows it (text/lt-y49-fieldref-into-text/Y49FieldRefIntoText.al:207-470, corpus master).

const SRC: &str = r#"
enum 50716 YKind { Extensible = true; value(0; " ") { Caption = ' '; } value(1; Alpha) { Caption = 'Alpha Cap'; } value(2; "Beta Two") { Caption = 'Beta Cap'; } }
table 50716 YRow { fields { field(1; Id; Integer) { } field(2; Kind; Enum YKind) { } field(3; Int; Integer) { } field(4; Tm; Time) { }
    field(5; Dt; DateTime) { } field(6; Dur; Duration) { } field(7; Df; DateFormula) { } field(8; G; Guid) { } field(9; Bin; Blob) { }
    field(10; Pic; Media) { } field(11; Pics; MediaSet) { } field(12; Rid; RecordId) { } field(13; Big; BigInteger) { } } keys { key(PK; Id) { } } }
codeunit 50768 C {
    local procedure Seed(var RR: RecordRef)
    var Row: Record YRow; OutS: OutStream;
    begin
        Row.Id := 1; Row.Kind := Row.Kind::"Beta Two"; Row.Int := 12345; Row.Tm := 134506T; Row.Dt := CreateDateTime(20240305D, 134506T);
        Row.Dur := 3600000; Evaluate(Row.Df, '<1M>'); Evaluate(Row.G, '{3F2504E0-4F89-11D3-9A0C-0305E82C3301}'); Row.Big := 9876543210L;
        Row.Bin.CreateOutStream(OutS); OutS.WriteText('abc');
        Row.Insert(); Row.Rid := Row.RecordId; Row.Modify();
        RR.GetTable(Row);
    end;
    local procedure Take250(Value: Text[250]): Text begin exit(Value); end;
    local procedure TakeCode20(Value: Code[20]): Text begin exit(Value); end;
    local procedure TakeCode3(Value: Code[3]): Text begin exit(Value); end;
    procedure EnumIntoParams(): Text
    var RR: RecordRef;
    begin
        Seed(RR);
        exit(Take250(RR.Field(2).Value()) + '|' + TakeCode20(RR.Field(2).Value()));
    end;
    procedure EnumIntoCode3Param(): Text
    var RR: RecordRef;
    begin
        Seed(RR);
        exit(TakeCode3(RR.Field(2).Value()));
    end;
    procedure EnumIntoVariables(): Text
    var RR: RecordRef; T: Text; C: Code[20]; Row: Record YRow;
    begin
        Row.Kind := Row.Kind::" "; RR.GetTable(Row);
        T := RR.Field(2).Value(); C := RR.Field(2).Value();
        exit('[' + T + '][' + C + ']');
    end;
    procedure NumbersIntoParams(): Text
    var RR: RecordRef;
    begin
        Seed(RR);
        exit(Take250(RR.Field(3).Value()) + '|' + Take250(RR.Field(13).Value()));
    end;
    procedure IntIntoCode3Param(): Text
    var RR: RecordRef;
    begin
        Seed(RR);
        exit(TakeCode3(RR.Field(3).Value()));
    end;
    procedure ValidateCaption(): Text
    var RR: RecordRef; FR: FieldRef;
    begin
        RR.Open(Database::YRow); RR.Init(); FR := RR.Field(2);
        FR.Validate('Beta Cap');
    end;
    procedure AppendAll(Filled: Boolean): Text
    var RR: RecordRef; TB: TextBuilder; Row: Record YRow; i: Integer;
    begin
        if Filled then Seed(RR) else RR.GetTable(Row);
        for i := 2 to RR.FieldCount() do begin TB.Append(RR.FieldIndex(i).Value()); TB.Append('|'); end;
        exit(TB.ToText());
    end;
}
"#;

fn run(routine: &str) -> String {
    al_testkit::text(al_testkit::run_in(SRC, "C", routine))
}

#[test]
fn an_enum_from_fieldref_value_into_a_text_or_code_parameter_is_its_caption() {
    assert_eq!(run("EnumIntoParams"), "Beta Cap|BETA CAP");
    assert_eq!(run("EnumIntoVariables"), "[ ][]");
}

#[test]
fn an_enum_caption_longer_than_the_code_parameter_raises_with_the_caption_as_written() {
    let e = al_testkit::error_message(al_testkit::run_in(SRC, "C", "EnumIntoCode3Param"));
    assert_eq!(
        e,
        "The length of the string is 8, but it must be less than or equal to 3 characters. Value: Beta Cap"
    );
}

#[test]
fn a_number_from_fieldref_value_passed_as_a_text_argument_is_its_format_text() {
    assert_eq!(run("NumbersIntoParams"), "12345|9876543210");
    let e = al_testkit::error_message(al_testkit::run_in(SRC, "C", "IntIntoCode3Param"));
    assert_eq!(
        e,
        "The length of the string is 5, but it must be less than or equal to 3 characters. Value: 12345"
    );
}

fn append_all(filled: bool) -> String {
    let r = al_testkit::run_with(SRC, "C", "AppendAll", vec![al_builtins::value::Value::Boolean(filled)]);
    al_testkit::text(r.map(|(v, _)| v))
}

#[test]
fn textbuilder_append_of_every_field_value_kind_is_its_corpus_text() {
    let empty = "{00000000-0000-0000-0000-000000000000}";
    assert_eq!(
        append_all(true),
        format!(
            "Beta Cap|12345| 1:45:06 PM|03/05/24 01:45 PM|1 hour|1M|{{3F2504E0-4F89-11D3-9A0C-0305E82C3301}}|*|{empty}|{empty}|YRow: 1|9876543210|"
        )
    );
    assert_eq!(append_all(false), format!(" |0|||||{empty}| |{empty}|{empty}||0|"));
}

const OPT: &str = r#"
table 50718 ORow { fields { field(1; Id; Integer) { } field(2; Opt; Option) { OptionMembers = " ",Red,Green; OptionCaption = ' ,Red Cap,Green Cap'; } } keys { key(PK; Id) { } } }
codeunit 50772 C {
    local procedure Take250(Value: Text[250]): Text begin exit(Value); end;
    local procedure TakeCode20(Value: Code[20]): Text begin exit(Value); end;
    procedure OptionIntoParams(): Text
    var RR: RecordRef; Row: Record ORow;
    begin
        Row.Opt := Row.Opt::Green; RR.GetTable(Row);
        exit(Take250(RR.Field(2).Value()) + '|' + TakeCode20(RR.Field(2).Value()));
    end;
}
"#;

#[test]
fn an_option_from_fieldref_value_into_a_bounded_text_or_code_parameter_is_its_caption() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(OPT, "C", "OptionIntoParams")),
        "Green Cap|GREEN CAP"
    );
}

#[test]
fn fieldref_validate_of_an_enum_field_with_a_caption_raises_the_option_evaluate_error() {
    let e = al_testkit::error_message(al_testkit::run_in(SRC, "C", "ValidateCaption"));
    assert_eq!(e, "The value \"Beta Cap\" can't be evaluated into type Option.");
}
