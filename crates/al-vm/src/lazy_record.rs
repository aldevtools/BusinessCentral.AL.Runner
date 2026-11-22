//! Record variables built on first use (issue 8610): the one owner of which local of a frame starts as an unbuilt
//! record (`Value::UnbuiltRecord`), of the locals a statement names (the compiler emits `Op::BuildLocal` for them at
//! the start of the statement), of building one, and of the session's counts. Nothing else makes or builds an unbuilt
//! record.
//!
//! A local is a candidate when it is a declared local (not a parameter, not the named return value) of type
//! `Record(_, false)` ([`watched_local`]); the session defers it when, in addition, its table is a stored table
//! (`record_without_state`), no debugger or value capture watches the frame, and no app is marked not installed
//! ([`Session::defers_local`]).
//!
//! The record an unbuilt local becomes is the one `slot_default` gives for its declaration, which reads nothing of the
//! session but the apps marked not installed (`records.rs` `clear_uninstalled_init_values`): a local is only deferred while
//! none is, and it is built as if none were. A local whose Blob fields the Blob rules measure (`blob_transfer`) gets the
//! commit count of its declaration, which the marker carries. So the value is the one an eager build gave, and the order in
//! which locals are built is not observable: building a stored table's record draws no temporary table number and counts
//! nothing.

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

    /// Whether local `i` of routine `r` starts unbuilt: a watched local of a stored table ([`watched_local`],
    /// `record_without_state`), while no debugger or value capture renders the frame's locals and no app is marked not
    /// installed.
    pub(crate) fn defers_local(&self, r: &ir::Routine, i: usize) -> bool {
        watched_local(r, i)
            && self.record_without_state(&r.locals[i].ty)
            && self.debug.is_none()
            && self.capture.is_none()
            && self.events.not_installed_apps.as_ref().is_none_or(|a| a.is_empty())
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

    /// `Op::BuildLocal`: local `i` is about to be named by the statement that starts; built now if it is not yet.
    #[inline(always)]
    pub(crate) fn build_local(&mut self, i: u16, f: &mut Frame) {
        if let Value::UnbuiltRecord(born) = f.locals[usize::from(i)] {
            self.build_unbuilt(usize::from(i), born, f);
        }
    }

    /// Builds unbuilt local `i` of frame `f`, declared when the session's commit count was `born`.
    #[cold]
    #[inline(never)]
    fn build_unbuilt(&mut self, i: usize, born: u64, f: &mut Frame) {
        let engine = self.engine.clone();
        let o = &engine.program.objects[f.object];
        let r = &o.routines[f.routine];
        // As at the declaration, where no app was marked not installed (`defers_local`).
        let held = self.events.not_installed_apps.take();
        let v = self.slot_default(&r.locals[i]);
        self.events.not_installed_apps = held;
        if blob_transfer::measured_local(o.kind, r, i) {
            blob_transfer::new_variable(&v, born, &mut self.blob_sources);
        }
        self.lazy_records.built += 1;
        f.locals[i] = v;
    }

    /// A read of local `i` of frame `f` that is still unbuilt: a statement named it that the compiler did not see naming
    /// it, so no `Op::BuildLocal` ran. Refused, in a release build too, rather than handing the marker on as a value.
    #[cold]
    #[inline(never)]
    pub(crate) fn unbuilt_read(&self, i: usize, f: &Frame) -> Signal {
        let r = &self.engine.program.objects[f.object].routines[f.routine];
        Signal::unsupported(format!(
            "record variable `{}` of `{}` is read by a statement that did not build it first",
            r.locals[i].name, r.name
        ))
    }
}

/// Whether the compiler emits `Op::BuildLocal` for local `i` of `r`: a declared local of a record type that is not
/// temporary by declaration. Whether its table is a stored table is the session's question ([`Session::defers_local`]).
pub(crate) fn watched_local(r: &ir::Routine, i: usize) -> bool {
    i >= r.params && Some(i) != r.ret_local && matches!(r.locals.get(i).map(|s| &s.ty), Some(RtType::Record(_, false)))
}

