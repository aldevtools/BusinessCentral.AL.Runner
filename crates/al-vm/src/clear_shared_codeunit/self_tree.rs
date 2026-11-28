//! The store cell of `ClearAll()` over a codeunit instance that holds itself, or that codeunits nested in it hold back
//! ([`super::Store::SelfTree`], assumption clear-all-self-holding-tree, open corpus PR 1672,
//! tests/al-language/codeunits/clear-instance/ClearSharedCodeunitSelfHold.al and ClearSharedCodeunitSelfHoldNested.al):
//! the shape of a sale codeunit whose `Init` stores the caller's variable in a global of its own type, shared with a
//! SingleInstance session codeunit. BC resets every instance of the tree in place, each once, and replaces no codeunit
//! reference: the self link, the back links and every holder stay on the same instances (`Session::reset_once` does
//! this). Which codeunits form the tree is [`Session::self_tree`]'s, which holders the probe measures
//! [`Session::self_tree_as_measured`]'s; the (owner, holding) cell is [`super::measured`]'s.
//! A plain holder of the store that the owner also holds is reset in place with it ([`Session::self_tree_holder`]).
//! Entry points: `Session::self_tree` (from `Session::store_kind_in`), `Session::self_tree_as_measured` and
//! `Session::self_tree_runs` (from `Session::measured_cell`), `Session::self_tree_holder` (from
//! `Session::clear_all_in_place`).

use super::*;

/// The deepest level a codeunit global of the tree sits at, the store counted again where a back link reaches it: "CSH
/// Deep L3" holds the store at level 4 (ClearSharedCodeunitSelfHoldNested.al:713-717, DeepSess_ClearAll :874) and "CSH
/// Chain C4" sits at level 4 (:1073, ChainSess_ClearAll :1146).
const MAX_DEPTH: u8 = 4;

/// What [`Session::self_tree_as_measured`] reads of one instance of the tree.
struct Reached {
    at: *const RefCell<Instance>,
    object: u32,
    /// `Rc::strong_count` when the walk first met it.
    holders: usize,
    /// The codeunit globals of instances of the tree that hold it.
    links: usize,
}

impl Session {
    /// Whether codeunit `store` is the store of a tree corpus PR 1672 measures and no other store shape names: a codeunit
    /// that is not SingleInstance, has no TableNo and lists no interface, whose every global is an Integer or a codeunit,
    /// each codeunit global [`MAX_DEPTH`] levels deep at most and one of: the store itself (the self link "Myself",
    /// ClearSharedCodeunitSelfHold.al:36-38; the back links of "CSH Line" :77, "CSH Deep L1" to "L3" and "CSH Deep Pay",
    /// ClearSharedCodeunitSelfHoldNested.al:635-748), a codeunit of the same kind met nowhere else in the tree ("CSH Deep
    /// Front" :772-775, the chain "CSH Chain C1" to "C4" :1014-1076), or, one level below the store, a SingleInstance
    /// codeunit with no TableNo whose globals are Integers ("CSH Deep Setup" :788-793). No interface: no Interface, List
    /// or Dictionary of Interface can hold an instance of the tree, a holder no cell measures.
    pub(super) fn self_tree(&self, store: usize) -> bool {
        self.self_tree_node(store) && self.self_tree_level(store, store, 0, &mut vec![store])
    }

    /// Whether codeunit `object` may be a node of a [`Session::self_tree`]: not SingleInstance, no TableNo, no interface.
    fn self_tree_node(&self, object: usize) -> bool {
        self.plain_codeunit(object)
            && self.engine.program.objects[object].rec_slot.is_none()
            && self.implements_no_interface(object)
    }

    /// Whether every global of codeunit `object`, `depth` levels below `store`, is one [`Session::self_tree`] admits;
    /// `seen` holds the codeunits already met.
    fn self_tree_level(&self, store: usize, object: usize, depth: u8, seen: &mut Vec<usize>) -> bool {
        let objects = &self.engine.program.objects;
        objects[object].globals.iter().all(|g| {
            !g.label
                && match g.ty {
                    RtType::Integer => true,
                    RtType::Codeunit(c) => {
                        let (c, d) = (c as usize, depth + 1);
                        d <= MAX_DEPTH
                            && (c == store
                                || d == 1 && self.self_tree_setup(c)
                                || self.self_tree_node(c) && !seen.contains(&c) && {
                                    seen.push(c);
                                    self.self_tree_level(store, c, d, seen)
                                })
                    }
                    _ => false,
                }
        })
    }

    /// Whether codeunit `object` is SingleInstance, has no TableNo and declares only Integers ("CSH Deep Setup",
    /// ClearSharedCodeunitSelfHoldNested.al:788-793): it keeps its state (DeepSess_ClearAll :874, 'setup' 6).
    fn self_tree_setup(&self, object: usize) -> bool {
        let o = &self.engine.program.objects[object];
        o.kind == al_ir::ObjKind::Codeunit
            && self.engine.object_facts[object].single_instance
            && o.rec_slot.is_none()
            && o.globals.iter().all(|g| !g.label && matches!(g.ty, RtType::Integer))
    }

