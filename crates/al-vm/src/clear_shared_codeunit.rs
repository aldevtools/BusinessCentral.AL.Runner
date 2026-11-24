//! `Clear` and `ClearAll()` of a codeunit variable whose instance an interface, a second codeunit variable or a parameter
//! shares (assumption clear-shared-codeunit, open corpus PR 1560, tests/al-language/codeunits/clear-instance/
//! ClearSharedCodeunit.al), and `ClearAll()` of a store codeunit with temporary records (open corpus PR 1561,
//! ClearSharedCodeunitTempRef.al, ClearSharedCodeunitSIOwner.al). `Clear` gives such a variable nothing used yet a fresh
//! instance, and `ClearAll()` resets such an instance in place, keeping its temporary rows, in the measured (owner, store)
//! pairs; the pairs the probes do not measure refuse. An instance nothing else holds keeps main's answer unless its
//! temporary table holds rows. Entry points: `Session::clear_shared_codeunit` (from the `Clear` site, `exec/run.rs`) and
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

    /// The globals of `owner` (not in `keep`) that `ClearAll()` in `owner` resets in place. A global whose instance another
    /// holder shares, of a measured (owner, store) pair ([`Session::shared_pair`]): its globals go back to their declared
    /// values, a temporary table keeping its rows, and the variable keeps the instance, so the holder sees the reset and later
    /// writes through either side (ClearSharedCodeunit.al ClearAll_GlobalUnusedIface; ClearSharedCodeunitTempRef.al
    /// ClearAll_GlobalUnusedAlias :403). A shared instance of any other pair refuses: main's fresh instance would detach it,
    /// which BC does not do in the measured pairs. A global nothing else holds whose temporary table holds rows, of the "CLB
    /// Store" shape under any [`Owner`], is reset the same way (ClearAll_GlobalWritten_NoAlias :1362,
    /// ClearSharedCodeunitSIOwner.al A3_Step3_ClearAllInOwner :447, ClearAll_TestGlobalWritten_NoAlias :2207); any other
    /// with rows refuses, main's fresh instance would lose them. Any other global nothing else holds keeps main's answer.
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
                    if !self.shared_pair(owner, v) {
                        return Err(Signal::unsupported(
                            "ClearAll of a global codeunit variable whose instance another variable shares, in a shape \
                             the corpus does not pin",
                        ));
                    }
                    al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::ClearSharedCodeunit);
                    if rc.borrow().initialized {
                        self.reset_keeping_rows(rc);
                    }
                    kept.push(i);
                }
                // Nothing else holds the instance and its temporary table holds rows: BC resets a store of 1561's shape in
                // place and keeps the rows; main's fresh instance would lose them.
                Value::Codeunit(rc)
                    if self.plain_codeunit(rc.borrow().object as usize) && self.holds_temp_rows(v, 0) =>
                {
                    let object = rc.borrow().object as usize;
                    if !(matches!(
                        self.owner_kind(owner),
                        Some(Owner::Plain | Owner::SingleInstance | Owner::Test)
                    ) && self.store_kind(object, Labels::Counted) == Some(Store::TempRef)
                        && !self.frames.contains(&object))
                    {
                        return Err(temp_rows_refusal());
                    }
                    al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::ClearSharedCodeunit);
                    self.reset_keeping_rows(rc);
                    kept.push(i);
                }
                Value::Array(items)
                    if (items.borrow().iter())
                        .any(|e| matches!(e, Value::Codeunit(_)) && self.holds_temp_rows(e, 0)) =>
                {
                    return Err(temp_rows_refusal());
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

    /// Checks the globals of `owner` (not in `keep`) that `ClearAll()` resets for an array of temporary records whose table
    /// was written: `keep_temp_table` (`interp.rs`) gives the fresh array the old table, so the rows stay, in a codeunit that
    /// is not SingleInstance with no TableNo that states no Subtype ("CLS Arr Store", ClearSharedCodeunitSIOwner.al
    /// ArrStore_ClearAll_InsideStore :1808, rows1=2 rows2=2 after ClearAll), through assumption clear-shared-codeunit. Any
    /// other owner refuses: main gave such an array fresh tables, and 1561 measures only that owner.
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
        if self.owner_kind(owner) == Some(Owner::Plain) {
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

    /// The kind of object `owner` is, for the owners whose `ClearAll()` the corpus measures on a store global: a codeunit
    /// with no TableNo that is not SingleInstance and states no Subtype ("CLX Holder", ClearSharedCodeunit.al codeunit 75581;
    /// "CLB Holder", ClearSharedCodeunitTempRef.al:122), one that is SingleInstance and states no Subtype ("CLS SI Owner
    /// B04", ClearSharedCodeunitSIOwner.al:672), or one that is not SingleInstance and states `Subtype = Test` ("CLS TG
    /// ClearAll 1", ClearSharedCodeunitSIOwner.al:2048). `None` for any other object.
    fn owner_kind(&self, owner: usize) -> Option<Owner> {
        let o = &self.engine.program.objects[owner];
        if o.kind != al_ir::ObjKind::Codeunit || o.rec_slot.is_some() {
            return None;
        }
        match (
            self.declared_subtype(owner),
            self.engine.object_facts[owner].single_instance,
        ) {
            (None, false) => Some(Owner::Plain),
            (None, true) => Some(Owner::SingleInstance),
            (Some(v), false) if v.eq_ignore_ascii_case("Test") => Some(Owner::Test),
            _ => None,
        }
    }

    /// The store shape of codeunit `object`, by its globals (a Label counted or skipped as `labels` says), for the stores the
    /// corpus measures: not SingleInstance, no TableNo, and globals of [`Store`]'s kinds. `None` for any other.
    fn store_kind(&self, object: usize, labels: Labels) -> Option<Store> {
        let o = &self.engine.program.objects[object];
        if !self.plain_codeunit(object) || o.rec_slot.is_some() {
            return None;
        }
        let kinds: Vec<&RtType> = (o.globals.iter())
            .filter(|g| labels == Labels::Counted || !g.label)
            .map(|g| &g.ty)
            .collect();
        match kinds.as_slice() {
            [RtType::Integer] => Some(Store::Integer),
            _ if temp_ref_kinds(&kinds) => Some(Store::TempRef),
            [r, f, i] if array_with(r, 1, temp_record) && array_with(f, 1, record_ref) && array_with(i, 1, integer) => {
                Some(Store::TempArray)
            }
            _ => None,
        }
    }

    /// Whether `ClearAll()` in `owner` resets the shared instance `v` (a global that another holder shares) in place: the
    /// instance is not bound by BindSubscription (`clear_all_bound_globals` owns those), no routine of its codeunit runs, and
    /// (owner, store) is a pair the corpus measures. Each measured pair runs through assumption clear-shared-codeunit:
    /// - a plain owner and the Integer store (ClearSharedCodeunit.al ClearAll_GlobalUnusedIface,
    ///   ClearAll_GlobalUsedAlias_AliasWroteBefore, ClearAll_GlobalUsedValueParam);
    /// - any [`Owner`] and the "CLB Store" shape: plain (ClearSharedCodeunitTempRef.al ClearAll_GlobalUnusedIface :382,
    ///   ClearAll_GlobalUnusedValueParam :424, ClearAll_GlobalUsedRowsAlias_AliasWroteBefore :1211), SingleInstance
    ///   (ClearSharedCodeunitSIOwner.al ClearAll_SIOwnerUnused_Alias :1411, ClearAll_SIOwnerTouched_Iface :1585) and test
    ///   (ClearAll_TestGlobalUnused_LocalAlias :2058, ClearAll_TestGlobalWritten_Keeper :2120): rows kept, RecordRef closed,
    ///   Integers 0;
    /// - a plain owner and the "CLS Arr Store" shape whose table holds rows (ArrStore_ClearAll_Alias_AliasWroteBefore
    ///   :1759, rows1=2 rows2=2 ref1=0 int1=0); a never-written one is not measured.
    fn shared_pair(&self, owner: usize, v: &Value) -> bool {
        let Value::Codeunit(rc) = v else { return false };
        let object = rc.borrow().object as usize;
        let measured = match (self.owner_kind(owner), self.store_kind(object, Labels::Counted)) {
            (Some(Owner::Plain), Some(Store::Integer)) => true,
            (Some(_), Some(Store::TempRef)) => true,
            (Some(Owner::Plain), Some(Store::TempArray)) => self.holds_temp_rows(v, 0),
            _ => false,
        };
        measured && !self.holds_bound_codeunit(v) && !self.frames.contains(&object)
    }

    /// Resets instance `rc` in place, as `ClearAll()` in a measured owner does to a store global: its globals go back to
    /// their declared values, a temporary record keeping its table (`keep_temp_table`), so its rows stay
    /// (ClearSharedCodeunitSIOwner.al A3_Step3_ClearAllInOwner :447, rows=1 ref11=0 int1=0). The caller marks assumption
    /// clear-shared-codeunit.
    fn reset_keeping_rows(&mut self, rc: &Rc<RefCell<Instance>>) {
        let engine = self.engine.clone();
        let object = rc.borrow().object as usize;
        let fresh = self.slot_defaults(&engine.program.objects[object].globals);
        let mut inst = rc.borrow_mut();
        for (old, new) in inst.globals.iter().zip(&fresh) {
            crate::interp::keep_temp_table(old, new);
        }
        inst.globals = fresh;
    }

    /// Whether `object` is a codeunit of the store shape of open corpus PR 1561 (ClearSharedCodeunitTempRef.al:55-59, codeunit
    /// 75587 "CLB Store"): not SingleInstance, no TableNo, and its globals, in this order, one temporary record, one array of
    /// RecordRef of two dimensions and one array of Integer of one dimension. The sizes of the arrays and the table are not
    /// read. A global Label or TextConst ([`ir::Slot::label`]) may stand anywhere among them: AL code cannot assign it, and
    /// 1561's store has none.
    pub(crate) fn temp_ref_store(&self, object: usize) -> bool {
        self.store_kind(object, Labels::Skipped) == Some(Store::TempRef)
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
/// temporary rows, in a shape open corpus PR 1561 does not measure.
fn temp_rows_refusal() -> Signal {
    Signal::unsupported(
        "ClearAll of a global codeunit variable whose instance holds temporary rows, in a shape the corpus does not pin",
    )
}

/// Whether `kinds`, the types of a codeunit's globals in order, are those of "CLB Store" (open corpus PR 1561,
/// ClearSharedCodeunitTempRef.al:55-59): one temporary record, one array of RecordRef of two dimensions, one array of
/// Integer of one dimension. The sizes of the arrays and the table are not read.
fn temp_ref_kinds(kinds: &[&RtType]) -> bool {
    matches!(kinds, [r, f, i] if temp_record(r) && array_with(f, 2, record_ref) && array_with(i, 1, integer))
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
    /// A codeunit that is not SingleInstance and states no Subtype.
    Plain,
    /// A SingleInstance codeunit that states no Subtype.
    SingleInstance,
    /// A codeunit that is not SingleInstance and states `Subtype = Test`.
    Test,
}

/// The store shapes the corpus measures `ClearAll()` on ([`Session::store_kind`]).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Store {
    /// One Integer global ("CLX Store", ClearSharedCodeunit.al codeunit 75580).
    Integer,
    /// "CLB Store" ([`temp_ref_kinds`]), with no other global, a Label neither.
    TempRef,
    /// "CLS Arr Store" (ClearSharedCodeunitSIOwner.al:42-47): an array of temporary records, an array of RecordRef and an
    /// array of Integer, each of one dimension.
    TempArray,
}

/// Whether [`Session::store_kind`] counts a global Label or TextConst ([`ir::Slot::label`]) as a global of the store.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Labels {
    /// Counted: a Label makes the store another shape (`ClearAll()`; 1561's stores have none).
    Counted,
    /// Skipped: AL code cannot assign a Label (the login check, `temp_ref_store`).
    Skipped,
}
