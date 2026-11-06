//! The global slots a report's procedures write, to tell the request page values a caller set before the run from the ones it left alone.

use al_ir::{Arg, Expr, Place, Stmt};
use std::collections::HashSet;

fn root(p: &Place, out: &mut HashSet<usize>) {
    match p {
        Place::Global(n) => {
            out.insert(*n);
        }
        Place::Index(base, _) | Place::Field(base, _) => root(base, out),
        Place::Local(_) | Place::BaseGlobal(_) | Place::ExtGlobal(..) | Place::RecGlobal(..) => {}
    }
}

fn stmts(body: &[Stmt], out: &mut HashSet<usize>) {
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

fn expr(e: &Expr, out: &mut HashSet<usize>) {
    al_ir::types::children(e, &mut |c| expr(c, out));
    if let Expr::Call { args, .. } = e {
        for a in args {
            if let Arg::Ref(p) = a {
                root(p, out);
            }
        }
    }
}

/// The global slots the procedures (not the triggers) of `routines` assign or pass to a `var` parameter. A call another procedure
/// makes is not followed: its own body is in the set already, as every procedure of the object is scanned.
pub(super) fn written_by_procedures(routines: &[al_ir::Routine]) -> HashSet<usize> {
    let mut out = HashSet::new();
    for r in routines.iter().filter(|r| !r.is_trigger) {
        stmts(r.body.stmts(), &mut out);
    }
    out
}
