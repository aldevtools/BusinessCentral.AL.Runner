//! `@` alone (al-runner issue 8637): what a filter of `@` and nothing else keeps rests on where the rows are read, not on the
//! field's type (open corpus PR 1495, BgTaskQueryCache.al, BQ Tests 9b). A read of the database keeps the rows whose value is
//! empty: a Code field (:5654) and a Text one (:5818), a FlowFilter's `@` that CalcFields applies to a source field of either
//! type, the same or the other (:5791, :5805), a lookup FlowField of a record in the database (:5847), and a query's Code and
//! Text column, by SetFilter and by its DataItemTableFilter (BQ Tests 9c, :6407-:6457). A temporary record keeps every row: a
//! Text field (record/filters/WcInvalidFilterExpr.al:66-90, pinned), a Code field (:5818) and a lookup FlowField of a
//! temporary record (:5847). A reader that knows where its rows are rewrites the filter here; any other, and a platform table
//! its provider serves (no cell measures one), keeps every row of a Text field, as main does, and refuses `@` alone on a Code
//! field ([`FilterExpr::AtSignAloneCode`]).

use crate::assumptions::{Assumption, assume};
use crate::filter::{FieldFilter, FilterExpr};
use crate::value::Value;
use std::borrow::Cow;

impl FilterExpr {
    /// This filter as a read of `database` rows answers it, or None when it is not `@` alone: the empty value `blank` of the
    /// field in the database, every value in a temporary record.
    pub fn at_sign_alone_read(&self, database: bool, blank: impl FnOnce() -> Value) -> Option<FilterExpr> {
        let code = match self {
            FilterExpr::AtSignAlone(false) => false,
            FilterExpr::AtSignAloneCode => true,
            _ => return None,
        };
        if database || code {
            assume(Assumption::AtSignAloneDatabaseBlank);
        }
        Some(if database {
            FilterExpr::Eq(blank())
        } else {
            FilterExpr::AtSignAlone(false)
        })
    }
}

/// `filters` as a read of `database` rows answers them ([`FilterExpr::at_sign_alone_read`]); `blank`: the empty value of a
/// field by its position.
pub fn at_sign_alone_read<'a>(
    filters: &'a [FieldFilter],
    database: bool,
    blank: impl Fn(usize) -> Value,
) -> Cow<'a, [FieldFilter]> {
    if !filters
        .iter()
        .any(|f| matches!(f.expr, FilterExpr::AtSignAlone(false) | FilterExpr::AtSignAloneCode))
    {
        return Cow::Borrowed(filters);
    }
    Cow::Owned(
        filters
            .iter()
            .map(|f| match f.expr.at_sign_alone_read(database, || blank(f.field)) {
                Some(expr) => FieldFilter { expr, ..f.clone() },
                None => f.clone(),
            })
            .collect(),
    )
}
