//! The static type of each expression of a routine (ADR-036, step 0).
//!
//! Lowering records a type for the expressions it has converted and leaves the rest `Unknown`. A reader that gets
//! `Unknown` behaves exactly as it would without the table. The table is a `Vec<ExprTy>` on [`Routine`]: one
//! entry per [`Expr`] node of the body in post-order (children in field order, then the node; statements in order;
//! the expressions inside a [`Place`] and an [`Arg`] included). An empty table means every node is `Unknown`.
//! [`ExprTypes`] pairs the walk with the table and answers by the address of a node.

use crate::*;
use serde::{Deserialize, Serialize};

/// The static type of an expression. Small and `Copy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ExprTy {
    /// Not recorded.
    #[default]
    Unknown,
    Integer,
    BigInteger,
    Decimal,
    Boolean,
    Char,
    Byte,
    Text,
    Code,
    Date,
    Time,
    DateTime,
    Duration,
    Guid,
    Option,
    Enum,
    Record,
    /// Known, and none of the above.
    Other,
}

impl ExprTy {
    /// The type of a slot of this runtime type.
    pub fn of_rt(t: &al_builtins::value::RtType) -> ExprTy {
        use al_builtins::value::RtType as R;
        match t {
            R::Boolean => ExprTy::Boolean,
            R::Integer => ExprTy::Integer,
            R::BigInteger => ExprTy::BigInteger,
            R::Decimal => ExprTy::Decimal,
            R::Byte => ExprTy::Byte,
            R::Char => ExprTy::Char,
            R::Text(_) => ExprTy::Text,
            R::Code(_) => ExprTy::Code,
            R::Date => ExprTy::Date,
            R::Time => ExprTy::Time,
            R::DateTime => ExprTy::DateTime,
            R::Duration => ExprTy::Duration,
            R::Guid => ExprTy::Guid,
            R::Option(_) => ExprTy::Option,
            R::Enum(_) => ExprTy::Enum,
            R::Record(..) => ExprTy::Record,
            R::Variant => ExprTy::Unknown,
            _ => ExprTy::Other,
        }
    }
}

/// Calls `f` on every expression of `body` in post-order (the order of the table).
pub fn for_each_expr_post(body: &[Stmt], f: &mut dyn FnMut(&Expr)) {
    for s in body {
        stmt_roots(s, &mut |e| post(e, f));
    }
}

fn post(e: &Expr, f: &mut dyn FnMut(&Expr)) {
    children(e, &mut |c| post(c, f));
    f(e);
}

/// The number of expression nodes in `body`.
pub fn count_exprs(body: &[Stmt]) -> usize {
    let mut n = 0;
    for_each_expr_post(body, &mut |_| n += 1);
    n
}

/// The number of expression nodes in `e`, itself included.
pub fn count_expr(e: &Expr) -> usize {
    let mut n = 0;
    post(e, &mut |_| n += 1);
    n
}

/// Calls `f` on every expression node of `e`, itself included, in post-order, with the number of nodes of its subtree.
pub fn for_each_sized(e: &Expr, f: &mut dyn FnMut(&Expr, usize)) -> usize {
    let mut size = 1;
    children(e, &mut |c| size += for_each_sized(c, f));
    f(e, size);
    size
}

/// A number for the kind of an expression node (its variant), to compare a node with a record of it.
pub fn kind_of(e: &Expr) -> u8 {
    match e {
        Expr::Const(_) => 0,
        Expr::Load(_) => 1,
        Expr::Unary(..) => 2,
        Expr::Binary(..) => 3,
        Expr::In(..) => 4,
        Expr::Ternary(..) => 5,
        Expr::Convert(..) => 6,
        Expr::Call { .. } => 7,
        Expr::InterfaceCall { .. } => 8,
        Expr::BaseInstance => 9,
        Expr::ThisInstance => 10,
        Expr::Refused(_) => 11,
        Expr::Fail { .. } => 12,
        Expr::InterfaceTest { .. } => 13,
        Expr::EnumImpl { .. } => 14,
        Expr::Control { .. } => 15,
        Expr::FieldOf(..) => 16,
        Expr::QueryColumn { .. } => 17,
        Expr::DotNet { .. } => 18,
        Expr::CharAt(..) => 19,
        Expr::DotNetCoerce(..) => 20,
        Expr::DotNetOperand(..) => 21,
        Expr::Builtin { .. } => 22,
        Expr::TddStub { .. } => 23,
        Expr::TddFit { .. } => 24,
        Expr::FitOrRefuse { .. } => 25,
    }
}

/// Calls `f` on each expression a statement holds directly, in the order of the table, and on those of the
/// statements nested in it (their statements follow the expressions that come before them in the walk).
pub fn stmt_roots(s: &Stmt, f: &mut dyn FnMut(&Expr)) {
    stmt_roots_nested(s, &mut |e, _| f(e));
}

