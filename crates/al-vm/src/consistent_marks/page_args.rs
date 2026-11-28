//! How a guarded `Codeunit.Run` ends while `Record.Consistent(false)` marks are held, when its record argument has the
//! page-style filters of corpus PR 1600 (errors/ConsistentGuardedRunRecPage.al:2591-2813, codeunit "GRP Tests"; not yet
//! in the pinned corpus): run through the assumption `guarded-run-consistent-page`. Every other shape gets `None`, and the
//! caller refuses it. Also owns the field facts the record argument's filter shapes read (`ArgFields`), for `ArgFilter::of`.
//!
//! Entry points: `page_probe_end` (by `Session::guarded_run_end_marks`), `page_shape` (by `ArgFilter::of`).

use super::{ArgFacts, ArgFilter, MarkState, RunArg, RunEnd};
use al_builtins::assumptions::Assumption;
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
    /// PageMix and PageMixKey of corpus PR 1616 (errors/ConsistentGuardedRunRecTwoKey.al, PrepareZ :601 and :605): the
    /// view is `SetRange("Doc Kind", Order)` on the Enum field first in a primary key of an Enum and a Code field, and the
    /// extra filter is `SetFilter("No.", '1')` on the Code field. Run through the assumption `guarded-run-consistent-two-key`.
    PageMixTwoKey { date_end: i32 },
    /// `SetRange(Name, 'caller'); SetRange("Doc Type", 1); SetRange(Amount, 5)`.
    Three,
    /// The filters of the page cells of corpus PR 1622 (errors/ConsistentGuardedRunPageConst.al, codeunit 99562, :675-:1200;
    /// not yet in the pinned corpus): in filter group 2 one equality to the member with ordinal 0 to 3 of `GPC Kind` on the
    /// Enum key field (`"Doc Kind"`) or an Enum field outside the key (`Kind`), in a primary key of an Enum then a Code field;
    /// in filter group 0 nothing, or one value on the Code key field (`No.`: a TestPage filter :900, SetRange :945, SetFilter
    /// :990). Read only with the page's view mark (`consistent_marks/page_view.rs`).
    PageView,
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

    /// The filter is on a field outside a one-field primary key (Name, Doc Type, Amount of the Three cells, Kind of the
    /// PageMix cells); false for a table whose primary key has more than one field.
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

    /// The two fields of a primary key of an Enum field then a Code field (`key(PK; "Doc Kind", "No.")` of corpus PR 1616,
    /// errors/ConsistentGuardedRunRecTwoKey.al:56-57, :67, the shape of Purchase Header); `None` for any other key.
    fn enum_code_key(&self) -> Option<(usize, usize)> {
        let t = self.table?;
        match t.keys.first()?.as_slice() {
            [k1, k2] => match (&t.fields.get(*k1)?.ty, &t.fields.get(*k2)?.ty) {
                (RtType::Enum(_), RtType::Code(_)) => Some((*k1, *k2)),
                _ => None,
            },
            _ => None,
        }
    }

    /// The ordinal of the one Enum value the filter is equal to (`Eq`, or a range with two equal ends).
    fn enum_ordinal(&self, f: &FieldFilter) -> Option<i32> {
        match &f.expr {
            FilterExpr::Eq(Value::Enum(_, o)) => Some(*o),
            FilterExpr::Range(Some(a @ Value::Enum(_, o)), Some(b)) if a.identical(b) => Some(*o),
            _ => None,
        }
    }

    /// The page view shape (`PageArgShape::PageView`) of the group-0 and group-2 filters.
    fn page_view(&self, group0: &[&FieldFilter], group2: &[&FieldFilter]) -> bool {
        let Some((_, k2)) = self.enum_code_key() else {
            return false;
        };
        let view = match group2 {
            // Any Enum field is the Enum key field or a field outside the two-field key, whose other field is a Code.
            [v] => self.equal(v) && self.enum_field(v) && matches!(self.enum_ordinal(v), Some(0..=3)),
            _ => false,
        };
        let extra = match group0 {
            [] => true,
            [a] => self.equal(a) && a.field == k2,
            _ => false,
        };
        view && extra
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
    if fields.page_view(&group0, &group2) {
        return Some(PageArgShape::PageView);
    }
    // The view of both PageMix arms: one value on an Enum field outside a one-field primary key (Kind, key(PK; Id)), or
    // (corpus PR 1616, PrepareZ :601, :605) SetRange to one value on the Enum field first in an Enum-and-Code primary key.
    let view_two_key = |f: &FieldFilter| {
        let in_key = |(k1, _): (usize, usize)| k1 == f.field;
        if fields.equal(f) && fields.enum_field(f) && fields.other_than_key(f) {
            Some(false)
        } else if fields.one_value(f) && fields.enum_code_key().is_some_and(in_key) {
            Some(true)
        } else {
            None
        }
    };
    // The extra group-0 filter of PageMixKey: one value on the one key field (`SetFilter(Id, '1')`), or with the view in
    // an Enum-and-Code key `SetFilter("No.", '1')` on its Code field (PR 1616, :605).
    let extra = |f: &FieldFilter, v: &FieldFilter, two_key: bool| match two_key {
        false => fields.equal(f) && fields.key_field(f) && f.field != v.field,
        true => {
            fields.normal(f)
                && matches!(f.expr, FilterExpr::Eq(_))
                && fields.enum_code_key().is_some_and(|(_, k2)| k2 == f.field)
        }
    };
    let mix = |two_key: bool, date_end: i32| match two_key {
        false => PageArgShape::PageMix { date_end },
        true => PageArgShape::PageMixTwoKey { date_end },
    };
    match (group0.as_slice(), group2.as_slice()) {
        ([a, b, c], []) => {
            let other = |f: &FieldFilter| fields.one_value(f) && fields.other_than_key(f);
            let distinct = a.field != b.field && a.field != c.field && b.field != c.field;
            (other(a) && other(b) && other(c) && distinct).then_some(PageArgShape::Three)
        }
        ([date], [v]) => {
            let two_key = view_two_key(v)?;
            Some(mix(two_key, fields.date_range_end(date)?))
        }
        ([a, b], [v]) => {
            let two_key = view_two_key(v)?;
            let end = match (fields.date_range_end(a), fields.date_range_end(b)) {
                (Some(d), None) if extra(b, v, two_key) => d,
                (None, Some(d)) if extra(a, v, two_key) => d,
                _ => return None,
            };
            Some(mix(two_key, end))
        }
        _ => None,
    }
}

