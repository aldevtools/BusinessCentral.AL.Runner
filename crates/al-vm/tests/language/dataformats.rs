//! End to end: JSON, XML, Evaluate and in-memory streams. Each test names the
//! corpus test that pins the behaviour it asserts.

use al_builtins::native::Signal;
use al_builtins::value::Value;

fn run(src: &str, routine: &str) -> Result<Value, Signal> {
    al_testkit::run(src, routine)
}

fn cu(body: &str) -> String {
    format!("codeunit 50100 C {{ {body} }}")
}

/// Runs `procedure F(): Text` with the given locals and body.
fn text_of(vars: &str, body: &str) -> String {
    let src = cu(&format!("procedure F(): Text var {vars} begin {body} end;"));
    match run(&src, "F") {
        Ok(Value::Text(s)) | Ok(Value::Code(s)) => s.to_string(),
        other => panic!("{other:?}"),
    }
}

/// The message and code of the error the body raises.
fn raised(vars: &str, body: &str) -> (String, String) {
    let src = cu(&format!("procedure F(): Text var {vars} begin {body} end;"));
    match run(&src, "F") {
        Err(Signal::Error(e)) => (e.message, e.code),
        other => panic!("{other:?}"),
    }
}

fn unsupported(vars: &str, body: &str) -> bool {
    let src = cu(&format!("procedure F(): Text var {vars} begin {body} end;"));
    matches!(run(&src, "F"), Err(Signal::Unsupported(_)))
}

// ---------------------------------------------------------------- JSON

#[test]
fn json_object_add_get_as_value() {
    // json/TestJsonObject.al: JsonObject_Add_Get_TextValue / IntegerValue / BooleanValue.
    let t = text_of(
        "J: JsonObject; T: JsonToken;",
        "J.Add('key', 'hello'); J.Add('num', 42); J.Add('flag', true);
         J.Get('key', T); exit(T.AsValue().AsText());",
    );
    assert_eq!(t, "hello");
    let t = text_of(
        "J: JsonObject; T: JsonToken;",
        "J.Add('num', 42); J.Get('num', T); exit(Format(T.AsValue().AsInteger()));",
    );
    assert_eq!(t, "42");
    let t = text_of(
        "J: JsonObject; T: JsonToken;",
        "J.Add('flag', true); J.Get('flag', T); exit(Format(T.AsValue().AsBoolean()));",
    );
    assert_eq!(t, "Yes");
}

#[test]
fn json_object_contains_get_remove() {
    // TestJsonObject.al: Contains_ExistingKey / MissingKey, Get_MissingKey_ReturnsFalse, Remove_DeletesKey.
    let t = text_of(
        "J: JsonObject; T: JsonToken; Ok: Boolean;",
        "J.Add('x', 1); Ok := J.Get('nothere', T);
         exit(Format(J.Contains('x')) + Format(J.Contains('missing')) + Format(Ok));",
    );
    assert_eq!(t, "YesNoNo");
    let t = text_of(
        "J: JsonObject;",
        "J.Add('a', 1); J.Remove('a'); exit(Format(J.Contains('a')));",
    );
    assert_eq!(t, "No");
}

#[test]
fn json_object_keys_are_case_sensitive() {
    // json/TestJsonXmlDeepContracts.al: JsonObject_Keys_AreCaseSensitive.
    let t = text_of(
        "J: JsonObject; T: JsonToken;",
        "J.Add('key', 'lower'); J.Add('KEY', 'upper');
         J.Get('key', T); exit(T.AsValue().AsText() + '|' + Format(J.Contains('KEY')));",
    );
    assert_eq!(t, "lower|Yes");
}

#[test]
fn json_write_to_and_read_from() {
    // TestJsonXmlDeepContracts.al: empty object/array serialise to {} and [].
    assert_eq!(text_of("J: JsonObject; S: Text;", "J.WriteTo(S); exit(S);"), "{}");
    assert_eq!(text_of("J: JsonArray; S: Text;", "J.WriteTo(S); exit(S);"), "[]");
    // TestJsonObject.al: WriteTo contains "k"; ReadFrom parses.
    let t = text_of("J: JsonObject; S: Text;", "J.Add('k', 'v'); J.WriteTo(S); exit(S);");
    assert!(t.contains("\"k\""), "{t}");
    let t = text_of(
        "J: JsonObject; T: JsonToken;",
        "J.ReadFrom('{\"name\":\"Bob\",\"age\":25}'); J.Get('name', T); exit(T.AsValue().AsText());",
    );
    assert_eq!(t, "Bob");
}

#[test]
fn json_array_basics() {
    // json/TestJsonArray.al: Count, Get by zero-based index, ReadFrom, nested object.
    let t = text_of(
        "A: JsonArray; T: JsonToken;",
        "A.Add(10); A.Add(20); A.Get(1, T); exit(Format(A.Count()) + Format(T.AsValue().AsInteger()));",
    );
    assert_eq!(t, "220");
    let t = text_of(
        "A: JsonArray; T: JsonToken;",
        "A.ReadFrom('[\"a\",\"b\"]'); A.Get(0, T); exit(T.AsValue().AsText() + Format(A.Count()));",
    );
    assert_eq!(t, "a2");
    let t = text_of(
        "A: JsonArray; I: JsonObject;",
        "I.Add('id', 1); A.Add(I); exit(Format(A.Count()));",
    );
    assert_eq!(t, "1");
}

#[test]
fn json_tokens_share_and_classify() {
    // json/TestJsonToken.al + TestJsonTypedGetters.al: AsToken/AsObject/AsArray, Is*, nested Get.
    let t = text_of(
        "J: JsonObject; T: JsonToken; O: JsonObject; X: JsonToken;",
        "J.ReadFrom('{\"a\":1}'); T := J.AsToken(); O := T.AsObject(); O.Get('a', X);
         exit(Format(T.IsObject()) + Format(T.IsArray()) + Format(T.IsValue()) + Format(X.IsValue()) + Format(X.AsValue().AsInteger()));",
    );
    assert_eq!(t, "YesNoNoYes1");
    // JsonObject is a reference type: AsObject shares, so a write through O is visible in J.
    let t = text_of(
        "J: JsonObject; T: JsonToken; O: JsonObject;",
        "T := J.AsToken(); O := T.AsObject(); O.Add('late', 1); exit(Format(J.Contains('late')));",
    );
    assert_eq!(t, "Yes");
    let t = text_of(
        "J: JsonObject; A: JsonArray; T: JsonToken;",
        "A.Add(1); J.Add('items', A); J.Get('items', T); exit(Format(T.IsArray()));",
    );
    assert_eq!(t, "Yes");
}

#[test]
fn json_typed_getters() {
    // json/TestJsonTypedGetters.al and TestJsonComplete.al.
    let t = text_of(
        "J: JsonObject;",
        "J.ReadFrom('{\"t\":true,\"f\":false,\"n\":42,\"d\":2.5,\"s\":\"hello\",\"b\":1000000,\"o\":2}');
         exit(Format(J.GetBoolean('t')) + Format(J.GetBoolean('f')) + Format(J.GetInteger('n')) + Format(J.GetDecimal('d') > 2.0)
              + J.GetText('s') + Format(J.GetBigInteger('b')) + Format(J.GetOption('o')));",
    );
    assert_eq!(t, "YesNo42Yeshello10000002");
    let t = text_of(
        "J: JsonObject; Inner: JsonArray; O: JsonObject;",
        "J.ReadFrom('{\"arr\":[1,2,3],\"obj\":{\"x\":1}}'); Inner := J.GetArray('arr'); O := J.GetObject('obj');
         exit(Format(Inner.Count()) + Format(O.GetInteger('x')));",
    );
    assert_eq!(t, "31");
    let t = text_of(
        "A: JsonArray;",
        "A.ReadFrom('[true,false,20,1.5,\"apple\",999999]');
         exit(Format(A.GetBoolean(0)) + Format(A.GetBoolean(1)) + Format(A.GetInteger(2)) + Format(A.GetDecimal(3) > 1.0) + A.GetText(4) + Format(A.GetBigInteger(5)));",
    );
    assert_eq!(t, "YesNo20Yesapple999999");
}

