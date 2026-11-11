//! The entry conditions of the four pages P4 rules (al-runner PR 7807 review): each rule runs only on the scenario its corpus
//! pin has, with every fact of that scenario required; every neighbor ends exactly as it did before the rules.
//!
//! `pages_p4_allowlists.al` holds the scenarios (the reviewers' probes of rounds 2 and 3 and more neighbors); each test ends
//! with `Error(<what it saw>)`. Two lists say how each must end, one line per test, `name<TAB>ran|refused<TAB>text`:
//! - `pages_p4_main.txt`: how the test ended before the rules, read with the runner of main at ca0c74808 (one codeunit per
//!   test). A test that is not in the second list must still end like this.
//! - `pages_p4_pinned.txt`: the tests that reach a rule on its pinned scenario, and how they end:
//!   - `Last` past a full window reads to the end of the view (`long_view.rs`, LastOnLongList.al:253-411): the pin's page,
//!     opened by OpenView or as a lookup variable with 41 to 100 rows, by OpenEdit or the static RunModal with 100, `Last`
//!     the first move (View45_T, View61_T, NView41_T, NView100_T, NLastTwice45_T, Edit100_T, LookupVar45_T, StaticModal100_T).
//!   - The open of the 60-row tree reads on (LastOnLongList.al:482: Tree60View_T, Tree60Last_T).
//!   - The preloaded temporary Worksheet (`temp_ws_preloaded.rs`, PageVarOpenRow.al:942-998) opens on Rec with 2 to 4 rows
//!     (CWs2Rows2_T, CWs2Pin_T, CWs2MPin_T, CWs2NoteNext_T); its first typed value with `Update(false)` on the first row
//!     drops the edit and reads the view again; on another row that is refused (CWs2SecondRow_T, WsUpdOffFirstRow_T); a row
//!     that was dirty before, and a second value, keep the earlier behavior (CWs2DirtyThenUpd_T, CWs2MDirtyThenUpd_T,
//!     CWs2MDirtyThenUpdNext_T, CWs2EditTwice_T, CWs2EditThenWalk_T).
//!   - A key typed on a `CurrPage.Run()` list is dropped (`currpage_run_typed_key.rs`, PageVarOpenRow.al:788: PinKeyNext_T,
//!     NPick2Pin_T); a later call of the test is refused (NPick2After_T).
//!
//! Everything else keeps main's outcome: a filter, a descending view, a part, an action, a function source, another property,
//! a table with triggers, a page extension, a subscriber, other row counts, a Rec OnOpenPage moved, three tree levels, a
//! non-modal Page.Run, a blank key, another typing order.

use al_testrun::Outcome;
use std::collections::BTreeMap;

const SRC: &str = include_str!("pages_p4_allowlists.al");
const MAIN: &str = include_str!("pages_p4_main.txt");
const PINNED: &str = include_str!("pages_p4_pinned.txt");
const CODEUNITS: [&str; 4] = ["RV Tests", "RV Tests 2", "RV Tests 3", "RV Tests 4"];

/// `name -> (ran, text)` of a list.
fn list(text: &'static str) -> BTreeMap<&'static str, (bool, &'static str)> {
    text.lines()
        .map(|l| {
            let mut parts = l.splitn(3, '\t');
            let (name, kind, text) = (parts.next().unwrap(), parts.next().unwrap(), parts.next().unwrap());
            assert!(kind == "ran" || kind == "refused", "{l}");
            (name, (kind == "ran", text))
        })
        .collect()
}

#[test]
fn each_new_page_path_runs_only_on_its_pinned_scenario() {
    let main = list(MAIN);
    let pinned = list(PINNED);
    assert!(
        pinned.keys().all(|k| main.contains_key(k)),
        "a pinned test main's list does not have"
    );
    let mut out = Vec::new();
    for codeunit in CODEUNITS {
        out.extend(al_testkit::outcomes_in(SRC, codeunit));
    }
    assert_eq!(out.len(), main.len(), "every test has a line in pages_p4_main.txt");
    let mut wrong = Vec::new();
    for (name, got) in &out {
        let Some((ran, text)) = pinned.get(name.as_str()).or_else(|| main.get(name.as_str())) else {
            wrong.push(format!("{name}: no line in pages_p4_main.txt"));
            continue;
        };
        let ok = match got {
            // A test after a refused one of its codeunit is reported as not run, with what it ended with.
            Outcome::Fail(s) | Outcome::NotRun(s) if *ran => s.contains(text),
            Outcome::NotRun(s) => s.contains(text) && !s.contains("earlier test of this codeunit"),
            _ => false,
        };
        if !ok {
            let rule = if pinned.contains_key(name.as_str()) {
                "pinned"
            } else {
                "as on main"
            };
            let kind = if *ran { "ran" } else { "refused" };
            wrong.push(format!("{name} ({rule})\n  want {kind}: {text}\n  got  {got:?}"));
        }
    }
    assert!(
        wrong.is_empty(),
        "{} of {} wrong\n{}",
        wrong.len(),
        out.len(),
        wrong.join("\n")
    );
}
