//! The entry conditions of the pages P4 rules (al-runner PR 7807 review): each new path runs only on the shape its corpus pin
//! has; every neighbor refuses or keeps main's behavior. Main's outcomes were read with the runner of main at ca0c74808.
//! - `Last` past a full window reads to the end of the view (`long_view.rs`, LastOnLongList.al:253-411): a List over a stored
//!   table with only its own OnAfterGetRecord, 41 to 100 rows. A page extension's trigger or a page event subscriber, 40 or 101
//!   rows: refused as on main. OpenEdit of 20 to 40 rows: main's reads.
//! - The open of a long tree reads on (LastOnLongList.al:482): 60 rows, OpenView, Editable = false. OpenEdit, an editable
//!   tree, 20, 21, 95 and 100 rows: main's behavior. A tree of 60 rows that shows a full window: refused.
//! - The preloaded temporary Worksheet (`temp_ws_preloaded.rs`, PageVarOpenRow.al:942-998): an extension trigger keeps main's
//!   behavior; Update(true) keeps main's reads; Rec on a key no row has, more rows than a window and Update off the first row
//!   refuse.
//! - A key typed on a `CurrPage.Run()` list (`currpage_run_typed_key.rs`, PageVarOpenRow.al:788): only key typed, then Next, on
//!   the last row, on the pin's page shape. Close, Previous, a second field, another row, another page: refused as on main.
//! - GoToKey with a lowercase or wrong-type key: No, as on main.

use al_testrun::Outcome;

const SRC: &str = include_str!("pages_p4_allowlists.al");

