//! What an asserterror's rollback leaves of a number sequence that existed when the codeunit's tests began and that the
//! test restarted or deleted, under al-runner's own runner with TestIsolation = Codeunit. The cells copy corpus PR 1573
//! (open, "existing number sequence restarted or deleted, then the test fails"), tests/al-language-isolation-probe/src/
//! SqExistingRestart.al, measured on BC 29.0 under runners 130450 and 61301: after a Record write the asserterror undoes
//! the step at once, also in a test that passes (RE :892, RE2 :976, DE :2487, RE5 :1224); with nothing reached since the
//! step it keeps it (RE3 :1059, RE4 :1142). Until a corpus bump pins them, a test that reads such a sequence is a flagged
//! pass; every other shape refuses the later calls on the sequence.

use super::runner_isolation::{not_run, results_on};
use al_testrun::Outcome;
use al_testrun::isolation::TestIsolation;

const ID: &str = "existing-sequence-asserterror";

type Row = (u32, String, Outcome, Vec<&'static str>);

fn row<'a>(o: &'a [Row], test: &str) -> &'a Row {
    o.iter()
        .find(|r| r.1 == test)
        .unwrap_or_else(|| panic!("no {test} in {o:?}"))
}

/// `test` failed with `seen` and passed through `ID`.
fn flagged(o: &[Row], test: &str, seen: &str) {
    let r = row(o, test);
    assert_eq!(r.2, Outcome::Fail(seen.into()), "{test}: {o:?}");
    assert!(r.3.contains(&ID), "{test} is not flagged {ID}: {r:?}");
}

/// The fixture of each cell: OnRun makes the sequence when it is missing (Insert(Name, 1, 1), Next twice: Current 2),
/// test A writes a row and commits, as Initialize does (SqExistingRestart.al:119-131).
fn cell(id: u32, name: &str, b: &str) -> String {
    cell_with(id, name, &onrun(name), "", b, true)
}

/// A cell with its own OnRun, attributes of test B (`b_attr`) and test C reading a company sequence (`company`) or the
/// CompanyWide one.
fn cell_with(id: u32, name: &str, onrun: &str, b_attr: &str, b: &str, company: bool) -> String {
    format!(
        r#"
codeunit {id} "P5 {name}"
{{
    Subtype = Test;
    var Rows: Codeunit "Rows"; InnerErr: Label 'inner';
    trigger OnRun() begin {onrun} end;
    [Test] procedure A_{name}WritesCommits() begin Rows.Put('{name}-INIT'); Commit(); end;
    [Test] {b_attr} procedure B_{name}Step() begin {b} end;
    [Test] procedure C_{name}Reads() begin if not NumberSequence.Exists('P5{name}', {company}) then Error('OBS missing row=%1', Rows.Has('{name}-B1')); Error('OBS cur=%1 row=%2', NumberSequence.Current('P5{name}', {company}), Rows.Has('{name}-B1')); end;
}}
"#
    )
}

/// The OnRun of [`cell`] for `name`.
fn onrun(name: &str) -> String {
    format!(
        "if not NumberSequence.Exists('P5{name}') then begin NumberSequence.Insert('P5{name}', 1, 1); \
         NumberSequence.Next('P5{name}'); NumberSequence.Next('P5{name}'); end;"
    )
}

fn cells(isolation: TestIsolation) -> Vec<Row> {
    cells_on(isolation, None)
}

