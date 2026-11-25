//! What AL code reached of the table data between two points of a test codeunit's run, as the cells of corpus PRs 1465,
//! 1468, 1572 and 1573 reach it: [`Reached`] and its kinds by role, [`Interval`]. Read by al-testrun
//! `isolation/failed_test.rs` and by the rollback of an asserterror (`tx_changes.rs`). Entry points: `Reached::of`,
//! `Interval::of`.

use super::{DataAccess, RefLoads};

/// What a body reached of the table data. A table call is a Record method or a RecordRef, FieldRef or KeyRef method al-vm
/// `measured_ref_method` names, on a RecordRef that is not temporary and of the session's company ([`DataAccess`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reached {
    /// No table was read or written, no table call ran, and no other RecordRef, FieldRef, KeyRef or Query method or
    /// session builtin other than NumberSequence was called.
    Nothing,
    /// As `Nothing`, except that RecordRef.Open and RecordRef.Close ran, and no other table call.
    RefOpenOnly,
    /// Tables were read and none was written, every read inside a Record method call, and nothing else was called.
    RecordReads,
    /// As `RecordReads`, and a Record method call wrote a table.
    RecordWrites,
    /// Tables were read and none was written, every read inside a RecordRef table call, Record calls that read no table
    /// and CompanyName calls aside, and nothing else was called.
    RefReads,
    /// Every table read and write ran inside a table call, a RecordRef one among them read or wrote a table, one wrote, and
    /// nothing else was called but CompanyName.
    RefWrites,
    /// Anything else.
    Other,
}

impl Reached {
    /// What AL code reached between `from` and `to`.
    pub fn of(from: &DataAccess, to: &DataAccess) -> Reached {
        // Page calls are read only by `measured_leaves`.
        let pages = |a: &DataAccess| DataAccess { page_calls: 0, ..*a };
        if pages(from) == pages(to) {
            return Reached::Nothing;
        }
        let delta = |count: fn(&DataAccess) -> u64| count(to).saturating_sub(count(from));
        let store = delta(|a| a.store);
        let refs = delta(|a| a.ref_calls) > 0;
        let writes = delta(|a| a.table_writes) > 0;
        // CompanyName counts only in a body that read through a RecordRef, as GetNewLineNo does (SqRecordRefFail.al:80).
        let company_name = delta(|a| a.company_name_calls) > 0;
        if delta(|a| a.other_calls) > 0 || (company_name && !(refs && store > 0)) {
            return Reached::Other;
        }
        if store == 0 {
            let open_close = delta(|a| a.ref_open_close_calls) == delta(|a| a.ref_calls);
            return match refs && open_close && delta(|a| a.record_calls) == 0 {
                true => Reached::RefOpenOnly,
                false => Reached::Other,
            };
        }
        match (delta(|a| a.table_store) == store, refs, writes) {
            (true, false, false) => Reached::RecordReads,
            (true, false, true) => Reached::RecordWrites,
            (true, true, false) if delta(|a| a.ref_store) == store => Reached::RefReads,
            (true, true, true) if delta(|a| a.ref_store) > 0 => Reached::RefWrites,
            _ => Reached::Other,
        }
    }
}

/// The Record calls of cells R14R and R2R of corpus PR 1468 (LastLineByRecord, SqRecordRefFail.al:118) between `from` and
/// `to`: Record.SetRange and exactly one Record.FindLast, which did not fail, and every table read inside that FindLast.
/// Any other Record call (SetAutoCalcFields, LockTable, SetLoadFields, FilterGroup, a second FindLast) or no SetRange is
/// not measured.
fn reads_by_find_last(from: &DataAccess, to: &DataAccess) -> bool {
    let delta = |count: fn(&DataAccess) -> u64| count(to).saturating_sub(count(from));
    let (find_last, set_range) = (delta(|a| a.find_last_calls), delta(|a| a.set_range_calls));
    find_last == 1
        && set_range > 0
        && delta(|a| a.record_calls) == find_last + set_range
        && delta(|a| a.find_last_failures) == 0
        && delta(|a| a.find_last_store) == delta(|a| a.store)
}

