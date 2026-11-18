//! al-runner issue 7821, the reporter's variants of the journal whose Post left another primary key in Rec on its new-row line:
//! the page with one more field, two and three rows, one more row in another batch, and subscribers to the Post action. The
//! objects and tests are the pinned corpus file testpage/R7821JournalPostVariants.al, unchanged, with the corpus's Probe
//! Observer (crates/al-vm/src/pages/post_left_new_line.rs). Each neighbor below changes one fact of a measured cell and keeps
//! main's refusal.

use al_builtins::assumptions::Assumption;
use al_testkit::{blocked, compile};
use al_testrun::Outcome;
use al_testrun::isolation::TestIsolation;

/// The corpus's Probe Observer (codeunits 69160 and 69163).
const OBSERVER: &str = include_str!("../../../../corpus/tests/al-language/_fixtures/ProbeObserver.Codeunit.al");

/// The pinned corpus file of the measured cells.
const PROBE: &str = include_str!("../../../../corpus/tests/al-language/testpage/R7821JournalPostVariants.al");

/// Each test of codeunit `codeunit`, its outcome and the assumptions it went through.
fn run(src: &str, codeunit: &str) -> Vec<(String, Outcome, Vec<Assumption>)> {
    let l = compile(&format!("{src}\n{OBSERVER}"));
    let engine = al_vm::Engine::new(l.program.clone(), blocked(&l));
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
        TestIsolation::Codeunit,
        None,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut hits,
        None,
    );
    outcomes
        .into_iter()
        .map(|(name, outcome)| {
            let marks = hits
                .iter()
                .find(|(n, _): &&(String, Vec<Assumption>)| *n == name)
                .map(|(_, m)| m.clone())
                .unwrap_or_default();
            (name, outcome, marks)
        })
        .collect()
}

const TESTS: &str = "TP R7821V Tests";

/// The outcome of test `test` of `src`.
fn outcome(src: &str, test: &str) -> (Outcome, Vec<Assumption>) {
    run(src, TESTS)
        .into_iter()
        .find(|(n, ..)| n == test)
        .map(|(_, o, m)| (o, m))
        .unwrap_or_else(|| panic!("no test {test}"))
}

/// `src` with each `(from, to)` of `edits` applied once; each `from` must be there.
fn variant(edits: &[(&str, &str)]) -> String {
    edits.iter().fold(PROBE.to_string(), |s, (from, to)| {
        assert!(s.contains(from), "not in the probe: {from}");
        s.replacen(from, to, 1)
    })
}

#[test]
fn the_probe_passes_without_an_assumption() {
    let all = run(PROBE, TESTS);
    assert_eq!(all.len(), 11);
    for (name, o, marks) in all {
        assert!(matches!(o, Outcome::Pass), "{name}: {o:?}");
        assert!(marks.is_empty(), "{name}: {marks:?}");
    }
}

/// The refusal of main for an action that left another key in Rec outside the pinned cells.
fn refused_at_post(o: &Outcome) -> bool {
    matches!(o, Outcome::NotRun(r) if r.contains("another primary key"))
}

