//! `SetLoadFields` and `AddLoadFields` on a Record or a RecordRef: the request they leave and what the function forms answer.
//!
//! Pinned by record/lt-y67-addloadfields-flow/Y67AddLoadFlow.al:375-1517 (writer Y67, real BC 29, the same on 27.5 and 28.5): the
//! statement and function forms of both methods, on a Record and on a RecordRef, with a normal field, a FlowField and a FlowFilter
//! field alone and among several in both orders and no field at all, in four request states (none, after `SetLoadFields(Name)`,
//! after `SetLoadFields()`, after `SetLoadFields(Name, Qty)`). Earlier pins: record/lt-priority-assumptions/PriorityAssumptions.al:
//! 282-338, record/lt-loadfields-pagesource-validate/P81LoadFields.al:991-1028, record/TestRecordPartialLoad.al:61-99,
//! record/TestRecordExtended.al:58-65.

use crate::Session;
use crate::records::Rec;
use al_builtins::native::Signal;
use al_builtins::record_op::RecordOp;
use al_builtins::value::{RtType, Value};
use al_data::TableDef;

fn unsupported<T>(what: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(what.into()))
}

/// The field ids of the system fields (SystemId and the four audit fields), which every load holds.
const FIRST_SYSTEM_FIELD: u32 = 2_000_000_000;

impl Session {
    /// `SetLoadFields(wanted)` or `AddLoadFields(wanted)` (field positions, duplicates removed).
    pub(crate) fn set_or_add_load_fields(
        &mut self,
        method: RecordOp,
        rec: &Rec,
        t: &TableDef,
        wanted: Vec<usize>,
    ) -> Result<Value, Signal> {
        let engine = self.engine.clone();
        let schema = engine.program.objects[t.object as usize].table.as_ref();
        let stored = |p: usize| schema.is_some_and(|s| s.fields.get(p).is_some_and(|f| f.stored));
        let no_args = wanted.is_empty();
        let flow = wanted.iter().copied().find(|&p| !stored(p));
        if method == RecordOp::AddLoadFields {
            // The function form never raises. `AddLoadFields()` answers Yes in every state and drops the request; with a FlowField or a FlowFilter among
            // the fields, wherever it stands, it answers No and adds nothing (Name+Cnt+Qty after SetLoadFields(Name) leaves Qty
            // unloaded); without a request it answers No for any field; with one, Yes when it adds a field the request lacks
            // (Y67AddLoadFlow.al Record_*_AddFunction, RecordRef_*_AddFunction).
            //
            // The statement form raises DB:Ok for the first FlowField or FlowFilter among the fields, whatever its position and the
            // other fields, in every state, and adds nothing (Record_*_AddStatement `raised:Cnt|DB:Ok`, `raised:Flt|DB:Ok`).
            if let Some(p) = flow {
                if self.ret_used {
                    return Ok(Value::Boolean(false));
                }
                let name = schema.and_then(|s| s.fields.get(p)).map_or("", |f| f.name.as_str());
                return al_builtins::native::raise(
                    format!(
                        "The field '{name}' in the '{}' table is not valid for auto loading and cannot be passed to calls to AddLoadFields, SetLoadFields, or LoadFields.",
                        t.name
                    ),
                    "DB:Ok",
                );
            }
            let mut r = rec.borrow_mut();
            // `AddLoadFields()` with no field goes back to the full load, as `SetLoadFields()` does (Record_AfterSetName_AddStatement
            // and _AddFunction `1 none` loaded=YesYesYesYesYesYes).
            if no_args {
                r.load.request = None;
            }
            let added = match &mut r.load.request {
                // Without a request the load is already full: nothing to add.
                None => false,
                Some(req) => {
                    let mut added = false;
                    for p in wanted {
                        if !req.contains(&p) {
                            req.push(p);
                            added = true;
                        }
                    }
                    added
                }
            };
            return Ok(if self.ret_used {
                Value::Boolean(no_args || added)
            } else {
                Value::Null
            });
        }
        // SetLoadFields takes a FlowField and a FlowFilter and adds nothing to the request for them (PriorityAssumptions.al:282-312,
        // P81LoadFields.al:273-304); with no field at all it goes back to the full load (TestRecordPartialLoad.al:61-99).
        let new: Vec<usize> = wanted.into_iter().filter(|&p| stored(p)).collect();
        let answer = if self.ret_used {
            Some(no_args || self.load_set_changes(rec, t, &new)?)
        } else {
            None
        };
        rec.borrow_mut().load.request = if no_args { None } else { Some(new) };
        Ok(answer.map_or(Value::Null, Value::Boolean))
    }

    /// What the function form of `SetLoadFields` with fields answers: No when the stored fields it names are the fields the record
    /// would load already, Yes otherwise. Pinned (Y67AddLoadFlow.al Record_*_SetFunction, RecordRef_*_SetFunction): without a
    /// request (and after `SetLoadFields()`) `Name, Qty` and `Name, Cnt, Qty` answer No, every other list Yes; after
    /// `SetLoadFields(Name)` every list whose stored fields are `Name` answers No, a list of FlowFields and FlowFilters only and `Name,
    /// Qty` answer Yes; after `SetLoadFields(Name, Qty)` as without a request. The full load holds every stored field but the
    /// primary key and the system fields, which every load holds.
    fn load_set_changes(&self, rec: &Rec, t: &TableDef, new: &[usize]) -> Result<bool, Signal> {
        let pk = t.keys.first().cloned().unwrap_or_default();
        let counted = |p: usize| !pk.contains(&p) && t.fields.get(p).is_some_and(|f| f.id < FIRST_SYSTEM_FIELD);
        let mut new: Vec<usize> = new.iter().copied().filter(|&p| counted(p)).collect();
        let mut current: Vec<usize> = match &rec.borrow().load.request {
            Some(req) => req.iter().copied().filter(|&p| counted(p)).collect(),
            None => {
                let media = |p: usize| matches!(&t.fields[p].ty, RtType::Library(n) if n.eq_ignore_ascii_case("Blob") || n.eq_ignore_ascii_case("Media") || n.eq_ignore_ascii_case("MediaSet"));
                let full: Vec<usize> = (0..t.fields.len())
                    .filter(|&p| t.fields[p].stored && counted(p) && !media(p))
                    .collect();
                // Whether the full load counts a Blob or Media field is not pinned: a list that names every other stored field
                // could be the full load or not. A list that leaves out another stored field differs either way.
                let names_every_other = full.iter().all(|p| new.contains(p));
                if names_every_other && (0..t.fields.len()).any(|p| t.fields[p].stored && counted(p) && media(p)) {
                    return unsupported(
                        "the result of SetLoadFields without a request of every stored field of a table with a Blob or Media field (whether the full load counts it is not pinned)",
                    );
                }
                full
            }
        };
        new.sort_unstable();
        new.dedup();
        current.sort_unstable();
        current.dedup();
        Ok(new != current)
    }
}
