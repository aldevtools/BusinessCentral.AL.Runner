//! Own-area CaptionClass texts of a page's source table fields (al-runner issues 7410 and 7431, `literal_caption_open.rs` and
//! `own_area_control_caption_pinned` in al-vm): the Card shape record/captions/CaptionClassOwnArea.al:587-633 pins resolves the
//! literal texts at the open, the pages of :315-357 show a subscriber's caption for an expression. Every neighbor keeps main's
//! result (main's answer, or main's refusal) or refuses.

use crate::user_reports::run_with_deps;
use al_builtins::assumptions::Assumption;
use al_test_cli::report::Outcome;
use al_testkit::{blocked, compile};
use al_testrun::isolation::TestIsolation;

const FIXTURE: &str = include_str!("../fixtures/own-area-literal-open.al");

/// What main gives the literal Card test of the fixture: the first read resolves the text, the second reuses it.
const MAIN_LITERAL: &str = "open=none r1=[R[2,1]] ev=(TSTL|2,1) r2=[R[2,1]] ev=none";
/// What the pinned Card gives it (CaptionClassOwnArea.al:622-626).
const PINNED_LITERAL: &str = "open=(TSTL|2,1)(TSTM|2,1) r1=[R[2,1]] ev=none r2=[R[2,1]] ev=none";

const BEFORE_LITERAL_OPEN: &str = "        Log.Take();\n        Card.OpenEdit();\n        Got := 'open='";
const AFTER_LITERAL_OPEN: &str = "        Card.OpenEdit();\n        Got := 'open=' + Log.Take();\n";
const LITERAL_FIRST_READ: &str = "        Got += ' r1=[' + Card.\"Dimension 1\".Caption() + '] ev=' + Log.Take();\n";
const LITERAL_READS: &str = "        Got += ' r1=[' + Card.\"Dimension 1\".Caption() + '] ev=' + Log.Take();\n        Got += ' r2=[' + Card.\"Dimension 1\".Caption() + '] ev=' + Log.Take();\n";
const NOT_SHOWN_FIELD: &str = "field(3; \"Not Shown\"; Decimal) { CaptionClass = 'TSTM,2,1'; }";
const EXPR_CARD_OPEN: &str = "        Log.SetExpr('TSTX,2,1');\n        Card.OpenEdit();\n";
const EXPR_CARD_READ: &str = "        Error('open=%1 cap=[%2]', Log.Take(), Card.Dyn.Caption());\n";
const EXPR_CARD_FIELDS: &str = "            field(Id; Rec.Id) { ApplicationArea = All; }\n            field(Dyn; Rec.Dyn) { ApplicationArea = All; }\n        }\n    }\n}\n\npage 50416";
const EXPR_LIST_SEED: &str = "        Row.Id := 1;\n        Row.Insert();\n        Log.SetExpr('TSTY,2,1');\n";

/// `source` with the one occurrence of `what` replaced by `with`.
fn changed(source: &str, what: &str, with: &str) -> String {
    assert_eq!(source.matches(what).count(), 1, "{what}");
    source.replacen(what, with, 1)
}

/// `source` with `line` inserted after the one occurrence of `what`.
fn after(source: &str, what: &str, line: &str) -> String {
    changed(source, what, &format!("{what}{line}"))
}

/// `source` with `line` inserted before the one occurrence of `what`.
fn before(source: &str, what: &str, line: &str) -> String {
    changed(source, what, &format!("{line}{what}"))
}

fn outcome(source: &str, test: &str) -> Outcome {
    let results = run_with_deps(&[], source);
    results
        .iter()
        .find(|(n, _)| n == test)
        .map(|(_, o)| o.clone())
        .unwrap_or_else(|| panic!("no test {test}: {results:?}"))
}

fn assert_not_run(outcome: &Outcome, reason: &str, shape: &str) {
    assert!(
        matches!(outcome, Outcome::NotRun(why) if why.contains(reason)),
        "{shape}: {outcome:?}"
    );
}

/// The pinned Card (CaptionClassOwnArea.al:587-633, al-runner issue 7410) and the own-area expression on the pinned Card and List
/// (:315-357, al-runner issue 7431).
#[test]
fn own_area_literal_classes_resolve_at_the_open_of_the_pinned_card() {
    let results = run_with_deps(&[], FIXTURE);
    let named = |name: &str| results.iter().find(|(n, _)| n == name).map(|(_, o)| o.clone());
    assert_eq!(named("LiteralReads"), Some(Outcome::Fail(PINNED_LITERAL.into())));
    assert_eq!(
        named("ExpressionRead"),
        Some(Outcome::Fail("open=(TSTX|2,1) cap=[R[2,1]]".into()))
    );
    assert_eq!(
        named("ListExpressionRead"),
        Some(Outcome::Fail(
            "open=(TSTY|2,1) r1=[R[2,1]] ev=none r2=[R[2,1]] ev=none".into()
        ))
    );
}

