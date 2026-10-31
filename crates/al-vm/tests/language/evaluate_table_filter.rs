//! `Evaluate` into a TableFilter field (al-runner issues 5998 and 6006), pinned by records/W4PlatformAndMeta.al (writer SI:
//! TableFilterEvaluate_AcceptedForms, _RejectedForms, _ErrorsAsStatement, _StoredValue; writer SR: TableFilterEvaluate_SeveralCodeFields,
//! _SrMultiFieldForms, _SrSecondFieldTypes, _SrExpressionForms, _SrRejectedForms, _SrStoredSpaceValue). The stored text names the
//! table by its name and each field by its caption, an Option value by its caption, a Code value in capitals, the conditions in
//! the order of their field numbers.

use al_builtins::native::Signal;

const TABLES: &str = "table 89500 \"SI TF Target\" { Caption = 'SI Target Cap';
        fields { field(1; Code; Code[20]) { Caption = 'Code Cap'; }
        field(2; Kind; Option) { Caption = 'Kind Cap'; OptionMembers = DIAM,LGDIAM; OptionCaption = 'Diam Cap,LgDiam Cap'; }
        field(3; Qty; Integer) { Caption = 'Qty Cap'; } }
        keys { key(PK; Code) { Clustered = true; } } }
    table 89610 \"SR TF Pair\" { Caption = 'SR Pair Cap';
        fields { field(1; \"Location Code\"; Code[10]) { Caption = 'SR Loc Cap'; } field(2; Code; Code[20]) { Caption = 'SR Code Cap'; }
        field(3; Qty; Integer) { Caption = 'SR Qty Cap'; }
        field(4; Kind; Option) { Caption = 'SR Kind Cap'; OptionMembers = MAIN,SIDE; OptionCaption = 'Main Cap,Side Cap'; }
        field(5; Descr; Text[30]) { Caption = 'SR Descr Cap'; }
        field(6; D; Date) { } }
        keys { key(PK; \"Location Code\", Code) { Clustered = true; } } }
    table 50111 \"TF Row\" { fields { field(1; Id; Integer) { } field(2; Flt; TableFilter) { } }
        keys { key(PK; Id) { Clustered = true; } } }";

/// `Evaluate` as a function: `<answer>|[<Format of the field>]|<last error>|<last error code>`.
fn function_form(text: &str) -> Result<String, Signal> {
    let s = format!(
        "{TABLES} codeunit 50101 C {{ procedure F(): Text var R: Record \"TF Row\"; Ok: Boolean; begin ClearLastError();
        Ok := Evaluate(R.Flt, '{}');
        exit(Format(Ok) + '|[' + Format(R.Flt) + ']|' + GetLastErrorText() + '|' + GetLastErrorCode()); end; }}",
        text.replace('\'', "''")
    );
    al_testkit::run_in(&s, "C", "F").map(|v| al_testkit::text(Ok(v)))
}

/// `Evaluate` as a statement: `ok:[<Format>]` or `error:<text>|<code>`.
fn statement_form(text: &str) -> Result<String, Signal> {
    let s = format!(
        "{TABLES} codeunit 50101 C {{
        [TryFunction] local procedure T(var R: Record \"TF Row\") begin Evaluate(R.Flt, '{}'); end;
        procedure F(): Text var R: Record \"TF Row\"; begin ClearLastError();
        if T(R) then exit('ok:[' + Format(R.Flt) + ']');
        exit('error:' + GetLastErrorText() + '|' + GetLastErrorCode()); end; }}",
        text.replace('\'', "''")
    );
    al_testkit::run_in(&s, "C", "F").map(|v| al_testkit::text(Ok(v)))
}

fn yes(stored: &str) -> String {
    format!("Yes|[{stored}]||")
}

#[test]
fn accepted_forms_store_the_table_name_field_captions_and_option_captions() {
    // W4PlatformAndMeta.al TableFilterEvaluate_AcceptedForms.
    for (text, stored) in [
        ("SI TF Target: Kind=LGDIAM", "SI TF Target: Kind Cap=LgDiam Cap"),
        ("SI TF Target: Kind Cap=LGDIAM", "SI TF Target: Kind Cap=LgDiam Cap"),
        ("SI TF Target: Kind=LgDiam Cap", "SI TF Target: Kind Cap=LgDiam Cap"),
        ("SI TF Target: Kind=1", "SI TF Target: Kind Cap=LgDiam Cap"),
        (
            "SI TF Target: Code=A..B,Kind=DIAM",
            "SI TF Target: Code Cap=A..B,Kind Cap=Diam Cap",
        ),
        (
            "SI TF Target: Code=A..B, Kind=DIAM",
            "SI TF Target: Code Cap=A..B,Kind Cap=Diam Cap",
        ),
        ("89500: Kind=LGDIAM", "SI TF Target: Kind Cap=LgDiam Cap"),
        ("\"SI TF Target\": Kind=LGDIAM", "SI TF Target: Kind Cap=LgDiam Cap"),
        ("si tf target: kind=lgdiam", "SI TF Target: Kind Cap=LgDiam Cap"),
        ("SI TF Target:Kind=LGDIAM", "SI TF Target: Kind Cap=LgDiam Cap"),
        ("SI TF Target: Qty=1..5|7", "SI TF Target: Qty Cap=1..5|7"),
        ("SI TF Target: Qty=<>0", "SI TF Target: Qty Cap=<>0"),
        ("SI TF Target: Code=@a*", "SI TF Target: Code Cap=@A*"),
        ("SI TF Target: ", "SI TF Target: "),
        ("", ""),
    ] {
        assert_eq!(function_form(text).unwrap(), yes(stored), "{text}");
    }
}

