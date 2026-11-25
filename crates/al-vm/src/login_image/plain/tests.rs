//! `Plain` and `PlainFilter` read back what they were made from.

use super::*;
use al_builtins::filter::{FieldFilter, FilterExpr};
use al_builtins::text::Txt;
use al_builtins::value::{TextConstFrom, Value};
use rust_decimal::Decimal;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

/// A text whose lone surrogate unit `value_codec` would drop (`value_codec.rs:112-115` writes `as_str()`).
fn lone_text() -> Txt {
    Txt::from_units(&[0x41, 0xD800, 0x42])
}

fn every_plain_value() -> Vec<Value> {
    let names: Arc<[String]> = Arc::from(vec!["A".to_string(), "B".to_string()]);
    vec![
        Value::Null,
        Value::Boolean(true),
        Value::Integer(-7),
        Value::BigInteger(i64::MIN),
        Value::Decimal(Decimal::new(1_2300, 4)),
        Value::Byte(255),
        Value::Char(0xD800),
        Value::Text(Txt::new("é text".into())),
        Value::Text(lone_text()),
        Value::TextConst(Txt::new("label".into()), TextConstFrom::Label),
        Value::TextConst(Txt::new("const".into()), TextConstFrom::TextConst),
        Value::Code(Txt::new("ABC".into())),
        Value::Date(738_000, true),
        Value::Time(43_200_001),
        Value::DateTime(63_000_000_000_001),
        Value::Duration(-5),
        Value::Guid([7; 16]),
        Value::DateFormula("<1M>".into()),
        Value::Option(1, names),
        Value::Enum(50100, 3),
        Value::Unknown(Arc::from("not pinned")),
    ]
}

/// What `Value::identical` cannot see: the lone units of a text.
fn lone_units_of(v: &Value) -> Vec<(u32, u16)> {
    match v {
        Value::Text(t) | Value::Code(t) | Value::TextConst(t, _) => t.lone_units().to_vec(),
        _ => Vec::new(),
    }
}

#[test]
fn every_plain_value_reads_back_identical() {
    for v in every_plain_value() {
        let plain = Plain::of(&v).unwrap_or_else(|| panic!("{v:?} has a plain form"));
        let back = plain.value();
        // `Value::identical` has no arm for a TextConst or an Unknown: its `Debug` (text and declaration) is the exact form.
        let same = match &v {
            Value::TextConst(..) | Value::Unknown(_) => format!("{v:?}") == format!("{back:?}"),
            _ => v.identical(&back),
        };
        assert!(same, "{v:?} read back as {back:?}");
        assert_eq!(lone_units_of(&v), lone_units_of(&back), "lone units of {v:?}");
    }
}

#[test]
fn a_handle_has_no_plain_form() {
    let handles = [
        Value::Array(Rc::new(RefCell::new(vec![Value::Integer(1)]))),
        Value::Dictionary(Rc::new(RefCell::new(Vec::new()))),
    ];
    for v in handles {
        assert!(Plain::of(&v).is_none(), "{v:?} holds a handle");
    }
    // A filter with a handle in a value has none either.
    let with_handle = FieldFilter::new(1, FilterExpr::Eq(Value::Array(Rc::new(RefCell::new(Vec::new())))), 0);
    assert!(PlainFilter::of(&with_handle).is_none());
    let nested = FieldFilter::new(
        1,
        FilterExpr::Or(vec![
            FilterExpr::Eq(Value::Integer(1)),
            FilterExpr::Range(
                Some(Value::Integer(1)),
                Some(Value::Dictionary(Rc::new(RefCell::new(Vec::new())))),
            ),
        ]),
        0,
    );
    assert!(PlainFilter::of(&nested).is_none());
}

fn every_filter_expression() -> Vec<FilterExpr> {
    let t = || Value::Text(Txt::new("a".into()));
    let leaf = FilterExpr::Eq(Value::Integer(1));
    vec![
        FilterExpr::Range(Some(t()), None),
        FilterExpr::Range(None, Some(Value::Integer(9))),
        FilterExpr::Range(None, None),
        FilterExpr::Eq(Value::Text(lone_text())),
        FilterExpr::RangeSame(t()),
        FilterExpr::Ne(t()),
        FilterExpr::Lt(t()),
        FilterExpr::Le(t()),
        FilterExpr::Gt(t()),
        FilterExpr::Ge(t()),
        FilterExpr::Or(vec![leaf.clone(), FilterExpr::Ne(t())]),
        FilterExpr::And(vec![leaf.clone(), FilterExpr::Gt(t())]),
        FilterExpr::Like("a*".into(), true),
        FilterExpr::NotLike("a?".into(), false),
        FilterExpr::AtSignAlone(true),
        FilterExpr::AtSignAlone(false),
        FilterExpr::EmptyViewValue,
        FilterExpr::NumCode(Box::new(leaf.clone())),
        FilterExpr::UnpinnedMatch(Box::new(leaf.clone()), "'@x".into()),
        FilterExpr::UnpinnedText(Box::new(FilterExpr::Or(vec![leaf]))),
    ]
}

#[test]
fn every_filter_expression_reads_back_identical() {
    for (i, expr) in every_filter_expression().into_iter().enumerate() {
        let f = FieldFilter {
            lang: 1031,
            field: i,
            expr,
            group: i as i32 - 3,
        };
        let plain = PlainFilter::of(&f).unwrap_or_else(|| panic!("{f:?} has a plain form"));
        let back = plain.filter();
        assert!(f.identical(&back), "{f:?} read back as {back:?}");
    }
}

#[test]
fn plain_is_send_and_sync() {
    fn both<T: Send + Sync>() {}
    both::<Plain>();
    both::<PlainFilter>();
}
