//! IR to bytecode. Returns `None` only when the routine needs more registers than an instruction can name.

use super::*;
use al_builtins::builtin_id::{Class, ids};
use al_builtins::value::RtType;
use types::{DeclaredTypes, ExprTypes, Ty, of_rt};

/// Compiles `r`.
/// `captured`: the form a value capture runs (ADR-030): statement and loop ops that tell the capture
/// what a statement did.
pub(crate) fn compile(r: &ir::Routine, captured: bool) -> Option<Chunk> {
    compile_form(r, captured, false)
}

/// The debug form of `r` (ADR-048): the plain form with an `Op::DbgHit` after each statement marker, where a debugger
/// may stop. The ops between two markers are the plain form's, so a debugged test computes what it computes without one.
pub(crate) fn compile_debug(r: &ir::Routine) -> Option<Chunk> {
    compile_form(r, false, true)
}

fn compile_form(r: &ir::Routine, captured: bool, debug: bool) -> Option<Chunk> {
    let nlocals = u16::try_from(r.locals.len()).ok()?;
    let mut c = Compiler {
        locals_ty: r.locals.iter().map(|l| l.ty.clone()).collect(),
        tys: DeclaredTypes {
            locals: r.locals.iter().map(|l| l.ty.clone()).collect(),
            lowered: al_ir::types::ExprTypes::new(r),
        },
        chunk: Chunk {
            code: Vec::new(),
            cap_sites: Vec::new(),
            consts: Vec::new(),
            places: Vec::new(),
            types: Vec::new(),
            calls: Vec::new(),
            builtins: Vec::new(),
            fors: Vec::new(),
            appends: Vec::new(),
            dotnets: Vec::new(),
            aux: Vec::new(),
            foreachs: Vec::new(),
            specials: Vec::new(),
            faults: Vec::new(),
            ifaces: Vec::new(),
            nlocals,
            temps: 0,
        },
        next: nlocals,
        max: nlocals,
        loops: Vec::new(),
        captured,
        cap_depth: 0,
        debug,
    };
    c.block(&r.body).ok()?;
    c.chunk.temps = c.max - nlocals;
    if std::env::var_os("AL_EXEC_DUMP").is_some() {
        eprintln!("routine {} ({} locals, {} temps)", r.name, nlocals, c.chunk.temps);
        for (i, op) in c.chunk.code.iter().enumerate() {
            eprintln!("  {i:3} {op:?}");
        }
    }
    Some(c.chunk)
}

struct LoopCtx {
    /// `cap_depth` at the loop statement: a `break` or `continue` closes the statements above it.
    depth: u32,
    /// Jumps to patch to the loop's exit and to its next round.
    brk: Vec<usize>,
    cont: Vec<usize>,
}

struct Compiler<'a> {
    /// The declared type of each local.
    locals_ty: Vec<RtType>,
    /// Where the static types come from; the instruction selection asks only this.
    tys: DeclaredTypes<'a>,
    chunk: Chunk,
    next: u16,
    max: u16,
    loops: Vec<LoopCtx>,
    captured: bool,
    /// The statements open at this point of the code (captured form only).
    cap_depth: u32,
    /// The debug form (ADR-048).
    debug: bool,
}

type R<T> = Result<T, ()>;

/// Evaluating `e` cannot write a local or a global: no call, no builtin, no tree node.
/// An expression whose evaluation cannot fail: a constant or a plain variable.
fn trivial(e: &ir::Expr) -> bool {
    matches!(
        e,
        ir::Expr::Const(_) | ir::Expr::Load(ir::Place::Local(_) | ir::Place::Global(_))
    )
}

fn pure(e: &ir::Expr) -> bool {
    use ir::Expr as E;
    match e {
        E::Const(_) | E::Load(ir::Place::Local(_) | ir::Place::Global(_)) => true,
        E::Unary(_, a) | E::Convert(_, a) => pure(a),
        E::Binary(_, a, b) => pure(a) && pure(b),
        _ => false,
    }
}

/// The builtins the VM runs itself on operands it holds (`Op::Special`).
fn special(id: al_builtins::builtin_id::BuiltinId) -> bool {
    matches!(
        id,
        ids::SESSION_BINDSUBSCRIPTION_1
            | ids::SESSION_UNBINDSUBSCRIPTION_1
            | ids::RECORD_SETCURRFIELDNO_1
            | ids::RECORD_CURRFIELDNO_1
            | ids::SYSTEM_CLEARALL_1
            | ids::SESSION_APPLICATIONAREA_1
            | ids::TEXT_MAXSTRLENOFVARPARAMETER_1
    )
}

/// The builtins the VM runs itself on evaluated operands (`Session::vm_class_call`).
fn vm_class(id: al_builtins::builtin_id::BuiltinId) -> bool {
    let class = id.class();
    class != Class::Other && class != Class::InStreamRead
        || matches!(
            id,
            ids::REQUESTPAGE_UPDATE_1 | ids::REQUESTPAGE_CAPTION_1 | ids::DATABASE_ISINWRITETRANSACTION_1
        )
}

/// An expression that cannot fail and writes nothing.
fn simple(e: &ir::Expr) -> bool {
    matches!(
        e,
        ir::Expr::Const(_) | ir::Expr::Load(ir::Place::Local(_) | ir::Place::Global(_))
    )
}

/// Whether a place has an index expression that evaluating it would run.
fn has_index(p: &ir::Place) -> bool {
    match p {
        ir::Place::Index(..) => true,
        ir::Place::Field(base, _) => has_index(base),
        _ => false,
    }
}