/// One fact of a measured cell changed: the Post keeps main's refusal and takes no assumption.
#[test]
fn neighbors_of_the_cells_keep_mains_refusal() {
    let run_wks = "        RunWks(Observer, 1, '', true, 0);";
    let open_edit = |from: &'static str, to: &'static str| (from, to);
    let cases: Vec<(&str, String, &str)> = vec![
        (
            "two rows, OpenEdit",
            variant(&[open_edit(
                "        RunWks(Observer, 2, '', true, 0);",
                "        RunWks(Observer, 2, '', false, 0);",
            )]),
            "TPV7821_TwoLines_PageRun",
        ),
        (
            "four rows",
            variant(&[(run_wks, "        RunWks(Observer, 4, '', true, 0);")]),
            "TPV7821_Base_OneLine_PageRun",
        ),
        (
            "other row in A1, OpenEdit",
            variant(&[(
                "        RunWks(Observer, 1, 'A1', true, 0);",
                "        RunWks(Observer, 1, 'A1', false, 0);",
            )]),
            "TPV7821_OtherBatchBefore_OneLine_PageRun",
        ),
        (
            "logging subscribers, OpenEdit",
            variant(&[(
                "        RunWks(Observer, 1, '', true, 2);",
                "        RunWks(Observer, 1, '', false, 2);",
            )]),
            "TPV7821_ActionSubsLogged_OneLine_PageRun",
        ),
        (
            "other row in B3",
            variant(&[(
                "        RunWks(Observer, 1, 'B2', true, 0);",
                "        RunWks(Observer, 1, 'B3', true, 0);",
            )]),
            "TPV7821_OtherBatchAfter_OneLine_PageRun",
        ),
        (
            "other row on line 20000",
            variant(&[(
                "            Line.\"Line No.\" := 10000;",
                "            Line.\"Line No.\" := 20000;",
            )]),
            "TPV7821_OtherBatchAfter_OneLine_PageRun",
        ),
        (
            "other row with Quantity 5",
            variant(&[("            Line.Quantity := 100;", "            Line.Quantity := 5;")]),
            "TPV7821_OtherBatchAfter_OneLine_PageRun",
        ),
        (
            "two other rows",
            variant(&[(
                "            Line.Quantity := 100;\n            Line.Insert();",
                "            Line.Quantity := 100;\n            Line.Insert();\n            Line.\"Line No.\" := 20000;\n            \
                 Line.Insert();",
            )]),
            "TPV7821_OtherBatchAfter_OneLine_PageRun",
        ),
        (
            "both subscriber codeunits bound",
            variant(&[(
                "            1:\n                BindSubscription(Subs);",
                "            1:\n                begin\n                    BindSubscription(Subs);\n                    \
                 BindSubscription(LogSubs);\n                end;",
            )]),
            "TPV7821_ActionSubs_OneLine_PageRun",
        ),
        (
            "a counting subscriber that counts two",
            variant(&[("        Runs += 1;\n    end;", "        Runs += 2;\n    end;")]),
            "TPV7821_ActionSubs_OneLine_PageRun",
        ),
        (
            "the counting codeunit SingleInstance",
            variant(&[(
                "codeunit 67208 \"TP R7821V Subs\"\n{\n    EventSubscriberInstance = Manual;\n",
                "codeunit 67208 \"TP R7821V Subs\"\n{\n    EventSubscriberInstance = Manual;\n    SingleInstance = true;\n",
            )]),
            "TPV7821_ActionSubs_OneLine_PageRun",
        ),
        (
            "the logging codeunit SingleInstance",
            variant(&[(
                "codeunit 67209 \"TP R7821V Log Subs\"\n{\n    EventSubscriberInstance = Manual;\n",
                "codeunit 67209 \"TP R7821V Log Subs\"\n{\n    EventSubscriberInstance = Manual;\n    SingleInstance = true;\n",
            )]),
            "TPV7821_ActionSubsLogged_OneLine_PageRun",
        ),
        (
            "counting subscribers bound statically",
            variant(&[(
                "codeunit 67208 \"TP R7821V Subs\"\n{\n    EventSubscriberInstance = Manual;\n",
                "codeunit 67208 \"TP R7821V Subs\"\n{\n",
            )]),
            "TPV7821_Base_OneLine_PageRun",
        ),
        (
            "a third subscriber, to OnAfterGetCurrRecordEvent",
            variant(&[(
                "    procedure Count(): Integer",
                "    [EventSubscriber(ObjectType::Page, Page::\"TP R7821V Wks\", 'OnAfterGetCurrRecordEvent', '', false, false)]\n    \
                 local procedure OnCurr(var Rec: Record \"TP R7821V Line\")\n    begin\n    end;\n\n    procedure Count(): Integer",
            )]),
            "TPV7821_Base_OneLine_PageRun",
        ),
        (
            "the Batch Name page with two rows",
            variant(&[(
                "        InsertLines(Line, 1, '');\n        if WithPageRun then begin\n            Worksheet.Trap();\n            \
                 Page.Run(Page::\"TP R7821V Field Wks\", Line);",
                "        InsertLines(Line, 2, '');\n        if WithPageRun then begin\n            Worksheet.Trap();\n            \
                 Page.Run(Page::\"TP R7821V Field Wks\", Line);",
            )]),
            "TPV7821_ExtraField_OneLine_PageRun",
        ),
        (
            "the Batch Name page with a log variable of another name in one trigger",
            variant(&[(
                "    trigger OnNewRecord(BelowxRec: Boolean)\n    var\n        Log: Codeunit \"TP R7821V Log\";\n    begin\n        \
                 Log.Add('N:' + Format(Rec.\"Line No.\"));\n    end;\n\n    trigger OnAfterGetCurrRecord()\n    var\n        Log: \
                 Codeunit \"TP R7821V Log\";\n    begin\n        Log.Add('C:' + Format(Rec.\"Line No.\"));\n    end;\n}\n\ncodeunit",
                "    trigger OnNewRecord(BelowxRec: Boolean)\n    var\n        Log: Codeunit \"TP R7821V Log\";\n    begin\n        \
                 Log.Add('N:' + Format(Rec.\"Line No.\"));\n    end;\n\n    trigger OnAfterGetCurrRecord()\n    var\n        Other: \
                 Codeunit \"TP R7821V Log\";\n    begin\n        Other.Add('C:' + Format(Rec.\"Line No.\"));\n    end;\n}\n\ncodeunit",
            )]),
            "TPV7821_ExtraField_OneLine_PageRun",
        ),
    ];
    for (what, src, test) in cases {
        let (o, marks) = outcome(&src, test);
        assert!(refused_at_post(&o), "{what}: {o:?}");
        assert!(marks.is_empty(), "{what}: {marks:?}");
    }
}

