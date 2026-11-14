//! A temporary record going from a Variant into a Variant (`V := W` where `W` is a Variant variable, parameter, global,
//! array element or procedure result; al-runner issue 8070). The owner of that one move: `exec/compile.rs` emits
//! `Op::StoreVariantCopy` for an assignment whose right-hand side is one of the forms in [`VariantSource`], and
//! [`Session::store_variant_copy`] decides. Any other form of right-hand side (a ternary, a `try` call, an interface method)
//! keeps main's store and is not marked.
//!
//! The pinned neighbors stay where they are: a record variable put into a Variant is a by-value copy over an empty table
//! of its own (recordref-fieldref/recordref/PWRecordMisc.al:234-250, `Interp::write_as`), a temporary record handed to a
//! Variant parameter keeps its table (PWRecordMisc.al:221-231, `call_inner`).
//!
//! Not pinned: the corpus has no case of a Variant assigned from another Variant that holds a temporary record. The
//! reporter's run on real Business Central (al-runner issue 8070) measured two shapes, and only those run, under the
//! assumption `variant-from-variant-temp-record-keeps-rows` ([`Session::measured`]): the new Variant holds a record over the
//! same temporary table, and every write to that table refuses (`TableUnmeasured::Writes`). Every other temporary record
//! moved that way gets main's store, a by-value copy over an empty table of its own, and that table is marked
//! `TableUnmeasured::Rows`: its field values read as on main, every read of its rows and every write refuses
//! (`Session::store_for`). Anything that is not a temporary record keeps main's store.

use crate::{Frame, Session};
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::native::Signal;
use al_builtins::value::{RecordState, RtType, TableUnmeasured, Value};
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
    /// The place is a local that no other statement of the routine writes: no other assignment, `for` or `foreach`, no
    /// by-reference argument (`Clear`, a `var` parameter) ([`local_written_once`]).
    pub target_written_once: bool,
}

/// Whether `p` is a local whose only write in `body` is one assignment: the count of assignments, `for` and `foreach`
/// variables and by-reference arguments that name it is 1.
pub(crate) fn local_written_once(p: &ir::Place, body: &[ir::Stmt]) -> bool {
    let ir::Place::Local(i) = *p else {
        return false;
    };
    let names = |p: &ir::Place| root_local(p) == Some(i);
    let mut writes = 0;
    statement_writes(body, &names, &mut writes);
    al_ir::types::for_each_expr_post(body, &mut |e| {
        let (receiver, args) = match e {
            ir::Expr::Call { args, .. } | ir::Expr::InterfaceCall { args, .. } | ir::Expr::DotNet { args, .. } => {
                (None, args)
            }
            ir::Expr::Builtin { receiver, args, .. } => (receiver.as_deref(), args),
            _ => return,
        };
        writes += receiver
            .into_iter()
            .chain(args)
            .filter(|a| matches!(a, ir::Arg::Ref(p) if names(p)))
            .count();
    });
    writes == 1
}

fn root_local(p: &ir::Place) -> Option<usize> {
    match p {
        ir::Place::Local(i) => Some(*i),
        ir::Place::Index(b, _) | ir::Place::Field(b, _) | ir::Place::RecGlobal(b, ..) => root_local(b),
        ir::Place::Global(_) | ir::Place::BaseGlobal(_) | ir::Place::ExtGlobal(..) => None,
    }
}

