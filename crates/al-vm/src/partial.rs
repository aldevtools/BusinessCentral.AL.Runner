//! Partial records: `SetLoadFields`, `AddLoadFields`, `AreFieldsLoaded`.
//!
//! Reading a field that was left out of the load set returns its stored value, so the only thing
//! AL can observe is `AreFieldsLoaded` (record/TestRecordPartialLoad.al). The model, each rule
//! pinned by the corpus test named beside it:
//!
//! - `SetLoadFields(f...)` sets the request; with no arguments it resets to the default, a full
//!   load (:61-99). `AddLoadFields` adds to a request that exists.
//! - A record that fetched nothing reports nothing loaded, the primary key included (:102-141).
//! - A fetch loads the request, the fields of the current key and the fields of every active
//!   filter (:193-285).
//! - Reading an omitted field loads it (:144-190).
//! - The loaded set only grows until `Clear`: a narrower request after a full fetch keeps the
//!   fields in hand, even across a re-fetch of the row, and a wider one loads nothing until the
//!   next fetch (record/TestRecordPartialLoadNarrowAfterFetch.al:56-185).
//!
//! What the corpus does not pin is refused when asked about: a primary key field under a secondary
//! current key, a field of a row other than the one the buffer held, a field assigned without
//! being loaded, a record written but never fetched, a field that is not stored.

use crate::Session;
use crate::records::Rec;
use al_builtins::describe::Describe;
use al_builtins::native::Signal;
use al_builtins::record_op::RecordOp;
use al_builtins::value::{Buffer, FieldSet, Value};
use al_data::{Store, TableDef};

fn unsupported<T>(what: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(what.into()))
}

fn positions(args: &[Value]) -> Result<Vec<usize>, Signal> {
    let mut out: Vec<usize> = Vec::new();
    for a in args {
        match a {
            Value::Integer(p) => {
                let p = *p as usize;
                if !out.contains(&p) {
                    out.push(p);
                }
            }
            other => return unsupported(format!("field argument {}", other.describe())),
        }
    }
    Ok(out)
}

/// The primary key of a row, as a comparable text.
pub(crate) fn row_id(t: &TableDef, row: &[Value]) -> String {
    format!("{:?}", t.primary_key(row))
}