/// Like [`stmt_roots`]; the second argument of `f` is true for an expression of a nested statement.
pub fn stmt_roots_nested(s: &Stmt, f: &mut dyn FnMut(&Expr, bool)) {
    fn list(b: &[Stmt], f: &mut dyn FnMut(&Expr, bool)) {
        for s in b {
            stmt_roots_nested(s, &mut |e, _| f(e, true));
        }
    }
    fn own(f: &mut dyn FnMut(&Expr, bool)) -> impl FnMut(&Expr) + '_ {
        move |e| f(e, false)
    }
    match s {
        Stmt::Assign(p, e) => {
            place_children(p, &mut own(f));
            f(e, false);
        }
        Stmt::Expr(e) => f(e, false),
        Stmt::If(c, a, b) => {
            f(c, false);
            list(a, f);
            list(b, f);
        }
        Stmt::While(c, b) => {
            f(c, false);
            list(b, f);
        }
        Stmt::Repeat(b, c) => {
            list(b, f);
            f(c, false);
        }
        Stmt::For {
            var,
            from,
            to,
            down: _,
            body,
        } => {
            place_children(var, &mut own(f));
            f(from, false);
            f(to, false);
            list(body, f);
        }
        Stmt::Foreach { var, collection, body } => {
            place_children(var, &mut own(f));
            f(collection, false);
            list(body, f);
        }
        Stmt::Case {
            scrutinee,
            branches,
            otherwise,
        } => {
            f(scrutinee, false);
            for (elems, body) in branches {
                for e in elems {
                    set_elem_children(e, &mut own(f));
                }
                list(body, f);
            }
            list(otherwise, f);
        }
        Stmt::Exit(e) => {
            if let Some(e) = e {
                f(e, false);
            }
        }
        Stmt::AssertError(b) => list(b, f),
        Stmt::Break | Stmt::Pos(_) | Stmt::Assume(_) | Stmt::TddMark(_) | Stmt::Continue => {}
    }
}

fn set_elem_children(e: &SetElem, f: &mut dyn FnMut(&Expr)) {
    match e {
        SetElem::One(x) => f(x),
        SetElem::Range(a, b) => {
            if let Some(a) = a {
                f(a);
            }
            if let Some(b) = b {
                f(b);
            }
        }
    }
}

fn place_children(p: &Place, f: &mut dyn FnMut(&Expr)) {
    match p {
        Place::Local(_) | Place::Global(_) | Place::BaseGlobal(_) => {}
        Place::Index(base, idx) => {
            place_children(base, f);
            for e in idx {
                f(e);
            }
        }
        Place::Field(base, _) => place_children(base, f),
    }
}

fn arg_children(a: &Arg, f: &mut dyn FnMut(&Expr)) {
    match a {
        Arg::Value(e) => f(e),
        Arg::Ref(p) => place_children(p, f),
    }
}

fn args_children(a: &[Arg], f: &mut dyn FnMut(&Expr)) {
    for x in a {
        arg_children(x, f);
    }
}

/// Calls `f` on the expressions directly inside `e` (through places, arguments and sets), in the order of the table.
pub fn children(e: &Expr, f: &mut dyn FnMut(&Expr)) {
    match e {
        Expr::Const(_) | Expr::BaseInstance | Expr::ThisInstance | Expr::Refused(_) | Expr::Fail { .. } => {}
        Expr::Load(p) => place_children(p, f),
        Expr::Unary(_, x) | Expr::Convert(_, x) | Expr::DotNetCoerce(_, x) | Expr::DotNetOperand(x, _) => f(x),
        Expr::Binary(_, a, b) | Expr::CharAt(a, b) => {
            f(a);
            f(b);
        }
        Expr::In(x, set) => {
            f(x);
            for s in set {
                set_elem_children(s, f);
            }
        }
        Expr::Ternary(c, a, b) => {
            f(c);
            f(a);
            f(b);
        }
        Expr::Call {
            object: _,
            routine: _,
            instance,
            args,
            try_call: _,
        } => {
            if let Some(i) = instance {
                f(i);
            }
            args_children(args, f);
        }
        Expr::InterfaceCall {
            receiver,
            targets: _,
            args,
        } => {
            f(receiver);
            args_children(args, f);
        }
        Expr::InterfaceTest {
            value,
            objects: _,
            cast: _,
        } => f(value),
        Expr::EnumImpl {
            value,
            by_ordinal: _,
            unknown: _,
        } => f(value),
        Expr::Control {
            page,
            control: _,
            runtime: _,
        } => f(page),
        Expr::FieldOf(x, _) => f(x),
        Expr::QueryColumn { query, column: _ } => f(query),
        Expr::DotNet {
            ty: _,
            receiver,
            op: _,
            args,
        } => {
            if let Some(r) = receiver {
                f(r);
            }
            args_children(args, f);
        }
        Expr::Builtin {
            id: _,
            receiver,
            args,
            ret_used: _,
        } => {
            if let Some(r) = receiver {
                arg_children(r, f);
            }
            args_children(args, f);
        }
        Expr::TddStub { args, .. } => {
            for a in args {
                f(a);
            }
        }
        Expr::TddFit { inner, .. } | Expr::FitOrRefuse { inner, .. } => f(inner),
    }
}