#[test]
fn json_missing_key_throws_with_the_pinned_text_and_code() {
    // json/TestJsonRuntimeContracts.al:JsonObject_Errors_TextAndCode
    let t = text_of(
        "J: JsonObject; R: Integer;",
        "J.ReadFrom('{\"other\":5}'); asserterror R := J.GetInteger('missing'); exit(GetLastErrorText() + '|' + GetLastErrorCode());",
    );
    assert_eq!(
        t,
        "There is no property with the 'missing' key on the JSON object.|JsonPropertyNotFound"
    );
    // The DefaultIfNotFound = true form is not pinned and stays refused.
    assert!(unsupported(
        "J: JsonObject; R: Integer;",
        "J.ReadFrom('{\"other\":5}'); R := J.GetInteger('missing', true); exit('');"
    ));
}

#[test]
fn json_object_get_missing_key_as_statement_throws_property_not_found() {
    // misc/lt-y14-triage/Y14Small.al:389 (corpus master): JsonObject.Get of a missing name whose
    // result is not used raises JsonPropertyNotFound; the token keeps its value.
    let t = text_of(
        "J: JsonObject; T: JsonToken;",
        "J.Add('present', 1); asserterror J.Get('missing', T);
         exit(GetLastErrorText() + '|' + GetLastErrorCode() + '|' + Format(J.Get('missing', T)));",
    );
    assert_eq!(
        t,
        "There is no property with the 'missing' key on the JSON object.|JsonPropertyNotFound|No"
    );
}

#[test]
fn json_value_set_and_read() {
    // json/TestJsonValue.al and TestJsonComplete.al.
    let t = text_of("V: JsonValue;", "V.SetValue('hello'); exit(V.AsText());");
    assert_eq!(t, "hello");
    let t = text_of(
        "V: JsonValue;",
        "V.SetValue(42); exit(Format(V.AsInteger()) + Format(V.AsBigInteger()) + Format(V.AsOption()) + Format(V.AsDuration() > 0));",
    );
    assert_eq!(t, "424242Yes");
    let t = text_of(
        "V: JsonValue;",
        "V.SetValueToNull(); exit(Format(V.IsNull()) + Format(V.IsUndefined()));",
    );
    assert_eq!(t, "YesNo");
    let t = text_of(
        "V: JsonValue;",
        "V.SetValueToUndefined(); exit(Format(V.IsUndefined()));",
    );
    assert_eq!(t, "Yes");
    let t = text_of(
        "V: JsonValue;",
        "V.SetValue(1); exit(Format(V.IsNull()) + Format(V.IsUndefined()));",
    );
    assert_eq!(t, "NoNo");
    let t = text_of(
        "V: JsonValue; C: Char;",
        "V.SetValue('A'); C := V.AsChar(); exit(Format(C));",
    );
    assert_eq!(t, "A");
    let t = text_of(
        "V: JsonValue; C: Code[100];",
        "V.SetValue('ITEM001'); C := V.AsCode(); exit(C);",
    );
    assert_eq!(t, "ITEM001");
}

#[test]
fn json_null_is_distinct_from_a_missing_key_and_round_trips() {
    // TestJsonXmlDeepContracts.al JsonObject_NullValue_vs_MissingKey_Distinct;
    // TestRoundtripContracts.al JsonObject_WriteTo_ReadFrom_PreservesNull.
    let t = text_of(
        "J: JsonObject; J2: JsonObject; V: JsonValue; T: JsonToken; S: Text;",
        "V.SetValueToNull(); J.Add('nullkey', V); J.WriteTo(S); J2.ReadFrom(S); J2.Get('nullkey', T);
         exit(Format(J.Contains('nullkey')) + Format(J.Contains('missing')) + Format(T.AsValue().IsNull()));",
    );
    assert_eq!(t, "YesNoYes");
}

#[test]
fn json_round_trips_text_numbers_booleans_and_structure() {
    // TestRoundtripContracts.al and TestJsonXmlDeepContracts.al.
    let t = text_of(
        "J: JsonObject; J2: JsonObject; T: JsonToken; S: Text;",
        "J.Add('text', 'Ünïcödé'); J.Add('big', 2000000000); J.Add('active', true); J.Add('deleted', false);
         J.WriteTo(S); J2.ReadFrom(S);
         exit(J2.GetText('text') + Format(J2.GetInteger('big')) + Format(J2.GetBoolean('active')) + Format(J2.GetBoolean('deleted')));",
    );
    assert_eq!(t, "Ünïcödé2000000000YesNo");
    // Escaped characters survive a round trip (JsonObject_EscapeChars_RoundTrip uses a newline,
    // which needs Format(10, 0, '<Integer>')[1]: not lowered yet).
    let t = text_of(
        "J: JsonObject; J2: JsonObject; S: Text; Orig: Text;",
        "Orig := 'quote\"s and \\ and ' + 'tab'; J.Add('content', Orig); J.WriteTo(S); J2.ReadFrom(S);
         exit(Format(J2.GetText('content') = Orig));",
    );
    assert_eq!(t, "Yes");
    let t = text_of(
        "A: JsonArray; A2: JsonArray; I1: JsonObject; I2: JsonObject; T: JsonToken; S: Text;",
        "I1.Add('id', 1); A.Add(I1); I2.Add('id', 2); A.Add(I2); A.WriteTo(S); A2.ReadFrom(S);
         A2.Get(1, T); exit(Format(A2.Count()) + Format(T.AsObject().GetInteger('id')));",
    );
    assert_eq!(t, "22");
}

#[test]
fn json_select_token_by_path() {
    // TestJsonComplete.al JsonToken_SelectToken_ViaPath; TestJsonTypedGetters.al
    // JsonToken_SelectToken_FindsNestedValue / FindsArrayElement; TestJsonXmlDeepContracts.al ReturnsCorrectType.
    let t = text_of(
        "J: JsonObject; T: JsonToken; S: JsonToken;",
        "J.ReadFrom('{\"a\":{\"b\":{\"c\":42}},\"items\":[100,200],\"arr\":[1],\"obj\":{\"k\":\"v\"}}'); T := J.AsToken();
         T.SelectToken('a.b.c', S); exit(Format(S.AsValue().AsInteger()));",
    );
    assert_eq!(t, "42");
    let t = text_of(
        "J: JsonObject; T: JsonToken; S: JsonToken;",
        "J.ReadFrom('{\"items\":[100,200]}'); T := J.AsToken(); T.SelectToken('items[0]', S); exit(Format(S.AsValue().AsInteger()));",
    );
    assert_eq!(t, "100");
    let t = text_of(
        "J: JsonObject; T: JsonToken; S: JsonToken;",
        "J.ReadFrom('{\"arr\":[1],\"obj\":{\"k\":\"v\"}}'); T := J.AsToken();
         T.SelectToken('arr', S); exit(Format(S.IsArray()) + Format(S.IsObject()));",
    );
    assert_eq!(t, "YesNo");
}

#[test]
fn json_token_path_and_clone() {
    // TestJsonTypedGetters.al JsonToken_Path_ReturnsPath: the path contains "nested".
    let t = text_of(
        "J: JsonObject; I: JsonObject; T: JsonToken;",
        "I.Add('id', 1); J.Add('nested', I); J.Get('nested', T); exit(T.Path());",
    );
    assert_eq!(t, "nested");
    // TestJsonToken.al JsonToken_Clone_CreatesIndependentCopy.
    let t = text_of(
        "J: JsonObject; T: JsonToken; C: JsonToken;",
        "J.Add('v', 42); J.Get('v', T); C := T.Clone(); exit(Format(C.AsValue().AsInteger()));",
    );
    assert_eq!(t, "42");
}

#[test]
fn json_dates_and_durations() {
    // TestJsonComplete.al: GetDate/GetDateTime/GetTime parse ISO text; GetDuration reads a number;
    // JsonArray.Add(Date) then GetDate returns the date.
    let t = text_of(
        "J: JsonObject; D: Date; DT: DateTime; T: Time; Dur: Duration;",
        "J.ReadFrom('{\"d\":\"2024-01-15\",\"dt\":\"2024-01-15T12:00:00\",\"t\":\"12:00:00\",\"dur\":3600000}');
         D := J.GetDate('d'); DT := J.GetDateTime('dt'); T := J.GetTime('t'); Dur := J.GetDuration('dur');
         exit(Format(D <> 0D) + Format(DT <> 0DT) + Format(T <> 0T) + Format(Dur > 0));",
    );
    assert_eq!(t, "YesYesYesYes");
    let t = text_of(
        "A: JsonArray; D: Date;",
        "A.Add(20240315D); D := A.GetDate(0); exit(Format(D = 20240315D));",
    );
    assert_eq!(t, "Yes");
}