impl Session {
    /// A row is about to be read into a record whose request is partial.
    pub(crate) fn note_partial_fetch(&mut self, rec: &Rec, row: &[Value], via_get: bool) -> Result<(), Signal> {
        let t = self.table(rec)?;
        let r = rec.borrow();
        let mut have = r.load.request.clone().unwrap_or_default();
        // The current key and the filters are needed to find the row (:193-285).
        let key = &t.keys[r.key.min(t.keys.len() - 1)];
        have.extend(key.iter().copied());
        have.extend(r.filters.iter().map(|f| f.field));
        // SystemId and the four audit fields are always loaded (record/lt-loadfields-pagesource-validate/P81LoadFields.al:172-190
        // LoadFields_Name_Get_LoadedFlags `a_loaded_SystemId` to `a_loaded_SystemModifiedBy`; :1047-1120 `key_before_loaded_System*`).
        have.extend((0..t.fields.len()).filter(|&p| (2_000_000_000..=2_000_000_004).contains(&t.fields[p].id)));
        have.sort_unstable();
        have.dedup();
        // A secondary key ends in the primary key; whether that is loaded is not pinned.
        let mut unsure = Vec::new();
        if r.key != 0 {
            unsure.extend(t.keys[0].iter().copied().filter(|p| !have.contains(p)));
        }
        let new_row = row_id(&t, row);
        let buffer = match &r.load.buffer {
            None => Buffer {
                have: FieldSet::Of(have),
                unsure: FieldSet::Of(unsure),
                row: Some(new_row),
                cleared: false,
                assigned: Vec::new(),
                missing_loaded: false,
                fetched: Default::default(),
            },
            // After a Clear only a whole fetched row stays in hand for a Get of the same row (session/
            // TestProbeSessionMiscContracts.al:PartialRecords_AreFieldsLoaded). A partial buffer does not: after FindSet and
            // Next under SetLoadFields(Qty), Clear, SetLoadFields(Name) and Get of the row Next fetched, Qty is not loaded
            // (record/lt-are-fields-loaded-unfetched/AreFieldsLoadedUnfetched.al:119-162, get_qty).
            Some(old)
                if old.cleared
                    && (old.row.as_deref() != Some(new_row.as_str()) || !matches!(old.have, FieldSet::All)) =>
            {
                Buffer {
                    have: FieldSet::Of(have),
                    unsure: FieldSet::Of(unsure),
                    row: Some(new_row),
                    cleared: false,
                    assigned: Vec::new(),
                    missing_loaded: false,
                    fetched: Default::default(),
                }
            }
            Some(old) => {
                let old_row = old.row.clone().unwrap_or_else(|| row_id(&t, &r.fields));
                let mut next = old.clone();
                next.row = Some(new_row.clone());
                next.cleared = false;
                next.assigned.clear();
                next.missing_loaded = false;
                if old_row == new_row && !via_get {
                    // A Find on the row the buffer holds reads it again and keeps only what the request names: after a full
                    // FindFirst, `SetLoadFields(Amt)` and another FindFirst, the Name is no longer loaded (recordref/
                    // lt-recordref-fieldref-members/P92LoadFields.al:Record_LoadFields_ThenAreFieldsLoaded, `name_after_set` No). A `Get`
                    // of that row keeps it (the case below).
                    next.have = FieldSet::Of(have);
                    next.unsure = FieldSet::Of(unsure);
                } else if old_row == new_row {
                    // The same row again by Get: nothing is evicted (record/TestRecordPartialLoadNarrowAfterFetch.al:121-185).
                    if let FieldSet::Of(h) = &mut next.have {
                        for p in have {
                            if !h.contains(&p) {
                                h.push(p);
                            }
                        }
                    }
                    for p in unsure {
                        if !next.have.contains(p) {
                            next.unsure.add(p);
                        }
                    }
                } else {
                    // Another row: what the old buffer held is not known to carry over, except after a load of every missing
                    // field, which carries nothing (P81LoadFields.al:1047-1120 `key_after_next_loaded_*`).
                    // A Get of another row after a whole fetched row carries nothing either (record/lt-outside-records/
                    // ResetClearsLoadFields.al:111 ResetThenGet_AfterEarlierFullGet `control without Reset`: `notes=No`).
                    let carried = match &old.have {
                        _ if old.missing_loaded => FieldSet::Of(Vec::new()),
                        FieldSet::All if via_get => FieldSet::Of(Vec::new()),
                        FieldSet::All => FieldSet::All,
                        FieldSet::Of(h) => FieldSet::Of(h.iter().copied().filter(|p| !have.contains(p)).collect()),
                    };
                    next.have = FieldSet::Of(have);
                    next.unsure = match carried {
                        FieldSet::All => FieldSet::All,
                        FieldSet::Of(mut v) => {
                            if let FieldSet::Of(u) = &old.unsure {
                                v.extend(u.iter().copied());
                            }
                            v.extend(unsure);
                            FieldSet::Of(v)
                        }
                    };
                }
                next
            }
        };
        drop(r);
        let mut buffer = buffer;
        buffer.fetched = al_builtins::value::FetchedRow(Some(std::rc::Rc::new(row.to_vec())));
        let mut r = rec.borrow_mut();
        r.load.partial = !matches!(buffer.have, FieldSet::All);
        r.load.buffer = Some(buffer);
        Ok(())
    }

