//! `JsonObject.Remove` of a missing key in statement form raises nothing and leaves the object as it was
//! (json/JsonObjectBasics.al:398 QV_RemoveMissingKey_StatementForm; AL Runner issue 5680).

#[test]
fn remove_of_a_missing_key_as_a_statement_changes_nothing() {
    let src = "codeunit 50100 C { procedure F(): Text var O: JsonObject; T: Text; begin O.Add('a', 1); O.Remove('missing'); O.WriteTo(T); exit(T); end; }";
    assert_eq!(al_testkit::text(al_testkit::run_in(src, "C", "F")), "{\"a\":1}");
}

/// The SelectToken path forms json/JsonObjectBasics.al:293-382 pins (PathAndSelect, SelectTokenErrors): the tests before
/// QV_RemoveMissingKey_StatementForm in its codeunit.
#[test]
fn select_token_path_forms_with_the_result_used() {
    let src = "codeunit 50100 C { procedure F(): Text var O: JsonObject; Inner: JsonObject; Arr: JsonArray; T: JsonToken; S: Text; begin
        Inner.Add('name', 'n1'); Arr.Add(Inner); Arr.Add(10); O.Add('arr', Arr); O.Add('k', 'v');
        S := Format(O.SelectToken('arr[-1]', T)) + Format(O.SelectToken('arr[*]', T)) + Format(O.SelectToken('', T))
            + Format(O.SelectToken('$', T)) + Format(O.SelectToken('[''k'']', T)) + Format(O.SelectToken('arr[', T));
        S += Format(O.SelectToken('$..name', T)) + T.AsValue().AsText();
        exit(S); end; }";
    assert_eq!(
        al_testkit::text(al_testkit::run_in(src, "C", "F")),
        "NoNoYesYesYesNoYesn1"
    );
}

#[test]
fn select_token_path_forms_in_statement_form() {
    let run = |stmt: &str| {
        let src = format!(
            "codeunit 50100 C {{ procedure F(): Text var O: JsonObject; Arr: JsonArray; T: JsonToken; begin
            Arr.Add(1); Arr.Add(2); O.Add('a', Arr); {stmt} exit(Format(T.AsValue().AsInteger())); end; }}"
        );
        al_testkit::run_in(&src, "C", "F")
    };
    assert_eq!(al_testkit::text(run("O.SelectToken('a[0:1]', T);")), "1");
    let wild = al_testkit::error_message(run("O.SelectToken('a[*]', T);"));
    assert_eq!(
        wild,
        "The set of tokens matching the JSON path 'a[*]' contains more than one element."
    );
    for bad in ["a[x]", "a[("] {
        let m = al_testkit::error_message(run(&format!("O.SelectToken('{bad}', T);")));
        assert_eq!(
            m, "The requested operation cannot be performed in this context.",
            "{bad}"
        );
    }
}
