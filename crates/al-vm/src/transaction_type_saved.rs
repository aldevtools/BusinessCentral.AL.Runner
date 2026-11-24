//! A `Database.CurrentTransactionType` read while the type is not known (a test method before did not run to its end and
//! could set it, `TypeOrigin::unknown`) whose value only sets the type back: `Saved := CurrentTransactionType;` and, after
//! it in the same procedure, `CurrentTransactionType(Saved);`, both statements of the procedure's own body (not inside an
//! `if`, a loop or a `case`), the local `Saved` used nowhere else (Microsoft's `Library - Manufacturing.RefreshProdOrder`,
//! al-runner issue 8969). The value is not observed, so the read runs; the set back gives the type the read saw, which is
//! not known again. Entry points: `Session::read_saved_for_set_back` (the read), `Session::set_back_saved` (the set).

use crate::transaction_type::TypeOrigin;
use crate::{Session, Signal};
use al_builtins::assumptions::{self, Assumption};
use al_builtins::builtin_id::ids;
use al_builtins::native::unsupported;
use al_ir::{Arg, Expr, Place, Routine, Stmt};

/// A read in a save statement while the type was not known, waiting for its set back (`TxState::saved_types`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SavedType {
    /// The AL call depth of the procedure (`Session::call_stack`), its object and routine.
    depth: usize,
    object: usize,
    routine: usize,
    /// The position id of the set back statement (`Stmt::Pos`).
    set_back: u32,
    /// What the type rested on at the read.
    origin: TypeOrigin,
}

/// The statements of `routine`'s own body that save the type in a local and set it back, as `(local, position id of the
/// save, position id of the set back)`: exactly one save and, after it, exactly one set back, each a statement of the body
/// itself with its own position; the local is declared in the procedure (not a parameter or the return value) and these
/// two statements are its only uses.
fn save_and_set_back(routine: &Routine) -> Option<(usize, u32, u32)> {
    let body = routine.body.stmts();
    let ctt = |e: &Expr| matches!(e, Expr::Builtin { id, receiver: None, .. } if *id == ids::DATABASE_CURRENTTRANSACTIONTYPE_1);
    // The read, as the save assigns it: to a TransactionType local the lowering converts it to the same option.
    let read = |e: &Expr| match e {
        Expr::Convert(al_builtins::value::RtType::Option(m), x) => {
            al_builtins::sysenum::option_members("TransactionType").is_some_and(|t| m[..] == t[..])
                && matches!(&**x, Expr::Builtin { args, .. } if ctt(x) && args.is_empty())
        }
        _ => false,
    };
    // Each statement with the position id that marks it.
    let placed = body.windows(2).filter_map(|w| match w {
        [Stmt::Pos(p), s] => Some((*p, s)),
        _ => None,
    });
    let mut save = None;
    let mut set_back = None;
    for (p, s) in placed {
        match s {
            Stmt::Assign(Place::Local(l), e) if read(e) => {
                if save.is_some() {
                    return None;
                }
                save = Some((*l, p));
            }
            Stmt::Expr(e @ Expr::Builtin { args, .. }) if ctt(e) => {
                if let [Arg::Value(Expr::Load(Place::Local(l)))] = args.as_slice()
                    && save.is_some_and(|(saved, _)| saved == *l)
                {
                    if set_back.is_some() {
                        return None;
                    }
                    set_back = Some(p);
                }
            }
            _ => {}
        }
    }
    let ((local, save), set_back) = (save?, set_back?);
    if local < routine.params || Some(local) == routine.ret_local {
        return None;
    }
    let is_local = |p: &Place| crate::capture::root_local(p) == Some(local);
    // The save is the one statement that writes the local.
    let writes = body
        .iter()
        .filter(|s| !crate::effects::stmts_keep_local(std::slice::from_ref(s), &is_local));
    if writes.count() != 1 {
        return None;
    }
    // The set back's argument is the one expression that reads it, and no call takes it by `var`.
    let (mut loads, mut by_var) = (0, false);
    let refs = |args: &[Arg]| args.iter().any(|a| matches!(a, Arg::Ref(p) if is_local(p)));
    al_ir::types::for_each_expr_post(body, &mut |e| match e {
        Expr::Load(p) if is_local(p) => loads += 1,
        Expr::Builtin { receiver, args, .. } => {
            by_var |= refs(args) || receiver.as_deref().is_some_and(|r| refs(std::slice::from_ref(r)));
        }
        Expr::Call { args, .. } | Expr::InterfaceCall { args, .. } | Expr::DotNet { args, .. } => by_var |= refs(args),
        _ => {}
    });
    (loads == 1 && !by_var).then_some((local, save, set_back))
}

impl Session {
    /// The running procedure: its call depth, object, routine and the position id of its running statement.
    fn running_statement(&self) -> Option<(usize, usize, usize, u32)> {
        let f = self.call_stack.last()?;
        Some((self.call_stack.len(), *self.frames.last()?, f.routine, f.pos))
    }

    /// A read of the type while it is not known, in the save statement of [`save_and_set_back`]: the value the session
    /// holds, which only the set back uses, and the read waits for that set. `None` for every other read. A read forgets
    /// the saves of its own procedure and the ones it called: a save of a procedure that stopped before its set back.
    pub(crate) fn read_saved_for_set_back(&mut self) -> Option<i32> {
        let (depth, object, routine, pos) = self.running_statement()?;
        self.tx.saved_types.retain(|s| s.depth < depth);
        let origin = self.tx.type_origin;
        if !origin.unknown {
            return None;
        }
        let (_, save, set_back) = save_and_set_back(&self.engine.program.objects[object].routines[routine])?;
        if save != pos {
            return None;
        }
        self.tx.saved_types.push(SavedType {
            depth,
            object,
            routine,
            set_back,
            origin,
        });
        // It rests on the test before having left a type, as a write there does (`gate_read_only_type`).
        assumptions::add_hit_bits(origin.marks);
        assumptions::assume(Assumption::TransactionTypeCarriesToNextTest);
        Some(self.tx.transaction_type)
    }

    /// A set in the set back statement of a save that read a type not known: the set takes effect only where every set
    /// does (`fresh`: after a Commit, before any database call, `transaction_type.rs`), and the type is then again the
    /// one the read saw, not known. Anywhere else what the set does depends on the value: refused. `None` for every
    /// other set.
    pub(crate) fn set_back_saved(&mut self, new: i32, fresh: bool) -> Option<Result<(), Signal>> {
        let (depth, object, routine, pos) = self.running_statement()?;
        let at = self
            .tx
            .saved_types
            .iter()
            .rposition(|s| (s.depth, s.object, s.routine, s.set_back) == (depth, object, routine, pos))?;
        let saved = self.tx.saved_types.remove(at);
        if !fresh {
            return Some(unsupported(
                "Database.CurrentTransactionType set back to the type read while the type a test method before left was \
                 not known, after a database call or at a codeunit's first test method (what the set does depends on the \
                 type: not pinned)",
            ));
        }
        // No cell measures an Update report after it: the moment ends (`OpenTx::moment`).
        self.moment_type_set(new, false);
        self.tx.transaction_type = new;
        self.tx.type_origin = saved.origin;
        Some(Ok(()))
    }
}
