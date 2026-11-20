//! A temporary record going from a Variant into a Variant (`V := W` where `W` is a Variant variable, parameter, global,
//! array element, procedure result or a ternary of them; al-runner issue 8070). The owner of that one move:
//! `exec/compile.rs` registers a [`VariantCopySite`] for an assignment whose right-hand side is one of the forms in
//! [`VariantSource`] (`Op::StoreLocal` names a local's site by its place, `Op::StoreVariantCopy` every other), and
//! [`Session::store_variant_copy`] decides for a record. A `try` call yields a Boolean, never a Variant.
//!
//! The pinned neighbors stay where they are: a record variable put into a Variant is a by-value copy over an empty table
//! of its own (recordref-fieldref/recordref/PWRecordMisc.al:234-250, `Interp::write_as`), a temporary record handed to a
//! Variant parameter keeps its table (PWRecordMisc.al:221-231, `call_inner`).
//!
//! Pinned by tests/al-language/types/variant/VariantFromVariantTempRecord.al (codeunit 96471 "LT VFV Tests"): in its cells
//! C1-C7 and C9 (C1 :95 to C7 :202, C9 :228; [`Session::measured`]) the new Variant holds a record over the same temporary
//! table, one table for both holders. Every other temporary record moved that way gets main's store, a by-value copy over an
//! empty table of its own, and that table is marked (`TempId::mark_rows_unmeasured`): its field values read as on main, every read of its rows and every write
//! refuses (`Session::store_for`, which every read of rows and every write passes). Anything that is not a temporary
//! record keeps main's store (C8 is a stored record).

use crate::{Frame, Session};
use al_builtins::filter::FilterExpr;
use al_builtins::native::Signal;
use al_builtins::value::{RecordState, RtType, Value};
use al_ir as ir;
use std::cell::RefCell;
use std::rc::Rc;

/// The right-hand side of an assignment, as far as it may be a Variant (`Convert(Variant, x)` is lowered for a Variant
/// source too, so the form of `x` is what tells).
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum VariantSource {
    /// A by-value Variant parameter of the running routine.
    ByValueParam,
    /// A `var` Variant parameter of the running routine.
    VarParam,
    /// A Variant local that is no parameter (a declared local or the named return value): its index.
    Local(usize),
    /// A procedure call: a Variant source when the callee returns a Variant. `on_variable`: called on a global
    /// variable (`L.Proc()` with `L` a global codeunit variable).
    Call {
        object: usize,
        routine: usize,
        on_variable: bool,
    },
    /// A global or an array element: a Variant source when its declared type is Variant.
    Place(ir::Place),
    /// `c ? a : b` with both arms of these forms: a Variant source when both are.
    Ternary(Box<[VariantSource; 2]>),
    /// A call through an interface (its targets): a Variant source when the method returns a Variant.
    Interface(Vec<(usize, usize)>),
}

/// One assignment of a possible Variant: the place assigned and the form of the right-hand side.
#[derive(Debug, Clone)]
pub(crate) struct VariantCopySite {
    pub place: ir::Place,
    pub source: VariantSource,
    /// A local target: the index of its place in `Chunk::places`, which its `Op::StoreLocal` names. `u32::MAX` for a target
    /// that is no local (`Op::StoreVariantCopy` names the site).
    pub at: u32,
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
                VariantSource::Local(*j)
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
        ir::Expr::Ternary(_, a, b) => Some(VariantSource::Ternary(Box::new([
            source_of(a, locals, params, by_var)?,
            source_of(b, locals, params, by_var)?,
        ]))),
        ir::Expr::InterfaceCall { targets, .. } => Some(VariantSource::Interface(targets.clone())),
        _ => None,
    }
}

/// The source of a measured cell.
#[derive(Clone, Copy, PartialEq)]
enum Source {
    /// A procedure of a codeunit that declares no Subtype, with no parameters and no attributes, called on a global codeunit
    /// variable of the running codeunit, whose returned record and table nobody else holds (`Source.BuildTemp()` returns its
    /// own local temporary record).
    Call,
    ByValueParam,
    VarParam,
    /// A declared local Variant (no parameter, no named return value).
    Local,
}

/// The target of a measured cell: a declared local Variant (no parameter, no named return value), or a global Variant of
/// the running codeunit.
#[derive(Clone, Copy, PartialEq)]
enum Target {
    Local,
    Global,
}