#[test]
fn json_yaml_reads_and_writes_the_flat_form() {
    // json/TestJsonRuntimeContracts.al:JsonObject_Yaml_RoundTripsOnThisTier; a nested value is refused.
    let t = text_of(
        "J: JsonObject; Y: Text;",
        "J.ReadFromYaml('key: value'); J.WriteTo(Y); exit(Y);",
    );
    assert_eq!(t, "{\"key\":\"value\"}");
    assert!(unsupported(
        "J: JsonObject;",
        "J.ReadFromYaml('key:' + Format(10) + '  - a'); exit('');"
    ));
}

// ------------------------------------------------------------ Evaluate

#[test]
fn evaluate_numbers() {
    // text/TestEvaluate.al: Evaluate(I, '42') = 42; Evaluate(D, '3.14') > 3.0;
    // handlers/TestPageFieldFormats_Tests.al:753 Evaluate(BigInteger, '9000000000').
    let t = text_of(
        "I: Integer; D: Decimal; B: BigInteger;",
        "Evaluate(I, '42'); Evaluate(D, '3.14'); Evaluate(B, '9000000000'); exit(Format(I) + '|' + Format(D = 3.14) + '|' + Format(B));",
    );
    assert_eq!(t, "42|Yes|9000000000");
    let t = text_of("I: Integer;", "Evaluate(I, '-7'); exit(Format(I));");
    assert_eq!(t, "-7");
}

// errors/lt-ms-refusals-128/P128EvalBasic.al:P128_Evaluate_IntoText, P128_Evaluate_IntoCode (`lower`, `padded`,
// `one too long`), P128_Evaluate_IntoCode_ResultUnused and P128_Evaluate_IntoShortText_ResultUnused (`too long`).
#[test]
fn evaluate_into_text_and_code() {
    let t = text_of(
        "T: Text; C: Code[5]; O: Boolean;",
        "T := 'KEEP'; C := 'KEEP'; O := Evaluate(T, ' hello '); Evaluate(C, ' ab '); exit(Format(O) + '[' + T + '][' + C + ']');",
    );
    assert_eq!(t, "Yes[ hello ][AB]");
    let t = text_of(
        "C: Code[5]; O: Boolean;",
        "C := 'KEEP'; O := Evaluate(C, 'abcdef'); exit(Format(O) + '[' + C + ']');",
    );
    assert_eq!(t, "No[KEEP]");
    let (m, code) = raised("S: Text[5];", "S := 'KEEP'; Evaluate(S, 'abcdef'); exit(S);");
    assert_eq!(
        m,
        "The length of the string is 6, but it must be less than or equal to 5 characters. Value: abcdef"
    );
    assert_eq!(code, "StringLengthExceeded");
}

#[test]
fn evaluate_reports_failure_through_its_result() {
    // text/TestEvaluate.al: Evaluate_InvalidInteger_ReturnsFalse and Evaluate_EmptyString_ReturnsFalse;
    // json/TestRoundtripContracts.al Evaluate_InvalidText_ReturnsFalse (the variable is not asserted).
    let t = text_of(
        "I: Integer; R: Boolean; E: Boolean;",
        "R := Evaluate(I, 'notanumber'); E := Evaluate(I, ''); exit(Format(R) + Format(E));",
    );
    assert_eq!(t, "NoNo");
    let t = text_of(
        "I: Integer; R: Boolean;",
        "R := Evaluate(I, '12'); exit(Format(R) + Format(I));",
    );
    assert_eq!(t, "Yes12");
}

#[test]
fn evaluate_failure_without_using_the_result_raises_the_pinned_error() {
    // session/TestProbeSessionMiscContracts.al:Arithmetic_ErrorTexts, `evaluate_int_overflow_statement`;
    // types/TestProbeScalarContracts.al:Evaluate_BooleanFailureError.
    assert_eq!(
        raised("I: Integer;", "Evaluate(I, 'notanumber'); exit('');"),
        (
            "The value \"notanumber\" can't be evaluated into type Integer.".into(),
            "Evaluate".into()
        )
    );
}

#[test]
fn evaluate_booleans() {
    // text/TestEvaluate.al 'true' / 'false'; types/TestFormatBooleanContracts.al:112 evaluates Format(true) = 'Yes'.
    let t = text_of(
        "A: Boolean; B: Boolean; C: Boolean; D: Boolean;",
        "Evaluate(A, 'true'); Evaluate(B, 'false'); Evaluate(C, Format(true)); Evaluate(D, Format(false));
         exit(Format(A) + Format(B) + Format(C) + Format(D));",
    );
    assert_eq!(t, "YesNoYesNo");
}

#[test]
fn evaluate_dates_round_trip_through_format() {
    // text/TestEvaluate.al Evaluate_Date_ParsesDate; json/TestRoundtripContracts.al:408 (20240315D).
    let t = text_of(
        "D: Date; D2: Date; S: Text;",
        "D := 20240315D; S := Format(D); Evaluate(D2, S); exit(Format(D = D2) + S);",
    );
    assert!(t.starts_with("Yes"), "{t}");
    // Number 9 is the XML (ISO) form.
    let t = text_of("D: Date;", "Evaluate(D, '2024-03-15', 9); exit(Format(D = 20240315D));");
    assert_eq!(t, "Yes");
}

#[test]
fn evaluate_guid_and_datetime_xml_form() {
    // types/TestGuid.al:78 round trip; handlers/TestPageFieldFormats_Tests.al:1401 braces literal;
    // session/TestSessionEvaluateRegionalPatterns.al:38 DateTime with format number 9.
    let t = text_of(
        "G: Guid; G2: Guid;",
        "Evaluate(G, '{11111111-2222-3333-4444-555555555555}'); G2 := G; Evaluate(G2, Format(G)); exit(Format(G) + Format(G = G2));",
    );
    assert_eq!(t, "{11111111-2222-3333-4444-555555555555}Yes");
    let t = text_of(
        "Dt: DateTime; Ok: Boolean;",
        "Ok := Evaluate(Dt, '2026-01-02T10:11:12.0000000Z', 9); exit(Format(Ok) + Format(Dt = 0DT));",
    );
    assert_eq!(t, "YesNo");
}

#[test]
fn evaluate_of_forms_the_corpus_does_not_pin_is_refused() {
    // Grouped thousands are read (P86Eval.al); `1,00` is 100 (errors/lt-ms-refusals-129/P129Evaluate.al:420).
    assert_eq!(text_of("I: Integer;", "Evaluate(I, '1,00'); exit(Format(I));"), "100");
    assert_eq!(
        raised("B: Boolean;", "Evaluate(B, 'maybe'); exit('');"),
        (
            "The value \"maybe\" can't be evaluated into type Boolean.".into(),
            "Evaluate".into()
        )
    );
}

// ----------------------------------------------------------------- XML

#[test]
fn xml_read_from_and_get_root() {
    // xml/TestXmlDocument.al: ReadFrom parses, GetRoot returns the root element, invalid XML returns false.
    let t = text_of(
        "D: XmlDocument; E: XmlElement; Ok: Boolean;",
        "Ok := XmlDocument.ReadFrom('<r><c>hello</c></r>', D); D.GetRoot(E); exit(Format(Ok) + E.Name());",
    );
    assert_eq!(t, "Yesr");
    let t = text_of(
        "D: XmlDocument; Ok: Boolean;",
        "Ok := XmlDocument.ReadFrom('not xml', D); exit(Format(Ok));",
    );
    assert_eq!(t, "No");
    // The statement form raises (xml/TestProbeXmlContracts.al:ReadFrom_Invalid_Statement, `text only`).
    assert_eq!(
        raised("D: XmlDocument;", "XmlDocument.ReadFrom('not xml', D); exit('');"),
        (
            "The following exception was encountered when processing XML data: 'Data at the root level is invalid. Line 1, position 1.' at line 1 and position 1.".into(),
            "Xml:Xml".into()
        )
    );
}

#[test]
fn xml_inner_text_and_cdata() {
    // xml/TestXmlDocument.al, TestXmlElement.al, json/TestJsonXmlDeepContracts.al.
    let t = text_of(
        "D: XmlDocument; E: XmlElement;",
        "XmlDocument.ReadFrom('<r>content</r>', D); D.GetRoot(E); exit(E.InnerText());",
    );
    assert_eq!(t, "content");
    let t = text_of(
        "D: XmlDocument; E: XmlElement;",
        "XmlDocument.ReadFrom('<empty/>', D); D.GetRoot(E); exit('[' + E.InnerText() + ']' + E.InnerXml());",
    );
    assert_eq!(t, "[]");
    let t = text_of(
        "D: XmlDocument; E: XmlElement;",
        "XmlDocument.ReadFrom('<r><![CDATA[<not markup> & raw]]></r>', D); D.GetRoot(E); exit(E.InnerText());",
    );
    assert_eq!(t, "<not markup> & raw");
    let t = text_of(
        "D: XmlDocument; E: XmlElement;",
        "XmlDocument.ReadFrom('<item attr=\"v\">content text</item>', D); D.GetRoot(E); exit(E.InnerText());",
    );
    assert_eq!(t, "content text");
}

