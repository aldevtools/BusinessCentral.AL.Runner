//! Filters on one integer-like field as a sorted set of closed intervals, for the generated
//! tables (Integer, Date), whose rows cannot be listed before a filter bounds them.

use al_builtins::describe::Describe;
use al_builtins::filter::{FieldFilter, FilterExpr};
use al_builtins::value::Value;

/// Closed interval `lo..=hi`; empty when `lo > hi`.
pub(crate) type Iv = (i64, i64);

/// A sorted list of disjoint, non-adjacent intervals.
pub(crate) type Set = Vec<Iv>;

fn normalize(mut v: Vec<Iv>) -> Set {
    v.retain(|(a, b)| a <= b);
    v.sort_unstable();
    let mut out: Set = Vec::with_capacity(v.len());
    for (a, b) in v {
        match out.last_mut() {
            Some(last) if a <= last.1.saturating_add(1) => last.1 = last.1.max(b),
            _ => out.push((a, b)),
        }
    }
    out
}

pub(crate) fn intersect(x: &[Iv], y: &[Iv]) -> Set {
    let mut out = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < x.len() && j < y.len() {
        let (a, b) = (x[i].0.max(y[j].0), x[i].1.min(y[j].1));
        if a <= b {
            out.push((a, b));
        }
        if x[i].1 < y[j].1 {
            i += 1;
        } else {
            j += 1;
        }
    }
    out
}

fn complement(x: &[Iv], lo: i64, hi: i64) -> Set {
    let mut out = Vec::new();
    let mut at = lo;
    for &(a, b) in x {
        if a > at {
            out.push((at, a - 1));
        }
        at = at.max(b.saturating_add(1));
    }
    if at <= hi {
        out.push((at, hi));
    }
    out
}

/// The intervals within `lo..=hi` that `e` accepts. `num` reads a filter value as a number.
/// An expression that is not a set of numeric ranges (`Like`) is an error.
pub(crate) fn of_expr(e: &FilterExpr, num: &dyn Fn(&Value) -> Option<i64>, lo: i64, hi: i64) -> Result<Set, String> {
    let n = |v: &Value| num(v).ok_or_else(|| format!("filter value {} is not a number of this field", v.describe()));
    Ok(match e {
        FilterExpr::Range(a, b) => {
            let a = a.as_ref().map(n).transpose()?.unwrap_or(lo).max(lo);
            let b = b.as_ref().map(n).transpose()?.unwrap_or(hi).min(hi);
            normalize(vec![(a, b)])
        }
        FilterExpr::Eq(v) | FilterExpr::RangeSame(v) => normalize(vec![(n(v)?.max(lo), n(v)?.min(hi))]),
        FilterExpr::Ne(v) => {
            let x = n(v)?;
            complement(&normalize(vec![(x, x)]), lo, hi)
        }
        FilterExpr::Lt(v) => normalize(vec![(lo, n(v)?.saturating_sub(1).min(hi))]),
        FilterExpr::Le(v) => normalize(vec![(lo, n(v)?.min(hi))]),
        FilterExpr::Gt(v) => normalize(vec![(n(v)?.saturating_add(1).max(lo), hi)]),
        FilterExpr::Ge(v) => normalize(vec![(n(v)?.max(lo), hi)]),
        FilterExpr::Or(xs) => {
            let mut all = Vec::new();
            for x in xs {
                all.extend(of_expr(x, num, lo, hi)?);
            }
            normalize(all)
        }
        FilterExpr::And(xs) => {
            let mut acc = vec![(lo, hi)];
            for x in xs {
                acc = intersect(&acc, &of_expr(x, num, lo, hi)?);
            }
            acc
        }
        FilterExpr::Like(..) | FilterExpr::NotLike(..) => return Err("a wildcard filter on a numeric field".into()),
    })
}

/// The intervals of `lo..=hi` that every filter on row position `field` accepts.
pub(crate) fn of_field(
    filters: &[FieldFilter],
    field: usize,
    num: &dyn Fn(&Value) -> Option<i64>,
    lo: i64,
    hi: i64,
) -> Result<Set, String> {
    let mut acc = vec![(lo, hi)];
    for f in filters.iter().filter(|f| f.field == field) {
        acc = intersect(&acc, &of_expr(&f.expr, num, lo, hi)?);
    }
    Ok(acc)
}

/// The smallest member of `set` above `after` (or the smallest at all), or the largest below it
/// when `ascending` is false.
pub(crate) fn step(set: &[Iv], after: Option<i64>, ascending: bool) -> Option<i64> {
    if ascending {
        let from = after.map_or(i64::MIN, |a| a.saturating_add(1));
        set.iter().find(|&&(_, b)| b >= from).map(|&(a, _)| a.max(from))
    } else {
        let to = after.map_or(i64::MAX, |a| a.saturating_sub(1));
        set.iter().rev().find(|&&(a, _)| a <= to).map(|&(_, b)| b.min(to))
    }
}

/// How many integers `set` holds.
pub(crate) fn size(set: &[Iv]) -> i64 {
    set.iter().map(|(a, b)| b - a + 1).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn int(v: &Value) -> Option<i64> {
        match v {
            Value::Integer(i) => Some(i64::from(*i)),
            _ => None,
        }
    }
    fn r(a: i32, b: i32) -> FilterExpr {
        FilterExpr::Range(Some(Value::Integer(a)), Some(Value::Integer(b)))
    }

    #[test]
    fn a_union_of_two_ranges_is_not_their_envelope() {
        let e = FilterExpr::Or(vec![r(1, 50), r(200, 209)]);
        let s = of_expr(&e, &int, -1000, 1000).unwrap();
        assert_eq!(s, vec![(1, 50), (200, 209)]);
        assert_eq!(size(&s), 60);
        assert_eq!(step(&s, Some(50), true), Some(200));
        assert_eq!(step(&s, Some(200), false), Some(50));
    }

    #[test]
    fn open_ends_run_to_the_domain_and_an_inverted_range_is_empty() {
        let s = of_expr(&FilterExpr::Range(Some(Value::Integer(5)), None), &int, -10, 10).unwrap();
        assert_eq!(s, vec![(5, 10)]);
        assert!(of_expr(&r(10, 4), &int, -10, 10).unwrap().is_empty());
    }

    #[test]
    fn not_equal_is_the_complement() {
        let s = of_expr(&FilterExpr::Ne(Value::Integer(0)), &int, -2, 2).unwrap();
        assert_eq!(s, vec![(-2, -1), (1, 2)]);
    }
}