#[test]
fn several_fields_spaces_quotes_and_expressions() {
    // W4PlatformAndMeta.al TableFilterEvaluate_SrMultiFieldForms, _SrSecondFieldTypes, _SrExpressionForms, _SrRejectedForms.
    for (text, stored) in [
        (
            "SR TF Pair: Location Code=MAIN,Code=BIN01",
            "SR TF Pair: SR Loc Cap=MAIN,SR Code Cap=BIN01",
        ),
        (
            "SR TF Pair: Code=BIN WITH SPACE,Location Code=MAIN",
            "SR TF Pair: SR Loc Cap=MAIN,SR Code Cap=BIN WITH SPACE",
        ),
        (
            "SR TF Pair: Code=BIN01 ,Location Code=MAIN",
            "SR TF Pair: SR Loc Cap=MAIN,SR Code Cap=BIN01",
        ),
        ("SR TF Pair: Code= BIN01", "SR TF Pair: SR Code Cap=BIN01"),
        (
            "SR TF Pair: Code=bin with space",
            "SR TF Pair: SR Code Cap=BIN WITH SPACE",
        ),
        (
            "SR TF Pair: Descr=Some text here",
            "SR TF Pair: SR Descr Cap=Some text here",
        ),
        ("SR TF Pair: SR Loc Cap=MAIN", "SR TF Pair: SR Loc Cap=MAIN"),
        ("SR TF Pair: \"Location Code\"=MAIN", "SR TF Pair: SR Loc Cap=MAIN"),
        ("SR TF Pair: Code=", "SR TF Pair: SR Code Cap="),
        (
            "SR TF Pair: Code=BIN01,Qty=1..5",
            "SR TF Pair: SR Code Cap=BIN01,SR Qty Cap=1..5",
        ),
        (
            "SR TF Pair: Code=BIN01,Kind=1",
            "SR TF Pair: SR Code Cap=BIN01,SR Kind Cap=Side Cap",
        ),
        (
            "SR TF Pair: Code=BIN ONE|BIN TWO",
            "SR TF Pair: SR Code Cap=BIN ONE|BIN TWO",
        ),
        ("SR TF Pair: Code=BIN A..BIN B", "SR TF Pair: SR Code Cap=BIN A..BIN B"),
        ("SR TF Pair: Code=<>BIN01", "SR TF Pair: SR Code Cap=<>BIN01"),
        (
            "SR TF Pair: Code='BIN WITH SPACE'",
            "SR TF Pair: SR Code Cap=BIN WITH SPACE",
        ),
        ("SR TF Pair: Code='A|B'", "SR TF Pair: SR Code Cap='A|B'"),
        ("SR TF Pair: Code=\"BIN01\"", "SR TF Pair: SR Code Cap=BIN01"),
        ("SR TF Pair: Code=A,Code=B", "SR TF Pair: SR Code Cap=A,SR Code Cap=B"),
        (
            "SR TF Pair: Code=A,SR Code Cap=B",
            "SR TF Pair: SR Code Cap=A,SR Code Cap=B",
        ),
        ("SR TF Pair: Code=BIN01,", "SR TF Pair: SR Code Cap=BIN01"),
    ] {
        assert_eq!(function_form(text).unwrap(), yes(stored), "{text}");
    }
}