    /// Loads what a partial buffer misses from the stored row, as BC does on a read of a field that is not loaded and on a
    /// Modify (record/lt-loadfields-pagesource-validate/P81LoadFields.al:1047-1243, al-runner issue 5764): every stored field
    /// that is neither loaded nor assigned takes the row's current value (`RN_StaleModify_JitReadAfterOtherChange` reads 99,
    /// `RN_StaleModify_UnloadedFieldChangedElsewhere` keeps the other variable's 99); a loaded field keeps the buffer's value, also
    /// when it is stale (`RN_StaleModify_UnloadedFieldSetHere_LoadedChangedElsewhere`), and so does an assigned one. Afterwards
    /// every field is loaded (`name_after_qty_loaded_*` Yes), and the field `read` joins the load set of the next fetch
    /// (`key_after_next_loaded_Name` Yes, `_Qty` No). A row that is gone, a key changed since the fetch, and a field whose
    /// state is unsure and whose value differs from the row's are not pinned.
    pub(crate) fn load_missing_fields(&mut self, rec: &Rec, read: Option<usize>) -> Result<(), Signal> {
        let (have, unsure, assigned, held, fetched) = {
            let r = rec.borrow();
            match &r.load.buffer {
                Some(b) if r.load.partial && !b.cleared && !matches!(b.have, FieldSet::All) => (
                    b.have.clone(),
                    b.unsure.clone(),
                    b.assigned.clone(),
                    b.row.clone(),
                    b.fetched.0.clone(),
                ),
                _ => return Ok(()),
            }
        };
        let Some(fetched) = fetched else {
            return unsupported("a load of the missing fields of a partial record whose fetched row is not known");
        };
        if read.is_some_and(|p| have.contains(p)) {
            return Ok(());
        }
        let t = self.table(rec)?;
        let fields = rec.borrow().fields.clone();
        if held.as_deref() != Some(row_id(&t, &fields).as_str()) {
            return unsupported(
                "a load of the missing fields of a partial record whose key changed since the fetch (not pinned)",
            );
        }
        let pk = Self::sql_key(rec, &t, &fields);
        let cx = self.cx.clone();
        let stored = self
            .store_for(rec)
            .get(&cx, &t, &pk)
            .map_err(|e| Signal::Unsupported(format!("{e}")))?;
        let Some(stored) = stored else {
            return unsupported("a load of the missing fields of a partial record whose row is gone (not pinned)");
        };
        let mut merged = fields;
        for p in 0..t.fields.len() {
            if !t.fields[p].stored || have.contains(p) || assigned.contains(&p) {
                continue;
            }
            // Changed by AL code since the fetch (Validate, TransferFields, Init, ...): kept, as an assignment is.
            let changed = match (&merged[p], fetched.get(p)) {
                (Value::Blob(_), _) => false,
                (v, Some(f)) => !v.identical(f),
                (_, None) => true,
            };
            if changed {
                continue;
            }
            // A Blob is the same when the buffer's unwritten copy holds the row's bytes.
            let same = match (&merged[p], &stored[p]) {
                (Value::Blob(a), Value::Blob(b)) => {
                    let (a, b) = (a.borrow(), b.borrow());
                    !a.written && a.bytes == b.bytes
                }
                (a, b) => a.identical(b),
            };
            if unsure.contains(p) && !same {
                return unsupported(format!(
                    "a load of the missing fields of a partial record over field position {p}, whose state the corpus does not pin"
                ));
            }
            merged[p] = match (&merged[p], &stored[p]) {
                // A Blob the record calculated or wrote keeps its content (P81LoadFields.al:664-689
                // CalcFields_OnNotLoadedFlowAndBlob); an unread one takes the row's (:1224-1243 RN_StaleModify_BlobChangedElsewhere).
                (Value::Blob(a), _) if a.borrow().loaded || a.borrow().written => continue,
                (_, Value::Blob(b)) => Value::Blob(al_builtins::stream::copy_blob_db_row(b)),
                (_, v) => v.clone(),
            };
        }
        let mut r = rec.borrow_mut();
        r.fields = merged;
        if let Some(b) = &mut r.load.buffer {
            b.have = FieldSet::Of((0..t.fields.len()).collect());
            b.unsure = FieldSet::Of(Vec::new());
            b.missing_loaded = true;
        }
        if let (Some(p), Some(req)) = (read, &mut r.load.request) {
            if !req.contains(&p) {
                req.push(p);
            }
        }
        Ok(())
    }

