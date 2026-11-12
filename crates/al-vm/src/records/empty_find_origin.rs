//! What `GetLastErrorText(true)` answers after a find on an empty set: the one owner of the flagged texts of `DB:TableIsEmpty`,
//! `DB:NothingInsideFilter` and `DB:RecordNotFound`, for a Record and a RecordRef. The errors, their codes and their unflagged
//! texts are the same for both (`op_inner.rs`); the flagged text was measured apart for each, so each rule names the variable.
//!
//! A RecordRef find goes through the Record's find; `recordref_record_op` marks it (`Session::find_through_recordref`), and the
//! find takes the mark at its start (`FindVia::take`), so a find nested inside it is a Record's again.

use super::*;
use al_builtins::native::ErrorOrigin;

/// The variable a find runs through.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum FindVia {
    Record,
    RecordRef,
}

impl FindVia {
    /// The variable of the find that starts now; the mark is cleared for whatever runs inside it.
    pub(crate) fn take(s: &mut Session) -> FindVia {
        if std::mem::take(&mut s.find_through_recordref) {
            FindVia::RecordRef
        } else {
            FindVia::Record
        }
    }
}

/// The shape of a find that met no row: the method, its search string (`-` for FindFirst and FindSet, `+` for FindLast) and
/// whether the record is temporary.
#[derive(Clone, Copy)]
pub(crate) struct EmptyFind<'a> {
    pub(crate) via: FindVia,
    pub(crate) method: RecordOp,
    pub(crate) which: &'a str,
    pub(crate) temporary: bool,
}

impl EmptyFind<'_> {
    fn first_last_set(&self) -> bool {
        matches!(
            self.method,
            RecordOp::FindFirst | RecordOp::FindLast | RecordOp::FindSet
        )
    }
    fn minus_plus(&self) -> bool {
        self.method == RecordOp::Find && matches!(self.which, "-" | "+")
    }
}

impl Session {
    /// A RecordRef method that forwards to the record method of the same name (`Find`, `FindFirst`, `Insert`, ...).
    pub(crate) fn recordref_record_op(&mut self, method: &str, rec: &Rec, args: &mut [Value]) -> Result<Value, Signal> {
        let op = RecordOp::from_name(method).expect("a RecordRef method that forwards to the record");
        self.find_through_recordref = matches!(
            op,
            RecordOp::Find | RecordOp::FindFirst | RecordOp::FindLast | RecordOp::FindSet
        );
        let outer = std::mem::replace(&mut self.through_recordref, true);
        let r = self.record_op_of_al(op, rec, args);
        // A path that returned before the find took the mark leaves no mark behind.
        self.find_through_recordref = false;
        self.through_recordref = outer;
        r
    }

    /// The table is empty (DB:TableIsEmpty), with no filter: the text as it is.
    /// - Record: FindFirst and FindLast on every version (format-evaluate/P86LastError.al:339-353); FindSet, Find('-') and Find('+')
    ///   from BC 28 (errors/last-error/FindEmptyTableExcludeFlag.al:99-121, guarded with BC28PLUS); a temporary one: FindFirst only
    ///   (errors/last-error/LastErrorFlagRuntimeCodes.al:146 `empty_true`).
    /// - RecordRef: the five finds of a database table, Find('-') and Find('+') of a temporary one, on BC 27.5, 28.5 and 29.0
    ///   (LastErrorFlagRuntimeCodes.al RecordRef_EmptyTable_NoFilter_ExcludeFlag, RecordRef_Temporary_ExcludeFlag; corpus pull
    ///   request 1327, al-runner issue 7806).
    ///
    /// Everything else is not measured.
    pub(crate) fn table_is_empty_origin(&self, f: EmptyFind) -> ErrorOrigin {
        let measured = match f.via {
            FindVia::Record if f.temporary => f.method == RecordOp::FindFirst,
            FindVia::Record => {
                self.has(al_version::Gate::EmptyTableFindExcludeFlagFrom28)
                    || matches!(f.method, RecordOp::FindFirst | RecordOp::FindLast)
            }
            FindVia::RecordRef => !f.temporary || f.minus_plus(),
        };
        if measured {
            ErrorOrigin::SameText
        } else {
            ErrorOrigin::Unknown
        }
    }

    /// No row within the filter (DB:NothingInsideFilter): the filters hidden.
    /// - Record: FindFirst, FindLast and FindSet of a table with rows (errors/last-error/FindEmptyTableExcludeFlag.al:148, :152,
    ///   :156), Find('-') and Find('+') of one and all five on an empty table (LastErrorFlagRuntimeCodes.al:
    ///   NothingInsideFilter_EmptyTable_ExcludeFlag, _FindMinusPlus_TableWithRows_); FindFirst of a temporary record, with rows or
    ///   empty (_TemporaryRecord_ExcludeFlag).
    /// - RecordRef: the five finds of a database table, empty or with rows (RecordRef_EmptyTable_Filtered_ExcludeFlag,
    ///   RecordRef_TableWithRows_FilteredOut_ExcludeFlag), FindFirst, FindLast and FindSet of a temporary one
    ///   (RecordRef_Temporary_ExcludeFlag); corpus pull request 1327.
    ///
    /// Everything else is not measured.
    pub(crate) fn nothing_inside_filter_origin(&self, t: &TableDef, f: EmptyFind) -> ErrorOrigin {
        let measured = match f.via {
            FindVia::Record => !f.temporary || f.method == RecordOp::FindFirst,
            FindVia::RecordRef => f.first_last_set() || (f.minus_plus() && !f.temporary),
        };
        if measured {
            ErrorOrigin::Redacted(format!(
                "There is no {} within the filter.{BR}{BR}Filters: <redacted>",
                self.caption(t)
            ))
        } else {
            ErrorOrigin::Unknown
        }
    }

    /// Find('=') of an empty database table with the blank key (DB:RecordNotFound): the key hidden. Record: from BC 28
    /// (errors/last-error/FindEmptyTableExcludeFlag.al:123, guarded with BC28PLUS). RecordRef: on BC 27.5, 28.5 and 29.0
    /// (LastErrorFlagRuntimeCodes.al RecordRef_EmptyTable_NoFilter_ExcludeFlag `equal_true`; corpus pull request 1327). Other
    /// search strings, tables with rows, other keys and temporary records are not measured.
    pub(crate) fn record_not_found_origin(
        &self,
        t: &TableDef,
        f: EmptyFind,
        empty_table: bool,
        default_key: bool,
    ) -> ErrorOrigin {
        let version = match f.via {
            FindVia::Record => self.has(al_version::Gate::EmptyTableFindExcludeFlagFrom28),
            FindVia::RecordRef => true,
        };
        if f.which == "=" && empty_table && default_key && !f.temporary && version {
            ErrorOrigin::Redacted(format!(
                "The {} does not exist. Identification fields and values: <redacted>",
                self.caption(t)
            ))
        } else {
            ErrorOrigin::Unknown
        }
    }
}