#[test]
fn xml_names_and_namespaces() {
    // xml/TestXmlNamespace.al and TestXmlNamespaceManager.al.
    let t = text_of(
        "D: XmlDocument; E: XmlElement; P: Text; Ns: Text; A: Boolean; B: Boolean;",
        "XmlDocument.ReadFrom('<ns:root xmlns:ns=\"http://test.com\"/>', D); D.GetRoot(E);
         A := E.GetPrefixOfNamespace('http://test.com', P); B := E.GetNamespaceOfPrefix('ns', Ns);
         exit(E.LocalName() + '|' + E.Name() + '|' + E.NamespaceUri() + '|' + Format(A) + P + '|' + Format(B) + Ns);",
    );
    assert_eq!(t, "root|ns:root|http://test.com|Yesns|Yeshttp://test.com");
    let t = text_of(
        "D: XmlDocument; E: XmlElement;",
        "XmlDocument.ReadFrom('<root xmlns=\"http://default\"/>', D); D.GetRoot(E); exit(E.LocalName() + E.NamespaceUri());",
    );
    assert_eq!(t, "roothttp://default");
    let t = text_of(
        "D: XmlDocument; E: XmlElement;",
        "XmlDocument.ReadFrom('<root/>', D); D.GetRoot(E); exit('[' + E.NamespaceUri() + ']');",
    );
    assert_eq!(t, "[]");
    let t = text_of(
        "D: XmlDocument; Ok: Boolean;",
        "Ok := XmlDocument.ReadFrom('<a:root xmlns:a=\"http://a\" xmlns:b=\"http://b\"><b:child/></a:root>', D); exit(Format(Ok));",
    );
    assert_eq!(t, "Yes");
}

#[test]
fn xml_build_and_write() {
    // xml/TestXmlDocument.al XmlDocument_WriteTo_SerializesXml; json/TestJsonXmlDeepContracts.al
    // XmlElement_TextContent_SpecialChars_EscapedOnWrite and _SpecialChars_RoundTrip.
    let t = text_of(
        "D: XmlDocument; E: XmlElement; S: Text;",
        "D := XmlDocument.Create(); E := XmlElement.Create('test'); D.Add(E); D.WriteTo(S); exit(Format(StrPos(S, '<test') > 0));",
    );
    assert_eq!(t, "Yes");
    let t = text_of(
        "D: XmlDocument; E: XmlElement; S: Text;",
        "D := XmlDocument.Create(); E := XmlElement.Create('r'); E.Add(XmlText.Create('a < b & c > d')); D.Add(E); D.WriteTo(S);
         exit(Format(StrPos(S, '&lt;') > 0) + Format(StrPos(S, '&amp;') > 0));",
    );
    assert_eq!(t, "YesYes");
    let t = text_of(
        "D: XmlDocument; D2: XmlDocument; E: XmlElement; E2: XmlElement; S: Text;",
        "D := XmlDocument.Create(); E := XmlElement.Create('r'); E.Add(XmlText.Create('x < y')); D.Add(E); D.WriteTo(S);
         XmlDocument.ReadFrom(S, D2); D2.GetRoot(E2); exit(E2.InnerText());",
    );
    assert_eq!(t, "x < y");
    // Comments survive a round trip (XmlDocument_Comment_Preserved_InRoundTrip).
    let t = text_of(
        "D: XmlDocument; S: Text;",
        "XmlDocument.ReadFrom('<root><!-- my comment --><child/></root>', D); D.WriteTo(S); exit(Format(StrPos(S, 'my comment') > 0));",
    );
    assert_eq!(t, "Yes");
}

#[test]
fn xml_element_children_and_attributes() {
    // xml/TestXmlElement.al, TestXmlExtended.al, TestXmlDeep.al, TestXmlComplete.al.
    let t = text_of(
        "E: XmlElement; C: XmlElement; N: XmlNode; F: Boolean;",
        "E := XmlElement.Create('parent'); C := XmlElement.Create('child'); E.Add(C); F := E.SelectSingleNode('child', N); exit(Format(F) + N.AsXmlElement().Name());",
    );
    assert_eq!(t, "Yeschild");
    let t = text_of(
        "D: XmlDocument; E: XmlElement;",
        "XmlDocument.ReadFrom('<root><a/><b/><a/></root>', D); D.GetRoot(E);
         exit(Format(E.GetChildElements().Count()) + Format(E.GetChildElements('a').Count()) + Format(E.GetDescendantElements('a').Count()) + Format(E.HasElements()));",
    );
    assert_eq!(t, "322Yes");
    let t = text_of(
        "E: XmlElement;",
        "E := XmlElement.Create('e'); E.SetAttribute('a', '1'); E.SetAttribute('b', '2'); E.RemoveAttribute('a');
         exit(Format(E.HasAttributes()) + Format(E.Attributes().Count())); ",
    );
    assert_eq!(t, "Yes1");
    let t = text_of(
        "E: XmlElement;",
        "E := XmlElement.Create('e'); E.SetAttribute('a', '1'); E.RemoveAllAttributes(); exit(Format(E.HasAttributes()));",
    );
    assert_eq!(t, "No");
}

#[test]
fn xml_node_lists_and_node_kinds() {
    // xml/TestXmlComplete.al: GetChildNodes indexes from 1 and includes text, CDATA and comments.
    let t = text_of(
        "D: XmlDocument; E: XmlElement; L: XmlNodeList; N: XmlNode;",
        "XmlDocument.ReadFrom('<r>hello</r>', D); D.GetRoot(E); L := E.GetChildNodes(); L.Get(1, N); exit(Format(N.IsXmlText()) + Format(N.IsXmlElement()) + Format(N.IsXmlCData()));",
    );
    assert_eq!(t, "YesNoNo");
    let t = text_of(
        "D: XmlDocument; E: XmlElement; L: XmlNodeList; N: XmlNode;",
        "XmlDocument.ReadFrom('<r><![CDATA[data]]></r>', D); D.GetRoot(E); L := E.GetChildNodes(); L.Get(1, N); exit(Format(N.IsXmlCData()));",
    );
    assert_eq!(t, "Yes");
    let t = text_of(
        "D: XmlDocument; E: XmlElement; L: XmlNodeList; N: XmlNode;",
        "XmlDocument.ReadFrom('<root><!-- comment --><a/></root>', D); D.GetRoot(E); L := E.GetChildNodes(); L.Get(1, N); exit(Format(N.IsXmlComment()) + Format(L.Count()));",
    );
    assert_eq!(t, "Yes2");
    let t = text_of(
        "D: XmlDocument; E: XmlElement; N: XmlNode;",
        "XmlDocument.ReadFrom('<r/>', D); D.GetRoot(E); N := E.AsXmlNode(); exit(Format(N.IsXmlElement()) + Format(N.IsXmlDocument()) + Format(N.IsXmlText()) + N.AsXmlElement().Name());",
    );
    assert_eq!(t, "YesNoNor");
}