/// What the target held before the move.
#[derive(Clone, Copy, PartialEq)]
enum Held {
    Nothing,
    Integer,
}

/// The record's filters: none in any filter group, or one `SetRange(F, A)` of an Integer in group 0 (`Tmp.SetRange(Id, 2)`).
#[derive(Clone, Copy, PartialEq)]
enum Filters {
    None,
    OneValueRange,
}

/// Whether a move of these facts is a cell of tests/al-language/types/variant/VariantFromVariantTempRecord.al (codeunit 96471):
/// C1 :95, C2 :110, C3 :131, C4 :152, C5 :169, C6 :181, C7 :202, C9 :228. Every cell keeps the rows and the temporary state, and the copy holds the same
/// table as its source.
fn is_cell(cell: (Source, Target, Held, Filters)) -> bool {
    use self::{Filters as F, Held as H, Source as S, Target as T};
    matches!(
        cell,
        // C1_ResultIntoLocalVariant (and the reporter's ReturnValueStoredInVariantVariable and ...ThroughEvent).
        (S::Call, T::Local, H::Nothing, F::None)
            // C2_ByValueParameterIntoLocalVariant (and the reporter's VariantParameterCopiedToVariantVariable).
            | (S::ByValueParam, T::Local, H::Nothing, F::None)
            // C3_VarParameterIntoLocalVariant.
            | (S::VarParam, T::Local, H::Nothing, F::None)
            // C4_LocalVariantIntoLocalVariant; C6_WriteThroughEitherSide: a row inserted through either holder shows
            // through the other.
            | (S::Local, T::Local, H::Nothing, F::None)
            // C5_ResultIntoGlobalVariant.
            | (S::Call, T::Global, H::Nothing, F::None)
            // C7_FilteredTemporaryRecord: the filter travels.
            | (S::Call, T::Local, H::Nothing, F::OneValueRange)
            // C9_TargetAlreadyHoldsAValue: the Integer is replaced.
            | (S::Call, T::Local, H::Integer, F::None)
    )
}

