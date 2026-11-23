//! `@` alone and `<>@` alone (al-runner issue 8637): what a filter of `@` and nothing else keeps rests on how BC reads the rows,
//! not on the field's type ([`AtSignRows`]). The cells: open corpus PR 1495, pages/background-tasks/BgTaskQueryCache.al (BQ Tests
//! 9b and 9c) and record/filters/AtSignAloneReads.al. A Text field of a temporary record is pinned (record/filters/
//! WcInvalidFilterExpr.al:66-111: `@` keeps every row, `<>@` none); every other answer here runs under the assumption
//! `at-sign-alone-database-blank`. A reader that knows how its rows are read rewrites the filter here; any other (a platform
//! table its provider serves, a CRM table's connection, a query's own check, CalcSums or a TableRelation over a FlowField filter)
//! keeps the filter as it is: a Text field keeps main's answer and `@` alone on a Code field refuses
//! ([`FilterExpr::AtSignAloneCode`]). The places a filter comes from where `@` alone on a Code field is measured are those of
//! [`parse_filter_at_sign_measured`].
//!
//! `@` alone and `<>@` alone answer as BC reads the rows (`AtSignRows`: a read of the database, a row
//! of the database BC checks itself, a temporary record), whatever the field's type. al-data
//! `MemStore::at_sign_alone_read` picks the kind for the store's `next`, `count` and `delete_all`,
//! for `row_passes` and for `passes_flow_field_filters` (by the Record call that runs); a CRM table's
//! connection, a platform table and any other reader keep the filter, so `@` alone on a Code field
//! refuses (`FilterExpr::AtSignAloneCode`). It is parsed only at the places the cells measure
//! (`parse_filter_at_sign_measured`; `<>@` alone on a Code field only where its cells are,
//! `parse_filter_both_at_signs_measured`); `parse_filter_with` refuses it everywhere else. Entry
//! points: `AtSignRows`, `at_sign_alone_read`, `parse_filter_at_sign_measured`,
//! `parse_filter_both_at_signs_measured`.

use crate::assumptions::{Assumption, assume};
use crate::filter::{FieldFilter, FilterExpr};
use crate::value::Value;
use std::borrow::Cow;

/// How BC reads the rows a filter of `@` alone or `<>@` alone selects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AtSignRows {
    /// A read of the database: `@` keeps the empty values, `<>@` the others. FindSet and Next, FindFirst, FindLast, Find('-'),
    /// Find('+'), Find('<'), Count, IsEmpty, CalcSums, DeleteAll and ModifyAll (AtSignAloneReads.al:229, :265, :314, :399, :429,
    /// :698; BgTaskQueryCache.al:5780), a report's data item and a page (AtSignAloneReads.al:625, :658), CalcFields of a sum or a
    /// count (BgTaskQueryCache.al:5753, :5767; AtSignAloneReads.al:558, :598), a query (BgTaskQueryCache.al:6369-:6419), and
    /// Count and IsEmpty with the filter on a lookup FlowField (BgTaskQueryCache.al:5809; AtSignAloneReads.al:366, :730).
    Database,
    /// A row of the database BC checks against the filter itself: `@` keeps every row, `<>@` the values that are not empty.
    /// Find('=') (AtSignAloneReads.al:314, :698), and Find, FindFirst, FindLast, FindSet and Next with the filter on a lookup
    /// FlowField (:366, :730).
    DatabaseRowCheck,
    /// A temporary record: `@` keeps every row, `<>@` none (AtSignAloneReads.al:247, :265, :314, :366, :698, :730).
    Temporary,
}

