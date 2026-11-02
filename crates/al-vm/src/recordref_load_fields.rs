//! `RecordRef.SetLoadFields`, `AddLoadFields`, `AreFieldsLoaded` and `LoadFields`: the load set is given by field numbers;
//! the rest is the Record rule (partial.rs).

use crate::Session;
use crate::records::Rec;
use al_builtins::describe::Describe;
use al_builtins::native::Signal;
use al_builtins::record_op::RecordOp;
use al_builtins::value::Value;
use al_data::TableDef;

impl Session {
    /// A number no field has is an error that names it (record/TestRecordPartialLoad.al
    /// PartialLoad_SetLoadFields_UnknownFieldNo_Throws). `LoadFields` reads the fields of the row the ref holds as
    /// `Record.LoadFields` does (fieldref/lt-fieldref-testfield-types/RecordRefLoadFieldsList.al:13-50); its unknown number
    /// is not pinned.
    pub(crate) fn recordref_load_fields(
        &mut self,
        method: &str,
        rec: &Rec,
        t: &TableDef,
        args: &[Value],
    ) -> Result<Value, Signal> {
        let mut positions = Vec::with_capacity(args.len());
        for a in args.iter() {
            let Value::Integer(n) = a else {
                return Err(Signal::Unsupported(format!("RecordRef.{method} with {}", a.describe())));
            };
            match t.fields.iter().position(|f| i64::from(f.id) == i64::from(*n)) {
                Some(p) => positions.push(Value::Integer(p as i32)),
                // One unknown number raises FieldNotFound naming it (misc/lt-open-overloads-collections-isolated/
                // P107RecRefNames.al:159-171, records/W4PlatformAndMeta.al:517-518). Which number a call with
                // several arguments names is not pinned.
                None if args.len() == 1 && matches!(method, "SetLoadFields" | "AddLoadFields") => {
                    return Err(self.field_not_found(t, &n.to_string()));
                }
                None => return Err(Signal::Error(Box::new(al_builtins::native::unpinned_error()))),
            }
        }
        match method {
            "LoadFields" => self.load_fields_row(rec, t, &positions),
            "AreFieldsLoaded" => self.load_fields_op(RecordOp::AreFieldsLoaded, rec, t, &positions),
            _ => self.load_fields_op(
                RecordOp::from_name(method).expect("a RecordRef load method"),
                rec,
                t,
                &positions,
            ),
        }
    }
}
