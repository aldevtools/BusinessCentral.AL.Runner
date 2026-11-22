//! What the SQL `decimal(38,20)` column behind a Decimal field of a database table does to its values: it holds 20
//! decimals and cuts the rest toward zero. Not pinned yet: every cut runs through the assumption entry
//! `decimal-column-cuts-twenty-places` (open corpus pull request 1427, record/crud/RcAGetFind.al:749-842), and only for the field
//! shape, writers and readers that probe measures. A temporary table, and every other field shape, keep the values as they are.

use al_builtins::assumptions::{Assumption, assume};
use al_builtins::filter::{FieldFilter, FilterExpr};
use al_builtins::native::Signal;
use al_builtins::value::{RtType, Value};
use al_data::TableDef;
use rust_decimal::{Decimal, RoundingStrategy};

/// `d` as the column holds it, when that differs from `d`: cut toward zero after the 20th decimal (RcAGetFind.al:749-778,
/// 1000 / 3600000 reads back as 0.00027777777777777777, 0.000000000000000000019 as 0.00000000000000000001). A value with at
/// most 20 decimals costs one compare.
fn cut(d: &Decimal) -> Option<Decimal> {
    if d.scale() <= 20 {
        return None;
    }
    let c = d.round_dp_with_strategy(20, RoundingStrategy::ToZero);
    (c != *d).then_some(c)
}

/// Whether the column changes `v`.
fn cuts(v: &Value) -> bool {
    matches!(v, Value::Decimal(d) if cut(d).is_some())
}

/// Whether field `i` of database table `t` has the shape the probe measures (record/crud/RcAObjects.al:24 `Amt`, :35
/// `key(ByAmt; Amt)`): a Normal Decimal field the table itself declares, outside the primary key, held by exactly one key,
/// a secondary key of its own (the field alone) that the table itself declares with no properties.
pub(crate) fn measured_field(t: &TableDef, schema: &al_ir::Table, i: usize) -> bool {
    let f = &t.fields[i];
    let holding: Vec<usize> = schema
        .keys
        .iter()
        .enumerate()
        .filter(|(_, key)| key.contains(&i))
        .map(|(k, _)| k)
        .collect();
    f.ty == RtType::Decimal
        && f.stored
        && f.id < 2_000_000_000
        && schema.fields.get(i).is_some_and(|s| s.declared_in == Some(t.object))
        && holding.len() == 1
        && holding[0] > 0
        && schema.keys[holding[0]].as_slice() == [i]
        && schema.key_declared_in.get(holding[0]) == Some(&Some(t.object))
        && schema.key_props.get(holding[0]).is_some_and(Vec::is_empty)
}

/// Which write hands a row to the columns.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Write {
    /// An Insert, and whether it runs the trigger.
    Insert {
        run: bool,
    },
    /// A Modify, and whether it runs the trigger.
    Modify {
        run: bool,
    },
    Rename,
}

/// Cuts the Decimal values of `row` (a database table's) as the columns hold them, for the measured field shape only. The
/// probe measures an `Insert()` and a `Modify()` that AL code calls on a Record variable (`by_al`), without the trigger
/// (RcAGetFind.al:749-778, :780-799); any other write of a value the column cuts into such a field is refused.
pub(crate) fn store(
    t: &TableDef,
    schema: &al_ir::Table,
    row: &mut [Value],
    write: Write,
    by_al: bool,
) -> Result<(), Signal> {
    for i in 0..t.fields.len().min(row.len()) {
        let Value::Decimal(d) = &row[i] else { continue };
        let Some(c) = cut(d) else { continue };
        if !measured_field(t, schema, i) {
            continue;
        }
        let measured = by_al && matches!(write, Write::Insert { run: false } | Write::Modify { run: false });
        if !measured {
            return Err(Signal::unsupported(format!(
                "a write other than Record.Insert() or Record.Modify() called by AL of a Decimal value with more than 20 decimals into field {} of table {} (what the column holds is not measured for it)",
                t.fields[i].name, t.name
            )));
        }
        assume(Assumption::DecimalColumnCutsTwentyPlaces);
        row[i] = Value::Decimal(c);
    }
    Ok(())
}

