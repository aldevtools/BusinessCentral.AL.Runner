//! `SelectTokens(Path, var List of [JsonToken])` on JsonObject, JsonArray and JsonToken, the forms
//! json/json/SelectTokensList.al, json/AyJsonTokenSelectTokens.al and
//! json/DcJsonLeads.al:412 pin.

fn run(decl: &str, body: &str) -> String {
    let src = format!(
        "codeunit 50100 C {{
        local procedure R(L: List of [JsonToken]): Text var T: JsonToken; S: Text; P: Text; begin
            foreach T in L do begin T.WriteTo(P); S += P + '@' + T.Path() + ';'; end;
            exit(Format(L.Count()) + ':' + S); end;
        procedure F(): Text var L: List of [JsonToken]; X: JsonToken; B: Boolean; S: Text; {decl} begin {body} end; }}"
    );
    al_testkit::text(al_testkit::run_in(&src, "C", "F"))
}

#[test]
fn array_forms() {
    let out = run(
        "A: JsonArray;",
        "A.ReadFrom('[{\"a\":1,\"n\":{\"a\":\"deep\"}},{\"a\":2},{\"b\":3},[{\"a\":4}]]');
         B := A.SelectTokens('$[0].a', L); S += Format(B) + R(L) + '|';
         B := A.SelectTokens('$[9]', L); S += Format(B) + R(L) + '|';
         B := A.SelectTokens('$[*].a', L); S += Format(B) + R(L) + '|';
         B := A.SelectTokens('$..a', L); S += Format(B) + R(L) + '|';
         B := A.SelectTokens('$[1:3]', L); S += Format(B) + R(L) + '|';
         exit(S);",
    );
    assert_eq!(
        out,
        "Yes1:1@[0].a;|Yes0:|Yes2:1@[0].a;2@[1].a;|Yes4:1@[0].a;\"deep\"@[0].n.a;2@[1].a;4@[3][0].a;|Yes2:{\"a\":2}@[1];{\"b\":3}@[2];|"
    );
}

#[test]
fn object_forms_and_the_filter() {
    let out = run(
        "O: JsonObject;",
        "O.ReadFrom('{\"a\":1,\"list\":[{\"id\":10},{\"id\":20},{\"x\":30}],\"n\":{\"a\":\"deep\"}}');
         B := O.SelectTokens('$.list[?(@.id > 10)]', L); S += Format(B) + R(L) + '|';
         B := O.SelectTokens('$.*', L); S += Format(B) + Format(L.Count()) + '|';
         B := O.SelectTokens('', L); S += Format(B) + Format(L.Count()) + '|';
         X.ReadFrom('7');
         B := X.SelectTokens('$', L); S += Format(B) + R(L) + '|';
         B := X.SelectTokens('$.a', L); S += Format(B) + R(L) + '|';
         exit(S);",
    );
    assert_eq!(out, "Yes1:{\"id\":20}@list[1];|Yes3|Yes1|Yes1:7@;|Yes0:|");
}

#[test]
fn an_invalid_path_answers_false_keeps_the_list_and_sets_the_last_error() {
    let out = run(
        "O: JsonObject;",
        "O.ReadFrom('{\"a\":[{\"p\":5}]}');
         X.ReadFrom('\"old\"'); L.Add(X);
         ClearLastError();
         B := O.SelectTokens('$.a[', L); S += Format(B) + R(L) + GetLastErrorText() + '|' + GetLastErrorCode() + '|';
         ClearLastError();
         B := O.SelectTokens('$.a[?(@.p >)]', L); S += Format(B) + Format(L.Count()) + GetLastErrorCode();
         exit(S);",
    );
    assert_eq!(
        out,
        "No1:\"old\"@;The requested operation cannot be performed in this context.|InvalidOperation|No1InvalidOperation"
    );
}

#[test]
fn an_unpinned_filter_is_refused() {
    let src = "codeunit 50100 C { procedure F(): Boolean var A: JsonArray; L: List of [JsonToken]; begin
        A.ReadFrom('[{\"p\":5}]'); exit(A.SelectTokens('$[?(@.p < 10)]', L)); end; }";
    let r = al_testkit::run_in(src, "C", "F");
    assert!(format!("{r:?}").contains("not pinned"), "{r:?}");
}
