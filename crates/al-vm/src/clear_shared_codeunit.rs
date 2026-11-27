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
use al_builtins::assumptions::{Assumption, assume};
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
                    let holders = Rc::strong_count(rc);
                    let shared = holders > 2;
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
                    // A register a nested codeunit links back to is shared by that link too; held by nothing outside
                    // the owner, no cell measures it (1596 always has the caller hold it).
                    if shared && holders - self.back_links(rc) <= 2 {
                        return Err(Signal::unsupported(
                            "ClearAll of a global codeunit variable that only a codeunit nested in it shares, in a \
                             shape the corpus does not pin",
                        ));
                    }
                    let hold = self.temp_hold(v);
                    let holding = if shared {
                        if !rc.borrow().initialized {
                            Holding::SharedUncalled
                        } else if !self.temp_rows_now(v) && !self.wrote_scalar(rc, 0, self.kinds_shape(rc)) {
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
                                self.shared_store_refusal(owner, v, holding, hold)
                            }
                            Holding::Written => temp_rows_refusal(),
                        });
                    }
                    assume(match self.store_kind_in(owner, rc.borrow().object as usize) {
                        Some(Store::Kinds) => Assumption::ClearAllStoreGlobalKinds,
                        Some(Store::TableNoKinds) => Assumption::ClearAllTableNoRegister,
                        Some(Store::TableNoSingle) => Assumption::ClearAllRegisterSingleInstance,
                        Some(Store::TableNoWhse) => Assumption::ClearAllWhseRegisterStore,
                        Some(Store::Tree) => Assumption::ClearAllRegisterTree,
                        _ => Assumption::ClearSharedCodeunit,
                    });
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
        rc.borrow().globals.iter().any(|g| self.shared_element(g))
            && self.store_kind(rc.borrow().object as usize) == Some(Store::Wrapper)
    }

    /// Whether a routine of the codeunit of instance `v`, or of a codeunit its globals hold, runs.
    fn store_runs(&self, v: &Value) -> bool {
        self.store_runs_once(v, &mut Vec::new())
    }

    /// [`Session::store_runs`], each instance read once (`seen`), so a cycle of instances ends.
    fn store_runs_once(&self, v: &Value, seen: &mut Vec<*const RefCell<Instance>>) -> bool {
        let Value::Codeunit(rc) = v else { return false };
        if seen.contains(&Rc::as_ptr(rc)) {
            return false;
        }
        seen.push(Rc::as_ptr(rc));
        let inst = rc.borrow();
        self.frames.contains(&(inst.object as usize)) || inst.globals.iter().any(|g| self.store_runs_once(g, seen))
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

    /// [`Session::store_kind`], with the two stores that hold another store, [`Store::Wrapper`] and [`Store::Register`],
    /// only when `holder`. The store they hold is read with `holder` false: each measured one holds a store that holds no
    /// other ("CLB Store" in "CLQ Wrapper", ClearSharedCodeunitOwnerKinds.al:268; "CLX Store" in "CLR Register",
    /// ClearSharedCodeunitSetterOwner.al:47-58). So a codeunit that holds itself, directly or through other codeunits, has
    /// no measured shape and is read once more at most (al-runner issue 9022).
    fn store_kind_of(&self, object: usize, holder: bool) -> Option<Store> {
        let o = &self.engine.program.objects[object];
        if !self.plain_codeunit(object) {
            return None;
        }
        if o.rec_slot.is_some() {
            return match (
                holder && self.register_kinds(object),
                holder && self.tableno_unpinned(object).is_none(),
                holder && self.single_register_unpinned(object, false).is_none(),
                holder && self.single_register_unpinned(object, true).is_none(),
            ) {
                (true, ..) => Some(Store::Register),
                (false, true, ..) => Some(Store::TableNoKinds),
                (false, false, true, _) => Some(Store::TableNoSingle),
                (false, false, false, true) => Some(Store::TableNoWhse),
                (false, false, false, false) => None,
            };
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
            _ => return self.kinds_store(object, 0).then_some(Store::Kinds),
        };
        match (store, labels.as_slice()) {
            (_, []) => Some(store),
            // "CLL Store First" (:30), "CLL Store Mid" (:83), "CLL Store Last" (:136): the Label before, between the
            // temporary record and the RecordRef array, after.
            (Store::TempRef, [0 | 1 | 3]) => Some(Store::TempRefLabel),
            _ => self.kinds_store(object, 0).then_some(Store::Kinds),
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

    /// Whether codeunit `object` is a store of the global kinds ClearSharedCodeunitGlobalKinds.al "CKK Store" (:104-145),
    /// ClearSharedCodeunitWhseRegister.al (:10-149) and ClearSharedCodeunitLateBinding.al (:12-67) hold (merged corpus PR
    /// 1587, assumption clear-all-store-global-kinds): a plain codeunit without a TableNo whose every global is a Label with
    /// text and no property, or declared no Label or TextConst and of a kind [`store_global_kind`] lists, a codeunit global
    /// again such a store. The probe stores hold Labels without a property only ("CKW Register" `Text000`, `Text001`). The measured stores nest three
    /// deep ("CKW Register", "CKW WMS", "CKW Tracking", "CKW UOM"); a deeper or cyclic nesting is no such store.
    fn kinds_store(&self, object: usize, depth: u8) -> bool {
        self.kinds_unpinned(object, depth).is_none()
    }

    /// The first part of codeunit `object`, read at nesting `depth`, that makes it no [`Session::kinds_store`]: `None` when
    /// it is one. A codeunit global (or array of them) that is no such store gives the part of that codeunit.
    fn kinds_unpinned(&self, object: usize, depth: u8) -> Option<Unpinned> {
        let o = &self.engine.program.objects[object];
        if depth > 3 {
            return Some(Unpinned::Deep { object, depth });
        }
        if !self.plain_codeunit(object) {
            return Some(Unpinned::NotPlain { object, depth });
        }
        if o.rec_slot.is_some() {
            return Some(Unpinned::TableNo { object, depth });
        }
        let nested = |c: u32| self.kinds_store(c as usize, depth + 1);
        let admitted = |g: &al_ir::Slot| g.plain_label || (!g.label && store_global_kind(&g.ty, &nested));
        let (slot, g) = (o.globals.iter().enumerate()).find(|(_, g)| !admitted(g))?;
        let inner = match &g.ty {
            RtType::Array(dims, e) if dims.len() == 1 => match **e {
                RtType::Codeunit(c) => Some(c),
                _ => None,
            },
            RtType::Codeunit(c) => Some(*c),
            _ => None,
        };
        match inner {
            Some(c) if !nested(c) => self.kinds_unpinned(c as usize, depth + 1),
            _ => Some(Unpinned::Global { object, slot, depth }),
        }
    }

    /// The first part of codeunit `object`, which is not SingleInstance and has a TableNo, that makes it no register of
    /// merged corpus PR 1596 ([`Store::TableNoKinds`], assumption clear-all-tableno-register): `None` when it is one.
    /// ClearSharedCodeunitTableNoRegister.al "CKT Reg" (:86-104) declares, besides `Rec`, records that are not temporary,
    /// codeunits, an Integer, a Boolean, a `Label 'text', Comment = 'text'` and a `Label 'text'`; a scalar kind of merged
    /// corpus PR 1587 ([`scalar_kind`]) is admitted too. Each codeunit global is a [`Session::register_nested`] codeunit.
    fn tableno_unpinned(&self, object: usize) -> Option<Unpinned> {
        let nested = |c: u32| self.register_nested(c as usize, object, 1, 4, true);
        let kind = |g: &al_ir::Slot| {
            g.plain_label
                || g.comment_label
                || !g.label && (matches!(g.ty, RtType::Record(_, false)) || scalar_kind(&g.ty))
        };
        self.tableno_globals_unpinned(object, &kind, &nested, &|c| self.kinds_unpinned(c, 1))
    }

    /// The first part of codeunit `object`, which is not SingleInstance and has a TableNo, that makes it no register of
    /// merged corpus PR 1599 ([`Store::TableNoSingle`], assumption clear-all-register-single-instance): `None` when it is
    /// one. ClearSharedCodeunitNestedSingleInstance.al "CKS R D1PU" (:58-65) declares, besides `Rec`, records that are not
    /// temporary, codeunits, an Integer and a Boolean, and every other register of the probe ("CKS R D1PW" :155 to "CKS R
    /// KU") declares the same kinds; the register level admits only these. Each codeunit global is a
    /// [`Session::single_nested_unpinned`] codeunit. A Label or another scalar kind of merged corpus PR 1587 at the
    /// register level refuses: no probe cell measures it beside a SingleInstance codeunit (al-runner issue 9176), unless
    /// `wide` ([`Store::TableNoWhse`]): then the register level also admits the kinds of [`whse_register_kind`].
    fn single_register_unpinned(&self, object: usize, wide: bool) -> Option<Unpinned> {
        let nested = |c: u32| self.single_nested_unpinned(c as usize, 1, wide).is_none();
        let kind = |g: &al_ir::Slot| {
            !g.label && matches!(g.ty, RtType::Record(_, false) | RtType::Integer | RtType::Boolean)
                || wide && whse_register_kind(g)
        };
        self.tableno_globals_unpinned(object, &kind, &nested, &|c| self.single_nested_unpinned(c, 1, wide))
    }

    /// The register-level part of [`Session::tableno_unpinned`] and [`Session::single_register_unpinned`]: the first
    /// global of codeunit `object` that is not `Rec`, a global `kind` admits or a codeunit `nested` admits. A codeunit
    /// global is named by the part `why` gives it, when it gives one (a diagnostic only).
    fn tableno_globals_unpinned(
        &self,
        object: usize,
        kind: &dyn Fn(&al_ir::Slot) -> bool,
        nested: &dyn Fn(u32) -> bool,
        why: &dyn Fn(usize) -> Option<Unpinned>,
    ) -> Option<Unpinned> {
        let o = &self.engine.program.objects[object];
        let admitted = |(i, g): &(usize, &al_ir::Slot)| {
            o.rec_slot == Some(*i)
                || match &g.ty {
                    RtType::Codeunit(c) if !g.label => nested(*c),
                    _ => kind(g),
                }
        };
        let (slot, g) = o.globals.iter().enumerate().find(|e| !admitted(e))?;
        let own = Unpinned::Global { object, slot, depth: 0 };
        Some(match &g.ty {
            RtType::Codeunit(c) if !g.label => why(*c as usize).unwrap_or(own),
            _ => own,
        })
    }

    /// The first part of codeunit `object`, held at nesting `depth` below a register of [`Session::single_register_unpinned`],
    /// that makes it no nested codeunit of merged corpus PR 1599 (ClearSharedCodeunitNestedSingleInstance.al): `None` when
    /// it is one. The measured codeunits sit one to three levels below the register (D1_* :1995, D2_* :2087, D3_* :2179,
    /// Chain_* :2240 "CKS Trk U" below "CKS WMS U") and have no TableNo. A SingleInstance one holds only the globals of
    /// [`Session::single_instance_unpinned`]. Any other holds Integers, records (temporary or not, "CKS Trk U" :1369-1371),
    /// `Label 'text', Comment = 'text'` and codeunits of this kind, and one level below the register also `Label 'text'`
    /// ("CKS WMS U" :1403), `Label 'text', Locked = true`, a List of Text and a Dictionary of [Text, Integer] ("CKS Nest U"
    /// :1683-1688, Kinds_* :2301, :2331). Three levels below the register 1599 has only SingleInstance codeunits ("CKS SI
    /// D3U" :1043), and merged corpus PR 1596 a codeunit of Integers (ClearSharedCodeunitTableNoRegister.al "CKT M3" :30;
    /// its codeunit global sits four levels deep, which refuses here): any other one holds Integers only, and a record or
    /// a Label there refuses (al-runner issue 9176), unless `wide`: corpus PR 1619 (merged, not pinned) (ClearSharedCodeunitWhseStore.al
    /// "CSW D3" :228-232 below "CSW D2P", Deep_OwnerClearAll :451, assumption clear-all-whse-register-store) measures a
    /// record that is not temporary and a `Label 'text', Comment = 'text'` there too.
    fn single_nested_unpinned(&self, object: usize, depth: u8, wide: bool) -> Option<Unpinned> {
        let o = &self.engine.program.objects[object];
        if depth > 3 {
            return Some(Unpinned::Deep { object, depth });
        }
        if o.kind != al_ir::ObjKind::Codeunit || o.rec_slot.is_some() {
            return Some(Unpinned::TableNo { object, depth });
        }
        if self.engine.object_facts[object].single_instance {
            return self.single_instance_unpinned(object, depth, wide);
        }
        let first = depth == 1;
        (o.globals.iter().enumerate()).find_map(|(slot, g)| {
            let own = Some(Unpinned::Global { object, slot, depth });
            let deepest = depth == 3;
            if g.label {
                let measured = g.comment_label && (!deepest || wide) || first && (g.plain_label || g.locked_label);
                return if measured { None } else { own };
            }
            match &g.ty {
                RtType::Integer => None,
                RtType::Record(..) if !deepest => None,
                RtType::Record(_, false) if wide => None,
                RtType::Codeunit(c) => self.single_nested_unpinned(*c as usize, depth + 1, wide),
                RtType::List(e) if first && matches!(**e, RtType::Text(_)) => None,
                RtType::Dictionary(k, v)
                    if first && matches!(**k, RtType::Text(_)) && matches!(**v, RtType::Integer) =>
                {
                    None
                }
                _ => own,
            }
        })
    }

    /// The first global of SingleInstance codeunit `object`, held at nesting `depth` below a register of
    /// [`Session::single_register_unpinned`], that ClearSharedCodeunitNestedSingleInstance.al does not measure: `None` when
    /// every global is an Integer, a temporary record, a `Label 'text', Comment = 'text'`, a `Label 'text', Locked = true`
    /// or a codeunit that is not SingleInstance, has no TableNo and holds only Integers ("CKS SI RU" :340-347 with "CKS
    /// Leaf" :20-21; "CKS SI PU" :37-40). Its state, these globals included, survives the owner's ClearAll (D1_Written_OwnerClearAll
    /// :2025, Chain_Written_OwnerClearAll :2270). When `wide` and two levels below the register, corpus PR 1619 (merged, not pinned)
    /// (assumption clear-all-whse-register-store) measures two more codeunit globals that keep their state too: a
    /// SingleInstance codeunit with no TableNo whose globals are Booleans and Integers ("CSW Impl" :14-20 in "CSW Confirm"
    /// :34-40, Whse_OwnerClearAll_EveryLevel :356, Whse_DirectSiWrite_OwnerClearAll :412) and a codeunit that is not
    /// SingleInstance, has no TableNo and holds records that are not temporary, Integers and `Label 'text', Comment =
    /// 'text'` ("CSW D3" :228-232 in "CSW D2Si" :246-252, Deep_OwnerClearAll :451).
    fn single_instance_unpinned(&self, object: usize, depth: u8, wide: bool) -> Option<Unpinned> {
        let objects = &self.engine.program.objects;
        let leaf_of = |c: usize, single: bool, kind: &dyn Fn(&al_ir::Slot) -> bool| {
            objects[c].kind == al_ir::ObjKind::Codeunit
                && self.engine.object_facts[c].single_instance == single
                && objects[c].rec_slot.is_none()
                && objects[c].globals.iter().all(kind)
        };
        let leaf = |c: usize| leaf_of(c, false, &|g| !g.label && matches!(g.ty, RtType::Integer));
        let whse = wide && depth == 2;
        let whse_leaf = |c: usize| {
            leaf_of(c, true, &|g| {
                !g.label && matches!(g.ty, RtType::Integer | RtType::Boolean)
            }) || leaf_of(c, false, &|g| {
                g.comment_label || !g.label && matches!(g.ty, RtType::Integer | RtType::Record(_, false))
            })
        };
        let (slot, _) = (objects[object].globals.iter().enumerate()).find(|(_, g)| {
            !(g.comment_label
                || g.locked_label
                || !g.label
                    && match g.ty {
                        RtType::Integer | RtType::Record(_, true) => true,
                        RtType::Codeunit(c) => leaf(c as usize) || whse && whse_leaf(c as usize),
                        _ => false,
                    })
        })?;
        Some(Unpinned::Global { object, slot, depth })
    }

    /// Whether codeunit `object`, held at nesting `depth` below the register `root`, is a nested codeunit the register of
    /// [`Session::tableno_unpinned`] may hold: a codeunit of the chain ClearSharedCodeunitTableNoRegister.al measures, `max`
    /// levels deep at most (four: "CKT M1" :66 to "CKT Leaf" :14, DeepReg_Primed_OwnerRun :767): not SingleInstance, no
    /// TableNo, every global an Integer or a codeunit of the same kind, or, one level below the register and when `back`, a
    /// codeunit global of the register itself ("CKT Cyc" :179-183, CycReg_Primed_OwnerRun :824).
    fn register_nested(&self, object: usize, root: usize, depth: u8, max: u8, back: bool) -> bool {
        let o = &self.engine.program.objects[object];
        depth <= max
            && self.plain_codeunit(object)
            && o.rec_slot.is_none()
            && o.globals.iter().all(|g| {
                !g.label
                    && match g.ty {
                        RtType::Integer => true,
                        RtType::Codeunit(c) => {
                            (back && c as usize == root && depth == 1)
                                || self.register_nested(c as usize, root, depth + 1, max, back)
                        }
                        _ => false,
                    }
            })
    }

    /// Whether codeunit `object`, `depth` levels below a register (0: the register, with a TableNo, not SingleInstance), is a
    /// node of the tree corpus PR 1620 (merged, not pinned) measures (ClearSharedCodeunitWhseTree.al, assumption clear-all-register-tree):
    /// at most three levels deep ("CWT S4" :29 and "CWT P4" :152 below "CWT S2" :520 and "CWT P3" :396), no TableNo below
    /// the register, SingleInstance or not, every global one of [`tree_kind`] or a codeunit global that is such a node
    /// one level deeper ("CWT Reg" :1023-1056, "CWT P1" :767-794, "CWT S1" :894).
    pub(crate) fn tree_node(&self, object: usize, depth: u8) -> bool {
        let o = &self.engine.program.objects[object];
        depth <= 3
            && o.kind == al_ir::ObjKind::Codeunit
            && (o.rec_slot.is_some() == (depth == 0))
            && (depth > 0 || self.plain_codeunit(object))
            && o.globals.iter().enumerate().all(|(i, g)| {
                o.rec_slot == Some(i)
                    || match g.ty {
                        RtType::Codeunit(c) if !g.label => self.tree_node(c as usize, depth + 1),
                        _ => tree_kind(g),
                    }
            })
    }

    /// The store shape of codeunit `store` held by `owner`: [`Session::store_kind`], else [`Store::Tree`] for the three
    /// owner and store pairs of corpus PR 1620 (merged, not pinned) ([`Session::tree_pair`]).
    fn store_kind_in(&self, owner: usize, store: usize) -> Option<Store> {
        self.store_kind(store)
            .or_else(|| self.tree_pair(owner, store).then_some(Store::Tree))
    }

    /// Whether `ClearAll()` in `owner` of its global of codeunit `store` is one of the three cells of corpus PR 1620 (merged, not pinned)
    /// (ClearSharedCodeunitWhseTree.al): the plain owner whose one global is the register ("CWT Owner" :1171-1174,
    /// Tree_OwnerClearAll_After :1362); the register over a node one level below it ("CWT Reg" holding "CWT P1" and
    /// "CWT S1" :1055-1056, Tree_RegisterClearAll :1780); a plain node that a register declares, over a node one level
    /// below it ("CWT P1" holding "CWT S2" and "CWT P2" :793-794, Tree_PlainHolderOfSiClearAll :1864).
    fn tree_pair(&self, owner: usize, store: usize) -> bool {
        let objects = &self.engine.program.objects;
        let declares = |holder: usize, held: usize| {
            (objects[holder].globals.iter()).any(|g| matches!(g.ty, RtType::Codeunit(c) if c as usize == held))
        };
        match (self.owner_kind(owner), objects[store].rec_slot.is_some()) {
            (Some(Owner::Plain), true) => {
                matches!(objects[owner].globals.as_slice(), [g] if matches!(g.ty, RtType::Codeunit(c) if c as usize == store))
                    && self.tree_node(store, 0)
            }
            (Some(Owner::TableNo), false) => self.tree_node(owner, 0) && self.tree_node(store, 1),
            (Some(Owner::Plain), false) => {
                self.tree_node(owner, 1)
                    && self.tree_node(store, 2)
                    && (0..objects.len())
                        .any(|r| objects[r].rec_slot.is_some() && declares(r, owner) && self.tree_node(r, 0))
            }
            _ => false,
        }
    }

    /// Whether TableNo register `object` ([`Store::TableNoKinds`]) has the shape the plain owner is measured with:
    /// ClearSharedCodeunitTableNoRegister.al "CKT Reg" (:95-96, "CKT M3" and "CKT M2"), nested codeunits three levels
    /// deep at most and none holding the register (TableNoReg_Primed_PlainOwner :725 to TableNoReg_RunBeforeShare_PlainOwner
    /// :753). The deeper chain and the cycle are measured under the owner with a TableNo only.
    fn plain_owner_register(&self, object: usize) -> bool {
        (self.engine.program.objects[object].globals.iter())
            .all(|g| !matches!(g.ty, RtType::Codeunit(c) if !self.register_nested(c as usize, object, 1, 3, false)))
    }

    /// The refusal of `ClearAll()` in `owner` of store instance `v`, which another variable shares and which
    /// [`Session::measured_cell`] does not measure: the common text, then the store, the owner and the first reason the
    /// cell is not measured (a diagnostic only, ADR-009; which shapes refuse is `measured_cell`'s).
    fn shared_store_refusal(&self, owner: usize, v: &Value, holding: Holding, hold: TempHold) -> Signal {
        let objects = &self.engine.program.objects;
        let Value::Codeunit(rc) = v else {
            unreachable!("a store global holds a codeunit")
        };
        let store = rc.borrow().object as usize;
        let why = match (self.owner_kind(owner), self.store_kind_in(owner, store)) {
            (None, _) => "the owner is an object whose ClearAll the corpus does not measure".to_string(),
            (_, None) => match (objects[store].rec_slot, self.kinds_unpinned(store, 0)) {
                (Some(_), _) if self.plain_codeunit(store) => match self.tableno_unpinned(store) {
                    // A register with a SingleInstance codeunit is named by what 1599 does not measure in it.
                    Some(Unpinned::NotPlain { .. }) => (self.single_register_unpinned(store, true))
                        .map_or("no measured store shape".to_string(), |u| u.describe(objects)),
                    Some(u) => u.describe(objects),
                    None => "no measured store shape".to_string(),
                },
                (_, u) => match u {
                    Some(u) => u.describe(objects),
                    None => "no measured store shape".to_string(),
                },
            },
            (Some(o), Some(s)) if !measured(o, s, holding, hold) => format!(
                "an instance {} held by {} is not measured for {} store",
                holding.describe(),
                o.describe(),
                s.describe()
            ),
            (_, Some(Store::Register | Store::TableNoKinds | Store::TableNoSingle | Store::TableNoWhse)) if !self.register_as_measured(rc) => {
                "the register or a codeunit nested in it read or wrote a row, another variable shares one of its nested \
                 codeunits, Lists or Dictionaries, or a back link holds another register instance"
                    .to_string()
            }
            (_, Some(Store::Tree)) if !self.register_as_measured(rc) || !self.tree_as_measured(rc, 0) => {
                "a codeunit of the tree read a row, another variable shares one of its nested codeunits, Lists, \
                 Dictionaries, RecordRefs, FieldRefs, KeyRefs or TextBuilders, or a temporary record read a row"
                    .to_string()
            }
            (Some(Owner::Plain), Some(Store::TableNoKinds)) if !self.plain_owner_register(store) => {
                "the plain owner is measured with nested codeunits three levels deep at most and no back link"
                    .to_string()
            }
            (_, Some(Store::Kinds)) if !self.kinds_instances(rc, 0) => {
                "an Interface global holds an instance of a codeunit that is no store of the measured kinds".to_string()
            }
            _ if self.holds_bound_codeunit(v) => "the store holds a codeunit bound by BindSubscription".to_string(),
            _ if self.store_runs(v) => "a routine of the store runs".to_string(),
            _ => "the store implements an interface, or its table is shared without rows".to_string(),
        };
        Signal::unsupported(format!(
            "ClearAll of a global codeunit variable whose instance another variable shares, in a shape the corpus does not \
             pin: store \"{}\" (owner \"{}\"): {why}",
            objects[store].name, objects[owner].name
        ))
    }

    /// Whether every codeunit instance kinds store instance `rc` reaches through an Interface global (or a codeunit or an
    /// array of codeunits that holds one) is itself of a [`Session::kinds_store`] codeunit: the instance an Interface holds
    /// is reset in place (ClearSharedCodeunitGlobalKinds.al ClearAll_SetterVar_Codeunits :799 'face' 0 and :810 'hf' 0, the
    /// "CKK Leaf" instance), measured for such an instance only.
    fn kinds_instances(&self, rc: &Rc<RefCell<Instance>>, depth: u8) -> bool {
        let inst = rc.borrow();
        let slots = &self.engine.program.objects[inst.object as usize].globals;
        depth <= 3
            && (inst.globals.iter().zip(slots)).all(|(v, slot)| match (v, &slot.ty) {
                (Value::Codeunit(inner), RtType::Interface(_)) => {
                    self.kinds_store(inner.borrow().object as usize, depth + 1)
                        && self.kinds_instances(inner, depth + 1)
                }
                (Value::Codeunit(inner), RtType::Codeunit(_)) => self.kinds_instances(inner, depth + 1),
                (Value::Array(items), RtType::Array(_, e)) if matches!(**e, RtType::Codeunit(_)) => {
                    (items.borrow().iter())
                        .all(|e| !matches!(e, Value::Codeunit(i) if !self.kinds_instances(i, depth + 1)))
                }
                _ => true,
            })
    }

    /// Whether `ClearAll()` in `owner` resets store instance `v`, held as `holding` says, in place: the instance is not bound
    /// by BindSubscription (`clear_all_bound_globals` owns those), no routine of its codeunit runs, and (owner, store,
    /// holding) is a cell the corpus measures.
    fn measured_cell(&self, owner: usize, v: &Value, holding: Holding, hold: TempHold) -> bool {
        let Value::Codeunit(rc) = v else { return false };
        let (Some(o), Some(s)) = (
            self.owner_kind(owner),
            self.store_kind_in(owner, rc.borrow().object as usize),
        ) else {
            return false;
        };
        measured(o, s, holding, hold)
            && (!matches!(s, Store::Register | Store::TableNoKinds | Store::TableNoSingle | Store::TableNoWhse | Store::Tree) || self.register_as_measured(rc))
            && (s != Store::Tree || self.tree_as_measured(rc, 0))
            && ((o, s) != (Owner::Plain, Store::TableNoKinds) || self.plain_owner_register(rc.borrow().object as usize))
            && (s != Store::Kinds || self.kinds_instances(rc, 0))
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
    /// register and the price store wrote before the ClearAll. For a `kinds` store a global counts only by what
    /// [`kinds_written`] reads. Records and arrays of records are left to [`Session::temp_hold`].
    fn wrote_scalar(&mut self, rc: &Rc<RefCell<Instance>>, depth: u8, kinds: bool) -> bool {
        if depth > 8 || !rc.borrow().initialized {
            return false;
        }
        let engine = self.engine.clone();
        let object = rc.borrow().object as usize;
        let slots = &engine.program.objects[object].globals;
        let fresh = self.slot_defaults(slots);
        let held = rc.borrow().globals.clone();
        (held.iter().zip(&fresh).zip(slots)).any(|((v, f), slot)| match (v, &slot.ty) {
            (Value::Codeunit(inner), _) => self.wrote_scalar(inner, depth + 1, kinds),
            (_, RtType::Record(..)) => false,
            (_, RtType::Array(_, e)) if matches!(**e, RtType::Record(..)) => false,
            _ if kinds => kinds_written(v, f, &engine),
            _ => crate::single_track::fp(v, &engine) != crate::single_track::fp(f, &engine),
        })
    }

    /// Whether register instance `rc` ([`Store::Register`]) holds only what ClearSharedCodeunitSetterOwner.al measures before
    /// the ClearAll (`Mark` :62-70): its record that is not temporary has a filter and field values but read or wrote no row
    /// (`state_stamp::no_row_io`), and its nested codeunit instance no other holder shares. The same for a
    /// [`Store::TableNoKinds`] instance: ClearSharedCodeunitTableNoRegister.al "CKT Reg" `Prime` (:107-118) sets filters and
    /// field values of its records and reads or writes no row, and only the register holds its nested instances
    /// ([`Session::nested_as_measured`]).
    fn register_as_measured(&self, rc: &Rc<RefCell<Instance>>) -> bool {
        rc.borrow().globals.iter().all(|g| match g {
            Value::Record(r) => no_row_read(r),
            Value::Codeunit(n) => Rc::strong_count(n) == 1 && self.nested_as_measured(rc, n, 1),
            _ => true,
        })
    }

    /// Whether nested instance `n`, at `depth` below store instance `store`, holds only what
    /// ClearSharedCodeunitTableNoRegister.al measures: every codeunit instance it holds, at every depth, held by it alone
    /// ("CKT Reg" and "CKT Deep Reg" `Prime`, :107-118, :154-159), except, one level below the store, a global of the
    /// store's codeunit, which holds the store instance itself ("CKT Cyc" `Link` :185-189 from "CKT Cyc Reg" `Prime` :211-216, CycReg_Primed_OwnerRun :824)
    /// or an instance nothing made yet and nothing else holds. A record it holds read or wrote no row, and only it holds a
    /// List or Dictionary: the codeunits of ClearSharedCodeunitNestedSingleInstance.al (merged corpus PR 1599) set field
    /// values, insert into temporary records and add to their own Lists ("CKS Trk U" `SetN` :1377-1385, "CKS Nest U"
    /// :1691-1698). A SingleInstance codeunit global holds an instance that calls do not run on (the session's one
    /// instance is the one they run on), so nothing more is read below it. The register of [`Store::Register`] holds a
    /// nested codeunit with no codeunit global ("CLX Store"), so this reads nothing more for it.
    fn nested_as_measured(&self, store: &Rc<RefCell<Instance>>, n: &Rc<RefCell<Instance>>, depth: u8) -> bool {
        let object = store.borrow().object;
        depth <= 4
            && n.borrow().globals.iter().all(|g| match g {
                Value::Codeunit(m) if depth == 1 && Rc::ptr_eq(m, store) => true,
                Value::Codeunit(m) if depth == 1 && m.borrow().object == object => {
                    Rc::strong_count(m) == 1 && !m.borrow().initialized
                }
                Value::Codeunit(m) => Rc::strong_count(m) == 1 && self.nested_as_measured(store, m, depth + 1),
                Value::Record(r) => no_row_read(r),
                Value::List(l) => Rc::strong_count(l) == 1,
                Value::Dictionary(d) => Rc::strong_count(d) == 1,
                _ => true,
            })
    }

    /// Whether tree instance `rc` ([`Store::Tree`]), `depth` levels below the reset one, and every instance that is not
    /// SingleInstance it reaches through codeunits that are not SingleInstance, hold only what ClearSharedCodeunitWhseTree.al
    /// (corpus PR 1620, merged, not pinned) measures before the ClearAll: its nodes only insert into their temporary records,
    /// only open their RecordRef ("CWT Reg" `SetOwn` :1093-1103), and hand out no List, Dictionary, RecordRef, FieldRef, KeyRef
    /// or TextBuilder. So a temporary record read no row into its buffer and modified none, a RecordRef read or wrote no
    /// row (`state_stamp::no_row_io`), and only the instance holds such a value. A SingleInstance codeunit keeps all its
    /// state ([`Session::reset_once`]), so nothing is read below it. Assumption clear-all-register-tree.
    fn tree_as_measured(&self, rc: &Rc<RefCell<Instance>>, depth: u8) -> bool {
        if !self.plain_codeunit(rc.borrow().object as usize) {
            return true;
        }
        depth <= 3
            && rc.borrow().globals.iter().all(|g| match g {
                Value::Codeunit(n) => self.tree_as_measured(n, depth + 1),
                Value::Record(r) => r.try_borrow().is_ok_and(|r| !r.temporary || temp_inserted_only(&r)),
                Value::List(l) => Rc::strong_count(l) == 1,
                Value::Dictionary(d) => Rc::strong_count(d) == 1,
                Value::Library(_, l) => {
                    Rc::strong_count(l) == 1
                        && l.try_borrow().is_ok_and(|l| match &*l {
                            LibraryValue::RecordRef(r) => {
                                r.try_borrow().is_ok_and(|r| crate::state_stamp::no_row_io(&r))
                            }
                            _ => true,
                        })
                }
                _ => true,
            })
    }

    /// How many codeunit globals of the instances store instance `rc` holds directly hold `rc` itself: the back links of
    /// a cycle ("CKT Cyc", ClearSharedCodeunitTableNoRegister.al :179-183), each one more holder of `rc`.
    fn back_links(&self, rc: &Rc<RefCell<Instance>>) -> usize {
        let inst = rc.borrow();
        (inst.globals.iter())
            .filter_map(|g| match g {
                Value::Codeunit(n) if !Rc::ptr_eq(n, rc) => Some(n),
                _ => None,
            })
            .map(|n| {
                (n.borrow().globals.iter())
                    .filter(|m| matches!(m, Value::Codeunit(m) if Rc::ptr_eq(m, rc)))
                    .count()
            })
            .sum()
    }

    /// Whether store instance `rc` is of a store whose globals [`kinds_written`] reads ([`Store::Kinds`],
    /// [`Store::TableNoKinds`]).
    fn kinds_shape(&self, rc: &Rc<RefCell<Instance>>) -> bool {
        matches!(
            self.store_kind(rc.borrow().object as usize),
            Some(Store::Kinds | Store::TableNoKinds | Store::TableNoSingle | Store::TableNoWhse)
        )
    }

    /// Resets instance `rc` in place, as `ClearAll()` in a measured owner does to a store global: its globals go back to
    /// their declared values, a temporary record keeping its table (`keep_temp_table`), so its rows stay
    /// (ClearSharedCodeunitSIOwner.al A3_Step3_ClearAllInOwner :447, rows=1 ref11=0 int1=0); the store a
    /// [`Store::Wrapper`] holds is reset in place the same way and kept, so its alias sees the reset
    /// (ClearSharedCodeunitOwnerKinds.al Deep_StoreAliasWroteBefore :723). An instance nothing made yet stays as it is. The
    /// caller marks assumption clear-shared-codeunit.
    fn reset_keeping_rows(&mut self, rc: &Rc<RefCell<Instance>>) {
        self.reset_once(rc, &mut Vec::new());
    }

    /// [`Session::reset_keeping_rows`], each instance reset once (`seen`): a codeunit global that holds an instance being
    /// reset already keeps it as it is, so the back link of a cycle stays (ClearSharedCodeunitTableNoRegister.al
    /// CycReg_Primed_OwnerRun :829-832, 'cyc=0/1': the nested Integer reset, the caller reads the register through the link;
    /// assumption clear-all-tableno-register).
    fn reset_once(&mut self, rc: &Rc<RefCell<Instance>>, seen: &mut Vec<*const RefCell<Instance>>) {
        // A SingleInstance codeunit keeps its state and stays the session's one instance
        // (ClearSharedCodeunitNestedSingleInstance.al D1_Written_OwnerClearAll :2025, D3_Written_OwnerClearAll :2209,
        // Chain_Written_OwnerClearAll :2270; assumption clear-all-register-single-instance).
        if !rc.borrow().initialized
            || seen.contains(&Rc::as_ptr(rc))
            || !self.plain_codeunit(rc.borrow().object as usize)
        {
            return;
        }
        seen.push(Rc::as_ptr(rc));
        let engine = self.engine.clone();
        let object = rc.borrow().object as usize;
        let slots = &engine.program.objects[object].globals;
        let mut fresh = self.slot_defaults(slots);
        let old = rc.borrow().globals.clone();
        for ((old, new), slot) in old.iter().zip(fresh.iter_mut()).zip(slots) {
            match (old, &slot.ty) {
                (Value::Codeunit(inner), RtType::Codeunit(_) | RtType::Interface(_)) => {
                    self.reset_once(inner, seen);
                    *new = old.clone();
                }
                // Each element of an array of codeunits is reset in place and the variable that shares it sees the reset
                // (ClearSharedCodeunitGlobalKinds.al ClearAll_SetterVar_Codeunits :801-802 'arr1' 'arr2' 0, :809 'ha' 0).
                (Value::Array(items), RtType::Array(_, e)) if matches!(**e, RtType::Codeunit(_)) => {
                    for e in items.borrow().iter() {
                        if let Value::Codeunit(inner) = e {
                            self.reset_once(inner, seen);
                        }
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

/// The refusal of a `ClearAll` that resets a temporary table a record outside the variable shares, in a cell no probe
/// measures: a `Record.Copy(x, true)` target outside the measured cells, or any other sharer ([`TempHold::SharedOther`]).
/// Whether record `r`, held by a register or a codeunit nested in it, is temporary or read or wrote no row
/// (`state_stamp::no_row_io`): what the registers of ClearSharedCodeunitSetterOwner.al, ClearSharedCodeunitTableNoRegister.al
/// and ClearSharedCodeunitNestedSingleInstance.al hold before the ClearAll ([`Session::register_as_measured`]).
fn no_row_read(r: &Rc<RefCell<RecordState>>) -> bool {
    r.try_borrow()
        .is_ok_and(|r| r.temporary || crate::state_stamp::no_row_io(&r))
}

/// Whether temporary record `r` read no row into its buffer and modified none: it only inserted rows, as the temporary
/// records of ClearSharedCodeunitWhseTree.al do ("CWT Reg" `SetOwn` :1093-1095; [`Session::tree_as_measured`]).
fn temp_inserted_only(r: &RecordState) -> bool {
    r.load.buffer.is_none() && r.position.is_none() && r.last_modify.is_none() && r.failed_insert.is_none()
}

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
        // Merged corpus PR 1587 (assumption clear-all-store-global-kinds). ClearSharedCodeunitGlobalKinds.al: the plain owner
        // "CKK Owner" with the store written (ClearAll_SetterVar_* :707 to ClearAll_OwnerMadeGetterVar_* :1251), the
        // owner with a TableNo "CKK TNo Owner" (ClearAll_TableNoOwnerRun_* :1275 to :1393); ClearSharedCodeunitLateBinding.al,
        // the plain owner. ClearSharedCodeunitWhseRegister.al, the owner with a TableNo: the register written
        // (WhseRegister_Primed_Run :329, WhseRegister_OneEntry_Run :350, WhseRegister_Primed_DirectCall :384,
        // WhseRegister_Primed_NestedHandle :405) and never
        // called (WhseRegister_NeverUsed_Run :363). A store a call made but that wrote nothing is not measured: refused.
        (Plain, Kinds, Shared) | (TableNo, Kinds, Shared | SharedUncalled) => true,
        // Merged corpus PR 1596 (assumption clear-all-tableno-register), ClearSharedCodeunitTableNoRegister.al: the owner
        // with a TableNo, the register written (TableNoReg_Primed_OwnerRun :654, TableNoReg_RunBeforeShare_OwnerRun :697,
        // TableNoReg_RunThenPrimed_OwnerRun :711, DeepReg_Primed_OwnerRun :767, CycReg_Primed_OwnerRun :824), only read
        // (TableNoReg_Peeked_OwnerRun :668, DeepReg_Peeked_OwnerRun :781, CycReg_Peeked_OwnerRun :838) and never called
        // (TableNoReg_NeverCalled_OwnerRun :683, DeepReg_NeverCalled_OwnerRun :796, CycReg_NeverCalled_OwnerRun :853); the
        // plain owner, written (TableNoReg_Primed_PlainOwner :725, TableNoReg_RunBeforeShare_PlainOwner :753) and never
        // called (TableNoReg_NeverCalled_PlainOwner :739). Only read under the plain owner is not measured: refused.
        (TableNo, TableNoKinds, Shared | SharedUnwritten | SharedUncalled)
        | (Plain, TableNoKinds, Shared | SharedUncalled) => true,
        // Merged corpus PR 1599 (assumption clear-all-register-single-instance), ClearSharedCodeunitNestedSingleInstance.al:
        // the owner with a TableNo only, the register written (D1P_Written_OwnerClearAll :1933, D1_Written_OwnerClearAll
        // :2025, D2_Written_OwnerClearAll :2117, D3_Written_OwnerClearAll :2209, Chain_Written_OwnerClearAll :2270,
        // Kinds_Written_OwnerClearAll :2331) and never called (the *_Untouched_* cells, D1P :1903 to Kinds :2301, and the
        // *_WrittenByOtherVar_* cells :1964, :2056, :2148, which write the SingleInstance codeunit through another variable).
        // Only read and the plain owner are not measured: refused.
        (TableNo, TableNoSingle, Shared | SharedUncalled) => true,
        // Corpus PR 1619 (merged, not pinned) (assumption clear-all-whse-register-store), ClearSharedCodeunitWhseStore.al: the owner with a
        // TableNo, the register written before the ClearAll (Whse_OwnerClearAll_EveryLevel :356,
        // Whse_OwnerPrimes_OwnerClearAll :386, Whse_DirectSiWrite_OwnerClearAll :412, Kinds_OwnerClearAll :431,
        // Deep_OwnerClearAll :451). Never called, only read and the plain owner are not measured: refused.
        (TableNo, TableNoWhse, Shared) => true,
        // Corpus PR 1620 (merged, not pinned) (assumption clear-all-register-tree), ClearSharedCodeunitWhseTree.al: the register "CWT Reg"
        // (:1023-1056) held by the plain owner "CWT Owner" (:1171-1174) alone, holding temporary rows; the owner's ClearAll
        // (Tree_OwnerClearAll_After :1362), the register's own (Tree_RegisterClearAll :1780) and P1's (Tree_PlainHolderOfSiClearAll
        // :1864). Which owner goes with which store is [`Session::tree_pair`]'s. Shared and never called: refused.
        (Plain | TableNo, Tree, Written) => true,
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

/// Whether `t` is a global kind of the stores of merged corpus PR 1587 ([`Session::kinds_store`]): the scalars of
/// ClearSharedCodeunitGlobalKinds.al "CKK Store" :107-122 (ClearAll_*_Scalars), a record temporary or not :123-124
/// (ClearAll_*_Records), RecordRef, FieldRef and KeyRef :125-129 (ClearAll_*_Refs), a codeunit `store` accepts, an
/// Interface, a Variant and an array of one dimension of such codeunits :130-134 (ClearAll_*_Codeunits), and the
/// collections :135-144 (ClearAll_*_Collections, an array of one dimension of Integer or Text).
fn store_global_kind(t: &RtType, store: &dyn Fn(u32) -> bool) -> bool {
    use al_builtins::library_kind::LibraryKind as L;
    let key = |t: &RtType| matches!(t, RtType::Integer | RtType::Text(_));
    match t {
        t if scalar_kind(t) => true,
        RtType::Record(..) | RtType::Interface(_) | RtType::Variant => true,
        RtType::Library(k) => matches!(
            k,
            L::RecordRef | L::FieldRef | L::KeyRef | L::JsonObject | L::JsonArray | L::TextBuilder
        ),
        RtType::Codeunit(c) => store(*c),
        RtType::Array(dims, e) if dims.len() == 1 => match &**e {
            RtType::Codeunit(c) => store(*c),
            e => key(e),
        },
        RtType::List(e) => key(e) || matches!(**e, RtType::Interface(_)),
        RtType::Dictionary(k, v) => {
            matches!(**k, RtType::Integer) && matches!(**v, RtType::Text(_) | RtType::Interface(_))
        }
        _ => false,
    }
}

/// Whether register global `g` is one more kind corpus PR 1619 (merged, not pinned) (assumption clear-all-whse-register-store) measures at
/// the level of a [`Store::TableNoWhse`] register: ClearSharedCodeunitWhseStore.al "CSW Reg" :82-83 (a `Label 'text',
/// Comment = 'text'` and a `Label 'text'` beside a codeunit that holds a SingleInstance codeunit) and "CSW KReg" :165-176
/// (Text, Text[30], Code[20], Decimal, Date, BigInteger, Option, Char and the three Labels beside a SingleInstance codeunit,
/// Kinds_OwnerClearAll :431): the scalars go back to their defaults, the Labels keep their text.
fn whse_register_kind(g: &al_ir::Slot) -> bool {
    g.comment_label
        || g.plain_label
        || g.locked_label
        || !g.label
            && matches!(
                g.ty,
                RtType::Text(_)
                    | RtType::Code(_)
                    | RtType::Decimal
                    | RtType::Date
                    | RtType::BigInteger
                    | RtType::Option(_)
                    | RtType::Char
            )
}

/// Whether global `g` is of a kind every node of ClearSharedCodeunitWhseTree.al (corpus PR 1620 (merged, not pinned)) declares ("CWT Reg"
/// :1031-1054): a record, temporary or not, Boolean, Integer, Decimal, BigInteger, Text, Code, Date, DateTime, Option,
/// Enum, a List of Integer, a Dictionary of [Integer, Text], TextBuilder, Guid, RecordRef, FieldRef, KeyRef, or a Label
/// (plain, Comment, Locked, MaxLength :1051-1054; AL code cannot assign one, corpus must-not-compile 0350, 2628).
fn tree_kind(g: &al_ir::Slot) -> bool {
    use al_builtins::library_kind::LibraryKind as K;
    g.label
        || match &g.ty {
            RtType::List(e) => matches!(**e, RtType::Integer),
            RtType::Dictionary(k, v) => matches!((&**k, &**v), (RtType::Integer, RtType::Text(_))),
            RtType::Library(k) => matches!(k, K::RecordRef | K::FieldRef | K::KeyRef | K::TextBuilder),
            t => matches!(
                t,
                RtType::Record(..)
                    | RtType::Boolean
                    | RtType::Integer
                    | RtType::Decimal
                    | RtType::BigInteger
                    | RtType::Text(_)
                    | RtType::Code(_)
                    | RtType::Date
                    | RtType::DateTime
                    | RtType::Option(_)
                    | RtType::Enum(_)
                    | RtType::Guid
            ),
        }
}

/// Whether `t` is one of the scalars of ClearSharedCodeunitGlobalKinds.al "CKK Store" :107-122 (merged corpus PR 1587,
/// ClearAll_*_Scalars).
fn scalar_kind(t: &RtType) -> bool {
    matches!(
        t,
        RtType::Integer
            | RtType::BigInteger
            | RtType::Decimal
            | RtType::Boolean
            | RtType::Text(_)
            | RtType::Code(_)
            | RtType::Date
            | RtType::Time
            | RtType::DateTime
            | RtType::Guid
            | RtType::Option(_)
            | RtType::Enum(_)
            | RtType::Duration
            | RtType::Char
    )
}

/// Whether global `v` of a [`Store::Kinds`] store holds a write AL can read, against `fresh`, its declared value: a scalar
/// by its value fingerprint, an open RecordRef, a FieldRef or KeyRef that points at a field or key, a JsonObject or
/// JsonArray with a member, a TextBuilder with text. A value without a comparable form ([`crate::single_track::Fp::Opaque`])
/// is no proof of a write: such a store a call made but that wrote nothing is not measured (assumption
/// clear-all-store-global-kinds), so it refuses as main does.
fn kinds_written(v: &Value, fresh: &Value, engine: &crate::Engine) -> bool {
    use crate::single_track::{Fp, fp};
    use al_builtins::json::JsonNode;
    use al_builtins::library_kind::LibraryKind as L;
    use al_builtins::value::LibraryValue as V;
    if let Value::Library(k, l) = v {
        let Ok(l) = l.try_borrow() else { return false };
        return match (k, &*l) {
            (L::RecordRef, V::RecordRef(_))
            | (L::FieldRef, V::FieldRef(..) | V::KeyFieldRef(..))
            | (L::KeyRef, V::KeyRef(..)) => true,
            (L::JsonObject | L::JsonArray, V::Json(c)) => {
                c.unpinned.is_none()
                    && match &c.node {
                        JsonNode::Object(members) => !members.is_empty(),
                        JsonNode::Array(items) => !items.is_empty(),
                        JsonNode::Scalar(_) => false,
                    }
            }
            (L::TextBuilder, V::Text(t) | V::TextCap(t, _)) => !t.is_empty(),
            _ => false,
        };
    }
    matches!((fp(v, engine), fp(fresh, engine)), (Fp::Val(a), Fp::Val(b)) if a != b)
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
    /// A store of the global kinds of merged corpus PR 1587 ([`Session::kinds_store`], assumption
    /// clear-all-store-global-kinds) that no shape above names.
    Kinds,
    /// A codeunit with a TableNo of the register kinds of merged corpus PR 1596 ([`Session::tableno_unpinned`], assumption
    /// clear-all-tableno-register) that is no [`Store::Register`].
    TableNoKinds,
    /// A codeunit with a TableNo of the register kinds whose nested codeunits are those of merged corpus PR 1599
    /// ([`Session::single_register_unpinned`], assumption clear-all-register-single-instance), SingleInstance ones among
    /// them, that is no [`Store::TableNoKinds`].
    TableNoSingle,
    /// A codeunit with a TableNo of [`Session::single_register_unpinned`] when `wide` (corpus PR 1619 (merged, not pinned), assumption
    /// clear-all-whse-register-store, the shape of "Whse. Jnl.-Register Line" with "Confirm Management") that is no
    /// [`Store::TableNoSingle`].
    TableNoWhse,
    /// A codeunit of the tree of corpus PR 1620 (merged, not pinned) (assumption clear-all-register-tree) that is no other store, held by an
    /// owner of a measured pair ([`Session::tree_pair`]); [`Session::store_kind_in`] alone gives it.
    Tree,
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

impl Owner {
    /// The owner, as a refusal names it ([`Session::shared_store_refusal`]).
    fn describe(self) -> &'static str {
        match self {
            Owner::Plain => "a plain codeunit",
            Owner::SingleInstance => "a SingleInstance codeunit",
            Owner::Test => "a Test codeunit",
            Owner::Normal => "a Subtype Normal codeunit",
            Owner::TableNo => "a TableNo codeunit",
            Owner::Report => "a report",
        }
    }
}

impl Store {
    /// The store, as a refusal names it ([`Session::shared_store_refusal`]).
    fn describe(self) -> &'static str {
        match self {
            Store::Integer => "an Integer",
            Store::TempRef => "a temporary record and RecordRef",
            Store::TempRefLabel => "a temporary record, RecordRef and Label",
            Store::TempRefPlus => "a temporary record, RecordRef and Integer",
            Store::Wrapper => "a wrapper",
            Store::TempArray => "a temporary record array",
            Store::TwoDimArray => "a two-dimensional temporary record array",
            Store::Register => "a register",
            Store::PriceList => "a price list",
            Store::Kinds => "a global kinds",
            Store::TableNoKinds => "a TableNo register",
            Store::TableNoSingle => "a TableNo register with nested SingleInstance codeunits",
            Store::TableNoWhse => "a TableNo register in the shape of Whse. Jnl.-Register Line",
            Store::Tree => "a codeunit of the Whse. Jnl.-Register Line tree",
        }
    }
}

impl Holding {
    /// The holding, as a refusal names it ([`Session::shared_store_refusal`]).
    fn describe(self) -> &'static str {
        match self {
            Holding::Shared => "shared, called and written",
            Holding::SharedUnwritten => "shared, called but nothing written",
            Holding::SharedUncalled => "shared and never called",
            Holding::Written => "not shared, with rows",
        }
    }
}

/// The first part of a codeunit that makes it no store of the measured global kinds ([`Session::kinds_unpinned`]), with the
/// nesting depth it was read at (0: the store itself).
enum Unpinned {
    /// Read deeper than the three levels the probe nests: a deeper or cyclic nesting.
    Deep { object: usize, depth: u8 },
    /// A SingleInstance codeunit.
    NotPlain { object: usize, depth: u8 },
    /// A codeunit with a TableNo that is not the register shape.
    TableNo { object: usize, depth: u8 },
    /// A global `slot` of a kind [`store_global_kind`] does not list.
    Global { object: usize, slot: usize, depth: u8 },
}

impl Unpinned {
    /// The part, as a refusal names it ([`Session::shared_store_refusal`]).
    fn describe(&self, objects: &al_ir::Objects) -> String {
        match *self {
            Unpinned::Deep { object, depth } => format!(
                "codeunit \"{}\" nests deeper than 3 codeunits (depth {depth}), or holds itself",
                objects[object].name
            ),
            Unpinned::NotPlain { object, depth } => {
                format!(
                    "codeunit \"{}\" is SingleInstance (depth {depth})",
                    objects[object].name
                )
            }
            Unpinned::TableNo { object, depth } => format!(
                "codeunit \"{}\" has a TableNo and is not the measured register (depth {depth})",
                objects[object].name
            ),
            Unpinned::Global { object, slot, depth } => {
                let g = &objects[object].globals[slot];
                let dims = match &g.ty {
                    RtType::Array(d, _) if d.len() > 1 => format!(" of {} dimensions", d.len()),
                    _ => String::new(),
                };
                let kind = match g.label {
                    true => "Label with a property".to_string(),
                    false => al_builtins::describe::Describe::describe(&g.ty).to_string(),
                };
                format!(
                    "global \"{}\" of \"{}\" is a {kind}{dims}, a kind the corpus does not pin (depth {depth})",
                    g.name, objects[object].name
                )
            }
        }
    }
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