    /// Whether the holders of store instance `rc` of a [`Session::self_tree`], met by `ClearAll()` in its owner, are those
    /// corpus PR 1672 measures. Besides the owner's global, the copy `ClearAll` takes of it and, when `running`, the frame of
    /// the store's own routine that runs the `ClearAll`, the instance is held by codeunit globals of the tree (the self link
    /// and the back links) and by one other variable at most: a SingleInstance codeunit's global (SelfSess_ClearAll
    /// ClearSharedCodeunitSelfHold.al:202), a plain codeunit's global (SelfHolderLocal_ClearAll :459,
    /// SelfHolderGlobal_ClearAll :488), the test codeunit's global when a holder or the store runs the `ClearAll`
    /// (SelfSess_HolderClearAll :405, SelfSess_StoreClearAllSelf :432), or none (SelfOnly_ClearAll :177). Every other
    /// instance of the tree that is not SingleInstance is held by codeunit globals of the tree only, except one instance of
    /// a codeunit other than the store, which one other variable may hold too (a test local on "CSH Line",
    /// ClearSharedCodeunitSelfHoldNested.al LineSess_LineHandle_ClearAll :204, on "CSH Deep L3", DeepSess_L3Handle_ClearAll
    /// :896). An instance of the store's codeunit other than `rc` is one a read made and that no call linked
    /// (SelfSess_NoInit_ClearAll :259, LineSess_NoInit_ClearAll :232). The kind of the one other variable is not read,
    /// except that a Variant global of a SingleInstance codeunit refuses ([`Session::variant_holds`]).
    pub(super) fn self_tree_as_measured(&self, rc: &Rc<RefCell<Instance>>, running: bool) -> bool {
        let mut reached = vec![Reached {
            at: Rc::as_ptr(rc),
            object: rc.borrow().object,
            holders: Rc::strong_count(rc),
            links: 0,
        }];
        self.self_tree_walk(rc, &mut reached);
        let store = reached[0].object;
        let outside = |r: &Reached| r.holders.checked_sub(r.links);
        let root = outside(&reached[0]).and_then(|n| n.checked_sub(2 + usize::from(running)));
        let mut nested = 0;
        for r in &reached[1..] {
            match (r.object == store, outside(r)) {
                (true, Some(0)) | (false, Some(0)) => {}
                (false, Some(1)) => nested += 1,
                _ => return false,
            }
        }
        root.is_some_and(|n| n <= 1) && nested <= 1 && !self.variant_holds(&reached)
    }

    /// Whether a Variant global of a SingleInstance codeunit holds an instance of `reached`: a holder kind no cell measures.
    /// A Variant elsewhere is not seen here (the entry says so).
    fn variant_holds(&self, reached: &[Reached]) -> bool {
        let objects = &self.engine.program.objects;
        self.singletons.values().any(|si| {
            let si = si.borrow();
            (si.globals.iter().zip(&objects[si.object as usize].globals)).any(|(v, slot)| {
                matches!(slot.ty, RtType::Variant)
                    && matches!(v, Value::Codeunit(m) if reached.iter().any(|r| r.at == Rc::as_ptr(m)))
            })
        })
    }

    /// Notes in `reached` each instance that is not SingleInstance a codeunit global of instance `rc` holds, with one more
    /// link, and walks each instance a call made once.
    fn self_tree_walk(&self, rc: &Rc<RefCell<Instance>>, reached: &mut Vec<Reached>) {
        let inst = rc.borrow();
        for g in &inst.globals {
            let Value::Codeunit(m) = g else { continue };
            if !self.plain_codeunit(m.borrow().object as usize) {
                continue;
            }
            if let Some(r) = reached.iter_mut().find(|r| r.at == Rc::as_ptr(m)) {
                r.links += 1;
                continue;
            }
            reached.push(Reached {
                at: Rc::as_ptr(m),
                object: m.borrow().object,
                holders: Rc::strong_count(m),
                links: 1,
            });
            if m.borrow().initialized {
                self.self_tree_walk(m, reached);
            }
        }
    }

    /// Whether `rc`, the instance of a global of `owner` that nothing else holds, is a plain holder of a store instance that
    /// `ClearAll()` there resets in place as [`super::Store::SelfTree`]: BC resets the holder in place too, and it keeps its
    /// link to the store (SelfHolderGlobal_ClearAll, ClearSharedCodeunitSelfHold.al:488, 'S.SetValue(5) hg' v=5). Measured
    /// for a test codeunit owner, a holder that is not SingleInstance, has no TableNo, lists no interface and declares one
    /// global, of the store's codeunit ("CSH Self Holder" :118-121), a call made it, and that global holds the instance of
    /// another global of the owner declared as the store's codeunit. Any other holder keeps main's fresh instance.
    pub(super) fn self_tree_holder(&self, owner: usize, rc: &Rc<RefCell<Instance>>, globals: &[Value]) -> bool {
        let objects = &self.engine.program.objects;
        let object = rc.borrow().object as usize;
        let [g] = objects[object].globals.as_slice() else {
            return false;
        };
        let RtType::Codeunit(c) = g.ty else { return false };
        if g.label
            || !self.self_tree_node(object)
            || self.owner_kind(owner) != Some(Owner::Test)
            || self.store_kind_in(owner, c as usize) != Some(Store::SelfTree)
        {
            return false;
        }
        let inst = rc.borrow();
        let Some(Value::Codeunit(held)) = inst.globals.first() else {
            return false;
        };
        inst.initialized
            && (globals.iter().zip(&objects[owner].globals)).any(|(v, slot)| {
                matches!(slot.ty, RtType::Codeunit(s) if s == c)
                    && matches!(v, Value::Codeunit(o) if Rc::ptr_eq(o, held))
            })
    }

    /// Whether a routine of a codeunit of store instance `v` runs, other than the store's own routine on the top frame
    /// when `running`: the `ClearAll` inside the store (SelfSess_StoreClearAllSelf, ClearSharedCodeunitSelfHold.al:432).
    pub(super) fn self_tree_runs(&self, v: &Value, running: bool) -> bool {
        let below = self.frames.len() - usize::from(running && !self.frames.is_empty());
        self.store_runs_in(v, &self.frames[..below], &mut Vec::new())
    }
}