/// In a variant cell the Post lets only the corpus file's value reads and Close through: a move after it is refused.
#[test]
fn a_move_after_the_post_of_a_variant_cell_is_refused() {
    let src = variant(&[(
        "        Observer.Observe('shown line', Worksheet.\"Line No.\".Value());\n        Worksheet.Close();\n        \
         Observer.Observe('after close', Log.Take());\n        case Subscribers of",
        "        Observer.Observe('shown line', Worksheet.\"Line No.\".Value());\n        Worksheet.Next();\n        \
         Worksheet.Close();\n        Observer.Observe('after close', Log.Take());\n        case Subscribers of",
    )]);
    let (o, marks) = outcome(&src, "TPV7821_Base_OneLine_PageRun");
    assert!(refused_after_post(&o), "{o:?}");
    assert!(marks.is_empty(), "{marks:?}");
}

/// The refusal of a call after the Post of a variant cell other than the reads the corpus file makes and Close.
fn refused_after_post(o: &Outcome) -> bool {
    matches!(o, Outcome::NotRun(r) if r.contains("other than a value read of the line or the batch and Close"))
}

/// The repro's cell (no subscriber of the page, one row, no other row) keeps main's end: after the Post only Close, so the
/// probe's read of the shown line is refused.
#[test]
fn the_repro_cell_keeps_only_close_after_the_post() {
    let start = PROBE.find("// V4: counts the runs").expect("the subscribers");
    let end = PROBE.find("codeunit 67210").expect("the tests");
    let src = format!(
        "{}codeunit 67208 \"TP R7821V Subs\" {{ procedure Count(): Integer begin end; }}\ncodeunit 67209 \"TP R7821V Log Subs\" {{ }}\n{}",
        &PROBE[..start],
        &PROBE[end..]
    );
    let (o, marks) = outcome(&src, "TPV7821_Base_OneLine_PageRun");
    assert!(
        matches!(&o, Outcome::NotRun(r) if r.contains("other than Close")),
        "{o:?}"
    );
    assert!(marks.contains(&Assumption::PostLeftOtherKeyOnNewLine), "{marks:?}");
}

