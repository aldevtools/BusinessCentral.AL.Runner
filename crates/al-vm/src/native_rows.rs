//! Rows the test runner writes before a test codeunit runs (ADR-066 decision 2): the "AL Test Suite" and
//! "Test Method Line" rows Microsoft's runner reads, written as `Insert(false)` writes them.

use crate::Session;
use al_builtins::native::Signal;
use al_builtins::ops;
use al_builtins::record_op::RecordOp;
use al_builtins::value::{RtType, Value};

impl Session {
    /// Inserts a row of table object `table`: the fields named by id in `values` hold those values (converted to the
    /// field's type), the others their InitValue, as a record variable initialized and inserted with `Insert(false)`.
    /// The record is returned, positioned on the new row.
    pub fn insert_row(&mut self, table: usize, values: &[(u32, Value)]) -> Result<Value, Signal> {
        let rec = self.record_with(table, values)?;
        let Value::Record(r) = &rec else {
            unreachable!("a record")
        };
        self.record_op(RecordOp::Insert, r, &mut [Value::Boolean(false)])?;
        Ok(rec)
    }

    /// Deletes every row of table object `table` in the session company, as `DeleteAll(false)` on an unfiltered record.
    pub fn delete_all_rows(&mut self, table: usize) -> Result<(), Signal> {
        let rec = self.record_with(table, &[])?;
        let Value::Record(r) = &rec else {
            unreachable!("a record")
        };
        self.record_op(RecordOp::DeleteAll, r, &mut [Value::Boolean(false)])?;
        Ok(())
    }

    /// A record variable of table object `table`, initialized, with the fields named by id in `values` set (converted
    /// to the field's type). Nothing is written.
    pub fn record_with(&mut self, table: usize, values: &[(u32, Value)]) -> Result<Value, Signal> {
        let Value::Record(rec) = self.default_value(&RtType::Record(table as u32, false)) else {
            return Err(Signal::Unsupported(format!("object {table} is not a table")));
        };
        let t = self.table(&rec)?;
        for (id, v) in values {
            let Some(at) = t.fields.iter().position(|f| f.id == *id) else {
                return Err(Signal::Unsupported(format!("table {} has no field {id}", t.name)));
            };
            let v = ops::convert(v.clone(), &t.fields[at].ty)
                .map_err(|_| Signal::Unsupported(format!("field {id} of table {}: a value of another type", t.name)))?;
            rec.borrow_mut().fields[at] = v;
        }
        Ok(Value::Record(rec))
    }
}