/// The Record calls of PlainRead and UpdLockRead of corpus PR 1572 (SqFailedSeqFixture.al:315, :322) between `from` and
/// `to`: exactly one Record.FindFirst, after Record.ReadIsolation(IsolationLevel::UpdLock) or alone, and every table read
/// inside that FindFirst.
fn reads_by_find_first(from: &DataAccess, to: &DataAccess) -> bool {
    let delta = |count: fn(&DataAccess) -> u64| count(to).saturating_sub(count(from));
    let (find_first, upd_lock) = (delta(|a| a.find_first_calls), delta(|a| a.upd_lock_calls));
    find_first == 1
        && upd_lock <= 1
        && delta(|a| a.record_calls) == find_first + upd_lock
        && delta(|a| a.find_first_store) == delta(|a| a.store)
}

/// What AL code reached between two transaction ends, as the cells of corpus PRs 1465, 1468 and 1572 reach it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interval {
    /// Nothing: no table, no table call, no other call counted (NumberSequence calls are not).
    Untouched,
    /// RecordRef.Open and RecordRef.Close alone, which read no table (PR 1468 OpenOnly, SqRecordRefFail.al:141).
    OpenClose,
    /// Record calls that wrote a table ([`Reached::RecordWrites`]).
    RecordWrite,
    /// Record.SetRange and one Record.FindLast ([`reads_by_find_last`]).
    FindLast,
    /// One Record.FindFirst ([`reads_by_find_first`]).
    FindFirst,
    /// RecordRef table calls that read or wrote a table, beside Record calls ([`Reached::RefReads`],
    /// [`Reached::RefWrites`]).
    Ref,
    /// As `Ref`, with exactly the RecordRef.ReadIsolation and SetLoadFields calls of the helper of PR 1572 for a sequence
    /// it does not find (GetLastEntryNoFromTable, SqFailedSeqFixture.al:189, called by :116 and :176); `writes`: a table
    /// was written.
    Helper { writes: bool },
    /// Anything else.
    Unknown,
}

impl Interval {
    pub fn of(from: &DataAccess, to: &DataAccess) -> Interval {
        let delta = |count: fn(&DataAccess) -> u64| count(to).saturating_sub(count(from));
        if delta(|a| a.temporary_record_calls) > 0 {
            return Interval::Unknown;
        }
        let loads = RefLoads {
            read_uncommitted: delta(|a| a.ref_loads.read_uncommitted),
            upd_lock: delta(|a| a.ref_loads.upd_lock),
            other_isolation: delta(|a| a.ref_loads.other_isolation),
            set_load_fields: delta(|a| a.ref_loads.set_load_fields),
        };
        if loads != RefLoads::default() {
            // Exactly the two GetLastEntryNoFromTable calls the helper makes for a sequence it does not find (ValidateSeqNo
            // with ReadUncommitted, then CreateNewTableSequence with UpdLock, each with one SetLoadFields), as in S1V, S1Z,
            // S1A and S2E; these calls are other calls, and what else ran must be what `Ref` names.
            let helper = RefLoads {
                read_uncommitted: 1,
                upd_lock: 1,
                other_isolation: 0,
                set_load_fields: 2,
            };
            if loads != helper {
                return Interval::Unknown;
            }
            let others = DataAccess {
                other_calls: to
                    .other_calls
                    .saturating_sub(helper.read_uncommitted + helper.upd_lock + helper.set_load_fields),
                ..*to
            };
            return match Reached::of(from, &others) {
                Reached::RefReads => Interval::Helper { writes: false },
                Reached::RefWrites => Interval::Helper { writes: true },
                _ => Interval::Unknown,
            };
        }
        match Reached::of(from, to) {
            Reached::Nothing => Interval::Untouched,
            Reached::RefOpenOnly => Interval::OpenClose,
            Reached::RecordWrites => Interval::RecordWrite,
            Reached::RecordReads if reads_by_find_last(from, to) => Interval::FindLast,
            Reached::RecordReads if reads_by_find_first(from, to) => Interval::FindFirst,
            Reached::RefReads | Reached::RefWrites => Interval::Ref,
            _ => Interval::Unknown,
        }
    }

