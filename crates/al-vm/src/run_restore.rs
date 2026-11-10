//! The caller's record after a failed guarded `Codeunit.Run(Id, Rec)`.

use al_builtins::value::RecordState;
use al_builtins::value::Value;
use std::cell::RefCell;
use std::rc::Rc;

/// The caller's record and what it held before the run.
pub(crate) struct CallerRecord(Rc<RefCell<RecordState>>, RecordState);

impl CallerRecord {
    /// Takes the record's state (fields, keys, filters of every group, marks, position, calculated fields).
    pub(crate) fn take(rec: &Rc<RefCell<RecordState>>) -> CallerRecord {
        let b = rec.borrow();
        let mut state = b.clone();
        state.is_dirty_base = b.is_dirty_base.copy();
        CallerRecord(rec.clone(), state)
    }

    /// A failed run that returned false leaves the caller's record exactly as before the call, whatever OnRun did to Rec
    /// (codeunits/run-onrun/RunFailedRecordState.al:287-484, every `fails` cell: the result-used static and variable forms, `if not`,
    /// field, key, filter and mark changes, a failure raised directly, in a nested procedure, in a subscriber).
    /// The statement form under asserterror keeps the changes (RunFailedRecordState.al:305-307) and does not call this.
    /// The rows of a temporary table are not part of the state: the run's row changes stay (:457-484).
    pub(crate) fn restore(self) {
        *self.0.borrow_mut() = self.1;
    }
}

pub(crate) fn caller_record(arg: Option<&Value>) -> Option<CallerRecord> {
    match arg {
        Some(Value::Record(rc)) => Some(CallerRecord::take(rc)),
        _ => None,
    }
}
