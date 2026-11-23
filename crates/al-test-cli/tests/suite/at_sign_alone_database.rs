//! `@` alone and `<>@` alone answer as BC reads the rows (al-builtins `filter_at_sign`, assumption
//! `at-sign-alone-database-blank`): BQ Tests 9b and 9c of open corpus pull request 1495
//! (tests/al-language/pages/background-tasks/BgTaskQueryCache.al), its record/filters/AtSignAloneReads.al, and the neighbors. The
//! AL is the neighbor app's (tests/neighbors/at-sign-alone-database), one copy. Delete this file when the pin is in the corpus.

use al_builtins::assumptions::Assumption;
use al_test_cli::report::Outcome;

const PROBE: &str = include_str!("../../../../tests/neighbors/at-sign-alone-database/src/Probe.al");
const ROUTES: &str = include_str!("../../../../tests/neighbors/at-sign-alone-database/src/Routes.al");
const NEIGHBORS: &str = include_str!("../../../../tests/neighbors/at-sign-alone-database/src/Neighbors.al");
const CRM: &str = include_str!("../../../../tests/neighbors/at-sign-alone-database/src/Crm.al");
const METHODS: &str = include_str!("../../../../tests/neighbors/at-sign-alone-database/src/Methods.al");

fn outcomes(src: &str, codeunit: &str) -> Vec<(String, Outcome, Vec<Assumption>)> {
    let src = format!("{src}\n{}", include_str!("../fixtures/outside-sc/Assert.al"));
    let l = al_testkit::compile_apps(vec![al_testkit::app(&src)]);
    let engine = al_vm::Engine::new(l.program.clone(), al_testkit::blocked(&l));
    let oi = l
        .program
        .objects
        .iter()
        .position(|o| o.name == codeunit)
        .expect("the codeunit");
    let mut hits = Vec::new();
    let outcomes = al_testrun::runtime::run_codeunit_full(
        &engine,
        &l,
        oi,
        None,
        al_testrun::isolation::TestIsolation::Codeunit,
        None,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut hits,
        None,
    );
    assert!(!outcomes.is_empty());
    outcomes
        .into_iter()
        .map(|(n, o)| {
            let marks = hits
                .iter()
                .find(|(h, _)| *h == n)
                .map(|(_, m)| m.clone())
                .unwrap_or_default();
            (n, o, marks)
        })
        .collect()
}

/// The cells of BQ Tests 9b (corpus PR 1495, BgTaskQueryCache.al:5637-5831): @ alone keeps the empty values when the database
/// reads the rows, whatever the field's type (a FlowFilter of either type applied by CalcFields to a source field of either
/// type :5753, :5767; the source's own fields :5780; a lookup FlowField of a host in the database :5809), and every value of a
/// temporary record (:5780, :5809). All flagged.
#[test]
fn at_sign_alone_bq9b_cells() {
    let o = outcomes(PROBE, "BQ Tests 9b");
    assert_eq!(o.len(), 4);
    for (name, r, marks) in &o {
        assert_eq!(*r, Outcome::Pass, "{name}");
        assert!(
            marks.contains(&Assumption::AtSignAloneDatabaseBlank),
            "{name}: {marks:?}"
        );
    }
}

/// The cells of BQ Tests 9c (corpus PR 1495, BgTaskQueryCache.al:6291-6433): a query reads the database, so @ alone on a Code
/// and a Text column, by SetFilter and by a DataItemTableFilter, keeps the empty values. All flagged.
#[test]
fn at_sign_alone_bq9c_cells() {
    let o = outcomes(PROBE, "BQ Tests 9c");
    assert_eq!(o.len(), 4);
    for (name, r, marks) in &o {
        assert_eq!(*r, Outcome::Pass, "{name}");
        assert!(
            marks.contains(&Assumption::AtSignAloneDatabaseBlank),
            "{name}: {marks:?}"
        );
    }
}