/// Neighbors of the pinned Card keep main's result: a page or table property, a field that is not a normal enabled field, a
/// table with another row, an empty table, a page that is not the pin's, a manual subscription or another language at the open.
#[test]
fn own_area_literal_classes_outside_the_pinned_card_keep_main_result() {
    let usage = "    UsageCategory = None;\n    layout\n    {\n        area(Content)\n        {\n            field(Id; Rec.Id) { ApplicationArea = All; }\n            field(\"Dimension 1\"";
    let field = |props: &str| {
        changed(
            FIXTURE,
            NOT_SHOWN_FIELD,
            &format!("field(3; \"Not Shown\"; Decimal) {{ CaptionClass = 'TSTM,2,1'; {props} }}"),
        )
    };
    let seed = "        Row.Id := 1;\n        Row.Insert();\n        Log.Take();\n        Card.OpenEdit();";
    let manual = "codeunit 50417 \"Own Area Manual\"\n{\n    EventSubscriberInstance = Manual;\n    [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Caption Class\", 'OnResolveCaptionClass', '', true, true)]\n    local procedure OnResolve(CaptionArea: Text; CaptionExpr: Text; Language: Integer; var Caption: Text; var Resolved: Boolean)\n    begin\n    end;\n}\n";
    let bound = format!(
        "{manual}{}",
        before(FIXTURE, BEFORE_LITERAL_OPEN, "        BindSubscription(Manual);\n").replacen(
            "        Card: TestPage \"Own Area Literal Card\";\n",
            "        Card: TestPage \"Own Area Literal Card\";\n        Manual: Codeunit \"Own Area Manual\";\n",
            1
        )
    );
    for (shape, source) in [
        (
            "Editable = false",
            changed(FIXTURE, usage, &format!("    Editable = false;\n{usage}")),
        ),
        (
            "page Caption",
            changed(FIXTURE, usage, &format!("    Caption = 'Other';\n{usage}")),
        ),
        (
            "empty table",
            changed(
                FIXTURE,
                "        Row.Id := 1;\n        Row.Insert();\n        Log.Take();\n        Card.OpenEdit();",
                "        Log.Take();\n        Card.OpenEdit();",
            ),
        ),
        (
            "two rows",
            changed(
                FIXTURE,
                seed,
                &format!("        Row.Id := 2;\n        Row.Insert();\n{seed}"),
            ),
        ),
        ("Enabled = false", field("Enabled = false;")),
        ("FlowFilter", field("FieldClass = FlowFilter;")),
        (
            "ObsoleteState = Pending",
            field("ObsoleteState = Pending; ObsoleteReason = 'x';"),
        ),
        ("field Caption", field("Caption = 'Shown';")),
        (
            "table property",
            changed(
                FIXTURE,
                "table 50411 \"Own Area Literal Row\"\n{\n    DataClassification = SystemMetadata;\n",
                "table 50411 \"Own Area Literal Row\"\n{\n    DataClassification = SystemMetadata;\n    DataPerCompany = false;\n",
            ),
        ),
        ("manual subscription bound before the open", bound),
        (
            "GlobalLanguage 1031 before the open",
            before(FIXTURE, BEFORE_LITERAL_OPEN, "        GlobalLanguage(1031);\n"),
        ),
    ] {
        assert_eq!(
            outcome(&source, "LiteralReads"),
            Outcome::Fail(MAIN_LITERAL.into()),
            "{shape}"
        );
    }
}

/// OpenView as the first open of the Card raises at the open too (record/captions/WcExtFieldAfterResolve.al:391-413
/// WcPageOpen_UndisplayedFieldOfTheSameTable `log-open`; CaptionClassOwnArea.al:610-628 OpenView as the second open).
#[test]
fn own_area_literal_classes_resolve_at_a_first_open_view_of_the_pinned_card() {
    let source = changed(
        FIXTURE,
        "        Card.OpenEdit();\n        Got",
        "        Card.OpenView();\n        Got",
    );
    assert_eq!(outcome(&source, "LiteralReads"), Outcome::Fail(PINNED_LITERAL.into()));
}

