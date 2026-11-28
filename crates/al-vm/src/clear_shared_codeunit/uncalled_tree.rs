//! The store cell of `ClearAll()` in a codeunit with a TableNo over a register tree that another variable shares and that
//! no call has made yet ([`super::Store::UncalledTree`], assumption clear-all-uncalled-register-tree, open corpus PR 1650,
//! tests/al-language/codeunits/clear-instance/ClearSharedCodeunitNeverCalledTree.al): the shape of "Sales-Post" clearing
//! the "Whse. Jnl.-Register Line" that "Whse.-Activity-Post" gave it through a setter (al-runner issue 8964). Which
//! codeunits form the tree is [`Session::uncalled_tree_node`]'s, which call path and holder [`Session::uncalled_tree_path`]'s
//! (one of the [`MEASURED`] paths, read from the [`RunCaller`]s that [`Session::codeunit_run_noting_caller`] keeps; also
//! ClearSharedCodeunitNeverCalledShapes.al of PR 1650); the cell itself is [`super::measured`]'s.
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
    /// The one variable of the calling routine that holds the instance run, declared as its codeunit ([`Held`]).
    held: Option<Held>,
}

/// The facts of the call path of a `ClearAll()` over a never-called register that corpus PR 1650 varies
/// ([`Session::uncalled_tree_path`]): the holder's kind, how the holder is entered and runs the owner, which variable of the
/// holder holds the owner, and the access of the owner's procedure between its OnRun and "ClearAllVariables".
#[derive(Clone, Copy, PartialEq, Eq)]
struct Path {
    holder: Owner,
    entry: Entry,
    held: Held,
    check: Access,
}

/// How the holder is entered and reaches the statement `Owner.Run(...)` ([`Session::holder_entry`]).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Entry {
    /// A public procedure of the holder, called from a test codeunit, runs the owner.
    Public,
    /// A public procedure of the holder runs the owner, called from the holder's OnRun, which a test codeunit entered by the
    /// statement form of `Run` of a codeunit variable.
    PublicInOwnRun,
    /// A public procedure of the holder, called from a test codeunit, calls one local procedure that runs the owner.
    PublicThenLocal,
    /// The holder's OnRun, entered from a test codeunit by the statement form of `Run` of a codeunit variable, calls a chain
    /// of this many local procedures; the last runs the owner.
    OwnRunThenLocals(u8),
}

/// Which variable of the routine that runs the owner holds the owner ([`RunCaller::held`]).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Held {
    /// A global of the holder.
    HolderGlobal,
    /// A declared local of the routine that calls `Run` (no parameter, no return value).
    RunningLocal,
}

/// The access a routine declares.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Access {
    Public,
    Local,
    Internal,
}

fn access(r: &al_ir::Routine) -> Option<Access> {
    match (r.local, r.internal) {
        (false, false) => Some(Access::Public),
        (true, false) => Some(Access::Local),
        (false, true) => Some(Access::Internal),
        (true, true) => None,
    }
}