#[test]
fn xml_mutation_through_nodes() {
    // xml/TestXmlDeep.al Remove/ReplaceWith; TestXmlExtended.al AddAfterSelf/AddBeforeSelf/AddFirst;
    // elements are reference types, so edits through a node are visible in the document.
    let t = text_of(
        "D: XmlDocument; E: XmlElement; N: XmlNode;",
        "XmlDocument.ReadFrom('<root><a/><b/></root>', D); D.GetRoot(E); E.GetChildElements().Get(1, N); N.AsXmlElement().Remove(); exit(Format(E.GetChildElements().Count()));",
    );
    assert_eq!(t, "1");
    let t = text_of(
        "D: XmlDocument; E: XmlElement; N: XmlNode; N2: XmlNode;",
        "XmlDocument.ReadFrom('<root><old/></root>', D); D.GetRoot(E); E.GetChildElements().Get(1, N); N.AsXmlElement().ReplaceWith(XmlElement.Create('new')); E.GetChildElements().Get(1, N2); exit(N2.AsXmlElement().Name());",
    );
    assert_eq!(t, "new");
    let t = text_of(
        "D: XmlDocument; E: XmlElement; N: XmlNode; N2: XmlNode;",
        "XmlDocument.ReadFrom('<root><b/></root>', D); D.GetRoot(E); E.GetChildElements().Get(1, N); N.AddBeforeSelf(XmlElement.Create('a')); N.AddAfterSelf(XmlElement.Create('c')); E.AddFirst(XmlElement.Create('z'));
         E.GetChildElements().Get(1, N2); exit(N2.AsXmlElement().Name() + Format(E.GetChildElements().Count()));",
    );
    assert_eq!(t, "z4");
    let t = text_of(
        "D: XmlDocument; E: XmlElement; Ch: XmlNode; P: XmlElement;",
        "XmlDocument.ReadFrom('<root><child/></root>', D); D.GetRoot(E); E.GetChildElements().Get(1, Ch); Ch.GetParent(P); exit(P.Name());",
    );
    assert_eq!(t, "root");
    let t = text_of(
        "D: XmlDocument; E: XmlElement; D2: XmlDocument;",
        "XmlDocument.ReadFrom('<r/>', D); D.GetRoot(E); exit(Format(E.GetDocument(D2)));",
    );
    assert_eq!(t, "Yes");
}

#[test]
fn xml_select_nodes_by_xpath() {
    // xml/TestXmlDocument.al XmlDocument_SelectSingleNode_FindsNode ('//child'); TestXmlExtended.al SelectNodes.
    let t = text_of(
        "D: XmlDocument; N: XmlNode;",
        "XmlDocument.ReadFrom('<root><child id=\"1\"/></root>', D); D.SelectSingleNode('//child', N); exit(N.AsXmlElement().Name());",
    );
    assert_eq!(t, "child");
    let t = text_of(
        "D: XmlDocument; E: XmlElement; L: XmlNodeList;",
        "XmlDocument.ReadFrom('<r><a/><a/><a/></r>', D); D.GetRoot(E); E.SelectNodes('a', L); exit(Format(L.Count()));",
    );
    assert_eq!(t, "3");
    // A prefixed name without a namespace manager raises (xml/XmlXPathNamespaces.al:314 `prefix_without_manager`).
    let t = text_of(
        "D: XmlDocument; N: XmlNode;",
        "XmlDocument.ReadFrom('<a:r xmlns:a=\"u\"/>', D); asserterror D.SelectSingleNode('/a:r', N); exit(GetLastErrorCode());",
    );
    assert_eq!(t, "Xml:XPath");
}

#[test]
fn xml_declarations_attributes_and_processing_instructions() {
    // xml/TestXmlComplete.al and TestXmlDeep.al.
    let t = text_of(
        "X: XmlDeclaration;",
        "X := XmlDeclaration.Create('1.0', 'UTF-8', ''); exit(X.Encoding());",
    );
    assert_eq!(t, "UTF-8");
    let t = text_of(
        "X: XmlDeclaration;",
        "X := XmlDeclaration.Create('1.0', '', ''); X.Encoding('UTF-16'); X.Standalone('yes'); exit(X.Encoding() + X.Standalone() + X.Version());",
    );
    assert_eq!(t, "UTF-16yes1.0");
    let t = text_of(
        "D: XmlDocument; X: XmlDeclaration; Y: XmlDeclaration;",
        "XmlDocument.ReadFrom('<root/>', D); X := XmlDeclaration.Create('1.0', 'UTF-8', ''); D.SetDeclaration(X); D.GetDeclaration(Y); exit(Y.Encoding());",
    );
    assert_eq!(t, "UTF-8");
    let t = text_of(
        "P: XmlProcessingInstruction; Tg: Text;",
        "P := XmlProcessingInstruction.Create('mypi', 'data'); P.GetTarget(Tg); exit(Tg);",
    );
    assert_eq!(t, "mypi");
    let t = text_of(
        "A: XmlAttribute; N: XmlNode;",
        "A := XmlAttribute.Create('color', 'red'); N := A.AsXmlNode(); exit(A.Name() + A.Value() + A.LocalName() + '[' + A.NamespaceUri() + ']' + Format(N.IsXmlAttribute()));",
    );
    assert_eq!(t, "colorredcolor[]Yes");
    let t = text_of(
        "A: XmlAttribute;",
        "A := XmlAttribute.CreateNamespaceDeclaration('ns', 'http://example.com'); exit(A.NamespacePrefix() + Format(A.IsNamespaceDeclaration()));",
    );
    assert_eq!(t, "xmlnsYes");
}

#[test]
fn xml_get_document_type_without_a_doctype() {
    // xml/TestXmlRuntimeContracts.al:XmlDocument_GetDocumentType_WithoutDoctype
    let t = text_of(
        "D: XmlDocument; T: XmlDocumentType;",
        "XmlDocument.ReadFrom('<root/>', D); asserterror D.GetDocumentType(T); exit(GetLastErrorText() + '|' + GetLastErrorCode() + '|' + Format(D.GetDocumentType(T)));",
    );
    assert_eq!(t, "The XML node is not set.|XmlNodeNotFound|No");
}

#[test]
fn variants_report_json_and_xml_handles() {
    // types/TestVariantComplete.al: IsJsonArray/IsJsonObject/IsJsonToken/IsJsonValue/IsXmlElement.
    let t = text_of(
        "V: Variant; A: JsonArray; O: JsonObject; T: JsonToken; X: JsonValue; D: XmlDocument; E: XmlElement; S: Text;",
        "V := A; S := Format(V.IsJsonArray()) + Format(V.IsJsonObject());
         V := O; S += Format(V.IsJsonObject()) + Format(V.IsJsonArray());
         T := O.AsToken(); V := T; S += Format(V.IsJsonToken());
         V := X; S += Format(V.IsJsonValue());
         XmlDocument.ReadFrom('<r/>', D); D.GetRoot(E); V := E; S += Format(V.IsXmlElement());
         V := 5; S += Format(V.IsJsonArray());
         exit(S);",
    );
    assert_eq!(t, "YesNoYesNoYesYesYesNo");
}

// ---------------------------------------------------------------- Text

#[test]
fn text_search_methods() {
    // text/TestStringMethodContracts.al, TestTextExtended.al, TestTextProcessingContracts.al.
    let t = text_of(
        "S: Text;",
        "S := 'Hello World';
         exit(Format(S.Contains('World')) + Format(S.Contains('world')) + Format(S.StartsWith('Hello')) + Format(S.StartsWith('World'))
              + Format(S.EndsWith('World')) + Format(S.EndsWith('Hello')));",
    );
    assert_eq!(t, "YesNoYesNoYesNo");
    let t = text_of(
        "S: Text;",
        "S := 'hello';
         exit(Format(S.IndexOf('ll')) + Format(S.IndexOf('xyz')) + Format(S.IndexOf('l', 3)) + Format('hello'.IndexOf('h')) + Format(S.LastIndexOf('l')) + Format(S.LastIndexOf('xyz')));",
    );
    assert_eq!(t, "30314".to_string() + "0");
    let t = text_of(
        "S: Text; Chars: List of [Char]; Other: List of [Char];",
        "S := 'hello'; Chars.Add('e'); Chars.Add('o'); Other.Add('x'); Other.Add('y'); exit(Format(S.IndexOfAny(Chars)) + Format(S.IndexOfAny(Other)));",
    );
    assert_eq!(t, "20");
    // text/TestTextProcessingContracts.al StrPos_EmptySubstring_ReturnsZero.
    let t = text_of(
        "",
        "exit(Format(StrPos('hello', 'h')) + Format(StrPos('hello', 'z')) + Format(StrPos('hello', '')));",
    );
    assert_eq!(t, "100");
}

#[test]
fn text_transforming_methods() {
    // text/TestStringMethodContracts.al and TestTextExtended.al.
    let t = text_of(
        "S: Text;",
        "S := 'Hello World 123'; exit(S.ToUpper() + '|' + S.ToLower());",
    );
    assert_eq!(t, "HELLO WORLD 123|hello world 123");
    let t = text_of(
        "S: Text;",
        "S := '  hello  '; exit('[' + S.Trim() + '][' + S.TrimStart() + '][' + S.TrimEnd() + ']');",
    );
    assert_eq!(t, "[hello][hello  ][  hello]");
    let t = text_of("S: Text;", "S := 'a,b,,'; exit(S.TrimEnd(','));");
    assert_eq!(t, "a,b");
    let t = text_of(
        "S: Text;",
        "S := 'hello'; exit(S.Replace('e', 'X').Replace('o', 'X') + S.Replace('hello', 'world'));",
    );
    assert_eq!(t, "hXllXworld");
    let t = text_of(
        "S: Text;",
        "S := 'Hello World'; exit(S.Substring(7) + '|' + S.Substring(1, 5) + '|' + 'hello'.Substring(2, 3));",
    );
    assert_eq!(t, "World|Hello|ell");
    let t = text_of(
        "S: Text;",
        "S := 'hi'; exit(S.PadLeft(5) + '|' + S.PadRight(5) + '|' + S.PadLeft(5, '#') + '|' + S.PadRight(5, '#'));",
    );
    assert_eq!(t, "   hi|hi   |###hi|hi###");
}

