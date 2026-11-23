//! `Clear` and `ClearAll()` of a codeunit variable whose instance an interface, a second codeunit variable or a parameter
//! shares (assumption clear-shared-codeunit, open corpus PR 1560, tests/al-language/codeunits/clear-instance/
//! ClearSharedCodeunit.al). `Clear` gives such a variable nothing used yet a fresh instance, and `ClearAll()` resets such an
//! instance in place in the measured shape; the shapes the probe does not measure refuse. An instance nothing else holds
//! keeps main's answer. Entry points: `Session::clear_shared_codeunit` (from the `Clear` site, `exec/run.rs`) and
//! `Session::clear_all_in_place` (from `clear_all`).

use super::*;
use crate::var_option::ArrayOf;
use std::rc::Weak;

impl Session {
    /// Whether object `object` is a codeunit that is not SingleInstance: a variable of it holds an instance of its own,
    /// which `Clear` replaces and an assignment shares (ClearSharedCodeunit.al codeunit 75580 "CLX Store"). A SingleInstance
    /// codeunit variable reads the one instance whatever is cleared (ClearSharedCodeunitSI.al).
    pub(crate) fn plain_codeunit(&self, object: usize) -> bool {
        self.engine.program.objects[object].kind == al_ir::ObjKind::Codeunit
            && !self.engine.object_facts[object].single_instance
    }

