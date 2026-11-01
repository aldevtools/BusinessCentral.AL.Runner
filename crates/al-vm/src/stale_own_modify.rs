//! A Modify(true) through a stale buffer from the variable's own Insert two or three Commits back, in a transaction that
//! wrote nothing and took no lock: the table's OnModify trigger runs on the buffer, with xRec the buffer itself, then the
//! write raises DB:RecordChanged and the row stays (record/lt-sb-stale-own-modify/Y27StaleOtherVarTests.al:269-299,
//! Y27_2C_Nothing and Y27_3C_Nothing, `buffer` `Changed|100|1|1|100|Changed`; al-runner issue 6522). A table extension's
//! modify triggers, modify event subscribers and the OnDatabaseModify global trigger are not pinned there: with any of
//! them the error comes before the trigger, as before.

use crate::globaltriggers::GlobalOp;
use crate::Session;
use al_builtins::native::Signal;
use al_builtins::value::{RecordState, Value};
use al_data::TableDef;
use std::cell::RefCell;
use std::rc::Rc;

type Rec = Rc<RefCell<RecordState>>;

impl Session {
    /// The error of a refused stale Modify, after the OnModify trigger when the buffer has the Y27 shape.
    pub(crate) fn stale_own_insert_modify_refused(&mut self, t: &TableDef, rec: &Rec, run: bool) -> Result<Signal, Signal> {
        let i = t.fields.iter().position(|f| f.id == 2_000_000_005).expect("a table with SystemRowVersion");
        let own_insert = {
            let r = rec.borrow();
            let mine = if let Value::BigInteger(n) = &r.fields[i] { *n } else { 0 };
            r.row_origin
                .filter(|o| o.version == mine)
                .is_some_and(|o| o.own_write && (o.commits + 2 == self.commits || o.commits + 3 == self.commits))
        };
        let untouched_tx =
            self.tx.writes.is_empty() && !self.tx.rolled_back && !self.tx.lock_tx && self.tx.lock_unpinned.is_none();
        let mut hooks = false;
        for n in ["OnBeforeModify", "OnModify", "OnAfterModify"] {
            hooks |= !self.ext_triggers(t, n).is_empty();
        }
        hooks = hooks
            || self.listened(t, "onbeforemodifyevent", None)?
            || self.listened(t, "onaftermodifyevent", None)?
            || self.global_enabled(t, GlobalOp::Modify)?;
        if run && own_insert && untouched_tx && !hooks {
            if let Some(tr) = self.table_trigger(t, "OnModify") {
                let x = Self::xrec_handle(rec, rec.borrow().fields.clone());
                self.run_trigger(t.object as usize, tr, rec, Some(&x))?;
            }
        }
        self.record_changed(t, rec)
    }
}
