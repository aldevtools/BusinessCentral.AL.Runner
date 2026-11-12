//! `<`, `>`, `<=`, `>=` and range ends of a text that holds a marker (ADR-040 amendment 2026-11-12, A5 and loss mode L13).
//!
//! BC orders text by culture, not by code point (types/TestFormatEvaluateTextContracts.al
//! Operators_TextComparison_IsCultureOrderAndCaseSensitiveEquality), so the ordinal answer from the known characters before a
//! mark would be wrong: `('Body ' + Mark) < 'body'` is false on BC. Every ordered comparison with a marked operand refuses,
//! for every mark, with the mark at the start and after a known prefix that differs. `=` and `<>` keep their answers.

use std::cmp::Ordering;

use al_builtins::native::{CALL_STACK_MARK, CARRIED_TEXT_MARK, SQL_COUNT_TEXT_MARK, Signal};
use al_builtins::ops::{compare, compare_order, compare_range_end};
use al_builtins::value::Value;

fn marks() -> Vec<char> {
    vec![
        CALL_STACK_MARK,
        CARRIED_TEXT_MARK,
        SQL_COUNT_TEXT_MARK,
    ]
}

fn text(s: &str) -> Value {
    Value::Text(s.to_string().into())
}

fn code(s: &str) -> Value {
    Value::Code(s.to_string().into())
}

fn refused(r: Result<Ordering, Signal>, what: &str) {
    match r {
        Err(Signal::Unsupported(_)) => {}
        other => panic!("{what}: expected a refusal, got {other:?}"),
    }
}

#[test]
fn an_ordered_comparison_of_a_marked_text_refuses_for_every_mark() {
    for m in marks() {
        let at_start = text(&m.to_string());
        let after_prefix = text(&format!("Body {m}"));
        for (l, r, what) in [
            (&at_start, &text(""), "mark against ''"),
            (&text(""), &at_start, "'' against mark"),
            (&after_prefix, &text("body"), "differing prefix"),
            (&text("body"), &after_prefix, "differing prefix, right"),
            (&after_prefix, &code("BODY"), "against a Code"),
            (&after_prefix, &at_start, "two marked texts"),
        ] {
            refused(compare_order(l, r), &format!("{m:?} compare_order {what}"));
            refused(compare_range_end(l, r), &format!("{m:?} range end {what}"));
        }
    }
}

#[test]
fn equality_of_a_marked_text_keeps_its_answers() {
    for m in marks() {
        let after_prefix = text(&format!("Body {m}"));
        assert_eq!(compare(&after_prefix, &text("")).unwrap(), Ordering::Greater);
        assert_ne!(compare(&after_prefix, &text("body")).unwrap(), Ordering::Equal);
    }
}

#[test]
fn an_ordered_comparison_of_unmarked_texts_keeps_the_culture_order() {
    assert_eq!(compare_order(&text("Body x"), &text("body")).unwrap(), Ordering::Greater);
    assert_eq!(compare_order(&code("9"), &code("10")).unwrap(), Ordering::Less);
}
