//! What `CurrentTransactionType(X)` does once the transaction has started, and a write at a type that does not write: the
//! open probe of the review of al-runner pull request 8607 (record/TxTypeSetAfterCall.al and
//! session-system/TxTypeFirstTestMethod.al, copied to fixtures/outside-fb/TxTypeSetAfterCallProbe.al and
//! TxTypeFirstTestProbe.al). Every test passes; the cells no corpus case pins carry the mark of the open probe.

use al_sema::workspace::AppKind;
use al_testrun::Outcome;

const PROBE_APP_ID: &str = "7d2e4c1a-8607-4b3e-9f10-2a6b8c9d0e17";

fn run(src: &str, codeunit: &str) -> Vec<(String, Outcome, Vec<&'static str>)> {
    let src = format!(
        "{}\n{}",
        src.replace(
            &format!("\"{codeunit}\"\n{{\n    Subtype = Test;"),
            "T\n{\n    Subtype = Test;"
        ),
        include_str!("../fixtures/outside-sc/Assert.al")
    );
    crate::field_extension_package_ids::run(&[(PROBE_APP_ID, AppKind::Workspace, &src)])
        .into_iter()
        .map(|(n, o, m)| {
            let mut ids: Vec<&'static str> = m.iter().map(|a| a.info().id).collect();
            ids.sort();
            ids.dedup();
            (n, o, ids)
        })
        .collect()
}

const SET_AFTER_START: &str = "transaction-type-set-after-start";
const READ_ONLY: &str = "transaction-type-read-only-writes";
const ROLLBACK: &str = "first-test-rollback-ends-runner-transaction";

#[test]
fn the_probe_of_sets_after_a_database_call_passes_with_its_marks() {
    let src = include_str!("../fixtures/outside-fb/TxTypeSetAfterCallProbe.al");
    let o = run(src, "TxS Set After Call");
    let want = |n: &str| -> Vec<&str> {
        let no: u32 = n[3..5].parse().unwrap();
        match no {
            // Pinned: UpdateNoLocks then Update and Report (ProbeSystemEnumMembers.al:114-141), Snapshot and Browse
            // (PlTDatabase.al:143-171); Update then Snapshot (TestClearAllTransactionTypeContracts.al:274-286); a set to the
            // type the transaction has.
            1..=5 | 7 | 8 | 11..=15 | 35 => vec![],
            6 | 9 | 10 => vec![SET_AFTER_START],
            16..=34 => vec![READ_ONLY],
            _ => panic!("{n}"),
        }
    };
    let wrong: Vec<_> = o
        .iter()
        .filter(|(n, r, m)| *r != Outcome::Pass || *m != want(n))
        .collect();
    assert_eq!(o.len(), 35, "{o:#?}");
    assert!(wrong.is_empty(), "{wrong:#?}");
}

#[test]
fn the_probe_of_the_first_test_method_passes_with_its_marks() {
    // The first-test probe reads table 97210 of the other file.
    let src = format!(
        "{}\n{}",
        include_str!("../fixtures/outside-fb/TxTypeFirstTestProbe.al"),
        include_str!("../fixtures/outside-fb/TxTypeSetAfterCallProbe.al")
    );
    let mut wrong = Vec::new();
    let mut n = 0;
    for pre in ["AssertError", "AssertErrorWrite", "Commit"] {
        for ty in ["Update", "Snapshot", "Browse", "Report"] {
            for (name, outcome, marks) in run(&src, &format!("TxF {pre} {ty}")) {
                n += 1;
                // A set after a Commit takes effect as at any test's start; after a rollback it rests on the open probe.
                let want: Vec<&str> = if pre == "Commit" { vec![] } else { vec![ROLLBACK] };
                if outcome != Outcome::Pass || marks != want {
                    wrong.push(format!("{pre} {ty} {name}: {outcome:?} {marks:?}, expected {want:?}"));
                }
            }
        }
    }
    assert_eq!(n, 12);
    assert!(wrong.is_empty(), "{wrong:#?}");
}