/// [`cells`] on the BC version `target`.
fn cells_on(isolation: TestIsolation, target: Option<&al_version::Target>) -> Vec<Row> {
    let src = [
        // RE5: Restart, a Record insert, asserterror; the test passes.
        cell(50760, "RE5", "NumberSequence.Restart('P5RE5', 1002); if NumberSequence.Exists('P5RE5') then if NumberSequence.Current('P5RE5') = 0 then; Rows.Put('RE5-B1'); asserterror Error('inner');"),
        // RE4: Restart, asserterror with nothing reached; the test passes.
        cell(50761, "RE4", "NumberSequence.Restart('P5RE4', 1002); if NumberSequence.Exists('P5RE4') then if NumberSequence.Current('P5RE4') = 0 then; asserterror Error('inner');"),
        // RE: as RE5, then the test fails.
        cell(50762, "RE", "NumberSequence.Restart('P5RE', 1002); if NumberSequence.Exists('P5RE') then if NumberSequence.Current('P5RE') = 0 then; Rows.Put('RE-B1'); asserterror Error('inner'); Error('B deliberate');"),
        // RE2: Restart, asserterror of a Record insert that raises, then the test fails.
        cell(50763, "RE2", "NumberSequence.Restart('P5RE2', 1002); if NumberSequence.Exists('P5RE2') then if NumberSequence.Current('P5RE2') = 0 then; asserterror begin Rows.Put('RE2-B1'); Error('inner'); end; Error('B deliberate');"),
        // DE: Delete, a Record insert, asserterror, then the test fails.
        cell(50764, "DE", "NumberSequence.Delete('P5DE'); if NumberSequence.Exists('P5DE') then; Rows.Put('DE-B1'); asserterror Error('inner'); Error('B deliberate');"),
        // RE3: Restart, asserterror with nothing reached, then the test fails at once: the failed test's rollback keeps
        // the step the asserterror kept.
        cell(50765, "RE3", "NumberSequence.Restart('P5RE3', 1002); if NumberSequence.Exists('P5RE3') then if NumberSequence.Current('P5RE3') = 0 then; asserterror Error('inner'); Error('B deliberate');"),
        // Beside the cells: a number taken after the step.
        cell(50766, "NX", "NumberSequence.Restart('P5NX', 1002); if NumberSequence.Exists('P5NX') then if NumberSequence.Current('P5NX') = 0 then; Rows.Put('NX-B1'); NumberSequence.Next('P5NX'); asserterror Error('inner');"),
        // A Record write before the step.
        cell(50767, "WB", "Rows.Put('WB-B1'); NumberSequence.Restart('P5WB', 1002); if NumberSequence.Exists('P5WB') then if NumberSequence.Current('P5WB') = 0 then; asserterror Error('inner');"),
        // A CompanyWide sequence: as RE5, with the sequence made CompanyWide in OnRun.
        cell_with(
            50768,
            "CW",
            "if not NumberSequence.Exists('P5CW', false) then begin NumberSequence.Insert('P5CW', 1, 1, false); \
             NumberSequence.Next('P5CW', false); NumberSequence.Next('P5CW', false); end;",
            "",
            "NumberSequence.Restart('P5CW', 1002, false); if NumberSequence.Exists('P5CW', false) then if NumberSequence.Current('P5CW', false) = 0 then; Rows.Put('CW-B1'); asserterror Error('inner');",
            false,
        ),
        // A Record read and no write before the asserterror.
        cell(50769, "RD", "NumberSequence.Restart('P5RD', 1002); if NumberSequence.Exists('P5RD') then if NumberSequence.Current('P5RD') = 0 then; if Rows.Has('RD-INIT') then; asserterror Error('inner');"),
        // As RE5 in a test with TransactionModel AutoRollback.
        cell_with(50770, "AR", &onrun("AR"), "[TransactionModel(TransactionModel::AutoRollback)]", "NumberSequence.Restart('P5AR', 1002); if NumberSequence.Exists('P5AR') then if NumberSequence.Current('P5AR') = 0 then; Rows.Put('AR-B1'); asserterror Error('inner');", true),
        // As RE5 with an Error of a Label.
        cell(50771, "LB", "NumberSequence.Restart('P5LB', 1002); if NumberSequence.Exists('P5LB') then if NumberSequence.Current('P5LB') = 0 then; Rows.Put('LB-B1'); asserterror Error(InnerErr);"),
        // A failed guarded Codeunit.Run after the step and a write, in place of the asserterror.
        cell(50772, "GR", "NumberSequence.Restart('P5GR', 1002); if NumberSequence.Exists('P5GR') then if NumberSequence.Current('P5GR') = 0 then; Rows.Put('GR-B1'); if Codeunit.Run(Codeunit::\"P5 Fails\") then;"),
        // RE4 (the step kept), then a write and a second asserterror.
        cell(50773, "HK", "NumberSequence.Restart('P5HK', 1002); if NumberSequence.Exists('P5HK') then if NumberSequence.Current('P5HK') = 0 then; asserterror Error('inner'); Rows.Put('HK-B1'); asserterror Error('inner');"),
        // DE in a test that passes.
        cell(50774, "DP", "NumberSequence.Delete('P5DP'); if NumberSequence.Exists('P5DP') then; Rows.Put('DP-B1'); asserterror Error('inner');"),
        // A Delete with nothing reached before the asserterror.
        cell(50775, "DN", "NumberSequence.Delete('P5DN'); if NumberSequence.Exists('P5DN') then; asserterror Error('inner');"),
        // A read of the sequence in the step's test after the asserterror undid the Restart, the Delete, or kept the Restart.
        cell(50776, "RU", "NumberSequence.Restart('P5RU', 1002); if NumberSequence.Exists('P5RU') then if NumberSequence.Current('P5RU') = 0 then; Rows.Put('RU-B1'); asserterror Error('inner'); if NumberSequence.Current('P5RU') = 0 then;"),
        cell(50777, "DU", "NumberSequence.Delete('P5DU'); if NumberSequence.Exists('P5DU') then; Rows.Put('DU-B1'); asserterror Error('inner'); if NumberSequence.Exists('P5DU') then; Error('B deliberate');"),
        cell(50778, "RK", "NumberSequence.Restart('P5RK', 1002); if NumberSequence.Exists('P5RK') then if NumberSequence.Current('P5RK') = 0 then; asserterror Error('inner'); if NumberSequence.Current('P5RK') = 0 then;"),
        // The step with no read of the sequence after it (each cell reads it with Exists and Current).
        cell(50780, "NR", "NumberSequence.Restart('P5NR', 1002); asserterror Error('inner');"),
        // Current alone after the step.
        cell(50781, "CO", "NumberSequence.Restart('P5CO', 1002); if NumberSequence.Current('P5CO') = 0 then; asserterror Error('inner');"),
        // Two sequences restarted in the transaction, the second as the cells (each cell restarts one).
        cell_with(
            50782,
            "TS",
            &format!("{} {}", onrun("TS"), onrun("TS2")),
            "",
            "NumberSequence.Restart('P5TS2', 1002); NumberSequence.Restart('P5TS', 1002); \
             if NumberSequence.Exists('P5TS') then if NumberSequence.Current('P5TS') = 0 then; asserterror Error('inner');",
            true,
        ),
        "codeunit 50779 \"P5 Fails\" { trigger OnRun() begin Error('fails'); end; }".to_string(),
    ]
    .concat();
    results_on(&src, isolation, target)
}

