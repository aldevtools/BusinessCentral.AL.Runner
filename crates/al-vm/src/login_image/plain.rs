//! A [`Value`] that holds no handle, as owned data any thread may hold (the session image, al-runner issue 8211): the plain
//! mirror of `Value` and of a record filter. It lives with its only user rather than next to `Value` in al-builtins: there,
//! its conversions alone, which no run without the login replay calls, changed how al-builtins' hot functions inline and
//! cost every run about 0.1% more instructions.

use al_builtins::filter::{FieldFilter, FilterExpr};
use al_builtins::text::Txt;
use al_builtins::value::{TextConstFrom, Value};
use rust_decimal::Decimal;
use std::sync::Arc;

/// A value that holds no handle, as owned data any thread may hold. One variant per plain `Value` variant.
#[derive(Debug, Clone)]
pub(crate) enum Plain {
    Null,
    Boolean(bool),
    Integer(i32),
    BigInteger(i64),
    Decimal(Decimal),
    Byte(u8),
    Char(u16),
    Text(Txt),
    TextConst(Txt, TextConstFrom),
    Code(Txt),
    Date(i32, bool),
    Time(i64),
    DateTime(i64),
    Duration(i64),
    Guid([u8; 16]),
    DateFormula(String),
    Option(i32, Arc<[String]>),
    Enum(u32, i32),
    Unknown(Arc<str>),
}

impl Plain {
    /// `None` for a value that holds a handle (a Blob included: its buffer is a node of the arena). No `_` arm: a new
    /// `Value` variant is a compile error here until it is classified.
    pub(crate) fn of(v: &Value) -> Option<Plain> {
        Some(match v {
            Value::Null => Plain::Null,
            Value::Boolean(b) => Plain::Boolean(*b),
            Value::Integer(i) => Plain::Integer(*i),
            Value::BigInteger(i) => Plain::BigInteger(*i),
            Value::Decimal(d) => Plain::Decimal(*d),
            Value::Byte(b) => Plain::Byte(*b),
            Value::Char(c) => Plain::Char(*c),
            Value::Text(t) => Plain::Text(t.clone()),
            Value::TextConst(t, from) => Plain::TextConst(t.clone(), *from),
            Value::Code(t) => Plain::Code(t.clone()),
            Value::Date(d, closing) => Plain::Date(*d, *closing),
            Value::Time(t) => Plain::Time(*t),
            Value::DateTime(t) => Plain::DateTime(*t),
            Value::Duration(t) => Plain::Duration(*t),
            Value::Guid(g) => Plain::Guid(*g),
            Value::DateFormula(f) => Plain::DateFormula(f.clone()),
            Value::Option(o, names) => Plain::Option(*o, names.clone()),
            Value::Enum(e, o) => Plain::Enum(*e, *o),
            Value::Unknown(u) => Plain::Unknown(u.clone()),
            Value::List(_)
            | Value::Dictionary(_)
            | Value::Array(_)
            | Value::Codeunit(_)
            | Value::Library(..)
            | Value::Record(_)
            | Value::Blob(_)
            | Value::TestPage(_)
            | Value::TestControl(_)
            | Value::Query(_)
            | Value::DotNet(_)
            | Value::SqlCount(_)
            | Value::PlaceholderSize(..) => return None,
        })
    }

    /// The value this plain value is.
    pub(crate) fn value(&self) -> Value {
        match self {
            Plain::Null => Value::Null,
            Plain::Boolean(b) => Value::Boolean(*b),
            Plain::Integer(i) => Value::Integer(*i),
            Plain::BigInteger(i) => Value::BigInteger(*i),
            Plain::Decimal(d) => Value::Decimal(*d),
            Plain::Byte(b) => Value::Byte(*b),
            Plain::Char(c) => Value::Char(*c),
            Plain::Text(t) => Value::Text(t.clone()),
            Plain::TextConst(t, from) => Value::TextConst(t.clone(), *from),
            Plain::Code(t) => Value::Code(t.clone()),
            Plain::Date(d, closing) => Value::Date(*d, *closing),
            Plain::Time(t) => Value::Time(*t),
            Plain::DateTime(t) => Value::DateTime(*t),
            Plain::Duration(t) => Value::Duration(*t),
            Plain::Guid(g) => Value::Guid(*g),
            Plain::DateFormula(f) => Value::DateFormula(f.clone()),
            Plain::Option(o, names) => Value::Option(*o, names.clone()),
            Plain::Enum(e, o) => Value::Enum(*e, *o),
            Plain::Unknown(u) => Value::Unknown(u.clone()),
        }
    }
}

/// `FilterExpr` with `Plain` in place of `Value` (filter.rs:10-48), one variant per variant.
#[derive(Debug, Clone)]
pub(crate) enum PlainFilterExpr {
    Range(Option<Plain>, Option<Plain>),
    Eq(Plain),
    RangeSame(Plain),
    Ne(Plain),
    Lt(Plain),
    Le(Plain),
    Gt(Plain),
    Ge(Plain),
    Or(Vec<PlainFilterExpr>),
    And(Vec<PlainFilterExpr>),
    Like(String, bool),
    NotLike(String, bool),
    AtSignAlone(bool),
    AtSignAloneCode(bool),
    EmptyViewValue,
    NumCode(Box<PlainFilterExpr>),
    UnpinnedMatch(Box<PlainFilterExpr>, String),
    UnpinnedText(Box<PlainFilterExpr>),
}

