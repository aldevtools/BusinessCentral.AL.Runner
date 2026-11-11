//! Code fields the database stores as a number (`SQLDataType` Integer, BigInteger, Variant): their key order and the filters on
//! them. Pinned by the corpus writer's probe (needs corpus bump) tests/al-language/record/system-fields/ Y82PkTests.al
//! (`Pk*_NumericOrder`, `Pk*_Blank`, `Pk*_Mixed`, `Pk*_FilterLetters_*`) and Y82SecTests.al (`Sec*_NumericOrder`).

use crate::*;
use al_builtins::filter::{FieldFilter, FilterExpr as E, code_number};
use std::borrow::Cow;
use std::cmp::Ordering;
use std::sync::Arc;

/// The order of two values of a field stored as `s`. `Integer` and `BigInteger`: by number, the empty value first. `Variant`:
/// the empty value, then the texts that are no number, then the numbers (Y82PkTests.al `PkVnt_Mixed`: blank, 1A, A, A1, B, 9, 10).
pub fn cmp_code(s: CodeStorage, a: &Value, b: &Value) -> Ordering {
    let text_order = || al_builtins::ops::compare_key_value(a, b).unwrap_or(Ordering::Equal);
    let (Value::Code(x), Value::Code(y)) = (a, b) else {
        return text_order();
    };
    let class = |t: &str| -> u8 {
        if t.is_empty() {
            0
        } else if is_number(t) {
            2
        } else {
            1
        }
    };
    match s {
        CodeStorage::Integer | CodeStorage::BigInteger if is_number(x) && is_number(y) => by_number(x, y),
        CodeStorage::Variant => match class(x).cmp(&class(y)) {
            Ordering::Equal if class(x) == 2 => by_number(x, y),
            Ordering::Equal => text_order(),
            o => o,
        },
        _ => text_order(),
    }
}

fn is_number(t: &str) -> bool {
    !t.is_empty() && t.bytes().all(|c| c.is_ascii_digit())
}

fn by_number(x: &str, y: &str) -> Ordering {
    let (x, y) = (x.trim_start_matches('0'), y.trim_start_matches('0'));
    x.len().cmp(&y.len()).then_with(|| x.cmp(y))
}

/// How the fields of a primary key are stored, when any of them is not text; `None` for the usual table (no allocation).
pub fn pk_storage(t: &TableDef) -> Option<Arc<[CodeStorage]>> {
    t.keys[0]
        .iter()
        .any(|&i| !matches!(t.fields[i].code_storage, CodeStorage::Text))
        .then(|| t.keys[0].iter().map(|&i| t.fields[i].code_storage).collect())
}

/// `s` as one byte, for a table's key order written with its rows ([`storage_from_byte`] reads it back).
pub fn storage_byte(s: CodeStorage) -> u8 {
    match s {
        CodeStorage::Text => 0,
        CodeStorage::Integer => 1,
        CodeStorage::BigInteger => 2,
        CodeStorage::Variant => 3,
        CodeStorage::Other => 4,
    }
}

/// The storage [`storage_byte`] wrote; `None` for any other byte.
pub fn storage_from_byte(b: u8) -> Option<CodeStorage> {
    Some(match b {
        0 => CodeStorage::Text,
        1 => CodeStorage::Integer,
        2 => CodeStorage::BigInteger,
        3 => CodeStorage::Variant,
        4 => CodeStorage::Other,
        _ => return None,
    })
}

/// The order of two primary key tuples of a table whose key fields are stored as `storage`.
pub fn cmp_pk(storage: &[CodeStorage], a: &[Value], b: &[Value]) -> Ordering {
    for ((x, y), s) in a.iter().zip(b).zip(storage) {
        match cmp_code(*s, x, y) {
            Ordering::Equal => continue,
            o => return o,
        }
    }
    a.len().cmp(&b.len())
}