/// The record handed to Page.Run and the view when Post starts are facts of every Page.Run cell (InsertLines, :276-298: a stored
/// record filtered to batch B1 in filter group 0, on the first of its rows, which are lines 10000, 20000, ... with Quantity
/// 300). One of them changed: the Post keeps main's refusal and takes no assumption.
#[test]
fn neighbors_of_the_record_and_the_view_keep_mains_refusal() {
    let filtered = "        Line.SetRange(\"Batch Name\", 'B1');\n        Line.FindFirst();";
    let cases: Vec<(&str, String, &str)> = vec![
        (
            "two rows, the record on the last",
            variant(&[(
                filtered,
                "        Line.SetRange(\"Batch Name\", 'B1');\n        Line.FindLast();",
            )]),
            "TPV7821_TwoLines_PageRun",
        ),
        (
            "three rows, the record on the middle one",
            variant(&[(
                filtered,
                "        Line.SetRange(\"Batch Name\", 'B1');\n        Line.FindFirst();\n        Line.Next();",
            )]),
            "TPV7821_ThreeLines_PageRun",
        ),
        (
            "the record without a filter",
            variant(&[(filtered, "        Line.FindFirst();")]),
            "TPV7821_Base_OneLine_PageRun",
        ),
        (
            "the record with a Line No. filter too",
            variant(&[(
                filtered,
                "        Line.SetRange(\"Batch Name\", 'B1');\n        Line.SetRange(\"Line No.\", 10000);\n        Line.FindFirst();",
            )]),
            "TPV7821_Base_OneLine_PageRun",
        ),
        (
            "the record with a Quantity filter too",
            variant(&[(
                filtered,
                "        Line.SetRange(\"Batch Name\", 'B1');\n        Line.SetRange(Quantity, 300);\n        Line.FindFirst();",
            )]),
            "TPV7821_Base_OneLine_PageRun",
        ),
        (
            "the batch filter in filter group 2",
            variant(&[(
                filtered,
                "        Line.FilterGroup(2);\n        Line.SetRange(\"Batch Name\", 'B1');\n        Line.FilterGroup(0);\n        \
                 Line.FindFirst();",
            )]),
            "TPV7821_Base_OneLine_PageRun",
        ),
        (
            "the record without a filter on the A1 row",
            variant(&[(filtered, "        Line.FindFirst();")]),
            "TPV7821_OtherBatchBefore_OneLine_PageRun",
        ),
        (
            "a temporary record",
            variant(&[(
                "        Line: Record \"TP R7821V Line\";\n        Log: Codeunit \"TP R7821V Log\";\n        Subs:",
                "        Line: Record \"TP R7821V Line\" temporary;\n        Log: Codeunit \"TP R7821V Log\";\n        Subs:",
            )]),
            "TPV7821_Base_OneLine_PageRun",
        ),
        (
            "the row on line 20000",
            variant(&[(
                "            Line.\"Line No.\" := i * 10000;",
                "            Line.\"Line No.\" := i * 20000;",
            )]),
            "TPV7821_Base_OneLine_PageRun",
        ),
        (
            "the row with Quantity 0",
            variant(&[("            Line.Quantity := 300;", "            Line.Quantity := 0;")]),
            "TPV7821_Base_OneLine_PageRun",
        ),
        (
            "a second row inserted after the open",
            variant(&[(
                "        Log.Take();\n        Worksheet.Post.Invoke();\n        Observer.Observe('after post', Log.Take());\n        \
                 Observer.Observe('shown line', Worksheet.\"Line No.\".Value());\n        Worksheet.Close();",
                "        Line.\"Line No.\" := 20000;\n        Line.Insert();\n        Log.Take();\n        Worksheet.Post.Invoke();\n        \
                 Observer.Observe('after post', Log.Take());\n        Observer.Observe('shown line', Worksheet.\"Line No.\".Value());\n        \
                 Worksheet.Close();",
            )]),
            "TPV7821_Base_OneLine_PageRun",
        ),
    ];
    for (what, src, test) in cases {
        let (o, marks) = outcome(&src, test);
        assert!(refused_at_post(&o), "{what}: {o:?}");
        assert!(marks.is_empty(), "{what}: {marks:?}");
    }
}

