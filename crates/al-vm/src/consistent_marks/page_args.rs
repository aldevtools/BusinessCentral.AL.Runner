//! How a guarded `Codeunit.Run` ends while `Record.Consistent(false)` marks are held, when its record argument has the
//! page-style filters of corpus PR 1600 (errors/ConsistentGuardedRunRecPage.al:2591-2799, codeunit "GRP Tests"; not yet
//! in the pinned corpus): run through the assumption `guarded-run-consistent-page`. Every other shape gets `None`, and the
//! caller refuses it.
//!
//! Entry point: `page_probe_end` (by `Session::guarded_run_end_marks`).

use super::{ArgRecord, MarkState, RunArg, RunEnd};
use al_builtins::filter::{FieldFilter, FilterExpr};
use al_builtins::value::Value;
use al_ir::{FieldClass, FieldFacts};

/// The filter shapes of the cells (`PrepareArg` of the probe file, :657-:671).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PageArgShape {
    /// `FilterGroup(2); SetRange(Kind, Kind::Order); FilterGroup(0); SetRange("Date Filter", 0D, WorkDate())`, the same with
    /// `SetFilter(Id, '1')` (`PageMixKey`), and the real page cell whose SourceTableView filters Kind and whose OnOpenPage sets
    /// that Date Filter range (`EnumViewDateFilter`, page "GRP Page X" :359-:413).
    PageMix,
    /// `SetRange(Name, 'caller'); SetRange("Doc Type", 1); SetRange(Amount, 5)`.
    Three,
    /// `SetRange(Id, 1)`.
    Key,
}

/// `Some(false)` when the run returns false with the inner error and its marks are gone, `None` when no cell measures the
/// shape. The caller gives a run not nested in another guarded run, with no mark at its start and no Commit since it
/// started, whose marks were each set by a database variable other than the argument, and whose argument the run left
/// as it was; `fields` are the facts of the argument's table and `work_date` gives the session's WorkDate. Cells (each for GRP Z, a table no mark names, and GRP X, the table marked first, as a database record), all
/// failing with code Dialog (`Error('inner failed')`):
/// - PageMix with held then marked again (`startx,fya,fyc,err`): RecZ_PageMix_MarkedAgain :2591,
///   RecZ_PageMixKey_MarkedAgain :2606, PageZ_EnumViewDateFilter_MarkedAgain :2621, and the RecX_/PageX_ twins;
/// - Three or Key with released then held (`startx,txb,fya,err`) or marked again then held (`fxa,txb,fxc,fya,err`):
///   RecZ_Three_ReleasedThenHeld :2636, RecZ_Three_AgainThenHeld :2651, RecZ_Key_ReleasedThenHeld :2666,
///   RecZ_Key_AgainThenHeld :2681, and the RecX_ twins.
pub(super) fn page_probe_end(
    arg: RunArg,
    rec: &ArgRecord,
    fields: &[FieldFacts],
    work_date: impl FnOnce() -> i32,
    after: &[(u32, MarkState)],
    end: RunEnd,
) -> Option<bool> {
    use MarkState::{Marked as M, MarkedAgainByAnother as A, ReleasedByAnother as R};
    let RunArg::Record {
        table,
        temporary: false,
        ..
    } = arg
    else {
        return None;
    };
    let first_marked = after.first().map(|(t, _)| *t) == Some(table);
    let unmarked = after.iter().all(|(t, _)| *t != table);
    if end != RunEnd::Failed("Dialog") || !(first_marked || unmarked) {
        return None;
    }
    let states: Vec<MarkState> = after.iter().map(|(_, s)| *s).collect();
    let orders = match states.as_slice() {
        [M, A] => [PageArgShape::PageMix].as_slice(),
        [R, M] | [A, M] => [PageArgShape::Three, PageArgShape::Key].as_slice(),
        _ => return None,
    };
    let shape = page_arg_shape(rec, fields, work_date())?;
    orders.contains(&shape).then_some(false)
}

/// The shape of the argument's filters as the run started; `None` for any other.
fn page_arg_shape(rec: &ArgRecord, fields: &[FieldFacts], work_date: i32) -> Option<PageArgShape> {
    if rec.filter_group != 0 || rec.marked_only || !rec.marks.is_empty() {
        return None;
    }
    let class = |f: &FieldFilter| fields.get(f.field).map(|x| x.class);
    let normal = |f: &FieldFilter| class(f) == Some(FieldClass::Normal);
    // SetRange to one value (`FilterExpr::Range` with two equal ends).
    let one_value =
        |f: &FieldFilter| normal(f) && matches!(&f.expr, FilterExpr::Range(Some(a), Some(b)) if a.identical(b));
    // SetFilter to one value: SetFilter(Id, '1'), and the page's `const` view.
    let equal = |f: &FieldFilter| one_value(f) || (normal(f) && matches!(f.expr, FilterExpr::Eq(_)));
    let work_date_range = |f: &FieldFilter| {
        class(f) == Some(FieldClass::FlowFilter)
            && matches!(&f.expr, FilterExpr::Range(Some(Value::Date(0, false)), Some(Value::Date(d, false))) if *d == work_date)
    };
    let group = |g: i32| rec.filters.iter().filter(move |f| f.group == g).collect::<Vec<_>>();
    let (group0, group2) = (group(0), group(2));
    if group0.len() + group2.len() != rec.filters.len() {
        return None;
    }
    let distinct = |fs: &[&FieldFilter]| {
        fs.iter()
            .enumerate()
            .all(|(i, a)| fs[..i].iter().all(|b| b.field != a.field))
    };
    match (group0.as_slice(), group2.as_slice()) {
        ([key], []) if one_value(key) => Some(PageArgShape::Key),
        (three @ [_, _, _], []) if three.iter().all(|f| one_value(f)) && distinct(three) => Some(PageArgShape::Three),
        ([date], [view]) if work_date_range(date) && equal(view) => Some(PageArgShape::PageMix),
        ([a, b], [view]) if equal(view) && a.field != b.field => {
            let date_and_key = (work_date_range(a) && equal(b)) || (work_date_range(b) && equal(a));
            date_and_key.then_some(PageArgShape::PageMix)
        }
        _ => None,
    }
}
