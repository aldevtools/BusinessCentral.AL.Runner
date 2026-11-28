//! The store cell of `ClearAll()` in a codeunit with a TableNo over a register tree that another variable shares and that
//! no call has made yet ([`super::Store::UncalledTree`], assumption clear-all-uncalled-register-tree, open corpus PR 1650,
//! tests/al-language/codeunits/clear-instance/ClearSharedCodeunitNeverCalledTree.al): the shape of "Sales-Post" clearing
//! the "Whse. Jnl.-Register Line" that "Whse.-Activity-Post" gave it through a setter (al-runner issue 8964). Which
//! codeunits form the tree is [`Session::uncalled_tree_node`]'s, which call path and holder [`Session::uncalled_tree_path`]'s
//! (from the [`RunCaller`]s that [`Session::codeunit_run_noting_caller`] keeps); the cell itself is [`super::measured`]'s.
//! Entry points: `Session::uncalled_tree_pair` (from `Session::store_kind_in`), `Session::uncalled_tree_path` (from
//! `Session::measured_cell`), `Session::codeunit_run_noting_caller` (from `Session::vm_class_call`).

use super::*;

/// A `Codeunit.Run` of a codeunit variable that runs (`Session::run_callers`): who ran which instance, how.
pub(crate) struct RunCaller {
    /// The length of `Session::call_stack` when the Run began: the index of the frame of the run codeunit's OnRun.
    at: usize,
    /// The instance of the routine that called `Run`.
    caller: Weak<RefCell<Instance>>,
    /// The instance `Run` runs.
    run: Weak<RefCell<Instance>>,
    /// The Run's result is used (`if X.Run(...) then`).
    guarded: bool,
}

impl Session {
    /// `Codeunit.Run` of codeunit instance `rc` from frame `f` (`Codeunit.Run#1`), noted in `run_callers` while it runs.
    pub(crate) fn codeunit_run_noting_caller(
        &mut self,
        rc: Rc<RefCell<Instance>>,
        values: &[Value],
        guarded: bool,
        f: &Frame,
    ) -> Result<Value, Signal> {
        self.run_callers.push(RunCaller {
            at: self.call_stack.len(),
            caller: Rc::downgrade(&f.instance),
            run: Rc::downgrade(&rc),
            guarded,
        });
        let ran = self.codeunit_run(
            al_builtins::builtin_id::ids::CODEUNIT_RUN_1,
            Some(Value::Codeunit(rc)),
            values,
            guarded,
        );
        self.run_callers.pop();
        ran
    }

    /// Whether the `ClearAll()` in `owner` that meets register instance `rc` runs where corpus PR 1650 measures it
    /// (ClearSharedCodeunitNeverCalledTree.al): in the owner's OnRun trigger (:294-300) through two local procedures,
    /// "RunWithCheck" (:302) and "ClearAllVariables" (:313-316), with no event frame; the OnRun entered by the statement
    /// form of `Run` of a codeunit variable (`SalesPost.Run(TempHeader)` :364) from a procedure that is not local (`Post`
    /// :356) of a codeunit with no Subtype, no TableNo, not SingleInstance ("CNC Holder" :349); a global of that holder's
    /// instance declared as the owner holding the instance run (`SalesPost` :353), and one declared as the register's
    /// codeunit holding `rc` (`WhseJnlRegisterLine` :352).
    pub(super) fn uncalled_tree_path(&self, owner: usize, rc: &Rc<RefCell<Instance>>) -> bool {
        let Some(run) = self.run_callers.last() else {
            return false;
        };
        let n = self.call_stack.len();
        // Whether frame `i` runs a routine of `object` that is (not) local and (not) a trigger, not as an event.
        let frame_is = |i: usize, object: usize, local: bool, trigger: bool| {
            let frame = self.call_stack[i];
            self.frames[i] == object && !frame.event && {
                let r = &self.engine.program.objects[object].routines[frame.routine];
                r.local == local && r.is_trigger == trigger
            }
        };
        let Some(holder) = run.caller.upgrade() else {
            return false;
        };
        let holder_object = holder.borrow().object as usize;
        let holds = |declared: usize, instance: &Rc<RefCell<Instance>>| {
            let globals = &self.engine.program.objects[holder_object].globals;
            holder.borrow().globals.iter().zip(globals).any(|(v, g)| {
                g.ty == RtType::Codeunit(declared as u32) && matches!(v, Value::Codeunit(x) if Rc::ptr_eq(x, instance))
            })
        };
        let Some(below) = run.at.checked_sub(1) else {
            return false;
        };
        run.at + 3 == n
            && !run.guarded
            && frame_is(n - 1, owner, true, false)
            && frame_is(n - 2, owner, true, false)
            && frame_is(run.at, owner, false, true)
            && self.engine.program.objects[owner].routines[self.call_stack[run.at].routine]
                .name
                .eq_ignore_ascii_case("OnRun")
            && frame_is(below, holder_object, false, false)
            && self.owner_kind(holder_object) == Some(Owner::Plain)
            && run.run.upgrade().is_some_and(|o| holds(owner, &o))
            && holds(rc.borrow().object as usize, rc)
    }