impl<'a> Compiler<'a> {
    fn emit(&mut self, op: Op) -> usize {
        self.chunk.code.push(op);
        self.chunk.code.len() - 1
    }

    fn here(&self) -> u32 {
        self.chunk.code.len() as u32
    }

    fn patch(&mut self, at: usize, to: u32) {
        match &mut self.chunk.code[at] {
            Op::Jump { t }
            | Op::JumpIfNot { t, .. }
            | Op::JumpIfNotBool { t, .. }
            | Op::JumpUnlessEqInt { t, .. }
            | Op::JumpUnlessNeInt { t, .. }
            | Op::JumpUnlessLtInt { t, .. }
            | Op::JumpUnlessLeInt { t, .. }
            | Op::JumpUnlessGtInt { t, .. }
            | Op::JumpUnlessGeInt { t, .. }
            | Op::JumpEq { t, .. }
            | Op::JumpLt { t, .. }
            | Op::JumpGt { t, .. }
            | Op::ForeachNext { t, .. }
            | Op::ForTest { t, .. } => *t = to,
            other => unreachable!("not a jump: {other:?}"),
        }
    }

    fn alloc(&mut self) -> R<u16> {
        let r = self.next;
        self.next = self.next.checked_add(1).ok_or(())?;
        self.max = self.max.max(self.next);
        Ok(r)
    }

    fn k(&mut self, c: &ir::Const) -> u32 {
        self.chunk.consts.push(c.clone());
        (self.chunk.consts.len() - 1) as u32
    }

    fn place(&mut self, p: &ir::Place) -> u32 {
        self.chunk.places.push(p.clone());
        (self.chunk.places.len() - 1) as u32
    }

    fn is_local(&self, r: u16) -> bool {
        r < self.chunk.nlocals
    }

    // ---- expressions ----

    /// The register that holds the value of `e`, which may be a local itself: the caller must not let anything write
    /// that local before it reads the register.
    fn expr_any(&mut self, e: &'a ir::Expr) -> R<u16> {
        if let ir::Expr::Load(ir::Place::Local(i)) = e {
            return u16::try_from(*i).map_err(|_| ());
        }
        let d = self.alloc()?;
        self.expr_into(e, d)?;
        Ok(d)
    }

    /// The register that holds a copy of the value of `e` (a temporary), safe against later writes to locals.
    fn expr_copy(&mut self, e: &'a ir::Expr) -> R<u16> {
        let r = self.expr_any(e)?;
        if self.is_local(r) {
            let d = self.alloc()?;
            self.emit(Op::Move { d, a: r });
            Ok(d)
        } else {
            Ok(r)
        }
    }

    /// Operands `a` then `b`: `a` must be a copy when evaluating `b` could write a local.
    fn pair(&mut self, a: &'a ir::Expr, b: &'a ir::Expr) -> R<(u16, u16)> {
        let ra = if pure(b) { self.expr_any(a)? } else { self.expr_copy(a)? };
        let rb = self.expr_any(b)?;
        Ok((ra, rb))
    }

    fn aux(&mut self, site: AuxSite, d: u16, a: u16) {
        self.chunk.aux.push(site);
        let s = (self.chunk.aux.len() - 1) as u32;
        self.emit(Op::Aux { s, d, a });
    }

    fn typed_binary(&mut self, op: ir::BinOp, l: &'a ir::Expr, r: &'a ir::Expr, d: u16) -> R<bool> {
        use ir::BinOp as B;
        let (Some(tl), Some(tr)) = (self.tys.expr_ty(l), self.tys.expr_ty(r)) else {
            return Ok(false);
        };
        if (tl, tr) == (Ty::Boolean, Ty::Boolean) {
            if !matches!(op, B::And | B::Or | B::Xor) {
                return Ok(false);
            }
            let (a, b) = self.pair(l, r)?;
            self.emit(match op {
                B::And => Op::AndBool { d, a, b },
                B::Or => Op::OrBool { d, a, b },
                _ => Op::XorBool { d, a, b },
            });
            return Ok(true);
        }
        if (tl, tr) != (Ty::Integer, Ty::Integer) {
            return Ok(false);
        }
        if matches!(op, B::Add | B::Sub) {
            if let ir::Expr::Const(ir::Const::Integer(k)) = r {
                if op == B::Add || *k != i32::MIN {
                    let a = self.expr_any(l)?;
                    let k = if op == B::Add { *k } else { -*k };
                    self.emit(Op::AddIntK { d, a, k });
                    return Ok(true);
                }
            }
        }
        if !matches!(
            op,
            B::Add | B::Sub | B::Mul | B::IntDiv | B::Mod | B::Eq | B::Ne | B::Lt | B::Le | B::Gt | B::Ge
        ) {
            return Ok(false);
        }
        let (a, b) = self.pair(l, r)?;
        self.emit(match op {
            B::Add => Op::AddInt { d, a, b },
            B::Sub => Op::SubInt { d, a, b },
            B::Mul => Op::MulInt { d, a, b },
            B::IntDiv => Op::DivInt { d, a, b },
            B::Mod => Op::ModInt { d, a, b },
            B::Eq => Op::EqInt { d, a, b },
            B::Ne => Op::NeInt { d, a, b },
            B::Lt => Op::LtInt { d, a, b },
            B::Le => Op::LeInt { d, a, b },
            B::Gt => Op::GtInt { d, a, b },
            _ => Op::GeInt { d, a, b },
        });
        Ok(true)
    }

