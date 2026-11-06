//! The global slots a report's procedures write, to tell the request page values a caller set before the run from the ones it left alone.

use al_ir::{Arg, Expr, Place, Stmt};
use std::collections::HashSet;

fn root(p: &Place, out: &mut Scan) {
    match p {
        Place::Global(n) => {
            out.slots.insert(*n);
        }
        Place::Index(base, _) | Place::Field(base, _) => root(base, out),
        Place::Local(_) | Place::BaseGlobal(_) | Place::ExtGlobal(..) | Place::RecGlobal(..) => {}
    }
}

fn stmts(body: &[Stmt], out: &mut Scan) {
    for s in body {
        match s {
            Stmt::Assign(p, e) => {
                root(p, out);
                expr(e, out);
            }
            Stmt::Expr(e) => expr(e, out),
            Stmt::If(c, a, b) => {
                expr(c, out);
                stmts(a, out);
                stmts(b, out);
            }
            Stmt::While(c, b) | Stmt::Repeat(b, c) => {
                expr(c, out);
                stmts(b, out);
            }
            Stmt::For {
                var, from, to, body, ..
            } => {
                root(var, out);
                expr(from, out);
                expr(to, out);
                stmts(body, out);
            }
            Stmt::Foreach { var, collection, body } => {
                root(var, out);
                expr(collection, out);
                stmts(body, out);
            }
            Stmt::Case {
                branches, otherwise, ..
            } => {
                for (_, b) in branches {
                    stmts(b, out);
                }
                stmts(otherwise, out);
            }
            Stmt::AssertError(b) => stmts(b, out),
            _ => {}
        }
    }
}

fn expr(e: &Expr, out: &mut Scan) {
    al_ir::types::children(e, &mut |c| expr(c, out));
    if let Expr::InterfaceCall { .. } = e {
        out.unknown = true;
    }
    if let Expr::Call {
        args, object, routine, ..
    } = e
    {
        if *object == out.object {
            out.follow.push(*routine);
        }
        for a in args {
            if let Arg::Ref(p) = a {
                root(p, out);
            }
        }
    }
}

/// What a scan of routines found: the slots written, and the routines of the same object that are called.
pub(super) struct Scan {
    object: usize,
    pub(super) slots: HashSet<usize>,
    follow: Vec<usize>,
    unknown: bool,
}

/// The global slots routine `start` of `object` assigns or passes to a `var` parameter, with those of the procedures of the
/// same object it calls. `None` when a call through an interface makes the writes unattributable.
pub(crate) fn written_by_routine(routines: &[al_ir::Routine], object: usize, start: usize) -> Option<HashSet<usize>> {
    let mut scan = Scan {
        object,
        slots: HashSet::new(),
        follow: vec![start],
        unknown: false,
    };
    let mut seen = HashSet::new();
    while let Some(r) = scan.follow.pop() {
        if seen.insert(r) {
            if let Some(r) = routines.get(r) {
                stmts(r.body.stmts(), &mut scan);
            }
        }
    }
    (!scan.unknown).then_some(scan.slots)
}
