//! The `Integer` table (2000000026): one row per whole number, `Number` from -1000000000 to
//! 1000000000, computed on read.
//!
//! Pinned by corpus record/TestIntegerVirtualTable.al: the bounds (lines 246-260: `Get` answers
//! true at +/-1000000000 and false one past), ranges far from zero (110-150), open-ended filters
//! (152-181), the union of several ranges (184-243), an inverted range (96-106).

use super::{blank, pos, ranges, refused};
use al_builtins::filter::FieldFilter;
use al_builtins::value::Value;
use al_data::{Row, Scan, StoreError, TableDef};

const NUMBER: u32 = 1;
const LOW: i64 = -1_000_000_000;
const HIGH: i64 = 1_000_000_000;
/// The most candidate rows a read examines for a filter on another column.
const SEARCH_LIMIT: i64 = 20_000_000;

fn num(v: &Value) -> Option<i64> {
    match v {
        Value::Integer(i) => Some(i64::from(*i)),
        Value::BigInteger(i) => Some(*i),
        Value::Decimal(d) => d.to_string().parse::<i64>().ok(),
        _ => None,
    }
}

fn row(t: &TableDef, n: i64) -> Row {
    let mut r = blank(t);
    r[pos(t, NUMBER)] = Value::Integer(n as i32);
    r
}

fn set(t: &TableDef, filters: &[FieldFilter]) -> Result<ranges::Set, StoreError> {
    ranges::of_field(filters, pos(t, NUMBER), &num, LOW, HIGH).map_err(|e| refused(format!("Integer: {e}")))
}

pub(super) fn get(t: &TableDef, pk: &[Value]) -> Result<Option<Row>, StoreError> {
    let n = pk
        .first()
        .and_then(num)
        .ok_or_else(|| refused("Integer: a key that is not a number"))?;
    Ok((LOW..=HIGH).contains(&n).then(|| row(t, n)))
}

pub(super) fn next(t: &TableDef, scan: &Scan) -> Result<Option<Row>, StoreError> {
    super::refuse_unpinned(scan.filters)?;
    let set = set(t, scan.filters)?;
    let others: Vec<&FieldFilter> = scan.filters.iter().filter(|f| f.field != pos(t, NUMBER)).collect();
    let mut after = scan.after.and_then(|a| a.first()).and_then(num);
    let mut examined = 0;
    while let Some(n) = ranges::step(&set, after, scan.ascending) {
        let r = row(t, n);
        if others.iter().all(|f| f.expr.matches_checked(&r[f.field])) {
            return Ok(Some(r));
        }
        examined += 1;
        if examined > SEARCH_LIMIT {
            return Err(refused("Integer: a filter on another column over too many numbers"));
        }
        after = Some(n);
    }
    Ok(None)
}

pub(super) fn count(t: &TableDef, filters: &[FieldFilter]) -> Result<usize, StoreError> {
    super::refuse_unpinned(filters)?;
    let set = set(t, filters)?;
    if filters.iter().all(|f| f.field == pos(t, NUMBER)) {
        return Ok(ranges::size(&set) as usize);
    }
    let others: Vec<&FieldFilter> = filters.iter().filter(|f| f.field != pos(t, NUMBER)).collect();
    if ranges::size(&set) > SEARCH_LIMIT {
        return Err(refused("Integer: a filter on another column over too many numbers"));
    }
    Ok(set
        .iter()
        .flat_map(|&(a, b)| a..=b)
        .filter(|&n| {
            let r = row(t, n);
            others.iter().all(|f| f.expr.matches_checked(&r[f.field]))
        })
        .count())
}