#[test]
fn text_functions() {
    // text/TestTextExtended.al, TestTextOperations.al, TestTextProcessingContracts.al.
    let t = text_of(
        "",
        "exit(ConvertStr('hello', 'eo', '--') + ConvertStr('hello', 'helo', 'HELO') + ConvertStr('ab', 'ab', 'ba'));",
    );
    assert_eq!(t, "h-ll-HELLOba");
    let t = text_of(
        "",
        "exit(DelChr('hello', '=', 'e') + '|' + DelChr('  hello  ', '<>') + '|' + DelChr('h e l l o', '=', ' ') + '|' + DelChr('  hello  ', '<>', ' ') + '|' + DelChr('   ', '<>', ' ') + '|');",
    );
    assert_eq!(t, "hllo|hello|hello|hello||");
    // errors/lt-ms-refusals-136/P136Text.al:P136_DelChr_ThreeCharWhere, P136_DelChr_DoubledAndOddWhere.
    let t = text_of(
        "",
        "exit(DelChr('xxaxbxx', '=<>', 'x') + '|' + DelChr('xxaxbxx', '<=>', 'x') + '|' + DelChr('xxaxbxx', '=>', 'x') + '|' + DelChr('xxaxbxx', '<<', 'x') + '|' + DelChr('xxaxbxx', '>>', 'x') + '|' + DelChr('xxaxbxx', '<>>', 'x') + '|' + DelChr('xxaxbxx', '><', 'x') + '|' + DelChr('xxaxbxx', '', 'x') + '|');",
    );
    assert_eq!(t, "ab|ab|ab|axbxx|xxaxb|axb|axb|xxaxbxx|");
    let (m, c) = raised("", "exit(DelChr('xxaxbxx', '< ', 'x'));");
    assert_eq!(
        (m.as_str(), c.as_str()),
        ("Incorrect use of the where parameter.", "ALSystemString")
    );
    let t = text_of(
        "",
        "exit(IncStr('ABC1') + '|' + IncStr('ABC9') + '|' + IncStr('Item 001') + '|' + IncStr('Item 009') + '|' + IncStr('ABC') + '|');",
    );
    assert_eq!(t, "ABC2|ABC10|Item 002|Item 010||");
    let t = text_of(
        "",
        "exit(PadStr('hi', 5, ' ') + '|' + PadStr('abcdef', 4, ' ') + '|' + PadStr('hello', 5, ' ') + '|' + PadStr('', 5, 'X'));",
    );
    assert_eq!(t, "hi   |abcd|hello|XXXXX");
    let t = text_of(
        "",
        "exit(DelStr('Hello', 2, 1) + '|' + InsStr('Hello', 'x', 2) + '|' + SelectStr(2, 'A,B,C'));",
    );
    assert_eq!(t, "Hllo|Hxello|B");
    // SelectStr with an index outside the list throws (types/TestFormatEvaluateTextContracts.al:SelectStr_IndexOutsideTheList_Throws).
    let t = text_of(
        "",
        "asserterror exit(SelectStr(5, 'A,B,C')); exit(GetLastErrorText() + '|' + GetLastErrorCode());",
    );
    assert_eq!(
        t,
        "The SELECTSTR comma-string A,B,C does not contain a value for index 5.|InvalidStringFormat"
    );
}

#[test]
fn text_split() {
    // text/TestTextExtended.al TextSplit_*.
    let t = text_of(
        "P: List of [Text];",
        "P := 'a,b,c'.Split(','); exit(Format(P.Count()) + P.Get(1) + P.Get(3));",
    );
    assert_eq!(t, "3ac");
    let t = text_of(
        "P: List of [Text]; Seps: List of [Char];",
        "Seps.Add(','); Seps.Add(';'); P := 'a,b;c'.Split(Seps); exit(Format(P.Count()));",
    );
    assert_eq!(t, "3");
    let t = text_of(
        "P: List of [Text];",
        "P := 'a,b;c|d'.Split(',', ';', '|'); exit(Format(P.Count()) + P.Get(4));",
    );
    assert_eq!(t, "4d");
    let t = text_of(
        "P: List of [Text];",
        "P := 'a--b::c'.Split('--', '::'); exit(Format(P.Count()) + P.Get(2));",
    );
    assert_eq!(t, "3b");
    let t = text_of(
        "P: List of [Text];",
        "P := 'a-b'.Split('--', '::'); exit(Format(P.Count()) + P.Get(1));",
    );
    assert_eq!(t, "1a-b");
    let t = text_of(
        "P: List of [Text];",
        "P := 'a,;b;'.Split(',', ';'); exit(Format(P.Count()) + P.Get(1) + '[' + P.Get(2) + ']' + P.Get(3) + '[' + P.Get(4) + ']');",
    );
    assert_eq!(t, "4a[]b[]");
    let t = text_of(
        "P: List of [Text];",
        "P := 'a b c'.Split(); exit(Format(P.Count()) + P.Get(2));",
    );
    assert_eq!(t, "3b");
    let t = text_of(
        "S: Text; P: List of [Text];",
        "S := ''; P := S.Split(','); exit(Format(P.Count()) + '[' + P.Get(1) + ']');",
    );
    assert_eq!(t, "1[]");
}

#[test]
fn text_builder() {
    // text/TestTextBuilder.al and TestTextProcessingContracts.al TextBuilder_Insert_AtOne_Prepends.
    let t = text_of(
        "B: TextBuilder;",
        "B.Append('foo'); B.Append('bar'); exit(B.ToText() + Format(B.Length()));",
    );
    assert_eq!(t, "foobar6");
    let t = text_of(
        "B: TextBuilder;",
        "B.Append('hllo'); B.Insert(2, 'e'); B.Append(' '); B.Insert(1, '>'); exit(B.ToText());",
    );
    assert_eq!(t, ">hello ");
    let t = text_of(
        "B: TextBuilder;",
        "B.Append('hello'); B.Remove(2, 2); exit(B.ToText());",
    );
    assert_eq!(t, "hlo");
    let t = text_of(
        "B: TextBuilder;",
        "B.Append('data'); B.Clear(); exit('[' + B.ToText() + ']' + Format(B.Length()));",
    );
    assert_eq!(t, "[]0");
    let t = text_of(
        "B: TextBuilder;",
        "B.EnsureCapacity(100); exit('[' + B.ToText() + ']');",
    );
    assert_eq!(t, "[]");
    let t = text_of(
        "B: TextBuilder;",
        "B.AppendLine('line1'); B.Append('line2'); exit(Format(StrPos(B.ToText(), 'line1') > 0) + Format(StrPos(B.ToText(), 'line2') > 0));",
    );
    assert_eq!(t, "YesYes");
    // TextBuilder is a reference type like the other library handles.
    let t = text_of(
        "B: TextBuilder; C: TextBuilder;",
        "C := B; B.Append('shared'); exit(C.ToText());",
    );
    assert_eq!(t, "shared");
}

#[test]
fn big_text() {
    // text/TestBigText.al.
    let t = text_of(
        "B: BigText; S: Text;",
        "B.AddText('Hello World'); B.GetSubText(S, 1, 5); exit(S + Format(B.Length()));",
    );
    assert_eq!(t, "Hello11");
    let t = text_of(
        "B: BigText; S: Text;",
        "B.AddText('foo'); B.AddText('bar'); B.GetSubText(S, 1, 6); exit(S + Format(B.Length()));",
    );
    assert_eq!(t, "foobar6");
    // session/TestSystemExtended.al BigTextTextPos_*: 1-based, 0 when absent.
    let t = text_of(
        "B: BigText;",
        "B.AddText('Hello World'); exit(Format(B.TextPos('World')) + Format(B.TextPos('Hello')) + Format(B.TextPos('xyz')));",
    );
    assert_eq!(t, "7".to_string() + "10");
    // A position in AddText inserts before that character (types/lt-totext-bigtext-codeunit-run/P93BigText.al:95-160).
    assert_eq!(
        text_of(
            "B: BigText; S: Text;",
            "B.AddText('bc'); B.AddText('a', 1); B.GetSubText(S, 1); exit(S);"
        ),
        "abc"
    );
}