#[test]
fn an_asserterror_after_a_write_undoes_the_restart_of_an_existing_sequence() {
    let o = cells(TestIsolation::Codeunit);
    flagged(&o, "C_RE5Reads", "OBS cur=2 row=No");
    flagged(&o, "C_REReads", "OBS cur=2 row=No");
    flagged(&o, "C_RE2Reads", "OBS cur=2 row=No");
    flagged(&o, "C_DEReads", "OBS cur=2 row=No");
}

#[test]
fn an_asserterror_with_nothing_reached_keeps_the_restart() {
    let o = cells(TestIsolation::Codeunit);
    flagged(&o, "C_RE4Reads", "OBS cur=1002 row=No");
}

#[test]
fn a_failed_tests_rollback_right_after_the_asserterror_keeps_the_kept_restart() {
    let o = cells(TestIsolation::Codeunit);
    flagged(&o, "C_RE3Reads", "OBS cur=1002 row=No");
    let r = row(&o, "C_RE3Reads");
    assert!(r.3.contains(&"sequence-after-failed-test"), "{r:?}");
}

#[test]
fn asserterror_shapes_beside_the_cells_are_refused() {
    let o = cells(TestIsolation::Codeunit);
    for (test, key) in [
        ("C_NXReads", "'P5NX'"),
        ("C_WBReads", "'P5WB'"),
        ("C_CWReads", "'P5CW'"),
        ("C_RDReads", "'P5RD'"),
        ("C_ARReads", "'P5AR'"),
        ("C_LBReads", "'P5LB'"),
        ("C_GRReads", "'P5GR'"),
        ("C_HKReads", "'P5HK'"),
        ("C_DPReads", "'P5DP'"),
        ("C_DNReads", "'P5DN'"),
        ("C_NRReads", "'P5NR'"),
        ("C_COReads", "'P5CO'"),
        ("C_TSReads", "'P5TS'"),
    ] {
        assert!(not_run(&row(&o, test).2, key), "{test}: {o:?}");
    }
}

#[test]
fn under_test_isolation_function_an_asserterror_after_a_restart_is_refused() {
    let o = cells(TestIsolation::Function);
    for (test, key) in [("C_RE5Reads", "'P5RE5'"), ("C_RE4Reads", "'P5RE4'")] {
        assert!(not_run(&row(&o, test).2, key), "{test}: {o:?}");
    }
}

#[test]
fn a_read_in_the_steps_test_after_the_asserterror_is_refused() {
    let o = cells(TestIsolation::Codeunit);
    for (test, key) in [("B_RUStep", "'P5RU'"), ("B_DUStep", "'P5DU'"), ("B_RKStep", "'P5RK'")] {
        assert!(not_run(&row(&o, test).2, key), "{test}: {o:?}");
    }
}

#[test]
fn on_bc_27_and_28_the_asserterror_cells_are_refused() {
    for major in [27, 28] {
        let target = al_version::Target::for_bc(al_version::BcVersion::new(major, 5));
        let o = cells_on(TestIsolation::Codeunit, Some(&target));
        for (test, key) in [
            ("C_RE5Reads", "'P5RE5'"),
            ("C_RE4Reads", "'P5RE4'"),
            ("C_DEReads", "'P5DE'"),
        ] {
            assert!(not_run(&row(&o, test).2, key), "BC {major} {test}: {o:?}");
        }
    }
}
