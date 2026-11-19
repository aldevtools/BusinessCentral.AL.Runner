//! The corpus probe of al-runner issue 8434 (corpus pull request 1454, open: tests/al-language/pages/currpage/
//! ListNewLineFactboxesCode.al, measured on BC 29.1, 11 of 11 green) and the reporter's repro, run here until a bump pins the
//! probe: a List page with code, control expressions, actions and four FactBoxes, whose new line the table field's OnValidate
//! inserts before the control's OnValidate calls `CurrPage.SaveRecord()` and `CurrPage.Update(false)`
//! (`al-vm` pages/code_inserted_coded.rs). Delete this file and the fixtures when the pin is in the corpus.

use al_builtins::assumptions::Assumption::{self, ListNewLineCodedFactboxes};
use al_test_cli::report::Outcome;

const PROBE: &str = include_str!("../fixtures/outside-fb/Issue8434Probe.al");

/// A stand-in for the System Application's codeunit the probe's OnOpenPage calls: false, as on the OnPrem container the probe
/// was measured on.
const ENVIRONMENT_INFORMATION: &str = "codeunit 457 \"Environment Information\"\n{\n    procedure IsSaaSInfrastructure(): Boolean\n    begin\n        exit(false);\n    end;\n}\n";

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

/// The reporter's repro reaches the page's Where-Used part a second time after the Value step: its background task counts the
/// rows the step changed again, which BC answers from the query cache (al-runner issue 8467, `al-vm` query_cache.rs).
#[test]
fn issue_8434_repro_passes_through_the_query_cache() {
    let o = outcomes(
        include_str!("../fixtures/outside-fb/Issue8434.al"),
        "Repro Attr Line Tests",
    );
    let [(_, r, marks)] = o.as_slice() else { panic!("{o:?}") };
    assert_eq!(*r, Outcome::Pass);
    assert!(
        marks.contains(&ListNewLineCodedFactboxes) && marks.contains(&Assumption::BackgroundTaskQueryCache),
        "{marks:?}"
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

/// A first SetValue that fails before its validation lands (a text longer than the field) and a background task whose worker
/// fails are not measured, and refused.
#[test]
fn issue_8434_unmeasured_failures_are_refused() {
    for (name, change, why) in [
        (
            "an overlong first SetValue",
            replace(
                "        Lines.Name.SetValue('Station');\n        Check('setname', Log.Take());",
                "        asserterror Lines.Name.SetValue('Station Station Station Station');\n        Check('setname', Log.Take());",
            ),
            "ended without the landing",
        ),
        (
            "a failing worker",
            replace(
                "        Results.Add('Count', 'K' + Format(Kind.Count()));",
                "        if Kind.Count() >= 0 then\n            Error('worker failed');\n        Results.Add('Count', 'K' + Format(Kind.Count()));",
            ),
            "a failing worker of a background task on a List page with code",
        ),
    ] {
        let (r, marks) = probe_test("SecondaryKey_EmptyView_ReadsTheRowTwice", change);
        assert!(
            matches!(&r, Outcome::NotRun(w) if w.contains(why)),
            "{name}: {r:?} {marks:?}"
        );
    }
}

/// Between the save and the update the code may write other tables, and the row itself when it reads Rec again; a write to
/// another row of the page's table, and a row written while Rec still holds the old one, are refused. A part trigger that writes
/// the row at the refresh's reads is refused too.
#[test]
fn issue_8434_unmeasured_writes_are_refused() {
    let calc_modify =
        "            Line.\"Calc State\" := Line.\"Calc State\"::Processed;\n            Line.Modify(false);";
    type Change = Box<dyn Fn(&str) -> String>;
    let cases: [(&str, Change, &str); 2] = [
        (
            "another row of the page's table",
            Box::new(replace(
                "        Setup.\"Calc Count\" += 1;\n        Setup.Modify();",
                "        Setup.\"Calc Count\" += 1;\n        Setup.Modify();\n        Other.\"Group Code\" := 'G9';\n        Other.Insert();",
            )),
            "changed between CurrPage.SaveRecord",
        ),
        (
            "the row written through another record, Rec not read again",
            Box::new(move |s: &str| {
                replace(
                    "        if Line.GetBySystemId(Line.SystemId) then;\n    end;",
                    "    end;",
                )(&replace(
                    calc_modify,
                    "            Other.GetBySystemId(Line.SystemId);\n            Other.\"Calc State\" := Other.\"Calc State\"::Processed;\n            Other.Modify(false);",
                )(s))
            }),
            "changed between CurrPage.SaveRecord",
        ),
    ];
    for (name, change, why) in cases {
        let declare = |s: &str| {
            replace(
                "        Setup: Record \"LT NF Setup\";\n        Log: Codeunit \"LT NF Log\";\n    begin\n        if Line.GetBySystemId",
                "        Setup: Record \"LT NF Setup\";\n        Other: Record \"LT NF Line\";\n        Log: Codeunit \"LT NF Log\";\n    begin\n        if Line.GetBySystemId",
            )(&change(s))
        };
        let (r, _) = probe_test("SecondaryKey_EmptyView_ReadsTheRowTwice", declare);
        assert!(matches!(&r, Outcome::NotRun(w) if w.contains(why)), "{name}: {r:?}");
    }
    let (r, _) = probe_test(
        "SecondaryKey_EmptyView_ReadsTheRowTwice",
        replace(
            "        RulesText := Format(Change.Count());",
            "        RulesText := Format(Change.Count());\n        if Rec.Description = '' then begin\n            Rec.Description := 'read';\n            Rec.Modify();\n        end;",
        ),
    );
    assert!(
        matches!(&r, Outcome::NotRun(w) if w.contains("wrote or changed Rec when the refresh read")),
        "{r:?}"
    );
}

/// A fact the probe's pages do not have keeps main's way (no flag): a page property, a CaptionClass that calls a procedure
/// (a field's Visible, Editable and StyleExpr cannot: AL0322), and a part page trigger.
#[test]
fn issue_8434_unmeasured_page_facts_keep_main() {
    for (name, change) in [
        (
            "DelayedInsert",
            replace(
                "    Editable = true;\n    PageType = List;",
                "    Editable = true;\n    DelayedInsert = true;\n    PageType = List;",
            ),
        ),
        (
            "a CaptionClass that calls a procedure",
            replace(
                "                    CaptionClass = CustomHeader[1];",
                "                    CaptionClass = StyleOf();",
            ),
        ),
        (
            "a part trigger",
            replace(
                "    trigger OnAfterGetRecord()\n    var\n        Change: Record \"LT NF Change\";",
                "    trigger OnOpenPage()\n    begin\n    end;\n\n    trigger OnAfterGetRecord()\n    var\n        Change: Record \"LT NF Change\";",
            ),
        ),
    ] {
        let style_of = |s: &str| {
            let s = change(s);
            replace(
                "    local procedure RefreshUsage()",
                "    local procedure StyleOf(): Text\n    begin\n        exit(Rec.Style);\n    end;\n\n    local procedure RefreshUsage()",
            )(&s)
        };
        let (r, marks) = probe_test("SecondaryKey_EmptyView_ReadsTheRowTwice", style_of);
        assert!(
            r != Outcome::Pass && !marks.contains(&ListNewLineCodedFactboxes),
            "{name}: {r:?} {marks:?}"
        );
    }
}
