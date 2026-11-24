//! `Clear` and `ClearAll()` of a codeunit variable whose instance an interface, a second codeunit variable or a
//! parameter shares (assumption clear-shared-codeunit, open corpus PR 1560, tests/al-language/codeunits/clear-instance/
//! ClearSharedCodeunit.al), and `ClearAll()` of a store codeunit with temporary records (open corpus PRs 1561 and 1564,
//! ClearSharedCodeunitTempRef.al, ClearSharedCodeunitSIOwner.al, ClearSharedCodeunitOwnerKinds.al,
//! ClearSharedCodeunitLabelStore.al, and at commit d8215564 ClearSharedCodeunitSetterOwner.al,
//! ClearSharedCodeunitLabelShare.al, ClearSharedCodeunitTempArrayShare.al, and at commit 65262a80
//! ClearSharedCodeunitStoreShapes.al, and at commit 4531df96 ClearSharedCodeunitHolderKinds.al and
//! ClearSharedCodeunitHolderSlots.al: the slot of the sharer, measured for six stores, one answer for every slot).
//! `Clear` gives such a variable nothing used yet a fresh instance, and `ClearAll()` resets such an instance in place,
//! keeping its temporary rows, in the measured (owner, store, holding) cells (`measured`); the cells the probes do not
//! measure refuse. An instance nothing else holds keeps main's answer unless its temporary table holds rows. The store
//! shapes are also what the login check counts at their declared value (`Session::login_store`, read by
//! `state_stamp.rs`). Entry points: `Session::clear_shared_codeunit` (from the `Clear` site, `exec/run.rs`) and
//! `Session::clear_all_in_place` (from `clear_all`).

use super::*;
use crate::var_option::ArrayOf;
use al_builtins::value::LibraryValue;
use std::rc::Weak;

impl Session {
    /// Whether object `object` is a codeunit that is not SingleInstance: a variable of it holds an instance of its own,
    /// which `Clear` replaces and an assignment shares (ClearSharedCodeunit.al codeunit 75580 "CLX Store"). A SingleInstance
    /// codeunit variable reads the one instance whatever is cleared (ClearSharedCodeunitSI.al).
    pub(crate) fn plain_codeunit(&self, object: usize) -> bool {
        self.engine.program.objects[object].kind == al_ir::ObjKind::Codeunit
            && !self.engine.object_facts[object].single_instance
    }