#[test]
fn rejected_forms_answer_false_with_the_pinned_error() {
    let cant = |t: &str, more: &str| format!("The value \"{t}\" can't be evaluated into type TableFilter.{more}");
    for (text, error, code) in [
        (
            "SI TF Target: Kind=FOO",
            cant(
                "SI TF Target: Kind=FOO",
                " 'FOO' is not an option. The existing options are: Diam Cap,LgDiam Cap",
            ),
            "Evaluate:InvalidFilterExpression",
        ),
        (
            "SI TF Target: Nope=1",
            cant(
                "SI TF Target: Nope=1",
                " The supplied field number 'Nope' cannot be found in the 'SI Target Cap' table.",
            ),
            "Evaluate:FieldNotFound",
        ),
        (
            "No Such Table: Kind=1",
            cant("No Such Table: Kind=1", ""),
            "Evaluate:MetadataNotFound",
        ),
        ("SI TF Target", cant("SI TF Target", ""), "Evaluate"),
        ("xyz", cant("xyz", ""), "Evaluate"),
        ("Kind=LGDIAM", cant("Kind=LGDIAM", ""), "Evaluate"),
        (
            "SI TF Target: Kind=FILTER(LGDIAM)",
            cant(
                "SI TF Target: Kind=FILTER(LGDIAM)",
                " An error occurred while interpreting the filter: Did not expect '('.",
            ),
            "Evaluate:InvalidFilterExpression",
        ),
        (
            "SI TF Target: Qty=abc",
            cant(
                "SI TF Target: Qty=abc",
                " The value \"abc\" can't be evaluated into type Integer.",
            ),
            "Evaluate:InvalidFilterExpression",
        ),
        (
            "SR TF Pair: Location Code=ABCDEFGHIJKL",
            cant(
                "SR TF Pair: Location Code=ABCDEFGHIJKL",
                " The length of the string is 12, but it must be less than or equal to 10 characters. Value: ABCDEFGHIJKL",
            ),
            "Evaluate:InvalidFilterExpression",
        ),
        (
            "SR TF Pair: Code='A,B'",
            cant("SR TF Pair: Code='A,B'", " An single quote (') is missing."),
            "Evaluate:InvalidFilterExpression",
        ),
    ] {
        assert_eq!(function_form(text).unwrap(), format!("No|[]|{error}|{code}"), "{text}");
        assert_eq!(statement_form(text).unwrap(), format!("error:{error}|{code}"), "{text}");
    }
    // A condition without '=' answers false and leaves no error (TableFilterEvaluate_SrRejectedForms `missing_equals_second`).
    assert_eq!(function_form("SR TF Pair: Code=BIN01,Qty").unwrap(), "No|[]||");
}

#[test]
fn format_number_9_and_a_failed_evaluate_keep_the_value() {
    let s = format!(
        "{TABLES} codeunit 50101 C {{ procedure F(): Text var R: Record \"TF Row\"; Ok: Boolean; A: Text; begin
        Ok := Evaluate(R.Flt, 'SR TF Pair: Location Code=MAIN,Code=BIN WITH SPACE', 9);
        A := Format(Ok) + '|' + Format(R.Flt, 0, 9);
        Ok := Evaluate(R.Flt, 'xyz');
        exit(A + '|' + Format(Ok) + '|' + Format(R.Flt)); end; }}"
    );
    assert_eq!(
        al_testkit::text(al_testkit::run_in(&s, "C", "F")),
        "Yes|SR TF Pair: SR Loc Cap=MAIN,SR Code Cap=BIN WITH SPACE|No|SR TF Pair: SR Loc Cap=MAIN,SR Code Cap=BIN WITH SPACE"
    );
}

#[test]
fn forms_no_test_pins_are_refused() {
    for text in [
        // other field types, filter forms of a type the corpus does not pin, other format text shapes
        "SR TF Pair: D=010125D",
        "SR TF Pair: Qty=>5",
        "SR TF Pair: Kind=MAIN|SIDE",
        "SR TF Pair: Descr=A*",
        "SI TF Target:",
        "SR TF Pair: Code=A,,Qty=1",
        "SR TF Pair: Location Code=abcdefghijkl",
        // a first condition without '=': only a second one is pinned (`missing_equals_second`)
        "SR TF Pair: Qty",
        "SR TF Pair: Qty,Code=BIN01",
    ] {
        match function_form(text) {
            Err(Signal::Unsupported(why)) => assert!(why.contains("TableFilter"), "{text}: {why}"),
            other => panic!("{text}: expected a refusal, got {other:?}"),
        }
    }
    // A condition without '=' as a statement: the error is not pinned.
    assert!(matches!(
        statement_form("SR TF Pair: Code=BIN01,Qty"),
        Err(Signal::Unsupported(_))
    ));
}

#[test]
fn a_text_assigned_through_a_fieldref_raises_and_keeps_the_value() {
    // W4PlatformAndMeta.al TableFilterField_SetFromText (BC28PLUS): every text raises the Evaluate error, code Evaluate:Argument.
    let s = format!(
        "{TABLES} codeunit 50101 C {{
        [TryFunction] local procedure T(var RR: RecordRef; V: Text) begin RR.Field(2).Value := V; end;
        procedure F(): Text var RR: RecordRef; begin RR.Open(Database::\"TF Row\");
        if T(RR, 'Id: 1') then exit('ok');
        exit(GetLastErrorText() + '|' + GetLastErrorCode() + '|[' + Format(RR.Field(2).Value()) + ']'); end; }}"
    );
    assert_eq!(
        al_testkit::text(al_testkit::run_in(&s, "C", "F")),
        "The value \"Id: 1\" can't be evaluated into type TableFilter.|Evaluate:Argument|[]"
    );
}

/// A field given by its number: `89500: 2=1` is the Option field 2 with its ordinal 1, stored by the field's caption
/// (types/TestTableFilterEvaluate.al:26-36 TableFilter_Evaluate_NumericIds_NormalizesValue, `67021: 2=1` stores
/// `ALT TF Plain Target: Report Type=LGDIAM`). A number no field has is not pinned and is refused.
#[test]
fn a_field_given_by_its_number() {
    assert_eq!(
        function_form("89500: 2=1").unwrap(),
        yes("SI TF Target: Kind Cap=LgDiam Cap")
    );
    assert!(matches!(function_form("89500: 9=1"), Err(Signal::Unsupported(_))));
}