/// The places a filter comes from where the cells measure `@` alone on a Code field: Record.SetFilter in filter groups 0, 1 and 2
/// and FieldRef.SetFilter (AtSignAloneReads.al:457, :475, :538; the FlowFilters and lookup FlowField filters they set,
/// BgTaskQueryCache.al:5753, :5809), SetView and RecordRef.SetView (AtSignAloneReads.al:497, :538), a CalcFormula's
/// `field(filter())` of a normal field (:558) and its `filter('@')` (:579), a query's SetFilter and its DataItemTableFilter
/// (BgTaskQueryCache.al:6369, :6403). Every other place ([`crate::filter::parse_filter_with`]) refuses it, as before this module
/// answered it. Here `<>@` alone on a Code field refuses: the places that measure it too take
/// [`parse_filter_both_at_signs_measured`].
pub fn parse_filter_at_sign_measured(
    text: &str,
    args: &[Value],
    ty: &crate::value::RtType,
    enums: &dyn Fn(u32) -> Result<Vec<(i32, String)>, String>,
) -> Result<FilterExpr, String> {
    refused_where_not_measured(crate::filter::parse_filter_any(text, args, ty, enums)?, true)
}

/// [`parse_filter_at_sign_measured`] at the places where the cells also measure `<>@` alone on a Code field: Record.SetFilter on
/// stored fields (AtSignAloneReads.al:698) and on a lookup FlowField (:730), FieldRef.SetFilter, SetView and RecordRef.SetView
/// (:792), and a CalcFormula's `filter('<>@')` (:752).
pub fn parse_filter_both_at_signs_measured(
    text: &str,
    args: &[Value],
    ty: &crate::value::RtType,
    enums: &dyn Fn(u32) -> Result<Vec<(i32, String)>, String>,
) -> Result<FilterExpr, String> {
    crate::filter::parse_filter_any(text, args, ty, enums)
}

/// `e`, or the refusal of `@` alone on a Code field from a place no cell measures: a place that does not admit it (`at_sign`
/// false, [`crate::filter::parse_filter_with`]) refuses `@` alone and `<>@` alone, one of [`parse_filter_at_sign_measured`]
/// refuses `<>@` alone.
pub(crate) fn refused_where_not_measured(e: FilterExpr, at_sign: bool) -> Result<FilterExpr, String> {
    match e {
        FilterExpr::AtSignAloneCode(negated) if negated || !at_sign => Err(format!(
            "`{}` alone on a Code field from a place no cell measures (al-builtins `filter_at_sign`)",
            if negated { "<>@" } else { "@" }
        )),
        e => Ok(e),
    }
}

impl FilterExpr {
    /// This filter as BC reads `rows` with it ([`AtSignRows`]), or None when it is not `@` alone or `<>@` alone; `blank` is the
    /// field's empty value.
    pub fn at_sign_alone_read(&self, rows: AtSignRows, blank: impl FnOnce() -> Value) -> Option<FilterExpr> {
        let (code, negated) = match self {
            FilterExpr::AtSignAlone(negated) => (false, *negated),
            FilterExpr::AtSignAloneCode(negated) => (true, *negated),
            _ => return None,
        };
        if rows != AtSignRows::Temporary || code {
            assume(Assumption::AtSignAloneDatabaseBlank);
        }
        Some(match (rows, negated) {
            (AtSignRows::Database, false) => FilterExpr::Eq(blank()),
            (AtSignRows::Database | AtSignRows::DatabaseRowCheck, true) => FilterExpr::Ne(blank()),
            (AtSignRows::DatabaseRowCheck | AtSignRows::Temporary, negated) => FilterExpr::AtSignAlone(negated),
        })
    }
}

/// `filters` as BC reads `rows` with them ([`FilterExpr::at_sign_alone_read`]); `blank`: the empty value of a field by its
/// position.
pub fn at_sign_alone_read<'a>(
    filters: &'a [FieldFilter],
    rows: AtSignRows,
    blank: impl Fn(usize) -> Value,
) -> Cow<'a, [FieldFilter]> {
    if !filters
        .iter()
        .any(|f| matches!(f.expr, FilterExpr::AtSignAlone(_) | FilterExpr::AtSignAloneCode(_)))
    {
        return Cow::Borrowed(filters);
    }
    Cow::Owned(
        filters
            .iter()
            .map(|f| match f.expr.at_sign_alone_read(rows, || blank(f.field)) {
                Some(expr) => FieldFilter { expr, ..f.clone() },
                None => f.clone(),
            })
            .collect(),
    )
}