/// An open whose translation would refuse keeps main's open, which translates nothing: a text the session translated under
/// another language, and a subscriber of UI Helper Triggers' CaptionClassTranslate. The test reads no caption.
#[test]
fn own_area_literal_open_that_would_refuse_keeps_main_open() {
    let id_only = changed(
        FIXTURE,
        LITERAL_READS,
        "        Got += ' id=' + Format(Card.Id.AsInteger());\n",
    );
    let other_language = before(
        &id_only,
        "        Log.Take();\n        Card.OpenEdit();",
        "        GlobalLanguage(1031);\n        Got := 'fc=[' + Row.FieldCaption(\"Dimension 1\") + '] ev=' + Log.Take();\n        GlobalLanguage(1033);\n",
    )
    .replacen("        Got := 'open=' + Log.Take();\n", "        Got += ' open=' + Log.Take();\n", 1);
    assert_eq!(
        outcome(&other_language, "LiteralReads"),
        Outcome::Fail("fc=[R[2,1]] ev=(TSTL|2,1) open=none id=1".into())
    );
    let ui_helper = format!(
        "codeunit 50418 \"Own Area UI Sub\" {{ [EventSubscriber(ObjectType::Codeunit, Codeunit::\"UI Helper Triggers\", 'CaptionClassTranslate', '', false, false)] local procedure OnTranslate(Language: Integer; CaptionExpr: Text[1024]; var Caption: Text[1024]) begin end; }}\n{}",
        id_only.replacen(
            "        Card: TestPage \"Own Area Literal Card\";\n",
            "        Card: TestPage \"Own Area Literal Card\";\n        UiSub: Codeunit \"Own Area UI Sub\";\n",
            1
        )
    );
    assert_eq!(
        outcome(&ui_helper, "LiteralReads"),
        Outcome::Fail("open=none id=1".into())
    );
}

/// After the pinned open, `Caption()` reads the open's translation only after the pinned calls (CaptionClassOwnArea.al:599-608:
/// Caption, Caption, SetValue, Caption, Close). Any other call before the read, a failed SetValue, a language or binding change
/// and a translation the session dropped are not measured: refused.
#[test]
fn own_area_literal_read_after_calls_the_pin_does_not_have_refuses() {
    let unbound = "codeunit 50417 \"Own Area Manual\"\n{\n    EventSubscriberInstance = Manual;\n    [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Caption Class\", 'OnResolveCaptionClass', '', true, true)]\n    local procedure OnResolve(CaptionArea: Text; CaptionExpr: Text; Language: Integer; var Caption: Text; var Resolved: Boolean)\n    begin\n    end;\n}\n";
    let with_manual = |src: String| {
        format!(
            "{unbound}{}",
            src.replacen(
                "        Card: TestPage \"Own Area Literal Card\";\n",
                "        Card: TestPage \"Own Area Literal Card\";\n        Manual: Codeunit \"Own Area Manual\";\n",
                1
            )
        )
    };
    for (shape, source) in [
        (
            "Next before the first read",
            after(FIXTURE, AFTER_LITERAL_OPEN, "        Card.Next();\n"),
        ),
        (
            "Next between the reads",
            after(FIXTURE, LITERAL_FIRST_READ, "        Card.Next();\n"),
        ),
        (
            "failed SetValue before the read",
            after(
                FIXTURE,
                AFTER_LITERAL_OPEN,
                "        asserterror Card.\"Dimension 1\".SetValue('abc');\n",
            ),
        ),
        (
            "SetValue before the first read",
            after(
                FIXTURE,
                AFTER_LITERAL_OPEN,
                "        Card.\"Dimension 1\".SetValue(5);\n",
            ),
        ),
        (
            "filter before the read",
            after(FIXTURE, AFTER_LITERAL_OPEN, "        Card.Filter.SetFilter(Id, '1');\n"),
        ),
        (
            "GlobalLanguage after the open",
            after(FIXTURE, AFTER_LITERAL_OPEN, "        GlobalLanguage(1031);\n"),
        ),
        (
            "BindSubscription after the open",
            with_manual(after(
                FIXTURE,
                AFTER_LITERAL_OPEN,
                "        BindSubscription(Manual);\n",
            )),
        ),
        (
            "SelectLatestVersion after the open",
            after(FIXTURE, AFTER_LITERAL_OPEN, "        SelectLatestVersion();\n"),
        ),
    ] {
        assert_not_run(
            &outcome(&source, "LiteralReads"),
            "Caption() of a field CaptionClass the page resolved at the open",
            shape,
        );
    }
}