/// Adds to `n` the statements of `body` that write a place `names` accepts (assignment targets, loop variables).
fn statement_writes(body: &[ir::Stmt], names: &dyn Fn(&ir::Place) -> bool, n: &mut usize) {
    for s in body {
        match s {
            ir::Stmt::Assign(p, _) => *n += usize::from(names(p)),
            ir::Stmt::For { var, body, .. } | ir::Stmt::Foreach { var, body, .. } => {
                *n += usize::from(names(var));
                statement_writes(body, names, n);
            }
            ir::Stmt::If(_, a, b) => {
                statement_writes(a, names, n);
                statement_writes(b, names, n);
            }
            ir::Stmt::While(_, b) | ir::Stmt::Repeat(b, _) | ir::Stmt::AssertError(b) => statement_writes(b, names, n),
            ir::Stmt::Case {
                branches, otherwise, ..
            } => {
                for (_, b) in branches {
                    statement_writes(b, names, n);
                }
                statement_writes(otherwise, names, n);
            }
            ir::Stmt::Expr(_)
            | ir::Stmt::Exit(_)
            | ir::Stmt::Break
            | ir::Stmt::Pos(_)
            | ir::Stmt::Assume(_)
            | ir::Stmt::TddMark(_)
            | ir::Stmt::Continue => {}
        }
    }
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
    /// `Op::StoreVariantCopy` for a value `Op::StoreLocal` would not store straight into a register: `site.place := v`.
    pub(crate) fn store_variant_copy(&mut self, site: &VariantCopySite, v: Value, f: &mut Frame) -> Result<(), Signal> {
        // Whether the record state and its table have no other holder than `v` (counted before `v` is cloned).
        let (source, lone) = match &v {
            Value::Record(rc) if rc.borrow().temporary => {
                let lone = Rc::strong_count(rc) == 1 && rc.borrow().temp_id.as_ref().is_some_and(|t| t.sharers() == 1);
                (Some(rc.clone()), lone)
            }
            _ => (None, false),
        };
        let Some(rc) = source.filter(|_| self.variant_into_variant(site, f)) else {
            return self.write(&site.place, v, f);
        };
        if let ir::Place::Local(i) = site.place
            && self.measured(site, i, lone, &rc.borrow(), f)
        {
            // The reporter's shapes (al-runner issue 8070, the reporter's run on real Business Central): the new Variant
            // holds a record over the source's temporary table, a copy as a Variant parameter's is (`call_inner`). Whether
            // a write through one holder shows through the other is not measured: every write to the table refuses.
            assume(Assumption::VariantFromVariantTempRecordKeepsRows);
            let st = rc.borrow().copy_as_unmeasured_variable();
            if let Some(t) = &st.temp_id {
                t.mark_unmeasured(TableUnmeasured::Writes);
            }
            f.locals[i] = Value::Record(Rc::new(RefCell::new(st)));
            return Ok(());
        }
        // Every other shape: main's store, a by-value copy over an empty table of its own (`Interp::write_as`). Whether
        // real Business Central leaves that table empty is not measured: its rows are marked, its field values read as on
        // main.
        let from = rc.borrow().temp_id.as_ref().map(|t| t.id());
        drop(rc);
        self.write(&site.place, v, f)?;
        if let Value::Record(copy) = self.read(&site.place, f)?
            && let Some(t) = &copy.borrow().temp_id
            && Some(t.id()) != from
        {
            t.mark_unmeasured(TableUnmeasured::Rows);
        }
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

    /// Whether the move into local `i` is one of the two shapes the reporter measured. Each fact is one the reporter's
    /// repro has (al-runner issue 8070, `ReturnValueStoredInVariantVariable`, `ReturnValueStoredInVariantVariableThroughEvent`,
    /// `VariantParameterCopiedToVariantVariable`), required by equality. `lone`: the record state and its table have no
    /// holder but the value moved.
    fn measured(&self, site: &VariantCopySite, i: usize, lone: bool, rec: &RecordState, f: &Frame) -> bool {
        let running = &self.engine.program.objects[f.object];
        let r = &running.routines[f.routine];
        // The routine: a procedure of a codeunit, no trigger (a test method and a local procedure in the repro).
        let routine = running.kind == ir::ObjKind::Codeunit && !r.is_trigger;
        // The source: a by-value Variant parameter (`CountParameterAndCopy(P: Variant)`), or a procedure of a codeunit with
        // no parameters, called on a global codeunit variable, whose returned record and table nobody else holds
        // (`Lines.BuildLines()` returns its own local temporary record).
        let source = match site.source {
            VariantSource::ByValueParam => true,
            VariantSource::Call {
                object,
                routine,
                on_variable,
            } => {
                let callee = &self.engine.program.objects[object];
                on_variable && callee.kind == ir::ObjKind::Codeunit && callee.routines[routine].params == 0 && lone
            }
            _ => false,
        };
        // The target: a declared Variant local (no parameter, no named return value) that no other statement of the
        // routine writes and that holds nothing yet.
        let target =
            site.target_written_once && i >= r.params && r.ret_local != Some(i) && matches!(f.locals[i], Value::Null);
        routine && source && target && self.record_measured(rec)
    }

    /// The record: of a normal table declared temporary, in no other company, with the view, read isolation, load fields,
    /// auto-calculated fields and SecurityFiltering a new variable has (the repro's records are new variables that only
    /// assigned fields and inserted).
    fn record_measured(&self, rec: &RecordState) -> bool {
        rec.filters.is_empty()
            && rec.filter_group == 0
            && rec.marks.is_empty()
            && !rec.marked_only
            && rec.key == 0
            && rec.adhoc.is_none()
            && rec.ascending
            && rec.field_descending == 0
            && rec.company.is_none()
            && rec.unpinned_buffer.is_none()
            && rec.run_page_link.is_none()
            && rec.read_isolation == 0
            && rec.load.request.is_none()
            && !rec.load.partial
            && rec.auto_calc.is_empty()
            && rec.security_filtering == 0
            && rec.declared_security_filtering == 0
            && !rec.security_filtering_unpinned
            && !rec.declared_security_filtering_unpinned
            && !self.engine.object_facts[rec.table as usize].temporary()
    }
}

/// A write (`RecordOp::writes`) to a temporary table this module marked refuses, before any trigger runs (the dispatch of
/// every record method, `Session::record_op_inner`).
pub(crate) fn refuse_marked_write(
    method: al_builtins::record_op::RecordOp,
    rec: &crate::records::Rec,
) -> Result<(), Signal> {
    if !method.writes() {
        return Ok(());
    }
    let marked = rec.borrow().temp_id.as_ref().map(|t| t.unmeasured());
    match marked {
        Some(TableUnmeasured::Writes) => Err(writes_refusal(method.name())),
        Some(TableUnmeasured::Rows) => Err(rows_refusal()),
        _ => Ok(()),
    }
}

/// A by-value copy of `from` into a Variant (`Interp::write_as`: an assignment, the write-back of a `var` Variant parameter):
/// when the rows of `from`'s table are marked not measured, so are the copy's.
pub(crate) fn keep_rows_mark(from: &RecordState, copy: &Value) {
    if from
        .temp_id
        .as_ref()
        .is_some_and(|t| t.unmeasured() == TableUnmeasured::Rows)
        && let Value::Record(c) = copy
        && let Some(t) = &c.borrow().temp_id
    {
        t.mark_unmeasured(TableUnmeasured::Rows);
    }
}

/// A read of the rows of, or a write to, a temporary table marked `TableUnmeasured::Rows` (`Session::store_for`).
pub(crate) fn rows_refusal() -> Signal {
    Signal::unsupported(
        "a temporary record a Variant got from another Variant: whether its table holds the source's rows on real Business Central is not measured (not pinned; al-runner issue 8070)",
    )
}

/// A write to a temporary table marked `TableUnmeasured::Writes` ([`refuse_marked_write`]).
fn writes_refusal(method: &str) -> Signal {
    Signal::unsupported(format!(
        "Record.{method}() on a temporary table a Variant shares with the Variant it was assigned from: whether the write shows through the other holder is not measured (not pinned; al-runner issue 8070)"
    ))
}