    /// Evaluates the condition `c` and emits the jump taken when it is false, to be patched; returns the jump's index.
    /// An Integer compare is fused with the jump.
    fn cond_jump(&mut self, c: &'a ir::Expr, t: u32) -> R<usize> {
        use ir::BinOp as B;
        if let ir::Expr::Binary(op @ (B::Eq | B::Ne | B::Lt | B::Le | B::Gt | B::Ge), l, r) = c {
            if (self.tys.expr_ty(l), self.tys.expr_ty(r)) == (Some(Ty::Integer), Some(Ty::Integer)) {
                let (a, b) = self.pair(l, r)?;
                return Ok(self.emit(match op {
                    B::Eq => Op::JumpUnlessEqInt { a, b, t },
                    B::Ne => Op::JumpUnlessNeInt { a, b, t },
                    B::Lt => Op::JumpUnlessLtInt { a, b, t },
                    B::Le => Op::JumpUnlessLeInt { a, b, t },
                    B::Gt => Op::JumpUnlessGtInt { a, b, t },
                    _ => Op::JumpUnlessGeInt { a, b, t },
                }));
            }
        }
        let boolean = self.tys.expr_ty(c) == Some(Ty::Boolean);
        let a = self.expr_any(c)?;
        Ok(self.emit(if boolean {
            Op::JumpIfNotBool { a, t }
        } else {
            Op::JumpIfNot { a, t }
        }))
    }

    /// The builtins the VM runs on operands it holds (`special`): `false` when the shape of the call is not one it knows, which
    /// then raises the error of a builtin nothing implements.
    fn special(
        &mut self,
        id: al_builtins::builtin_id::BuiltinId,
        receiver: Option<&'a ir::Arg>,
        args: &'a [ir::Arg],
        ret_used: bool,
        d: u16,
    ) -> R<bool> {
        let (mut place, mut arg) = (None, None);
        match id {
            ids::SESSION_BINDSUBSCRIPTION_1 | ids::SESSION_UNBINDSUBSCRIPTION_1 => {
                let Some(ir::Arg::Value(e)) = args.first() else {
                    return Ok(false);
                };
                arg = Some(self.expr_copy(e)?);
            }
            ids::RECORD_SETCURRFIELDNO_1 => {
                let (Some(ir::Arg::Ref(p)), Some(ir::Arg::Value(e))) = (receiver, args.first()) else {
                    return Ok(false);
                };
                place = Some(self.rplace(p)?);
                arg = Some(self.expr_copy(e)?);
            }
            ids::RECORD_CURRFIELDNO_1 => {
                let Some(ir::Arg::Ref(p)) = receiver else {
                    return Ok(false);
                };
                place = Some(self.rplace(p)?);
            }
            ids::SYSTEM_CLEARALL_1 => {}
            ids::TEXT_MAXSTRLENOFVARPARAMETER_1 => {
                let Some(ir::Arg::Ref(p)) = args.first() else {
                    return Ok(false);
                };
                place = Some(self.rplace(p)?);
            }
            ids::SESSION_APPLICATIONAREA_1 => match args.first() {
                None => {}
                Some(ir::Arg::Value(e)) => arg = Some(self.expr_copy(e)?),
                Some(_) => return Ok(false),
            },
            _ => return Ok(false),
        }
        self.chunk.specials.push(SpecialSite {
            id,
            place,
            arg,
            ret_used,
        });
        let s = (self.chunk.specials.len() - 1) as u32;
        self.emit(Op::Special { s, d });
        Ok(true)
    }

    fn fault(&mut self, k: FaultKind) {
        self.chunk.faults.push(k);
        let k = (self.chunk.faults.len() - 1) as u32;
        self.emit(Op::Fault { k });
    }

