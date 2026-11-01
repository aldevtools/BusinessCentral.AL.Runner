//! Format of a JsonValue that holds a value is its WriteTo text at standard formats 0, 1 and 2; standard format 9 raises
//! InvalidStandardFormat (json/lt-y44-format-jsonvalue/Y44FormatJsonValue.al, corpus PR 962).

fn run(body: &str) -> String {
    let src = format!(
        "codeunit 50600 C {{ procedure F(): Text var V: JsonValue; I: Integer; D: Decimal; T: Text; Ch: Char;
            Op: Option A,B,C,D; Dt: Date; Tm: Time; R: Text; begin {body} end; }}"
    );
    al_testkit::text(al_testkit::run_in(&src, "C", "F"))
}

/// Y44FormatJsonValue.al FormatJsonValue_Numbers, _TextLike, _DateTime.
#[test]
fn format_of_a_json_value_is_its_write_to_text() {
    let got = run("I := 42; V.SetValue(I); R := Format(V);
        D := 12.5; V.SetValue(D); R += '|' + Format(V);
        Ch := 'A'; V.SetValue(Ch); R += '|' + Format(V);
        Op := Op::D; V.SetValue(Op); R += '|' + Format(V);
        T := 'x'; V.SetValue(T); R += '|' + Format(V);
        Dt := 20240315D; V.SetValue(Dt); R += '|' + Format(V);
        Tm := 143015.123T; V.SetValue(Tm); R += '|' + Format(V);
        V.SetValue(false); R += '|' + Format(V);
        exit(R);");
    assert_eq!(got, r#"42|12.5|65|3|"x"|"2024-03-15"|"14:30:15.123"|false"#);
}

/// Y44FormatJsonValue.al FormatJsonValue_StandardFormats_Scalars, _DatesAndNull.
#[test]
fn standard_formats_1_and_2_give_the_same_text_and_9_raises() {
    let got = run("T := 'x'; V.SetValue(T); R := Format(V, 0, 1) + '|' + Format(V, 0, 2);
        V.SetValueToNull(); R += '|' + Format(V, 0, 1) + '|' + Format(V, 0, 2);
        I := 42; V.SetValue(I); asserterror Format(V, 0, 9); R += '|' + GetLastErrorText() + '|' + GetLastErrorCode();
        V.SetValueToNull(); asserterror Format(V, 0, 9); R += '|' + GetLastErrorCode();
        exit(R);");
    assert_eq!(
        got,
        r#""x"|"x"|null|null|Standard format number 9 does not exist for the type 'JsonValue'.|InvalidStandardFormat|InvalidStandardFormat"#
    );
}

/// A standard format no corpus case pins (3 to 8) stays refused.
#[test]
fn an_unpinned_standard_format_of_a_json_value_is_refused() {
    let src = "codeunit 50600 C { procedure F(): Text var V: JsonValue; I: Integer; begin
        I := 42; V.SetValue(I); exit(Format(V, 0, 4)); end; }";
    let r = al_testkit::run_in(src, "C", "F");
    assert!(format!("{r:?}").contains("not pinned"), "{r:?}");
}
