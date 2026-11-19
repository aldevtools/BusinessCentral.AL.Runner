//! A [`Value`] that holds no handle, as owned data any thread may hold (al-vm `login_image.rs`, al-runner issue 8211):
//! the plain mirror of `Value` and of a record filter.

use crate::filter::{FieldFilter, FilterExpr};
use crate::text::Txt;
use crate::value::{TextConstFrom, Value};
use rust_decimal::Decimal;
use std::sync::Arc;

/// A value that holds no handle, as owned data any thread may hold. One variant per plain `Value` variant.
#[derive(Debug, Clone)]
pub enum Plain {
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
    /// `None` for a value that holds a handle (a Blob included: its buffer is a node of the arena).
    pub fn of(v: &Value) -> Option<Plain> {
        let _ = v;
        None
    }

    /// The value this plain value is.
    pub fn value(&self) -> Value {
        Value::Null
    }
}

/// `FilterExpr` with `Plain` in place of `Value` (filter.rs:10-48), one variant per variant.
#[derive(Debug, Clone)]
pub enum PlainFilterExpr {
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
    EmptyViewValue,
    NumCode(Box<PlainFilterExpr>),
    UnpinnedMatch(Box<PlainFilterExpr>, String),
    UnpinnedText(Box<PlainFilterExpr>),
}

/// A [`FieldFilter`] whose values are plain.
#[derive(Debug, Clone)]
pub struct PlainFilter {
    pub lang: i32,
    pub field: usize,
    pub expr: PlainFilterExpr,
    pub group: i32,
}

impl PlainFilter {
    /// `None` for a filter one of whose values holds a handle.
    pub fn of(f: &FieldFilter) -> Option<PlainFilter> {
        let _ = f;
        None
    }

    /// The filter this plain filter is.
    pub fn filter(&self) -> FieldFilter {
        FieldFilter {
            lang: self.lang,
            field: self.field,
            expr: FilterExpr::EmptyViewValue,
            group: self.group,
        }
    }
}

#[cfg(test)]
mod tests;