    /// Whether `ClearAll()` in `owner` of its global of codeunit `store` is the pair corpus PR 1650 measures
    /// (ClearSharedCodeunitNeverCalledTree.al): the owner a codeunit with a TableNo ([`Owner::TableNo`], "CNC Owner"
    /// :287-293) and the store the register of an [`Session::uncalled_tree_node`] tree ("CNC Reg" :215-229).
    pub(super) fn uncalled_tree_pair(&self, owner: usize, store: usize) -> bool {
        self.owner_kind(owner) == Some(Owner::TableNo) && self.uncalled_tree_node(store, 0)
    }

    /// Whether codeunit `object`, `depth` levels below the register (0: the register), is a node of the tree corpus PR 1650
    /// measures, by its role ([`Node`]) and the kinds of its globals ([`node_kind`]). The measured tree: the register
    /// (TableNo, not SingleInstance) "CNC Reg" holds the plain "CNC A" and "CNC B" one level below it (:224-225); "CNC B"
    /// holds "CNC A" and the plain "CNC W" two levels below it (:174-175); "CNC A" holds the SingleInstance "CNC C" and
    /// "CNC U" (:124-125), two or three levels below; "CNC C" holds the SingleInstance "CNC I" (:73), three or four levels
    /// below. No codeunit of the tree states a Subtype.
    pub(super) fn uncalled_tree_node(&self, object: usize, depth: u8) -> bool {
        let o = &self.engine.program.objects[object];
        let role = match (
            depth,
            self.engine.object_facts[object].single_instance,
            o.rec_slot.is_some(),
        ) {
            (0, false, true) => Node::Register,
            (1 | 2, false, false) => Node::Plain,
            (2..=4, true, false) => Node::SingleInstance,
            _ => return false,
        };
        o.kind == al_ir::ObjKind::Codeunit
            && self.declared_subtype(object).is_none()
            && o.globals.iter().enumerate().all(|(i, g)| {
                o.rec_slot == Some(i)
                    || match g.ty {
                        RtType::Codeunit(c) if !g.label => self.uncalled_tree_node(c as usize, depth + 1),
                        _ => node_kind(role, g),
                    }
            })
    }
}

/// The role of a codeunit in the tree of corpus PR 1650 ([`Session::uncalled_tree_node`]).
#[derive(Clone, Copy)]
enum Node {
    /// "CNC Reg" (:215-229): the register, with a TableNo, not SingleInstance.
    Register,
    /// "CNC A" (:116-133), "CNC B" (:165-181) and "CNC W" (:104-108): not SingleInstance, no TableNo.
    Plain,
    /// "CNC U" (:81-89), "CNC C" (:68-73) and "CNC I" (:50-55): SingleInstance, no TableNo.
    SingleInstance,
}

/// Whether global `g`, which is not a codeunit, is of a kind a node of role `role` declares in corpus PR 1650: records
/// that are not temporary, Integer, Boolean, `Label 'text', Comment = 'text'` and `Label 'text'` in the register
/// (:220-229); also temporary records and Date in a plain node (:119-133, :168-181, :107-108); records that are not
/// temporary, Boolean and the two Labels in a SingleInstance node (:86-89, :55).
fn node_kind(role: Node, g: &al_ir::Slot) -> bool {
    if g.label {
        return g.comment_label || g.plain_label;
    }
    match role {
        Node::Register => matches!(g.ty, RtType::Record(_, false) | RtType::Integer | RtType::Boolean),
        Node::Plain => matches!(
            g.ty,
            RtType::Record(..) | RtType::Integer | RtType::Date | RtType::Boolean
        ),
        Node::SingleInstance => matches!(g.ty, RtType::Record(_, false) | RtType::Boolean),
    }
}