    fn expr_into(&mut self, e: &'a ir::Expr, d: u16) -> R<()> {
        use ir::Expr as E;
        let mark = self.next;
        match e {
            E::Const(ir::Const::Integer(v)) => {
                self.emit(Op::ConstI { d, v: *v });
            }
            E::Const(c) => {
                let k = self.k(c);
                self.emit(Op::Const { d, k });
            }
            E::Load(ir::Place::Local(i)) => {
                let a = u16::try_from(*i).map_err(|_| ())?;
                self.emit(Op::Move { d, a });
            }
            E::Load(ir::Place::Global(i)) => {
                self.emit(Op::LoadGlobal { d, i: *i as u32 });
            }
            E::Load(p) => {
                let rp = self.rplace(p)?;
                let p = self.place(&rp);
                self.emit(Op::ReadPlace { d, p });
            }
            E::Unary(ir::UnOp::Not, a) if self.tys.expr_ty(a) == Some(Ty::Boolean) => {
                let a = self.expr_any(a)?;
                self.emit(Op::NotBool { d, a });
            }
            E::Unary(op, a) => {
                let a = self.expr_any(a)?;
                self.emit(Op::Un { op: *op, d, a });
            }
            E::Binary(op, l, r) if self.typed_binary(*op, l, r, d)? => {}
            E::Binary(op, l, r) => {
                let (a, b) = self.pair(l, r)?;
                use ir::BinOp as B;
                if matches!(
                    op,
                    B::Add | B::Sub | B::Mul | B::Mod | B::Eq | B::Ne | B::Lt | B::Le | B::Gt | B::Ge
                ) {
                    self.emit(Op::BinI { op: *op, d, a, b });
                } else {
                    self.emit(Op::Bin { op: *op, d, a, b });
                }
            }
            // A conversion to the type the operand is proven to have changes nothing.
            E::Convert(t, a) if of_rt(t).is_some() && of_rt(t) == self.tys.expr_ty(a) => {
                self.expr_into(a, d)?;
            }
            E::Convert(t, a) => {
                let a = self.expr_any(a)?;
                self.chunk.types.push(t.clone());
                let t = (self.chunk.types.len() - 1) as u32;
                self.emit(Op::Convert { d, a, t });
            }
            E::Call {
                object,
                routine,
                instance,
                args,
                try_call,
            } => {
                // Arguments that can fail are evaluated inside the try when the result is used (`Op::TryArgs`).
                let guard = (*try_call
                    && (instance.as_ref().is_some_and(|i| !trivial(i))
                        || !args.iter().all(|a| match a {
                            ir::Arg::Value(e) => trivial(e),
                            ir::Arg::Ref(p) => matches!(p, ir::Place::Local(_) | ir::Place::Global(_)),
                        })))
                .then(|| self.emit(Op::TryArgs { end: 0, d }));
                let instance = match instance {
                    Some(i) => Some(self.expr_copy(i)?),
                    None => None,
                };
                let (arg_regs, site_args) = self.arg_regs(args)?;
                if let Some(at) = guard {
                    let end = self.here();
                    if let Op::TryArgs { end: e, .. } = &mut self.chunk.code[at] {
                        *e = end;
                    }
                }
                self.chunk.calls.push(CallSite {
                    object: *object,
                    routine: *routine,
                    instance,
                    arg_regs,
                    args: site_args,
                    try_call: *try_call,
                });
                let s = (self.chunk.calls.len() - 1) as u32;
                self.emit(Op::Call { s, d });
            }
            E::Builtin {
                id,
                receiver,
                args,
                ret_used,
            } if special(*id) && self.special(*id, receiver.as_deref(), args, *ret_used, d)? => {}
            E::Builtin {
                id,
                receiver,
                args,
                ret_used,
            } if !special(*id) && (vm_class(*id) || al_builtins::native::by_id(*id).is_some()) => {
                let vm = vm_class(*id);
                let native = if vm { None } else { al_builtins::native::by_id(*id) };
                let n = args.len();
                let mut regs = Vec::with_capacity(n + 1);
                let mut kept: Vec<ir::Arg> = Vec::with_capacity(n + 1);
                let mut all: Vec<&'a ir::Arg> = Vec::with_capacity(n + 1);
                if let Some(r) = receiver.as_deref() {
                    all.push(r);
                }
                all.extend(args.iter());
                for (i, a) in all.iter().enumerate() {
                    let rest_pure = all[i + 1..].iter().all(|b| arg_pure(b));
                    let (r, arg) = self.arg_reg(a, rest_pure)?;
                    regs.push(r);
                    kept.push(arg);
                }
                let receiver_reg = if receiver.is_some() { Some(regs.remove(0)) } else { None };
                let receiver_arg = if receiver.is_some() { Some(kept.remove(0)) } else { None };
                let by_ref = |a: &ir::Arg| matches!(a, ir::Arg::Ref(_));
                let plain = !vm
                    && native.is_some()
                    && *id != al_builtins::builtin_id::ids::SYSTEM_CLEAR_1
                    && *id != al_builtins::builtin_id::ids::SYSTEM_COPYARRAY_1
                    && !receiver_arg.as_ref().is_some_and(by_ref)
                    && !kept.iter().any(by_ref);
                self.chunk.builtins.push(BuiltinSite {
                    id: *id,
                    vm_class: vm,
                    native,
                    class: id.class(),
                    receiver: receiver_reg,
                    receiver_arg,
                    arg_regs: regs,
                    args: kept,
                    ret_used: *ret_used,
                    plain,
                });
                let s = (self.chunk.builtins.len() - 1) as u32;
                self.emit(Op::Builtin { s, d });
            }
            E::CharAt(text, index) => {
                let (a, b) = self.pair(text, index)?;
                self.emit(Op::CharAt { d, a, b });
            }
            E::In(v, set) => {
                let a = self.expr_copy(v)?;
                let hits = self.label_tests(a, set)?;
                let k = self.k(&ir::Const::Boolean(false));
                self.emit(Op::Const { d, k });
                let over = self.emit(Op::Jump { t: 0 });
                let hit_at = self.here();
                for h in hits {
                    self.patch(h, hit_at);
                }
                let k = self.k(&ir::Const::Boolean(true));
                self.emit(Op::Const { d, k });
                let end = self.here();
                self.patch(over, end);
            }
            E::DotNet { ty, receiver, op, args } => {
                let recv = match receiver.as_deref() {
                    Some(r) => Some(self.expr_copy(r)?),
                    None => None,
                };
                let (arg_regs, site_args) = self.arg_regs(args)?;
                self.chunk.dotnets.push(DotNetSite {
                    ty: *ty,
                    op: op.clone(),
                    recv,
                    arg_regs,
                    args: site_args,
                });
                let s = (self.chunk.dotnets.len() - 1) as u32;
                self.emit(Op::DotNet { s, d });
            }
            E::InterfaceCall {
                receiver,
                targets,
                args,
            } => {
                // The target is resolved before the arguments are evaluated (a receiver that holds nothing raises before an
                // argument runs). Plain operands cannot tell the order; for any other the target is resolved first.
                let plain = args.iter().all(|a| {
                    matches!(a, ir::Arg::Value(v) if simple(v))
                        || matches!(a, ir::Arg::Ref(ir::Place::Local(_) | ir::Place::Global(_)))
                });
                let recv = self.expr_copy(receiver)?;
                self.chunk.ifaces.push(IfaceSite {
                    targets: targets.clone(),
                    recv,
                    arg_regs: Vec::new(),
                    args: Vec::new(),
                });
                let s = (self.chunk.ifaces.len() - 1) as u32;
                if !plain {
                    self.emit(Op::IfaceResolve { r: recv, s });
                }
                let (arg_regs, site_args) = self.arg_regs(args)?;
                self.chunk.ifaces[s as usize].arg_regs = arg_regs;
                self.chunk.ifaces[s as usize].args = site_args;
                self.emit(Op::IfaceCall { s, d });
            }
            E::DotNetCoerce(ty, inner) => {
                self.emit(Op::DotNetClear);
                let a = self.expr_any(inner)?;
                self.chunk.types.push(ty.clone());
                let t = (self.chunk.types.len() - 1) as u32;
                self.emit(Op::DotNetCoerce { d, a, t });
            }
            E::DotNetOperand(inner, h) => {
                self.emit(Op::DotNetClear);
                let a = self.expr_any(inner)?;
                self.emit(Op::DotNetOperand { d, a, h: *h });
            }
            E::ThisInstance => {
                self.emit(Op::ThisInstance { d });
            }
            E::BaseInstance => {
                self.emit(Op::BaseInstance { d });
            }
            E::Ternary(c, a, b) => {
                let jf = self.cond_jump(c, 0)?;
                self.expr_into(a, d)?;
                let jend = self.emit(Op::Jump { t: 0 });
                let else_at = self.here();
                self.patch(jf, else_at);
                self.expr_into(b, d)?;
                let end = self.here();
                self.patch(jend, end);
            }
            E::InterfaceTest { value, objects, cast } => {
                let a = self.expr_any(value)?;
                self.aux(
                    AuxSite::InterfaceTest {
                        objects: objects.clone(),
                        cast: *cast,
                    },
                    d,
                    a,
                );
            }
            E::EnumImpl {
                value,
                by_ordinal,
                unknown,
            } => {
                let a = self.expr_any(value)?;
                self.aux(
                    AuxSite::EnumImpl {
                        by_ordinal: by_ordinal.clone(),
                        unknown: *unknown,
                    },
                    d,
                    a,
                );
            }
            E::Control { page, control, runtime } => {
                let a = self.expr_any(page)?;
                self.aux(
                    AuxSite::Control {
                        control: *control,
                        runtime: *runtime,
                    },
                    d,
                    a,
                );
            }
            E::FieldOf(base, pos) => {
                let a = self.expr_any(base)?;
                self.aux(AuxSite::FieldOf { pos: *pos }, d, a);
            }
            E::QueryColumn { query, column } => {
                let a = self.expr_any(query)?;
                self.aux(AuxSite::QueryColumn { column: *column }, d, a);
            }
            E::TddFit { max, why, inner } => {
                let a = self.expr_any(inner)?;
                self.aux(
                    AuxSite::TddFit {
                        max: *max,
                        why: why.clone(),
                    },
                    d,
                    a,
                );
            }
            E::TddStub { stub, args, ty } => {
                // The arguments run left to right for what they do, as a call's would; the stub reads none of them.
                for a in args {
                    self.expr_any(a)?;
                }
                self.aux(
                    AuxSite::TddStub {
                        stub: stub.clone(),
                        ty: ty.clone(),
                    },
                    d,
                    d,
                );
            }
            E::Refused(why) => self.fault(FaultKind::Unsupported(why.clone())),
            E::Fail { message, code } => self.fault(FaultKind::Error {
                message: message.clone(),
                code: code.clone(),
            }),
            // A builtin no native implements: it is refused before an operand is evaluated.
            E::Builtin { id, .. } => self.fault(FaultKind::Unsupported(format!("builtin {id}"))),
        }
        // Temporaries used inside `e` are free again; `d` itself stays allocated by the caller.
        self.next = mark;
        Ok(())
    }

