//! JsonObject.Add with a key that exists, and JsonArray.Insert, Set and RemoveAt with an index outside the array, with the
//! return value used: false, no error, nothing changed (AL Runner issue 6924; the assumption `json-failed-call-result-used`).
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

#[test]
fn the_used_forms_mark_their_assumption_and_the_statement_forms_do_not() {
    use al_builtins::assumptions::{Assumption, take_hits};
    let mark = Assumption::JsonFailedCallResultUsed;
    for used in [
        "Ok := O.Add('a', 2);",
        "Ok := A.Insert(5, 9);",
        "Ok := A.Set(2, 9);",
        "Ok := A.RemoveAt(2);",
        "Ok := A.Insert(0, 9);",
        "Ok := A.Set(0, 9);",
    ] {
        take_hits();
        run(used).unwrap();
        assert!(take_hits().contains(&mark), "{used}");
    }
    // Pinned: json/JsonObjectBasics.al:23 `add_returns`, json/json/P88ArrayIndex.al:126 `removeat_1`, and every statement form.
    for pinned in [
        "Ok := O.Add('b', 2);",
        "Ok := A.RemoveAt(0);",
        "A.Insert(0, 9);",
        "A.Set(0, 9);",
        "A.RemoveAt(0);",
        "O.Add('b', 2);",
    ] {
        take_hits();
        run(pinned).unwrap();
        assert!(!take_hits().contains(&mark), "{pinned}");
    }
    take_hits();
    assert!(run("O.Add('a', 2);").is_err());
    assert!(!take_hits().contains(&mark));
}
