DrilldownMarkedRecOutside => {
    id: "drilldown-marked-rec-outside",
    assumed: "After a marking OnDrillDown that turned MarkedOnly on, called CurrPage.Update(false) and left Rec on the last row of the view, which is not marked, on a list with a part: the refill settles on the nearest marked row before Rec's row (in a key other than the primary key the first read is Rec's row); GoToRecord answers true for that row only and walks the marked rows from the first for any other row.",
    tests: "No Microsoft test: al-runner issue 6946 (a partner app's test, the reporter's repro passed on real Business Central).",
    probe: "pages/onopenpage-moves-rec/DrillDownMarkedRecLeft.al on corpus branch al-runtime-rs-staging-r6945, measured on the corpus CI legs of all three supported majors.",
    awaiting: Awaiting {
        issues: &[6946],
        files: &["tests/al-language/pages/onopenpage-moves-rec/DrillDownMarkedRecLeft.al"],
        since: "2026-11-04",
    },
}
