//! `RecordRef.SetLoadFields`, `AddLoadFields`, `AreFieldsLoaded`, `LoadFields` and `SetAutoCalcFields`: the fields are given by
//! number; the rest is the Record rule (partial.rs, flowfields.rs).

use crate::Session;
use crate::records::Rec;
use al_builtins::describe::Describe;
use al_builtins::native::Signal;
use al_builtins::record_op::RecordOp;
use al_builtins::value::Value;
use al_data::TableDef;

/// What one field number argument names.
enum Arg {
    Field(usize),
    /// Field number 0.
    Zero,
    /// A number no field of the table has.
    Unknown(i32),
}

fn args_of(method: &str, t: &TableDef, args: &[Value]) -> Result<Vec<Arg>, Signal> {
    args.iter()
        .map(|a| {
            let Value::Integer(n) = a else {
                return Err(Signal::unsupported(format!("RecordRef.{method} with {}", a.describe())));
            };
            Ok(match t.fields.iter().position(|f| i64::from(f.id) == i64::from(*n)) {
                Some(p) => Arg::Field(p),
                None if *n == 0 => Arg::Zero,
                None => Arg::Unknown(*n),
            })
        })
        .collect()
}

fn positions(args: &[Arg]) -> Vec<Value> {
    args.iter()
        .filter_map(|a| match a {
            Arg::Field(p) => Some(Value::Integer(*p as i32)),
            _ => None,
        })
        .collect()
}

impl Session {
    /// A number no field has is an error that names it (record/TestRecordPartialLoad.al
    /// PartialLoad_SetLoadFields_UnknownFieldNo_Throws). `LoadFields` reads the fields of the row the ref holds as
    /// `Record.LoadFields` does (recordref-fieldref/fieldref/RecordRefLoadFieldsList.al:13-50); its unknown number
    /// is not pinned.
    ///
    /// Field number 0 (recordref-fieldref/recordref/Y71RecRefLoadFields.al:293-525): SetLoadFields and AddLoadFields skip it
    /// silently, alone or among other numbers (SetLoadFields(0) loads the primary key only), and AreFieldsLoaded counts it as
    /// loaded. An unknown number among several raises FieldNotFound naming the first unknown one in SetLoadFields and
    /// AddLoadFields (:347-403), and makes AreFieldsLoaded answer No without an error (:486-525). The result of SetLoadFields or
    /// AddLoadFields with a 0, and 0 in LoadFields, are not pinned.
    pub(crate) fn recordref_load_fields(
        &mut self,
        method: &str,
        rec: &Rec,
        t: &TableDef,
        args: &[Value],
    ) -> Result<Value, Signal> {
        let parsed = args_of(method, t, args)?;
        let zero = parsed.iter().any(|a| matches!(a, Arg::Zero));
        let unknown = parsed.iter().find_map(|a| match a {
            Arg::Unknown(n) => Some(*n),
            _ => None,
        });
        let positions = positions(&parsed);
        match method {
            "AreFieldsLoaded" => {
                if positions.is_empty() || unknown.is_some() {
                    // Pinned on a ref that fetched a partial row (SetLoadFields(2) + FindFirst).
                    if rec.borrow().load.buffer.is_none() {
                        return Err(Signal::Unsupported(
                            "RecordRef.AreFieldsLoaded of only field number 0 or of an unknown number on a ref that fetched no row (not pinned)".into(),
                        ));
                    }
                    if positions.is_empty() {
                        return Ok(Value::Boolean(unknown.is_none()));
                    }
                }
                let known = self.load_fields_op(RecordOp::AreFieldsLoaded, rec, t, &positions)?;
                Ok(if unknown.is_some() {
                    Value::Boolean(false)
                } else {
                    known
                })
            }
            "LoadFields" => {
                if zero || unknown.is_some() {
                    return Err(Signal::Error(Box::new(al_builtins::native::unpinned_error())));
                }
                // A FlowField or a FlowFilter field follows the Record rule in every state (record/load-fields/
                // Y87RecordRefLoadFields.al, codeunit 94655: never fetched, after Get, after Get under SetLoadFields, after
                // FindFirst, key assigned; statement and function form).
                self.load_fields_row(rec, t, &positions)
            }
            _ => {
                if let Some(n) = unknown {
                    return Err(self.field_not_found(t, &n.to_string()));
                }
                if zero && self.ret_used {
                    return Err(Signal::unsupported(format!(
                        "the result of RecordRef.{method} with field number 0 (not pinned)"
                    )));
                }
                if zero && positions.is_empty() && method == "SetLoadFields" {
                    rec.borrow_mut().load.request = Some(Vec::new());
                    return Ok(Value::Null);
                }
                // AddLoadFields(0) adds nothing; it is not the call without fields, which goes back to the full load
                // (recordref-fieldref/recordref/Y71RecRefLoadFields.al:180-184, :380-382 `add_zero|loaded`).
                if zero && positions.is_empty() {
                    return Ok(Value::Null);
                }
                self.load_fields_op(
                    RecordOp::from_name(method).expect("a RecordRef load method"),
                    rec,
                    t,
                    &positions,
                )
            }
        }
    }

    /// `RecordRef.SetAutoCalcFields`: a read then calculates the fields as for a Record (record/TestSetAutoCalcFieldsOnGet.Codeunit.al:
    /// 112-124 RecordRef_SetAutoCalcFields_ThenGet_Calculates pins Get with one FlowField; the other reads share the Record rule).
    /// The numbers are checked in order (recordref-fieldref/recordref/Y71RecRefLoadFields.al:405-484): 0 names the timestamp field
    /// and raises `The timestamp field in the <table> table must be a FlowField.` (DB:Ok), an unknown number raises FieldNotFound,
    /// and a normal field the FlowField error that names it.
    pub(crate) fn recordref_set_auto_calc_fields(
        &mut self,
        rec: &Rec,
        t: &TableDef,
        args: &[Value],
    ) -> Result<Value, Signal> {
        let table = rec.borrow().table as usize;
        let mut positions = Vec::with_capacity(args.len());
        for a in args_of("SetAutoCalcFields", t, args)? {
            match a {
                Arg::Field(p) => {
                    if !self.is_blob_field(table, p) {
                        self.check_calc_field(table, p)?;
                    }
                    positions.push(Value::Integer(p as i32));
                }
                Arg::Zero => {
                    return al_builtins::native::raise(
                        format!(
                            "The timestamp field in the {} table must be a FlowField.",
                            self.table_caption(table)
                        ),
                        "DB:Ok",
                    );
                }
                Arg::Unknown(n) => return Err(self.field_not_found(t, &n.to_string())),
            }
        }
        self.flow_op_asked(
            RecordOp::SetAutoCalcFields,
            rec,
            &positions,
            crate::flowfields::Asked::Other,
        )
    }
}
