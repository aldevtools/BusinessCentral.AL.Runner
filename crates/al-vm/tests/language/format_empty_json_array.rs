//! Format of a default or cleared JsonArray is `[]` (al-runner issue 5956; misc/lt-probes-needed-5/PnE09Platform.al:163,172,
//! types/ProbeTypeFormsAndFilters.al:Clear_JsonTypes_GiveFreshUnassigned).

#[test]
fn a_default_and_a_cleared_json_array_format_as_brackets() {
    let src = "codeunit 50600 C { procedure F(): Text var A: JsonArray; B: JsonArray; begin
        B.Add(1); Clear(B); exit(Format(A) + '|' + Format(B)); end; }";
    assert_eq!(al_testkit::text(al_testkit::run_in(src, "C", "F")), "[]|[]");
}

/// Format(A, 0, 9) of a JsonArray nobody touched raises InvalidStandardFormat (json/JsonWriteTo.al FormatOfEmptyJsonArrays).
#[test]
fn standard_format_9_of_a_default_json_array_raises() {
    let src = "codeunit 50600 C { procedure F(): Text var A: JsonArray; begin
        asserterror Format(A, 0, 9); exit(GetLastErrorText() + '|' + GetLastErrorCode()); end; }";
    assert_eq!(
        al_testkit::text(al_testkit::run_in(src, "C", "F")),
        "Standard format number 9 does not exist for the type 'JsonArray'.|InvalidStandardFormat"
    );
}