/// The locals among `watched` that statement `s` names when it starts, in slot order: the whole of a simple statement, the
/// header of a compound one (its bodies are statements of their own). A loop also names what the statements of its body
/// name when they start: they run in every round, so building those locals once before the loop keeps `Op::BuildLocal` out
/// of the rounds (a loop that runs no round builds them for nothing, which is not observable). The condition of a `repeat`
/// runs after the body, but naming it at the start only builds the record earlier. The walk reads the statement in place
/// (`al_ir::types::child_nodes`): it clones nothing.
pub(crate) fn statement_locals(s: &ir::Stmt, watched: &[bool]) -> Vec<u16> {
    let mut names = Vec::new();
    header_locals(s, &mut names);
    let mut out: Vec<u16> = names
        .into_iter()
        .filter(|&i| watched.get(i).copied().unwrap_or(false))
        .filter_map(|i| u16::try_from(i).ok())
        .collect();
    out.sort_unstable();
    out.dedup();
    out
}

/// Pushes the locals the header of `s` names (see [`statement_locals`]) onto `out`.
fn header_locals(s: &ir::Stmt, out: &mut Vec<usize>) {
    use ir::Stmt as S;
    let rounds = |body: &[ir::Stmt], out: &mut Vec<usize>| body.iter().for_each(|s| header_locals(s, out));
    match s {
        S::Assign(p, e) => {
            place_locals(p, out);
            expr_locals(e, out);
        }
        S::Expr(e) | S::If(e, _, _) => expr_locals(e, out),
        S::Exit(e) => e.iter().for_each(|e| expr_locals(e, out)),
        S::While(c, body) | S::Repeat(body, c) => {
            expr_locals(c, out);
            rounds(body, out);
        }
        S::For {
            var,
            from,
            to,
            down: _,
            body,
        } => {
            place_locals(var, out);
            expr_locals(from, out);
            expr_locals(to, out);
            rounds(body, out);
        }
        S::Foreach { var, collection, body } => {
            place_locals(var, out);
            expr_locals(collection, out);
            rounds(body, out);
        }
        S::Case {
            scrutinee, branches, ..
        } => {
            expr_locals(scrutinee, out);
            for (labels, _) in branches {
                for l in labels {
                    ir::types::set_elem_children(l, &mut |e| expr_locals(e, out));
                }
            }
        }
        S::AssertError(_) | S::Break | S::Continue | S::Pos(_) | S::Assume(_) | S::TddMark(_) => {}
    }
}

/// Pushes the locals expression `e` names, at any depth, onto `out`.
fn expr_locals(e: &ir::Expr, out: &mut Vec<usize>) {
    ir::types::child_nodes(e, &mut |n| node_locals(n, out));
}

/// Pushes the locals place `p` names (its base, and its index expressions) onto `out`.
fn place_locals(p: &ir::Place, out: &mut Vec<usize>) {
    ir::types::place_nodes(p, &mut |n| node_locals(n, out));
}