/// The types of the expressions of one routine, found by the address of the node.
///
/// A node that is not part of the routine's body, or any node when the routine has no table (or one of the wrong
/// length), is `Unknown`. The addresses are only good while the routine stays where it is and unchanged, so the
/// value borrows the routine for `'a` and `of` takes a node borrowed for `'a`: the compiler refuses a change or a
/// drop of the routine while the value lives.
///
/// ```compile_fail
/// use al_ir::types::ExprTypes;
/// fn f(mut r: al_ir::Routine) {
///     let t = ExprTypes::new(&r);
///     r.body.clear(); // error: `r` is borrowed by `t`
///     let _ = t;
/// }
/// ```
pub struct ExprTypes<'a> {
    by_addr: std::collections::HashMap<usize, ExprTy>,
    _routine: std::marker::PhantomData<&'a Routine>,
}

impl<'a> ExprTypes<'a> {
    /// One walk of the body.
    pub fn new(routine: &'a Routine) -> ExprTypes<'a> {
        let mut by_addr = std::collections::HashMap::new();
        let table = &routine.expr_tys;
        if !table.is_empty() {
            let mut found = Vec::new();
            for_each_expr_post(&routine.body, &mut |e| found.push(e as *const Expr as usize));
            if table.len() == found.len() {
                by_addr.reserve(found.len());
                for (addr, ty) in found.into_iter().zip(table.iter()) {
                    if *ty != ExprTy::Unknown {
                        by_addr.insert(addr, *ty);
                    }
                }
            }
        }
        ExprTypes {
            by_addr,
            _routine: std::marker::PhantomData,
        }
    }

    /// The type of `e`, a node of the routine's body.
    pub fn of(&self, e: &'a Expr) -> ExprTy {
        self.by_addr
            .get(&(e as *const Expr as usize))
            .copied()
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn int(v: i32) -> Expr {
        Expr::Const(Const::Integer(v))
    }

    fn routine(body: Vec<Stmt>, expr_tys: Vec<ExprTy>) -> Routine {
        Routine {
            name: "R".into(),
            field: None,
            container: None,
            is_trigger: false,
            attributes: vec![],
            params: 0,
            by_var: vec![],
            locals: vec![],
            ret: None,
            ret_local: None,
            pos: NO_POS,
            end_pos: NO_POS,
            begin_pos: NO_POS,
            body: body.into(),
            expr_tys,
            reads_var_max_len: false,
        }
    }

    #[test]
    fn the_walk_is_post_order_and_counts_places_and_args() {
        let e = Expr::Binary(
            BinOp::Add,
            Box::new(int(1)),
            Box::new(Expr::Load(Place::Index(Box::new(Place::Local(0)), vec![int(2)]))),
        );
        let body = vec![Stmt::Assign(Place::Local(0), e)];
        let mut seen = Vec::new();
        for_each_expr_post(&body, &mut |e| {
            seen.push(match e {
                Expr::Const(_) => "const",
                Expr::Load(_) => "load",
                Expr::Binary(..) => "binary",
                _ => "other",
            })
        });
        assert_eq!(seen, ["const", "const", "load", "binary"]);
        assert_eq!(count_exprs(&body), 4);
    }

    #[test]
    fn a_table_is_found_by_node_and_a_table_of_the_wrong_length_is_ignored() {
        let body = vec![Stmt::Expr(Expr::Binary(BinOp::Add, Box::new(int(1)), Box::new(int(2))))];
        let r = routine(body, vec![ExprTy::Integer, ExprTy::Integer, ExprTy::Integer]);
        let t = ExprTypes::new(&r);
        let Stmt::Expr(Expr::Binary(_, a, _)) = &r.body[0] else {
            unreachable!()
        };
        assert_eq!(t.of(a), ExprTy::Integer);
        assert_eq!(t.of(&int(1)), ExprTy::Unknown, "a node of another tree");
        let short = routine(r.body.to_vec(), vec![ExprTy::Integer]);
        let Stmt::Expr(e) = &short.body[0] else { unreachable!() };
        assert_eq!(ExprTypes::new(&short).of(e), ExprTy::Unknown);
        let none = routine(r.body.to_vec(), vec![]);
        let Stmt::Expr(e) = &none.body[0] else { unreachable!() };
        assert_eq!(ExprTypes::new(&none).of(e), ExprTy::Unknown);
    }
}
