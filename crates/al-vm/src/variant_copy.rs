//! A temporary record going from a Variant into a Variant (`V := W` where `W` is a Variant variable, parameter or
//! procedure result; al-runner issue 8070). The owner of that one move: `exec/compile.rs` emits `Op::StoreVariantCopy`
//! for an assignment whose right-hand side is one of the forms in [`VariantSource`], and [`Session::store_variant_copy`]
//! decides.
//!
//! The pinned neighbors stay where they are: a record variable put into a Variant is a by-value copy over an empty table
//! of its own (recordref-fieldref/recordref/PWRecordMisc.al:234-250, `Interp::write_as`), a temporary record handed to a
//! Variant parameter keeps its table (PWRecordMisc.al:221-231, `call_inner`).
//!
//! Not pinned: the corpus has no case of a Variant assigned from another Variant that holds a temporary record. The
//! reporter's run on real Business Central (al-runner issue 8070) measured two shapes, and only those run, under the assumption
//! `variant-from-variant-temp-record-keeps-rows` ([`measured`]): the new Variant holds a record over the same temporary
//! table. Every other shape of a temporary record going from a Variant into a Variant is refused; anything that is not a
//! temporary record keeps main's store.

use crate::{Frame, Session};
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::native::Signal;
use al_builtins::value::{RecordState, RtType, Value};
use al_ir as ir;
use std::cell::RefCell;
use std::rc::Rc;

/// The right-hand side of an assignment, as far as it is known to be a Variant (`Convert(Variant, x)` is lowered for a
/// Variant source too, so the form of `x` is what tells).
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum VariantSource {
    /// A by-value Variant parameter of the running routine.
    ByValueParam,
    /// A `var` Variant parameter of the running routine.
    VarParam,
    /// A Variant local that is no parameter (a declared local or the named return value).
    Local,
    /// A procedure call: a Variant source when the callee returns a Variant. `on_variable`: called on a global
    /// variable (`L.Proc()` with `L` a global codeunit variable).
    Call {
        object: usize,
        routine: usize,
        on_variable: bool,
    },
    /// A global or an array element: a Variant source when its declared type is Variant.
    Place(ir::Place),
}

/// One `Op::StoreVariantCopy`: the place assigned and the form of the right-hand side.
#[derive(Debug, Clone)]
pub(crate) struct VariantCopySite {
    pub place: ir::Place,
    pub source: VariantSource,
}

/// The form of `inner`, the operand of `Convert(Variant, inner)` on the right of an assignment, when it may be a Variant;
/// `None` for every other form (main's store).
pub(crate) fn source_of(inner: &ir::Expr, locals: &[RtType], params: usize, by_var: &[bool]) -> Option<VariantSource> {
    match inner {
        ir::Expr::Load(ir::Place::Local(j)) if matches!(locals.get(*j), Some(RtType::Variant)) => {
            Some(if *j < params {
                if by_var.get(*j).copied().unwrap_or(false) {
                    VariantSource::VarParam
                } else {
                    VariantSource::ByValueParam
                }
            } else {
                VariantSource::Local
            })
        }
        ir::Expr::Load(p @ (ir::Place::Global(_) | ir::Place::Index(..))) => Some(VariantSource::Place(p.clone())),
        ir::Expr::Call {
            object,
            routine,
            instance,
            try_call: false,
            ..
        } => Some(VariantSource::Call {
            object: *object,
            routine: *routine,
            on_variable: matches!(instance.as_deref(), Some(ir::Expr::Load(ir::Place::Global(_)))),
        }),
        _ => None,
    }
}

impl Session {
    /// `Op::StoreVariantCopy`: `site.place := v`.
    pub(crate) fn store_variant_copy(&mut self, site: &VariantCopySite, v: Value, f: &mut Frame) -> Result<(), Signal> {
        let temp_record = match &v {
            Value::Record(rc) if rc.borrow().temporary => Some(rc.clone()),
            _ => None,
        };
        let Some(rc) = temp_record.filter(|_| self.variant_into_variant(site, f)) else {
            return self.store_as_main(&site.place, v, f);
        };
        let ir::Place::Local(i) = site.place else {
            return Err(refusal("the target is no local variable"));
        };
        if let Some(why) = self.unmeasured(&site.source, i, &rc.borrow(), f) {
            return Err(refusal(why));
        }
        // The reporter's shapes (al-runner issue 8070, the reporter's run on real Business Central): the new Variant holds a record over the source's
        // temporary table, a copy as a Variant parameter's is (`call_inner`).
        assume(Assumption::VariantFromVariantTempRecordKeepsRows);
        let st = rc.borrow().copy_as_unmeasured_variable();
        f.locals[i] = Value::Record(Rc::new(RefCell::new(st)));
        Ok(())
    }