/// `<>@` alone on a Text field of a record in the database keeps the rows whose value is not empty (AtSignAloneReads.al:698), and so
/// does `<>@` alone on a Code field set by Record.SetFilter (:698). The places no cell measures `@` alone on a Code field from keep
/// main's refusal: a report's DataItemTableView and a query column's ColumnFilter.
#[test]
fn at_sign_alone_neighbors() {
    let src = format!("{PROBE}\n{NEIGHBORS}");
    let o = outcomes(&src, "BQ At Neighbor Tests");
    let outcome = |name: &str| {
        &o.iter()
            .find(|(n, ..)| n == name)
            .unwrap_or_else(|| panic!("{name}: {o:?}"))
            .1
    };
    let flags = |name: &str| &o.iter().find(|(n, ..)| n == name).expect(name).2;
    for name in [
        "AtSignAloneNegated_TextField_SourceRows",
        "AtSignAloneNegated_CodeField_SourceRows",
    ] {
        assert_eq!(*outcome(name), Outcome::Pass, "{name}");
        assert!(flags(name).contains(&Assumption::AtSignAloneDatabaseBlank), "{name}");
    }
    let not_measured = "`@` alone on a Code field from a place no cell measures";
    for (name, origin) in [
        ("AtSignAlone_ReportDataItemTableView_CodeField", "SetView: "),
        ("AtSignAlone_QueryColumnFilter_CodeColumn", "query filter: "),
    ] {
        assert!(
            matches!(outcome(name), Outcome::NotRun(w) if w.starts_with(&format!("{origin}{not_measured}"))),
            "{name}: {:?}",
            outcome(name)
        );
    }
}

/// `<>@` alone on a Code field from a query's SetFilter and DataItemTableFilter and as a FlowFilter CalcFields applies through
/// field(): the cells (AtSignAloneReads.al:861, :871, :928) keep the K row and sum 10, and so do these neighbors, flagged.
#[test]
fn not_at_sign_alone_code_neighbors() {
    let src = format!("{PROBE}\n{NEIGHBORS}");
    let o = outcomes(&src, "BQ Not At Neighbor Tests");
    assert_eq!(o.len(), 3);
    for (name, r, marks) in &o {
        assert_eq!(*r, Outcome::Pass, "{name}");
        assert!(
            marks.contains(&Assumption::AtSignAloneDatabaseBlank),
            "{name}: {marks:?}"
        );
    }
}

/// The cells of AtSignAloneReads.al (corpus PR 1495): every read method and place a filter comes from, `<>@` alone on a Code field
/// (:698, :730, :792, :861, :871, :904, :928), the CalcFormula's quoted `filter('@')` and `filter('<>@')` (:579, :752) and the
/// read-back of both (:823) included, flagged.
#[test]
fn at_sign_alone_reads_cells() {
    let o = outcomes(ROUTES, "AS Kind Tests");
    assert_eq!(o.len(), 25);
    for (name, r, marks) in &o {
        assert_eq!(*r, Outcome::Pass, "{name}");
        assert!(
            marks.contains(&Assumption::AtSignAloneDatabaseBlank),
            "{name}: {marks:?}"
        );
    }
}

/// The cells of AtSignAloneMethods.al (corpus PR 1495): `<>@` alone read by DeleteAll, ModifyAll, CalcSums, FindLast, a report and
/// a page (:215, :244, :279, :312, :345), and a CalcFormula's `filter('@')` and `filter('<>@')` with every method (:376), flagged.
/// Over a Date source field (:395) and an Integer, Decimal, Option or Boolean one (:430) BC answers otherwise; those keep main's
/// refusal here.
#[test]
fn at_sign_alone_methods_cells() {
    let o = outcomes(METHODS, "AM Kind Tests");
    assert_eq!(o.len(), 8);
    for (name, r, marks) in &o {
        if name.starts_with("AtSignAlone_CalcFormula") && name != "AtSignAlone_CalcFormulaMethods" {
            assert!(
                matches!(r, Outcome::NotRun(w) if w.contains("value expected after @")),
                "{name}: {r:?}"
            );
            continue;
        }
        assert_eq!(*r, Outcome::Pass, "{name}");
        assert!(
            marks.contains(&Assumption::AtSignAloneDatabaseBlank),
            "{name}: {marks:?}"
        );
    }
}

