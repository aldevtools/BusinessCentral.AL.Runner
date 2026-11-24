//! `Clear` and `ClearAll()` of a codeunit variable whose instance an interface, a second codeunit variable or a parameter
//! shares (assumption clear-shared-codeunit, open corpus PR 1560, tests/al-language/codeunits/clear-instance/
//! ClearSharedCodeunit.al), and `ClearAll()` of a store codeunit with temporary records (open corpus PRs 1561 and 1564,
//! ClearSharedCodeunitTempRef.al, ClearSharedCodeunitSIOwner.al, ClearSharedCodeunitOwnerKinds.al,
//! ClearSharedCodeunitLabelStore.al). `Clear` gives such a variable nothing used yet a fresh instance, and `ClearAll()`
//! resets such an instance in place, keeping its temporary rows, in the measured (owner, store, holding) cells
//! (`measured`); the cells the probes do not measure refuse. An instance nothing else holds keeps main's answer unless its
//! temporary table holds rows. The store shapes are also what the login check counts at their declared value
//! (`Session::login_store`, read by `state_stamp.rs`). Entry points: `Session::clear_shared_codeunit` (from the `Clear`
//! site, `exec/run.rs`) and `Session::clear_all_in_place` (from `clear_all`).

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

    /// The globals of the owner of frame `f` (not in `keep`) that `ClearAll()` there resets in place. A global holding a store
    /// instance that another holder shares, or that nothing else holds while its temporary table holds rows: when the
    /// (owner, store, holding) cell is one the corpus measures ([`Session::measured_cell`]), its globals go back to their
    /// declared values, a temporary table keeping its rows, and the variable keeps the instance, so a sharer sees the
    /// reset and later writes through either side. Any other shared instance refuses: main's fresh instance would detach
    /// it. Any other with written rows refuses: main's fresh instance would lose them. A global array of "CLB Store"
    /// instances in a plain codeunit resets each element in place ([`Session::clear_all_store_array`]). Any other global
    /// nothing else holds keeps main's answer.
    ///
    /// `ClearAll` takes a copy of the globals, so the instance of a global nothing shares has two holders; an element of an
    /// array of codeunits has one, the array.
    pub(crate) fn clear_all_in_place(
        &mut self,
        f: &Frame,
        globals: &[Value],
        keep: &[usize],
    ) -> Result<Vec<usize>, Signal> {
        let owner = f.object;
        let mut kept = Vec::new();
        for (i, v) in globals.iter().enumerate() {
            if keep.contains(&i) {
                continue;
            }
            match v {
                Value::Codeunit(rc) if self.plain_codeunit(rc.borrow().object as usize) => {
                    let holding = if Rc::strong_count(rc) > 2 {
                        Holding::Shared
                    } else if self.holds_temp_rows(v, 0) {
                        Holding::Written
                    } else if self.wrapped_store_shared(rc) {
                        return Err(Signal::unsupported(
                            "ClearAll of a global codeunit variable whose inner store another variable shares, in a \
                             shape the corpus does not pin",
                        ));
                    } else {
                        continue;
                    };
                    if !self.measured_cell(owner, v, holding, f) {
                        return Err(match holding {
                            Holding::Shared => Signal::unsupported(
                                "ClearAll of a global codeunit variable whose instance another variable shares, in a \
                                 shape the corpus does not pin",
                            ),
                            Holding::Written => temp_rows_refusal(),
                        });
                    }
                    al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::ClearSharedCodeunit);
                    if rc.borrow().initialized {
                        self.reset_keeping_rows(rc);
                    }
                    kept.push(i);
                }
                Value::Array(items)
                    if (items.borrow().iter()).any(|e| {
                        self.shared_element(e) || matches!(e, Value::Codeunit(_)) && self.holds_temp_rows(e, 0)
                    }) =>
                {
                    self.clear_all_store_array(owner, i, items)?;
                    kept.push(i);
                }
                _ => {}
            }
        }
        Ok(kept)
    }

    /// `ClearAll()` in `owner` of its global array `items` (slot `slot`) of codeunits, an element of which another holder
    /// shares or holds written temporary rows. In a plain codeunit ([`Owner::Plain`]), for an array of one dimension of "CLB
    /// Store" ([`Store::TempRef`]) whose shared elements all hold rows, each element is reset in place: rows kept, the
    /// sharer sees the reset, an unused element stays as fresh (ClearSharedCodeunitOwnerKinds.al Arr2_Element1Written_NoAlias
    /// :770, Arr2_Element1AliasWroteBefore :790). Any other array refuses.
    fn clear_all_store_array(
        &mut self,
        owner: usize,
        slot: usize,
        items: &Rc<RefCell<Vec<Value>>>,
    ) -> Result<(), Signal> {
        let ty = &self.engine.program.objects[owner].globals[slot].ty;
        let measured = self.owner_kind(owner) == Some(Owner::Plain)
            && matches!(ty, RtType::Array(d, e) if d.len() == 1
                && matches!(**e, RtType::Codeunit(c) if self.store_kind(c as usize) == Some(Store::TempRef)))
            && (items.borrow().iter()).all(|e| !self.shared_element(e) || self.holds_temp_rows(e, 0))
            && (items.borrow().iter()).all(|e| !self.holds_bound_codeunit(e) && !self.store_runs(e));
        if !measured {
            let rows = (items.borrow().iter()).any(|e| matches!(e, Value::Codeunit(_)) && self.holds_temp_rows(e, 0));
            return Err(if rows {
                temp_rows_refusal()
            } else {
                Signal::unsupported(
                    "ClearAll of a global array whose codeunit element another variable shares (not pinned)",
                )
            });
        }
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::ClearSharedCodeunit);
        for e in items.borrow().iter() {
            if let Value::Codeunit(rc) = e
                && rc.borrow().initialized
            {
                self.reset_keeping_rows(rc);
            }
        }
        Ok(())
    }

    /// Checks the globals of `owner` (not in `keep`) that `ClearAll()` resets for an array of temporary records whose table
    /// was written: `keep_temp_table` (`interp.rs`) gives the fresh array the old table, so the rows stay, in a codeunit that
    /// is not SingleInstance with no TableNo that states no Subtype ("CLS Arr Store", ClearSharedCodeunitSIOwner.al
    /// ArrStore_ClearAll_InsideStore :1808, rows1=2 rows2=2 after ClearAll) and in a report of [`Owner::Report`]
    /// (ClearSharedCodeunitOwnerKinds.al ReportRows_ArrayElement1Written :814), through assumption clear-shared-codeunit.
    /// Any other owner refuses: main gave such an array fresh tables, and the probes measure only these.
    pub(crate) fn clear_all_temp_arrays(
        &mut self,
        owner: usize,
        globals: &[Value],
        keep: &[usize],
    ) -> Result<(), Signal> {
        let written = (globals.iter().enumerate())
            .filter(|(i, _)| !keep.contains(i))
            .any(|(_, v)| matches!(v, Value::Array(items) if items.borrow().iter().any(|e| matches!(e, Value::Record(_)) && self.holds_temp_rows(e, 0))));
        if !written {
            return Ok(());
        }
        if matches!(self.owner_kind(owner), Some(Owner::Plain | Owner::Report)) {
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::ClearSharedCodeunit);
            return Ok(());
        }
        Err(Signal::unsupported(
            "ClearAll of a global array of temporary records whose table holds rows, in an object the corpus does not pin",
        ))
    }

    /// Whether `v` is or holds, through the globals of an instance of a codeunit that is not SingleInstance and through
    /// arrays, a temporary record whose table was written (its change counter moved).
    fn holds_temp_rows(&self, v: &Value, depth: u8) -> bool {
        match v {
            _ if depth > 8 => true,
            Value::Record(r) => {
                let r = r.borrow();
                r.temporary
                    && (r.temp_id.as_ref())
                        .is_some_and(|t| self.temps.get(&t.id()).is_some_and(|s| s.change_counter() > 0))
            }
            Value::Codeunit(rc) if self.plain_codeunit(rc.borrow().object as usize) => {
                rc.borrow().globals.iter().any(|g| self.holds_temp_rows(g, depth + 1))
            }
            Value::Array(items) => items.borrow().iter().any(|e| self.holds_temp_rows(e, depth + 1)),
            _ => false,
        }
    }

    /// Whether array element `e` holds an instance of a codeunit that is not SingleInstance that another holder shares (the
    /// array is its one own holder).
    fn shared_element(&self, e: &Value) -> bool {
        matches!(e, Value::Codeunit(rc) if self.plain_codeunit(rc.borrow().object as usize) && Rc::strong_count(rc) > 1)
    }

    /// Whether instance `rc`, of a [`Store::Wrapper`], holds a store instance another holder shares (its one own holder is
    /// the wrapper's global): a wrapper never written whose store an alias shares is not measured (the probe's alias wrote).
    fn wrapped_store_shared(&self, rc: &Rc<RefCell<Instance>>) -> bool {
        self.store_kind(rc.borrow().object as usize) == Some(Store::Wrapper)
            && rc.borrow().globals.iter().any(|g| self.shared_element(g))
    }

    /// Whether a routine of the codeunit of instance `v`, or of a codeunit its globals hold, runs.
    fn store_runs(&self, v: &Value) -> bool {
        let Value::Codeunit(rc) = v else { return false };
        let inst = rc.borrow();
        self.frames.contains(&(inst.object as usize)) || inst.globals.iter().any(|g| self.store_runs(g))
    }

    /// The kind of object `owner` is, for the owners whose `ClearAll()` the corpus measures on a store global. `None` for
    /// any other object.
    fn owner_kind(&self, owner: usize) -> Option<Owner> {
        let o = &self.engine.program.objects[owner];
        if o.kind == al_ir::ObjKind::Report {
            return o.report.as_ref().is_some_and(measured_report).then_some(Owner::Report);
        }
        if o.kind != al_ir::ObjKind::Codeunit {
            return None;
        }
        let subtype = self.declared_subtype(owner);
        let is = |name: &str| subtype.is_some_and(|v| v.eq_ignore_ascii_case(name));
        match (
            subtype.is_none(),
            o.rec_slot.is_some(),
            self.engine.object_facts[owner].single_instance,
        ) {
            (true, false, false) => Some(Owner::Plain),
            (true, false, true) => Some(Owner::SingleInstance),
            (true, true, false) => Some(Owner::TableNo),
            (false, false, false) if is("Test") => Some(Owner::Test),
            (false, false, false) if is("Normal") => Some(Owner::Normal),
            _ => None,
        }
    }

    /// The store shape of codeunit `object`, by all its globals, for the stores the corpus measures: not SingleInstance, no
    /// TableNo, and globals of [`Store`]'s kinds. A global `Label 'text'` ([`ir::Slot::plain_label`]) counts only in the
    /// positions ClearSharedCodeunitLabelStore.al measures; any other Label or TextConst is a global of its own type, which
    /// no store has. `None` for any other.
    fn store_kind(&self, object: usize) -> Option<Store> {
        let o = &self.engine.program.objects[object];
        if !self.plain_codeunit(object) || o.rec_slot.is_some() {
            return None;
        }
        let labels: Vec<usize> = (o.globals.iter().enumerate())
            .filter(|(_, g)| g.plain_label)
            .map(|(i, _)| i)
            .collect();
        let kinds: Vec<&RtType> = (o.globals.iter()).filter(|g| !g.plain_label).map(|g| &g.ty).collect();
        let store = match kinds.as_slice() {
            [RtType::Integer] => Store::Integer,
            [r, f, i] if temp_ref_kinds(r, f, i) => Store::TempRef,
            [r, f, i, n] if temp_ref_kinds(r, f, i) && integer(n) => Store::TempRefPlus,
            [RtType::Codeunit(c), n] if integer(n) && self.store_kind(*c as usize) == Some(Store::TempRef) => {
                Store::Wrapper
            }
            [r, f, i] if array_with(r, 1, temp_record) && array_with(f, 1, record_ref) && array_with(i, 1, integer) => {
                Store::TempArray
            }
            _ => return None,
        };
        match (store, labels.as_slice()) {
            (_, []) => Some(store),
            // "CLL Store First" (:30), "CLL Store Mid" (:83), "CLL Store Last" (:136): the Label before, between the
            // temporary record and the RecordRef array, after.
            (Store::TempRef, [0 | 1 | 3]) => Some(Store::TempRefLabel),
            _ => None,
        }
    }

    /// Whether `ClearAll()` in `owner` resets store instance `v`, held as `holding` says, in place: the instance is not bound
    /// by BindSubscription (`clear_all_bound_globals` owns those), no routine of its codeunit runs, and (owner, store,
    /// holding) is a cell the corpus measures. A shared instance of [`Store::TempRefLabel`] counts only when no by-value
    /// parameter of the routine `f` runs holds it (the probe shares it through a second variable only).
    fn measured_cell(&self, owner: usize, v: &Value, holding: Holding, f: &Frame) -> bool {
        let Value::Codeunit(rc) = v else { return false };
        let (Some(o), Some(s)) = (self.owner_kind(owner), self.store_kind(rc.borrow().object as usize)) else {
            return false;
        };
        let cell = measured(o, s, holding)
            && !(s == Store::TempRefLabel && holding == Holding::Shared && value_parameter_holds(self, f, rc));
        cell && !self.holds_bound_codeunit(v) && !self.store_runs(v)
    }

    /// Resets instance `rc` in place, as `ClearAll()` in a measured owner does to a store global: its globals go back to
    /// their declared values, a temporary record keeping its table (`keep_temp_table`), so its rows stay
    /// (ClearSharedCodeunitSIOwner.al A3_Step3_ClearAllInOwner :447, rows=1 ref11=0 int1=0); the store a
    /// [`Store::Wrapper`] holds is reset in place the same way and kept, so its alias sees the reset
    /// (ClearSharedCodeunitOwnerKinds.al Deep_StoreAliasWroteBefore :723). The caller marks assumption clear-shared-codeunit.
    fn reset_keeping_rows(&mut self, rc: &Rc<RefCell<Instance>>) {
        let engine = self.engine.clone();
        let object = rc.borrow().object as usize;
        let mut fresh = self.slot_defaults(&engine.program.objects[object].globals);
        let old = rc.borrow().globals.clone();
        for (old, new) in old.iter().zip(fresh.iter_mut()) {
            match old {
                Value::Codeunit(inner) => {
                    if inner.borrow().initialized {
                        self.reset_keeping_rows(inner);
                    }
                    *new = old.clone();
                }
                _ => crate::interp::keep_temp_table(old, new),
            }
        }
        rc.borrow_mut().globals = fresh;
    }

    /// Whether `object` is a store the login check counts at its declared value when the SingleInstance codeunit holds it
    /// directly and a call made it while writing nothing (`state_stamp.rs` `at_declared`): "CLB Store" (open corpus PR 1561,
    /// ClearSharedCodeunitSIOwner.al A2_Step2_ReadX :385), the same with a Label (ClearSharedCodeunitLabelStore.al
    /// First_L2_Step2_ReadX :1344, Mid :1541, Last :1738) and "CLS Arr Store" (ClearSharedCodeunitOwnerKinds.al
    /// M2_Step2_ReadX :1205). The sizes of the arrays and the table are not read.
    pub(crate) fn login_store(&self, object: usize) -> bool {
        matches!(
            self.store_kind(object),
            Some(Store::TempRef | Store::TempRefLabel | Store::TempArray)
        )
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
        if !matches!(v, Value::Codeunit(_) | Value::Array(_)) {
            return Ok(());
        }
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

/// The refusal of a `ClearAll` that would give a fresh instance to a global codeunit variable whose instance holds
/// temporary rows, in a shape the probes do not measure.
fn temp_rows_refusal() -> Signal {
    Signal::unsupported(
        "ClearAll of a global codeunit variable whose instance holds temporary rows, in a shape the corpus does not pin",
    )
}

/// The (owner, store, holding) cells of `ClearAll()` in which the corpus measures a store global reset in place (rows kept,
/// RecordRefs closed, Integers 0, a sharer on the same instance). File names below are in
/// tests/al-language/codeunits/clear-instance/ of open corpus PRs 1560, 1561 and 1564.
fn measured(owner: Owner, store: Store, holding: Holding) -> bool {
    use {Holding::*, Owner::*, Store::*};
    match (owner, store, holding) {
        // ClearSharedCodeunit.al ClearAll_GlobalUnusedIface, ClearAll_GlobalUsedAlias_AliasWroteBefore,
        // ClearAll_GlobalUsedValueParam; ClearSharedCodeunitOwnerKinds.al SICLX_ClearAll_UnusedAlias :1472,
        // SICLX_ClearAll_AliasWroteBefore :1492, ClearAll_TestGlobalCLXUnused_LocalAlias :1540,
        // ClearAll_TestGlobalCLX_LocalAliasWroteBefore :1568.
        (Plain | SingleInstance | Test, Integer, Shared) => true,
        // ClearSharedCodeunitTempRef.al ClearAll_GlobalUnusedIface :382, ClearAll_GlobalUnusedValueParam :424,
        // ClearAll_GlobalWritten_NoAlias :1362; ClearSharedCodeunitSIOwner.al ClearAll_SIOwnerUnused_Alias :1411,
        // A3_Step3_ClearAllInOwner :447, ClearAll_TestGlobalUnused_LocalAlias :2058, ClearAll_TestGlobalWritten_NoAlias
        // :2207; ClearSharedCodeunitOwnerKinds.al NormalSubtypeOwner_* :470 :489 :511, TableNoOwner_* :528 :547 :569,
        // ReportOwner_* :586 :605 :627, SIValueParam_ClearAll_* :1805 :1818, TestGlobalValueParam_ClearAll_* :1854 :1888.
        (Plain | SingleInstance | Test | Normal | TableNo | Report, TempRef, _) => true,
        // ClearSharedCodeunitSIOwner.al ArrStore_ClearAll_Alias_AliasWroteBefore :1759; ClearSharedCodeunitOwnerKinds.al
        // ArrStore_ClearAll_UnusedAlias :418, ArrStore_ClearAll_InHolder_NoAlias :439, SIArr_* :987 :1013 :1029,
        // M2_Step5_ClearAllInOwner :1241, ClearAll_TestGlobalArr* :1276 :1303 :1333.
        (Plain | SingleInstance | Test, TempArray, _) => true,
        // ClearSharedCodeunitLabelStore.al LabelFirst_ClearAllInHolder_Written_NoAlias :354, _AliasWroteBefore :386,
        // _UnusedAlias :422, and the same for Mid (:497 :529 :565) and Last (:640 :672 :708).
        (Plain, TempRefLabel, _) => true,
        // ClearSharedCodeunitLabelStore.al First_L3_Step3_ClearAllInOwner :1415, Mid :1612, Last :1809.
        (SingleInstance, TempRefLabel, Written) => true,
        // ClearSharedCodeunitOwnerKinds.al StorePlus_* :644 :661 :684, Deep_* :701 :723 :745.
        (Plain, TempRefPlus | Wrapper, _) => true,
        _ => false,
    }
}

/// Whether a by-value parameter of the routine frame `f` runs holds instance `rc`.
fn value_parameter_holds(s: &Session, f: &Frame, rc: &Rc<RefCell<Instance>>) -> bool {
    let r = &s.engine.program.objects[f.object].routines[f.routine];
    (f.locals.iter().take(r.params).enumerate())
        .any(|(i, l)| r.by_var.get(i) != Some(&true) && matches!(l, Value::Codeunit(x) if Rc::ptr_eq(x, rc)))
}

/// Whether report `r` is the owner shape ClearSharedCodeunitOwnerKinds.al measures ("CLQ Report Owner" :138, "CLQ Report
/// Rows" :378, used as a variable and never run): `ProcessingOnly = true`, `UseRequestPage = false`, no request page, no
/// data item, no OnInitReport, OnPreReport or OnPostReport, no reportextension that changes its run.
fn measured_report(r: &al_ir::Report) -> bool {
    r.processing_only
        && r.use_request_page == Some(false)
        && !r.has_request_page
        && r.items.is_empty()
        && r.on_init.is_none()
        && r.on_pre.is_none()
        && r.on_post.is_none()
        && r.extensions.is_empty()
}

/// Whether `r`, `f` and `i`, three globals in order, are those of "CLB Store" (open corpus PR 1561,
/// ClearSharedCodeunitTempRef.al:55-59): one temporary record, one array of RecordRef of two dimensions, one array of
/// Integer of one dimension. The sizes of the arrays and the table are not read.
fn temp_ref_kinds(r: &RtType, f: &RtType, i: &RtType) -> bool {
    temp_record(r) && array_with(f, 2, record_ref) && array_with(i, 1, integer)
}

/// Whether `t` is an array of `dims` dimensions whose element type passes `el`.
fn array_with(t: &RtType, dims: usize, el: fn(&RtType) -> bool) -> bool {
    matches!(t, RtType::Array(d, e) if d.len() == dims && el(e))
}

fn temp_record(t: &RtType) -> bool {
    matches!(t, RtType::Record(_, true))
}

fn record_ref(t: &RtType) -> bool {
    matches!(t, RtType::Library(k) if *k == al_builtins::library_kind::LibraryKind::RecordRef)
}

fn integer(t: &RtType) -> bool {
    matches!(t, RtType::Integer)
}

/// The owners whose `ClearAll()` the corpus measures on a store global ([`Session::owner_kind`]).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Owner {
    /// A codeunit with no TableNo that is not SingleInstance and states no Subtype ("CLX Holder", ClearSharedCodeunit.al
    /// codeunit 75581; "CLB Holder", ClearSharedCodeunitTempRef.al:122).
    Plain,
    /// A SingleInstance codeunit with no TableNo that states no Subtype ("CLS SI Owner B04", ClearSharedCodeunitSIOwner.al:672).
    SingleInstance,
    /// A codeunit with no TableNo that is not SingleInstance and states `Subtype = Test` ("CLS TG ClearAll 1",
    /// ClearSharedCodeunitSIOwner.al:2048).
    Test,
    /// A codeunit with no TableNo that is not SingleInstance and states `Subtype = Normal` ("CLQ Normal Owner",
    /// ClearSharedCodeunitOwnerKinds.al:50).
    Normal,
    /// A codeunit with a TableNo that is not SingleInstance and states no Subtype ("CLQ TableNo Owner",
    /// ClearSharedCodeunitOwnerKinds.al:94).
    TableNo,
    /// A report of the shape [`measured_report`] reads.
    Report,
}

