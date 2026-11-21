//! Record variables built on first use (PERF-28): the one owner of which local of a frame starts as an unbuilt record
//! (`Value::UnbuiltRecord`), of the locals a statement names (the compiler emits `Op::BuildLocal` for them at the start of the
//! statement), of building one, and of the session's counts. Nothing else makes or builds an unbuilt record.
//!
//! A local is a candidate when it is a declared local (not a parameter, not the named return value) of type
//! `Record(_, false)` ([`watched_local`]); the session defers it when, in addition, its table is a stored table
//! (`record_without_state`), no debugger or value capture watches the frame, and no app is marked not installed
//! ([`Session::defers_local`]).
//!
//! Stage A0: inert. [`Session::defers_local`] answers false, so every local is built when the frame is made, as before, and
//! `Op::BuildLocal` finds nothing to build.

use al_builtins::native::Signal;
use al_builtins::value::{RtType, Value};
use al_ir as ir;

use crate::{Frame, Session, blob_transfer, plain_default};

/// How many record locals a session left unbuilt at frame entry, and how many of those it built later.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LazyRecordStats {
    pub deferred: u64,
    pub built: u64,
}

impl Session {
    /// The session's counts of deferred and later built record locals.
    pub fn lazy_record_stats(&self) -> LazyRecordStats {
        self.lazy_records
    }

    /// Whether local `i` of routine `r` starts unbuilt. Stage A0: never.
    pub(crate) fn defers_local(&self, _r: &ir::Routine, _i: usize) -> bool {
        false
    }

    /// The locals of a frame of routine `r` of `object` when it is entered, before the `passed` arguments are bound.
    pub(crate) fn frame_locals(&mut self, object: usize, r: &ir::Routine, passed: usize, locals: &mut Vec<Value>) {
        let kind = self.engine.program.objects[object].kind;
        for (i, s) in r.locals.iter().enumerate() {
            // The default of a parameter that is passed in is never seen: the argument replaces it. One with state of
            // its own (a record takes the number of a temporary table) is still made, as the values are made in order.
            if i < passed && (plain_default(&s.ty) || matches!(&s.ty, RtType::Array(_, el) if plain_default(el))) {
                locals.push(Value::Null);
            } else if i < passed && self.record_without_state(&s.ty) {
                // A record of a stored table takes no temporary table: its fresh buffer would only be dropped (issue 6133).
                locals.push(Value::Null);
            } else if self.defers_local(r, i) {
                // A record the Blob rules measure as a source is marked as one from its declaration on
                // (`blob_transfer::Sources`), and keeps the commit count of its declaration.
                if blob_transfer::measured_local(kind, r, i) {
                    self.blob_sources.records_marked();
                }
                self.lazy_records.deferred += 1;
                locals.push(Value::UnbuiltRecord(self.commits));
            } else {
                let v = self.slot_default(s);
                if blob_transfer::measured_local(kind, r, i) {
                    blob_transfer::new_variable(&v, self.commits, &mut self.blob_sources);
                }
                locals.push(v);
            }
        }
    }

    /// `Op::BuildLocal`: local `i` is about to be named by the statement that starts. Stage A0: no local is ever unbuilt.
    #[inline]
    pub(crate) fn build_local(&mut self, _i: u16, _f: &mut Frame) -> Result<(), Signal> {
        Ok(())
    }
}

/// Whether the compiler emits `Op::BuildLocal` for local `i` of `r`: a declared local of a record type that is not
/// temporary by declaration. Whether its table is a stored table is the session's question ([`Session::defers_local`]).
pub(crate) fn watched_local(r: &ir::Routine, i: usize) -> bool {
    i >= r.params && Some(i) != r.ret_local && matches!(r.locals.get(i).map(|s| &s.ty), Some(RtType::Record(_, false)))
}

/// The locals among `watched` that statement `s` names when it starts, in slot order: the whole of a simple statement, the
/// header of a compound one (its bodies are statements of their own).
pub(crate) fn statement_locals(s: &ir::Stmt, watched: &[bool]) -> Vec<u16> {
    let Some(mut head) = header(s) else { return Vec::new() };
    let mut names = Names(Vec::new());
    ir::walk::walk_body(std::slice::from_mut(&mut head), &mut names);
    let mut out: Vec<u16> = names
        .0
        .into_iter()
        .filter(|&i| watched.get(i).copied().unwrap_or(false))
        .filter_map(|i| u16::try_from(i).ok())
        .collect();
    out.sort_unstable();
    out.dedup();
    out
}

/// The part of `s` that runs when it starts. The condition of a `repeat` runs after the body, but naming it at the start
/// only builds the record earlier.
fn header(s: &ir::Stmt) -> Option<ir::Stmt> {
    use ir::Stmt as S;
    Some(match s {
        S::Assign(..) | S::Expr(_) | S::Exit(_) => s.clone(),
        S::If(c, _, _) => S::If(c.clone(), Vec::new(), Vec::new()),
        S::While(c, _) => S::While(c.clone(), Vec::new()),
        S::Repeat(_, c) => S::Repeat(Vec::new(), c.clone()),
        S::For {
            var, from, to, down, ..
        } => S::For {
            var: var.clone(),
            from: from.clone(),
            to: to.clone(),
            down: *down,
            body: Vec::new(),
        },
        S::Foreach { var, collection, .. } => S::Foreach {
            var: var.clone(),
            collection: collection.clone(),
            body: Vec::new(),
        },
        S::Case {
            scrutinee, branches, ..
        } => S::Case {
            scrutinee: scrutinee.clone(),
            branches: branches.iter().map(|(e, _)| (e.clone(), Vec::new())).collect(),
            otherwise: Vec::new(),
        },
        S::AssertError(_) | S::Break | S::Continue | S::Pos(_) | S::Assume(_) | S::TddMark(_) => return None,
    })
}

/// Collects the locals a walk meets; every other reference is left as it is.
struct Names(Vec<usize>);

impl ir::walk::RefVisitor for Names {
    fn object(&mut self, _r: &mut usize) {}
    fn routine(&mut self, _r: &mut usize, _object: usize) {}
    fn field(&mut self, _r: &mut usize, _table: ir::walk::TableOf) {}
    fn expansion(&mut self, _index: usize, _site: ir::walk::ExpansionSite<'_>) {}
    fn control(&mut self, _r: &mut usize) {}
    fn local(&mut self, slot: usize) {
        self.0.push(slot);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ir::{Arg, Expr, Place, Stmt};

    #[test]
    fn a_statement_names_the_locals_of_its_header_only() {
        let watched = [false, true, true, true];
        let get = Expr::Builtin {
            id: al_builtins::builtin_id::ids::RECORD_GET_1,
            receiver: Some(Box::new(Arg::Ref(Place::Local(1)))),
            args: vec![Arg::Value(Expr::Load(Place::Local(0)))],
            ret_used: true,
        };
        let body = vec![Stmt::Assign(
            Place::Field(Box::new(Place::Local(2)), 0),
            Expr::Load(Place::Field(Box::new(Place::Local(3)), 0)),
        )];
        // `if L1.Get(P0) then L2.F := L3.F`: the header names L1 (P0 is a parameter).
        assert_eq!(
            statement_locals(&Stmt::If(get, body.clone(), Vec::new()), &watched),
            vec![1]
        );
        // The body statement names L2 and L3.
        assert_eq!(statement_locals(&body[0], &watched), vec![2, 3]);
        assert!(statement_locals(&Stmt::Pos(0), &watched).is_empty());
    }
}
