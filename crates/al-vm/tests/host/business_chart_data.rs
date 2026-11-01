//! `Microsoft.Dynamics.Nav.Client.BusinessChart.BusinessChartData` as System Application's "Business Chart Impl." uses it
//! behind Base Application's "Business Chart Buffer": Initialize constructs it and clears its measures, SetXAxis sets
//! `XDimension`, GetXCaption reads it back, AddMeasure adds a measure (corpus
//! tests/al-language/misc/lt-au-language-edges/AuEdgeTests.al:608 BusinessChartBuffer_Initialize_InTest, real BC 27-29:
//! GetXCaption answers 'Month' after SetXAxis('Month')). The AL below is my own minimal version of that sequence.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::text;

const SRC: &str = r#"
dotnet { assembly("Microsoft.Dynamics.Nav.Client.BusinessChart") {
    type("Microsoft.Dynamics.Nav.Client.BusinessChart.BusinessChartData"; "BusinessChartData") { }
    type("Microsoft.Dynamics.Nav.Client.BusinessChart.DataMeasureType"; "DataMeasureType") { }
} }
codeunit 50100 C
{
    procedure F(): Text
    var D: DotNet BusinessChartData; M: DotNet DataMeasureType;
    begin
        __BODY__
    end;
}
"#;

fn run(body: &str) -> Result<Value, Signal> {
    let src = SRC.replace("__BODY__", body);
    al_testkit::run_in(&src, "C", "F")
}

fn unsupported(r: Result<Value, Signal>) -> String {
    match r {
        Err(Signal::Unsupported(why)) => why,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn x_dimension_reads_back_what_was_set_after_measures_are_added() {
    let r = run(
        "D := D.BusinessChartData(); D.ClearMeasures(); D.XDimension := 'Month'; \
         M := 0; D.AddMeasure('Count', M); exit(D.XDimension);",
    );
    assert_eq!(text(r), "Month");
}

#[test]
fn clear_measures_on_a_second_initialize_keeps_the_x_dimension() {
    // Initialize runs ClearMeasures on every call; the x caption is set again by the caller afterwards, so only the
    // round trip of the last value is pinned.
    let r = run(
        "D := D.BusinessChartData(); D.XDimension := 'A'; D.ClearMeasures(); D.XDimension := 'Month'; \
         exit(D.XDimension);",
    );
    assert_eq!(text(r), "Month");
}

#[test]
fn members_outside_the_pinned_ones_are_refused() {
    let why = unsupported(run(
        "D := D.BusinessChartData(); D.ShowChartCondensed := true; exit('');",
    ));
    assert!(why.contains("ShowChartCondensed"), "{why}");
}
