//! A name in a view given to `SetView` that names no field of the table. BC drops a WHERE condition on such a name and gives
//! the primary key for a SORTING of one (record/lt-setview-caption-form/SetViewCaptionForm.al:179-191 SetView_UnknownField_ErrorText:
//! `WHERE(Nothing Here=CONST(5))` sets no filter; types/lt-evaluate-format-lasterror-setview/P86SetView.al:222 SetView_Malformed `v3`
//! `WHERE(Nope=CONST(1))`, :184 SetView_GetViewRoundTrip_NameFormSetFilter `v8` `SORTING(Nonexistent)`). Pinned only for a view
//! given at run time, for a name that is the only condition or the only SORTING field, and for a plain name: a lowercase
//! `field<n>`, a `Field<n>` with no such field and a quoted name are refused (gap G5-14 probes them).

use crate::Session;
use al_builtins::native::Signal;
use al_data::TableDef;

/// A name the `Field<n>` form or a quote could stand behind: not pinned when it names no field.
fn unpinned_unknown(name: &str) -> bool {
    let digits = |p: &str| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit());
    name.contains('"')
        || name
            .get(..5)
            .is_some_and(|p| p.eq_ignore_ascii_case("field") && digits(&name[5..]))
}

/// `CONST(..)` or `FILTER(..)` and nothing after: the form of the pinned conditions.
fn plain_condition(cond: &str) -> bool {
    let cond = cond.trim();
    let Some(open) = cond.find('(') else {
        return false;
    };
    matches!(cond[..open].trim().to_ascii_uppercase().as_str(), "CONST" | "FILTER") && cond.ends_with(')')
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
            Err(_) if runtime && alone && !unpinned_unknown(name) && plain_condition(cond) => Ok(None),
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