    /// The tests of the labels of `case` or of `in [...]` on the value in `a`: the jumps (to patch) taken on a match.
    /// A label is evaluated when it is reached, in order, and the first that matches ends the tests, as before.
    fn label_tests(&mut self, a: u16, labels: &'a [ir::SetElem]) -> R<Vec<usize>> {
        let mut hits = Vec::new();
        for l in labels {
            match l {
                ir::SetElem::One(e) => {
                    let b = self.expr_copy(e)?;
                    hits.push(self.emit(Op::JumpEq { a, b, t: 0 }));
                }
                ir::SetElem::Range(lo, hi) => {
                    let mut fails = Vec::new();
                    if let Some(lo) = lo {
                        let b = self.expr_copy(lo)?;
                        fails.push(self.emit(Op::JumpLt { a, b, t: 0 }));
                    }
                    if let Some(hi) = hi {
                        let b = self.expr_copy(hi)?;
                        fails.push(self.emit(Op::JumpGt { a, b, t: 0 }));
                    }
                    hits.push(self.emit(Op::Jump { t: 0 }));
                    let next = self.here();
                    for f in fails {
                        self.patch(f, next);
                    }
                }
            }
        }
        Ok(hits)
    }

    /// `p` with the index expressions of it evaluated now, into registers that stay allocated until the instruction that
    /// uses the place has run; the place that comes back reads them as locals.
    fn rplace(&mut self, p: &'a ir::Place) -> R<ir::Place> {
        Ok(match p {
            ir::Place::Local(_) | ir::Place::Global(_) | ir::Place::BaseGlobal(_) => p.clone(),
            ir::Place::Field(base, pos) => ir::Place::Field(Box::new(self.rplace(base)?), *pos),
            ir::Place::Index(base, idx) => {
                let base = self.rplace(base)?;
                let mut regs = Vec::with_capacity(idx.len());
                for e in idx {
                    let r = self.expr_copy(e)?;
                    self.keep(r);
                    regs.push(ir::Expr::Load(ir::Place::Local(r as usize)));
                }
                ir::Place::Index(Box::new(base), regs)
            }
        })
    }