/// The call paths corpus PR 1650 measures. Each serves the never-called register the same way (the same values in every
/// cell, on a local BC of the three supported majors). The base shape, ClearSharedCodeunitNeverCalledTree.al: the plain
/// "CNC Holder" (:349-419) holds the register (:352) and the owner (:353) in globals and runs the owner by
/// `SalesPost.Run(TempHeader)` (:364) from its public `Post` (:356), called from the test codeunit; "CNC Owner"'s OnRun
/// (:294-300) reaches its ClearAll through the local "RunWithCheck" (:302) and the local "ClearAllVariables" (:313-316).
/// Each other row changes the facts of ClearSharedCodeunitNeverCalledShapes.al (at 03b9f669) its comment names.
const MEASURED: [Path; 7] = [
    // The base shape (NeverCalled_OwnerClearAll_EveryGlobal :470 to NeverCalled_OwnerRunsInOnRun_ThenHolderRuns :623).
    Path {
        holder: Owner::Plain,
        entry: Entry::Public,
        held: Held::HolderGlobal,
        check: Access::Local,
    },
    // "CNC Holder TableNo" (:87-155) has a TableNo, its public Post called from the test (TableNo_NeverCalled_* :478 :519).
    Path {
        holder: Owner::TableNo,
        entry: Entry::Public,
        held: Held::HolderGlobal,
        check: Access::Local,
    },
    // The same holder entered by `H.Run(Row)` (:552), its OnRun (:96-99) calling Post (TableNoRun_NeverCalled_* :543 :590).
    Path {
        holder: Owner::TableNo,
        entry: Entry::PublicInOwnRun,
        held: Held::HolderGlobal,
        check: Access::Local,
    },
    // "CNC Holder LocalProc" (:157-217): public Post calls the local PostSourceDocument, which runs the owner
    // (LocalProc_NeverCalled_* :617 :658).
    Path {
        holder: Owner::Plain,
        entry: Entry::PublicThenLocal,
        held: Held::HolderGlobal,
        check: Access::Local,
    },
    // "CNC Holder LocalOwner" (:219-276): the owner a local of Post (:228), which runs it (LocalOwner_NeverCalled_* :682
    // :723).
    Path {
        holder: Owner::Plain,
        entry: Entry::Public,
        held: Held::RunningLocal,
        check: Access::Local,
    },
    // "CNC Holder Internal" (:278-333) runs "CNC Owner Internal" (:30-85), whose OnRun calls the internal RunWithCheck
    // (:43), which calls the local ClearAllVariables (:57-60) (Internal_NeverCalled_* :747 :788).
    Path {
        holder: Owner::Plain,
        entry: Entry::Public,
        held: Held::HolderGlobal,
        check: Access::Internal,
    },
    // All four: "CNC Holder Activity" (:335-427, TableNo) entered by `H.Run(Row)`, OnRun -> local Code -> local
    // PostSourceDoc -> local PostSourceDocument, whose local SalesPost (:368) is "CNC Owner Internal"
    // (Activity_NeverCalled_* :812 :859 :884).
    Path {
        holder: Owner::TableNo,
        entry: Entry::OwnRunThenLocals(3),
        held: Held::RunningLocal,
        check: Access::Internal,
    },
];

