//! JsonObject.Add with a key that exists, and JsonArray.Insert, Set and RemoveAt with an index outside the array, with the
//! return value used: false, no error, nothing changed (AL Runner issue 6924; pinned by json/json/JsonObjectUsedReturn.al and
//! JsonArrayTokenUsedReturn.al).
//! As a statement they raise (json/JsonObjectBasics.al:35-55, json/JsonArrayBasics.al:96-99, :155-158, :186-195).

fn run(body: &str) -> Result<al_builtins::value::Value, al_builtins::native::Signal> {
    let src = format!(
        "codeunit 50100 C {{
            procedure F(): Text var O: JsonObject; A: JsonArray; Ok: Boolean; S: Text; T: Text; begin
                O.Add('a', 1); A.Add(1); A.Add(2); Ok := true; {body}
                O.WriteTo(T); S += '|' + T; A.WriteTo(T); exit(S + '|' + T + '|' + GetLastErrorText()); end;
            local procedure AddDup(var O: JsonObject): Boolean begin exit(O.Add('a', 2)); end;
            local procedure Take(B: Boolean): Text begin if B then exit('true'); exit('false'); end;
        }}"
    );
    al_testkit::run_in(&src, "C", "F")
}

const UNCHANGED: &str = "|{\"a\":1}|[1,2]|";

#[test]
fn add_of_a_key_that_exists_answers_false_when_the_result_is_used() {
    // The reporter's shape: exit(JsonObj.Add(Key, Value)).
    assert_eq!(
        al_testkit::text(run("S := Format(AddDup(O));")),
        format!("No{UNCHANGED}")
    );
    assert_eq!(
        al_testkit::text(run("Ok := O.Add('a', 2); S := Format(Ok);")),
        format!("No{UNCHANGED}")
    );
    assert_eq!(
        al_testkit::text(run("if not O.Add('a', 2) then S := 'then' else S := 'else';")),
        format!("then{UNCHANGED}")
    );
    assert_eq!(
        al_testkit::text(run("S := Take(O.Add('a', 2));")),
        format!("false{UNCHANGED}")
    );
}

#[test]
fn add_of_a_key_that_exists_still_raises_as_a_statement() {
    // json/JsonObjectBasics.al:35-36, :51-52
    assert_eq!(
        al_testkit::error_message(run("O.Add('a', 2);")),
        "The JSON object already contains the key 'a'."
    );
}

#[test]
fn array_calls_outside_the_array_answer_false_when_the_result_is_used() {
    for call in [
        "A.Insert(5, 9)",
        "A.Insert(-1, 9)",
        "A.Set(2, 9)",
        "A.Set(-1, 9)",
        "A.RemoveAt(2)",
        "A.RemoveAt(-1)",
    ] {
        assert_eq!(
            al_testkit::text(run(&format!("Ok := {call}; S := Format(Ok);"))),
            format!("No{UNCHANGED}"),
            "{call}"
        );
        // json/JsonArrayBasics.al:96-99, :155-158, :186-195
        assert_eq!(
            al_testkit::error_message(run(&format!("{call};"))),
            "Index out of bounds.",
            "{call}"
        );
    }
}

#[test]
fn insert_and_set_inside_the_array_answer_true_when_the_result_is_used() {
    assert_eq!(
        al_testkit::text(run("S := Format(A.Insert(2, 3)) + Format(A.Set(0, 0));")),
        "YesYes|{\"a\":1}|[0,2,3]|"
    );
}
