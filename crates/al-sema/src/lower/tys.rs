//! The static types of the expressions of a routine, built with the expressions (ADR-036, step 0).
//!
//! `Lowerer::tys` is a stack of one record per expression node in post-order. Every function that returns an
//! expression leaves the records of exactly that expression's tree on the top (`expr` seals its node; an
//! expression built by anything else is checked and, when the records do not fit, replaced by `Unknown`).
//! A statement does the same for the expressions it holds. So a mistake in lowering code can make types
//! `Unknown`; it cannot move a type to another node: a record is kept only when the kinds and sizes of the
//! records match the nodes below it, and the table of a routine is kept only when it tiles the whole body.
use super::*;
use al_ir::types::{self, ExprTy};

/// The record of one expression node.
#[derive(Clone, Copy)]
pub(crate) struct TyRec {
    pub ty: ExprTy,
    pub kind: u8,
    /// The number of expressions directly inside (saturating): with the kind, it makes a record fit one node only.
    pub arity: u8,
    /// The number of nodes of the subtree, itself included.
    pub size: u32,
}

/// The type of a variable of this checker type, for the kinds slice 2 fills.
fn of_ty(t: &Ty) -> ExprTy {
    match t {
        Ty::Integer => ExprTy::Integer,
        Ty::BigInteger => ExprTy::BigInteger,
        Ty::Decimal => ExprTy::Decimal,
        Ty::Boolean => ExprTy::Boolean,
        Ty::Char => ExprTy::Char,
        Ty::Byte => ExprTy::Byte,
        _ => ExprTy::Unknown,
    }
}

/// The number of expressions directly inside `e`, at most 255.
fn arity_of(e: &ir::Expr) -> usize {
    match e {
        ir::Expr::Const(_)
        | ir::Expr::Load(
            ir::Place::Local(_) | ir::Place::Global(_) | ir::Place::BaseGlobal(_) | ir::Place::ExtGlobal(..),
        )
        | ir::Expr::BaseInstance
        | ir::Expr::ThisInstance => 0,
        ir::Expr::Binary(..) => 2,
        ir::Expr::Unary(..) => 1,
        _ => {
            let mut n = 0usize;
            types::children(e, &mut |_| n += 1);
            n.min(255)
        }
    }
}

/// Kind, number of expressions directly inside, and number of nodes of a tree.
pub(crate) type Shape = (u8, usize, usize);

fn shape_of(e: &ir::Expr) -> Shape {
    let arity = arity_of(e);
    (
        types::kind_of(e),
        arity,
        if arity == 0 { 1 } else { types::count_expr(e) },
    )
}

/// Kind and number of expressions directly inside, size not checked (0).
fn shape_cheap(e: &ir::Expr) -> Shape {
    (types::kind_of(e), arity_of(e), 0)
}

fn of_const(c: &ir::Const) -> ExprTy {
    match c {
        ir::Const::Boolean(_) => ExprTy::Boolean,
        ir::Const::Integer(_) => ExprTy::Integer,
        ir::Const::BigInteger(_) => ExprTy::BigInteger,
        ir::Const::Decimal(_) => ExprTy::Decimal,
        ir::Const::Text(_) => ExprTy::Text,
        _ => ExprTy::Unknown,
    }
}

/// The type of a node whose children have the types `c` (what `binary_op` and `unary_op` of the interpreter answer).
fn of_node(e: &ir::Expr, c: &[ExprTy], load: impl FnOnce() -> ExprTy) -> ExprTy {
    use ir::BinOp as B;
    match e {
        ir::Expr::Const(k) => of_const(k),
        ir::Expr::Load(
            ir::Place::Local(_) | ir::Place::Global(_) | ir::Place::BaseGlobal(_) | ir::Place::ExtGlobal(..),
        ) => load(),
        ir::Expr::Unary(ir::UnOp::Not, _) => ExprTy::Boolean,
        ir::Expr::Unary(ir::UnOp::Neg, _) if c[0] == ExprTy::Integer => ExprTy::Integer,
        ir::Expr::Binary(op, ..) => match op {
            B::And
            | B::Or
            | B::Xor
            | B::Eq
            | B::Ne
            | B::Lt
            | B::Le
            | B::Gt
            | B::Ge
            | B::DotNetEq
            | B::DotNetNe
            | B::DotNetLt
            | B::DotNetLe
            | B::DotNetGt
            | B::DotNetGe => ExprTy::Boolean,
            B::Add | B::Sub | B::Mul | B::IntDiv | B::Mod if c[0] == ExprTy::Integer && c[1] == ExprTy::Integer => {
                ExprTy::Integer
            }
            _ => ExprTy::Unknown,
        },
        _ => ExprTy::Unknown,
    }
}

