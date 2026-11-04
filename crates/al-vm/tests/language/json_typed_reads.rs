//! JSON reads the corpus pins across types: AsText of typed values (json/json/P101ValueSetValue.al),
//! GetChar of numbers and texts (P101PathValues.al), a Char read as a number (json/json/JsonTypedCharRead.al),
//! IndexOf across number types (JsonTypedIndexOfCross.al), Values and RemoveAt (P101PathValues.al, json/JsonArrayBasics.al).

fn run(decl: &str, body: &str) -> String {
    let src = format!("codeunit 50100 C {{ procedure F(): Text var S: Text; {decl} begin {body} end; }}");
    al_testkit::text(al_testkit::run_in(&src, "C", "F"))
}

#[test]
fn typed_values_read_as_their_written_text() {
    let out = run(
        "Jv: JsonValue; By: Byte; Da: Date; Ti: Time; Du: Duration; Op: Option A,B,C;",
        "By := 255; Jv.SetValue(By); S := Jv.AsText() + '|';
         Da := 20240229D; Jv.SetValue(Da); S += Jv.AsText() + '|';
         Ti := 123456T; Jv.SetValue(Ti); S += Jv.AsText() + '|';
         Du := 1500; Jv.SetValue(Du); S += Jv.AsText() + '|';
         Op := Op::C; Jv.SetValue(Op); S += Jv.AsText();
         exit(S);",
    );
    assert_eq!(out, "255|2024-02-29|12:34:56|1500|2");
}

#[test]
fn get_char_of_a_number_and_a_char_as_a_number() {
    let out = run(
        "O: JsonObject; A: JsonArray; Ch: Char; T: JsonToken;",
        "O.Add('i', 8364); S := Format(O.GetChar('i'));
         Ch := 65; A.Add(Ch); A.Get(0, T); S += Format(T.AsValue().AsInteger()) + Format(T.AsValue().AsDecimal());
         exit(S);",
    );
    assert_eq!(out, "€6565");
    let src = "codeunit 50100 C { procedure F(): Text var O: JsonObject; begin O.Add('t', 'xyz'); exit(Format(O.GetChar('t'))); end; }";
    assert_eq!(
        al_testkit::error_message(al_testkit::run_in(src, "C", "F")),
        "Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavJsonValue to System.Char."
    );
}

#[test]
fn index_of_across_number_types() {
    let out = run(
        "A: JsonArray; Ig: Integer; By: Byte; De: Decimal; Ch: Char;",
        "By := 255; A.Add(By); Ig := 0; A.Add(Ig); De := 3; A.Add(De); Ch := 70; A.Add(Ch);
         Ig := 255; S := Format(A.IndexOf(Ig)) + ',';
         By := 0; S += Format(A.IndexOf(By)) + ',';
         Ig := 3; S += Format(A.IndexOf(Ig)) + ',';
         Ig := 70; S += Format(A.IndexOf(Ig));
         exit(S);",
    );
    assert_eq!(out, "0,1,-1,3");
}

#[test]
fn values_and_remove_at() {
    let out = run(
        "O: JsonObject; A: JsonArray; L: List of [JsonToken]; T: JsonToken; R: Text;",
        "O.ReadFrom('{\"a\":1,\"b\":\"two\"}'); L := O.Values(); O.Add('c', 3);
         foreach T in L do begin T.WriteTo(R); S += R + ';'; end;
         A.ReadFrom('[1,2,3]'); S += Format(A.RemoveAt(1)); A.WriteTo(R); S += R;
         exit(S);",
    );
    assert_eq!(out, "1;\"two\";Yes[1,3]");
    let src = "codeunit 50100 C { procedure F() var A: JsonArray; begin A.Add(1); A.RemoveAt(1); end; }";
    assert_eq!(
        al_testkit::error_message(al_testkit::run_in(src, "C", "F")),
        "Index out of bounds."
    );
}