/// After the Post of a variant cell the corpus file reads only Line No. and, on the Batch Name page, Batch Name, by Value()
/// (:334, :364, :365): a read of Quantity, or of the line by another method, is refused.
#[test]
fn other_reads_after_the_post_of_a_variant_cell_are_refused() {
    let base_read = "        Observer.Observe('shown line', Worksheet.\"Line No.\".Value());\n        Worksheet.Close();\n        \
                     Observer.Observe('after close', Log.Take());\n        case Subscribers of";
    let base = |read: &str| {
        variant(&[(
            base_read,
            &format!(
                "        Observer.Observe('shown line', Worksheet.\"Line No.\".Value());\n{read}\n        Worksheet.Close();\n        \
                 Observer.Observe('after close', Log.Take());\n        case Subscribers of"
            ),
        )])
    };
    let field_read = "        Observer.Observe('shown batch', Worksheet.\"Batch Name\".Value());";
    let cases: Vec<(&str, String, &str)> = vec![
        (
            "Quantity.Value()",
            base("        Observer.Observe('q', Worksheet.Quantity.Value());"),
            "TPV7821_Base_OneLine_PageRun",
        ),
        (
            "Quantity.AsDecimal()",
            base("        Observer.Observe('q', Format(Worksheet.Quantity.AsDecimal()));"),
            "TPV7821_Base_OneLine_PageRun",
        ),
        (
            "Quantity.AssertEquals(0)",
            base("        Worksheet.Quantity.AssertEquals(0);"),
            "TPV7821_Base_OneLine_PageRun",
        ),
        (
            "Line No..AsInteger()",
            base("        Observer.Observe('l', Format(Worksheet.\"Line No.\".AsInteger()));"),
            "TPV7821_Base_OneLine_PageRun",
        ),
        (
            "Quantity.Value() on the Batch Name page",
            variant(&[(
                field_read,
                "        Observer.Observe('shown batch', Worksheet.\"Batch Name\".Value());\n        \
                 Observer.Observe('q', Worksheet.Quantity.Value());",
            )]),
            "TPV7821_ExtraField_OneLine_PageRun",
        ),
    ];
    for (what, src, test) in cases {
        let (o, marks) = outcome(&src, test);
        assert!(refused_after_post(&o), "{what}: {o:?}");
        assert!(marks.is_empty(), "{what}: {marks:?}");
    }
}

/// The table named without quotes (`SourceTable = TPR7821VLine;`) is the same table: the pages read as the pinned ones, and every
/// test passes as with the corpus file's spelling.
#[test]
fn the_probe_passes_with_the_table_named_without_quotes() {
    let src = PROBE.replace("\"TP R7821V Line\"", "TPR7821VLine");
    for (name, o, marks) in run(&src, TESTS) {
        assert!(matches!(o, Outcome::Pass), "{name}: {o:?}");
        assert!(marks.is_empty(), "{name}: {marks:?}");
    }
}