/// The store shapes the corpus measures `ClearAll()` on ([`Session::store_kind`]).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Store {
    /// One Integer global ("CLX Store", ClearSharedCodeunit.al codeunit 75580).
    Integer,
    /// "CLB Store" ([`temp_ref_kinds`]), with no other global.
    TempRef,
    /// "CLB Store" with one global `Label 'text'` before, between the temporary record and the RecordRef array, or after
    /// ("CLL Store First", "CLL Store Mid", "CLL Store Last", ClearSharedCodeunitLabelStore.al:30, :83, :136).
    TempRefLabel,
    /// "CLB Store" with one Integer global after ("CLQ Store Plus", ClearSharedCodeunitOwnerKinds.al:183).
    TempRefPlus,
    /// A "CLB Store" global then an Integer global ("CLQ Wrapper", ClearSharedCodeunitOwnerKinds.al:268).
    Wrapper,
    /// "CLS Arr Store" (ClearSharedCodeunitSIOwner.al:42-47): an array of temporary records, an array of RecordRef and an
    /// array of Integer, each of one dimension.
    TempArray,
}

/// How the instance of a store global is held when `ClearAll()` runs in its owner.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Holding {
    /// Another variable, parameter or interface holds it too.
    Shared,
    /// Nothing else holds it, and its temporary table holds rows.
    Written,
}