    fn arg_regs(&mut self, args: &'a [ir::Arg]) -> R<(Vec<u16>, Vec<ir::Arg>)> {
        let mut regs = Vec::with_capacity(args.len());
        let mut out = Vec::with_capacity(args.len());
        for (i, a) in args.iter().enumerate() {
            let rest_pure = args[i + 1..].iter().all(arg_pure);
            let (r, arg) = self.arg_reg(a, rest_pure)?;
            regs.push(r);
            out.push(arg);
        }
        Ok((regs, out))
    }

    /// The register of one call or builtin argument. A local is used in place when the arguments after it cannot write
    /// it; the register stays allocated until the instruction runs (the caller resets the temporaries per statement).
    /// The register of one argument, and the argument as the site keeps it: a value blanked, a place with its indexes in registers.
    fn arg_reg(&mut self, a: &'a ir::Arg, rest_pure: bool) -> R<(u16, ir::Arg)> {
        let blank = ir::Arg::Value(ir::Expr::Const(ir::Const::Integer(0)));
        match a {
            ir::Arg::Value(e) => {
                if rest_pure {
                    let r = self.expr_any(e)?;
                    self.keep(r);
                    Ok((r, blank))
                } else {
                    let r = self.expr_copy(e)?;
                    self.keep(r);
                    Ok((r, blank))
                }
            }
            ir::Arg::Ref(ir::Place::Local(i)) if rest_pure => {
                Ok((u16::try_from(*i).map_err(|_| ())?, ir::Arg::Ref(ir::Place::Local(*i))))
            }
            ir::Arg::Ref(p) => {
                let rp = self.rplace(p)?;
                let d = self.alloc()?;
                let pi = self.place(&rp);
                self.emit(Op::ReadPlace { d, p: pi });
                Ok((d, ir::Arg::Ref(rp)))
            }
        }
    }

    /// Keeps a temporary allocated past the end of the expression that made it.
    fn keep(&mut self, r: u16) {
        if !self.is_local(r) && r >= self.next {
            self.next = r + 1;
            self.max = self.max.max(self.next);
        }
    }

    // ---- statements ----

    fn block(&mut self, stmts: &'a [ir::Stmt]) -> R<()> {
        if self.captured {
            return self.block_captured(stmts);
        }
        for s in stmts {
            let mark = self.next;
            self.stmt(s)?;
            self.next = mark;
        }
        Ok(())
    }

    /// A block as the capture sees it: a marker and the statement after it are one statement (`exec_block_captured`).
    fn block_captured(&mut self, stmts: &'a [ir::Stmt]) -> R<()> {
        let mut i = 0;
        while i < stmts.len() {
            let mark = self.next;
            if let ir::Stmt::Pos(p) = &stmts[i] {
                self.stmt(&stmts[i])?;
                self.emit(Op::CapHit { p: *p });
                // Markers of an assumption sit between the position and its statement.
                let mut j = i + 1;
                while let Some(m @ (ir::Stmt::Assume(_) | ir::Stmt::TddMark(_))) = stmts.get(j) {
                    self.stmt(m)?;
                    j += 1;
                }
                if let Some(next) = stmts.get(j) {
                    self.emit(Op::CapBegin { p: *p });
                    self.cap_depth += 1;
                    self.stmt(next)?;
                    self.cap_depth -= 1;
                    self.chunk.cap_sites.push(CapSite {
                        pos: *p,
                        writes: crate::capture::written_locals(next),
                    });
                    let s = (self.chunk.cap_sites.len() - 1) as u32;
                    self.emit(Op::CapEnd { s });
                    self.next = mark;
                    i = j + 1;
                    continue;
                }
                self.next = mark;
                i = j;
                continue;
            }
            self.stmt(&stmts[i])?;
            self.next = mark;
            i += 1;
        }
        Ok(())
    }

    /// A loop statement starts: the register for the capture's id of it (captured form), after its `Enter` op.
    fn cap_loop_enter(&mut self) -> R<Option<u16>> {
        if !self.captured {
            return Ok(None);
        }
        let d = self.alloc()?;
        self.emit(Op::CapLoopEnter { d });
        Ok(Some(d))
    }

    fn cap_loop_pass(&mut self, lid: Option<u16>) {
        if let Some(a) = lid {
            self.emit(Op::CapLoopPass { a });
        }
    }

    fn cap_loop_exit(&mut self, lid: Option<u16>) {
        if let Some(a) = lid {
            self.emit(Op::CapLoopExit { a });
        }
    }

    /// The pops a `break` or `continue` needs before it leaves the statements open inside the innermost loop.
    fn cap_pops(&mut self) {
        if let Some(depth) = self.loops.last().map(|l| l.depth) {
            for _ in depth..self.cap_depth {
                self.emit(Op::CapPop);
            }
        }
    }

    fn patch_all(&mut self, list: Vec<usize>, to: u32) {
        for at in list {
            self.patch(at, to);
        }
    }