// streams/StTBigText.al:120 GetSubTextForms and :157 GetSubTextErrors: a length past the end gives the rest; a start one past
// the end or further gives nothing; a start below 1 or a negative length raise CopyStr's range errors.
// streams/StTBigText.al:240 ConvertTextAndBack: Format gives the whole text.
#[test]
fn big_text_get_sub_text_clips_at_the_end() {
    let v = "B: BigText; S: Text; N: Integer;";
    let t = text_of(
        v,
        "B.AddText('0123456789'); N := B.GetSubText(S, 8, 100); exit(S + '/' + Format(N));",
    );
    assert_eq!(t, "789/3");
    let t = text_of(
        v,
        "B.AddText('0123456789'); N := B.GetSubText(S, 11, 1); exit(S + '/' + Format(N));",
    );
    assert_eq!(t, "/0");
    let t = text_of(v, "B.AddText('dir'); B.AddText('ect'); exit(Format(B));");
    assert_eq!(t, "direct");
    let (m, c) = raised(v, "B.AddText('0123456789'); N := B.GetSubText(S, 0, 1); exit('');");
    assert_eq!(
        (m.as_str(), c.as_str()),
        (
            "The value of COPYSTR parameter 2 is outside of the permitted range. The current value is: 0. The permitted range is: from 1 to 2,147,483,647.",
            "OutsidePermittedRange"
        )
    );
    let (m, _) = raised(v, "B.AddText('0123456789'); N := B.GetSubText(S, 1, -1); exit('');");
    assert!(m.starts_with("The value of COPYSTR parameter 3 is outside of the permitted range. The current value is: -1. The permitted range is: from 0"), "{m}");
    // A start further out gives nothing too (corpus StTBigText.al:169 GetSubTextErrors and the cases of main that pass on it).
    let t = text_of(
        v,
        "B.AddText('0123456789'); N := B.GetSubText(S, 20); exit(S + '/' + Format(N));",
    );
    assert_eq!(t, "/0");
}

#[test]
fn max_str_len_reads_the_declared_length() {
    // text/TestTextExtended.al MaxStrLen_*; also record fields (handlers/OngCard.Page.al).
    let t = text_of(
        "T50: Text[50]; U: Text; C: Code[10];",
        "exit(Format(MaxStrLen(T50)) + '|' + Format(MaxStrLen(U)) + '|' + Format(MaxStrLen(C)));",
    );
    assert_eq!(t, "50|2147483647|10");
}

// ----------------------------------------------------------- SecretText

#[test]
fn secret_text_is_observable_only_through_is_empty() {
    // types/TestSecretText.al: construction, emptiness, composition, copying and containers.
    let t = text_of(
        "S: SecretText; E: SecretText; P: Text; C: SecretText; D: SecretText;",
        "S := SecretStrSubstNo('a-constant-template'); P := ''; C := P;
         D := SecretStrSubstNo('pwd=%1', E);
         exit(Format(S.IsEmpty()) + Format(SecretStrSubstNo('').IsEmpty()) + Format(E.IsEmpty()) + Format(C.IsEmpty()) + Format(SecretStrSubstNo('%1', E).IsEmpty()) + Format(D.IsEmpty()));",
    );
    assert_eq!(t, "NoYesYesYesYesNo");
    let t = text_of(
        "S: SecretText; P: Text; Pw: SecretText; U: SecretText; Composed: SecretText; Copy: SecretText;",
        "P := ' '; S := P; Pw := SecretStrSubstNo('p4ssw0rd'); U := SecretStrSubstNo('admin'); Composed := SecretStrSubstNo('user=%1;pwd=%2', U, Pw);
         Copy := Composed; Composed := SecretStrSubstNo('');
         exit(Format(S.IsEmpty()) + Format(Copy.IsEmpty()) + Format(Composed.IsEmpty()));",
    );
    assert_eq!(t, "NoNoYes");
    let t = text_of(
        "L: List of [SecretText]; D: Dictionary of [Text, SecretText]; S: SecretText; R: SecretText; R2: SecretText;",
        "S := SecretStrSubstNo('in-a-list'); L.Add(S); R := L.Get(1); D.Add('api-key', S); R2 := D.Get('api-key');
         exit(Format(L.Count()) + Format(R.IsEmpty()) + Format(R2.IsEmpty()) + Format(D.ContainsKey('api-key')));",
    );
    assert_eq!(t, "1NoNoYes");
}

// ------------------------------------------------- parser edge cases

#[test]
fn json_parser_edge_cases() {
    // Escapes, surrogate pairs, number forms and white space (no corpus pin beyond the round trips;
    // these guard the hand-written parser).
    let t = text_of(
        "J: JsonObject; S: Text;",
        "J.ReadFrom('  { \"a\" : \"\\u00e9\\n\\\\\\\"x\\ud83d\\ude00\" , \"b\":[ ] , \"c\" : { } }  '); J.WriteTo(S); exit(S);",
    );
    assert_eq!(t, "{\"a\":\"é\\n\\\\\\\"x😀\",\"b\":[],\"c\":{}}");
    let t = text_of(
        "J: JsonObject;",
        "J.ReadFrom('{\"a\":1e3,\"b\":-0.5,\"c\":1.0,\"d\":123456789012345,\"e\":-7}');
         exit(Format(J.GetDecimal('a'), 0, 2) + '|' + Format(J.GetDecimal('b'), 0, 2) + '|' + Format(J.GetDecimal('c'), 0, 2) + '|' + Format(J.GetDecimal('d'), 0, 2) + '|' + Format(J.GetInteger('e')));",
    );
    assert_eq!(t, "1000|-0.5|1|123456789012345|-7");
    for bad in [
        "{",
        "{\"a\":}",
        "[1,]",
        "{\"a\" 1}",
        "01",
        "'x'",
        "",
        "{\"a\":\"\\x\"}",
        "[1 2]",
        "nul",
    ] {
        let src = "J: JsonObject; A: JsonArray; Ok: Boolean;";
        let body = format!("Ok := J.ReadFrom('{}'); exit(Format(Ok));", bad.replace('\'', "''"));
        // Objects and arrays differ in the accepted top-level kind; both reject invalid text.
        assert_eq!(text_of(src, &body), "No", "{bad}");
    }
    // A top-level kind mismatch is a failed ReadFrom, not a silent success.
    let t = text_of(
        "J: JsonObject; Ok: Boolean;",
        "Ok := J.ReadFrom('[1]'); exit(Format(Ok));",
    );
    assert_eq!(t, "No");
    let t = text_of(
        "A: JsonArray; Ok: Boolean;",
        "Ok := A.ReadFrom('{}'); exit(Format(Ok));",
    );
    assert_eq!(t, "No");
    // Duplicate keys in the input are refused (the winner is not pinned).
    assert!(unsupported(
        "J: JsonObject;",
        "J.ReadFrom('{\"a\":1,\"a\":2}'); exit('');"
    ));
}

#[test]
fn json_tree_sharing_and_paths() {
    // A free node is shared by Add, an attached one is copied; Path walks parents.
    let t = text_of(
        "J: JsonObject; I: JsonObject; T: JsonToken; K: JsonObject; S: Text;",
        "J.Add('inner', I); I.Add('late', 1); K.Add('again', I); I.Add('later', 2); J.WriteTo(S); exit(S + '|' + Format(K.Contains('again')));",
    );
    assert_eq!(t, "{\"inner\":{\"late\":1,\"later\":2}}|Yes");
    let t = text_of(
        "A: JsonArray; J: JsonObject; T: JsonToken;",
        "J.ReadFrom('{\"a\":{\"b\":[10,{\"c\":1}]}}'); J.SelectToken('a.b[1].c', T); exit(T.Path());",
    );
    assert_eq!(t, "a.b[1].c");
    // A removed child no longer reports a parent.
    let t = text_of(
        "J: JsonObject; T: JsonToken;",
        "J.ReadFrom('{\"x\":{\"y\":1}}'); J.Get('x', T); J.Remove('x'); exit('[' + T.Path() + ']');",
    );
    assert_eq!(t, "[]");
    // ReadFrom into an object that is itself a child keeps it attached.
    let t = text_of(
        "J: JsonObject; T: JsonToken; O: JsonObject;",
        "J.ReadFrom('{\"x\":{}}'); J.Get('x', T); O := T.AsObject(); O.ReadFrom('{\"y\":2}'); exit(T.Path() + Format(O.GetInteger('y')));",
    );
    assert_eq!(t, "x2");
}