    /// What was reached over this interval and then `next`: two intervals that each reached something are not measured.
    pub fn then(self, next: Interval) -> Interval {
        match (self, next) {
            (Interval::Untouched, other) | (other, Interval::Untouched) => other,
            _ => Interval::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn access(store: u64, table_store: u64, table_writes: u64, record_calls: u64, other_calls: u64) -> DataAccess {
        DataAccess {
            store,
            table_store,
            table_writes,
            record_calls,
            other_calls,
            ..DataAccess::default()
        }
    }

    fn by_ref(a: DataAccess, ref_store: u64, ref_calls: u64) -> DataAccess {
        DataAccess {
            ref_store,
            ref_calls,
            ..a
        }
    }

    #[test]
    fn only_table_access_inside_record_calls_is_record_access() {
        let from = access(5, 3, 1, 2, 1);
        assert_eq!(Reached::of(&from, &from), Reached::Nothing);
        assert_eq!(Reached::of(&from, &access(9, 7, 1, 4, 1)), Reached::RecordReads);
        assert_eq!(Reached::of(&from, &access(9, 7, 2, 4, 1)), Reached::RecordWrites);
        // A Record call that reached no table (a temporary record, Init).
        assert_eq!(Reached::of(&from, &access(5, 3, 1, 3, 1)), Reached::Other);
        // A table read outside a Record call (a page, a native).
        assert_eq!(Reached::of(&from, &access(9, 6, 2, 4, 1)), Reached::Other);
        // A RecordRef method no cell of corpus PR 1468 calls, a Query or session builtin call.
        assert_eq!(Reached::of(&from, &access(9, 7, 2, 4, 2)), Reached::Other);
    }

    #[test]
    fn recordref_table_calls_are_told_apart_from_record_calls() {
        let from = access(5, 3, 1, 2, 1);
        let open_close = |a: DataAccess, n: u64| DataAccess {
            ref_open_close_calls: n,
            ..a
        };
        // Open and Close only.
        assert_eq!(
            Reached::of(&from, &open_close(by_ref(from, 0, 2), 2)),
            Reached::RefOpenOnly
        );
        // Open, Close and RecordRef calls that read no table (KeyIndex, FieldRef.SetRange or Value, GetTable and Number).
        assert_eq!(Reached::of(&from, &open_close(by_ref(from, 0, 4), 2)), Reached::Other);
        assert_eq!(Reached::of(&from, &by_ref(from, 0, 2)), Reached::Other);
        // Open and Close, and a Record write: the RecordRef calls read and wrote no table.
        assert_eq!(
            Reached::of(&from, &open_close(by_ref(access(9, 7, 2, 3, 1), 0, 2), 2)),
            Reached::Other
        );
        // Open and Close, and a Record call that reached no table.
        assert_eq!(
            Reached::of(&from, &by_ref(access(5, 3, 1, 3, 1), 0, 2)),
            Reached::Other
        );
        // Every read through a RecordRef.
        assert_eq!(
            Reached::of(&from, &by_ref(access(9, 7, 1, 2, 1), 4, 3)),
            Reached::RefReads
        );
        // Reads through a RecordRef and through a Record.
        assert_eq!(
            Reached::of(&from, &by_ref(access(9, 7, 1, 3, 1), 2, 3)),
            Reached::Other
        );
        // A RecordRef read and a Record write (GetNewRecNo, then Insert).
        assert_eq!(
            Reached::of(&from, &by_ref(access(9, 7, 2, 3, 1), 2, 3)),
            Reached::RefWrites
        );
        // CompanyName counts only beside a RecordRef read (GetNewLineNo).
        let named = |a: DataAccess| DataAccess {
            company_name_calls: 1,
            ..a
        };
        assert_eq!(
            Reached::of(&from, &named(by_ref(access(9, 7, 2, 3, 1), 2, 3))),
            Reached::RefWrites
        );
        assert_eq!(Reached::of(&from, &named(access(9, 7, 2, 4, 1))), Reached::Other);
        assert_eq!(
            Reached::of(&from, &named(open_close(by_ref(from, 0, 2), 2))),
            Reached::Other
        );
    }
}