    /// The globals of `owner` (not in `keep`) that `ClearAll()` in `owner` resets in place: each holds an instance of a
    /// codeunit in the measured shape ([`Session::reset_in_place`]) that another holder shares; its globals go back to their
    /// declared values and the variable keeps the instance, so the holder sees the reset and later writes through either
    /// side (ClearSharedCodeunit.al ClearAll_GlobalUnusedIface, ClearAll_GlobalUsedAlias_AliasWroteBefore,
    /// ClearAll_GlobalUsedValueParam, X unused and used once alike). An instance nothing else holds keeps main's answer (the
    /// variable gets a fresh one, which holds the same). A shared instance of any other shape refuses: main's fresh instance
    /// would detach it, which BC does not do in the measured shape.
    ///
    /// `ClearAll` takes a copy of the globals, so the instance of a global nothing shares has two holders; an element of an
    /// array of codeunits has one, the array.
    pub(crate) fn clear_all_in_place(
        &mut self,
        owner: usize,
        globals: &[Value],
        keep: &[usize],
    ) -> Result<Vec<usize>, Signal> {
        let mut kept = Vec::new();
        for (i, v) in globals.iter().enumerate() {
            if keep.contains(&i) {
                continue;
            }
            match v {
                Value::Codeunit(rc) if self.plain_codeunit(rc.borrow().object as usize) && Rc::strong_count(rc) > 2 => {
                    if !(self.clears_in_place(owner) && self.reset_in_place(rc)) {
                        return Err(Signal::unsupported(
                            "ClearAll of a global codeunit variable whose instance another variable shares, in a shape \
                             the corpus does not pin",
                        ));
                    }
                    al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::ClearSharedCodeunit);
                    if rc.borrow().initialized {
                        let engine = self.engine.clone();
                        let object = rc.borrow().object as usize;
                        let fresh = self.slot_defaults(&engine.program.objects[object].globals);
                        rc.borrow_mut().globals = fresh;
                    }
                    kept.push(i);
                }
                Value::Array(items) if items.borrow().iter().any(|e| self.shared_element(e)) => {
                    return Err(Signal::unsupported(
                        "ClearAll of a global array whose codeunit element another variable shares (not pinned)",
                    ));
                }
                _ => {}
            }
        }
        Ok(kept)
    }

    /// Whether array element `e` holds an instance of a codeunit that is not SingleInstance that another holder shares (the
    /// array is its one own holder).
    fn shared_element(&self, e: &Value) -> bool {
        matches!(e, Value::Codeunit(rc) if self.plain_codeunit(rc.borrow().object as usize) && Rc::strong_count(rc) > 1)
    }

    /// Whether `ClearAll()` in `owner` may reset a global codeunit's instance in place: `owner` is a codeunit that is not
    /// SingleInstance, states no Subtype and has no TableNo, as "CLX Holder" (ClearSharedCodeunit.al codeunit 75581).
    fn clears_in_place(&self, owner: usize) -> bool {
        let o = &self.engine.program.objects[owner];
        self.plain_codeunit(owner)
            && o.properties.iter().all(|(k, _)| !k.eq_ignore_ascii_case("Subtype"))
            && o.rec_slot.is_none()
    }

    /// Whether the instance `rc`, of a codeunit that is not SingleInstance (the caller checks), is of the measured shape: of a
    /// codeunit that has no TableNo and holds
    /// exactly one global, an Integer, as "CLX Store" (ClearSharedCodeunit.al codeunit 75580, `N: Integer`); not bound by
    /// BindSubscription (`clear_all_bound_globals` owns those) and with no routine of its codeunit running.
    fn reset_in_place(&self, rc: &Rc<RefCell<Instance>>) -> bool {
        let object = rc.borrow().object as usize;
        let o = &self.engine.program.objects[object];
        o.rec_slot.is_none()
            && matches!(o.globals.as_slice(), [g] if matches!(g.ty, RtType::Integer))
            && !self.holds_bound_codeunit(&Value::Codeunit(rc.clone()))
            && !self.frames.contains(&object)
    }

    /// `Clear` of place `p`, whose value `v` the native clears next (al-builtins `native/values.rs`, which keeps an unused
    /// codeunit variable as it is). For a Codeunit variable that holds an instance nothing used yet, of a codeunit that is
    /// not SingleInstance, that another holder shares, `v` becomes a fresh instance first: the variable gets it and the
    /// holder keeps the old one (ClearSharedCodeunit.al Clear_LocalUnusedIface, Clear_GlobalUnusedAlias,
    /// Clear_LocalUnusedAliasClearedThroughVarParam, Clear_GlobalUnusedValueParam: X reads 0, the holder 5). Measured for a
    /// declared local, a global and a var parameter ([`ClearPlace::Measured`]); any other place refuses, a by-value
    /// parameter among them (the probe never clears one itself, ClearThenSet clears the caller's variable). So does an array
    /// of codeunits whose unused element another holder shares. An instance nothing else holds keeps main's answer with no
    /// mark: a fresh unused instance reads the same. An Interface or Variant place keeps its own path (`exec/run.rs`).
    pub(crate) fn clear_shared_codeunit(&mut self, p: &ir::Place, v: &mut Value, f: &Frame) -> Result<(), Signal> {
        match (self.place_type(p, f), &*v) {
            (RtType::Codeunit(_), Value::Codeunit(rc))
                if !rc.borrow().initialized && self.plain_codeunit(rc.borrow().object as usize) =>
            {
                let place = self.clear_place(p, f);
                let own = place.own_copies(rc, f);
                if Rc::strong_count(rc) <= own.unwrap_or(2) {
                    return Ok(());
                }
                if place == ClearPlace::Other || own.is_none() {
                    return Err(Signal::unsupported(
                        "Clear of a codeunit variable that nothing used yet and another variable shares, in a place \
                         the corpus does not pin (a by-value parameter, an array element, a var parameter whose \
                         caller's global was assigned since)",
                    ));
                }
                al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::ClearSharedCodeunit);
                let object = rc.borrow().object;
                *v = Value::default_for(&RtType::Codeunit(object));
                Ok(())
            }
            (RtType::Array(_, el), Value::Array(items))
                if matches!(*el, RtType::Codeunit(_))
                    && items.borrow().iter().any(|e| {
                        self.shared_element(e) && matches!(e, Value::Codeunit(rc) if !rc.borrow().initialized)
                    }) =>
            {
                Err(Signal::unsupported(
                    "Clear of an array whose unused codeunit element another variable shares (not pinned)",
                ))
            }
            _ => Ok(()),
        }
    }

    /// The kind of place `p` a `Clear` names, in the routine `f` runs.
    fn clear_place(&self, p: &ir::Place, f: &Frame) -> ClearPlace {
        match (p, self.array_of(p, f)) {
            (ir::Place::Global(_), _) | (_, ArrayOf::Declared) => ClearPlace::Measured,
            (ir::Place::Local(slot), ArrayOf::VarParameter) => ClearPlace::VarParameter(*slot),
            _ => ClearPlace::Other,
        }
    }
}

/// The kind of place a `Clear` of a codeunit variable names ([`Session::clear_shared_codeunit`]).
#[derive(Clone, Copy, PartialEq, Eq)]
enum ClearPlace {
    /// A declared local or a global (ClearSharedCodeunit.al Clear_LocalUnusedIface, Clear_GlobalUnusedIface).
    Measured,
    /// The var parameter in this slot (Clear_LocalUnusedAliasClearedThroughVarParam, ClearVar).
    VarParameter(usize),
    /// A by-value parameter, an element, the named return value or any other place.
    Other,
}

