//! A name in a view given to `SetView` that names no field of the table. BC drops a WHERE condition on such a name and gives
//! the primary key for a SORTING of one (record/lt-setview-caption-form/SetViewCaptionForm.al:179-191 SetView_UnknownField_ErrorText:
//! `WHERE(Nothing Here=CONST(5))` sets no filter; types/lt-evaluate-format-lasterror-setview/P86SetView.al:222 SetView_Malformed `v3`
//! `WHERE(Nope=CONST(1))`, :184 SetView_GetViewRoundTrip_NameFormSetFilter `v8` `SORTING(Nonexistent)`). Pinned only for a view
//! given at run time, for a name that is the only condition or the only SORTING field, and for a plain name or a `Field<n>`
//! with no such field (record/lt-y51-setview-shapes/Y51SetViewShapes.al:166-172, `WHERE(Field99=1(x))` and `SORTING(Field99)`).
//! `Field0` names the timestamp column, which the runtime does not model (Y51SetViewShapes.al:164-173): refused, as is a
//! quoted name.

use crate::Session;
use al_builtins::native::Signal;
use al_data::TableDef;

/// The number of a `Field<n>` name (any case).
fn field_number(name: &str) -> Option<u32> {
    name.get(..5)
        .filter(|p| p.eq_ignore_ascii_case("field"))
        .and_then(|_| name[5..].parse().ok())
}

/// A name of a user-name system FlowField (2000000005 to 2000000008): its name or `Field<n>`.
fn user_name_field(name: &str) -> bool {
    let name = name.trim().trim_matches('"');
    (2_000_000_005..=2_000_000_008).any(|no| {
        field_number(name) == Some(no)
            || crate::platform::user_name_system_field(no).is_some_and(|(n, _)| n.eq_ignore_ascii_case(name))
    })
}

/// A name whose absence is not pinned: a quoted name, or `Field0` (the timestamp column).
fn unpinned_unknown(name: &str) -> bool {
    name.contains('"') || field_number(name) == Some(0)
}

/// `CONST(..)` or `FILTER(..)` and nothing after: the form of the pinned conditions; on a `Field<n>` name also `1(..)`
/// (Y51SetViewShapes.al:166 `WHERE(Field99=1(x))`).
fn plain_condition(name: &str, cond: &str) -> bool {
    let cond = cond.trim();
    let Some(open) = cond.find('(') else {
        return false;
    };
    match cond[..open].trim().to_ascii_uppercase().as_str() {
        "CONST" | "FILTER" => cond.ends_with(')'),
        "1" => field_number(name).is_some() && cond.ends_with(')'),
        _ => false,
    }
}

impl Session {
    /// The field a WHERE condition `name=cond` names, or `None` when the condition is dropped (see the module comment). `alone`: the
    /// condition is the only one of the WHERE.
    pub(crate) fn view_where_field(
        &self,
        t: &TableDef,
        name: &str,
        cond: &str,
        runtime: bool,
        alone: bool,
    ) -> Result<Option<usize>, Signal> {
        match self.view_field(t, name) {
            Ok(p) => Ok(Some(p)),
            // The user-name system FlowFields are fields of every table on BC 29: a condition on one is kept and filters
            // (al-runner issue 6645; corpus branch al-runtime-rs-staging-y64 85d86e7b, system-fields/lt-y64-username-filters/
            // Y64UserNameFilters.al:154-191). The runtime does not model these filters: refused, never dropped.
            Err(_) if user_name_field(name) => Err(Signal::Unsupported(format!(
                "a view condition on the user-name system FlowField `{name}` (filters on it are not supported yet)"
            ))),
            Err(_) if runtime && alone && !unpinned_unknown(name) && plain_condition(name, cond) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// The fields the SORTING `inner` of `view` names, or `None` for the primary key when its one field is a name of no field
    /// (see the module comment; also record/lt-setview-sorting-nonkey/SetViewSortingNonKey.al:204 `unknown_field`). An empty
    /// SORTING is an invalid view (SetViewSortingNonKey.al:205, :224-226 `empty_sorting`).
    pub(crate) fn view_sorting_fields(
        &self,
        t: &TableDef,
        inner: &str,
        view: &str,
        runtime: bool,
    ) -> Result<Option<Vec<usize>>, Signal> {
        if inner.trim().is_empty() {
            return Err(crate::views::invalid_view(view));
        }
        let names: Vec<&str> = inner.split(',').map(str::trim).collect();
        match names
            .iter()
            .map(|n| self.view_field(t, n))
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(wanted) => Ok(Some(wanted)),
            Err(_) if runtime && names.len() == 1 && !unpinned_unknown(names[0]) => Ok(None),
            Err(e) => Err(e),
        }
    }
}