/// A CalcFormula's unquoted `filter(<>@)` does not compile on BC (must-not-compile case 3806: AL0726, AL0104, AL0870), as
/// `filter(@)` does not (case 3805): the lowering refuses both, over a Code and over a Text field, and never reads them as `@` alone
/// or `<>@` alone.
#[test]
fn unquoted_calcformula_at_signs_refused() {
    for (filter, field) in [("<>@", "Kind"), ("<>@", "Note"), ("@", "Kind")] {
        let src = format!(
            r#"table 97590 "UQ Row" {{ fields {{ field(1; "Group"; Code[20]) {{ }} field(2; Kind; Code[20]) {{ }} field(3; Note; Text[50]) {{ }} field(4; Amount; Decimal) {{ }} }} keys {{ key(PK; "Group", Kind) {{ }} }} }}
table 97591 "UQ Host" {{ fields {{ field(1; "No."; Code[20]) {{ }} field(2; Total; Decimal) {{ FieldClass = FlowField; CalcFormula = sum("UQ Row".Amount where("Group" = field("No."), {field} = filter({filter}))); }} }} keys {{ key(PK; "No.") {{ }} }} }}
codeunit 97590 "UQ Tests" {{ Subtype = Test; [Test] procedure Unquoted() var Host: Record "UQ Host"; begin Host."No." := 'G'; Host.CalcFields(Total); end; }}"#
        );
        let o = outcomes(&src, "UQ Tests");
        let [(_, r, _)] = o.as_slice() else { panic!("{o:?}") };
        assert!(
            matches!(r, Outcome::NotRun(w) if w.contains(&format!("filter({filter}) without quotes"))),
            "{filter} on {field}: {r:?}"
        );
    }
}

/// `@` alone on a Code field of a CRM table: the connection's rows are not the database's, and no cell measures them (a local
/// Business Central 29.1 container keeps every row), so the read refuses with the store's refusal of `FilterExpr::AtSignAloneCode`.
#[test]
fn at_sign_alone_crm_table_refused() {
    let o = outcomes(CRM, "AS Crm Neighbor Tests");
    let [(_, r, _)] = o.as_slice() else { panic!("{o:?}") };
    assert!(
        matches!(r, Outcome::NotRun(w) if w.contains("a read through the filter `@` (alone) on a Code field by a reader no cell measures")),
        "{r:?}"
    );
}

/// FilterPageBuilder.SetView with `@` alone on a Code field: no cell measures a filter page's view.
#[test]
fn at_sign_alone_filter_page_view_refused() {
    let src = r#"
table 97590 "AS Fp Row" { fields { field(1; "No."; Code[20]) { } field(2; Kind; Code[20]) { } } keys { key(PK; "No.") { Clustered = true; } } }
codeunit 97591 "AS Fp Tests"
{
    Subtype = Test;
    [Test]
    procedure FilterPageView_AtSignAlone()
    var
        Row: Record "AS Fp Row";
        Builder: FilterPageBuilder;
    begin
        Builder.AddRecord('Rows', Row);
        Builder.SetView('Rows', 'WHERE(Kind=FILTER(@))');
    end;
}
"#;
    let o = outcomes(src, "AS Fp Tests");
    let [(_, r, _)] = o.as_slice() else { panic!("{o:?}") };
    assert!(
        matches!(r, Outcome::NotRun(w) if w.starts_with("FilterPageBuilder.SetView with `@` alone on a Code field")),
        "{r:?}"
    );
}