impl ClearPlace {
    /// How many holders of `rc`, the instance the place holds in frame `f`, are copies of the variable itself: the variable
    /// and the copy `Clear` takes of it (the VM evaluates its argument); for a var parameter, instead of the variable, the
    /// slots of the var parameter chain that still hold `rc` ([`VarCodeunit`]). A var parameter whose caller is not an AL
    /// call (an event subscriber, a routine the platform calls) counts its slot and one caller variable. `None`: the chain
    /// starts at a global that was assigned another instance since, which the parameter does not follow (not measured).
    fn own_copies(self, rc: &Rc<RefCell<Instance>>, f: &Frame) -> Option<usize> {
        match self {
            ClearPlace::VarParameter(slot) => match f.var_codeunits.get(slot).and_then(Option::as_ref) {
                Some(got) => Some(got.copies(rc)? + 1),
                None => Some(3),
            },
            _ => Some(2),
        }
    }
}

/// What a `var` Codeunit parameter got from its caller (`Frame::var_codeunits`): the instance handed in and how many slots of
/// the var parameter chain held it then (the caller's variable, each var parameter that handed it on, this slot). A frame
/// does not run while a routine it called runs, so its slots keep what they held; a global at the start of the chain can
/// be assigned by its name meanwhile, and is checked again.
#[derive(Clone)]
pub(crate) struct VarCodeunit {
    handed: Weak<RefCell<Instance>>,
    chain: usize,
    origin: Option<(Weak<RefCell<Instance>>, usize)>,
}

impl VarCodeunit {
    /// The slots of the chain that hold `rc` now: none but this one when the parameter was assigned another instance since
    /// (its slot holds `rc` like a local variable). `None` when the global the chain starts at holds another instance now.
    fn copies(&self, rc: &Rc<RefCell<Instance>>) -> Option<usize> {
        if !std::ptr::eq(self.handed.as_ptr(), Rc::as_ptr(rc)) {
            return Some(1);
        }
        let origin_holds = self.origin.as_ref().is_none_or(|(owner, g)| {
            owner
                .upgrade()
                .is_some_and(|o| matches!(o.borrow().globals.get(*g), Some(Value::Codeunit(x)) if Rc::ptr_eq(x, rc)))
        });
        origin_holds.then_some(self.chain)
    }
}

impl Session {
    /// For each `var` Codeunit parameter of routine `routine` of `object` that a call from frame `f` with `args` hands
    /// `values`: what it got ([`VarCodeunit`]). A variable of `f` starts a chain of two (it and the parameter); a var
    /// parameter of `f` that still holds what it got hands it on one longer, one assigned another instance since starts a
    /// chain of two; one whose chain starts at a global assigned since hands that on. Any other argument, and a var
    /// parameter of `f` with no record, gives none.
    pub(crate) fn var_codeunits_of(
        &self,
        object: usize,
        routine: usize,
        args: &[ir::Arg],
        values: &[Value],
        f: &Frame,
    ) -> Vec<Option<VarCodeunit>> {
        let r = &self.engine.program.objects[object].routines[routine];
        let caller = &self.engine.program.objects[f.object].routines[f.routine];
        let make = |rc: &Rc<RefCell<Instance>>, chain, origin| {
            Some(VarCodeunit {
                handed: Rc::downgrade(rc),
                chain,
                origin,
            })
        };
        (0..r.params.min(args.len()))
            .map(|i| {
                let Some(Value::Codeunit(rc)) = values.get(i) else {
                    return None;
                };
                if r.by_var.get(i) != Some(&true) || !matches!(r.locals[i].ty, RtType::Codeunit(_)) {
                    return None;
                }
                match &args[i] {
                    ir::Arg::Ref(ir::Place::Local(j)) if *j < caller.params && caller.by_var.get(*j) == Some(&true) => {
                        let got = f.var_codeunits.get(*j).and_then(Option::as_ref)?;
                        match got.copies(rc) {
                            Some(1) => make(rc, 2, None),
                            n => make(rc, n.unwrap_or(got.chain) + 1, got.origin.clone()),
                        }
                    }
                    ir::Arg::Ref(ir::Place::Local(_)) => make(rc, 2, None),
                    ir::Arg::Ref(ir::Place::Global(g)) => make(rc, 2, Some((Rc::downgrade(&f.instance), *g))),
                    _ => None,
                }
            })
            .collect()
    }

    /// The records `finish_call` left for the frame of a routine with `var` Codeunit parameters (`var_codeunits_of`); empty
    /// for any other routine.
    pub(crate) fn take_var_codeunits(&mut self, wanted: bool) -> Vec<Option<VarCodeunit>> {
        if wanted {
            std::mem::take(&mut self.events.next_var_codeunits)
        } else {
            Vec::new()
        }
    }
}