/// A filter expression for a field stored as a number: the bounds are numbers and `NumCode` compares the number the cell is. For
/// Integer and BigInteger a bound that is no number (`A..B`, `1A`) matches no row (Y82PkTests.al `Pk*_FilterLetters_*`). For
/// Variant a text bound is kept as text, so an equality finds the text cells; any other comparison with a text bound is refused
/// (where a text stands to a number is pinned only by the order of the key).
fn map_expr(e: &E, field: &str, variant: bool) -> Result<E, StoreError> {
    let refuse = |what: &str| {
        StoreError::Refused(format!(
            "{what} on the Code field {field}, which the database stores as a number: not pinned"
        ))
    };
    let bound = |v: &Value, eq: bool| -> Result<Option<Value>, StoreError> {
        match v {
            // The empty value is the number 0 (Y82SecTests.al `SecFInt_NumericOrder`: `<10` and `..9` keep it, `<>''` drops it).
            Value::Code(s) | Value::Text(s) if s.trim().is_empty() => Ok(Some(Value::BigInteger(0))),
            Value::Code(_) | Value::Text(_) => match code_number(v) {
                Some(n) => Ok(Some(Value::BigInteger(n))),
                None if variant && eq => Ok(Some(v.clone())),
                None if variant => Err(refuse("a comparison with a text that is no number")),
                None => Ok(None),
            },
            _ => Ok(None),
        }
    };
    let never = || E::Or(vec![]);
    let one = |v: &Value, wrap: fn(Value) -> E, eq: bool| -> Result<E, StoreError> {
        Ok(match bound(v, eq)? {
            Some(n) => E::NumCode(Box::new(wrap(n))),
            None => never(),
        })
    };
    Ok(match e {
        E::Eq(v) => one(v, E::Eq, true)?,
        E::RangeSame(v) => one(v, E::RangeSame, true)?,
        E::Ne(v) => match bound(v, false)? {
            Some(n) => E::NumCode(Box::new(E::Ne(n))),
            // Every cell differs from a bound that is no number.
            None => E::And(vec![]),
        },
        E::Lt(v) => one(v, E::Lt, false)?,
        E::Le(v) => one(v, E::Le, false)?,
        E::Gt(v) => one(v, E::Gt, false)?,
        E::Ge(v) => one(v, E::Ge, false)?,
        E::Range(lo, hi) => {
            let same =
                matches!((lo, hi), (Some(a), Some(b)) if al_builtins::ops::compare(a, b).ok() == Some(Ordering::Equal));
            // A Variant range of two texts keeps the texts of the range (Y82SecTests.al `SecFVnt_Mixed` `range A..B`).
            let texts = |v: &Option<Value>| {
                matches!(v, Some(Value::Code(_) | Value::Text(_))) && v.as_ref().and_then(code_number).is_none()
            };
            let same = same || (variant && texts(lo) && texts(hi));
            let lo = lo.as_ref().map(|v| bound(v, same)).transpose()?;
            let hi = hi.as_ref().map(|v| bound(v, same)).transpose()?;
            match (lo, hi) {
                (Some(None), _) | (_, Some(None)) => never(),
                (lo, hi) => E::NumCode(Box::new(E::Range(lo.flatten(), hi.flatten()))),
            }
        }
        E::UnpinnedText(inner) => E::UnpinnedText(Box::new(map_expr(inner, field, variant)?)),
        E::Or(xs) => E::Or(
            xs.iter()
                .map(|x| map_expr(x, field, variant))
                .collect::<Result<_, _>>()?,
        ),
        E::And(xs) => E::And(
            xs.iter()
                .map(|x| map_expr(x, field, variant))
                .collect::<Result<_, _>>()?,
        ),
        // Wildcards read the text, which the cell is.
        E::Like(..) | E::NotLike(..) | E::AtSignAlone(_) | E::EmptyViewValue | E::NumCode(_) | E::UnpinnedMatch(..) => {
            e.clone()
        }
    })
}

/// Whether `e` compares a number (a wildcard filter alone does not).
fn compares_numbers(e: &E) -> bool {
    match e {
        E::Or(xs) | E::And(xs) => xs.iter().any(compares_numbers),
        E::UnpinnedText(inner) => compares_numbers(inner),
        E::Like(..) | E::NotLike(..) | E::AtSignAlone(_) | E::EmptyViewValue | E::UnpinnedMatch(..) => false,
        _ => true,
    }
}

/// Whether `e` can match a value below every number (`<10`, `<>5`, `..9`).
fn reaches_texts(e: &E) -> bool {
    match e {
        E::Lt(_) | E::Le(_) | E::Ne(_) | E::Range(None, _) => true,
        E::Or(xs) | E::And(xs) => xs.iter().any(reaches_texts),
        E::UnpinnedText(inner) => reaches_texts(inner),
        _ => false,
    }
}

/// The filters of a read, with the ones on a number-stored Code field made to compare numbers. `rows` are the table's rows: a
/// comparison of numbers is answered only while every value of the field is a number, because how an empty value (or, for a
/// Variant, a text) stands to a bound is not pinned.
pub fn number_filters<'a>(
    t: &TableDef,
    filters: &'a [FieldFilter],
    rows: &mut dyn Iterator<Item = &Row>,
) -> Result<Cow<'a, [FieldFilter]>, StoreError> {
    let stored = |f: &FieldFilter| t.fields.get(f.field).map_or(CodeStorage::Text, |d| d.code_storage);
    let numeric = |f: &FieldFilter| {
        matches!(
            stored(f),
            CodeStorage::Integer | CodeStorage::BigInteger | CodeStorage::Variant
        )
    };
    if !filters.iter().any(|f| numeric(f) && compares_numbers(&f.expr)) {
        return Ok(Cow::Borrowed(filters));
    }
    let mut mapped = Vec::with_capacity(filters.len());
    for f in filters {
        if numeric(f) && compares_numbers(&f.expr) {
            mapped.push(FieldFilter {
                lang: al_builtins::filter::filter_language(),
                field: f.field,
                expr: map_expr(&f.expr, &t.fields[f.field].name, stored(f) == CodeStorage::Variant)?,
                group: f.group,
            });
        } else {
            mapped.push(f.clone());
        }
    }
    // A Variant orders its texts below its numbers: a filter that reaches down (`<10`, `<>5`, `..9`) would keep the texts. Whether it
    // does is not pinned, so it is answered only while no row holds a text there.
    let fields: Vec<usize> = filters
        .iter()
        .filter(|f| stored(f) == CodeStorage::Variant && reaches_texts(&f.expr))
        .map(|f| f.field)
        .collect();
    if !fields.is_empty() {
        for row in rows {
            for &i in &fields {
                if matches!(&row[i], Value::Code(c) if !c.is_empty() && !is_number(c)) {
                    return Err(StoreError::Refused(format!(
                        "a comparison that reaches below the numbers on the Variant Code field {}, while a row holds a text that is no number there: not pinned",
                        t.fields[i].name
                    )));
                }
            }
        }
    }
    Ok(Cow::Owned(mapped))
}