/// Whether a filter expression holds a value the column cuts.
fn expr_cuts(e: &FilterExpr) -> bool {
    use FilterExpr as F;
    match e {
        F::Range(a, b) => a.as_ref().is_some_and(cuts) || b.as_ref().is_some_and(cuts),
        F::Eq(v) | F::RangeSame(v) | F::Ne(v) | F::Lt(v) | F::Le(v) | F::Gt(v) | F::Ge(v) => cuts(v),
        F::Or(es) | F::And(es) => es.iter().any(expr_cuts),
        F::UnpinnedText(inner) | F::NumCode(inner) | F::UnpinnedMatch(inner, _) => expr_cuts(inner),
        F::Like(..) | F::NotLike(..) | F::AtSignAlone(_) | F::EmptyViewValue => false,
    }
}

/// The measured filter forms with their values cut: `Range` (SetRange with one or two values), `Lt` and `Gt`
/// (SetFilter('<%1') and SetFilter('>%1')); `None` for any other form.
fn cut_expr(e: &FilterExpr) -> Option<FilterExpr> {
    use FilterExpr as F;
    let one = |v: &Value| match v {
        Value::Decimal(d) => Value::Decimal(cut(d).unwrap_or(*d)),
        other => other.clone(),
    };
    match e {
        F::Range(a, b) => Some(F::Range(a.as_ref().map(one), b.as_ref().map(one))),
        F::Lt(v) => Some(F::Lt(one(v))),
        F::Gt(v) => Some(F::Gt(one(v))),
        _ => None,
    }
}

/// Whether a filter just set is one the probe measures (RcAGetFind.al:801-842): a database table's measured field, set by
/// AL code on the Record variable (`by_al`) with `SetRange` (`text` is `None`) and one or two Decimal values, or with
/// `SetFilter` of the text `<%1` or `>%1` and one Decimal value, holding a value the column cuts. `values` are the arguments
/// after the field (SetRange) or after the text (SetFilter).
pub(crate) fn measured_setter(
    t: &TableDef,
    schema: &al_ir::Table,
    filter: &FieldFilter,
    by_al: bool,
    text: Option<&str>,
    values: &[Value],
) -> bool {
    let decimals = values.iter().all(|v| matches!(v, Value::Decimal(_)));
    let form = match text {
        None => matches!(values.len(), 1 | 2) && matches!(filter.expr, FilterExpr::Range(..)),
        Some("<%1") => values.len() == 1 && matches!(filter.expr, FilterExpr::Lt(_)),
        Some(">%1") => values.len() == 1 && matches!(filter.expr, FilterExpr::Gt(_)),
        Some(_) => false,
    };
    by_al && filter.group == 0 && form && decimals && measured_field(t, schema, filter.field) && expr_cuts(&filter.expr)
}

/// The filters a database table's read compares, for the measured field shape: a filter value the column cuts is compared
/// cut when the read is a `Count()` AL code called on the Record variable (`count_by_al`) and a measured setter set exactly
/// this filter (`marks`, [`measured_setter`]); any other read or setter of such a filter value is refused
/// (RcAGetFind.al:801-842 measures only these).
pub(crate) fn read_filters(
    t: &TableDef,
    schema: &al_ir::Table,
    filters: &mut [FieldFilter],
    marks: &[FieldFilter],
    count_by_al: bool,
) -> Result<(), Signal> {
    for f in filters {
        if !measured_field(t, schema, f.field) || !expr_cuts(&f.expr) {
            continue;
        }
        let marked = marks
            .iter()
            .any(|m| m.field == f.field && m.group == f.group && m.expr.identical(&f.expr));
        match cut_expr(&f.expr) {
            Some(cut) if count_by_al && marked => {
                assume(Assumption::DecimalColumnCutsTwentyPlaces);
                f.expr = cut;
            }
            _ => {
                return Err(Signal::unsupported(format!(
                    "a read other than Record.Count() through a filter on field {} of table {} with a Decimal value of more than 20 decimals, or such a filter set other than by Record.SetRange or Record.SetFilter('<%1' or '>%1') (how the column compares it is not measured)",
                    t.fields[f.field].name, t.name
                )));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn cuts_toward_zero_after_twenty_decimals() {
        let d = |s: &str| Decimal::from_str(s).unwrap();
        assert_eq!(cut(&(d("1000") / d("3600000"))), Some(d("0.00027777777777777777")));
        assert_eq!(cut(&d("-0.000000000000000000019")), Some(d("-0.00000000000000000001")));
        assert_eq!(cut(&d("0.1")), None);
        assert_eq!(cut(&d("0.00000000000000000001")), None);
    }
}