impl Lowerer<'_> {
    /// The type a `Convert` to this slot type answers: the kinds whose value is always of the slot's type.
    pub(crate) fn exact_ty(&self, t: &RtType) -> ExprTy {
        match t {
            RtType::Integer | RtType::BigInteger | RtType::Decimal | RtType::Boolean | RtType::Char | RtType::Byte => {
                ExprTy::of_rt(t)
            }
            _ => ExprTy::Unknown,
        }
    }

    /// Checks that the records above `mark` are the records of `roots` (the kinds and sizes of the trees, last
    /// one on top) and fills `types` with the type of each root. False: they do not fit.
    fn tiles(&self, mark: usize, roots: &[Shape], types: &mut [ExprTy]) -> bool {
        let mut pos = self.tys.len();
        for (i, (kind, arity, size)) in roots.iter().enumerate().rev() {
            if pos <= mark {
                return false;
            }
            let r = self.tys[pos - 1];
            if r.kind != *kind
                || r.arity as usize != *arity
                || (*size != 0 && r.size as usize != *size)
                || r.size as usize > pos - mark
            {
                return false;
            }
            if let Some(t) = types.get_mut(i) {
                *t = r.ty;
            }
            pos -= r.size as usize;
        }
        pos == mark
    }

    /// `Unknown` records for every node of `e`.
    fn push_unknown(&mut self, e: &ir::Expr) {
        let tys = &mut self.tys;
        types::for_each_sized(e, &mut |x, size| {
            tys.push(TyRec {
                ty: ExprTy::Unknown,
                kind: types::kind_of(x),
                arity: arity_of(x) as u8,
                size: size as u32,
            })
        });
    }

    /// `e`, lowered from syntax node `n`, is done: its children's records are above `mark`.
    pub(crate) fn seal_expr(&mut self, mark: usize, n: &SyntaxNode, e: &ir::Expr) {
        // A node with no expression inside: no records to check.
        if let ir::Expr::Const(_)
        | ir::Expr::Load(
            ir::Place::Local(_) | ir::Place::Global(_) | ir::Place::BaseGlobal(_) | ir::Place::ExtGlobal(..),
        ) = e
        {
            if self.tys.len() == mark {
                let ty = of_node(e, &[], || {
                    self.checked
                        .types
                        .get(&key(self.file, n))
                        .map_or(ExprTy::Unknown, of_ty)
                });
                self.tys.push(TyRec {
                    ty,
                    kind: types::kind_of(e),
                    arity: 0,
                    size: 1,
                });
                return;
            }
        }
        let mut kinds: [Shape; 6] = [(0, 0, 0); 6];
        let mut count = 0;
        match e {
            ir::Expr::Binary(_, a, b) => {
                kinds[0] = shape_cheap(a);
                kinds[1] = shape_cheap(b);
                count = 2;
            }
            ir::Expr::Unary(_, x) => {
                kinds[0] = shape_cheap(x);
                count = 1;
            }
            _ => types::children(e, &mut |c| {
                if count < kinds.len() {
                    kinds[count] = shape_cheap(c);
                }
                count += 1;
            }),
        }
        let mut ctys = [ExprTy::Unknown; 6];
        if count > kinds.len() || !self.tiles(mark, &kinds[..count], &mut ctys[..count]) {
            self.tys.truncate(mark);
            self.push_unknown(e);
            return;
        }
        let ty = of_node(e, &ctys[..count], || {
            self.checked
                .types
                .get(&key(self.file, n))
                .map_or(ExprTy::Unknown, of_ty)
        });
        let size = (self.tys.len() - mark + 1) as u32;
        self.tys.push(TyRec {
            ty,
            kind: types::kind_of(e),
            arity: count.min(255) as u8,
            size,
        });
    }

    /// The statements `out`, lowered from one syntax node, are done: the records of their expressions are above `mark`.
    pub(crate) fn seal_stmts(&mut self, mark: usize, out: &[ir::Stmt]) {
        let mut roots = std::mem::take(&mut self.roots);
        roots.clear();
        for s in out {
            // The expressions of a nested statement were checked by its own seal: kind and arity only.
            types::stmt_roots_nested(s, &mut |e, nested| {
                roots.push(if nested { shape_cheap(e) } else { shape_of(e) })
            });
        }
        let ok = self.tiles(mark, &roots, &mut []);
        self.roots = roots;
        if !ok {
            self.tys.truncate(mark);
            for s in out {
                types::stmt_roots(s, &mut |e| self.push_unknown(e));
            }
        }
    }

    /// The table of the routine whose body is `body`: empty unless the records tile the whole body, and every
    /// record is the record of the node at its place (kind, number of expressions directly inside, size).
    /// Lowering checks only kind and number directly inside while it works, so one pass here makes the
    /// sizes exact.
    pub(crate) fn take_table(&mut self, body: &[ir::Stmt]) -> Vec<ExprTy> {
        let mut roots = std::mem::take(&mut self.roots);
        roots.clear();
        for s in body {
            types::stmt_roots(s, &mut |e| roots.push(shape_cheap(e)));
        }
        let ok = self.tiles(0, &roots, &mut []);
        #[cfg(debug_assertions)]
        let mut ok = ok;
        self.roots = roots;
        #[cfg(debug_assertions)]
        if ok {
            let mut i = 0;
            for s in body {
                types::stmt_roots(s, &mut |e| ok = ok && self.exact(e, &mut i).is_some());
            }
            // A record that is not its node's means a lowering function built an expression beside `expr`.
            // Release builds keep the table when its length is the body's (`ExprTypes::new` checks it).
            assert!(
                ok && i == self.tys.len(),
                "the records are not the records of the body's nodes"
            );
        }
        let table = if ok {
            self.tys.iter().map(|r| r.ty).collect()
        } else {
            Vec::new()
        };
        self.tys.clear();
        table
    }

    #[cfg(debug_assertions)]
    /// The size of `e` when the records from `*i` on are its records, and `*i` after them.
    fn exact(&self, e: &ir::Expr, i: &mut usize) -> Option<usize> {
        let mut size = 1;
        let mut ok = true;
        let mut arity = 0usize;
        types::children(e, &mut |c| {
            arity += 1;
            if ok {
                match self.exact(c, i) {
                    Some(n) => size += n,
                    None => ok = false,
                }
            }
        });
        let r = self.tys.get(*i)?;
        *i += 1;
        (ok && r.kind == types::kind_of(e) && r.arity as usize == arity.min(255) && r.size as usize == size)
            .then_some(size)
    }
}