    /// Whether both sides are Variants: the target's declared type, and the source's (the callee's return type, the
    /// global's or element's declared type).
    fn variant_into_variant(&self, site: &VariantCopySite, f: &Frame) -> bool {
        if !matches!(self.place_type(&site.place, f), RtType::Variant) {
            return false;
        }
        match &site.source {
            VariantSource::ByValueParam | VariantSource::VarParam | VariantSource::Local => true,
            VariantSource::Call { object, routine, .. } => matches!(
                self.engine.program.objects[*object].routines[*routine].ret,
                Some(RtType::Variant)
            ),
            VariantSource::Place(p) => matches!(self.place_type(p, f), RtType::Variant),
        }
    }

    /// Why the move into local `i` is not one of the two shapes the reporter measured, or `None` when it is. Each fact is
    /// one the reporter's repro has (al-runner issue 8070, `ReturnValueStoredInVariantVariable`,
    /// `ReturnValueStoredInVariantVariableThroughEvent`, `VariantParameterCopiedToVariantVariable`).
    fn unmeasured(&self, source: &VariantSource, i: usize, rec: &RecordState, f: &Frame) -> Option<&'static str> {
        let running = &self.engine.program.objects[f.object];
        let r = &running.routines[f.routine];
        // The routine: a procedure of a codeunit, no trigger (a test method and a local procedure in the repro).
        if !(running.kind == ir::ObjKind::Codeunit && !r.is_trigger) {
            return Some("the assignment is not in a procedure of a codeunit");
        }
        // The source: a by-value Variant parameter, or a procedure of a codeunit called on a global codeunit variable.
        let source_measured = match source {
            VariantSource::ByValueParam => true,
            VariantSource::Call {
                object, on_variable, ..
            } => *on_variable && self.engine.program.objects[*object].kind == ir::ObjKind::Codeunit,
            _ => false,
        };
        if !source_measured {
            return Some("the source is no by-value Variant parameter and no procedure of a global codeunit variable");
        }
        // The target: a declared Variant local, no parameter and no named return value, that holds nothing yet.
        if i < r.params || r.ret_local == Some(i) || !matches!(f.locals[i], Value::Null) {
            return Some("the target is no declared local Variant that holds nothing yet");
        }
        // The record: of a normal table declared temporary, in no other company, with the view a new variable has.
        let view_new = rec.filters.is_empty()
            && rec.filter_group == 0
            && rec.marks.is_empty()
            && !rec.marked_only
            && rec.key == 0
            && rec.adhoc.is_none()
            && rec.ascending
            && rec.field_descending == 0
            && rec.company.is_none()
            && rec.unpinned_buffer.is_none()
            && rec.run_page_link.is_none();
        if !view_new || self.engine.object_facts[rec.table as usize].temporary() {
            return Some(
                "the record has filters, marks, a key, a company of its own or a table of TableType Temporary",
            );
        }
        None
    }

    /// The store main makes (`Op::StoreLocal`, `Op::StorePlace`).
    fn store_as_main(&mut self, p: &ir::Place, v: Value, f: &mut Frame) -> Result<(), Signal> {
        match p {
            ir::Place::Local(i) if !matches!(v, Value::Record(_) | Value::Library(..) | Value::Blob(_)) => {
                f.locals[*i] = v;
                Ok(())
            }
            _ => self.write(p, v, f),
        }
    }
}

fn refusal(why: &str) -> Signal {
    Signal::unsupported(format!(
        "an assignment of a Variant holding a temporary record to another Variant: {why} (not pinned; al-runner issue 8070)"
    ))
}
