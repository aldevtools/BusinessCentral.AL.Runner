//! The parameters text `RunRequestPage` returns for request page fields that are not bound to a plain global variable
//! (al-runner issue 6960). A binding the text is not pinned for is refused, never left out of the text silently.

use al_test_cli::report::Outcome;

const OBJECTS: &str = r#"
table 61410 "RB Row"
{
    fields { field(1; Id; Integer) { } field(2; Name; Code[10]) { } }
    keys { key(PK; Id) { } }
}
report 61411 "RB Arr"
{
    ProcessingOnly = true;
    dataset { dataitem(Row; "RB Row") { } }
    requestpage
    {
        layout { area(Content) { field(PlainCtl; PlainVal) { ApplicationArea = All; } field(ArrCtl; BufArr[1].Name) { ApplicationArea = All; } } }
    }
    var PlainVal: Code[10]; BufArr: array[2] of Record "RB Row";
}
report 61412 "RB Src"
{
    ProcessingOnly = true;
    dataset { dataitem(Row; "RB Row") { } }
    requestpage
    {
        SourceTable = "RB Row";
        layout { area(Content) { field(PlainCtl; PlainVal) { ApplicationArea = All; } field(SrcCtl; Rec.Name) { ApplicationArea = All; } } }
    }
    var PlainVal: Code[10];
}
"#;

fn outcomes(tests: &str) -> Vec<(String, Outcome)> {
    let src = format!("{OBJECTS}\ncodeunit 61419 T {{ Subtype = Test; {tests} }}");
    al_testkit::outcomes_in(&src, "T")
}

// A field bound to a field of an element of an array of records: what BC writes for it is not pinned yet.
#[test]
fn a_field_bound_to_an_array_element_field_is_refused_not_left_out() {
    let o = outcomes(
        "[Test] [HandlerFunctions('H')]
        procedure Params() var T: Text; begin T := Report.RunRequestPage(Report::\"RB Arr\"); end;
        [RequestPageHandler] procedure H(var RP: TestRequestPage \"RB Arr\") begin RP.ArrCtl.SetValue('B1'); RP.OK().Invoke(); end;",
    );
    match &o[..] {
        [(_, Outcome::NotRun(r))] => assert!(r.contains("ArrCtl"), "{r}"),
        other => panic!("expected one refusal, got {other:?}"),
    }
}

// reports/TestReportRequestPageSourceTableCompiledContracts.al:221: a field of the request page's SourceTable is not in
// the text.
#[test]
fn a_field_of_the_source_table_is_still_left_out() {
    let o = outcomes(
        "[Test] [HandlerFunctions('H')]
        procedure Params() var T: Text; begin
            T := Report.RunRequestPage(Report::\"RB Src\");
            if T <> '<?xml version=\"1.0\" standalone=\"yes\"?><ReportParameters name=\"RB Src\" id=\"61412\"><Options><Field name=\"PlainVal\">P</Field></Options><DataItems><DataItem name=\"Row\">VERSION(1) SORTING(Field1)</DataItem></DataItems></ReportParameters>' then Error(T); end;
        [RequestPageHandler] procedure H(var RP: TestRequestPage \"RB Src\") begin RP.PlainCtl.SetValue('P'); RP.OK().Invoke(); end;",
    );
    for (n, r) in &o {
        assert_eq!(r, &Outcome::Pass, "{n}");
    }
}