    /// The globals of `owner` (not in `keep`) that `ClearAll()` there resets in place. A global holding a store
    /// instance that another holder shares, or that nothing else holds while its temporary table holds rows: when the
    /// (owner, store, holding) cell is one the corpus measures ([`Session::measured_cell`]), its globals go back to their
    /// declared values, a temporary table keeping its rows, and the variable keeps the instance, so a sharer sees the
    /// reset and later writes through either side; a temporary table a record outside shares stays shared only in the cells
    /// [`measured`] names. Any other shared instance refuses: main's fresh instance would detach it. Any other with rows now
    /// refuses: main's fresh instance would lose them. So does an unshared one whose table a record outside shares. A global array of "CLB Store"
    /// instances in a plain codeunit resets each element in place ([`Session::clear_all_store_array`]). Any other global
    /// nothing else holds keeps main's answer.
    ///
    /// Only a global declared as a codeunit takes this path. One declared as an interface whose instance another holder
    /// shares refuses; any other (an interface nothing else holds, a Variant) keeps main's answer.
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
                Value::Codeunit(rc) if self.plain_codeunit(rc.borrow().object as usize) => {
                    let shared = Rc::strong_count(rc) > 2;
                    match self.engine.program.objects[owner].globals.get(i).map(|g| &g.ty) {
                        Some(RtType::Codeunit(_)) => {}
                        // BC resets the shared instance in place for another store only (corpus PR 1564
                        // ClearSharedCodeunitStoreShapes.al Whse_InterfaceHolder_HolderClearAll :2001); main clears the
                        // interface and leaves the instance as it was.
                        Some(RtType::Interface(_)) if shared => {
                            return Err(Signal::unsupported(
                                "ClearAll of a global interface variable whose codeunit instance another variable \
                                 shares, in a shape the corpus does not pin",
                            ));
                        }
                        _ => continue,
                    }
                    let hold = self.temp_hold(v);
                    let holding = if shared {
                        if !rc.borrow().initialized {
                            Holding::SharedUncalled
                        } else if !self.temp_rows_now(v) && !self.wrote_scalar(rc, 0) {
                            Holding::SharedUnwritten
                        } else {
                            Holding::Shared
                        }
                    } else if hold != TempHold::Empty {
                        Holding::Written
                    } else if self.wrapped_store_shared(rc) {
                        return Err(Signal::unsupported(
                            "ClearAll of a global codeunit variable whose inner store another variable shares, in a \
                             shape the corpus does not pin",
                        ));
                    } else {
                        continue;
                    };
                    if !self.measured_cell(owner, v, holding, hold) {
                        return Err(match holding {
                            _ if hold.shared() => shared_table_refusal(),
                            Holding::Shared | Holding::SharedUnwritten | Holding::SharedUncalled => {
                                Signal::unsupported(
                                    "ClearAll of a global codeunit variable whose instance another variable shares, in a \
                                 shape the corpus does not pin",
                                )
                            }
                            Holding::Written => temp_rows_refusal(),
                        });
                    }
                    al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::ClearSharedCodeunit);
                    self.reset_keeping_rows(rc);
                    kept.push(i);
                }
                Value::Array(items)
                    if (items.borrow().iter()).any(|e| {
                        self.shared_element(e)
                            || matches!(e, Value::Codeunit(_)) && self.temp_hold(e) != TempHold::Empty
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
    /// shares or holds a temporary table that holds rows or that a record outside shares. In a plain codeunit ([`Owner::Plain`]), for an array of one dimension of "CLB
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
        let holds: Vec<TempHold> = items.borrow().iter().map(|e| self.temp_hold(e)).collect();
        let hold = holds.iter().copied().max().unwrap_or(TempHold::Empty);
        if hold.shared() {
            return Err(shared_table_refusal());
        }
        let measured = self.owner_kind(owner) == Some(Owner::Plain)
            && matches!(ty, RtType::Array(d, e) if d.len() == 1
                && matches!(**e, RtType::Codeunit(c) if self.store_kind(c as usize) == Some(Store::TempRef)))
            && (items.borrow().iter().zip(&holds)).all(|(e, h)| !self.shared_element(e) || *h == TempHold::Rows)
            && (items.borrow().iter()).all(|e| !self.holds_bound_codeunit(e) && !self.store_runs(e));
        if !measured {
            return Err(if hold == TempHold::Rows {
                temp_rows_refusal()
            } else {
                Signal::unsupported(
                    "ClearAll of a global array whose codeunit element another variable shares (not pinned)",
                )
            });
        }
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::ClearSharedCodeunit);
        for e in items.borrow().iter() {
            if let Value::Codeunit(rc) = e {
                self.reset_keeping_rows(rc);
            }
        }
        Ok(())
    }

    /// The slots of the globals of `owner` (not in `keep`) that `ClearAll()` resets whose array of temporary records keeps its
    /// table (`keep_temp_table`, `interp.rs`), through assumption clear-shared-codeunit, in the cells [`measured_array`]
    /// names: an array whose table holds rows now keeps them, one whose table a record outside shares (`Record.Copy(x, true)`)
    /// stays shared with it. Any other such array refuses: main gave it fresh tables. An array whose table holds no row
    /// and that nothing outside shares gets fresh tables, main's answer with no mark (it reads the same).
    pub(crate) fn clear_all_temp_arrays(
        &mut self,
        owner: usize,
        globals: &[Value],
        keep: &[usize],
    ) -> Result<Vec<usize>, Signal> {
        let mut kept = Vec::new();
        for (i, v) in globals.iter().enumerate() {
            let Value::Array(items) = v else { continue };
            if keep.contains(&i) || !items.borrow().iter().any(|e| matches!(e, Value::Record(_))) {
                continue;
            }
            let hold = self.temp_hold(v);
            if hold == TempHold::Empty {
                continue;
            }
            let dims = match &self.engine.program.objects[owner].globals[i].ty {
                RtType::Array(d, _) => d.len(),
                _ => 0,
            };
            if !self.owner_kind(owner).is_some_and(|o| measured_array(o, dims, hold)) {
                return Err(if hold.shared() {
                    shared_table_refusal()
                } else {
                    Signal::unsupported(
                        "ClearAll of a global array of temporary records whose table holds rows, in an object the \
                         corpus does not pin",
                    )
                });
            }
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::ClearSharedCodeunit);
            kept.push(i);
        }
        Ok(kept)
    }

    /// What the temporary tables `v` reaches hold, through the globals of instances of codeunits that are not
    /// SingleInstance, through arrays and through RecordRefs: [`TempHold::SharedOther`] when record states outside those
    /// hold one of the tables (`TempId::sharers` counts more than the walk reaches) other than as one measured share,
    /// else [`TempHold::SharedTable`] when one holds a table as that share, else [`TempHold::Rows`] when one holds a row
    /// now, else [`TempHold::Empty`]. The measured share: exactly one record outside holds the table, and it got it as the
    /// target of `Record.Copy(x, true)` from a record the walk reached (`TempId::copy_targets`). A table written and later
    /// emptied holds no row. Each instance is walked once, so a cycle of instances ends.
    fn temp_hold(&self, v: &Value) -> TempHold {
        let walk = self.temp_walk(v);
        let tables = walk.tables.values();
        let measured_share = |t: &TempTable| {
            let outside: Vec<_> = (t.copy_targets.iter())
                .filter(|(p, _)| !walk.records.contains(p))
                .collect();
            t.sharers == t.own + 1
                && outside.len() == 1
                && outside[0].1.is_some_and(|source| walk.records.contains(&source))
        };
        if tables.clone().any(|t| t.own < t.sharers && !measured_share(t)) {
            TempHold::SharedOther
        } else if tables.clone().any(|t| t.own < t.sharers) {
            TempHold::SharedTable
        } else if tables.clone().any(|t| t.rows) {
            TempHold::Rows
        } else {
            TempHold::Empty
        }
    }

    /// Whether a temporary table `v` reaches ([`Session::temp_hold`]) holds a row now, shared by a record outside or not.
    fn temp_rows_now(&self, v: &Value) -> bool {
        self.temp_walk(v).tables.values().any(|t| t.rows)
    }

    fn temp_walk(&self, v: &Value) -> TempWalk {
        let mut walk = TempWalk::default();
        self.walk_temp(v, &mut walk);
        walk
    }

    fn walk_temp(&self, v: &Value, walk: &mut TempWalk) {
        match v {
            Value::Record(r) => self.note_temp(r, walk),
            Value::Library(k, l) if *k == al_builtins::library_kind::LibraryKind::RecordRef => {
                if let Ok(LibraryValue::RecordRef(r)) = l.try_borrow().as_deref() {
                    self.note_temp(r, walk);
                }
            }
            Value::Codeunit(rc) if self.plain_codeunit(rc.borrow().object as usize) => {
                if !walk.instances.insert(Rc::as_ptr(rc) as usize) {
                    return;
                }
                for g in rc.borrow().globals.iter() {
                    self.walk_temp(g, walk);
                }
            }
            Value::Array(items) => items.borrow().iter().for_each(|e| self.walk_temp(e, walk)),
            _ => {}
        }
    }

    /// Counts record state `r`, once, as a holder of its temporary table.
    fn note_temp(&self, r: &Rc<RefCell<RecordState>>, walk: &mut TempWalk) {
        if !walk.records.insert(Rc::as_ptr(r) as usize) {
            return;
        }
        let r = r.borrow();
        let (true, Some(t)) = (r.temporary, &r.temp_id) else {
            return;
        };
        let table = walk.tables.entry(t.id()).or_insert_with(|| TempTable {
            sharers: t.sharers(),
            copy_targets: t.copy_targets(),
            own: 0,
            rows: self.temps.get(&t.id()).is_some_and(|s| s.holds_rows_unobserved()),
        });
        table.own += 1;
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
        self.store_kind_of(object, true)
    }

    /// [`Session::store_kind`], with the register and the wrapper (the stores that hold another store) only when `holder`.
    /// The store they hold is read with `holder` false: it must be one that holds none, so a codeunit that holds itself,
    /// directly or through others, is no store and is not read again.
    fn store_kind_of(&self, object: usize, holder: bool) -> Option<Store> {
        let o = &self.engine.program.objects[object];
        if !self.plain_codeunit(object) {
            return None;
        }
        if o.rec_slot.is_some() {
            return (holder && self.register_kinds(object)).then_some(Store::Register);
        }
        if matches!(o.globals.as_slice(), [t, e, i, l]
            if temp_record(&t.ty) && matches!(e.ty, RtType::Enum(_)) && integer(&i.ty) && l.comment_label)
        {
            return Some(Store::PriceList);
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
            [RtType::Codeunit(c), n]
                if holder && integer(n) && self.store_kind_of(*c as usize, false) == Some(Store::TempRef) =>
            {
                Store::Wrapper
            }
            [r, f, i] if array_with(r, 1, temp_record) && array_with(f, 1, record_ref) && array_with(i, 1, integer) => {
                Store::TempArray
            }
            [r, i] if array_with(r, 2, temp_record) && array_with(i, 1, integer) => Store::TwoDimArray,
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

    /// Whether codeunit `object`, which has a TableNo, has the globals of "CLR Register" (ClearSharedCodeunitSetterOwner.al
    /// :47-58), in this order: a temporary record, a record that is not temporary, a Decimal, a Boolean, a Code and a
    /// codeunit of [`Store::Integer`]. The length of the Code and the tables are not read.
    fn register_kinds(&self, object: usize) -> bool {
        let o = &self.engine.program.objects[object];
        let globals = &o.globals;
        // The record the TableNo gives the codeunit (`Rec`) is a slot of its own, not a declared global.
        let kinds: Vec<&RtType> = (globals.iter().enumerate())
            .filter(|(i, _)| o.rec_slot != Some(*i))
            .map(|(_, g)| &g.ty)
            .collect();
        !globals.iter().any(|g| g.plain_label)
            && matches!(kinds.as_slice(),
                [RtType::Record(_, true), RtType::Record(_, false), RtType::Decimal, RtType::Boolean, RtType::Code(_),
                    RtType::Codeunit(c)] if self.store_kind_of(*c as usize, false) == Some(Store::Integer))
    }

    /// Whether `ClearAll()` in `owner` resets store instance `v`, held as `holding` says, in place: the instance is not bound
    /// by BindSubscription (`clear_all_bound_globals` owns those), no routine of its codeunit runs, and (owner, store,
    /// holding) is a cell the corpus measures.
    fn measured_cell(&self, owner: usize, v: &Value, holding: Holding, hold: TempHold) -> bool {
        let Value::Codeunit(rc) = v else { return false };
        let (Some(o), Some(s)) = (self.owner_kind(owner), self.store_kind(rc.borrow().object as usize)) else {
            return false;
        };
        measured(o, s, holding, hold)
            && (s != Store::Register || self.register_as_measured(rc))
            // The store with one more Integer that no call made is measured for "CLQ Store Plus" only
            // (ClearSharedCodeunitOwnerKinds.al:183, StorePlus_UnusedAlias :684; ClearSharedCodeunitPlusUncalled.al), which
            // implements no interface: no Interface, List or Dictionary slot can share it.
            && ((s, holding) != (Store::TempRefPlus, Holding::SharedUncalled) || self.implements_no_interface(rc))
            && (s != Store::PriceList || hold != TempHold::SharedTable || self.temp_rows_now(v))
            && !self.holds_bound_codeunit(v)
            && !self.store_runs(v)
    }

    /// Whether codeunit instance `rc` is of a codeunit that lists no interface after `implements`
    /// ([`al_ir::IMPLEMENTS_INTERFACE`]).
    fn implements_no_interface(&self, rc: &Rc<RefCell<Instance>>) -> bool {
        let o = &self.engine.program.objects[rc.borrow().object as usize];
        o.properties.iter().all(|(k, _)| k != al_ir::IMPLEMENTS_INTERFACE)
    }

    /// Whether a global of instance `rc`, or of an instance a codeunit global of it holds, holds a scalar other than its
    /// declared value by its fingerprint (`single_track::fp`): what every shared cell of the wrapper, the 2-D store, the
    /// register and the price store wrote before the ClearAll. Records and arrays of records are left to
    /// [`Session::temp_hold`].
    fn wrote_scalar(&mut self, rc: &Rc<RefCell<Instance>>, depth: u8) -> bool {
        if depth > 8 || !rc.borrow().initialized {
            return false;
        }
        let engine = self.engine.clone();
        let object = rc.borrow().object as usize;
        let slots = &engine.program.objects[object].globals;
        let fresh = self.slot_defaults(slots);
        let held = rc.borrow().globals.clone();
        (held.iter().zip(&fresh).zip(slots)).any(|((v, f), slot)| match (v, &slot.ty) {
            (Value::Codeunit(inner), _) => self.wrote_scalar(inner, depth + 1),
            (_, RtType::Record(..)) => false,
            (_, RtType::Array(_, e)) if matches!(**e, RtType::Record(..)) => false,
            _ => crate::single_track::fp(v, &engine) != crate::single_track::fp(f, &engine),
        })
    }

    /// Whether register instance `rc` ([`Store::Register`]) holds only what ClearSharedCodeunitSetterOwner.al measures before
    /// the ClearAll (`Mark` :62-70): its record that is not temporary has a filter and field values but read or wrote no row
    /// (`state_stamp::no_row_io`), and its nested codeunit instance no other holder shares.
    fn register_as_measured(&self, rc: &Rc<RefCell<Instance>>) -> bool {
        rc.borrow().globals.iter().all(|g| match g {
            Value::Record(r) => r
                .try_borrow()
                .is_ok_and(|r| r.temporary || crate::state_stamp::no_row_io(&r)),
            Value::Codeunit(n) => Rc::strong_count(n) == 1,
            _ => true,
        })
    }

    /// Resets instance `rc` in place, as `ClearAll()` in a measured owner does to a store global: its globals go back to
    /// their declared values, a temporary record keeping its table (`keep_temp_table`), so its rows stay
    /// (ClearSharedCodeunitSIOwner.al A3_Step3_ClearAllInOwner :447, rows=1 ref11=0 int1=0); the store a
    /// [`Store::Wrapper`] holds is reset in place the same way and kept, so its alias sees the reset
    /// (ClearSharedCodeunitOwnerKinds.al Deep_StoreAliasWroteBefore :723). An instance nothing made yet stays as it is. The
    /// caller marks assumption clear-shared-codeunit.
    fn reset_keeping_rows(&mut self, rc: &Rc<RefCell<Instance>>) {
        if !rc.borrow().initialized {
            return;
        }
        let engine = self.engine.clone();
        let object = rc.borrow().object as usize;
        let mut fresh = self.slot_defaults(&engine.program.objects[object].globals);
        let old = rc.borrow().globals.clone();
        for (old, new) in old.iter().zip(fresh.iter_mut()) {
            match old {
                Value::Codeunit(inner) => {
                    self.reset_keeping_rows(inner);
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

/// The refusal of a `ClearAll` that resets a temporary table a record outside the variable shares, in a cell no probe
/// measures: a `Record.Copy(x, true)` target outside the measured cells, or any other sharer ([`TempHold::SharedOther`]).
fn shared_table_refusal() -> Signal {
    Signal::unsupported(
        "ClearAll of a global variable that reaches a temporary table that another record shares, in a shape the corpus \
         does not pin",
    )
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
fn measured(owner: Owner, store: Store, holding: Holding, hold: TempHold) -> bool {
    use {Holding::*, Owner::*, Store::*};
    if hold.shared() {
        // ClearSharedCodeunitSetterOwner.al: the register's table shared with a record of the test through Copy(Rows, true)
        // stays shared, rows or none (SharedRows, SharedNever, SharedEmptied :397 :434 :470 TableNo Run, :587 :624 :660
        // TableNo procedure, :777 :814 :850 plain Run, :967 :1004 :1040 plain procedure, :1130 by-value setter).
        // ClearSharedCodeunitStoreShapes.al Price_T1SharedByCopy_SetLine :1322: the price store nothing else holds, its
        // T1 shared with a record of the test through R.Copy(T1, true), stays shared (r=2, writes cross). Measured with
        // rows only (`Session::measured_cell` reads them).
        // A table shared some other way (`TempHold::SharedOther`) is not measured.
        return hold == TempHold::SharedTable
            && matches!(
                (owner, store, holding),
                (Plain | TableNo, Register, Shared) | (Plain, PriceList, Written)
            );
    }
    match (owner, store, holding) {
        // ClearSharedCodeunit.al ClearAll_GlobalUnusedIface, ClearAll_GlobalUsedAlias_AliasWroteBefore,
        // ClearAll_GlobalUsedValueParam; ClearSharedCodeunitOwnerKinds.al SICLX_ClearAll_UnusedAlias :1472,
        // SICLX_ClearAll_AliasWroteBefore :1492, ClearAll_TestGlobalCLXUnused_LocalAlias :1540,
        // ClearAll_TestGlobalCLX_LocalAliasWroteBefore :1568. No call made the instance: the Unused cells of each owner.
        // Every holder slot, written and never, under each owner: ClearSharedCodeunitOwnerSlotsCLX.al PlainCLX_VarParam_Written
        // :584 to :1762, SICLX_* :4819 to :5997, TGCLX_* :6482 to :10215.
        (Plain | SingleInstance | Test, Integer, Shared | SharedUnwritten | SharedUncalled) => true,
        // ClearSharedCodeunitTempRef.al ClearAll_GlobalUnusedIface :382, ClearAll_GlobalUnusedValueParam :424,
        // ClearAll_GlobalWritten_NoAlias :1362; ClearSharedCodeunitSIOwner.al ClearAll_SIOwnerUnused_Alias :1411,
        // A3_Step3_ClearAllInOwner :447, ClearAll_TestGlobalUnused_LocalAlias :2058, ClearAll_TestGlobalWritten_NoAlias
        // :2207; ClearSharedCodeunitOwnerKinds.al NormalSubtypeOwner_* :470 :489 :511, TableNoOwner_* :528 :547 :569,
        // ReportOwner_* :586 :605 :627, SIValueParam_ClearAll_* :1805 :1818, TestGlobalValueParam_ClearAll_* :1854 :1888.
        // Every holder slot, rows now and none, under each owner: ClearSharedCodeunitOwnerSlotsCLB.al PlainCLB_VarParam_Rows
        // :1831 to :3009, NormalCLB_* :3064 to :4242, TableNoCLB_* :4297 to :5475, ReportCLB_* :5530 to :6708, SICLB_*
        // :9765 to :10943, TGCLB_* :11428 to :15161.
        (Plain | SingleInstance | Test | Normal | TableNo | Report, TempRef, _) => true,
        // ClearSharedCodeunitSIOwner.al ArrStore_ClearAll_Alias_AliasWroteBefore :1759; ClearSharedCodeunitOwnerKinds.al
        // ArrStore_ClearAll_UnusedAlias :418, ArrStore_ClearAll_InHolder_NoAlias :439, SIArr_* :987 :1013 :1029,
        // M2_Step5_ClearAllInOwner :1241, ClearAll_TestGlobalArr* :1276 :1303 :1333. Every holder slot under the plain
        // owner, rows now and none: ClearSharedCodeunitHolderKinds.al Arr_VarParam_Rows :2090 to Arr_SetterIn_Never :2368;
        // ClearSharedCodeunitHolderSlots.al :7263 to :8102. Under the SingleInstance and test owners:
        // ClearSharedCodeunitOwnerSlotsArr.al SIArr_VarParam_Rows :1806 to :2488, SIArrF_* :6469 to :7004, TGArr_* :2759
        // to :4918, TGArrF_* :7313 to :8975.
        (Plain | SingleInstance | Test, TempArray, _) => true,
        // ClearSharedCodeunitLabelStore.al LabelFirst_ClearAllInHolder_Written_NoAlias :354, and the same for Mid (:497)
        // and Last (:640).
        (Plain, TempRefLabel, Written) => true,
        // Shared and written before: a var-parameter alias (ClearSharedCodeunitLabelStore.al :386), an interface
        // (ClearSharedCodeunitLabelShare.al :451 :567 :683), a by-value parameter of the clearing routine (:503 :619 :735)
        // or of its caller (:519 :635 :751). Never written, only the alias is measured (:422), and the kind of holder is
        // not read: refused.
        (Plain, TempRefLabel, Shared) => hold == TempHold::Rows,
        // ClearSharedCodeunitLabelStore.al First_L3_Step3_ClearAllInOwner :1415, Mid :1612, Last :1809.
        (SingleInstance, TempRefLabel, Written) => true,
        // ClearSharedCodeunitOwnerKinds.al StorePlus_* :644 :661 :684, Deep_* :701 :723 :745. Every holder slot, rows now
        // and none: ClearSharedCodeunitHolderKinds.al Plus_VarParam_Rows :2396 to Wrap_SetterIn_Never :3184;
        // ClearSharedCodeunitHolderSlots.al Plus :8127 to :9132, Wrap :9161 to :10534.
        // No call made the shared instance: StorePlus_UnusedAlias :684, and every slot that needs no interface under the
        // plain owner, ClearSharedCodeunitPlusUncalled.al PlainPlus_VarParam_Never :765 to PlainPlus_VariantGlobal_Never
        // :1084, for "CLQ Store Plus", which implements no interface (`Session::measured_cell`). The wrapper runs shared only
        // after a call made the instance and wrote (every such cell writes first). Shared with no call, or after calls that
        // wrote nothing: refused; the no-call shape is measured since corpus PR 1564 commit b77f20dc
        // (ClearSharedCodeunitTwoHolders.al PlainWrap_TwoVarParam_Never :13650), not opened yet.
        (Plain, TempRefPlus, _) | (Plain, Wrapper, Written | Shared) => true,
        // ClearSharedCodeunitSetterOwner.al: shared through a setter, TableNo owner (:318 :345 :372 Run, :508 :535 :562
        // procedure), plain owner (:698 :725 :752 Run, :888 :915 :942 procedure), by-value setter (:1078 :1105); the owner's
        // own register (:1166 :1187 TableNo, :1208 :1229 plain).
        // Every holder slot, rows now and none, under both owners: ClearSharedCodeunitHolderKinds.al RegPlain_VarParam_Rows
        // :1496 to RegTableNo_SetterOut_Never :2057; ClearSharedCodeunitHolderSlots.al by-value parameters and Variant
        // (RegPlain :5195 to :5378, RegTableNo :5607 to :5790), Interface, List and Dictionary slots (RegPlain :6025 to
        // :6310, RegTableNo :6641 to :6926); the same with no row (RegPlain :5405 to :5580 and :6335 to :6618, RegTableNo
        // :5817 to :5992 and :6951 to :7234).
        // Shared only after a call made the instance and wrote; with no call (measured since b77f20dc,
        // ClearSharedCodeunitTwoHolders.al PlainReg_TwoVarParam_Never :19745, not opened yet) or after calls that wrote
        // nothing: refused.
        (Plain | TableNo, Register, Written | Shared) => true,
        // ClearSharedCodeunitStoreShapes.al: the price store nothing else holds, rows now, ClearAll in SetLine called
        // through an interface (Price_ViaInterface_Filled_SetLine :1206) or directly (:1225), twice (:1277).
        // Shared, rows now or none: a test local through a var getter (Price_StoreAlias_SetLine :1296), and every holder
        // slot of ClearSharedCodeunitHolderKinds.al (var parameter, other and SingleInstance global, caller local, array
        // element, setters: Price_VarParam_Rows :1156 to Price_SetterIn_Never :1464) and ClearSharedCodeunitHolderSlots.al
        // (by-value parameter live, passed, deep and through setters :4215 to :4309, Variant :4338 :4373 :4390, Interface
        // local, caller local and global :4418 :4447 :4464, List and Dictionary of Interface :4492 :4527 :4555 :4590,
        // Interface parameter var and by value :4618 :4639 :4660 :4680; the same cells with no row :4702 to :5167). Every
        // shared cell of these files made the instance with a call that wrote a scalar first (Price_VarParam_Never :1323
        // calls Mark()). With no call (measured since b77f20dc, ClearSharedCodeunitTwoHolders.al
        // PlainPrice_TwoVarParam_Never :22805, not opened yet) or after calls that wrote nothing: refused.
        (Plain, PriceList, Written | Shared) => true,
        // ClearSharedCodeunitTempArrayShare.al TwoDimWritten_InStore_ClearAll_NoAlias :688, ..._AliasWroteBefore :707.
        // Every holder slot, rows now and none: ClearSharedCodeunitHolderKinds.al TwoDim_VarParam_Rows :3228 to
        // TwoDim_SetterIn_Never :3511; ClearSharedCodeunitHolderSlots.al :10569 to :11502.
        // Shared only after a call made the instance and wrote; with no call (measured since b77f20dc, not opened yet) or
        // after calls that wrote nothing: refused.
        (Plain, TwoDimArray, Written | Shared) => true,
        _ => false,
    }
}

/// The cells in which `ClearAll()` in `owner` keeps the table of its global array of temporary records of `dims` dimensions
/// whose tables hold `hold` (never [`TempHold::Empty`]); the elements of such an array read one table.
fn measured_array(owner: Owner, dims: usize, hold: TempHold) -> bool {
    use Owner::*;
    match (hold, dims, owner) {
        // ClearSharedCodeunitSIOwner.al ArrStore_ClearAll_InsideStore :1808 (plain, rows1=2 rows2=2 after ClearAll);
        // ClearSharedCodeunitOwnerKinds.al ReportRows_ArrayElement1Written :814 (report, `array[2]`).
        (TempHold::Rows, 1, Plain | Report) => true,
        // ClearSharedCodeunitTempArrayShare.al TwoDimWritten_OwnerGlobal_ClearAll :655 (plain),
        // SITwoDimWritten_OwnerGlobal_ClearAll :872 (SingleInstance): `array[2, 2]`, rows kept.
        (TempHold::Rows, 2, Plain | SingleInstance) => true,
        // ClearSharedCodeunitTempArrayShare.al: element 1 shared with a record of the test through Copy(A[1], true), never
        // written or written once, keeps the shared table (ArrNever_ :519, ArrWritten_ :555 plain; SIArrNever_ :736,
        // SIArrWritten_ :772 SingleInstance).
        (TempHold::SharedTable, 1, Plain | SingleInstance) => true,
        _ => false,
    }
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
    /// "CLT B2 Store" (ClearSharedCodeunitTempArrayShare.al:457-461): an array of temporary records of two dimensions and an
    /// array of Integer of one dimension.
    TwoDimArray,
    /// "CLR Register" ([`Session::register_kinds`]), a codeunit with a TableNo.
    Register,
    /// "CLV Price Store" (ClearSharedCodeunitStoreShapes.al:1080-1086): exactly, in this order, a temporary record, an
    /// Enum, an Integer and a `Label 'text', Comment = 'text'` ([`ir::Slot::comment_label`]).
    PriceList,
}

/// How the instance of a store global is held when `ClearAll()` runs in its owner.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Holding {
    /// Another variable, parameter or interface holds it too, a call made the instance (`Instance::initialized`), and a
    /// temporary table holds rows or a scalar holds another value than its declared one ([`Session::wrote_scalar`]).
    Shared,
    /// Another variable, parameter or interface holds it too, a call made the instance, and no row and no scalar was
    /// written.
    SharedUnwritten,
    /// Another variable, parameter or interface holds it too, and no call has made the instance yet.
    SharedUncalled,
    /// Nothing else holds it, and its temporary table holds rows now ([`TempHold::Rows`]) or a record outside shares it
    /// ([`TempHold::shared`]).
    Written,
}

/// What the temporary tables a global reaches hold ([`Session::temp_hold`]), in rising order.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum TempHold {
    /// No row now, and no record outside shares a table.
    Empty,
    /// A row now, and no record outside shares a table.
    Rows,
    /// One record outside holds a table, the target of `Record.Copy(x, true)` from a record the walk reached, the one share
    /// the probes measure (ClearSharedCodeunitSetterOwner.al:87, ClearSharedCodeunitTempArrayShare.al:39,
    /// ClearSharedCodeunitStoreShapes.al:1119, ClearSharedCodeunitCopyHolder.al:39: one target, copied from the store's
    /// own record).
    SharedTable,
    /// Records outside hold a table some other way: a RecordRef's SetTable or Copy with ShareTable, a page's Rec, a record
    /// state clone, two Copy targets, or a Copy target whose source is not the store's record. Never measured: every cell
    /// refuses it.
    SharedOther,
}

impl TempHold {
    /// Whether a record outside holds one of the tables ([`TempHold::SharedTable`] or [`TempHold::SharedOther`]).
    fn shared(self) -> bool {
        self >= TempHold::SharedTable
    }
}

/// The holders [`Session::walk_temp`] reached, per temporary table.
#[derive(Default)]
struct TempWalk {
    tables: std::collections::HashMap<u64, TempTable>,
    instances: std::collections::HashSet<usize>,
    records: std::collections::HashSet<usize>,
}

struct TempTable {
    /// `TempId::sharers` of the table.
    sharers: usize,
    /// The live records that got the table as the target of `Record.Copy(x, true)`, by address, each with the address of
    /// the record it copied while that record lives (`TempId::copy_targets`).
    copy_targets: Vec<(usize, Option<usize>)>,
    /// The record states the walk reached that hold it.
    own: usize,
    rows: bool,
}