/// The assumption entry when the run returns false with the inner error and its marks are gone, `None` when no cell
/// measures the shape. The caller gives a run not nested in another guarded run, with no mark at its start and no Commit since it
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
    facts: &ArgFacts,
    work_date: impl FnOnce() -> i32,
    after: &[(u32, MarkState)],
    end: RunEnd,
) -> Option<Assumption> {
    use MarkState::{Marked as M, MarkedAgainByAnother as A, ReleasedByAnother as R};
    let RunArg::Record {
        temporary: false,
        filter,
        ..
    } = facts.arg
    else {
        return None;
    };
    if end != RunEnd::Failed("Dialog") || !facts.unmarked_or_first() {
        return None;
    }
    let (entry, date_end) = match (after, filter) {
        ([(_, M), (_, A)], ArgFilter::Page(PageArgShape::PageMix { date_end })) => {
            (Assumption::GuardedRunConsistentPage, date_end)
        }
        // RecZ_PageMix_MarkedAgain :865, RecZ_PageMixKey_MarkedAgain :961, RecX_PageMix_MarkedAgain :2113 and
        // RecX_PageMixKey_MarkedAgain :2209 of corpus PR 1616 (errors/ConsistentGuardedRunRecTwoKey.al).
        ([(_, M), (_, A)], ArgFilter::Page(PageArgShape::PageMixTwoKey { date_end })) => {
            (Assumption::GuardedRunConsistentTwoKey, date_end)
        }
        ([(_, R | A), (_, M)], ArgFilter::Page(PageArgShape::Three) | ArgFilter::Measured { key: true }) => {
            return Some(Assumption::GuardedRunConsistentPage);
        }
        _ => return None,
    };
    (date_end == work_date()).then_some(entry)
}