    pub(crate) fn load_fields_op(
        &mut self,
        method: RecordOp,
        rec: &Rec,
        t: &TableDef,
        args: &[Value],
    ) -> Result<Value, Signal> {
        let wanted = positions(args)?;
        let engine = self.engine.clone();
        let schema = engine.program.objects[t.object as usize].table.as_ref();
        let stored = |p: usize| schema.is_some_and(|s| s.fields.get(p).is_some_and(|f| f.stored));
        let flow_filter = |p: usize| schema.is_some_and(|s| s.fields.get(p).is_some_and(|f| f.flow_filter));
        match method {
            RecordOp::SetLoadFields | RecordOp::AddLoadFields => {
                // SetLoadFields takes a FlowField and adds nothing to the request for it; AddLoadFields refuses it with
                // an error of code DB:Ok (record/lt-priority-assumptions/PriorityAssumptions.al:282-312 SetLoadFields,
                // :303-338 AddLoadFields, real BC 29). The error is pinned for one FlowField as the only argument, after
                // a SetLoadFields and without a request; a longer list is not pinned. A FlowFilter is not pinned.
                // SetLoadFields takes a FlowFilter as it takes a FlowField (record/lt-loadfields-pagesource-validate/P81LoadFields.al:
                // 273-304 LoadFields_FlowFilterField), and answers Yes (:991-1028 BR_SetLoadFields_ReturnValue_FlowFieldAndFlowFilter:
                // a FlowField, a FlowFilter, a normal field with and without a FlowField, no field). The function form of AddLoadFields
                // answers No for a FlowField or a FlowFilter alone and raises nothing (`add flowfield`, `add flowfilter` ok|No);
                // its statement form with a FlowFilter is not pinned.
                let no_args = wanted.is_empty();
                let mut wanted = wanted;
                if method == RecordOp::AddLoadFields && wanted.iter().any(|&p| !stored(p)) && self.ret_used {
                    if wanted.len() != 1 || rec.borrow().load.request.is_none() {
                        return unsupported(
                            "the result of AddLoadFields of a FlowField or FlowFilter among several fields or without a request (not pinned)",
                        );
                    }
                    return Ok(Value::Boolean(false));
                }
                if method == RecordOp::AddLoadFields && wanted.iter().any(|&p| flow_filter(p)) {
                    return unsupported("AddLoadFields of a FlowFilter in its statement form (not pinned)");
                }
                if method == RecordOp::AddLoadFields && wanted.iter().any(|&p| !stored(p)) {
                    let p = wanted[0];
                    if wanted.len() != 1 {
                        return unsupported(
                            "AddLoadFields with a FlowField among several fields (only a single argument is pinned)",
                        );
                    }
                    let name = schema.and_then(|s| s.fields.get(p)).map_or("", |f| f.name.as_str());
                    return al_builtins::native::raise(
                        format!(
                            "The field '{name}' in the '{}' table is not valid for auto loading and cannot be passed to calls to AddLoadFields, SetLoadFields, or LoadFields.",
                            t.name
                        ),
                        "DB:Ok",
                    );
                }
                wanted.retain(|&p| stored(p));
                let mut r = rec.borrow_mut();
                match (method, no_args) {
                    // No arguments: back to the default load set (record/TestRecordPartialLoad.al:61-99).
                    (RecordOp::SetLoadFields, true) => r.load.request = None,
                    (RecordOp::SetLoadFields, false) => r.load.request = Some(wanted),
                    // AddLoadFields answers Yes for a field it adds (record/TestRecordExtended.al:58-65) and No for one the request
                    // holds already (P81LoadFields.al:1012-1024 `add normal` ok|No, after SetLoadFields(Note)).
                    (RecordOp::AddLoadFields, _) if self.ret_used => match &mut r.load.request {
                        None => return Ok(Value::Boolean(false)),
                        Some(req) => {
                            let mut added = false;
                            for p in wanted {
                                if !req.contains(&p) {
                                    req.push(p);
                                    added = true;
                                }
                            }
                            return Ok(Value::Boolean(added));
                        }
                    },
                    // Without a request the load is already full: nothing to add. The result is
                    // the documentation's (docs/probes-needed.md).
                    (RecordOp::AddLoadFields, _) => match &mut r.load.request {
                        None => return Ok(Value::Boolean(false)),
                        Some(req) => {
                            for p in wanted {
                                if !req.contains(&p) {
                                    req.push(p);
                                }
                            }
                            return Ok(Value::Boolean(true));
                        }
                    },
                    _ => unreachable!(),
                }
                Ok(if self.ret_used {
                    Value::Boolean(true)
                } else {
                    Value::Null
                })
            }
            RecordOp::SetBaseLoadFields => {
                // Only the full-load case is pinned as callable (record/TestRecordExtended.al:71-80);
                // what the base set holds is not.
                // After SetLoadFields(Cd), SetBaseLoadFields loads the whole row again (session/
                // TestProbeSessionMiscContracts.al:PartialRecords_AreFieldsLoaded, `baseloadfields_txt` Yes).
                let had = rec.borrow().load.request.is_some();
                if had && self.ret_used {
                    return unsupported(
                        "the result of SetBaseLoadFields on a partial record (only the statement form is pinned)",
                    );
                }
                rec.borrow_mut().load.request = None;
                if had {
                    return Ok(Value::Null);
                }
                Ok(Value::Boolean(false))
            }
            RecordOp::AreFieldsLoaded => {
                if wanted.is_empty() {
                    return unsupported("AreFieldsLoaded without fields");
                }
                let flow = wanted.iter().any(|&p| !stored(p));
                let wanted: Vec<usize> = wanted.into_iter().filter(|&p| stored(p)).collect();
                // A FlowField reads as loaded on a record that fetched a partial request, whether or not it was requested
                // (PriorityAssumptions.al:282-312 flow_only_cnt_loaded, mixed_cnt_loaded; :325-333 cnt_loaded). A record
                // that fetched nothing or a whole row is not pinned for it. A FlowFilter reads the same: Yes after a partial
                // fetch, after a read of an unloaded field and after Next (record/lt-loadfields-pagesource-validate/
                // P81LoadFields.al:1047-1120 RN_JitRead_UnloadedField_LoadsWhich, `_loaded_KindFilter`).
                if flow {
                    let r = rec.borrow();
                    if !r.load.buffer.as_ref().is_some_and(|b| !matches!(b.have, FieldSet::All)) {
                        return unsupported(
                            "AreFieldsLoaded of a FlowField on a record that holds no partial fetch (not pinned)",
                        );
                    }
                }
                let r = rec.borrow();
                let Some(b) = &r.load.buffer else {
                    // Nothing was fetched (record/TestRecordPartialLoad.al:102-141). A buffer that
                    // was filled by hand is not pinned.
                    let fresh = self.initial_fields(t.object as usize);
                    // A record filled by assignment, Validate or Insert and never fetched reports every field
                    // loaded, whichever field was assigned; after Init it reports none
                    // (record/lt-are-fields-loaded-unfetched/AreFieldsLoadedUnfetched.al:42-68 assigned,
                    // :70-92 Init and Validate, :94-117 Insert and Modify, :174-221 temporary).
                    // Any assignment counts, a field assigned its own default value too
                    // (record/lt-arefieldsloaded-default/AreFieldsLoadedDefault.al:57-88, :122-137). Insert, Modify and
                    // Validate are read from the buffer differing from a fresh one.
                    let filled = r.assigned
                        || fresh.len() != r.fields.len()
                        || fresh.iter().zip(r.fields.iter()).any(|(a, b)| !a.identical(b));
                    return Ok(Value::Boolean(filled));
                };
                // A Blob reads as loaded on a record that holds a partial fetch, requested or not (record/
                // lt-loadfields-pagesource-validate/P81LoadFields.al:172-190 LoadFields_Name_Get_LoadedFlags `a_loaded_Notes`,
                // :372-388 LoadFields_BlobNotLoaded_CalcFields `loaded_notes_before`; a Media field is not: `_loaded_Pic` No).
                let blob = |p: usize| {
                    schema.is_some_and(|s| {
                        s.fields
                            .get(p)
                            .is_some_and(|f| f.stored && f.ty == al_builtins::value::RtType::Library("Blob".into()))
                    })
                };
                let partial = !matches!(b.have, FieldSet::All);
                // After Clear a partial buffer holds no field (misc/lt-probes-needed-2/PnBErrorsCollections.al:640-674
                // SetLoadFields_ThenRead_AndClear `afterClear.name` No).
                if partial && b.cleared {
                    return Ok(Value::Boolean(false));
                }
                let mut all = true;
                for &p in &wanted {
                    if b.have.contains(p) || (partial && blob(p)) {
                        continue;
                    }
                    if b.unsure.contains(p) {
                        return unsupported(format!(
                            "AreFieldsLoaded of a field whose state the corpus does not pin (position {p})"
                        ));
                    }
                    all = false;
                }
                Ok(Value::Boolean(all))
            }
            _ => unreachable!(),
        }
    }