#[test]
fn xml_parser_edge_cases() {
    let t = text_of(
        "D: XmlDocument; E: XmlElement; S: Text;",
        "XmlDocument.ReadFrom('<?xml version=\"1.0\" encoding=\"utf-8\"?><!-- c --><r a=''x&amp;y'' b=\"&#65;&#x42;\"><![CDATA[<>]]>t&lt;&gt;&quot;&apos;</r>', D);
         D.GetRoot(E); exit(E.InnerText() + '|' + Format(E.Attributes().Count()));",
    );
    assert_eq!(t, "<>t<>\"'|2");
    let t = text_of(
        "D: XmlDocument; E: XmlElement; A: XmlAttribute;",
        "XmlDocument.ReadFrom('<r a=''x&amp;y'' b=\"&#65;&#x42;\"/>', D); D.GetRoot(E); E.Attributes().Get('a', A); exit(A.Value()); ",
    );
    assert_eq!(t, "x&y");
    for bad in [
        "<a>",
        "<a></b>",
        "<a/><b/>",
        "text",
        "<a b=1/>",
        "<a b='1' b='2'/>",
        "<a>&nope;</a>",
        "<a:b/>",
        "<a><b></a></b>",
        "<>",
        "<a",
        "<a>]]></a>",
        "<a x='<'/>",
        "",
    ] {
        let body = format!(
            "Ok := XmlDocument.ReadFrom('{}', D); exit(Format(Ok));",
            bad.replace('\'', "''")
        );
        assert_eq!(text_of("D: XmlDocument; Ok: Boolean;", &body), "No", "{bad}");
    }
    // Namespaces: undeclaring the default puts `a` in no namespace, so both lookups find it, and `p:b` is in u2.
    let t = text_of(
        "D: XmlDocument; E: XmlElement; L: XmlNodeList; N: XmlNode; C: XmlElement; A: XmlAttribute;",
        "XmlDocument.ReadFrom('<r xmlns=\"u1\" xmlns:p=\"u2\"><a xmlns=\"\" x=\"1\"/><p:b p:y=\"2\" z=\"3\"/></r>', D); D.GetRoot(E);
         exit(E.NamespaceUri() + '|' + Format(E.GetChildElements('a').Count()) + '|' + Format(E.GetChildElements('a', '').Count()) + '|' + Format(E.GetChildElements('b', 'u2').Count()));",
    );
    assert_eq!(t, "u1|1|1|1");
    // Serialisation round trip keeps prefixes and declares what a created tree needs.
    let t = text_of(
        "D: XmlDocument; E: XmlElement; S: Text;",
        "XmlDocument.ReadFrom('<r xmlns:p=\"u2\"><p:b p:y=\"2\"/></r>', D); D.WriteTo(S); exit(S);",
    );
    assert_eq!(
        t,
        "<?xml version=\"1.0\" encoding=\"utf-16\"?>\r\n<r xmlns:p=\"u2\">\r\n  <p:b p:y=\"2\" />\r\n</r>"
    );
    let t = text_of(
        "E: XmlElement; C: XmlElement; S: Text;",
        "E := XmlElement.Create('root', 'urn:x'); C := XmlElement.Create('child', 'urn:x'); E.Add(C); E.SetAttribute('a', 'v\"&<'); E.WriteTo(S); exit(S);",
    );
    assert_eq!(
        t,
        "<root xmlns=\"urn:x\" a=\"v&quot;&amp;&lt;\">\r\n  <child />\r\n</root>"
    );
}

// Evaluate into an enum: Format(Enum, 0, 9) gives the ordinal in this runtime, and System
// Application's "Reten. Pol. Allowed Tbl. Impl." reads it back with Evaluate(Enum, Text, 9). The
// member names are pinned by types/TestBlankEnumInitTests.Codeunit.al:92 (a named InitValue is read
// by the same evaluator); the text of a failed statement form is not pinned and stays refused.
fn enum_text_of(vars: &str, body: &str) -> Result<Value, Signal> {
    let src = format!(
        "enum 50101 \"Ev Enum\" {{ Extensible = true; value(0; A) {{ }} value(1; Bee) {{ }} value(5; Zed) {{ }} }}\n{}",
        cu(&format!("procedure F(): Text var {vars} begin {body} end;"))
    );
    run(&src, "F")
}

#[test]
fn evaluate_into_an_enum_reads_the_xml_ordinal_and_member_names() {
    let t = |body: &str| match enum_text_of("E: Enum \"Ev Enum\"; R: Boolean;", body) {
        Ok(Value::Text(s)) => s,
        other => panic!("{other:?}"),
    };
    assert_eq!(
        t("R := Evaluate(E, Format(E::Zed, 0, 9), 9); exit(Format(R) + Format(E));"),
        "YesZed"
    );
    assert_eq!(t("R := Evaluate(E, '1', 9); exit(Format(R) + Format(E));"), "YesBee");
    assert_eq!(t("R := Evaluate(E, 'Zed'); exit(Format(R) + Format(E));"), "YesZed");
    // A name that is none makes Evaluate return false. A number that is no member is accepted with
    // format 0 (types/ProbeTypeFormsAndFilters.al:Enum_Evaluate_Forms) and with format 9 (P86Eval.al, codeunit "LT P86 Eval Enum").
    assert_eq!(
        t("E := E::Bee; R := Evaluate(E, '2'); exit(Format(R) + Format(E, 0, 2));"),
        "Yes2"
    );
    assert_eq!(
        t("E := E::Bee; R := Evaluate(E, '2', 9); exit(Format(R) + Format(E, 0, 2));"),
        "Yes2"
    );
    assert_eq!(
        t("E := E::Bee; R := Evaluate(E, 'Nope'); exit(Format(R) + Format(E));"),
        "NoBee"
    );
    // The statement form of a failure raises (P86Eval.al, codeunit "LT P86 Eval Enum": `Dflt1_Error_FNone`).
    match enum_text_of("E: Enum \"Ev Enum\";", "Evaluate(E, 'Nope'); exit('');") {
        Err(Signal::Error(e)) => assert!(
            e.message
                .starts_with("'Nope' is not an option. The existing options are: "),
            "{}",
            e.message
        ),
        other => panic!("{other:?}"),
    }
}

#[test]
fn evaluate_a_date_formula_with_format_number_2() {
    // System Application stores `Format(DateFormula, 0, 2)` and reads it back with `Evaluate(.., 2)`
    // ("Reten. Pol. Allowed Tbl. Impl."). The formula text is the same as in format 0, which
    // types/TestDateFormulaContracts.al:21-139 pins; the runtime has the English formula letters only.
    let t = text_of(
        "F: DateFormula; G: DateFormula; R: Boolean;",
        "Evaluate(F, '<-1Y>', 2); R := Evaluate(G, Format(F, 0, 2), 2); exit(Format(R) + Format(G, 0, 2) + Format(F, 0, 2));",
    );
    assert_eq!(t, "Yes-1Y-1Y");
}

#[test]
fn clearing_a_json_variable_gives_a_fresh_empty_one() {
    // Clear resets a variable to its default, which for JsonArray and JsonObject is the empty one
    // (the fresh variable of json/TestJsonArray.al:90 and TestJsonObject.al:114 writes `[]` / `{}`).
    // System Application's `Clear(TableFilters)` relies on it.
    let t = text_of(
        "A: JsonArray; J: JsonObject; S: Text; U: Text; V: Text;",
        "A.Add(1); Clear(A); A.WriteTo(S); A.Add(2); A.WriteTo(U); J.Add('k', 1); Clear(J); J.WriteTo(V); exit(S + U + V);",
    );
    assert_eq!(t, "[][2]{}");
}

#[test]
fn clearing_an_array_resets_every_element() {
    // Doc (Clear): a variable goes back to its initial value, an array element by element. The
    // oracle runs pageupdate/PageUpdateFacts.Page.al:58 (`Clear(Derived)`, an array of Decimal).
    let t = text_of(
        "D: array[3] of Decimal; S: array[2] of Text; I: Integer;",
        "D[1] := 5; D[3] := 7; S[2] := 'x'; Clear(D); Clear(S); exit(Format(D[1]) + Format(D[3]) + '|' + S[2] + '|' + Format(StrLen(S[1])));",
    );
    assert_eq!(t, "00||0");
}