fn node_locals(n: ir::types::Node<'_>, out: &mut Vec<usize>) {
    match n {
        ir::types::Node::Place(ir::Place::Local(i)) => out.push(*i),
        ir::types::Node::Place(_) => {}
        ir::types::Node::Expr(e) => expr_locals(e, out),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ir::{Arg, Expr, Place, Stmt};

    /// A session and the frame of `C.Run` (one Record local `R` of a stored table with a Blob) as entered at commit count
    /// `commits`: `R` is unbuilt.
    fn entered(commits: u64) -> (Session, Frame) {
        const SRC: &str = "table 50100 Row { fields { field(1; Id; Integer) { } field(2; B; Blob) { } } }
            codeunit 50101 C { procedure Run() var R: Record Row; begin R.Id := 1; end; }";
        let (ws, l) = al_sema::lower::compile(
            vec![al_sema::workspace::AppSource {
                id: "a".into(),
                name: "a".into(),
                dependencies: vec![],
                files: vec![("f.al".into(), SRC.into())],
            }],
            &Default::default(),
        );
        assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
        let engine = crate::Engine::new(l.program, Vec::new());
        let object = engine.program.objects.iter().position(|o| o.name == "C").unwrap();
        let routine = engine.program.objects[object]
            .routines
            .iter()
            .position(|r| r.name == "Run")
            .unwrap();
        let mut s = Session::new(engine.clone());
        s.commits = commits;
        let mut locals = Vec::new();
        s.frame_locals(
            object,
            &engine.program.objects[object].routines[routine],
            0,
            &mut locals,
        );
        let f = Frame {
            object,
            routine,
            instance: s.instance(object),
            locals,
            aliases: Vec::new(),
            var_lens: Vec::new(),
            option_hops: Vec::new(),
        };
        (s, f)
    }

    /// A measured local built after a Commit carries the commit count of its declaration, as the eager build stamped it
    /// (`blob_transfer::new_variable`, `measured_variable`): the visible value of a copy does not show it.
    #[test]
    fn a_local_built_after_a_commit_keeps_the_commit_count_of_its_declaration() {
        let (mut s, mut f) = entered(3);
        assert!(matches!(f.locals[0], Value::UnbuiltRecord(3)), "{:?}", f.locals[0]);
        // A Commit between the declaration and the first statement that names it.
        s.commits = 4;
        s.build_local(0, &mut f);
        let Value::Record(rc) = &f.locals[0] else {
            panic!("{:?}", f.locals[0])
        };
        assert_eq!(rc.borrow().probed_source.1, 3, "the declaration's commit count");
        assert_eq!(s.lazy_record_stats(), LazyRecordStats { deferred: 1, built: 1 });
    }

    /// A read of a local that no `Op::BuildLocal` built (a statement the compiler failed to see naming it) is refused, in a
    /// release build too, instead of handing the marker on as a value.
    #[test]
    fn a_read_of_a_local_never_built_is_refused() {
        let (mut s, mut f) = entered(0);
        match s.read_place(&ir::Place::Local(0), true, &mut f) {
            Err(Signal::Unsupported(r)) => assert_eq!(
                r.to_string(),
                "record variable `R` of `Run` is read by a statement that did not build it first"
            ),
            other => panic!("{other:?}"),
        }
    }

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
        // A loop names what the statements of its body name when they start, not what their own bodies name:
        // `while P0 do if P0 then L2.F := L3.F` names nothing, `while P0 do L2.F := L3.F` names L2 and L3.
        let inner = Stmt::If(Expr::Load(Place::Local(0)), body.clone(), Vec::new());
        let looped = Stmt::While(Expr::Load(Place::Local(0)), vec![inner]);
        assert!(statement_locals(&looped, &watched).is_empty());
        let looped = Stmt::While(Expr::Load(Place::Local(0)), body.clone());
        assert_eq!(statement_locals(&looped, &watched), vec![2, 3]);
    }

    /// The oracle of the read-only walk: the header of `s` (see [`statement_locals`]) as a statement of its own, which the
    /// exhaustive mutable walk (`al_ir::walk`, the `local` hook) reads.
    fn header(s: &ir::Stmt) -> Option<ir::Stmt> {
        use ir::Stmt as S;
        let rounds = |body: &[ir::Stmt]| body.iter().filter_map(header).collect::<Vec<_>>();
        Some(match s {
            S::Assign(..) | S::Expr(_) | S::Exit(_) => s.clone(),
            S::If(c, _, _) => S::If(c.clone(), Vec::new(), Vec::new()),
            S::While(c, body) => S::While(c.clone(), rounds(body)),
            S::Repeat(body, c) => S::Repeat(rounds(body), c.clone()),
            S::For {
                var,
                from,
                to,
                down,
                body,
            } => S::For {
                var: var.clone(),
                from: from.clone(),
                to: to.clone(),
                down: *down,
                body: rounds(body),
            },
            S::Foreach { var, collection, body } => S::Foreach {
                var: var.clone(),
                collection: collection.clone(),
                body: rounds(body),
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

    /// The locals the oracle finds in the header of `s`, as [`statement_locals`] lists them with every slot watched.
    fn oracle_locals(s: &Stmt) -> Vec<u16> {
        let Some(mut head) = header(s) else { return Vec::new() };
        let mut names = Names(Vec::new());
        ir::walk::walk_body(std::slice::from_mut(&mut head), &mut names);
        let mut out: Vec<u16> = names.0.into_iter().filter_map(|i| u16::try_from(i).ok()).collect();
        out.sort_unstable();
        out.dedup();
        out
    }

    fn each_stmt(body: &[Stmt], f: &mut dyn FnMut(&Stmt)) {
        for s in body {
            f(s);
            match s {
                Stmt::If(_, a, b) => {
                    each_stmt(a, f);
                    each_stmt(b, f);
                }
                Stmt::While(_, b) | Stmt::Repeat(b, _) | Stmt::AssertError(b) => each_stmt(b, f),
                Stmt::For { body, .. } | Stmt::Foreach { body, .. } => each_stmt(body, f),
                Stmt::Case {
                    branches, otherwise, ..
                } => {
                    for (_, b) in branches {
                        each_stmt(b, f);
                    }
                    each_stmt(otherwise, f);
                }
                _ => {}
            }
        }
    }

    /// The read-only walk names the same locals as the exhaustive mutable walk on a header it clones, for every statement
    /// of routines that use locals in each place an expression, a place or an argument can hold one.
    #[test]
    fn the_read_only_walk_names_what_the_cloned_header_names() {
        const SRC: &str = "table 50100 Row { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } } }
            codeunit 50101 C {
                procedure Run(P: Integer; var V: Record Row): Integer
                var R: Record Row; S: Record Row; A: array[3] of Record Row; N: array[4] of Integer; L: List of [Integer];
                    I: Integer; J: Integer; T: Text; RR: RecordRef;
                begin
                    R.Id := P;
                    A[I + 1].Name := S.Name;
                    N[R.Id mod 4 + 1] := A[2].Id;
                    if R.Get(N[1]) and S.IsEmpty() then begin R.Name := 'x'; S.Name := 'y'; end else A[1].Init();
                    case S.Id of 1, 2: R.Id := 1; 3 .. 5: S.Id := 2; else A[3].Id := 3; end;
                    case true of R.Id = 1: I := 1; N[J] > 2: J := 2; end;
                    while I < R.Id do begin I += 1; if S.Id = I then A[1].Id := I; end;
                    repeat J += 1; S.Name := Format(J); until (J > 3) or R.IsEmpty();
                    for I := R.Id to S.Id + N[2] do begin A[2].Id := I; while J > A[3].Id do J -= 1; end;
                    foreach I in L do T += Format(I) + R.Name;
                    asserterror R.Get(S.Id);
                    Fill(R);
                    Fill(A[2]);
                    RR.GetTable(S);
                    T := CopyStr(R.Name + S.Name, 1, StrLen(A[1].Name) + 1);
                    if Twice(R.Id) > Twice(S.Id) then exit(V.Id);
                    exit(R.Id + S.Id + A[3].Id);
                end;
                local procedure Fill(var X: Record Row) begin X.Id := 9; end;
                local procedure Twice(K: Integer): Integer begin exit(K * 2); end;
            }";
        let (ws, l) = al_sema::lower::compile(
            vec![al_sema::workspace::AppSource {
                id: "a".into(),
                name: "a".into(),
                dependencies: vec![],
                files: vec![("f.al".into(), SRC.into())],
            }],
            &Default::default(),
        );
        assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
        let mut statements = 0;
        for o in &l.program.objects {
            for r in &o.routines {
                let watched = vec![true; r.locals.len()];
                each_stmt(&r.body, &mut |s| {
                    statements += 1;
                    assert_eq!(statement_locals(s, &watched), oracle_locals(s), "{s:?}");
                });
            }
        }
        assert!(statements > 30, "{statements}");
    }
}