impl Session {
    /// `Codeunit.Run` of codeunit instance `rc` from frame `f` (`Codeunit.Run#1`), noted in `run_callers` while it runs.
    pub(crate) fn codeunit_run_noting_caller(
        &mut self,
        rc: Rc<RefCell<Instance>>,
        values: &[Value],
        guarded: bool,
        f: &Frame,
    ) -> Result<Value, Signal> {
        let held = self.held_by(f, &rc);
        self.run_callers.push(RunCaller {
            at: self.call_stack.len(),
            caller: Rc::downgrade(&f.instance),
            run: Rc::downgrade(&rc),
            guarded,
            held,
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

    /// Which one variable of frame `f` declared as `rc`'s codeunit holds `rc`: exactly one global of its instance and no
    /// declared local, or exactly one declared local (not a parameter, not the return value) and no global.
    fn held_by(&self, f: &Frame, rc: &Rc<RefCell<Instance>>) -> Option<Held> {
        let declared = RtType::Codeunit(rc.borrow().object);
        let holds = |v: &Value, ty: &RtType| *ty == declared && matches!(v, Value::Codeunit(x) if Rc::ptr_eq(x, rc));
        let program = &self.engine.program;
        let instance = f.instance.borrow();
        let globals = instance
            .globals
            .iter()
            .zip(&program.objects[instance.object as usize].globals)
            .filter(|(v, g)| holds(v, &g.ty))
            .count();
        let r = &program.objects[f.object].routines[f.routine];
        let locals = f
            .locals
            .iter()
            .zip(&r.locals)
            .enumerate()
            .filter(|&(i, (v, s))| i >= r.params && r.ret_local != Some(i) && holds(v, &s.ty))
            .count();
        match (globals, locals) {
            (1, 0) => Some(Held::HolderGlobal),
            (0, 1) => Some(Held::RunningLocal),
            _ => None,
        }
    }

    /// Whether the `ClearAll()` in `owner` that meets register instance `rc` runs in a call path corpus PR 1650 measures
    /// ([`MEASURED`]): in the owner's OnRun trigger through a procedure ("RunWithCheck") and the local "ClearAllVariables",
    /// with no event frame; the OnRun entered by the statement form of `Run` of a codeunit variable, its result not used,
    /// from a holder whose kind, entry and variable holding the owner are a measured row; and a global of that holder
    /// declared as the register's codeunit holds `rc`.
    pub(super) fn uncalled_tree_path(&self, owner: usize, rc: &Rc<RefCell<Instance>>) -> bool {
        let Some(run) = self.run_callers.last() else {
            return false;
        };
        let (Some(holder), Some(held)) = (run.caller.upgrade(), run.held) else {
            return false;
        };
        let holder_object = holder.borrow().object as usize;
        let n = self.call_stack.len();
        let check = match (run.at + 3 == n).then(|| self.frame_access(n - 2, owner, false)) {
            Some(Some(check @ (Access::Local | Access::Internal))) => check,
            _ => return false,
        };
        let path = Path {
            holder: match self.owner_kind(holder_object) {
                Some(kind) => kind,
                None => return false,
            },
            entry: match run
                .at
                .checked_sub(1)
                .and_then(|below| self.holder_entry(below, holder_object, &holder))
            {
                Some(entry) => entry,
                None => return false,
            },
            held,
            check,
        };
        let globals = &self.engine.program.objects[holder_object].globals;
        let holds_register = holder.borrow().globals.iter().zip(globals).any(|(v, g)| {
            g.ty == RtType::Codeunit(rc.borrow().object) && matches!(v, Value::Codeunit(x) if Rc::ptr_eq(x, rc))
        });
        !run.guarded
            && MEASURED.contains(&path)
            && self.frame_access(n - 1, owner, false) == Some(Access::Local)
            && self.is_on_run(run.at, owner)
            && holds_register
    }

    /// How the holder (object `holder_object`, instance `holder`) reached the `Run` its frame `below` makes ([`Entry`]).
    fn holder_entry(&self, below: usize, holder_object: usize, holder: &Rc<RefCell<Instance>>) -> Option<Entry> {
        let mut first = below;
        while self.frame_access(first, holder_object, false) == Some(Access::Local) {
            first = first.checked_sub(1)?;
        }
        let locals = u8::try_from(below - first).ok()?;
        let caller = first.checked_sub(1)?;
        if self.frame_access(first, holder_object, false) == Some(Access::Public) {
            let from_test = self.is_test_frame(caller);
            let in_own_run = self.own_run_from_test(caller, holder_object, holder);
            return match (locals, from_test, in_own_run) {
                (0, true, false) => Some(Entry::Public),
                (0, false, true) => Some(Entry::PublicInOwnRun),
                (1, true, false) => Some(Entry::PublicThenLocal),
                _ => None,
            };
        }
        self.own_run_from_test(first, holder_object, holder)
            .then_some(Entry::OwnRunThenLocals(locals))
    }

    /// Whether frame `i` is the OnRun of `holder` (object `holder_object`) that the codeunit variable run next below the
    /// current one entered by the statement form of `Run`, its result not used, called from a test codeunit.
    fn own_run_from_test(&self, i: usize, holder_object: usize, holder: &Rc<RefCell<Instance>>) -> bool {
        let entered = self
            .run_callers
            .iter()
            .rev()
            .nth(1)
            .is_some_and(|r| r.at == i && !r.guarded && r.run.upgrade().is_some_and(|x| Rc::ptr_eq(&x, holder)));
        entered && self.is_on_run(i, holder_object) && i.checked_sub(1).is_some_and(|c| self.is_test_frame(c))
    }

    /// The access of the routine of frame `i` when it is a routine of `object`, a trigger exactly when `trigger`, not run
    /// as an event.
    fn frame_access(&self, i: usize, object: usize, trigger: bool) -> Option<Access> {
        let frame = self
            .call_stack
            .get(i)
            .filter(|f| self.frames[i] == object && !f.event)?;
        let r = &self.engine.program.objects[object].routines[frame.routine];
        (r.is_trigger == trigger).then(|| access(r))?
    }

    /// Whether frame `i` runs the OnRun trigger of `object`, not as an event.
    fn is_on_run(&self, i: usize, object: usize) -> bool {
        self.frame_access(i, object, true).is_some()
            && self.engine.program.objects[object].routines[self.call_stack[i].routine]
                .name
                .eq_ignore_ascii_case("OnRun")
    }

    /// Whether frame `i` runs a routine of a codeunit that states `Subtype = Test` ([`Owner::Test`]), not as an event.
    fn is_test_frame(&self, i: usize) -> bool {
        self.call_stack.get(i).is_some_and(|frame| !frame.event) && self.owner_kind(self.frames[i]) == Some(Owner::Test)
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
