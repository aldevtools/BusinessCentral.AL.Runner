//! How a guarded `Codeunit.Run` ends while `Record.Consistent(false)` marks are held, when its record argument has the
//! page-style filters of corpus PR 1600 (errors/ConsistentGuardedRunRecPage.al:2591-2813, codeunit "GRP Tests"; not yet
//! in the pinned corpus): run through the assumption `guarded-run-consistent-page`. Every other shape gets `None`, and the
//! caller refuses it. Also owns the field facts the record argument's filter shapes read (`ArgFields`), for `ArgFilter::of`.
//!
//! Entry points: `page_probe_end` (by `Session::guarded_run_end_marks`), `page_shape` (by `ArgFilter::of`).

use super::{ArgFilter, MarkState, RunArg, RunEnd};
use al_builtins::filter::{FieldFilter, FilterExpr};
use al_builtins::value::{RecordState, RtType, Value};
use al_data::TableDef;
use al_ir::{FieldClass, FieldFacts};

/// The page-style filter shapes of the cells besides Key (`PrepareX` :659-:667, `PrepareZ` :734-:742 of the probe file);
/// Key (`SetRange(Id, 1)`, :671 and :746) is `ArgFilter::Measured { key: true }`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum PageArgShape {
    /// `FilterGroup(2); SetRange(Kind, Kind::Order); FilterGroup(0); SetRange("Date Filter", 0D, WorkDate())`, the same with
    /// `SetFilter(Id, '1')` (`PageMixKey`), and the real page cell whose SourceTableView filters Kind and whose OnOpenPage sets
    /// that Date Filter range (`EnumViewDateFilter`, page "GRP Page X" :359-:413). `date_end` is the date the range ends at,
    /// compared with the WorkDate as the run ends.
    PageMix { date_end: i32 },
    /// `SetRange(Name, 'caller'); SetRange("Doc Type", 1); SetRange(Amount, 5)`.
    Three,
}

/// The facts of the argument's table that the filter shapes read.
pub(super) struct ArgFields<'a> {
    pub(super) facts: &'a [FieldFacts],
    pub(super) table: Option<&'a TableDef>,
}

impl ArgFields<'_> {
    fn class(&self, f: &FieldFilter) -> Option<FieldClass> {
        self.facts.get(f.field).map(|x| x.class)
    }

    /// The filter is on a field of FieldClass Normal.
    pub(super) fn normal(&self, f: &FieldFilter) -> bool {
        self.class(f) == Some(FieldClass::Normal)
    }

    /// SetRange to one value (`FilterExpr::Range` with two equal ends) on a field of FieldClass Normal.
    pub(super) fn one_value(&self, f: &FieldFilter) -> bool {
        self.normal(f) && matches!(&f.expr, FilterExpr::Range(Some(a), Some(b)) if a.identical(b))
    }

    /// SetRange or SetFilter to one value on a field of FieldClass Normal: `SetFilter(Id, '1')`, the page's `const` view.
    fn equal(&self, f: &FieldFilter) -> bool {
        self.one_value(f) || (self.normal(f) && matches!(f.expr, FilterExpr::Eq(_)))
    }

    /// The one field of the primary key, when it has one field (`key(PK; Id)` of the probe's tables, :64, :80, :100).
    fn single_key(&self) -> Option<usize> {
        match self.table?.keys.first()?.as_slice() {
            [k] => Some(*k),
            _ => None,
        }
    }

    /// The filter is on the one field of a one-field primary key (`Id`, the Key and PageMixKey cells).
    pub(super) fn key_field(&self, f: &FieldFilter) -> bool {
        self.single_key() == Some(f.field)
    }

    /// The filter is on a field outside a one-field primary key (Name, Doc Type, Amount of the Three cells).
    fn other_than_key(&self, f: &FieldFilter) -> bool {
        self.single_key().is_some_and(|k| k != f.field)
    }

    /// The filter is on a field of an Enum type (`Kind; Enum "GRP Kind"`, :59 and :95, the view of the PageMix cells).
    fn enum_field(&self, f: &FieldFilter) -> bool {
        matches!(
            self.table.and_then(|t| t.fields.get(f.field)).map(|d| &d.ty),
            Some(RtType::Enum(_))
        )
    }

    /// The date a SetRange from 0D ends at, on a field of FieldClass FlowFilter (`"Date Filter"`, :58 and :94).
    fn date_range_end(&self, f: &FieldFilter) -> Option<i32> {
        match (&f.expr, self.class(f)) {
            (
                FilterExpr::Range(Some(Value::Date(0, false)), Some(Value::Date(d, false))),
                Some(FieldClass::FlowFilter),
            ) => Some(*d),
            _ => None,
        }
    }
}