enum Want {
    /// The test ran and its Error text holds this.
    Ran(String),
    /// The test was refused with this reason.
    Refused(&'static str),
}

/// `a a+1 ... b ` as the probe logs rows.
fn ids(a: i32, b: i32) -> String {
    (a..=b).map(|i| format!("{i} ")).collect()
}

fn ran(s: impl Into<String>) -> Want {
    Want::Ran(s.into())
}

const SECOND_BLOCK: &str = "a list with a second full block of rows past the window";
const ONE_WINDOW: &str = "a list of exactly one window";
const GONE_ROW: &str = "a page saves a changed row that is no longer stored";
const WIDE: &str = "with more rows than a window holds";

fn wants() -> Vec<(&'static str, Want)> {
    let w = format!("{}1 ", ids(1, 20));
    let open = |n: i32| format!("open[{w}] last=Yes on {n}");
    let edit_main = |n: i32| {
        format!(
            "{} [{n} ] prev=Yes on {} [{} {n} {} ] first=Yes on 1 [{w}] next=Yes on 2 []",
            open(n),
            n - 1,
            n - 1,
            n - 1
        )
    };
    let whole = |n: i32| {
        format!(
            "{} [{}] prev=Yes on {} [] first=Yes on 1 [] next=Yes on 2 []",
            open(n),
            ids(20, n),
            n - 1
        )
    };
    let tree_long = format!("open on 1 [{w}{}]", ids(20, 60));
    let tree_main = |walk: i32, last: i32| {
        // The rows past the window as the walk reads them; with more than one, the first of them once more at the end.
        let past: Vec<String> = (21..=last).step_by(5).map(|i| format!("{i} ")).collect();
        let reads = if past.len() > 1 {
            format!("{}21 ", past.concat())
        } else {
            past.concat()
        };
        format!("open on 1 [{w}] walk={walk} on {last} [{reads}]")
    };
    let ws_open = "open on C4 [R:C4 R:C4 R:C3 R:C2 R:C1 R:C4 ] first on C4 []";
    vec![
        // Last on a long list.
        (
            "Edit20_T",
            ran(format!(
                "{} [] prev=Yes on 19 [] first=Yes on 1 [] next=Yes on 2 []",
                open(20)
            )),
        ),
        ("Edit21_T", ran(edit_main(21))),
        ("Edit25_T", ran(edit_main(25))),
        ("Edit39_T", ran(edit_main(39))),
        ("Edit40_T", ran(edit_main(40))),
        ("Edit41_T", ran(whole(41))),
        ("Edit60_T", ran(format!("{} [{}]", open(60), ids(20, 60)))),
        ("View20_T", Want::Refused(ONE_WINDOW)),
        ("View25_T", ran(whole(25))),
        ("View40_T", Want::Refused(SECOND_BLOCK)),
        ("View45_T", ran(whole(45))),
        ("View101_T", Want::Refused(SECOND_BLOCK)),
        (
            "View45Filter_T",
            ran(format!("open[{w}{w}] last=Yes on 45 [{}]", ids(20, 45))),
        ),
        ("View45NextThenLast_T", ran(format!("{} [{}]", open(45), ids(20, 45)))),
        ("NewPage45Edit_T", ran(format!("{} [N 45 ]", open(45)))),
        ("NewPage25Edit_T", ran(format!("{} [N 25 ]", open(25)))),
        ("ExtPage45View_T", Want::Refused(SECOND_BLOCK)),
        (
            "ExtPage25View_T",
            ran(format!("open[{w}XC ] last=Yes on 25 [{}]", ids(20, 25))),
        ),
        ("SubPage45View_T", Want::Refused(SECOND_BLOCK)),
        // The open of a long tree.
        ("Tree60View_T", ran(format!("{tree_long} walk=12 on 56 []"))),
        (
            "Tree60Last_T",
            ran(format!("{tree_long} last=Yes on 56 [] expand, next=Yes on 57 []")),
        ),
        ("Tree60Edit_T", ran(tree_main(12, 56))),
        ("Tree21View_T", ran(tree_main(5, 21))),
        ("Tree20View_T", ran(tree_main(4, 16))),
        ("Tree95View_T", ran(tree_main(19, 91))),
        ("Tree100View_T", ran(tree_main(20, 96))),
        (
            "TreeEd60View_T",
            ran(format!("open on 1 [{w}] walk=12 [21 26 31 36 41 46 51 56 21 ]")),
        ),
        (
            "TreeEd60Edit_T",
            ran(format!("open on 1 [{w}] walk=12 [21 26 31 36 41 46 51 56 21 ]")),
        ),
        (
            "TreeTop60View_T",
            Want::Refused("a tree whose view shows a full window of rows or more"),
        ),
        // GoToKey key forms.
        ("GoToKeyLowerListWithTrigger_T", ran("No on A []")),
        ("GoToKeyLowerWorksheet_T", ran("No on A []")),
        ("GoToKeyWrongTypeListWithTrigger_T", ran("No on 1 []")),
        ("GoToKeyUpperListWithTrigger_T", ran("Yes on B []")),
        // A key typed on a CurrPage.Run list.
        ("PickKeyNext_T", Want::Refused(GONE_ROW)),
        ("PickKeyClose_T", Want::Refused(GONE_ROW)),
        ("PickKeyAndNameNext_T", Want::Refused(GONE_ROW)),
        ("PickNameThenKeyNext_T", Want::Refused(GONE_ROW)),
        ("PickKeyPrevious_T", Want::Refused(GONE_ROW)),
        ("PickKeyOnFirstRowNext_T", Want::Refused(GONE_ROW)),
        (
            "PinKeyNext_T",
            ran("on P2 [O:P2 C:P2 ] next=Yes [N C: ] | on : P1=preloaded;P2=preloaded;"),
        ),
        ("PinKeyClose_T", Want::Refused(GONE_ROW)),
        ("PinKeyAndNameNext_T", Want::Refused(GONE_ROW)),
        ("PinKeyPrevious_T", Want::Refused(GONE_ROW)),
        ("PinFirstRowKeyNext_T", Want::Refused(GONE_ROW)),
        ("PinExtKeyNext_T", Want::Refused(GONE_ROW)),
        // The preloaded temporary Worksheet.
        (
            "WsUpdFalseFirstRow_T",
            ran(format!(
                "{ws_open} after on C4/n4 [R:C4 R:C4 R:C3 R:C2 R:C1 R:C4 ] | on : "
            )),
        ),
        (
            "WsUpdTrueFirstRow_T",
            ran(format!("{ws_open} after on C4/edited [R:C4 ] | on : ")),
        ),
        (
            "WsNoUpdFirstRow_T",
            ran(format!("{ws_open} after on C4/edited [] | on : ")),
        ),
        (
            "WsNoUpdEditNext_T",
            ran(format!("{ws_open} next=Yes after on C3/n3 [] | on C4: ")),
        ),
        ("WsOffKey_T", Want::Refused("on a primary key no row has")),
        ("Ws21Rows_T", Want::Refused(WIDE)),
        ("Ws25RowsMiddle_T", Want::Refused(WIDE)),
        ("WsUpdOffFirstRow_T", Want::Refused("off its first row")),
        (
            "WsExtTrigger_T",
            ran("open on C4 [R:C4 R:C3 R:C2 R:C1 R:C4 XC:C4 ] after on C4/edited [R:C4 XC:C4 ]"),
        ),
    ]
}

#[test]
fn each_new_page_path_runs_only_on_its_pinned_shape() {
    let mut out = al_testkit::outcomes_in(SRC, "RV Tests");
    out.extend(al_testkit::outcomes_in(SRC, "RV Tests 2"));
    let mut wrong = Vec::new();
    for (name, want) in wants() {
        let Some((_, got)) = out.iter().find(|(t, _)| t == name) else {
            wrong.push(format!("{name}: no outcome"));
            continue;
        };
        let ok = match (&want, got) {
            (Want::Refused(r), Outcome::NotRun(s)) => s.contains(r) && !s.contains("earlier test of this codeunit"),
            (Want::Ran(p), Outcome::Fail(s) | Outcome::NotRun(s)) => s.contains(p.as_str()),
            _ => false,
        };
        if !ok {
            let want = match want {
                Want::Ran(p) => format!("ran: {p}"),
                Want::Refused(r) => format!("refused: {r}"),
            };
            wrong.push(format!("{name}\n  want {want}\n  got  {got:?}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
