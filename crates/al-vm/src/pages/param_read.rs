//! Whether a trigger reads its first parameter (a refusal of a value that is not pinned only matters when the trigger can see it).

use al_ir as ir;
use ir::{Arg, Expr, Place};

fn rooted_in_first(p: &Place) -> bool {
    match p {
        Place::Local(0) => true,
        Place::Index(b, _) | Place::Field(b, _) => rooted_in_first(b),
        _ => false,
    }
}

fn arg_uses_first(a: &Arg) -> bool {
    match a {
        Arg::Ref(p) => rooted_in_first(p),
        Arg::Value(_) => false,
    }
}

/// Some statement of `body` reads the first parameter, or hands it on by reference.
pub(super) fn reads_first_param(body: &[ir::Stmt]) -> bool {
    let mut read = false;
    ir::types::for_each_expr_post(body, &mut |e| {
        read |= match e {
            Expr::Load(p) => rooted_in_first(p),
            Expr::Call { args, .. }
            | Expr::InterfaceCall { args, .. }
            | Expr::DotNet { args, .. }
            | Expr::Builtin { args, .. } => args.iter().any(arg_uses_first),
            _ => false,
        };
        if let Expr::Builtin { receiver: Some(r), .. } = e {
            read |= arg_uses_first(r);
        }
    });
    read
}