/// An own-area expression on a page or after calls outside the pins keeps main's refusal: a Card opened with OpenView, a Card
/// with a group, a table trigger, a page Caption, a Next before the read; a List with two rows or opened with OpenEdit.
#[test]
fn own_area_expression_caption_outside_the_pinned_pages_keeps_the_refusal() {
    let list_second_row = "        Row.Id := 2;\n        Row.Insert();\n";
    for (shape, test, source) in [
        (
            "Card OpenView",
            "ExpressionRead",
            changed(
                FIXTURE,
                EXPR_CARD_OPEN,
                "        Log.SetExpr('TSTX,2,1');\n        Card.OpenView();\n",
            ),
        ),
        (
            "Card with a group",
            "ExpressionRead",
            changed(
                FIXTURE,
                EXPR_CARD_FIELDS,
                "            group(G)\n            {\n                field(Id; Rec.Id) { ApplicationArea = All; }\n                field(Dyn; Rec.Dyn) { ApplicationArea = All; }\n            }\n        }\n    }\n}\n\npage 50416",
            ),
        ),
        (
            "table trigger",
            "ExpressionRead",
            changed(
                FIXTURE,
                "    local procedure DynCc(): Text",
                "    trigger OnModify()\n    begin\n    end;\n\n    local procedure DynCc(): Text",
            ),
        ),
        (
            "page Caption",
            "ExpressionRead",
            changed(
                FIXTURE,
                "    SourceTable = \"Own Area Expr Row\";\n    UsageCategory = None;\n    layout\n    {\n        area(Content)\n        {\n            field(Id",
                "    SourceTable = \"Own Area Expr Row\";\n    UsageCategory = None;\n    Caption = 'Other';\n    layout\n    {\n        area(Content)\n        {\n            field(Id",
            ),
        ),
        (
            "Next before the read",
            "ExpressionRead",
            before(FIXTURE, EXPR_CARD_READ, "        Card.Next();\n"),
        ),
        (
            "List with two rows",
            "ListExpressionRead",
            before(FIXTURE, EXPR_LIST_SEED, list_second_row),
        ),
        (
            "List OpenEdit",
            "ListExpressionRead",
            changed(FIXTURE, "        List.OpenView();\n", "        List.OpenEdit();\n"),
        ),
    ] {
        assert_not_run(&outcome(&source, test), "al-runner issue 7431", shape);
    }
}

/// The open of a page whose CaptionClass expressions leave nothing behind (al-runner issue 7431's Card,
/// record/captions/CaptionClassOwnArea.al:543-583, two expression fields with different texts) runs the empty-record runs with no
/// reader of their count: the pass is not flagged with `page-open-caption-class-empty-record-count`.
#[test]
fn own_area_expression_open_whose_runs_leave_nothing_is_not_flagged() {
    let source = changed(
        FIXTURE,
        "        field(2; Dyn; Decimal) { Caption = 'Dyn Caption'; CaptionClass = DynCc(); }\n",
        "        field(2; Dyn; Decimal) { Caption = 'Dyn Caption'; CaptionClass = DynCc(); }\n        field(3; Dyn2; Decimal) { Caption = 'Dyn2 Caption'; CaptionClass = 'TSTZ,' + DynCc(); }\n",
    );
    let l = compile(&source);
    let engine = al_vm::Engine::new(l.program.clone(), blocked(&l));
    let oi = l
        .program
        .objects
        .iter()
        .position(|o| o.name == "T")
        .expect("codeunit T");
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
    let (_, got) = outcomes
        .iter()
        .find(|(n, _)| n == "ExpressionRead")
        .expect("ExpressionRead");
    assert_eq!(
        format!("{got:?}"),
        format!("{:?}", Outcome::Fail("open=(TSTX|2,1) cap=[R[2,1]]".into()))
    );
    let flagged = hits.iter().any(|(n, h): &(String, Vec<Assumption>)| {
        n == "ExpressionRead" && h.contains(&Assumption::PageOpenCaptionClassEmptyRecordCount)
    });
    assert!(!flagged, "{hits:?}");
}

/// A subscriber's caption the pinned Card filled at the open is read only after the pinned test calls: a `Next` before the read
/// refuses at the read (`refuse_own_area_expression_read`).
#[test]
fn own_area_expression_read_after_a_next_refuses_at_the_read() {
    assert_not_run(
        &outcome(
            &before(FIXTURE, EXPR_CARD_READ, "        Card.Next();\n"),
            "ExpressionRead",
        ),
        "whose own-area CaptionClass expression a subscriber resolved",
        "Next before the read",
    );
}