impl PlainFilterExpr {
    fn of(e: &FilterExpr) -> Option<PlainFilterExpr> {
        fn opt(v: &Option<Value>) -> Option<Option<Plain>> {
            match v {
                Some(v) => Plain::of(v).map(Some),
                None => Some(None),
            }
        }
        fn all(v: &[FilterExpr]) -> Option<Vec<PlainFilterExpr>> {
            v.iter().map(PlainFilterExpr::of).collect()
        }
        Some(match e {
            FilterExpr::Range(a, b) => PlainFilterExpr::Range(opt(a)?, opt(b)?),
            FilterExpr::Eq(v) => PlainFilterExpr::Eq(Plain::of(v)?),
            FilterExpr::RangeSame(v) => PlainFilterExpr::RangeSame(Plain::of(v)?),
            FilterExpr::Ne(v) => PlainFilterExpr::Ne(Plain::of(v)?),
            FilterExpr::Lt(v) => PlainFilterExpr::Lt(Plain::of(v)?),
            FilterExpr::Le(v) => PlainFilterExpr::Le(Plain::of(v)?),
            FilterExpr::Gt(v) => PlainFilterExpr::Gt(Plain::of(v)?),
            FilterExpr::Ge(v) => PlainFilterExpr::Ge(Plain::of(v)?),
            FilterExpr::Or(v) => PlainFilterExpr::Or(all(v)?),
            FilterExpr::And(v) => PlainFilterExpr::And(all(v)?),
            FilterExpr::Like(p, ci) => PlainFilterExpr::Like(p.clone(), *ci),
            FilterExpr::NotLike(p, ci) => PlainFilterExpr::NotLike(p.clone(), *ci),
            FilterExpr::AtSignAlone(b) => PlainFilterExpr::AtSignAlone(*b),
            FilterExpr::AtSignAloneCode(b) => PlainFilterExpr::AtSignAloneCode(*b),
            FilterExpr::EmptyViewValue => PlainFilterExpr::EmptyViewValue,
            FilterExpr::NumCode(i) => PlainFilterExpr::NumCode(Box::new(PlainFilterExpr::of(i)?)),
            FilterExpr::UnpinnedMatch(i, t) => {
                PlainFilterExpr::UnpinnedMatch(Box::new(PlainFilterExpr::of(i)?), t.clone())
            }
            FilterExpr::UnpinnedText(i) => PlainFilterExpr::UnpinnedText(Box::new(PlainFilterExpr::of(i)?)),
        })
    }

    fn expr(&self) -> FilterExpr {
        fn opt(v: &Option<Plain>) -> Option<Value> {
            v.as_ref().map(Plain::value)
        }
        fn all(v: &[PlainFilterExpr]) -> Vec<FilterExpr> {
            v.iter().map(PlainFilterExpr::expr).collect()
        }
        match self {
            PlainFilterExpr::Range(a, b) => FilterExpr::Range(opt(a), opt(b)),
            PlainFilterExpr::Eq(v) => FilterExpr::Eq(v.value()),
            PlainFilterExpr::RangeSame(v) => FilterExpr::RangeSame(v.value()),
            PlainFilterExpr::Ne(v) => FilterExpr::Ne(v.value()),
            PlainFilterExpr::Lt(v) => FilterExpr::Lt(v.value()),
            PlainFilterExpr::Le(v) => FilterExpr::Le(v.value()),
            PlainFilterExpr::Gt(v) => FilterExpr::Gt(v.value()),
            PlainFilterExpr::Ge(v) => FilterExpr::Ge(v.value()),
            PlainFilterExpr::Or(v) => FilterExpr::Or(all(v)),
            PlainFilterExpr::And(v) => FilterExpr::And(all(v)),
            PlainFilterExpr::Like(p, ci) => FilterExpr::Like(p.clone(), *ci),
            PlainFilterExpr::NotLike(p, ci) => FilterExpr::NotLike(p.clone(), *ci),
            PlainFilterExpr::AtSignAlone(b) => FilterExpr::AtSignAlone(*b),
            PlainFilterExpr::AtSignAloneCode(b) => FilterExpr::AtSignAloneCode(*b),
            PlainFilterExpr::EmptyViewValue => FilterExpr::EmptyViewValue,
            PlainFilterExpr::NumCode(i) => FilterExpr::NumCode(Box::new(i.expr())),
            PlainFilterExpr::UnpinnedMatch(i, t) => FilterExpr::UnpinnedMatch(Box::new(i.expr()), t.clone()),
            PlainFilterExpr::UnpinnedText(i) => FilterExpr::UnpinnedText(Box::new(i.expr())),
        }
    }
}

/// A [`FieldFilter`] whose values are plain.
#[derive(Debug, Clone)]
pub(crate) struct PlainFilter {
    pub(crate) lang: i32,
    pub(crate) field: usize,
    pub(crate) expr: PlainFilterExpr,
    pub(crate) group: i32,
}

impl PlainFilter {
    /// `None` for a filter one of whose values holds a handle.
    pub(crate) fn of(f: &FieldFilter) -> Option<PlainFilter> {
        Some(PlainFilter {
            lang: f.lang,
            field: f.field,
            expr: PlainFilterExpr::of(&f.expr)?,
            group: f.group,
        })
    }

    /// The filter this plain filter is.
    pub(crate) fn filter(&self) -> FieldFilter {
        FieldFilter {
            lang: self.lang,
            field: self.field,
            expr: self.expr.expr(),
            group: self.group,
        }
    }
}

#[cfg(test)]
mod tests;