/// Facts of a Page.Run or OpenEdit cell that `new_line_post_start` must see by equality: the sort of Rec (a record fresh from
/// Reset, :280, in its primary key ascending), no marks, no write to the store and no bind or unbind between the end of the open
/// and the Post (the corpus file writes and binds only before the open, :319-330), and the record handed to Page.Run as
/// InsertLines leaves it (:296-297: on a stored row, unchanged, filter group 0, no SetAscending, ReadIsolation default). One of
/// them changed: the Post keeps main's refusal and takes no assumption.
#[test]
fn neighbors_of_the_sort_the_open_and_the_handed_record_keep_mains_refusal() {
    let filtered = "        Line.SetRange(\"Batch Name\", 'B1');\n        Line.FindFirst();";
    let after_find =
        |extra: &str| format!("        Line.SetRange(\"Batch Name\", 'B1');\n        Line.FindFirst();\n{extra}");
    let before_find =
        |extra: &str| format!("        Line.SetRange(\"Batch Name\", 'B1');\n{extra}        Line.FindFirst();");
    let after_open = "        Log.Take();\n        Worksheet.Post.Invoke();";
    let insert_b2 = "        Line.Reset();\n        Line.Init();\n        Line.\"Batch Name\" := 'B2';\n        Line.\"Line No.\" := 10000;\n        \
                     Line.Quantity := 100;\n        Line.Insert();\n";
    let then_post = |extra: &str| format!("{extra}{after_open}");
    let cases: Vec<(&str, String, &str)> = vec![
        (
            "one row, Ascending(false)",
            variant(&[(filtered, &before_find("        Line.Ascending(false);\n"))]),
            "TPV7821_Base_OneLine_PageRun",
        ),
        (
            "one row, Ascending(false), Batch Name page",
            variant(&[(filtered, &before_find("        Line.Ascending(false);\n"))]),
            "TPV7821_ExtraField_OneLine_PageRun",
        ),
        (
            "two rows, SetCurrentKey(Quantity)",
            variant(&[(filtered, &before_find("        Line.SetCurrentKey(Quantity);\n"))]),
            "TPV7821_TwoLines_PageRun",
        ),
        (
            "a marked record",
            variant(&[(filtered, &after_find("        Line.Mark(true);"))]),
            "TPV7821_Base_OneLine_PageRun",
        ),
        (
            "a field value not stored",
            variant(&[(filtered, &after_find("        Line.Quantity := 5;"))]),
            "TPV7821_Base_OneLine_PageRun",
        ),
        (
            "filter group 4 left current",
            variant(&[(filtered, &after_find("        Line.FilterGroup(4);"))]),
            "TPV7821_Base_OneLine_PageRun",
        ),
        (
            "the record on a key no row has",
            variant(&[(filtered, &after_find("        Line.\"Line No.\" := 5;"))]),
            "TPV7821_Base_OneLine_PageRun",
        ),
        (
            "SetAscending of Line No.",
            variant(&[(
                filtered,
                &before_find("        Line.SetAscending(\"Line No.\", false);\n"),
            )]),
            "TPV7821_Base_OneLine_PageRun",
        ),
        (
            "ReadIsolation set",
            variant(&[(
                filtered,
                &after_find("        Line.ReadIsolation := IsolationLevel::ReadCommitted;"),
            )]),
            "TPV7821_Base_OneLine_PageRun",
        ),
        (
            "the B2 row inserted after the open",
            variant(&[(after_open, &then_post(insert_b2))]),
            "TPV7821_Base_OneLine_PageRun",
        ),
        (
            "the B2 row inserted after OpenEdit",
            variant(&[
                (after_open, &then_post(insert_b2)),
                (
                    "        RunWks(Observer, 1, '', false, 1);",
                    "        RunWks(Observer, 1, '', false, 0);",
                ),
            ]),
            "TPV7821_ActionSubs_OneLine_OpenEdit",
        ),
        (
            "the B2 row deleted after the open",
            variant(&[(
                after_open,
                &then_post("        Line.Reset();\n        Line.Get('B2', 10000);\n        Line.Delete();\n"),
            )]),
            "TPV7821_OtherBatchAfter_OneLine_PageRun",
        ),
        (
            "a Modify that keeps the values after the open",
            variant(&[(after_open, &then_post("        Line.Modify();\n"))]),
            "TPV7821_Base_OneLine_PageRun",
        ),
        (
            "the counting codeunit bound after the open",
            variant(&[(after_open, &then_post("        BindSubscription(Subs);\n"))]),
            "TPV7821_Base_OneLine_PageRun",
        ),
        (
            "the counting codeunit unbound after the open",
            variant(&[(
                after_open,
                &then_post("        if Subscribers = 1 then\n            UnbindSubscription(Subs);\n"),
            )]),
            "TPV7821_ActionSubs_OneLine_PageRun",
        ),
    ];
    let ran: Vec<String> = cases
        .into_iter()
        .filter_map(|(what, src, test)| {
            let (o, marks) = outcome(&src, test);
            (!refused_at_post(&o) || !marks.is_empty()).then(|| format!("{what}: {o:?} {marks:?}"))
        })
        .collect();
    assert!(ran.is_empty(), "{ran:#?}");
}
