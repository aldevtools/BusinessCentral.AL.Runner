//! `record_op_inner`: the dispatch of every Record method (moved out of `records.rs`).

use super::*;

impl Session {
    pub(crate) fn record_op_inner(&mut self, method: RecordOp, rec: &Rec, args: &mut [Value]) -> Result<Value, Signal> {
        self.connection_check(rec)?;
        if rec.borrow().unpinned_buffer.is_some() {
            return self.unpinned_buffer_op(method, rec, args);
        }
        // Copy, CopyFilters, TransferFields and the like from a table element record after an export under SourceTableView
        // would carry its unpinned view or buffer over.
        if let Some(Value::Record(src)) = args.first() {
            if let Some(why) = xp_export_marked(src) {
                return unsupported(format!("Record.{}() from a record: {why}", method.name()));
            }
        }
        let t = self.table(rec)?;
        Self::refuse_filter_in_unpinned_group(method, rec)?;
        if !rec.borrow().temporary {
            self.install_assumed(t.object);
        }
        // A table declared ObsoleteState = Moved is not in the runtime app: AllObj has no row for it and RecordRef.Open
        // fails (metadata/platform-tables/AwTests.al:413-442). What a record of it does is not pinned.
        if self.engine.object_facts.get(t.object as usize).is_some_and(|f| f.moved) {
            return unsupported(format!(
                "{} on table {}, which is declared ObsoleteState = Moved (what BC does is not pinned)",
                method.name(),
                t.name
            ));
        }
        self.table_op_checks(&t, rec, method, args)?;
        // The impact trace, and a subscriber whose install order is not known run once under watch (order_trial.rs).
        self.note_table_op(t.object, method.writes(), rec.borrow().temporary)?;
        if matches!(
            method,
            RecordOp::Find
                | RecordOp::FindFirst
                | RecordOp::FindLast
                | RecordOp::FindSet
                | RecordOp::Next
                | RecordOp::Count
                | RecordOp::IsEmpty
                | RecordOp::CalcSums
        ) {
            let temporary = rec.borrow().temporary;
            self.unseeded_read(&t, temporary, method.name())?;
            self.coverage_read(&t, temporary)?;
            // `Count` stays answered (it counts only the companies the runtime knows): refusing it loses the baseline case
            // session-system/TestProbeSessionMiscContracts.al:Company_Names (69815, `Count() >= 1`). Kept, not pinned.
            if !temporary && self.is_company_table(&t) && method != RecordOp::Count {
                self.company_table_read(rec, method.name())?;
            }
        }
        if matches!(
            method,
            RecordOp::Get | RecordOp::GetBySystemId | RecordOp::CalcFields | RecordOp::LoadFields
        ) {
            let temporary = rec.borrow().temporary;
            self.coverage_read(&t, temporary)?;
        }
        let cx = self.cx_for(rec, &t)?;
        // A Count or IsEmpty of a table whose demo rows the corpus database holds, before the session's own code wrote to it:
        // BC counts 1 Company Information row and 36 Accounting Period rows (session-system/company/
        // CompanyInformationValues.al:45 QG_CompanyInformation_CorpusRow, :89 QG_AccountingPeriod_CorpusRows) and holds G/L entries
        // (test-framework/RlWorkDateAtStart.al:48 `G/L entries exist` Yes); the runtime has
        // only what install code wrote (ADR-047 rejected). A test that writes its own rows is answered. Media Resources holds
        // 46 rows there (metadata/platform-tables/PtCloudTables.al:269 QX_MediaResources_FreshTenantCodes, read with FindSet),
        // so every read of it is refused before the session wrote to it.
        if !rec.borrow().temporary && self.engine.demo_tables.contains(&(t.object as usize)) {
            if method.writes() {
                if !self.install_scope {
                    self.demo_written.insert(t.object);
                }
            } else if (matches!(method, RecordOp::Count | RecordOp::IsEmpty)
                || (self.engine.program.objects[t.object as usize].id == Some(crate::platform::MEDIA_RESOURCES)
                    && matches!(
                        method,
                        RecordOp::Find
                            | RecordOp::FindFirst
                            | RecordOp::FindLast
                            | RecordOp::FindSet
                            | RecordOp::Get
                            | RecordOp::GetBySystemId
                            | RecordOp::Next
                            | RecordOp::CountApprox
                            | RecordOp::CalcSums
                    )))
                && !self.demo_written.contains(&t.object)
            {
                return unsupported(format!(
                    "{} on {} before the test wrote to it: the corpus database holds demo rows there that the runtime does not have (ADR-047 rejected)",
                    method.name(),
                    t.name
                ));
            }
        }
        // A write of General Ledger Setup by the test's own code: the captions of areas 1 and 101 read it (`refuse_setup_caption`).
        if method.writes()
            && !self.gl_setup_written
            && !self.install_scope
            && !rec.borrow().temporary
            && self
                .engine
                .program
                .objects
                .get(t.object as usize)
                .is_some_and(|o| o.id == Some(98))
        {
            self.gl_setup_written = true;
        }
        // The next access of the table raises a deferred duplicate-key Insert, an Insert of another row not (the buffered inserts
        // stay buffered: record/P146TableExtProtected.al:P146_Triggers_BaseThenExtension, where
        // `R2.Insert(true)` succeeds and the next Modify raises it).
        if matches!(
            method,
            RecordOp::Modify
                | RecordOp::Delete
                | RecordOp::DeleteAll
                | RecordOp::ModifyAll
                | RecordOp::Rename
                | RecordOp::Truncate
                | RecordOp::Get
                | RecordOp::LoadFields
                | RecordOp::Find
                | RecordOp::FindFirst
                | RecordOp::FindLast
                | RecordOp::FindSet
                | RecordOp::Next
                | RecordOp::Count
                | RecordOp::IsEmpty
                | RecordOp::CalcFields
                | RecordOp::CalcSums
        ) && !rec.borrow().temporary
            && self
                .tx
                .deferred_insert
                .as_ref()
                .is_some_and(|(o, _)| *o == t.object as usize)
        {
            let (_, e) = self.tx.deferred_insert.take().expect("checked");
            self.tx.deferred_raised += 1;
            return Err(Signal::Error(Box::new(e)));
        }
        if let Some(schema) = self.engine.program.objects[t.object as usize]
            .table
            .as_ref()
            .filter(|_| self.engine.table_facts[t.object as usize].unresolved_init)
        {
            if let Some(f) = schema.fields.iter().find(|f| f.init_text.is_some() && f.init.is_none()) {
                // An InitValue BC accepts at compile time and cannot evaluate when it builds the
                // buffer: Init() on a written record raises, on an untouched one does nothing
                // (record/TestDateTimeInitValue.Codeunit.al:29-77). Nothing else is pinned.
                let messages: std::collections::BTreeSet<&String> = (0..schema.fields.len())
                    .filter_map(|pos| self.engine.unevaluable_init.get(&(t.object, pos)))
                    .collect();
                let only_unevaluable = schema
                    .fields
                    .iter()
                    .enumerate()
                    .filter(|(_, f)| f.init_text.is_some() && f.init.is_none())
                    .all(|(pos, _)| self.engine.unevaluable_init.contains_key(&(t.object, pos)));
                if method == RecordOp::Init && only_unevaluable && messages.len() == 1 {
                    if !rec.borrow().touched {
                        return Ok(Value::Null);
                    }
                    return Err(Signal::Error(Box::new(AlError {
                        message: (*messages.iter().next().expect("one message")).clone(),
                        code: String::new(),
                        call_stack: String::new(),
                        ..Default::default()
                    })));
                }
                return unsupported(format!(
                    "{}.{}: InitValue `{}` is not implemented yet",
                    t.name,
                    f.name,
                    f.init_text.as_deref().unwrap_or("")
                ));
            }
        }
        // OnInstallAppPerDatabase runs with CompanyName() empty. IsEmpty and FindFirst of a per-company table fail there
        // with `You must choose a company before you can access the "<table>" table.` for a table without a Caption
        // (record/BfInstallTests.al:138-139, reads at BfInstallFixtures.al:131-146; ADR-046
        // amendment 1, A5). The code BC gave, DB:Ok, waits for the Windows check (a Linux suspect), so it is not pinned.
        // Every other operation, and a table with a Caption, is not pinned: refused below.
        if self.cx.company.is_empty()
            && t.per_company
            && !rec.borrow().temporary
            && matches!(method, RecordOp::IsEmpty | RecordOp::FindFirst)
            && !self.engine.program.objects[t.object as usize]
                .properties
                .iter()
                .any(|(k, _)| k.eq_ignore_ascii_case("Caption"))
        {
            return Err(Signal::Error(Box::new(AlError {
                message: format!(
                    "You must choose a company before you can access the \"{}\" table.",
                    t.name
                ),
                code: al_builtins::native::UNPINNED_ERROR.into(),
                call_stack: String::new(),
                ..Default::default()
            })));
        }
        if self.cx.company.is_empty()
            && t.per_company
            && !rec.borrow().temporary
            && matches!(
                method,
                RecordOp::Insert
                    | RecordOp::Modify
                    | RecordOp::Delete
                    | RecordOp::DeleteAll
                    | RecordOp::ModifyAll
                    | RecordOp::Rename
                    | RecordOp::Truncate
                    | RecordOp::Get
                    | RecordOp::Find
                    | RecordOp::FindFirst
                    | RecordOp::FindLast
                    | RecordOp::FindSet
                    | RecordOp::Next
                    | RecordOp::Count
                    | RecordOp::IsEmpty
                    | RecordOp::CalcFields
                    | RecordOp::CalcSums
            )
        {
            return unsupported(format!(
                "{method} on the per-company table {} with no company chosen (BC: \"You must choose a company before you can access the ...\")",
                t.name
            ));
        }
        if matches!(
            method,
            RecordOp::Insert
                | RecordOp::Modify
                | RecordOp::Delete
                | RecordOp::DeleteAll
                | RecordOp::ModifyAll
                | RecordOp::Rename
                | RecordOp::Truncate
        ) && !rec.borrow().temporary
        {
            self.bg_refuse_write(t.object as usize, method.name())?;
            self.gate_write(method)?;
            if matches!(method, RecordOp::Insert | RecordOp::DeleteAll)
                && let Some(e) = self.read_only_platform_table(&t, rec, method)
            {
                return Err(Signal::Error(Box::new(e)));
            }
            if self.engine.program.objects[t.object as usize].id == Some(2_000_000_130) {
                return Err(self.device_write(&t, method, args)?);
            }
            // Inside a TryFunction whose result is used only Insert is pinned (errors/CuTryTests.al:83-150);
            // writes to a temporary record are allowed (:153-171), and never reach here. A write to a CRM table goes to its
            // table connection, not to SQL: inside such a TryFunction Insert, Modify, Delete and Rename run and raise their own
            // errors (tests/al-language-onprem/session-system/Y16CrmTableConnectionOps.al:184-251, `rename` ok, `insert duplicate`
            // DB:RecordExists, `modify missing` DB:RecordNotFound).
            if rec.borrow().crm
                && matches!(
                    method,
                    RecordOp::Insert | RecordOp::Modify | RecordOp::Delete | RecordOp::Rename
                )
            {
            } else if matches!(method, RecordOp::Insert | RecordOp::Modify) {
                self.gate_used_try_row_write(&t, method)?;
            } else {
                self.gate_used_try_other_write(&t, method)?;
            }
            self.user_bulk_refusal(&t, rec.borrow().temporary, method)?;
            // A write call starts the transaction, also a DeleteAll that finds no row
            // (session-system/ProbeSystemEnumMembers.al:143-161, session-system/TestDatabaseExtended.al:
            // Database_CurrentTransactionType_SetAfterWriteOp_Throws over fixture DeleteAlls).
            self.tx.tx_phase = crate::transactions::TX_WRITE;
        } else if self.tx.tx_phase == crate::transactions::TX_FRESH
            && !rec.borrow().temporary
            && matches!(
                method,
                RecordOp::Get
                    | RecordOp::GetBySystemId
                    | RecordOp::Find
                    | RecordOp::FindFirst
                    | RecordOp::FindLast
                    | RecordOp::FindSet
                    | RecordOp::Next
                    | RecordOp::Count
                    | RecordOp::IsEmpty
                    | RecordOp::CalcFields
                    | RecordOp::CalcSums
            )
        {
            // A read call starts it too (session-system/ProbeSystemEnumMembers.al:94-112).
            self.tx.tx_phase = crate::transactions::TX_READ;
        }
        if !rec.borrow().temporary
            && matches!(
                method,
                RecordOp::Get
                    | RecordOp::GetBySystemId
                    | RecordOp::Find
                    | RecordOp::FindFirst
                    | RecordOp::FindLast
                    | RecordOp::FindSet
                    | RecordOp::Next
                    | RecordOp::Count
                    | RecordOp::IsEmpty
                    | RecordOp::CalcFields
                    | RecordOp::CalcSums
            )
        {
            // What LastUsedRowVersion reports after a read of a table the open transaction inserted into.
            let company = rec.borrow().company.clone();
            // GetBySystemId, Find('='), Find('-'), Find('+') and a CalcFields of a record whose own table holds inserted rows
            // send those rows too (metadata/platform-tables/PlTRowVersionValue.al:917-980
            // NP3_RowVersions_InsertThenGetBySystemId_OpenTransaction, _InsertThenFind_OpenTransaction, :1009-1040
            // _InsertThenCalcFields_OpenTransaction). PlTRowVersionValue.al NP4_RowVersions_InsertA_FindSetNextOfB and
            // _InsertA_FindFormsOfB: Next() and Find('='/'>'/'<'/'-'/'+') of another table send nothing;
            // _InsertA_FindSetNextAndFindGreaterOfA: Find('>') of the inserted table sends its rows. A CalcFields whose own
            // table holds none, Next or Find('<') of the inserted table, Next(N) and other Find forms stay unpinned.
            let own_pending = self
                .tx
                .row_versions
                .pending_inserts
                .iter()
                .any(|(o, c, _)| *o == t.object && *c == company);
            let find_form = match args.first() {
                Some(Value::Text(w)) => Some(w.as_str()),
                _ => None,
            };
            let next_or_find_form = match method {
                RecordOp::Find => match find_form {
                    Some(">") => true,
                    Some("<") => !own_pending,
                    _ => false,
                },
                // Only Next() is pinned: Next(1) and other steps stay unpinned.
                RecordOp::Next => !own_pending && args.is_empty(),
                _ => false,
            };
            let more = next_or_find_form
                || match method {
                    RecordOp::GetBySystemId => true,
                    RecordOp::Find => matches!(find_form, Some("=" | "-" | "+")),
                    RecordOp::CalcFields => own_pending,
                    _ => false,
                };
            self.note_read_for_row_versions(
                t.object,
                &company,
                more || matches!(
                    method,
                    RecordOp::Get
                        | RecordOp::FindFirst
                        | RecordOp::FindLast
                        | RecordOp::FindSet
                        | RecordOp::IsEmpty
                        | RecordOp::Count
                        | RecordOp::CalcSums
                ),
            );
            use crate::recmeta::{ISOLATION_REPEATABLE_READ, ISOLATION_UPDLOCK};
            // The assignment of a locking isolation opened the write transaction (`RecordOp::ReadIsolation`), the read does
            // not: a Get, Find('='), IsEmpty, Count, Next or CalcFields under UpdLock or RepeatableRead set in an earlier
            // transaction leaves IsInWriteTransaction No and a guarded Codeunit.Run runs (corpus PR 896
            // format-evaluate/Np4Misc.al Y9_UpdLock_SetBeforeCommit_FirstRead, Y9_RepeatableRead_SetBeforeCommit_FirstRead
            // `before=No|...|after=No`, `ran=Yes tx=No`). Another read there (FindFirst, FindSet, FindLast, another Find) is not
            // pinned. A ReadCommitted read opens none (codeunits/TestCodeunitRunWriteTransaction.al:254-278); a ReadUncommitted
            // read opens none under an assumption.
            let pinned_read = match method {
                RecordOp::Get | RecordOp::IsEmpty | RecordOp::Count | RecordOp::Next | RecordOp::CalcFields => true,
                RecordOp::Find => matches!(args.first(), Some(Value::Text(w)) if w.as_str() == "="),
                _ => false,
            };
            self.tx.db_reads += 1;
            match rec.borrow().read_isolation {
                ISOLATION_REPEATABLE_READ | ISOLATION_UPDLOCK
                    if self.tx.lock_tx && matches!(method, RecordOp::FindFirst | RecordOp::FindSet) =>
                {
                    self.tx.isolation_lock = true;
                    self.tx.note_isolation_lock(t.object, rec.borrow().company.clone());
                }
                ISOLATION_REPEATABLE_READ | ISOLATION_UPDLOCK => {
                    if !self.tx.lock_tx && !pinned_read {
                        self.tx.locking_read_tx_unpinned = true;
                    }
                    // Another cause already recorded is kept: `lock_unpinned` names this read only while such reads are its
                    // only cause (see `read_of_others_only` in the stale-buffer decision).
                    if self.tx.lock_unpinned.is_none_or(|w| w == UNPINNED_LOCKING_READ) {
                        self.tx.lock_unpinned = Some(UNPINNED_LOCKING_READ);
                    }
                    self.tx.note_isolation_lock(t.object, rec.borrow().company.clone());
                }
                _ => {}
            }
        }
        match method {
            RecordOp::Init => {
                let fresh = self.init_fields(&t, rec);
                let mut r = rec.borrow_mut();
                crate::partial::note_init(&mut r);
                r.assigned = false;
                r.inserted = false;
                if matches!(r.is_dirty_base, al_builtins::value::DirtyBase::Synced) {
                    // The buffer is the snapshot Record.IsDirty compares with, and Init keeps it (isdirty.rs): the old
                    // buffer becomes the snapshot instead of being copied.
                    let mut fresh = fresh;
                    for &i in &t.keys[0] {
                        fresh[i] = r.fields[i].clone();
                    }
                    let old = std::mem::replace(&mut r.fields, fresh);
                    r.is_dirty_base = al_builtins::value::DirtyBase::Row {
                        base: old,
                        alt: None,
                        dirty_only: false,
                        stale_store: false,
                    };
                    return Ok(Value::Null);
                }
                for (i, v) in fresh.into_iter().enumerate() {
                    // Doc (Record.Init): primary key fields keep their values.
                    if !t.keys[0].contains(&i) {
                        r.fields[i] = v;
                    }
                }
                Ok(Value::Null)
            }
            RecordOp::Clear => self.clear_record(&t, rec),
            RecordOp::Insert => {
                if !rec.borrow().temporary {
                    self.page_metadata_before_insert(&t, &rec.borrow().fields)?;
                }
                // A temporary record has no sequence: the key stays as it is (corpus
                // record/TestTempTableContracts.al:52).
                let auto = if rec.borrow().temporary {
                    None
                } else {
                    self.auto_increment_field(&t)?
                };
                let run = arg_bool(args, 0)?;
                // A duplicate key does not skip the Before event and the trigger: they run, then
                // the write fails and the After event does not (corpus record/TestTableEventSequenceContracts.al:
                // FailingWrites_RaiseTheBeforeEventAndTheTrigger_ThenTheError).
                // Before event, then the trigger, then the write, then the After event
                // (record/TestTriggerDispatchOrder.al:TriggerOrder_Insert_RecordPipeline_IsStable).
                // Rec carries its SystemId from the Before event on, the one that is stored (record/TestXRecStampFields.al:80-90;
                // events/TestProbeEventContracts.al:SystemId_BeforeOnInsert_AndPresetBuffer, `sysid_null=No` in the Before event);
                // the stamps and the key come below it.
                let keep_id = match args.get(1) {
                    Some(v) => ops::truthy(v)?,
                    None => false,
                };
                // xRec in the insert triggers is the buffer as it was before the SystemId is given: the SystemId an
                // earlier insert left in the variable, else the null Guid (Insert_Events_TriggerFalseAndTrue).
                let before_id = t
                    .fields
                    .iter()
                    .position(|f| f.id == 2_000_000_000)
                    .map(|i| rec.borrow().fields[i].clone());
                self.assign_system_id(&t, rec, keep_id);
                let id_slot = t.fields.iter().position(|f| f.id == 2_000_000_000);
                // The id the platform gave the row, before any event or trigger could change it.
                let assigned = id_slot.map(|i| rec.borrow().fields[i].clone());
                self.table_event(&t, "onbeforeinsertevent", rec, None, run, None)?;
                // xRec in the insert triggers is the buffer without the SystemId it was just given
                // (record/TestXRecStampFields.al:38-52).
                let insert_xrec = id_slot.map(|i| {
                    let mut copy = rec.borrow().clone();
                    copy.fields[i] = before_id.clone().unwrap_or(Value::Guid([0; 16]));
                    Rc::new(RefCell::new(copy))
                });
                if run {
                    // The extension's triggers bracket the base table's own (tableextensiontrigger/
                    // TestTableExtensionTableTriggers_Tests.al); Insert(false) runs none of them.
                    self.run_ext_trigger(&t, "OnBeforeInsert", rec, insert_xrec.as_ref())?;
                    if let Some(tr) = self.table_trigger(&t, "OnInsert") {
                        self.run_trigger(t.object as usize, tr, rec, insert_xrec.as_ref())?;
                    }
                    self.document_service_insert(&t, rec)?;
                    self.run_ext_trigger(&t, "OnInsert", rec, insert_xrec.as_ref())?;
                }
                // The SystemId the OnBeforeInsertEvent subscribers and the insert triggers left in the buffer.
                let mut set_by_code = id_slot.map(|i| rec.borrow().fields[i].clone());
                // A SystemId that Init() or Clear(SystemId) in the table's OnInsert, or Init() in an OnBeforeInsertEvent subscriber,
                // made the null Guid: the row is stored with a new id, neither the one the trigger saw nor a preset one, with or
                // without InsertWithSystemId, and the variable holds the stored id (record/system-fields/
                // QrSystemIdTests.al:393-530, AL Runner issue 5803). A table extension's insert triggers, the database trigger and the
                // User trigger running then are not pinned.
                if let (Some(i), Some(Value::Guid(g))) = (id_slot, &set_by_code)
                    && *g == [0; 16]
                    && !rec.borrow().temporary
                {
                    let ext = run
                        && (!self.ext_triggers(&t, "OnBeforeInsert").is_empty()
                            || !self.ext_triggers(&t, "OnInsert").is_empty());
                    if ext || self.global_enabled(&t, GlobalOp::Insert)? || self.is_user_row(&t, false) {
                        return unsupported(
                            "a SystemId an Insert event or trigger cleared, with table extension insert triggers, the database trigger or the User trigger: not pinned",
                        );
                    }
                    let id = Value::Guid(self.fresh_system_id());
                    rec.borrow_mut().fields[i] = id.clone();
                    set_by_code = Some(id);
                }
                // The database event follows the table's triggers and precedes the write
                // (record/TestTriggerDispatchOrder.al:TriggerOrder_Insert_RecordPipeline_IsStable).
                self.global_trigger(&t, GlobalOp::Insert, rec, None)?;
                // The platform's own User trigger runs below the events and triggers (usertable.rs).
                let user_row = self.is_user_row(&t, rec.borrow().temporary);
                if user_row {
                    self.user_before_insert(&t, rec)?;
                }
                // A SystemId that an OnBeforeInsertEvent subscriber, the table's OnInsert or a table extension's OnBeforeInsert or
                // OnInsert sets is kept in the variable and the stored row, without InsertWithSystemId, also through
                // RecordRef.Insert and on a temporary record (record/system-fields/QrSystemIdTests.al:56-78, :150-196;
                // types/AtSystemIdOnInsert.al:135-170; record/events/AfSystemIdBeforeInsert.al).
                if let (false, Some(i), Some(platform), Some(set)) = (keep_id, id_slot, assigned, set_by_code) {
                    let same = |a: &Value, b: &Value| matches!((a, b), (Value::Guid(x), Value::Guid(y)) if x == y);
                    // The database trigger and the platform's own User trigger changing it, and code clearing it, are not pinned.
                    if !same(&rec.borrow().fields[i], &set)
                        || (matches!(set, Value::Guid(g) if g == [0; 16]) && !same(&set, &platform))
                    {
                        return unsupported(
                            "a SystemId set by the database trigger, or cleared by an Insert event or trigger, without InsertWithSystemId: whether BC keeps it is not pinned",
                        );
                    }
                    if !same(&set, &platform) {
                        // An id another row holds: Insert with a return value answers false and the variable does not keep it
                        // (QrSystemIdTests.al:213-246, `caller has first id No`; which id it holds instead is not pinned, docs/probes-needed.md);
                        // the statement form raises DB:RecordExists (:251-293).
                        if rec.borrow().temporary {
                            if self.system_id_taken(rec, &t)? {
                                return unsupported(
                                    "a SystemId an Insert trigger sets on a temporary record that another of its rows holds (not pinned)",
                                );
                            }
                        } else if let Err(e) = self.check_system_id_unique(rec, &t) {
                            if self.ret_used && matches!(e, Signal::Error(_)) {
                                rec.borrow_mut().fields[i] = platform;
                                return Ok(Value::Boolean(false));
                            }
                            return Err(e);
                        }
                    }
                }
                if keep_id {
                    self.check_system_id_unique(rec, &t)?;
                }
                self.check_code_storage(&t, rec, false)?;
                self.sql_timestamp_before(&t, rec, None)?;
                let old_version = self.bump_row_version(&t, rec);
                self.sql_timestamp_after(&t, rec);
                let stamped = self.stamp_insert(&t, rec);
                // The key is chosen below the trigger: OnInsert starts with it still 0 and a
                // refused Insert never takes a value (corpus record/TestAutoIncrementExplicitKey.al:63,
                // TestAutoIncrementEdges.al:354).
                let mut taken = false;
                if let Some(a) = auto {
                    let mut row = rec.borrow().fields.clone();
                    taken = self.take_auto_increment(&t, a, &mut row, &cx.company)?;
                    rec.borrow_mut().fields = row;
                }
                let row = rec.borrow().fields.clone();
                // A unique secondary key refuses the row after its AutoIncrement value was taken
                // (corpus record/TestAutoIncrementExplicitKey.al:131,163).
                if let Some(unique_key) = self.unique_violation(rec, &t, &row, None)? {
                    Self::restore_row_version(rec, old_version);
                    Self::unstamp(rec, stamped);
                    if let Some(a) = auto {
                        if taken {
                            rec.borrow_mut().fields[a] = self.default_value(&t.fields[a].ty);
                        } else {
                            self.note_explicit_key(&t, a, &row, &cx.company);
                        }
                    }
                    let r = self.fail_unique(&t, unique_key, &row);
                    // The flag hides the values of a unique index an Insert into a table with a Blob field breaks
                    // (errors/last-error/InsertDuplicateKeyExcludeFlag.al:121). Other tables are not pinned.
                    if self.has_blob_field(&t) && !rec.borrow().temporary {
                        return r.map_err(|s| redact_error(s, format!(
                            "There is already a record in table {} that has the same values in a unique index for the following fields: <redacted>",
                            self.caption(&t)
                        )));
                    }
                    return r;
                }
                let mut view = self.stored_view(&t, &row);
                // A temporary row keeps the FlowFilter values the Insert wrote (`flow_filter_positions`).
                if rec.borrow().temporary {
                    for i in self.flow_filter_positions(&t) {
                        view[i] = row[i].clone();
                    }
                }
                let mut stored = Self::to_store(rec, view);
                self.webhook_application_id(&t, rec, &mut stored)?;
                if let Some(bad) = self.sql_columns(rec, &t, &mut stored)? {
                    let shape = InsertShape {
                        auto: auto.is_some(),
                        run,
                    };
                    return self.defer_insert_sql_range(rec, &t, bad, shape, (old_version, stamped));
                }
                let empty_blob = Self::temp_insert_blobs(rec, &stored);
                Self::mark_blobs_inserted(rec, &stored, true);
                let before = self.store.change_counter();
                // The SQL counters' buffer before their first read (sqlcount.rs, `SqlCounters::pre_insert`).
                let pre_ops = (!self.sql.active && !rec.borrow().temporary).then(al_data::memory::store_ops);
                match self
                    .store_for(rec)
                    .insert(&cx, &t, stored)
                    .map_err(|e| Signal::unsupported(format!("{e}")))?
                {
                    InsertOutcome::Inserted => {
                        if let Some(ops) = pre_ops {
                            let plain = !run && auto.is_none() && rec.borrow().company.is_none();
                            self.sql.pre_insert(t.object, plain, ops);
                            self.sql.pre_inserted(al_data::memory::store_ops());
                        }
                        self.note_insert(rec, &t, before);
                        Self::note_empty_blob_insert(rec, &t, empty_blob);
                        Self::clear_blob_written(rec);
                        rec.borrow_mut().inserted = true;
                        if !rec.borrow().temporary {
                            let company = rec.borrow().company.clone();
                            if !self
                                .tx
                                .tables_inserted
                                .iter()
                                .any(|(o, c)| *o == t.object && *c == company)
                            {
                                self.tx.tables_inserted.push((t.object, company));
                            }
                        }
                        let version = t
                            .fields
                            .iter()
                            .position(|f| f.id == 2_000_000_005)
                            .map(|i| rec.borrow().fields[i].clone());
                        if let Some(Value::BigInteger(v)) = version {
                            let commits = self.commits;
                            rec.borrow_mut().row_origin = Some(al_builtins::value::RowOrigin::new(true, v, commits));
                        }
                        // The Blob content an Insert of a temporary record stored is persisted, as one a Modify stored (see Rename).
                        if rec.borrow().temp_id.is_some() {
                            for v in &rec.borrow().fields {
                                if let Value::Blob(b) = v {
                                    b.borrow_mut().persisted = true;
                                }
                            }
                        }
                        if let (Some(a), false) = (auto, taken) {
                            self.note_explicit_key(&t, a, &row, &cx.company);
                        }
                        if user_row {
                            self.user_after_insert(&t, rec)?;
                        }
                        if run {
                            self.run_ext_trigger(&t, "OnAfterInsert", rec, None)?;
                        }
                        self.table_event(&t, "onafterinsertevent", rec, None, run, None)?;
                        Ok(Value::Boolean(true))
                    }
                    InsertOutcome::AlreadyExists => {
                        // A duplicate key in Active Session: Insert with a return value answers false and leaves an error whose
                        // text is not pinned (metadata/platform-tables/RvActiveSessionWrites.al:136-171, "Error code:
                        // 85132273" on Linux, a suspect). Its statement form is not pinned.
                        if self.store_for(rec).takes_inserts(&t) {
                            Self::restore_row_version(rec, old_version);
                            Self::unstamp(rec, stamped);
                            if self.ret_used {
                                self.last_error = al_builtins::native::unpinned_error();
                                return Ok(Value::Boolean(false));
                            }
                            return unsupported(format!(
                                "a duplicate-key Insert into {} in its statement form: not pinned",
                                t.name
                            ));
                        }
                        // A key that only the column's rounding makes a duplicate: what Insert answers and the error it
                        // names are not pinned (P130DateFilters.al:576-589 pins only a key the column holds as it is).
                        if !rec.borrow().temporary && crate::sqltime::key_rounds(&t.primary_key(&row)) {
                            return unsupported(
                                "an Insert whose Time or DateTime key value the SQL column rounds onto an existing row (not pinned)",
                            );
                        }
                        Self::restore_row_version(rec, old_version);
                        Self::unstamp(rec, stamped);
                        // The value stays consumed; the key it took does not stay on the record.
                        if let (Some(a), true) = (auto, taken) {
                            rec.borrow_mut().fields[a] = self.default_value(&t.fields[a].ty);
                        }
                        // Without a Blob field the platform defers the failure of an unguarded
                        // Insert to the next access of the table or Commit; a guarded Insert
                        // answers false at once, and a Blob field makes it fail at once (corpus
                        // record/TestRecordRuntimeTextsAndViews.al:Record_Insert_DuplicateKey_*).
                        // A temporary table's behaviour is not pinned: it fails at once.
                        let has_blob = self.has_blob_field(&t);
                        // A table with an AutoIncrement field needs the platform's answer at once
                        // (record/TestAutoIncrementContracts.al:AutoIncrement_SecondInsert_...).
                        // A CRM table's duplicate fails at once, through its table connection
                        // (tests/al-language-onprem/session-system/Y16CrmTableConnectionOps.al:224-251, `insert duplicate`).
                        if !self.ret_used
                            && !has_blob
                            && auto.is_none()
                            && !self.tx.immediate_errors
                            && !rec.borrow().temporary
                            && !rec.borrow().crm
                        {
                            match self.fail(&t, &row, true) {
                                Err(Signal::Error(e)) => {
                                    self.defer_insert(rec, t.object, *e);
                                }
                                // A key whose format is not pinned stays a refusal.
                                Err(other) => return Err(other),
                                Ok(_) => {}
                            }
                            // The table-extension OnAfterInsert trigger and the OnAfterInsertEvent subscribers run with the
                            // duplicate's values, after the error is deferred: a read of the table inside them raises it and the
                            // Insert fails (record/keys-sorting/Y38DupKeyAfterEvent.al DK_AfterEvent_Statement_ThenCount,
                            // DK_AfterEvent_InsertFalse, DK_ExtAfterTrigger_Statement, DK_AfterEvent_ReadInsideSubscriber;
                            // events/binding-senders/DbtEvents.al:347 Insert_DuplicateKey).
                            if run {
                                self.run_ext_trigger(&t, "OnAfterInsert", rec, None)?;
                            }
                            self.table_event(&t, "onafterinsertevent", rec, None, run, None)?;
                            return Ok(Value::Boolean(true));
                        }
                        // A guarded Insert that fails on an existing key leaves the variable's SystemId null: the id it was given
                        // and one the code set before are both dropped (events/table-events/UfFailedInsertSystemId.al:
                        // 52-60 `entry sysid null after failed insert` Yes, :124-132 Insert(true) and an explicit SystemId; al-runner
                        // issue 6153). A temporary record and InsertWithSystemId are not pinned and keep what they held.
                        if self.ret_used && !keep_id && !rec.borrow().temporary {
                            // Pinned by events/table-events/UfFailedInsertSystemId.al (was assumption `failed-insert-null-system-id`).
                            if let Some(i) = id_slot {
                                rec.borrow_mut().fields[i] = Value::Guid([0; 16]);
                            }
                        }
                        let r = self.fail(&t, &row, true);
                        self.note_failed_insert(&t, rec, &r);
                        // A guarded Insert of the key whose statement-form Insert is pending drops that pending error: a Commit
                        // or a read after it raises nothing, and GetLastErrorText gives its text (Y38DupKeyAfterEvent.al
                        // DK_AfterEvent_Statement_ThenGuardedInsert,
                        // DK_AfterEvent_Statement_GuardedInsert_ThenCount). Another key's guarded Insert is not pinned.
                        if self.ret_used
                            && matches!(r, Ok(Value::Boolean(false)))
                            && self
                                .tx
                                .deferred_insert
                                .as_ref()
                                .is_some_and(|(o, _)| *o == t.object as usize)
                        {
                            let same_key = format!(
                                "The record in table {} already exists. Identification fields and values: {}",
                                self.caption(&t),
                                self.identification(&t, &row)?
                            );
                            if self
                                .tx
                                .deferred_insert
                                .as_ref()
                                .is_some_and(|(_, e)| e.message == same_key)
                            {
                                // GetLastErrorText then gives the duplicate's text (`guarded`).
                                let (_, e) = self.tx.deferred_insert.take().expect("checked");
                                self.last_error = e;
                            }
                        }
                        // The flag hides the key of a duplicate Insert into a table with a Blob field (error-handling/
                        // lt-last-error-text-forms/InsertDuplicateKeyExcludeFlag.al:68, :85, :102). Other tables are not pinned.
                        if has_blob && auto.is_none() && !rec.borrow().temporary {
                            r.map_err(|s| redact_error(s, format!(
                                "The record in table {} already exists. Identification fields and values: <redacted>",
                                self.caption(&t)
                            )))
                        } else {
                            r
                        }
                    }
                }
            }
            RecordOp::Modify => {
                let run = arg_bool(args, 0)?;
                let from_al = std::mem::take(&mut self.tx.modify_from_al);
                self.tenant_license_state_refuses(&t, "modify")?;
                self.document_service_trigger_refused(&t, rec, run, "Modify")?;
                // A key value the column rounds finds the rounded row (P130DateFilters.al:635-637, `modify var`).
                let pk = Self::sql_key(rec, &t, &rec.borrow().fields);
                let stored = self
                    .store_for(rec)
                    .get(&cx, &t, &pk)
                    .map_err(|e| Signal::unsupported(format!("{e}")))?;
                self.feature_key_read_only_change(&t, rec, stored.as_deref())?;
                if let Some(e) = self.read_only_platform_table(&t, rec, RecordOp::Modify) {
                    return match stored {
                        Some(_) => Err(Signal::Error(Box::new(e))),
                        None => unsupported(format!("a Modify of a {} row that does not exist (not pinned)", t.name)),
                    };
                }
                if let Some(answer) = self.modify_version_checks(&t, rec, &cx, &pk, run, stored.as_deref())? {
                    return Ok(answer);
                }
                // Modify never changes the stored SystemId, whatever the buffer carries
                // (corpus record/TestRecordSystemId.al:176-215, :285-330).
                let (audit, kept) = self.modify_audit(&t, rec, stored.as_deref(), run);
                // One xRec handle for the events, the triggers and the global trigger, so a change a
                // subscriber makes to it reaches the rest. When code drives the Modify, xRec equals
                // Rec (record/TestXRecContracts.al:69-87 OnModify_xRec_MirrorsRecValues_WhenCalledFromCode;
                // record/TestTableEventSequenceContracts.al:ModifyRenameDelete_EventsAndTrigger pins the
                // same for the events). A row a page saves carries the page's own previous row
                // (record/TestXRecDriverContracts.al:32-51 Record_Modify_FromPage_xRecHoldsPreviousValue).
                let stored_row = stored.clone();
                // The buffer's version before this Modify and its origin, for the own Modify after an own Insert or Get below.
                let version_before = t.fields.iter().position(|f| f.id == 2_000_000_005).map(|i| {
                    let r = rec.borrow();
                    (
                        i,
                        if let Value::BigInteger(n) = &r.fields[i] { *n } else { 0 },
                        r.row_origin,
                    )
                });
                if let Some(old) = &stored {
                    self.check_modify_auto_increment(&t, rec, old, run)?;
                }
                // A Modify loads every field of a partially loaded record: the subscribers' Rec and xRec and the caller's record
                // report all fields loaded afterwards, for Modify(true) and Modify(false)
                // (record/events/AfModifyLoadFields.al:92, :112, :130). A record nothing was fetched into is not pinned
                // and stays as it is.
                // What it writes: the loaded and the assigned fields; a field never loaded or assigned keeps the stored value
                // (record/load-fields/P81LoadFields.al:1127-1243, al-runner issue 5764).
                // A row that is gone fails below as RecordNotFound, partial or not (P170JitLoadGone.al:550-600 PG_Modify_*).
                if !rec.borrow().temporary && stored.is_some() {
                    self.load_missing_fields(rec, None)?;
                }
                if let Some(b) = &mut rec.borrow_mut().load.buffer {
                    b.have = al_builtins::value::FieldSet::All;
                    b.unsure = al_builtins::value::FieldSet::Of(Vec::new());
                }
                let before_image = self
                    .pages
                    .page_xrec
                    .take()
                    .unwrap_or_else(|| rec.borrow().fields.clone());
                let xrec = Some(Self::xrec_handle(rec, before_image));
                self.table_event(&t, "onbeforemodifyevent", rec, xrec.as_ref(), run, None)?;
                if run {
                    self.run_ext_trigger(&t, "OnBeforeModify", rec, xrec.as_ref())?;
                    if let Some(tr) = self.table_trigger(&t, "OnModify") {
                        self.run_trigger(t.object as usize, tr, rec, xrec.as_ref())?;
                    }
                    self.run_ext_trigger(&t, "OnModify", rec, xrec.as_ref())?;
                }
                self.global_trigger(&t, GlobalOp::Modify, rec, xrec.as_ref())?;
                if self.is_user_row(&t, rec.borrow().temporary) {
                    self.user_before_modify(&t, rec)?;
                }
                // A Modify that changes no field leaves the row version as it is: Microsoft's tests Modify a record after a refresh report
                // modified its row without a change (Create Prod. Order Lines.Copy) and pass, which a bumped version would make a
                // stale write (record/PriorityAssumptions.al:143-244, real BC: after a Modify that changes nothing and a Commit, the
                // other variable's Modify of a changed field is accepted, while a Modify that changed a field makes it fail with DB:RecordChanged).
                self.check_code_storage(&t, rec, true)?;
                self.check_failed_insert_stale(&t, rec, stored_row.as_deref())?;
                self.sql_timestamp_before(&t, rec, stored_row.as_deref())?;
                // A bulk ModifyAll(.., false) without subscribers stamps SystemModifiedAt of a row that has the value already
                // (RcBTriggers.al:ModifyAll_Equal_Value_Rows_BQ) and gives it a new row version, with or without an OnModify
                // trigger (record/crud/ModifyWithoutChange.al:375-448, `row1_version_changed = Yes`).
                let unchanged = match stored_row.as_deref() {
                    Some(old) if !rec.borrow().temporary && Session::modify_changes_nothing(&t, old, rec) => Some(old),
                    _ => None,
                };
                let old_version = match unchanged {
                    Some(old) if !self.bulk_modify => {
                        if let Some(i) = t.fields.iter().position(|f| f.id == 2_000_000_005) {
                            rec.borrow_mut().fields[i] = old[i].clone();
                        }
                        None
                    }
                    _ => self.bump_row_version(&t, rec),
                };
                self.sql_timestamp_after(&t, rec);
                let keeps_assigned = stored_row.as_deref().is_some_and(|s| {
                    crate::assigned_audit::modify_keeps_assigned_modified(&t, s, &rec.borrow().fields, &audit)
                });
                let (stamped, row_only) = self.stamp_modify(&t, rec, stored_row.as_deref(), keeps_assigned);
                let mut row = rec.borrow().fields.clone();
                for (i, v) in kept.into_iter().chain(row_only) {
                    row[i] = v;
                }
                // A unique secondary key refuses the write after the trigger (corpus
                // record/TestCalcFieldsTruncateUniqueContracts.al:UniqueIndex_Violation_..., Modify).
                if let Some(unique_key) = self.unique_violation(rec, &t, &row, Some(&pk))? {
                    Self::restore_row_version(rec, old_version);
                    Self::unstamp(rec, stamped);
                    return self.fail_unique(&t, unique_key, &row);
                }
                let mut view = self.stored_view(&t, &row);
                // A temporary row keeps the FlowField values a Modify wrote: a Get, a Find and a later CalcFields-free read show them
                // (record/flowfields/DiFwWriteTests.al:606-616, `after Modify`, `after Get`, `Get after CalcFields`).
                // An Insert does not (`insert then Get` = 0).
                if rec.borrow().temporary {
                    let engine = self.engine.clone();
                    let schema = engine.program.objects[t.object as usize].table.as_ref().expect("table");
                    for (i, f) in t.fields.iter().enumerate() {
                        if !f.stored && f.id < 2_000_000_000 && !schema.fields[i].flow_filter {
                            view[i] = row[i].clone();
                        }
                    }
                    // A FlowFilter value is not written by the Modify: the row keeps what the Insert stored
                    // (`flow_filter_positions`, DiFwWriteTests.al:937 `Get(1) after Modify`).
                    if let Some(old) = stored_row.as_deref() {
                        for i in self.flow_filter_positions(&t) {
                            view[i] = old[i].clone();
                        }
                    }
                }
                let mut stored = Self::to_store(rec, view);
                if let Some(bad) = self.sql_columns(rec, &t, &mut stored)? {
                    let shape = ModifyShape {
                        from_al,
                        row_exists: stored_row.is_some(),
                    };
                    return self.fail_modify_sql_range(rec, &t, bad, shape, (old_version, stamped));
                }
                Self::mark_blobs_inserted(rec, &stored, false);
                let before = self.store.change_counter();
                match self
                    .store_for(rec)
                    .modify(&cx, &t, stored)
                    .map_err(|e| Signal::unsupported(format!("{e}")))?
                {
                    WriteOutcome::Done => {
                        self.note_write(rec, &t, crate::state::TxWriteKind::Modify, &pk, before);
                        Self::clear_blob_written(rec);
                        // A variable whose SystemId is null (a guarded Insert failed on the existing key) holds the stored row's SystemId
                        // after the Modify; the row keeps its own (events/table-events/UfFailedInsertSystemId.al:61-70
                        // `entry sysid = stored` Yes, :86-95 the `if not Insert then Modify` idiom; al-runner issue 6153). Another id in
                        // the buffer stays as it was (record/TestRecordSystemId.al:176-215 pins only the stored row).
                        let temporary = rec.borrow().temporary;
                        let id_slot = t.fields.iter().position(|f| f.id == 2_000_000_000);
                        let null_id =
                            id_slot.is_some_and(|i| matches!(rec.borrow().fields[i], Value::Guid(g) if g == [0; 16]));
                        if let (Some(old), false, true, Some(i)) = (stored_row.as_deref(), temporary, null_id, id_slot)
                        {
                            // Pinned by events/table-events/UfFailedInsertSystemId.al (was assumption `failed-insert-null-system-id`).
                            rec.borrow_mut().fields[i] = old[i].clone();
                        }
                        if !rec.borrow().temporary {
                            let image = rec.borrow().fields.clone();
                            rec.borrow_mut().last_modify = Some(image);
                            // The variable's own Modify (false or true, the first or a later one) after its own Insert or Get, in that
                            // transaction, makes the buffer's new version its own write (record/crud/SbStaleOwnModify.al:
                            // 132-264, :410-448). An own Modify in a later transaction leaves the origin behind, as before.
                            if let Some((i, was, Some(o))) = version_before
                                && o.version == was
                            {
                                let now = if let Value::BigInteger(n) = &rec.borrow().fields[i] {
                                    *n
                                } else {
                                    0
                                };
                                if now != was {
                                    let keep = o.commits == self.commits;
                                    rec.borrow_mut().row_origin = keep.then_some(al_builtins::value::RowOrigin {
                                        own_write: true,
                                        version: now,
                                        read_in_write_tx: false,
                                        own_modified: true,
                                        ..o
                                    });
                                }
                            }
                            let company = rec.borrow().company.clone();
                            self.tx.rows_touched.push((t.object, company, pk.clone()));
                        }
                        // The Modify that stores a Media field's new media removes the Tenant Media row of the one it held
                        // (streams/ArRecordProbes.al:141-143). Only a media replaced by another is pinned; a
                        // cleared field and a temporary record keep it.
                        if let (Some(old), false) = (stored_row.as_deref(), rec.borrow().temporary) {
                            for (i, f) in t.fields.iter().enumerate() {
                                if !matches!(f.ty, RtType::Library(al_builtins::library_kind::LibraryKind::Media)) {
                                    continue;
                                }
                                if let (Value::Guid(was), Value::Guid(now)) = (&old[i], &row[i])
                                    && was != now
                                    && *was != [0; 16]
                                    && *now != [0; 16]
                                {
                                    self.release_media(&cx, &t, i, *was)?;
                                }
                            }
                        }
                        // A Blob a Modify of a temporary record persisted does not survive a later Rename
                        // (streams/TestBlobRenameIsolation.al:TempBlob_CommittedWrite_Rename_SameInstanceReGet_LosesBlobValue);
                        // one written after the Insert and never modified does.
                        if rec.borrow().temp_id.is_some() {
                            for v in &rec.borrow().fields {
                                if let Value::Blob(b) = v {
                                    b.borrow_mut().persisted = true;
                                }
                            }
                        }
                        if run {
                            self.run_ext_trigger(&t, "OnAfterModify", rec, xrec.as_ref())?;
                        }
                        self.table_event(&t, "onaftermodifyevent", rec, xrec.as_ref(), run, None)?;
                        Ok(Value::Boolean(true))
                    }
                    WriteOutcome::NotFound => {
                        Self::restore_row_version(rec, old_version);
                        Self::unstamp(rec, stamped);
                        self.fail(&t, &row, false)
                    }
                }
            }
            RecordOp::Delete => {
                let run = arg_bool(args, 0)?;
                self.tenant_license_state_refuses(&t, "delete")?;
                self.document_service_trigger_refused(&t, rec, run, "Delete")?;
                if let Some(e) = self.read_only_platform_table(&t, rec, RecordOp::Delete) {
                    let row = rec.borrow().fields.clone();
                    let pk = Self::sql_key(rec, &t, &row);
                    let exists = self
                        .store_for(rec)
                        .get(&cx, &t, &pk)
                        .map_err(|e| Signal::unsupported(format!("{e}")))?
                        .is_some();
                    return if exists {
                        Err(Signal::Error(Box::new(e)))
                    } else {
                        unsupported(format!("a Delete of a {} row that does not exist (not pinned)", t.name))
                    };
                }
                // A Delete loads the missing fields of a partially loaded record first: afterwards they read as loaded, with the
                // values of the row it deleted (record/load-fields/LoadFieldsFetched.al:270-289,
                // P170_JitLoad_After_Delete_Of_The_Same_Partial_Record_BQ, Delete without triggers). Delete(true) does the same:
                // OnBeforeDeleteEvent, OnDelete, OnAfterDeleteEvent and the caller's buffer afterwards read the stored value
                // (record/load-fields/P170JitLoadGone.al WA_Delete_FindSetForUpdate_DeleteTrue, WA_Delete_Control_PlainTable,
                // WA_Delete_Control_TriggerTable; al-runner issue 6924).
                // A row an AL Insert stored is deleted as a stored row (streams/media/Y48TenantMediaRows.al:648).
                if self.is_tenant_media(&t) && !rec.borrow().temporary && !self.tenant_media_stored_key(&t, rec)? {
                    return self.delete_tenant_media(&t, rec, run);
                }
                if !rec.borrow().temporary {
                    self.load_missing_fields(rec, None)?;
                }
                self.delete_one(&cx, &t, rec, run)
            }
            RecordOp::DeleteAll => {
                let at = self.note_bulk_write(rec, &t, crate::state::TxWriteKind::DeleteAll);
                let r = arg_bool(args, 0).and_then(|run| self.delete_all(&t, rec, &cx, run));
                self.note_bulk_end(at);
                r
            }
            RecordOp::Get => {
                // `Get(RecordId)`: the row the RecordId names, as RecordRef.Get(RecordId) does (corpus
                // recordref-fieldref/recordref/TestRecordRefKeys.al:48). The Record form is pinned for a row that exists
                // (record/TestSetAutoCalcFieldsOnGet.Codeunit.al:68-79; Base Application's Item
                // Category Management does `ItemCategory.Get(RecId)`).
                let from_id: Option<Vec<Value>> = match args {
                    [Value::Library(_, l)] => match &*l.borrow() {
                        al_builtins::value::LibraryValue::RecordId { table, key, .. } if *table == t.object => {
                            Some(key.clone())
                        }
                        al_builtins::value::LibraryValue::RecordId { table, .. } => {
                            return self.get_other_tables_record_id(&t, *table as usize);
                        }
                        // An empty RecordId finds no row: false, no error (record/lt-record-conversion-assumptions/
                        // GetRecordId.al: P98_Get_RecordId_Empty). With the result unused the outcome is not pinned.
                        al_builtins::value::LibraryValue::Empty if self.ret_used => return Ok(Value::Boolean(false)),
                        _ => None,
                    },
                    _ => None,
                };
                let args = from_id.as_deref().unwrap_or(args);
                // More values than the primary key has fields raise, with the result used and unused
                // (record/builtin-name-procedures/Y36BuiltinNameSameArity.al:131-157 Y36_SameArity_Get_IntegerArgs: two
                // Integers on a one-field key of a table without a Caption). A table with a Caption is not pinned (name or caption).
                if from_id.is_none() && args.len() > t.keys[0].len() {
                    let o = &self.engine.program.objects[t.object as usize];
                    if o.properties
                        .iter()
                        .any(|(k, _)| k.eq_ignore_ascii_case("Caption") || k == al_ir::EXT_CAPTION_OVER_OWN)
                    {
                        return unsupported(
                            "Get with more values than the primary key has fields, on a table with a Caption (not pinned)",
                        );
                    }
                    return Err(crate::transactions::al_error_code(
                        &format!(
                            "Too many key fields were specified, so \"{}\" could not be retrieved. The number of fields in the primary key is {}.",
                            t.name,
                            t.keys[0].len()
                        ),
                        "AL:ToManyKeyFields",
                    ));
                }
                let key = self.get_key(rec, &t, args, from_id.is_some())?;
                let mut pk = key.pk.clone();
                // A Time or DateTime key value the SQL column rounds looks for the rounded row: Get('A', 10:00:00.001) and
                // Get('A', 10:00:00) find the row stored from 10:00:00.001, Get('A', 10:00:00.002) does not, as it looks for
                // 10:00:00.003 (record/filters/P130DateFilters.al:603-605, :617-618, :722-723). Through a RecordId
                // it is not pinned.
                let get_rounds = !rec.borrow().temporary && crate::sqltime::key_rounds(&pk);
                if get_rounds {
                    if from_id.is_some() {
                        return unsupported(
                            "Get of a RecordId whose Time or DateTime key value the SQL column rounds (not pinned)",
                        );
                    }
                    pk = crate::sqltime::rounded_key(&pk);
                }
                match self
                    .store_for(rec)
                    .get(&cx, &t, &pk)
                    .map_err(|e| Signal::unsupported(format!("{e}")))?
                {
                    Some(row) => {
                        self.get_load = true;
                        let loaded = self.load(rec, row);
                        self.get_load = false;
                        loaded?;
                        Ok(Value::Boolean(true))
                    }
                    // Not found, by a key value the column rounds: only the answer false is pinned (P130DateFilters.al:605), not the error.
                    None if get_rounds && !self.ret_used => unsupported(
                        "the not-found error of a Get by a Time or DateTime key value the SQL column rounds (not pinned)",
                    ),
                    None => {
                        // A row that is there: record/TestSetAutoCalcFieldsOnGet.Codeunit.al:68-79; a row that is not: the
                        // error of Get(key) (record/crud/GetRecordId.al: P98_Get_RecordId_Missing_*).
                        let held = rec.borrow().fields.clone();
                        let ident = self.get_not_found_ident(&t, args, &key, held)?;
                        // GetLastErrorObject.al:232-255: the exception of a Get without a row.
                        self.fail(&t, &ident, false).map_err(|s| match s {
                            Signal::Error(mut e) if e.code == "DB:RecordNotFound" => {
                                e.exception = al_builtins::native::ExceptionKind::RecordNotFound;
                                Signal::Error(e)
                            }
                            other => other,
                        })
                    }
                }
            }
            RecordOp::FindFirst | RecordOp::FindSet | RecordOp::FindLast | RecordOp::Find => {
                let via = empty_find_origin::FindVia::take(self);
                let asc = {
                    let r = rec.borrow();
                    r.ascending
                };
                let which = match (method, args.first()) {
                    (RecordOp::FindLast, _) => "+".to_string(),
                    (RecordOp::Find, Some(v)) => al_builtins::native::text_of(v).unwrap_or_default().to_string(),
                    // Doc (Record.Find): Which defaults to '='.
                    (RecordOp::Find, None) => "=".to_string(),
                    _ => "-".to_string(),
                };
                // Only `-` or `+` alone, or one to three of `=`, `<` and `>` with none repeated, are directions; anything
                // else, the empty text included, raises (record/PZFindDirections.al:Find_InvalidDirections and
                // Find_InvalidDirection_ErrorText, Record and RecordRef alike; the text carries the backslashes as BC
                // writes them).
                let valid = match which.as_str() {
                    "-" | "+" => true,
                    w => {
                        !w.is_empty()
                            && w.chars().all(|c| matches!(c, '=' | '<' | '>'))
                            && w.char_indices().all(|(i, c)| !w[i + 1..].contains(c))
                    }
                };
                if !valid {
                    return Err(Signal::Error(Box::new(AlError {
                        message: format!(
                            "There is an invalid character in the Find expression '{which}'.\\\\In Find expressions, you can use =, >, and < in combinations, where each sign can only appear once, or you can use - or +."
                        ),
                        code: "DB:BadSearchMethod".into(),
                        call_stack: String::new(),
                        ..Default::default()
                    })));
                }
                let found = match which.as_str() {
                    "-" => self.scan(rec, &t, asc, None)?,
                    "+" => self.scan(rec, &t, !asc, None)?,
                    // Every form honours the active filters
                    // (record/TestFilterTextBooleanFindContracts.al:143, Find_AllForms_HonourTheActiveFilters).
                    // The one exception: a record that moved its own row out of its filter still
                    // finds the row by its key, while a second variable with the old filter does
                    // not (record/TestFindSetSortKeyModify.al:OwnFilter_ModifiedOutOfIt_FindStillFindsTheRow
                    // against OwnFilter_SecondVarWithTheSameFilter_FindDoesNotFindTheRow). A record
                    // remembers the key of the row it last read as its position.
                    "=" => {
                        let fields = rec.borrow().fields.clone();
                        let pk = Self::exact_key(rec, &t, &fields, "Find('=')")?;
                        let row = self
                            .store_for(rec)
                            .get(&cx, &t, &pk)
                            .map_err(|e| Signal::unsupported(format!("{e}")))?;
                        match row {
                            Some(row) if self.row_passes(rec, &t, &row)? || Self::on_own_row(rec, &pk)? => Some(row),
                            _ => None,
                        }
                    }
                    // `>` and `<`: the first row past the buffer's position in the current key.
                    ">" | "<" => {
                        let key = rec.borrow().key;
                        let pos = position(&t, key, &rec.borrow().fields);
                        self.scan(rec, &t, (which == ">") == asc, Some(pos))?
                    }
                    // Two or three direction characters are tried in the order written, and the first that finds a row
                    // wins: `=` the row itself, `<` the row before it, `>` the row after it. So `<=` answers the
                    // previous row even when the key exists, and `=<>` takes the previous row before the next one
                    // (types/PYLoadFindThis.al:Find_DirectionCharacters_Orders, all eleven orders on BC 27.5 and 29.0;
                    // the neighbour search of a row that is gone: pages/currpage/TestPageUpdateGoneRow.al:423,
                    // events/TestProbeEventContracts.al:Find_NeighbourSearch_*).
                    w if w.len() >= 2 && w.chars().all(|c| matches!(c, '<' | '=' | '>')) => {
                        let key = rec.borrow().key;
                        let pos = position(&t, key, &rec.borrow().fields);
                        let mut found = None;
                        for c in w.chars() {
                            found = match c {
                                // Another current key looks for the neighbour by that key's fields
                                // (events/TestProbeEventContracts.al:Find_NeighbourSearch_WithFilterAndOtherKey, key Name).
                                // The row itself is the one with the record's primary key whose fields of that key still
                                // hold the record's values and that passes the filters (pages/triggers/
                                // NewRowListSecondaryKey.al:P197_SecKey_PositionedAfterFindFirst, _PositionedAfterFindLast,
                                // NewRowListTemporary.al:P197_Temp_SecondaryKeyAfterFindFirst: a list over a secondary key opens on
                                // the positioned record). Base Application's Req. Wksh.-Make Order `Code` needs it as well.
                                '=' => {
                                    let fields = rec.borrow().fields.clone();
                                    let pk = Self::exact_key(rec, &t, &fields, "Find")?;
                                    let exact = self
                                        .store_for(rec)
                                        .get(&cx, &t, &pk)
                                        .map_err(|e| Signal::unsupported(format!("{e}")))?;
                                    match exact {
                                        Some(row) if self.row_passes(rec, &t, &row)? => {
                                            if key == 0 || Self::same_key_values(&t, key, &row, &pos) {
                                                Some(row)
                                            } else {
                                                None
                                            }
                                        }
                                        _ => None,
                                    }
                                }
                                '<' => self.scan(rec, &t, !asc, Some(pos.clone()))?,
                                _ => self.scan(rec, &t, asc, Some(pos.clone()))?,
                            };
                            if found.is_some() {
                                break;
                            }
                        }
                        found
                    }
                    other => return unsupported(format!("Find('{other}')")),
                };
                match found {
                    Some(row) => {
                        // Every form of Find gives the origin FindFirst gives (record/stale-find/Y89StaleFind.al:256-480,
                        // Y89_Part1_find_*, against record/crud/Y76StaleBuffers.al Y76_Part1_*).
                        self.find_load = true;
                        let loaded = self.load(rec, row);
                        self.find_load = false;
                        loaded?;
                        Ok(Value::Boolean(true))
                    }
                    None if self.ret_used => Ok(Value::Boolean(false)),
                    // The error BC raises for an empty set has no corpus test (no `asserterror` on a Find call):
                    // its text is not pinned (docs/probes-needed.md).
                    // events/TestProbeEventContracts.al:RecordErrors_GetMissing_TestField_Modify (FindFirst, FindLast, FindSet
                    // with a filter): the text lists the filters; without one it is not pinned.
                    None => {
                        let filters = self.get_filters(rec, &t)?;
                        // Without a filter the table itself is empty, on every version: FindFirst and FindLast
                        // record/filters/P68C_EmptyFinds.al:215-250 ( passes on the BC 27 oracle
                        // too), FindSet ErTTriggerErrors.al:140-147 and P68C_EmptyFinds.al:202-204.
                        // Find('-') and Find('+') of an empty, unfiltered table say so as well
                        // (record/crud/FindEmptyTable.al:Find_EmptyTable `unused_first`, `unused_last`).
                        if filters.is_empty()
                            && (matches!(method, RecordOp::FindFirst | RecordOp::FindLast | RecordOp::FindSet)
                                || (method == RecordOp::Find && matches!(which.as_str(), "-" | "+")))
                        {
                            let origin = self.table_is_empty_origin(empty_find_origin::EmptyFind {
                                via,
                                method,
                                which: &which,
                                temporary: rec.borrow().temporary,
                            });
                            return Err(Signal::Error(Box::new(AlError {
                                message: format!("The {} table is empty.", self.caption(&t)),
                                code: "DB:TableIsEmpty".into(),
                                call_stack: String::new(),
                                origin,
                                ..Default::default()
                            })));
                        }
                        // The search strings the corpus pins on an empty table: none or `=` (the key alone), `>`, `<` and `=><`
                        // (FindEmptyTable.al:Find_EmptyTable `unused_default` .. `unused_all_directions`). On a table that has rows
                        // where no row meets the search, on an empty table with a key value other than the default, and `=>` and
                        // `=<`, the same texts with the record's key (writer TV, corpus branch al-runtime-rs-staging-tv 8ae7ac0b,
                        // FindEmptyTable.al TV_Find_TableWithRows_NoMatch, TV_Find_EmptyTable_KeyNotDefault). The other
                        // combinations are not pinned.
                        let empty_table = filters.is_empty()
                            && method == RecordOp::Find
                            && self
                                .store_for(rec)
                                .count(&cx, &t, &[])
                                .map_err(|e| Signal::unsupported(format!("{e}")))?
                                == 0;
                        // A set filtered down to nothing says the same (recordref-fieldref/Y81EmptySetFinds.al:224-:520
                        // Y81_RecRef_FilteredOut_Unused: `=`, `>` and `<` on a RecordRef and a temporary Record; needs corpus bump).
                        let filtered_miss = !filters.is_empty() && matches!(which.as_str(), "=" | ">" | "<");
                        if (filters.is_empty() || filtered_miss)
                            && method == RecordOp::Find
                            && matches!(which.as_str(), "=" | ">" | "<" | "=><" | "=>" | "=<")
                        {
                            let fields = rec.borrow().fields.clone();
                            let pk = t.primary_key(&fields);
                            let default_key = pk.iter().all(|v| crate::recmeta::is_blank(v).unwrap_or(false));
                            if !empty_table || !default_key || matches!(which.as_str(), "=>" | "=<") {
                                // Pinned from codeunit code only: in a page's code (a trigger such as OnNewRecord of a page a
                                // TestPage opens) what BC does with the error is not pinned for these shapes.
                                let engine = self.engine.clone();
                                if self.frames.iter().any(|&o| {
                                    matches!(
                                        engine.program.objects[o].kind,
                                        al_ir::ObjKind::Page | al_ir::ObjKind::PageExtension
                                    )
                                }) {
                                    return unsupported(format!(
                                        "Find on table '{}' in page code, no row meeting the search, without using the result: not pinned",
                                        t.name
                                    ));
                                }
                                // Pinned by record/crud/FindEmptyTable.al TV_Find_TableWithRows_NoMatch, TV_Find_EmptyTable_KeyNotDefault (was assumption `find-miss-key-text`).
                            }
                            let id = self.identification(&t, &fields)?;
                            let message = if which == "=" {
                                format!(
                                    "The {} does not exist. Identification fields and values: {id}",
                                    self.caption(&t)
                                )
                            } else {
                                format!(
                                    "The {} does not exist. {which}{BR}{BR}Identification fields and values:{BR}{BR}{id}",
                                    self.caption(&t)
                                )
                            };
                            let find = empty_find_origin::EmptyFind {
                                via,
                                method,
                                which: &which,
                                temporary: rec.borrow().temporary,
                            };
                            let origin = self.record_not_found_origin(&t, find, empty_table, default_key);
                            return Err(Signal::Error(Box::new(AlError {
                                message,
                                code: "DB:RecordNotFound".into(),
                                call_stack: String::new(),
                                origin,
                                ..Default::default()
                            })));
                        }
                        // Find('-') and Find('+') of a filtered set too (Y81EmptySetFinds.al Y81_RecRef_FilteredOut_Unused).
                        if filters.is_empty()
                            || !(matches!(method, RecordOp::FindFirst | RecordOp::FindLast | RecordOp::FindSet)
                                || (method == RecordOp::Find && matches!(which.as_str(), "-" | "+")))
                        {
                            return unsupported(format!(
                                "{method} on an empty set of table '{}' without using the result and without a filter: BC's error text is not pinned (docs/probes-needed.md)",
                                t.name
                            ));
                        }
                        let find = empty_find_origin::EmptyFind {
                            via,
                            method,
                            which: &which,
                            temporary: rec.borrow().temporary,
                        };
                        let origin = self.nothing_inside_filter_origin(&t, find);
                        Err(Signal::Error(Box::new(AlError {
                            message: format!(
                                "There is no {} within the filter.{BR}{BR}Filters: {filters}",
                                self.caption(&t)
                            ),
                            code: "DB:NothingInsideFilter".into(),
                            call_stack: String::new(),
                            origin,
                            ..Default::default()
                        })))
                    }
                }
            }
            RecordOp::Next => {
                let steps = match args.first() {
                    Some(Value::Integer(n)) => *n,
                    // A whole-number Decimal steps like the Integer (`Calendar.Next(12 / CountFrequency)`); one with a fraction
                    // raises and leaves the row (errors/PnF11DateDecimalArithmetic.al:107-175,
                    // Next_Decimal_Whole: 1, 2, -1, -2, 12 / 4, 0, 20 past the end; Next_Decimal_Fraction: 1.5, -1.5, 0.4, ...).
                    Some(Value::Decimal(d)) if d.fract().is_zero() && i32::try_from(d.trunc()).is_ok() => {
                        i32::try_from(d.trunc()).unwrap_or(0)
                    }
                    Some(Value::Decimal(d)) if !d.fract().is_zero() => {
                        return Err(Signal::Error(Box::new(AlError {
                            message: format!(
                                "Overflow under conversion of Microsoft.Dynamics.Nav.Runtime.Decimal18 value {d} to System.Int32."
                            ),
                            code: "ConversionCast".into(),
                            call_stack: String::new(),
                            ..Default::default()
                        })));
                    }
                    None => 1,
                    other => return unsupported(format!("Next({})", other.describe())),
                };
                let (key, asc) = {
                    let r = rec.borrow();
                    (r.key, r.ascending)
                };
                let forward = (steps >= 0) == asc;
                let mut moved = 0;
                for _ in 0..steps.unsigned_abs().max(if steps == 0 { 0 } else { 1 }) {
                    let pos = position(&t, key, &rec.borrow().fields);
                    match self.scan(rec, &t, forward, Some(pos))? {
                        Some(row) => {
                            // A row Next loads gives the origin FindFirst gives (record/stale-find/Y89StaleFind.al:408-484,
                            // Y89_Part1_next, Y89_Part1_findset_next, Y89_Part1_descending_next; :560-578 transfer_next).
                            self.find_load = true;
                            let loaded = self.load(rec, row);
                            self.find_load = false;
                            loaded?;
                            moved += 1;
                        }
                        None => break,
                    }
                }
                Ok(Value::Integer(if steps < 0 { -moved } else { moved }))
            }
            RecordOp::Count | RecordOp::IsEmpty => {
                let (filters, flow_field_filters) = self.split_filters(rec);
                let n = if flow_field_filters.is_empty() && !rec.borrow().marked_only {
                    self.store_for(rec)
                        .count(&cx, &t, &filters)
                        .map_err(|e| Signal::unsupported(format!("{e}")))?
                } else {
                    // Rows are tested one by one against the calculated FlowFields.
                    let mut n = 0;
                    let mut after = None;
                    while let Some(row) = self.scan(rec, &t, true, after)? {
                        n += 1;
                        if method == RecordOp::IsEmpty {
                            break;
                        }
                        after = Some(position(&t, rec.borrow().key, &row));
                    }
                    n
                };
                Ok(if method == RecordOp::Count {
                    Value::Integer(n as i32)
                } else {
                    Value::Boolean(n == 0)
                })
            }
            RecordOp::SetRange => {
                let pos = arg_pos(args, 0)?;
                let ty = t.fields[pos].ty.clone();
                let or_flowfield = self.is_flowfield(&t, pos);
                self.blob_setrange_check(&ty, &args[1..])?;
                let mut r = rec.borrow_mut();
                r.position = None;
                let group = r.filter_group;
                r.filters.retain(|f| !(f.field == pos && f.group == group));
                // A FlowField filter in the OR group is dropped: GetFilters does not show it and no row is filtered by it
                // (record/PWFilterGroupOr.al:OrGroup_FlowFieldFilter_BQ, `getFilters`, `flowFieldOnly`, `nameOrFlowField`).
                if group == al_builtins::filter::OR_GROUP && or_flowfield {
                    return Ok(Value::Null);
                }
                if let Some(range) = super::setrange_filter::setrange_filter(&ty, &args[1..])? {
                    r.filters.push(FieldFilter::new(pos, range, group));
                }
                Ok(Value::Null)
            }
            RecordOp::SetFilter => {
                let from_al = std::mem::take(&mut self.setfilter_from_al);
                let pos = arg_pos(args, 0)?;
                rec.borrow_mut().position = None;
                let ty = t.fields[pos].ty.clone();
                let text = crate::filter_text_arg::filter_text(args.get(1));
                // An empty filter text removes the field's filter: a Query column does so
                // (queries/TestQueryRuntimeContracts.al:208-209, "an empty SetFilter clears the filter"), and the
                // documentation says the same of a Record, also in a filter group other than 0 (types/duration/
                // A_BlankFilter.al: Blank_AfterSetFilterGt_Group0, Blank_Group2_AfterSetFilterGt).
                // `==` on a Text field keeps no filter either; `=` raises (`setfilter_equals_text`).
                if text.trim().is_empty() || self.setfilter_equals_text(&t, pos, &text, args.len())? {
                    let mut r = rec.borrow_mut();
                    let group = r.filter_group;
                    r.filters.retain(|f| !(f.field == pos && f.group == group));
                    return Ok(Value::Null);
                }
                let enums = self.enum_members_fn();
                let date_ctx = self.date_filter_context_for(&text);
                let parse = |text: &str| {
                    al_builtins::filter::with_date_context(date_ctx, || {
                        al_builtins::filter::parse_filter_with(text, &args[2..], &ty, &enums)
                    })
                };
                // A RecordId text (`Format(RecordId)`, alone or after `<>`): see `record_id_filter_text`.
                let record_id = if matches!(ty, RtType::Library(al_builtins::library_kind::LibraryKind::RecordId))
                    && args.len() <= 2
                {
                    self.record_id_filter_text(text.trim())?
                } else {
                    None
                };
                let mut parsed = match record_id {
                    Some(p) => p,
                    None => parse(&text),
                };
                // A Date text with a piece no filter reads asks `MakeDateFilter` subscribers first.
                if matches!(&parsed, Err(e) if e.starts_with(al_builtins::filter::INVALID_DATE))
                    && matches!(ty, RtType::Date)
                    && self
                        .engine
                        .events
                        .is_subscribed(crate::events::FILTER_HELPER_TRIGGERS, "makedatefilter")
                {
                    // Outside the pinned shape (an English session, no placeholders) the event is not raised, as before.
                    if self.global_language == 1033 && args.len() <= 2 {
                        if let Some(fixed) = self.repair_date_filter_text(&text, &|t| parse(t).is_ok())? {
                            parsed = parse(&fixed);
                        }
                    }
                }
                // An Integer or Time text of letters asks `MakeIntFilter` / `MakeTimeFilter` subscribers first (PtObserve.al:Bad_Int,
                // Bad_Time); what BC asks `MakeDateTimeFilter` is pinned only in part (the second call of Bad_DateTime m2 carries a
                // text made from the work date), so a DateTime text that raises it is refused; Y62DateFilterShort.al:SetFilter_DateTime_TextForms pins the error for the shapes `english_datetime` rejects.
                // A number filter with a placeholder kept as written next to a wildcard has placeholders, so with subscribers it is refused below as well: whether BC raises the event for it is not pinned.
                if matches!(&parsed, Err(e) if e.starts_with(al_builtins::filter::NOT_EVALUATED)
                    || e.starts_with(al_builtins::filter::VARIABLE_NOT_UNDERSTOOD))
                {
                    let event = match ty {
                        RtType::Integer => Some(("makeintfilter", "IntFilterText")),
                        RtType::Time => Some(("maketimefilter", "TimeFilterText")),
                        RtType::DateTime if !dt_rejected(&text) => Some(("makedatetimefilter", "DateTimeFilterText")),
                        _ => None,
                    };
                    if let Some((event, param)) = event {
                        if self
                            .engine
                            .events
                            .is_subscribed(crate::events::FILTER_HELPER_TRIGGERS, event)
                        {
                            if matches!(ty, RtType::DateTime) {
                                return unsupported(format!(
                                    "SetFilter of `{}` on a DateTime field with MakeDateTimeFilter subscribers (not pinned)",
                                    text.trim()
                                ));
                            }
                            if self.global_language == 1033 && args.len() <= 2 {
                                let failing = parsed
                                    .as_ref()
                                    .err()
                                    .and_then(|e| al_builtins::filter::not_evaluated_value(e))
                                    .unwrap_or_default()
                                    .to_string();
                                if let Some(fixed) =
                                    self.repair_word_filter_text(event, param, &text, &failing, &|t| {
                                        parse(t).map(|_| ())
                                    })?
                                {
                                    parsed = parse(&fixed);
                                }
                            } else {
                                // Another session language or placeholders: whether BC raises the event is not pinned.
                                return unsupported(format!(
                                    "SetFilter of `{}` with {event} subscribers in session language {} or with placeholders (not pinned)",
                                    text.trim(),
                                    self.global_language
                                ));
                            }
                        }
                    }
                }
                let expr = match parsed {
                    Ok(e) => e,
                    // record/crud/RcAFilters.al:SetFilter_Integer_Invalid, events/platform-triggers/PtObserve.al:Bad_Int,
                    // Bad_Decimal, Bad_Time, Bad_DateTime m0.
                    Err(e) if e.starts_with(al_builtins::filter::NOT_EVALUATED) && self.global_language == 1033 => {
                        let (type_name, value) = e[al_builtins::filter::NOT_EVALUATED.len()..]
                            .split_once('\t')
                            .expect("type and value");
                        let reason = format!("The value \"{value}\" can't be evaluated into type {type_name}.");
                        return Err(self.filter_error(&t, pos, text.trim(), &reason));
                    }
                    // A number filter that starts with a placeholder kept as written next to a wildcard
                    // (record/filters/W4Filters.al:694 `num_star_after`).
                    Err(e)
                        if e.starts_with(al_builtins::filter::VARIABLE_NOT_UNDERSTOOD)
                            && self.global_language == 1033 =>
                    {
                        let value = &e[al_builtins::filter::VARIABLE_NOT_UNDERSTOOD.len()..];
                        let reason = format!("The variable \"{value}\" was not understood.");
                        return Err(self.filter_error(&t, pos, text.trim(), &reason));
                    }
                    // The German error of a date no Date filter reads (P143Date_DePlain.al, _DeRanges.al, _DeFormulas.al).
                    Err(e) if e.starts_with(al_builtins::filter::INVALID_DATE) && self.global_language == 1033 => {
                        // P81DateFilters.al:SetFilter_Date_InvalidTexts.
                        let bad = &e[al_builtins::filter::INVALID_DATE.len()..];
                        let reason = format!(
                            "The format of the Date '{bad}' does not match your device's date settings.\\\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format 'yyyy-mm-dd'."
                        );
                        return Err(self.filter_error(&t, pos, text.trim(), &reason));
                    }
                    Err(e) if e.starts_with(al_builtins::filter::FILTER_SYNTAX) && self.global_language == 1033 => {
                        let inner = &e[al_builtins::filter::FILTER_SYNTAX.len()..];
                        let (shown, inner) = match inner.strip_prefix(al_builtins::filter::AS_WRITTEN) {
                            Some(i) => (text.as_str(), i),
                            None => (text.trim(), inner),
                        };
                        return Err(self.filter_error(&t, pos, shown, inner));
                    }
                    Err(e) if e.starts_with(al_builtins::filter::INVALID_DATE) => {
                        let bad = &e[al_builtins::filter::INVALID_DATE.len()..];
                        return Err(Signal::Error(Box::new(AlError {
                            message: format!(
                                "Der Filter '{}' ist für das Feld '{}' der Tabelle '{}' nicht gültig. Das Format Date „{bad}“ stimmt nicht mit den Datumseinstellungen Ihres Geräts überein.\\\\Stellen Sie sicher, dass Tag, Monat und Jahr die richtige Reihenfolge aufweisen und das richtige Trennzeichen verwendet wird (z. b. - oder /). Alternativ können Sie das Format „jjjj-mm-tt“ verwenden.",
                                text.trim(),
                                self.field_caption(&t, pos),
                                self.caption(&t)
                            ),
                            code: "Filter:CSide".into(),
                            call_stack: String::new(),
                            ..Default::default()
                        })));
                    }
                    // record/TestFilterTextBooleanFindContracts.al:130 (SetFilter_BooleanField_UnknownWord_Throws).
                    // A leading blank stays in the quoted filter (format-evaluate/EvalLastError.al:501-539,
                    // BT_Evaluate_Boolean_LeadingBlank `filter [ Da]`); a trailing one is not pinned.
                    Err(e) if e.starts_with(al_builtins::filter::INVALID_BOOLEAN) => {
                        let word = &e[al_builtins::filter::INVALID_BOOLEAN.len()..];
                        return Err(self.boolean_filter_error(&t, pos, &text, word));
                    }
                    // types/date-time/P143Guid_Filters.al:P143_GuidInvalid, P143Date_FieldRef.al:P143_FieldRefGuid.
                    Err(e) if e.starts_with(al_builtins::filter::INVALID_GUID) => {
                        let reason = "Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits.";
                        return Err(self.filter_error(&t, pos, text.trim(), reason));
                    }
                    // An empty side of `|` in the measured shapes (`setfilter_empty_or_side`); every other text main refused.
                    Err(e) if e == al_builtins::filter::EMPTY_TERM => {
                        return Err(self
                            .setfilter_empty_or_side(from_al, &t, pos, rec, args)
                            .unwrap_or_else(|| Signal::unsupported(format!("SetFilter: {e}"))));
                    }
                    Err(e) => return Err(Signal::unsupported(format!("SetFilter: {e}"))),
                };
                let or_flowfield = self.is_flowfield(&t, pos);
                let mut r = rec.borrow_mut();
                let group = r.filter_group;
                r.filters.retain(|f| !(f.field == pos && f.group == group));
                // PWFilterGroupOr.al:OrGroup_FlowFieldFilter_BQ (`nameOrFlowField`: SetFilter too).
                if group == al_builtins::filter::OR_GROUP && or_flowfield {
                    return Ok(Value::Null);
                }
                r.filters.push(FieldFilter {
                    lang: al_builtins::filter::filter_language(),
                    field: pos,
                    expr,
                    group,
                });
                Ok(Value::Null)
            }
            RecordOp::Reset => {
                // Reset also sets the table's globals back to their initial values (ADR-021
                // decision 4; corpus probe TableGlobals_Reset_WhatHappensToTheGlobals).
                self.reset_table_globals(rec);
                // SetAutoCalcFields survives Reset and Clear (corpus record/TestCalcFieldsTruncateUniqueContracts.al:
                // SetAutoCalcFields_SurvivesResetAndClear).
                let mut r = rec.borrow_mut();
                r.filters.clear();
                // The filter group goes back to 0 (recordref-fieldref/PnE08RecordMisc.al:RecordRef_FilterGroup `afterReset`).
                r.filter_group = 0;
                // Reset drops the marks too (record/crud/RcAMisc.al:ClearMarks_And_Reset `markAfterReset`).
                r.marks.clear();
                r.marked_only = false;
                r.key = 0;
                r.adhoc = None;
                r.ascending = true;
                r.field_descending = 0;
                // Reset sets ReadIsolation back to Default (record/CvReadIsolationState.al:43-44, :58).
                r.read_isolation = 0;
                // Reset drops the SetLoadFields request: the next fetch loads every field, the fields in hand stay loaded until
                // then (record/ResetClearsLoadFields.al:59-179, after AddLoadFields and with filters too; Init
                // keeps the request, :181, and Clear drops it, :196).
                r.load.request = None;
                Ok(Value::Null)
            }
            RecordOp::SetCurrentKey => {
                let mut wanted: Vec<usize> = (0..args.len()).map(|i| arg_pos(args, i)).collect::<Result<_, _>>()?;
                // Only the table's own keys: `t` may end with this record's earlier ad hoc key, and a
                // prefix of it is a new ad hoc key, not that one.
                let declared = self.engine.tables[t.object as usize]
                    .as_ref()
                    .map_or(t.keys.len(), |e| e.keys.len());
                // Fields followed by the whole primary key sort as the key of those fields alone, when the table
                // declares that key: the view and the SetAscending error name only them (record/keys-sorting/
                // SetAscendingTies.al:173-174, SetCurrentKey(Grp, Id) with primary key Id and a key on Grp).
                {
                    let pk = &t.keys[0];
                    let n = wanted.len();
                    if n > pk.len()
                        && wanted[n - pk.len()..] == pk[..]
                        && wanted[..n - pk.len()].iter().all(|p| !pk.contains(p))
                    {
                        let head = &wanted[..n - pk.len()];
                        if t.keys[..declared].iter().any(|k| k[..] == *head) {
                            wanted.truncate(n - pk.len());
                        }
                    }
                }
                match t.keys[..declared]
                    .iter()
                    .position(|k| k.len() >= wanted.len() && k[..wanted.len()] == wanted[..])
                {
                    Some(k) => {
                        let mut r = rec.borrow_mut();
                        // Another sorting starts ascending again; the same one keeps its directions
                        // (record/filters/P68D_Ascending.al:Reset_And_SetCurrentKey_Reset_Direction).
                        if r.key != k || r.adhoc.is_some() {
                            r.field_descending = 0;
                        }
                        r.key = k;
                        r.adhoc = None;
                        Ok(Value::Boolean(true))
                    }
                    // Fields no key starts with are allowed and order the rows by themselves, then
                    // by the primary key (record/TestRecordNavContracts.al:180-195,
                    // xmlports/TestXmlPortTriggersReplaceViewContracts.al:339-375).
                    None => {
                        let schema = self.engine.program.objects[t.object as usize]
                            .table
                            .as_ref()
                            .expect("table");
                        // A FlowFilter field and a Blob cannot order rows: `Sorting cannot be done based on the
                        // <field> field(s) in the <table> table.` (code DB:Ok; record/W4SortKeys.al:SetCurrentKey_NotStoredFields
                        // and record/ProbeClearAllSetKeyHyperlink.al:276-318, BC 27.5 and 29.0). A FlowField and a Media
                        // field are accepted and leave the current key the primary key (`CurrentKey()` answers `[Id]`).
                        let is_blob = |p: usize| {
                            matches!(
                                &t.fields[p].ty,
                                RtType::Library(al_builtins::library_kind::LibraryKind::Blob)
                            )
                        };
                        if let Some(&bad) = wanted.iter().find(|&&p| {
                            is_blob(p) || matches!(t.fields[p].ty, RtType::Variant) || schema.fields[p].flow_filter
                        }) {
                            return Err(crate::transactions::al_error_code(
                                &format!(
                                    "Sorting cannot be done based on the {} field(s) in the {} table.",
                                    t.fields[bad].name, t.name
                                ),
                                "DB:Ok",
                            ));
                        }
                        let unordered = |p: usize| {
                            schema.fields[p].calc.is_some()
                                || matches!(
                                    &t.fields[p].ty,
                                    RtType::Library(
                                        al_builtins::library_kind::LibraryKind::Media
                                            | al_builtins::library_kind::LibraryKind::MediaSet
                                    )
                                )
                        };
                        let row_dependent = |p: usize| {
                            schema.fields[p].calc.as_ref().is_some_and(|c| {
                                c.conds.iter().any(|k| {
                                    matches!(
                                        k.value,
                                        al_ir::CalcValue::Field(_)
                                            | al_ir::CalcValue::UpperLimit(_)
                                            | al_ir::CalcValue::FieldFilter(_)
                                    )
                                })
                            })
                        };
                        if let Some(&p) = wanted.iter().find(|&&p| schema.fields[p].calc_refusal.is_some()) {
                            return unsupported(format!(
                                "SetCurrentKey on the FlowField {}, whose formula is not lowered (probe wanted)",
                                t.fields[p].name
                            ));
                        }
                        // A FlowField whose value depends on the row (a `field()` condition) is reported as the key, alone and
                        // after a normal field (record/Y8SetCurrentKeyNotStored.al:84-106; SetCurrentKeyFlowFieldOrder.al:
                        // 109-113 also pins the order of the rows, not built: [`FLOWFIELD_KEY`]), against the FlowField without
                        // conditions below (W4SortKeys.al:471). Together with a Media field or a FlowField without conditions:
                        // not pinned.
                        if wanted.iter().any(|&p| row_dependent(p)) {
                            let is_media = |p: usize| {
                                matches!(
                                    &t.fields[p].ty,
                                    RtType::Library(
                                        al_builtins::library_kind::LibraryKind::Media
                                            | al_builtins::library_kind::LibraryKind::MediaSet
                                    )
                                )
                            };
                            if let Some(&p) = wanted
                                .iter()
                                .find(|&&p| !row_dependent(p) && (unordered(p) || is_media(p) || !t.fields[p].stored))
                            {
                                return unsupported(format!(
                                    "SetCurrentKey on a FlowField whose value depends on the row together with {}: not pinned (probe wanted)",
                                    t.fields[p].name
                                ));
                            }
                            if let Some(answer) = self.temporary_lookup_key(rec, &t, &wanted)? {
                                return Ok(answer);
                            }
                            let index = self.engine.tables[t.object as usize]
                                .as_ref()
                                .map_or(0, |e| e.keys.len());
                            let mut r = rec.borrow_mut();
                            r.field_descending = 0;
                            r.key = index;
                            r.adhoc = Some((index, wanted));
                            if r.unpinned_buffer.is_none() {
                                r.unpinned_buffer = Some(FLOWFIELD_KEY);
                            }
                            return Ok(Value::Boolean(true));
                        }
                        let media = |p: usize| {
                            matches!(
                                &t.fields[p].ty,
                                RtType::Library(
                                    al_builtins::library_kind::LibraryKind::Media
                                        | al_builtins::library_kind::LibraryKind::MediaSet
                                )
                            )
                        };
                        // Media and MediaSet fields: the record answers them as its key (ArRecordProbes.al:198-229), a by-value
                        // copy the primary key (W4SortKeys.al:471); the rest is refused ([`MEDIA_KEY`]). Together with a
                        // FlowField: not pinned.
                        if wanted.iter().all(|&p| media(p)) {
                            let index = self.engine.tables[t.object as usize]
                                .as_ref()
                                .map_or(0, |e| e.keys.len());
                            let mut r = rec.borrow_mut();
                            r.field_descending = 0;
                            r.key = index;
                            r.adhoc = Some((index, wanted));
                            if r.unpinned_buffer.is_none() {
                                r.unpinned_buffer = Some(MEDIA_KEY);
                            }
                            return Ok(Value::Boolean(true));
                        }
                        if wanted.iter().any(|&p| media(p)) && wanted.iter().all(|&p| unordered(p)) {
                            return unsupported(
                                "SetCurrentKey on a Media field together with a FlowField: what the key is is not pinned",
                            );
                        }
                        if wanted.iter().all(|&p| unordered(p)) {
                            let mut r = rec.borrow_mut();
                            r.key = 0;
                            r.adhoc = None;
                            return Ok(Value::Boolean(true));
                        }
                        if let Some(&bad) = wanted.iter().find(|&&p| unordered(p) || !t.fields[p].stored) {
                            return unsupported(format!(
                                "SetCurrentKey on {} together with other fields: whether the key stays the primary key is not pinned",
                                t.fields[bad].name
                            ));
                        }
                        // `t` already holds this record's earlier ad hoc key, if any.
                        let index = self.engine.tables[t.object as usize]
                            .as_ref()
                            .map_or(0, |e| e.keys.len());
                        let mut r = rec.borrow_mut();
                        if r.adhoc.as_ref().map(|a| &a.1) != Some(&wanted) {
                            r.field_descending = 0;
                        }
                        r.key = index;
                        r.adhoc = Some((index, wanted));
                        Ok(Value::Boolean(true))
                    }
                }
            }
            RecordOp::Validate => {
                let entry = self.take_validate_site(rec);
                let pos = arg_pos(args, 0)?;
                let engine = self.engine.clone();
                let schema = engine.program.objects[t.object as usize].table.as_ref().expect("table");
                let field = &schema.fields[pos];
                // A field with an OnValidate trigger is left to the trigger: PG_Validate_TriggerReadsUnloadedField names the field the
                // trigger reads, not the validated one (P170JitLoadGone.al:634-651).
                if !rec.borrow().temporary && field.on_validate.is_none() {
                    let (before, after) = self.ext_validate_order(&t, field)?;
                    if before.is_empty() && after.is_empty() {
                        self.validate_jit_gone(rec, pos)?;
                    }
                }
                // A FlowField (record/flowfields/DiFwWriteTests.al): flowfields.rs `validate_flowfield` raises or refuses
                // what is not a Decimal Sum; for that one the value goes in, OnValidate runs, then `write_sum_difference`.
                let sum_flowfield = !field.stored && !field.flow_filter && field.id < 2_000_000_000;
                // The error of a FlowField that is no Decimal, raised after OnValidate ran (FfmOperations.al:520-566).
                let mut flowfield_error = None;
                if sum_flowfield {
                    let value = args.get(1).cloned();
                    flowfield_error = self.validate_flowfield(rec, &t, pos, value.as_ref())?;
                }
                // A Validate nested in an OnValidate of the same record keeps the outer call's
                // before-image as xRec (corpus record/TestXRecContracts.al:133).
                // `handle` is the address of `rec`, which this call borrows for as long as the entry exists, and the
                // entry is popped below on every way out (the result is held, not `?`-ed, until then), so no other
                // record can get the address while it is on the stack (tests/store/xrec.rs).
                let handle = entry.0;
                let outer = self.outer_validate_before(handle, t.object as usize);
                let before = self.validate_before_image(handle, outer.clone(), rec, args.get(1).is_some())?;
                let old_value = rec.borrow().fields[pos].clone();
                let snapshot = rec.borrow().fields.clone();
                let new_value = match args.get(1) {
                    // Validate of an Option field with a BigInteger beyond Integer raises Overflow, where an assignment raises
                    // ConversionCast (types/operators-expressions/P79BigOption.al:284-302 Field_ValidateBigToOption). An Enum
                    // field takes a BigInteger only through FieldRef.Validate (a Variant parameter), and raises the same
                    // (types/enum-option/BigIntToEnumOption.al:449-463 QO_Variant_ToEnumFieldRef_Validate).
                    Some(Value::BigInteger(n))
                        if i32::try_from(*n).is_err() && matches!(field.ty, RtType::Option(_) | RtType::Enum(_)) =>
                    {
                        return al_builtins::native::raise(
                            "Arithmetic operation resulted in an overflow.",
                            "AL:Overflow",
                        );
                    }
                    Some(v) => {
                        crate::recmeta::tdd_fits(field, v)?;
                        Some(ops::convert(v.clone(), &field.ty)?)
                    }
                    None => None,
                };
                // The relation is checked before the value goes in and before OnValidate runs; a
                // refused value is never stored (corpus record/TestCalcFormulaSystemFieldsTests.Codeunit.al:253).
                self.validate_relation(rec, &t, pos, new_value.clone().unwrap_or_else(|| old_value.clone()))?;
                if let Some(v) = new_value {
                    rec.borrow_mut().fields[pos] = v;
                }
                let xrec = Self::xrec_handle(rec, before.clone());
                let outermost = outer.is_none().then_some(before);
                self.while_validating(entry, outermost, |s| s.validate_hooks(&t, rec, &xrec, field, snapshot))?;
                if sum_flowfield {
                    self.finish_flowfield_validate(rec, &t, pos, flowfield_error, args.get(1).cloned(), old_value)?;
                }
                Ok(Value::Null)
            }
            RecordOp::Rename => {
                let from_al = std::mem::take(&mut self.tx.rename_from_al);
                let rename_note = (self.tx.writes.len(), self.store.change_counter());
                self.note_bulk_write(rec, &t, crate::state::TxWriteKind::Other);
                if !rec.borrow().temporary {
                    self.rename_jit_gone(rec)?;
                }
                // Fields that relate to this table follow the rename; a temporary record has none.
                let refs = if rec.borrow().temporary {
                    Vec::new()
                } else {
                    self.rename_references(&t)?
                };
                let old = rec.borrow().fields.clone();
                let mut new = old.clone();
                // A temporary record's Blob reads empty through a stream right after a Rename, whether the write was
                // persisted or not (streams/TestProbeStreamContracts.al:TempRename_UnpersistedBlobWrite,
                // TempRename_PersistedBlobWrite, TempRename_BlobInsertedWithValueThenRename), while the bytes stay in the
                // row: a Get and CalcFields read them back (streams/TestBlobRenameIsolation.al:
                // TempBlob_UncommittedWrite_Rename_SameInstanceReGet_KeepsWrite).
                if rec.borrow().temp_id.is_some() {
                    for (i, v) in new.iter_mut().enumerate() {
                        if let Value::Blob(b) = v {
                            let kept = al_builtins::stream::copy_blob(b, false);
                            *b = match self.default_value(&t.fields[i].ty) {
                                Value::Blob(empty) if b.borrow().persisted => empty,
                                _ => kept,
                            };
                        }
                    }
                }
                // Every Blob field of a database record: the variable shows no value in any of them after a Rename, the
                // table's own field and other apps' extension fields alike, while the renamed row holds the variable's
                // content from before the Rename, also a write since the last Insert or Modify
                // (table-objects/extensions/BwCrossAppTests.al:89-115, :234, :258, :282; a table of the caller's
                // own app: :309).
                let database_blobs: Vec<usize> = if rec.borrow().temporary {
                    Vec::new()
                } else {
                    t.fields
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| matches!(old.get(*i), Some(Value::Blob(_))))
                        .map(|(i, _)| i)
                        .collect()
                };
                // More values than the primary key has fields (recordref-fieldref/recordref/RfTWrite.al:Rename_Variants `rename_too_many`).
                if args.len() > t.keys[0].len() {
                    return Err(Signal::Error(Box::new(AlError {
                        message: "The number of field arguments does not match the primary key size.".into(),
                        code: "RenameArgument".into(),
                        call_stack: String::new(),
                        ..Default::default()
                    })));
                }
                for (n, &field) in t.keys[0].iter().enumerate() {
                    let ty = t.fields[field].ty.clone();
                    if let Some(v) = args.get(n) {
                        new[field] = ops::convert(v.clone(), &ty)?;
                    }
                }
                let auto_increment_refusal = self.check_rename_values(rec, &t, &old, &mut new)?;
                if self.is_profile_row(&t, rec.borrow().temporary) {
                    self.profile_before_rename(&t, &old, &new)?;
                }
                // After the All Profile key check: a tenant profile renamed from the empty App ID gets that error
                // (profiles/TestAllProfileTable.al:186).
                // A stale record gets DB:RecordChanged before the blank-key error (stale_rename.rs).
                self.stale_rename(&cx, rec, &t, &old, &new, from_al, rename_note.0)?;
                let blank_key = self.blank_key_rename(&cx, rec, &t, &refs, &old, &new, from_al)?;
                // The row is found under the key the column holds (P130DateFilters.al:641-648: the record holds 10:00:00 after
                // Get('A', 10:00:00.001) and its Rename to 12:00:00.001 is found under 12:00:00).
                let old_pk = Self::sql_key(rec, &t, &old);
                // Read before the triggers, on the buffer the test left (`assigned_audit::rename_audit`).
                let audit = self.rename_audit(&cx, &t, rec, &old_pk)?;
                let new_rounds = !rec.borrow().temporary && crate::sqltime::key_rounds(&t.primary_key(&new));
                if new_rounds && !refs.is_empty() {
                    return unsupported(
                        "a Rename to a Time or DateTime key value the SQL column rounds, of a table other tables relate to (not pinned)",
                    );
                }
                // Rec carries the new key and xRec the old row in the events and the trigger
                // (record/TestTableEventDispatch.al:TableEvent_OnAfterRename_Subscriber_FiresWithOldAndNewKeys).
                let old_fields = old.clone();
                rec.borrow_mut().fields = new.clone();
                let xrec = Self::xrec_handle(rec, old.clone());
                // Rename always runs the trigger; RunTrigger is true (docs/probes-needed.md).
                self.table_event(&t, "onbeforerenameevent", rec, Some(&xrec), true, None)?;
                self.run_ext_trigger(&t, "OnBeforeRename", rec, Some(&xrec))?;
                if let Some(tr) = self.table_trigger(&t, "OnRename") {
                    self.run_trigger(t.object as usize, tr, rec, Some(&xrec))?;
                }
                self.run_ext_trigger(&t, "OnRename", rec, Some(&xrec))?;
                self.raise_deferred_auto_increment(&cx, &t, rec, &xrec, (&old_pk, &old), auto_increment_refusal)?;
                self.blank_key_rename_raise(&t, rec, blank_key, &old)?;
                let _ = self.bump_row_version(&t, rec);
                let _ = self.stamp_rename(&t, rec);
                let row = audit.row(&rec.borrow().fields);
                if let Some(unique_key) = self.unique_violation(rec, &t, &row, Some(&old_pk))? {
                    rec.borrow_mut().fields = old.clone();
                    return self.fail_unique(&t, unique_key, &row);
                }
                // A temporary row whose Insert stored a FlowFilter value (`flow_filter_positions`): what a Rename keeps is not pinned.
                if rec.borrow().temporary {
                    let ff = self.flow_filter_positions(&t);
                    if !ff.is_empty() {
                        let stored_old = self
                            .store_for(rec)
                            .get(&cx, &t, &old_pk)
                            .map_err(|e| Signal::unsupported(format!("{e}")))?;
                        if let Some(o) = stored_old
                            && ff.iter().any(|&i| {
                                ops::compare(&o[i], &self.default_value(&t.fields[i].ty)).ok()
                                    != Some(std::cmp::Ordering::Equal)
                            })
                        {
                            return unsupported(
                                "a Rename of a temporary row whose Insert stored a FlowFilter value (not pinned)",
                            );
                        }
                    }
                }
                let mut stored = Self::to_store(rec, self.stored_view(&t, &row));
                if self.sql_columns(rec, &t, &mut stored)?.is_some() {
                    return unsupported("a Rename with a value its SQL column cannot hold (not pinned)");
                }
                // A rename to the key the row has succeeds and writes the row again, which is what
                // stamps it (record/TestRecordSystemId.al:752-775).
                let same_key = ops::compare_keys(&old_pk, &Self::sql_key(rec, &t, &row))?;
                let before_store = self.store.change_counter();
                let outcome = if same_key {
                    match self
                        .store_for(rec)
                        .modify(&cx, &t, stored.clone())
                        .map_err(|e| Signal::unsupported(format!("{e}")))?
                    {
                        WriteOutcome::Done => RenameOutcome::Renamed,
                        WriteOutcome::NotFound => RenameOutcome::NotFound,
                    }
                } else {
                    self.store_for(rec)
                        .rename(&cx, &t, &old_pk, stored.clone())
                        .map_err(|e| Signal::unsupported(format!("{e}")))?
                };
                match outcome {
                    RenameOutcome::Renamed => {
                        // A Rename of another key with nothing written before it (no trigger write) is noted as one write of the
                        // row (`table_writes_known`). Fields of other tables that relate to the table do not matter while no row
                        // follows the rename: the rows of ALT Universal, which fields of other tables relate to, are renamed this way
                        // (record/crud/SbStaleOwnModify.al TE_*_RenameOtherRow_*). A row that follows is a store
                        // write nobody notes, which `writes_all_noted` sees.
                        if !same_key
                            && before_store == rename_note.1
                            && self.tx.writes.len() == rename_note.0 + 1
                            && let Some(w) = self.tx.writes.last_mut()
                            && w.kind == crate::state::TxWriteKind::Other
                        {
                            w.kind = crate::state::TxWriteKind::Rename;
                            w.pk = t.primary_key(&old_fields);
                            w.before = rename_note.1;
                            w.after = self.store.change_counter();
                        }
                        // The variable's own Rename after its own Insert, in that transaction, makes the buffer's new version its own
                        // write (record/crud/SbStaleOwnModify.al:268-298). After a Get it is not pinned: the origin
                        // is left behind, as before.
                        if !rec.borrow().temporary
                            && let Some(i) = t.fields.iter().position(|f| f.id == 2_000_000_005)
                        {
                            let mut r = rec.borrow_mut();
                            let (was, now) = match (&old[i], &r.fields[i]) {
                                (Value::BigInteger(a), Value::BigInteger(b)) => (*a, *b),
                                _ => (0, 0),
                            };
                            if let Some(o) = r.row_origin
                                && o.version == was
                                && now != was
                            {
                                let keep = o.own_write && !o.read_in_write_tx && o.commits == self.commits;
                                r.row_origin = keep.then_some(al_builtins::value::RowOrigin { version: now, ..o });
                            }
                        }
                        // The record itself shows no Blob value after a Rename of a temporary record; the row keeps one only when a write came
                        // after the last Insert or Modify (record/PnABlobAssignRename.al:RenameTemporary_* `A..E-rec-content`).
                        // So does a database record for every Blob field, while the renamed row keeps the bytes
                        // (BwCrossAppTests.al:89-115: `blob in variable after rename = <none>`, `NEW blob = blob of OLD`;
                        // :234-309: `variable = <none>;<none>;<none>;`, own field and both extension fields).
                        // CalcFields on the variable then reads the renamed row (BwCrossAppTests.al:234-309, `variable
                        // after CalcFields`), as for a row read with Get.
                        for &i in &database_blobs {
                            let empty = self.default_value(&t.fields[i].ty);
                            if let Value::Blob(b) = &empty {
                                b.borrow_mut().fetched = false;
                            }
                            rec.borrow_mut().fields[i] = empty;
                        }
                        if rec.borrow().temp_id.is_some() {
                            let own: Vec<(usize, Value)> = rec
                                .borrow()
                                .fields
                                .iter()
                                .enumerate()
                                .filter(|(_, v)| matches!(v, Value::Blob(_)))
                                .map(|(i, _)| (i, self.default_value(&t.fields[i].ty)))
                                .collect();
                            let mut r = rec.borrow_mut();
                            for (i, empty) in own {
                                r.fields[i] = empty;
                            }
                        }
                        // A company that is renamed takes its data with it
                        // (record/TestChangeCompanyRows... TestChangeCompanyFiltersCopyDeleteContracts.al:170-212).
                        if !same_key && self.is_company_table(&t) && !rec.borrow().temporary {
                            let (from, to) = (
                                al_builtins::native::text_of(&old_fields[0]).unwrap_or_default(),
                                al_builtins::native::text_of(&row[0]).unwrap_or_default(),
                            );
                            self.store.rename_company(&from, &to);
                            if self.full_copies.remove(from.as_str()) {
                                self.full_copies.insert(to.to_string());
                            }
                        }
                        if !same_key && self.is_profile_row(&t, rec.borrow().temporary) {
                            self.profile_after_rename(&t, &old_fields, &row)?;
                        }
                        if !same_key {
                            self.propagate_rename(rec, &t, &refs, &old_fields, &row)?;
                            // The platform moves the record's links with system rights: a set without any grant on
                            // Record Link renames (corpus record/Y19PermWrites.al
                            // Y19_P1_Rename, rename_rim), so the session's sets do not apply here (ADR-062).
                            let sets = self.permissions.take();
                            let moved = self.move_record_links(&t, rec, &old_fields);
                            self.permissions = sets;
                            moved?;
                        }
                        self.run_ext_trigger(&t, "OnAfterRename", rec, Some(&xrec))?;
                        self.table_event(&t, "onafterrenameevent", rec, Some(&xrec), true, None)?;
                        Ok(Value::Boolean(true))
                    }
                    RenameOutcome::NotFound | RenameOutcome::AlreadyExists
                        if new_rounds
                            || (!rec.borrow().temporary && crate::sqltime::key_rounds(&t.primary_key(&old))) =>
                    {
                        rec.borrow_mut().fields = old.clone();
                        unsupported(
                            "a failed Rename of a record whose Time or DateTime key value the SQL column rounds (its error is not pinned)",
                        )
                    }
                    RenameOutcome::NotFound => {
                        rec.borrow_mut().fields = old.clone();
                        self.fail(&t, &old, false)
                    }
                    // A rename that cannot be done raises whether or not the Boolean is used (RfTWrite.al:Rename_Variants
                    // `rename_existing`: `B := RecRef.Rename('D2')`).
                    RenameOutcome::AlreadyExists => {
                        rec.borrow_mut().fields = old;
                        self.ret_used = false;
                        self.fail(&t, &row, true)
                    }
                }
            }
            RecordOp::Truncate => self.truncate_all(&cx, &t, rec, arg_bool(args, 0)?),
            RecordOp::CalcFields | RecordOp::CalcSums | RecordOp::SetAutoCalcFields => self.flow_op(method, rec, args),
            RecordOp::LoadFields => self.load_fields_row(rec, &t, args),
            RecordOp::SetLoadFields
            | RecordOp::AddLoadFields
            | RecordOp::AreFieldsLoaded
            | RecordOp::SetBaseLoadFields => self.load_fields_op(method, rec, &t, args),
            other => self.record_meta_op(other, rec, &t, args),
        }
    }

    /// The fields `Init` gives `rec`: the initial values, except that Init of a temporary record keeps the SQLTimestamp fields
    /// and SystemRowVersion (record/temporary/QrTempTimestamp.al:221-226, `shared copy insert`).
    fn init_fields(&mut self, t: &TableDef, rec: &Rec) -> Vec<Value> {
        let mut fresh = self.initial_fields(t.object as usize);
        if self.temp_never_stamps(t, rec) {
            let r = rec.borrow();
            let facts = &self.engine.table_facts[t.object as usize].sql_timestamp;
            for (i, f) in t.fields.iter().enumerate() {
                if f.id == 2_000_000_005 || facts.contains(&i) {
                    fresh[i] = r.fields[i].clone();
                }
            }
        }
        fresh
    }

    /// `Clear` of a record variable.
    fn clear_record(&mut self, t: &TableDef, rec: &Rec) -> Result<Value, Signal> {
        // Clear resets the buffer, the filters and the keys but does not empty the table of a
        // temporary record: its rows stay and it stays temporary
        // (types/ProbeTypeFormsAndFilters.al:Clear_TemporaryRecordWithRows_ThenCount, BC 27.5).
        let fresh = self.initial_fields(t.object as usize);
        let old_row_id = Some(crate::partial::row_id(t, &rec.borrow().fields));
        // Clear sets the table's globals back to their initial values (ADR-021 decision 4;
        // corpus probe TableGlobals_Clear_WhatHappensToTheGlobals).
        self.reset_table_globals(rec);
        let mut r = rec.borrow_mut();
        r.fields = fresh;
        r.assigned = false;
        r.inserted = false;
        r.filters.clear();
        r.filter_group = 0;
        r.marks.clear();
        r.marked_only = false;
        r.key = 0;
        r.adhoc = None;
        r.ascending = true;
        r.field_descending = 0;
        r.position = None;
        // Clear sets ReadIsolation back to Default (record/CvReadIsolationState.al:58, :64).
        r.read_isolation = 0;
        // Clear sets SecurityFiltering back to the value the variable was declared with, also after a SecurityFiltering(x) call.
        r.security_filtering_clear();
        // A Clear leaves the record's record of what it fetched: the fields in hand stay loaded for a later
        // partial fetch (session-system/TestProbeSessionMiscContracts.al:PartialRecords_AreFieldsLoaded: a full Get,
        // Clear, SetLoadFields(Cd) and Get still reports Txt loaded).
        let (mut buffer, partial) = (r.load.buffer.take(), r.load.partial);
        if let Some(b) = buffer.as_mut() {
            if b.row.is_none() {
                b.row = old_row_id.clone();
            }
            b.cleared = true;
        }
        r.load = Default::default();
        r.load.buffer = buffer;
        r.load.partial = partial;
        Ok(Value::Null)
    }
}