    /// `Record.LoadFields(Fields)`: on a record that was never fetched it does not read the row (the buffer keeps
    /// what was assigned, the key stays as it is) and the function form answers false, with an existing and with a
    /// missing key; on a record that holds a whole fetched row it changes nothing (types/PYLoadFindThis.al, BC 27.5 and
    /// 29.0). The answer on a fetched record and a record that holds a partial buffer are not pinned: refused.
    pub(crate) fn load_fields_row(&mut self, rec: &Rec, t: &TableDef, args: &[Value]) -> Result<Value, Signal> {
        let wanted = positions(args)?;
        let engine = self.engine.clone();
        let schema = engine.program.objects[t.object as usize].table.as_ref();
        if let Some(p) = wanted
            .iter()
            .find(|&&p| !schema.is_some_and(|s| s.fields.get(p).is_some_and(|f| f.stored)))
        {
            return unsupported(format!("LoadFields of field position {p}, which is not a stored field"));
        }
        // A buffer that survives a Clear is for a later fetch of the same row, not a row the record holds.
        if rec
            .borrow()
            .load
            .buffer
            .as_ref()
            .is_some_and(|b| !b.cleared && !matches!(b.have, FieldSet::All))
        {
            return unsupported(
                "LoadFields on a record that holds a partly fetched row: whether BC reads the missing fields, over values assigned since, is not pinned (docs/probes-needed.md)",
            );
        }
        if self.ret_used && rec.borrow().position.is_some() {
            return unsupported(
                "LoadFields with its result used on a record that holds a fetched row: BC's answer is not pinned (docs/probes-needed.md)",
            );
        }
        Ok(Value::Boolean(false))
    }
}
