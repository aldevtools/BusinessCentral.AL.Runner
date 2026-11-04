//! The parameters text `RunRequestPage` returns for request page fields that are not bound to a plain global variable
//! (al-runner issue 6960), pinned by reports/request-page/R6960RequestParamBindings.al (corpus branch
//! al-runtime-rs-staging-r6960). A binding the text is not pinned for is refused, never left out of the text silently.

use al_test_cli::report::Outcome;

const OBJECTS: &str = r#"
table 61410 "RB Row"
{
    fields { field(1; Id; Integer) { } field(2; Name; Code[10]) { } }
    keys { key(PK; Id) { } }
}
codeunit 61413 "RB Log"
{
    SingleInstance = true;
    var Entries: Text;
    procedure Add(E: Text) begin Entries += E + ';'; end;
    procedure Take(): Text var R: Text; begin R := Entries; Entries := ''; exit(R); end;
}
report 61411 "RB Arr"
{
    ProcessingOnly = true;
    dataset
    {
        dataitem(Row; "RB Row")
        {
            trigger OnPreDataItem() begin L.Add('pre arr1=' + BufArr[1].Name + ' txt2=' + TxtArr[2] + ' item=' + Row.Name); CurrReport.Break(); end;
        }
    }
    requestpage
    {
        layout
        {
            area(Content)
            {
                field(PlainCtl; PlainVal) { ApplicationArea = All; }
                field(ArrCtl; BufArr[1].Name) { ApplicationArea = All; }
                field(RecCtl; Buf.Name) { ApplicationArea = All; }
                field(TxtCtl; TxtArr[2]) { ApplicationArea = All; }
                field(ItemCtl; Row.Name) { ApplicationArea = All; }
            }
        }
    }
    var PlainVal: Code[10]; BufArr: array[2] of Record "RB Row"; Buf: Record "RB Row"; TxtArr: array[3] of Text[20]; L: Codeunit "RB Log";
}
report 61414 "RB Call"
{
    ProcessingOnly = true;
    dataset { dataitem(Row; "RB Row") { } }
    requestpage { layout { area(Content) { field(CallCtl; Format(PlainVal)) { ApplicationArea = All; } } } }
    var PlainVal: Code[10];
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
    let src =
        format!("{OBJECTS}\ncodeunit 61419 T {{ Subtype = Test; var L: Codeunit \"RB Log\"; Mode: Integer; {tests} }}");
    al_testkit::outcomes_in(&src, "T")
}

fn passes(tests: &str) {
    let o = outcomes(tests);
    assert!(!o.is_empty());
    for (n, r) in &o {
        assert_eq!(r, &Outcome::Pass, "{n}");
    }
}

// reports/request-page/R6960RequestParamBindings.al Rrp_TypedValues_Text (corpus branch al-runtime-rs-staging-r6960): fields bound
// to a field of a record variable, of an array element, of the data item's record, and to an element of a plain array are in the
// text, named by their source as written.
#[test]
fn fields_bound_to_places_written_as_expressions_are_named_by_their_source() {
    passes(
        "[Test] [HandlerFunctions('H')]
        procedure Params() var T: Text; begin
            T := Report.RunRequestPage(Report::\"RB Arr\");
            if T <> '<?xml version=\"1.0\" standalone=\"yes\"?><ReportParameters name=\"RB Arr\" id=\"61411\"><Options><Field name=\"PlainVal\">P</Field><Field name=\"BufArr[1].Name\">B1</Field><Field name=\"Buf.Name\" /><Field name=\"TxtArr[2]\">T2</Field><Field name=\"Row.Name\">I1</Field></Options><DataItems><DataItem name=\"Row\">VERSION(1) SORTING(Field1)</DataItem></DataItems></ReportParameters>' then Error(T); end;
        [RequestPageHandler] procedure H(var RP: TestRequestPage \"RB Arr\") begin
            RP.PlainCtl.SetValue('P'); RP.ArrCtl.SetValue('B1'); RP.TxtCtl.SetValue('T2'); RP.ItemCtl.SetValue('I1'); RP.OK().Invoke(); end;",
    );
}

// R6960RequestParamBindings.al Rrp_TypedValues_ThenExecute and Rrp_TypedValues_ThenSeededRequestPage: Execute and a seeded
// RunRequestPage set each place again.
#[test]
fn execute_and_a_seeded_request_page_set_the_places_again() {
    passes(
        "[Test] [HandlerFunctions('H')]
        procedure RoundTrip() var T: Text; begin
            Mode := 0; T := Report.RunRequestPage(Report::\"RB Arr\");
            Report.Execute(Report::\"RB Arr\", T);
            if L.Take() <> 'pre arr1=B1 txt2=T2 item=I1;' then Error('execute');
            Mode := 1; Report.RunRequestPage(Report::\"RB Arr\", T);
            if L.Take() <> 'h arr1=B1 txt2=T2 item=I1;' then Error('seeded'); end;
        [RequestPageHandler] procedure H(var RP: TestRequestPage \"RB Arr\") begin
            if Mode = 1 then L.Add('h arr1=' + RP.ArrCtl.Value() + ' txt2=' + RP.TxtCtl.Value() + ' item=' + RP.ItemCtl.Value())
            else begin RP.ArrCtl.SetValue('B1'); RP.TxtCtl.SetValue('T2'); RP.ItemCtl.SetValue('I1'); end;
            RP.OK().Invoke(); end;",
    );
}

// A call is not a place the probe covers: refused, not left out.
#[test]
fn a_field_bound_to_a_call_is_refused() {
    let o = outcomes(
        "[Test] [HandlerFunctions('H')]
        procedure Params() var T: Text; begin T := Report.RunRequestPage(Report::\"RB Call\"); end;
        [RequestPageHandler] procedure H(var RP: TestRequestPage \"RB Call\") begin RP.OK().Invoke(); end;",
    );
    match &o[..] {
        [(_, Outcome::NotRun(r))] => assert!(r.contains("CallCtl"), "{r}"),
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