impl Session {
    /// The record of a site's assignment: `site.place := v`.
    #[cold]
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
        if self.measured(site, lone, &rc.borrow(), f) {
            // The probe's cells: the new Variant holds a record over the source's temporary table, a copy as a Variant
            // parameter's is (`call_inner`); a write through either holder shows through the other (C6).
            let shared = Value::Record(Rc::new(RefCell::new(rc.borrow().copy_as_unmeasured_variable())));
            return match site.place {
                ir::Place::Local(i) => {
                    f.locals[i] = shared;
                    Ok(())
                }
                ir::Place::Global(g) => self.put_global(g, shared, f, true),
                _ => unreachable!("a measured target is a local or a global"),
            };
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
            t.mark_rows_unmeasured();
        }
        Ok(())
    }

    /// Whether both sides are Variants: the target's declared type, and the source's.
    fn variant_into_variant(&self, site: &VariantCopySite, f: &Frame) -> bool {
        matches!(self.place_type(&site.place, f), RtType::Variant) && self.is_variant_source(&site.source, f)
    }

    /// Whether `source` is a Variant: always for a Variant local or parameter; the callee's return type, the global's or
    /// element's declared type, both arms of a ternary.
    fn is_variant_source(&self, source: &VariantSource, f: &Frame) -> bool {
        let returns_variant =
            |(o, r): (usize, usize)| matches!(self.engine.program.objects[o].routines[r].ret, Some(RtType::Variant));
        match source {
            VariantSource::ByValueParam | VariantSource::VarParam | VariantSource::Local(_) => true,
            VariantSource::Call { object, routine, .. } => returns_variant((*object, *routine)),
            VariantSource::Place(p) => matches!(self.place_type(p, f), RtType::Variant),
            VariantSource::Ternary(arms) => arms.iter().all(|a| self.is_variant_source(a, f)),
            // Every target has the interface's signature; a default-body entry names no routine of its codeunit.
            VariantSource::Interface(targets) => targets
                .iter()
                .find(|(o, r)| *r < self.engine.program.objects[*o].routines.len())
                .is_some_and(|t| returns_variant(*t)),
        }
    }

    /// Whether the move is a cell of the probe ([`is_cell`]), each fact required by equality. The move runs in a test
    /// codeunit, in a `[Test]` method or a procedure with no attributes (C1, C4-C7, C9 in `[Test]` methods; C2 and C3 in
    /// local procedures), no trigger. `lone`: the record state and its table have no holder but the value moved.
    fn measured(&self, site: &VariantCopySite, lone: bool, rec: &RecordState, f: &Frame) -> bool {
        let running = &self.engine.program.objects[f.object];
        let r = &running.routines[f.routine];
        let attributes = match r.attributes.as_slice() {
            [] => true,
            [a] => a.name.eq_ignore_ascii_case("Test") && a.args.is_empty(),
            _ => false,
        };
        let routine = running.kind == ir::ObjKind::Codeunit
            && self
                .declared_subtype(f.object)
                .is_some_and(|s| s.eq_ignore_ascii_case("Test"))
            && !r.is_trigger
            && attributes;
        let declared_local = |i: usize| i >= r.params && r.ret_local != Some(i);
        let source = match site.source {
            VariantSource::ByValueParam => Some(Source::ByValueParam),
            VariantSource::VarParam => Some(Source::VarParam),
            VariantSource::Local(j) if declared_local(j) => Some(Source::Local),
            VariantSource::Call {
                object,
                routine,
                on_variable: true,
            } => {
                let callee = &self.engine.program.objects[object];
                let c = &callee.routines[routine];
                (lone
                    && callee.kind == ir::ObjKind::Codeunit
                    && self.declared_subtype(object).is_none()
                    && c.params == 0
                    && c.attributes.is_empty()
                    && !c.is_trigger)
                    .then_some(Source::Call)
            }
            _ => None,
        };
        let (target, held) = match site.place {
            ir::Place::Local(i) if declared_local(i) => (Target::Local, held(&f.locals[i])),
            ir::Place::Global(g) => (Target::Global, held(&f.instance.borrow().globals[g])),
            _ => return false,
        };
        let (Some(source), Some(held), Some(filters)) = (source, held, filters(rec)) else {
            return false;
        };
        routine && is_cell((source, target, held, filters)) && self.record_measured(rec)
    }

    /// The record: of a normal table declared temporary, in no other company, with the marks, key, read isolation, load
    /// fields, auto-calculated fields and SecurityFiltering a new variable has (the probe's records are new local variables
    /// that only assigned fields, inserted and, in C7, set one range).
    fn record_measured(&self, rec: &RecordState) -> bool {
        rec.filter_group == 0
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

/// What a target Variant holds, among the values a cell measured.
fn held(v: &Value) -> Option<Held> {
    match v {
        Value::Null => Some(Held::Nothing),
        Value::Integer(_) => Some(Held::Integer),
        _ => None,
    }
}

/// The record's filters, among the shapes a cell measured.
fn filters(rec: &RecordState) -> Option<Filters> {
    match rec.filters.as_slice() {
        [] => Some(Filters::None),
        [one]
            if one.group == 0
                && matches!(&one.expr, FilterExpr::Range(Some(Value::Integer(a)), Some(Value::Integer(b))) if a == b) =>
        {
            Some(Filters::OneValueRange)
        }
        _ => None,
    }
}

/// A by-value copy of `from` into a Variant (`Interp::write_as`: an assignment, the write-back of a `var` Variant parameter):
/// when the rows of `from`'s table are marked not measured, so are the copy's.
pub(crate) fn keep_rows_mark(from: &RecordState, copy: &Value) {
    if from.temp_id.as_ref().is_some_and(|t| t.rows_unmeasured())
        && let Value::Record(c) = copy
        && let Some(t) = &c.borrow().temp_id
    {
        t.mark_rows_unmeasured();
    }
}

/// A read of the rows of, or a write to, a temporary table whose rows are marked not measured (`Session::store_for`). It
/// carries nothing, so the store's result stays one register on the common path; `?` makes it the refusal.
#[derive(Debug)]
pub(crate) struct RowsUnmeasured;

impl From<RowsUnmeasured> for Signal {
    #[cold]
    fn from(_: RowsUnmeasured) -> Signal {
        Signal::unsupported(
            "a temporary record a Variant got from another Variant: whether its table holds the source's rows on real Business Central is not measured (not pinned; al-runner issue 8070)",
        )
    }
}