/// The page shape of the filters of `r`, in filter group 0 without marked rows or MarkedOnly; `None` for any other.
pub(super) fn page_shape(r: &RecordState, fields: &ArgFields) -> Option<PageArgShape> {
    let group = |g: i32| r.filters.iter().filter(move |f| f.group == g).collect::<Vec<_>>();
    let (group0, group2) = (group(0), group(2));
    if group0.len() + group2.len() != r.filters.len() {
        return None;
    }
    let view = |f: &FieldFilter| fields.equal(f) && fields.enum_field(f);
    let extra = |f: &FieldFilter, v: &FieldFilter| fields.equal(f) && fields.key_field(f) && f.field != v.field;
    match (group0.as_slice(), group2.as_slice()) {
        ([a, b, c], []) => {
            let other = |f: &FieldFilter| fields.one_value(f) && fields.other_than_key(f);
            let distinct = a.field != b.field && a.field != c.field && b.field != c.field;
            (other(a) && other(b) && other(c) && distinct).then_some(PageArgShape::Three)
        }
        ([date], [v]) if view(v) => fields
            .date_range_end(date)
            .map(|date_end| PageArgShape::PageMix { date_end }),
        ([a, b], [v]) if view(v) => {
            let end = match (fields.date_range_end(a), fields.date_range_end(b)) {
                (Some(d), None) if extra(b, v) => d,
                (None, Some(d)) if extra(a, v) => d,
                _ => return None,
            };
            Some(PageArgShape::PageMix { date_end: end })
        }
        _ => None,
    }
}

/// `Some(false)` when the run returns false with the inner error and its marks are gone, `None` when no cell measures the
/// shape. The caller gives a run not nested in another guarded run, with no mark at its start and no Commit since it
/// started, whose marks were each set by a database variable other than the argument and released or marked again by
/// none but another variable, and whose argument the run left as it was; `work_date` gives the session's WorkDate. Cells
/// (each for GRP Z, a table no mark names, and GRP X, the table marked first, as a database record), all failing with
/// code Dialog (`Error('inner failed')`):
/// - PageMix with held then marked again (`startx,fya,fyc,err`): RecZ_PageMix_MarkedAgain :2591,
///   RecZ_PageMixKey_MarkedAgain :2607, PageZ_EnumViewDateFilter_MarkedAgain :2623, and the RecX_/PageX_ twins;
/// - Three or Key with released then held (`startx,txb,fya,err`) or marked again then held (`fxa,txb,fxc,fya,err`):
///   RecZ_Three_ReleasedThenHeld :2639, RecZ_Three_AgainThenHeld :2655, RecZ_Key_ReleasedThenHeld :2671,
///   RecZ_Key_AgainThenHeld :2687, and the RecX_ twins (to RecX_Key_AgainThenHeld :2799).
pub(super) fn page_probe_end(
    arg: RunArg,
    work_date: impl FnOnce() -> i32,
    after: &[(u32, MarkState)],
    end: RunEnd,
) -> Option<bool> {
    use MarkState::{Marked as M, MarkedAgainByAnother as A, ReleasedByAnother as R};
    let RunArg::Record {
        table,
        temporary: false,
        filter,
    } = arg
    else {
        return None;
    };
    if end != RunEnd::Failed("Dialog") || !super::unmarked_or_first(table, after) {
        return None;
    }
    let measured = match after {
        [(_, M), (_, A)] => {
            matches!(filter, ArgFilter::Page(PageArgShape::PageMix { date_end }) if date_end == work_date())
        }
        [(_, R | A), (_, M)] => matches!(
            filter,
            ArgFilter::Page(PageArgShape::Three) | ArgFilter::Measured { key: true }
        ),
        _ => false,
    };
    measured.then_some(false)
}