    fn stmt(&mut self, s: &'a ir::Stmt) -> R<()> {
        use ir::Stmt as S;
        match s {
            S::Pos(p) => {
                self.emit(Op::Pos { p: *p });
                if self.debug {
                    self.emit(Op::DbgHit { p: *p });
                }
            }
            S::Assign(p, e) => self.assign(p, e)?,
            S::Expr(e) => {
                let d = self.alloc()?;
                self.expr_into(e, d)?;
                self.emit(Op::Clear { a: d });
            }
            S::If(c, t, e) => {
                let jf = self.cond_jump(c, 0)?;
                self.block(t)?;
                if e.is_empty() {
                    let end = self.here();
                    self.patch(jf, end);
                } else {
                    let jend = self.emit(Op::Jump { t: 0 });
                    let else_at = self.here();
                    self.patch(jf, else_at);
                    self.block(e)?;
                    let end = self.here();
                    self.patch(jend, end);
                }
            }
            S::While(c, body) => {
                let saved = self.alloc()?;
                self.emit(Op::SavePos { d: saved });
                let lid = self.cap_loop_enter()?;
                let top = self.here();
                let mark = self.next;
                let jf = self.cond_jump(c, 0)?;
                self.next = mark;
                self.emit(Op::CheckStop);
                self.cap_loop_pass(lid);
                self.loops.push(LoopCtx {
                    depth: self.cap_depth,
                    brk: Vec::new(),
                    cont: Vec::new(),
                });
                self.block(body)?;
                let l = self.loops.pop().ok_or(())?;
                let cont = self.here();
                self.emit(Op::RestorePos { a: saved });
                self.emit(Op::Jump { t: top });
                let end = self.here();
                self.cap_loop_exit(lid);
                self.patch(jf, end);
                self.patch_all(l.cont, cont);
                self.patch_all(l.brk, end);
            }
            S::Repeat(body, until) => {
                let saved = self.alloc()?;
                self.emit(Op::SavePos { d: saved });
                let lid = self.cap_loop_enter()?;
                let top = self.here();
                self.emit(Op::CheckStop);
                self.cap_loop_pass(lid);
                self.loops.push(LoopCtx {
                    depth: self.cap_depth,
                    brk: Vec::new(),
                    cont: Vec::new(),
                });
                self.block(body)?;
                let l = self.loops.pop().ok_or(())?;
                let cont = self.here();
                self.emit(Op::RestorePos { a: saved });
                let mark = self.next;
                self.cond_jump(until, top)?;
                self.next = mark;
                let end = self.here();
                self.cap_loop_exit(lid);
                self.patch_all(l.cont, cont);
                self.patch_all(l.brk, end);
            }
            S::For {
                var,
                from,
                to,
                down,
                body,
            } => {
                let i = self.alloc()?;
                let end_reg = self.alloc()?;
                let lid = if self.captured { Some(self.alloc()?) } else { None };
                let (fr, tr) = self.pair(from, to)?;
                let rvar = self.rplace(var)?;
                let int_local = match var {
                    ir::Place::Local(v) if matches!(self.locals_ty.get(*v), Some(RtType::Integer)) => {
                        u16::try_from(*v).ok()
                    }
                    _ => None,
                };
                self.chunk.fors.push(ForSite {
                    var: rvar,
                    down: *down,
                    from: fr,
                    to: tr,
                    i,
                    end: end_reg,
                    int_local,
                    cap_loop: lid,
                });
                let site = (self.chunk.fors.len() - 1) as u32;
                self.emit(Op::ForInit { s: site });
                if let Some(d) = lid {
                    self.emit(Op::CapLoopEnter { d });
                }
                let top = self.here();
                let test = self.emit(Op::ForTest { s: site, t: 0 });
                self.emit(Op::CheckStop);
                self.emit(Op::ForSet { s: site });
                self.loops.push(LoopCtx {
                    depth: self.cap_depth,
                    brk: Vec::new(),
                    cont: Vec::new(),
                });
                let mark = self.next;
                self.block(body)?;
                self.next = mark;
                let l = self.loops.pop().ok_or(())?;
                let cont = self.here();
                self.emit(Op::ForNext { s: site });
                self.emit(Op::Jump { t: top });
                let end = self.here();
                if let Some(a) = lid {
                    self.emit(Op::CapLoopExit { a });
                }
                self.patch(test, end);
                self.patch_all(l.cont, cont);
                self.patch_all(l.brk, end);
            }
            S::Exit(None) => {
                self.emit(Op::Ret);
            }
            S::Exit(Some(e)) => {
                let a = self.expr_any(e)?;
                self.emit(Op::RetVal { a });
            }
            S::Break => match self.loops.last_mut() {
                Some(_) => {
                    self.cap_pops();
                    let at = self.emit(Op::Jump { t: 0 });
                    if let Some(l) = self.loops.last_mut() {
                        l.brk.push(at);
                    }
                }
                None => {
                    self.emit(Op::RaiseBreak);
                }
            },
            S::Continue => match self.loops.last_mut() {
                Some(_) => {
                    self.cap_pops();
                    let at = self.emit(Op::Jump { t: 0 });
                    if let Some(l) = self.loops.last_mut() {
                        l.cont.push(at);
                    }
                }
                None => {
                    self.emit(Op::RaiseContinue);
                }
            },
            S::Case {
                scrutinee,
                branches,
                otherwise,
            } => {
                let a = self.expr_copy(scrutinee)?;
                let mut ends = Vec::new();
                for (labels, body) in branches {
                    let hits = self.label_tests(a, labels)?;
                    let miss = self.emit(Op::Jump { t: 0 });
                    let hit_at = self.here();
                    for h in hits {
                        self.patch(h, hit_at);
                    }
                    self.emit(Op::Clear { a });
                    self.block(body)?;
                    ends.push(self.emit(Op::Jump { t: 0 }));
                    let next = self.here();
                    self.patch(miss, next);
                }
                self.emit(Op::Clear { a });
                self.block(otherwise)?;
                let end = self.here();
                for j in ends {
                    self.patch(j, end);
                }
            }
            S::AssertError(body) => {
                let at = self.emit(Op::Assert {
                    end: 0,
                    from: self.next,
                });
                self.block(body)?;
                let end = self.here();
                if let Op::Assert { end: e, .. } = &mut self.chunk.code[at] {
                    *e = end;
                }
            }
            S::Foreach { var, collection, body } => {
                let coll = self.expr_copy(collection)?;
                let rvar = self.rplace(var)?;
                let lid = if self.captured { Some(self.alloc()?) } else { None };
                let items = self.alloc()?;
                let at = self.alloc()?;
                let watch = self.alloc()?;
                let ver = self.alloc()?;
                self.chunk.foreachs.push(ForeachSite {
                    var: rvar,
                    coll,
                    items,
                    at,
                    watch,
                    ver,
                    collection: matches!(collection, ir::Expr::Load(_)).then(|| collection.clone()),
                    cap_loop: lid,
                });
                let site = (self.chunk.foreachs.len() - 1) as u32;
                self.emit(Op::ForeachInit { s: site });
                let top = self.here();
                let next = self.emit(Op::ForeachNext { s: site, t: 0 });
                self.loops.push(LoopCtx {
                    depth: self.cap_depth,
                    brk: Vec::new(),
                    cont: Vec::new(),
                });
                let mark = self.next;
                self.block(body)?;
                self.next = mark;
                let l = self.loops.pop().ok_or(())?;
                self.emit(Op::Jump { t: top });
                let end = self.here();
                if let Some(a) = lid {
                    self.emit(Op::CapLoopExit { a });
                }
                self.emit(Op::Clear { a: items });
                self.patch(next, end);
                self.patch_all(l.cont, top);
                self.patch_all(l.brk, end);
            }
            S::Assume(a) => {
                self.emit(Op::Assume { a: *a });
            }
            // `--tdd` (ADR-051 decision 10): the stub is noted as reached; the slot it writes is a scratch one.
            S::TddMark(stub) => {
                let mark = self.next;
                let t = self.alloc()?;
                self.aux(
                    AuxSite::TddStub {
                        stub: stub.clone(),
                        ty: None,
                    },
                    t,
                    t,
                );
                self.next = mark;
            }
        }
        Ok(())
    }

