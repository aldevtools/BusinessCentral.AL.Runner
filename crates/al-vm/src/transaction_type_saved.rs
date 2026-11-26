//! A `Database.CurrentTransactionType` read while the type is not known (a test method before did not run to its end and
//! could set it, `TypeOrigin::unknown`) whose value only sets the type back: `Saved := CurrentTransactionType;` and, after
//! it in the same procedure, `CurrentTransactionType(Saved);`, both statements of the procedure's own body (not inside an
//! `if`, a loop or a `case`), the local `Saved` used nowhere else, and no other read of the type in the procedure
//! (Microsoft's `Library - Manufacturing.RefreshProdOrder`, al-runner issue 8969). The value is not observed, so the read
//! runs; the set back gives the type the read saw, which is not known again. Entry points:
//! `Session::read_saved_for_set_back` (the read), `Session::set_back_saved` (the set).

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
    /// The session's rollbacks at the read (`Session::rollbacks`): a set after a rollback is not measured.
    rollbacks: u64,
}

#[cfg(test)]
impl SavedType {
    /// A read waiting for its set back, of no routine in particular (al-vm `login_image` tests).
    pub(crate) fn any() -> SavedType {
        SavedType {
            depth: 0,
            object: 0,
            routine: 0,
            set_back: 0,
            origin: TypeOrigin::default(),
            rollbacks: 0,
        }
    }
}

/// The statements of `routine`'s own body that save the type in a local and set it back, as `(position id of the save,
/// position id of the set back)`: exactly one save and, after it, exactly one set back, each a statement of the body
/// itself with its own position; the local is declared in the procedure (not a parameter or the return value), these
/// two statements are its only uses, and the save's read is the procedure's only read of the type.
fn save_and_set_back(routine: &Routine) -> Option<(u32, u32)> {
    let body = routine.body.stmts();
    let ctt = |e: &Expr| matches!(e, Expr::Builtin { id, receiver: None, .. } if *id == ids::DATABASE_CURRENTTRANSACTIONTYPE_1);
    // A read: the call without arguments.
    let ctt_read = |e: &Expr| matches!(e, Expr::Builtin { args, .. } if ctt(e) && args.is_empty());
    // The read, as the save assigns it: to a TransactionType local the lowering converts it to the same option.
    let read = |e: &Expr| match e {
        Expr::Convert(al_builtins::value::RtType::Option(m), x) => {
            ctt_read(x) && m[..] == al_builtins::sysenum::transaction_type_members()[..]
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
                if save.replace((*l, p)).is_some() {
                    return None;
                }
            }
            Stmt::Expr(e @ Expr::Builtin { args, .. }) if ctt(e) => {
                if let [Arg::Value(Expr::Load(Place::Local(l)))] = args.as_slice()
                    && save.is_some_and(|(saved, _)| saved == *l)
                    && set_back.replace(p).is_some()
                {
                    return None;
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
    // The set back's argument is the one expression that reads it, and no call takes it by `var`. The save's read is the
    // procedure's one read of the type: a second read would forget the save's wait (`read_saved_for_set_back`).
    let (mut loads, mut reads) = (0, 0);
    al_ir::types::for_each_expr_post(body, &mut |e| {
        loads += usize::from(matches!(e, Expr::Load(p) if is_local(p)));
        reads += usize::from(ctt_read(e));
    });
    (loads == 1 && reads == 1 && !crate::effects::passes_by_var(body, &is_local)).then_some((save, set_back))
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
        let (save, set_back) = save_and_set_back(&self.engine.program.objects[object].routines[routine])?;
        if save != pos {
            return None;
        }
        self.tx.saved_types.push(SavedType {
            depth,
            object,
            routine,
            set_back,
            origin,
            rollbacks: self.rollbacks,
        });
        // It rests on the test before having left a type, as a write there does (`gate_read_only_type`).
        assumptions::add_hit_bits(origin.marks);
        assumptions::assume(Assumption::TransactionTypeCarriesToNextTest);
        Some(self.tx.transaction_type)
    }

    /// A set in the set back statement of a save that read a type not known: the set takes effect only where every set
    /// does (`fresh`: after a Commit, before any database call, `transaction_type.rs`) and no rollback ran since the read,
    /// and the type is then again the one the read saw, not known. Anywhere else what the set does depends on the value:
    /// refused. `None` for every other set.
    pub(crate) fn set_back_saved(&mut self, new: i32, fresh: bool) -> Option<Result<(), Signal>> {
        if self.tx.saved_types.is_empty() {
            return None;
        }
        let (depth, object, routine, pos) = self.running_statement()?;
        let at = self
            .tx
            .saved_types
            .iter()
            .rposition(|s| (s.depth, s.object, s.routine, s.set_back) == (depth, object, routine, pos))?;
        let saved = self.tx.saved_types.remove(at);
        if !(fresh && saved.rollbacks == self.rollbacks) {
            return Some(unsupported(
                "Database.CurrentTransactionType set back to the type read while the type a test method before left was \
                 not known, after a database call or a rollback (what the set does depends on the type: not pinned)",
            ));
        }
        // No cell measures an Update report after it: the moment ends (`OpenTx::moment`).
        self.moment_type_set(new, false);
        self.tx.transaction_type = new;
        self.tx.type_origin = saved.origin;
        Some(Ok(()))
    }
}
