//! `Dictionary.Set`, `List.Set` with a `var OldValue` and `List.GetRange` with a `var Result`
//! (collections/P107Collections.al).

use al_builtins::native::Signal;

fn code(body: &str) -> String {
    format!("codeunit 50100 C {{ {body} }}")
}

fn read(src: &str) -> String {
    al_testkit::text(al_testkit::run_with(src, "C", "F", vec![]).map(|(v, _)| v))
}

#[test]
fn dictionary_set_with_old_value_hands_back_the_replaced_value() {
    // P107Collections.al:14-39 (P107_DictionarySetOld).
    let src = code(
        "procedure F(): Text var D: Dictionary of [Text, Integer]; Old: Integer; R: Text; begin Old := 99; \
         D.Set('a', 1, Old); R := Format(Old) + '|' + Format(D.Count()); D.Set('a', 2, Old); \
         R += '|' + Format(Old) + '|' + Format(D.Get('a')); exit(R); end;",
    );
    assert_eq!(read(&src), "99|1|1|2");
}

#[test]
fn list_set_with_old_value_hands_back_the_replaced_value_and_raises_outside_the_list() {
    // P107Collections.al:138-176 (P107_ListSetOld).
    let src = code(
        "procedure F(): Text var L: List of [Text]; Old: Text; R: Text; begin L.Add('a'); L.Add('b'); Old := 'u'; \
         L.Set(2, 'B', Old); R := Old + L.Get(2); Old := 'keep'; asserterror L.Set(3, 'z', Old); \
         R += '|' + Old + '|' + GetLastErrorCode(); exit(R); end;",
    );
    assert_eq!(read(&src), "bB|keep|ArgumentOutOfRange");
}

#[test]
fn list_get_range_into_a_var_result_replaces_its_content() {
    // P107Collections.al:67-113 (P107_ListGetRangeVar), :115-136 (P107_ListGetRangeVarSelf).
    let src = code(
        "procedure F(): Text var L: List of [Integer]; R: List of [Integer]; i: Integer; T: Text; begin \
         for i := 1 to 5 do L.Add(i * 10); R.Add(777); L.GetRange(2, 3, R); T := Format(R.Count()) + Format(R.Get(1)); \
         asserterror L.GetRange(4, 3, R); T += '|' + GetLastErrorCode() + Format(R.Count()); \
         L.GetRange(2, 2, L); T += '|' + Format(L.Count()) + Format(L.Get(1)); exit(T); end;",
    );
    assert_eq!(read(&src), "320|Argument3|220");
}

#[test]
fn the_unpinned_forms_are_refused() {
    // The Boolean results, and a range of no items that starts past the end, are not pinned.
    for body in [
        "var L: List of [Integer]; Old: Integer; B: Boolean; begin L.Add(1); B := L.Set(1, 2, Old); end;",
        "var D: Dictionary of [Integer, Integer]; Old: Integer; B: Boolean; begin B := D.Set(1, 2, Old); end;",
        "var L: List of [Integer]; R: List of [Integer]; begin L.Add(1); L.GetRange(2, 0, R); end;",
    ] {
        let src = code(&format!("procedure F() {body}"));
        match al_testkit::run_with(&src, "C", "F", vec![]) {
            Err(Signal::Unsupported(_)) => {}
            other => panic!("{body}: {other:?}"),
        }
    }
}