    fn assign(&mut self, p: &'a ir::Place, e: &'a ir::Expr) -> R<()> {
        // `S += x` and `S := S + x` on a text variable append in place: the append routine does it.
        if let (ir::Place::Local(_) | ir::Place::Global(_), ir::Expr::Convert(RtType::Text(_), inner)) = (p, e) {
            if let ir::Expr::Binary(ir::BinOp::Add, l, _) = &**inner {
                if matches!(&**l, ir::Expr::Load(lp) if lp == p) {
                    let ir::Expr::Convert(RtType::Text(max), inner) = e else {
                        unreachable!("matched above")
                    };
                    let ir::Expr::Binary(_, _, r) = &**inner else {
                        unreachable!("matched above")
                    };
                    // The old value is read before `x` is evaluated.
                    let old = self.alloc()?;
                    let pi = self.place(p);
                    self.emit(Op::ReadPlace { d: old, p: pi });
                    let a = self.expr_any(r)?;
                    self.chunk.appends.push(AppendSite {
                        place: p.clone(),
                        max: *max,
                    });
                    let s = (self.chunk.appends.len() - 1) as u32;
                    self.emit(Op::AppendText { s, old, a });
                    return Ok(());
                }
            }
        }
        match p {
            // A local of a typed kind takes a proven value of that kind straight into its register.
            ir::Place::Local(i)
                if self.locals_ty.get(*i).and_then(of_rt).is_some_and(|t| {
                    let inner = if let ir::Expr::Convert(_, x) = e { x } else { e };
                    self.tys.expr_ty(inner) == Some(t)
                }) =>
            {
                let i = u16::try_from(*i).map_err(|_| ())?;
                let inner = if let ir::Expr::Convert(_, x) = e { x } else { e };
                self.expr_into(inner, i)?;
            }
            ir::Place::Local(i) => {
                let i = u16::try_from(*i).map_err(|_| ())?;
                let a = self.expr_any(e)?;
                let pi = self.place(p);
                self.emit(Op::StoreLocal { i, a, p: pi });
            }
            _ => {
                let a = if has_index(p) {
                    self.expr_copy(e)?
                } else {
                    self.expr_any(e)?
                };
                let rp = self.rplace(p)?;
                let pi = self.place(&rp);
                self.emit(Op::StorePlace { p: pi, a });
            }
        }
        Ok(())
    }
}

fn arg_pure(a: &ir::Arg) -> bool {
    match a {
        ir::Arg::Value(e) => pure(e),
        ir::Arg::Ref(ir::Place::Local(_) | ir::Place::Global(_)) => true,
        ir::Arg::Ref(_) => false,
    }
}
