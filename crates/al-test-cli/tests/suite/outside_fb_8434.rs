//! The corpus probe of al-runner issue 8434 (corpus pull request 1454, merged, not pinned yet: tests/al-language/pages/currpage/
//! ListNewLineFactboxesCode.al, measured on BC 29.1, 11 of 11 green) and the reporter's repro, run here until a bump pins the
//! probe: a List page with code, control expressions, actions and four FactBoxes, whose new line the table field's OnValidate
//! inserts before the control's OnValidate calls `CurrPage.SaveRecord()` and `CurrPage.Update(false)`
//! (`al-vm` pages/code_inserted_coded.rs). The probe is read from its copy in the neighbor app. Delete this file when the pin
//! is in the corpus.

use al_builtins::assumptions::Assumption::{self, ListNewLineCodedFactboxes};
use al_test_cli::report::Outcome;

/// The corpus probe, whole, as the neighbor app holds it (its first line says so).
const PROBE: &str = include_str!("../../../../tests/neighbors/8434-factboxes-code/src/Probe.al");

/// A stand-in for the System Application's codeunit the probe's OnOpenPage calls: false, as on the OnPrem container the probe
/// was measured on.
const ENVIRONMENT_INFORMATION: &str =
    include_str!("../../../../tests/neighbors/8434-factboxes-code/src/EnvironmentInformation.al");

/// Main's refusal at the landing of a page that is not a measured cell.
const MAIN_REFUSAL: &str = "CurrPage.Update on a page that shows no row";

fn outcomes(src: &str, codeunit: &str) -> Vec<(String, Outcome, Vec<Assumption>)> {
    let src = format!(
        "{src}\n{}\n{ENVIRONMENT_INFORMATION}",
        include_str!("../fixtures/outside-sc/Assert.al")
    );
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

/// The outcome of test `name` of the probe's codeunit after `change` rewrote the probe, run alone.
fn probe_test(name: &str, change: impl Fn(&str) -> String) -> (Outcome, Vec<Assumption>) {
    let src = change(PROBE);
    let o = outcomes(&src, "LT NF Tests");
    let (_, r, marks) = o
        .into_iter()
        .find(|(n, _, _)| n == &format!("ListNewLine_FactboxesCode_{name}"))
        .expect("the test");
    (r, marks)
}

fn replace(from: &'static str, to: &'static str) -> impl Fn(&str) -> String {
    move |s: &str| {
        assert!(s.contains(from), "{from}");
        s.replacen(from, to, 1)
    }
}

/// The measured cells pass, flagged by list-new-line-coded-factboxes: both keys, with and without part reads, a blocked kind
/// (the page's TableRelation is not checked), the value already there and a second Value. The row before, the error before the
/// landing, Value before Name and the bound subscribers are refused.
#[test]
fn issue_8434_probe_cells() {
    let o = outcomes(PROBE, "LT NF Tests");
    for name in [
        "SecondaryKey_EmptyView_ReadsTheRowTwice",
        "SecondaryKey_WithoutPartReads_ReadsTheRowTwice",
        "PrimaryKey_EmptyView_ReadsTheRowOnce",
        "PrimaryKey_WithoutPartReads_ReadsTheRowOnce",
        "BlockedName_PageRelationNotChecked",
        "SameValue_SaveModifiesNothing",
        "SecondValue_SameStepsAgain",
    ] {
        let (_, r, marks) = o
            .iter()
            .find(|(n, _, _)| n == &format!("ListNewLine_FactboxesCode_{name}"))
            .expect("the test");
        assert_eq!(*r, Outcome::Pass, "{name}");
        assert!(marks.contains(&ListNewLineCodedFactboxes), "{name}: {marks:?}");
    }
    for (name, why) in [
        (
            "SecondaryKey_RowBefore_ReadsTheRowAgainFirst",
            "CurrPage.Update on a page that shows no row",
        ),
        ("UnknownName_ErrorUndoesTheInsert", "did not leave the line"),
        ("ValueFirst_SaveRecordInsertsTheLine", ""),
        ("SubscriberBound_RunsAfterEachModify", "did not leave the line"),
    ] {
        let (r, _) = probe_test(name, |s| s.to_string());
        assert!(matches!(&r, Outcome::NotRun(w) if w.contains(why)), "{name}: {r:?}");
    }
}

/// The reporter's repro: its code is not the probe's (review of #8463, round 1, finding 1), so it keeps main's way, with no
/// flag.
#[test]
fn issue_8434_repro_keeps_main() {
    let o = outcomes(
        include_str!("../fixtures/outside-fb/Issue8434.al"),
        "Repro Attr Line Tests",
    );
    let [(_, r, marks)] = o.as_slice() else { panic!("{o:?}") };
    assert!(
        matches!(r, Outcome::NotRun(w) if w.contains(MAIN_REFUSAL)) && marks.is_empty(),
        "{r:?} {marks:?}"
    );
}

/// The refusal of a call on the row the new line became that the probe does not make.
const CALL_REFUSAL: &str = "a call on a List page with a CardPart FactBox, on the row its new line became";

/// After the landing only reads, up to two SetValues that save and update, and OK() are measured: a third SetValue, Close(),
/// Next() and an action are refused at that call.
#[test]
fn issue_8434_unmeasured_calls_are_refused() {
    let drive_value2 = "            Check('rows2b', Rows());\n        end;";
    for (name, test, change) in [
        (
            "a third SetValue",
            "SecondValue_SameStepsAgain",
            replace(
                drive_value2,
                "            Check('rows2b', Rows());\n            Lines.Value.SetValue('ST-07');\n        end;",
            ),
        ),
        (
            "Next()",
            "SecondaryKey_EmptyView_ReadsTheRowTwice",
            replace(
                "        Lines.Name.SetValue('Station');\n        Check('setname', Log.Take());",
                "        Lines.Name.SetValue('Station');\n        Lines.Next();\n        Check('setname', Log.Take());",
            ),
        ),
        (
            "an action",
            "SecondaryKey_EmptyView_ReadsTheRowTwice",
            replace(
                "        Lines.Name.SetValue('Station');\n        Check('setname', Log.Take());",
                "        Lines.Name.SetValue('Station');\n        Lines.Recalculate.Invoke();\n        Check('setname', Log.Take());",
            ),
        ),
        (
            "a SetValue of a control the probe does not set",
            "SecondaryKey_EmptyView_ReadsTheRowTwice",
            replace(
                "        Lines.Value.SetValue(NewValue);",
                "        Lines.Active.SetValue(true);\n        Lines.Value.SetValue(NewValue);",
            ),
        ),
        (
            "a second SetValue of the control that landed",
            "SecondaryKey_EmptyView_ReadsTheRowTwice",
            replace(
                "        Lines.Value.SetValue(NewValue);",
                "        Lines.Name.SetValue('Station');\n        Lines.Value.SetValue(NewValue);",
            ),
        ),
    ] {
        let (r, _) = probe_test(test, change);
        assert!(
            matches!(&r, Outcome::NotRun(w) if w.contains(CALL_REFUSAL)),
            "{name}: {r:?}"
        );
    }
}

/// A first SetValue that fails before its validation lands (a text longer than the field) is not measured, and refused.
#[test]
fn issue_8434_overlong_first_set_value_is_refused() {
    let (r, marks) = probe_test(
        "SecondaryKey_EmptyView_ReadsTheRowTwice",
        replace(
            "        Lines.Name.SetValue('Station');\n        Check('setname', Log.Take());",
            "        asserterror Lines.Name.SetValue('Station Station Station Station');\n        Check('setname', Log.Take());",
        ),
    );
    assert!(
        matches!(&r, Outcome::NotRun(w) if w.contains("ended without the landing")),
        "{r:?} {marks:?}"
    );
}

/// The save snapshots every row of the page's table to check what the code between the save and the update writes; past 1000
/// rows it is not measured, and refused (a table of 1002 rows: 1001 of another group, outside the view).
#[test]
fn issue_8434_large_table_is_refused() {
    let (r, marks) = probe_test("SecondaryKey_EmptyView_ReadsTheRowTwice", |s| {
        let s = replace(
            "        KindValue: Record \"LT NF Value\";\n    begin\n        Line.DeleteAll();",
            "        KindValue: Record \"LT NF Value\";\n        Extra: Integer;\n    begin\n        Line.DeleteAll();",
        )(s);
        replace(
            "        if RowBefore then begin\n            Line.Init();",
            "        for Extra := 1 to 1001 do begin\n            Line.Init();\n            Line.\"Group Code\" := 'G9';\n            Line.\"Entry No.\" := 0;\n            Line.Insert();\n        end;\n        if RowBefore then begin\n            Line.Init();",
        )(&s)
    });
    assert!(
        matches!(&r, Outcome::NotRun(w) if w.contains("more than 1000 rows")),
        "{r:?} {marks:?}"
    );
}

/// Code that is not the probe's, on page 97492, its parts, its table or what they call, keeps main's way with no flag (review of
/// #8463, round 1, finding 1; `code_inserted_pinned.rs` `coded_code`): the review's shapes (a) a part's
/// OnPageBackgroundTaskCompleted that ends with `CurrPage.Update(false)`, (b) `CurrPage.Update(false)` without
/// `CurrPage.Usage.Page.Invalidate()`, (c) a Value OnValidate that inserts a second row after its `CurrPage.Update(false)`, (d)
/// another OnModify body, (e) another SourceTableView; and a page property, a control, a table field, a part trigger, a
/// CaptionClass that calls a procedure, a worker that fails, and writes the probe does not make (another row of the page's
/// table, the row through another record, a part trigger that writes the row). Main refuses each at the landing.
#[test]
fn issue_8434_other_code_keeps_main() {
    let value_validate = "                    trigger OnValidate()\n                    begin\n                        Log.Add('P.VV(' + Rec.Value + ')');";
    let calc_modify =
        "            Line.\"Calc State\" := Line.\"Calc State\"::Processed;\n            Line.Modify(false);";
    let calc_vars = "        Setup: Record \"LT NF Setup\";\n        Log: Codeunit \"LT NF Log\";\n    begin\n        if Line.GetBySystemId";
    let calc_other = "        Setup: Record \"LT NF Setup\";\n        Other: Record \"LT NF Line\";\n        Log: Codeunit \"LT NF Log\";\n    begin\n        if Line.GetBySystemId";
    type Change = Box<dyn Fn(&str) -> String>;
    let cases: Vec<(&str, Change)> = vec![
        (
            "(a) OnPageBackgroundTaskCompleted ends with CurrPage.Update(false)",
            Box::new(replace(
                "        Log.Add('W.DONE(' + UsageText + ')');\n    end;",
                "        Log.Add('W.DONE(' + UsageText + ')');\n        CurrPage.Update(false);\n    end;",
            )),
        ),
        (
            "(b) CurrPage.Update(false) without Invalidate",
            Box::new(replace(
                "        CurrPage.Usage.Page.Invalidate();\n        Log.Add('P.UPD');",
                "        Log.Add('P.UPD');",
            )),
        ),
        (
            "(c) the Value OnValidate inserts a second row after the update",
            Box::new(move |s: &str| {
                let s = replace(
                    value_validate,
                    "                    trigger OnValidate()\n                    var\n                        Second: Record \"LT NF Line\";\n                    begin\n                        Log.Add('P.VV(' + Rec.Value + ')');",
                )(s);
                replace(
                    "                        CalculateAndRefresh();\n                        Log.Add('P.VV.end');",
                    "                        CalculateAndRefresh();\n                        Second.Init();\n                        Second.\"Group Code\" := Rec.\"Group Code\";\n                        Second.Insert(true);\n                        Log.Add('P.VV.end');",
                )(&s)
            }),
        ),
        (
            "(d) another OnModify body",
            Box::new(replace(
                "        Log.Add('T.M(' + Name + '/' + Value + ')');",
                "        Log.Add('T.M(' + Name + '/' + Value + ')');\n        Description := 'modified';",
            )),
        ),
        (
            "(e) SourceTableView sorting(Name)",
            Box::new(replace(
                "    SourceTableView = sorting(\"Sorting\") order(ascending);",
                "    SourceTableView = sorting(Name) order(ascending);",
            )),
        ),
        (
            "DelayedInsert",
            Box::new(replace(
                "    Editable = true;\n    PageType = List;",
                "    Editable = true;\n    DelayedInsert = true;\n    PageType = List;",
            )),
        ),
        (
            "a control",
            Box::new(replace(
                "                field(Description; Rec.Description)",
                "                field(Style; Rec.Style) { ApplicationArea = Basic, Suite; }\n                field(Description; Rec.Description)",
            )),
        ),
        (
            "a table field",
            Box::new(replace(
                "        field(18; \"Old Flag\"; Boolean) { }",
                "        field(18; \"Old Flag\"; Boolean) { }\n        field(20; Extra; Integer) { }",
            )),
        ),
        (
            "a part trigger",
            Box::new(replace(
                "    trigger OnAfterGetRecord()\n    var\n        Change: Record \"LT NF Change\";",
                "    trigger OnOpenPage()\n    begin\n    end;\n\n    trigger OnAfterGetRecord()\n    var\n        Change: Record \"LT NF Change\";",
            )),
        ),
        (
            "a CaptionClass that calls a procedure",
            Box::new(|s: &str| {
                let s = replace(
                    "                    CaptionClass = CustomHeader[1];",
                    "                    CaptionClass = StyleOf();",
                )(s);
                replace(
                    "    local procedure RefreshUsage()",
                    "    local procedure StyleOf(): Text\n    begin\n        exit(Rec.Style);\n    end;\n\n    local procedure RefreshUsage()",
                )(&s)
            }),
        ),
        (
            "a worker that fails",
            Box::new(replace(
                "        Results.Add('Count', 'K' + Format(Kind.Count()));",
                "        if Kind.Count() >= 0 then\n            Error('worker failed');\n        Results.Add('Count', 'K' + Format(Kind.Count()));",
            )),
        ),
        (
            "another row of the page's table",
            Box::new(move |s: &str| {
                replace(calc_vars, calc_other)(&replace(
                    "        Setup.\"Calc Count\" += 1;\n        Setup.Modify();",
                    "        Setup.\"Calc Count\" += 1;\n        Setup.Modify();\n        Other.\"Group Code\" := 'G9';\n        Other.Insert();",
                )(s))
            }),
        ),
        (
            "the row written through another record",
            Box::new(move |s: &str| {
                let s = replace(
                    calc_modify,
                    "            Other.GetBySystemId(Line.SystemId);\n            Other.\"Calc State\" := Other.\"Calc State\"::Processed;\n            Other.Modify(false);",
                )(s);
                replace(calc_vars, calc_other)(&s)
            }),
        ),
        (
            "a part trigger that writes the row",
            Box::new(replace(
                "        RulesText := Format(Change.Count());",
                "        RulesText := Format(Change.Count());\n        if Rec.Description = '' then begin\n            Rec.Description := 'read';\n            Rec.Modify();\n        end;",
            )),
        ),
    ];
    for (name, change) in cases {
        let (r, marks) = probe_test("SecondaryKey_EmptyView_ReadsTheRowTwice", change);
        // Main refuses the landing, or fails the probe's first check already: it does not run the parts' OnAfterGetCurrRecord at
        // the open of a page that is not the page with code.
        let main = matches!(&r, Outcome::NotRun(w) if w.contains(MAIN_REFUSAL))
            || matches!(&r, Outcome::Fail(w) if w.contains("Actual:<open=P.OPEN;>"));
        assert!(
            main && !marks.contains(&ListNewLineCodedFactboxes),
            "{name}: {r:?} {marks:?}"
        );
    }
}
