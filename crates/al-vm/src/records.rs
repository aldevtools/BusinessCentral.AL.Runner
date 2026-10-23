//! Record operations on the store (ADR-003) for the `Record.*` builtins.
//!
//! Pinned by the corpus or the docs as noted; behaviour neither pins yet
//! raises `Signal::Unsupported` instead of guessing.

use crate::Session;
use crate::globaltriggers::GlobalOp;
use al_builtins::describe::Describe;
use al_builtins::filter::{FieldFilter, FilterExpr, key_cmp};
use al_builtins::native::{AlError, Signal};
use al_builtins::ops;
use al_builtins::record_op::RecordOp;
use al_builtins::value::{Instance, RecordState, RtType, Value};
use al_data::{InsertOutcome, RenameOutcome, Scan, Store, TableDef, WriteOutcome, memory::MemStore, position};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

pub(crate) type Rec = Rc<RefCell<RecordState>>;

/// BC's line break inside error texts: a carriage return and a backslash.
pub(crate) const BR: &str = "\\";

fn unsupported<T>(what: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(what.into()))
}

/// The `RecordState::unpinned_buffer` reason of Rec while the OnOpenPage of a page an action opens through `RunPageLink` runs
/// (set in pages/runobject.rs, its methods in [`Session::run_page_link_open_op`]).
pub(crate) const RUN_PAGE_LINK_OPEN: &str = "in the OnOpenPage of a page an action opens through RunPageLink: real BC fills the linked fields of Rec and GetFilters() reads '' there, the runtime holds the link as a filter (corpus issue 712)";

fn arg_bool(args: &[Value], i: usize) -> Result<bool, Signal> {
    match args.get(i) {
        None => Ok(false),
        Some(v) => ops::truthy(v),
    }
}

fn arg_pos(args: &[Value], i: usize) -> Result<usize, Signal> {
    match args.get(i) {
        Some(Value::Integer(p)) => Ok(*p as usize),
        other => unsupported(format!("field argument {}", other.describe())),
    }
}

/// What the `Rec` and `xRec` slots of a table instance held before a call filled them.
#[derive(Default)]
pub(crate) struct RecSlots {
    rec: Option<(usize, Value)>,
    xrec: Option<(usize, Value)>,
}

impl Session {
    /// The platform takes Insert on Tenant License State and refuses Modify and Delete (al-language-onprem/session/PZPlatformSurvey.al:
    /// PlatformTable_TenantLicenseState: `It is not possible to modify any record in the Tenant License State table.`, code `DB:Ok`).
    fn tenant_license_state_refuses(&self, t: &TableDef, verb: &str) -> Result<(), Signal> {
        if self.engine.program.objects[t.object as usize].id == Some(2_000_000_189) {
            return Err(Signal::Error(Box::new(AlError {
                message: format!("It is not possible to {verb} any record in the Tenant License State table."),
                code: "DB:Ok".into(),
                call_stack: String::new(),
                ..Default::default()
            })));
        }
        Ok(())
    }

    pub(crate) fn table(&self, rec: &Rec) -> Result<Arc<TableDef>, Signal> {
        let r = rec.borrow();
        let t = self.engine.tables[r.table as usize]
            .clone()
            .map_or_else(|| unsupported("record of a table without a schema"), Ok)?;
        if let Some(why) = self.engine.program.objects[t.object as usize]
            .table
            .as_ref()
            .and_then(|x| x.unsupported.first())
        {
            return unsupported(format!("table {}: {why}", t.name));
        }
        // A key set on fields no key starts with is one more key, as far as ordering goes.
        if let Some((index, fields)) = &r.adhoc {
            if r.key == *index && *index == t.keys.len() {
                // The table with that extra key is built once per (table, key fields), not on every operation.
                let mut cache = self.adhoc_tables.borrow_mut();
                let entry = cache.entry((r.table, fields.clone())).or_insert_with(|| {
                    let mut with_key = (*t).clone();
                    with_key.keys.push(fields.clone());
                    Arc::new(with_key)
                });
                return Ok(entry.clone());
            }
        }
        Ok(t)
    }

    /// Field values of a fresh record: `InitValue` where declared, else the type default.
    pub(crate) fn initial_fields(&mut self, table: usize) -> Vec<Value> {
        // The values are the same for every record of the table, so a table whose defaults are all scalars is
        // worked out once per session and copied (a text shares its buffer, copy on write); a table with a Blob, an
        // array or another shared object builds fresh ones each time.
        if let Some(Some(template)) = self.initial_cache.get(&table) {
            return template.clone();
        }
        let engine = self.engine.clone();
        let Some(t) = engine.program.objects[table].table.as_ref() else {
            return Vec::new();
        };
        let fields: Vec<Value> = t
            .fields
            .iter()
            .map(|f| match &f.init {
                Some(c) => ops::convert(crate::constant(c), &f.ty).unwrap_or_else(|_| self.default_value(&f.ty)),
                None => self.default_value(&f.ty),
            })
            .collect();
        if let std::collections::hash_map::Entry::Vacant(slot) = self.initial_cache.entry(table) {
            let scalar = fields.iter().all(|v| {
                matches!(
                    v,
                    Value::Null
                        | Value::Boolean(_)
                        | Value::Integer(_)
                        | Value::BigInteger(_)
                        | Value::Decimal(_)
                        | Value::Byte(_)
                        | Value::Char(_)
                        | Value::Text(_)
                        | Value::Code(_)
                        | Value::Date(..)
                        | Value::Time(_)
                        | Value::DateTime(_)
                        | Value::Duration(_)
                        | Value::Guid(_)
                        | Value::DateFormula(_)
                        | Value::Option(..)
                        | Value::Enum(..)
                )
            });
            slot.insert(scalar.then(|| fields.clone()));
        }
        fields
    }

    pub(crate) fn store_for(&mut self, rec: &Rec) -> &mut MemStore {
        let id = rec.borrow().temp_id.as_ref().map(|t| t.id());
        match id {
            Some(id) => {
                // Tables whose last record variable is gone are freed first.
                if !self.temp_release.borrow().is_empty() {
                    for dead in self.temp_release.borrow_mut().drain(..) {
                        self.temps.remove(&dead);
                    }
                }
                self.temps.entry(id).or_default()
            }
            None => &mut self.store,
        }
    }

    /// The table's caption for an error text. Where the caption is refused (`al_ir::CAPTION_REFUSED`) the text holds the
    /// unpinned-error mark, so a read of the error's text or code refuses (`al_builtins::native::text_unpinned`).
    pub(crate) fn caption(&self, t: &TableDef) -> String {
        if self.caption_refused(t.object as usize).is_some() {
            return al_builtins::native::UNPINNED_ERROR.to_string();
        }
        self.engine.program.objects[t.object as usize]
            .properties
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Caption"))
            .map(|(_, v)| v.trim().trim_matches('\'').to_string())
            .unwrap_or_else(|| t.name.clone())
    }

    /// The caption a read of it gives (TableCaption, RecordRef.Caption, Format of a RecordId): refused where a
    /// tableextension sets it over another caption (`al_ir::CAPTION_REFUSED`).
    pub(crate) fn caption_checked(&self, t: &TableDef) -> Result<String, Signal> {
        match self.caption_refused(t.object as usize) {
            Some(why) => Err(Signal::Unsupported(why)),
            None => Ok(self.caption(t)),
        }
    }

    /// Why the caption of the table with object index `object` is not pinned, if it is not.
    pub(crate) fn caption_refused(&self, object: usize) -> Option<String> {
        self.engine.program.objects[object]
            .properties
            .iter()
            .find(|(k, _)| k == al_ir::CAPTION_REFUSED)
            .map(|(_, v)| v.clone())
    }

    /// `No.='A'` style identification of a row's primary key. A key value whose format is not
    /// pinned refuses.
    pub(crate) fn identification(&self, t: &TableDef, fields: &[Value]) -> Result<String, Signal> {
        let mut parts = Vec::new();
        for &i in &t.keys[0] {
            let v = al_builtins::format::try_format(&fields[i], 0, 0, self).map_err(Signal::Unsupported)?;
            // The field's caption names it (recordref/lt-recordref-fieldref-members + records/lt-record-meta: `Code Caption='A1'`).
            parts.push(format!("{}='{}'", self.field_caption(t, i), v));
        }
        Ok(parts.join(","))
    }

    /// Refuses a read whose answer depends on the rows of table `t` when the install baseline could
    /// not create the rows BC has there (`Engine::unseeded`, ADR-013 decision 5): the answer would
    /// be "empty" or "fewer rows", which is not BC's. Rows this session wrote itself do not make
    /// the answer BC's, so they are not an exception. Temporary records are private to the session.
    /// Whether a `Get` that finds nothing, or a scan, on table object `object` is not BC's answer.
    pub(crate) fn is_unseeded(&self, object: u32) -> bool {
        let o = object as usize;
        self.engine.unseeded.contains(&o) || (!self.install_scope && self.engine.platform_unseeded.contains(&o))
    }

    /// Why the rows install code left in table `object` are not known (ADR-046: `InstallFacts::unknown`). Install code
    /// itself reads them (it runs in an order of the runtime's own; the tables it writes are in the set).
    pub(crate) fn install_unknown(&self, object: u32) -> Option<&str> {
        if self.install_scope {
            return None;
        }
        let facts = self.engine.install_facts.get()?;
        if facts.unknown.is_empty() {
            return None;
        }
        facts.unknown.get(&(object as usize)).map(String::as_str)
    }

    /// The refusal of a read of field `pos` of a row of table `table` (`fields`, the record's values) when that field is
    /// `SystemRowVersion`, `SystemCreatedAt` or `SystemModifiedAt`, the row was written by install code, and the order of
    /// the install routines that write the table is not known (ADR-046 amendment 1, A2 item d; the row version orders
    /// rows across tables: install/lt-bf-install-probes/BfInstallTests.al:8-9, :196-241).
    pub(crate) fn install_row_order(
        &self,
        table: u32,
        temporary: bool,
        pos: usize,
        fields: &[Value],
    ) -> Option<String> {
        if temporary || self.install_scope {
            return None;
        }
        let facts = self.engine.install_facts.get()?;
        if facts.row_order.is_empty() {
            return None;
        }
        let why = facts.row_order.get(&(table as usize))?;
        let def = self.engine.tables.get(table as usize)?.as_ref()?;
        let field = def.fields.get(pos)?;
        let name = match field.id {
            2_000_000_001 => "SystemCreatedAt",
            2_000_000_003 => "SystemModifiedAt",
            2_000_000_005 => "SystemRowVersion",
            _ => return None,
        };
        let version = def.fields.iter().position(|f| f.id == 2_000_000_005)?;
        match fields.get(version) {
            Some(Value::BigInteger(v)) if *v > 0 && *v <= self.install_row_version => Some(format!(
                "{name} of a row of {} that install code wrote: {why}",
                def.name
            )),
            _ => None,
        }
    }

    /// ADR-046 decision 4a: in the default mode (one company) a read of the Company table that could list a company the
    /// runtime does not know is refused. The corpus database holds a second company the runtime does not have
    /// (install/lt-bf-install-probes/BfInstallTests.al:60-62). A read filtered to one known name (a single-value filter on
    /// Name: the session company or a company row the run holds) is answered.
    pub(crate) fn company_table_read(&self, rec: &Rec, what: &str) -> Result<(), Signal> {
        let named = {
            let r = rec.borrow();
            r.filters
                .iter()
                .filter(|f| f.field == 0 && f.group != al_builtins::filter::OR_GROUP)
                .find_map(|f| match &f.expr {
                    al_builtins::filter::FilterExpr::Eq(v) | al_builtins::filter::FilterExpr::RangeSame(v) => {
                        al_builtins::native::text_of(v).map(String::from)
                    }
                    al_builtins::filter::FilterExpr::Range(Some(a), Some(b)) => {
                        let (a, b) = (al_builtins::native::text_of(a)?, al_builtins::native::text_of(b)?);
                        (a == b).then(|| a.to_string())
                    }
                    _ => None,
                })
        };
        if let Some(name) = named {
            if self.find_company(&name)?.is_some() {
                return Ok(());
            }
        }
        unsupported(format!(
            "{what} on the Company table: the database may hold companies this runtime does not know (ADR-046 decision 4a)"
        ))
    }

    pub(crate) fn unseeded_read(&self, t: &TableDef, temporary: bool, what: &str) -> Result<(), Signal> {
        if !temporary {
            if let Some(why) = self.install_unknown(t.object) {
                return unsupported(format!("{what} on {}: {why}", t.name));
            }
        }
        if !temporary && self.is_unseeded(t.object) {
            return unsupported(format!(
                "{what} on {}: BC holds rows here that an install trigger or Company-Initialize creates, and the install baseline could not create them (ADR-013 decision 5)",
                t.name
            ));
        }
        Ok(())
    }

    /// Message texts: docs/probes-needed.md (the corpus pins 'already exists'
    /// and 'does not exist' fragments only).
    fn fail(&mut self, t: &TableDef, fields: &[Value], exists: bool) -> Result<Value, Signal> {
        // Code reads rows (setup records, mostly) that installing a dependency app and
        // initialising a company create. Where the install baseline could not create them
        // (ADR-013 decision 5), a missing row there is not BC's answer, whoever asks (a dependency
        // app, or the tests of the project): refuse rather than report it as one.
        if !exists {
            if let Some(why) = self.install_unknown(t.object) {
                return unsupported(format!("{} row not found: {why}", t.name));
            }
        }
        if !exists && self.is_unseeded(t.object) {
            return unsupported(format!(
                "{} row not found: the rows a dependency app's install and Company-Initialize create could not be created (ADR-013 decision 5)",
                t.name
            ));
        }
        if self.ret_used {
            return Ok(Value::Boolean(false));
        }
        let identification = self.identification(t, fields)?;
        let (message, code) = if exists {
            (
                format!(
                    "The record in table {} already exists. Identification fields and values: {}",
                    self.caption(t),
                    identification
                ),
                "DB:RecordExists",
            )
        } else {
            (
                format!(
                    "The {} does not exist. Identification fields and values: {}",
                    self.caption(t),
                    identification
                ),
                "DB:RecordNotFound",
            )
        };
        // A missing row: the flag hides the key (P86LastError.al:309, :316, :319, P70_02LastErrFlag.al:76, :82, :88). A row that
        // exists is not pinned.
        let origin = if exists {
            al_builtins::native::ErrorOrigin::Unknown
        } else {
            al_builtins::native::ErrorOrigin::Redacted(format!(
                "The {} does not exist. Identification fields and values: <redacted>",
                self.caption(t)
            ))
        };
        Err(Signal::Error(Box::new(AlError {
            message,
            code: code.into(),
            origin,
            ..Default::default()
        })))
    }

    /// The table-level trigger `name` (`OnBeforeInsert`, `OnInsert`, ...) of every table extension that declares it,
    /// in the order of the extensions' object ids: two extensions run the same trigger one after the other
    /// (corpus events/TestProbeEventContracts.al:ExtensionTriggers_TwoExtensionsSameTrigger_Order, ids 69800 and 69801).
    pub(crate) fn ext_triggers(&self, t: &TableDef, name: &str) -> Vec<(usize, usize)> {
        let mut found = Vec::new();
        if let Some(schema) = &self.engine.program.objects[t.object as usize].table {
            for &e in &schema.extensions {
                for (ri, r) in self.engine.program.objects[e].routines.iter().enumerate() {
                    if r.is_trigger && r.field.is_none() && r.name.eq_ignore_ascii_case(name) {
                        found.push((e, ri));
                    }
                }
            }
        }
        found.sort_by_key(|(e, _)| self.engine.program.objects[*e].id.unwrap_or(u32::MAX));
        found
    }

    /// Runs the extension trigger `name`, if any, with the record and `xrec`.
    fn run_ext_trigger(&mut self, t: &TableDef, name: &str, rec: &Rec, xrec: Option<&Rec>) -> Result<(), Signal> {
        for (o, r) in self.ext_triggers(t, name) {
            self.run_trigger(o, r, rec, xrec)?;
        }
        Ok(())
    }

    /// Whether `event` of table `t` would reach a subscriber now (static ones, and
    /// manual ones with a bound instance).
    pub(crate) fn listened(&mut self, t: &TableDef, event: &str, element: Option<&str>) -> Result<bool, Signal> {
        self.has_listeners(t.object as usize, event, element)
    }

    /// A record handle holding `fields` and otherwise `rec`'s state: an `xRec`.
    fn xrec_handle(rec: &Rec, fields: Vec<Value>) -> Rec {
        let mut st = rec.borrow().clone();
        st.fields = fields;
        // The variable's xRec keeps its globals from one operation to the next (ADR-021).
        st.globals = rec.borrow().globals.xrec();
        Rc::new(RefCell::new(st))
    }

    /// Raises table event `event` (lower-case, e.g. `onbeforeinsertevent`) for
    /// `rec`. Table events fire whether or not the trigger runs
    /// (record/TestStorageLayerFieldWriteRoutes.al: Insert(false)).
    /// `Rec` and `xRec` are passed by reference, so subscribers' changes show
    /// in the caller and in later subscribers
    /// (record/TestTableEventByRefMutation.al).
    fn table_event(
        &mut self,
        t: &TableDef,
        event: &str,
        rec: &Rec,
        xrec: Option<&Rec>,
        run_trigger: bool,
        element: Option<&str>,
    ) -> Result<(), Signal> {
        self.note_event(t.object as usize, event, element);
        if !self.listened(t, event, element)? {
            return Ok(());
        }
        // A temporary record raises the table events and runs the triggers like any other
        // (corpus record/TestTableEventSequenceContracts.al:TemporaryRecord_RaisesTheEventsAndRunsTheTriggers).
        let (names, tail): (&[&str], Value) = match event {
            "onbeforeinsertevent" | "onafterinsertevent" | "onbeforedeleteevent" | "onafterdeleteevent" => {
                (&["Rec", "RunTrigger"], Value::Boolean(run_trigger))
            }
            "onbeforemodifyevent" | "onaftermodifyevent" | "onbeforerenameevent" | "onafterrenameevent" => {
                (&["Rec", "xRec", "RunTrigger"], Value::Boolean(run_trigger))
            }
            // CurrFieldNo is 0 for a Validate called from code (the field's own OnValidate
            // sees 0: handlers/TestPageCurrFieldNo_Tests.al); docs/probes-needed.md for the event.
            // Through a TestPage write it is the field's number (record/lt-loadfields-pagesource-validate/P81PageValidateEvents.al:
            // `cur=2`).
            "onbeforevalidateevent" | "onaftervalidateevent" => {
                (&["Rec", "xRec", "CurrFieldNo"], Value::Integer(self.curr_field_of(rec)))
            }
            other => return unsupported(format!("table event {other}")),
        };
        let mut args = vec![Value::Record(rec.clone())];
        if names.len() == 3 {
            let Some(x) = xrec else {
                return unsupported(format!("{event} without an xRec"));
            };
            args.push(Value::Record(x.clone()));
        }
        args.push(tail);
        let names: Vec<String> = names.iter().map(|n| n.to_string()).collect();
        let call = crate::dispatch::EventCall {
            object: t.object as usize,
            event,
            element,
            isolated: false,
            names: &names,
            sender: None,
        };
        self.raise(&call, &mut args)
    }

    /// The Before event, the field's OnValidate and the After event of a `Validate`.
    fn validate_hooks(
        &mut self,
        t: &TableDef,
        rec: &Rec,
        xrec: &Rec,
        field: &al_ir::Field,
        snapshot: Vec<Value>,
    ) -> Result<(), Signal> {
        // Before event (Rec holds the new value, xRec the old row), the field's OnValidate,
        // After event: record/TestTriggerDispatchOrder.al:TriggerOrder_Validate_FieldPipeline_IsStable.
        // That the Before event sees the new value is docs/probes-needed.md.
        self.table_event(t, "onbeforevalidateevent", rec, Some(xrec), false, Some(&field.name))?;
        // The triggers table extensions attach to the field bracket its own OnValidate
        // (record/TestTriggerDispatchOrder.al:TriggerOrder_Validate_FieldPipeline_IsStable). The
        // order of two extensions' triggers on one field is not pinned.
        if field.ext_before_validate.len() > 1 || field.ext_after_validate.len() > 1 {
            return unsupported(format!(
                "Validate of {}.{}: more than one table extension declares OnBeforeValidate or OnAfterValidate for it (their order is not pinned)",
                t.name, field.name
            ));
        }
        for &(object, routine) in &field.ext_before_validate {
            self.run_trigger(object, routine, rec, Some(xrec))?;
        }
        if let Some((object, routine)) = field.on_validate {
            if let Err(e) = self.run_trigger(object, routine, rec, Some(xrec)) {
                // The whole record is back at its values from before the Validate when the
                // field's OnValidate raises, not only the validated field
                // (record/TestValidateRaisesAndHandlerAsserterrorContracts.al:68,
                // Validate_ThatRaises_RestoresTheWholeRecord; the validated field alone:
                // record/TestTriggerRollback.al:OnValidate_Throws_FieldValueRolledBack).
                if matches!(e, Signal::Error(_)) {
                    rec.borrow_mut().fields = snapshot;
                }
                return Err(e);
            }
        }
        for &(object, routine) in &field.ext_after_validate {
            self.run_trigger(object, routine, rec, Some(xrec))?;
        }
        self.table_event(t, "onaftervalidateevent", rec, Some(xrec), false, Some(&field.name))
    }

    /// `Record.Delete(run)` of the row `rec` holds: Before event, OnDelete, the
    /// delete, After event.
    /// A Modify or Delete let go of media `id` that field `pos` of table `t` held: the media leaves Tenant Media unless another
    /// row of the same field still holds it (media/TestMediaPngImport.al:297-334: kept while the other row holds it, removed with
    /// the Delete of that row; record/lt-ar-records/ArRecordProbes.al:141-143: removed when no row holds it). A media another
    /// table or field holds is not pinned and refused. The removal is undone by a rollback (`Session::rollback_transaction`).
    fn release_media(
        &mut self,
        cx: &al_data::AccessContext,
        t: &TableDef,
        pos: usize,
        id: [u8; 16],
    ) -> Result<(), Signal> {
        let held = |s: &mut Self, table: &TableDef, field: usize| -> Result<bool, Signal> {
            let filters = [FieldFilter {
                field,
                expr: FilterExpr::Eq(Value::Guid(id)),
                group: 0,
            }];
            let scan = Scan {
                key: 0,
                filters: &filters,
                ascending: true,
                after: None,
            };
            Ok(s.store
                .next(cx, table, &scan)
                .map_err(|e| Signal::Unsupported(format!("{e}")))?
                .is_some())
        };
        // Another row of the same table holding it in the same field keeps it (media/TestMediaPngImport.al:297-334
        // Media_Replace_WhileAnotherRowSharesTheMedia). A row of another table, or another field of a row, does not: the
        // media leaves Tenant Media, and that holder keeps the MediaId with HasValue false and ExportStream false, before
        // and after Commit (TestMediaPngImport.al:441-674 QE_Media_*).
        if held(self, t, pos)? {
            return Ok(());
        }
        if let Some(obj) = self.media.objects.borrow_mut().remove(&id) {
            self.media.removed.borrow_mut().insert(id);
            self.media_removed.0.push((id, obj));
        }
        Ok(())
    }

    fn delete_row(&mut self, t: &TableDef, rec: &Rec, run: bool) -> Result<Value, Signal> {
        let cx = self.cx_for(rec, t)?;
        self.table_event(t, "onbeforedeleteevent", rec, None, run, None)?;
        if run {
            let x = Self::xrec_handle(rec, rec.borrow().fields.clone());
            self.run_ext_trigger(t, "OnBeforeDelete", rec, Some(&x))?;
            if let Some(tr) = self.table_trigger(t, "OnDelete") {
                self.run_trigger(t.object as usize, tr, rec, Some(&x))?;
            }
            self.run_ext_trigger(t, "OnDelete", rec, Some(&x))?;
        }
        self.global_trigger(t, GlobalOp::Delete, rec, None)?;
        let user_row = self.is_user_row(t, rec.borrow().temporary);
        if user_row {
            self.user_before_delete(t, rec)?;
        }
        if self.is_profile_row(t, rec.borrow().temporary) {
            self.profile_before_delete(t, &rec.borrow().fields)?;
        }
        let row = rec.borrow().fields.clone();
        let pk = t.primary_key(&row);
        let company = if self.is_company_table(t) && !rec.borrow().temporary {
            let name = al_builtins::native::text_of(&row[0]).unwrap_or_default();
            self.before_company_row_delete(&name, run)?;
            Some(name)
        } else {
            None
        };
        // The Media fields of the stored row: the Delete of the last row that holds a media removes it from Tenant Media
        // (media/TestMediaPngImport.al:326-332).
        let media: Vec<(usize, [u8; 16])> = if rec.borrow().temporary {
            Vec::new()
        } else {
            let positions: Vec<usize> = (0..t.fields.len()).filter(|&i| is_media_field(t, i)).collect();
            let sets: Vec<usize> = (0..t.fields.len())
                .filter(|&i| {
                    t.fields[i].stored
                        && matches!(
                            t.fields[i].ty,
                            RtType::Library(al_builtins::library_kind::LibraryKind::MediaSet)
                        )
                })
                .collect();
            match positions.is_empty() && sets.is_empty() {
                true => Vec::new(),
                false => {
                    let stored = self
                        .store_for(rec)
                        .get(&cx, t, &pk)
                        .map_err(|e| Signal::Unsupported(format!("{e}")))?;
                    // What a Delete does to the Tenant Media rows of a MediaSet is not pinned (media/TestMediaPngImport.al:337-375
                    // pins it, but that test is refused earlier): refused while the set holds anything.
                    if sets
                        .iter()
                        .any(|&i| matches!(stored.as_ref().map(|r| &r[i]), Some(Value::Guid(g)) if *g != [0; 16]))
                    {
                        return unsupported(
                            "a Delete of a row whose MediaSet holds media (what happens to its Tenant Media rows is not pinned)",
                        );
                    }
                    positions
                        .into_iter()
                        .filter_map(|i| match stored.as_ref().map(|r| &r[i]) {
                            Some(Value::Guid(g)) if *g != [0; 16] => Some((i, *g)),
                            _ => None,
                        })
                        .collect()
                }
            }
        };
        match self
            .store_for(rec)
            .delete(&cx, t, &pk)
            .map_err(|e| Signal::Unsupported(format!("{e}")))?
        {
            WriteOutcome::Done => {
                self.deletes += 1;
                for (i, g) in media {
                    self.release_media(&cx, t, i, g)?;
                }
                if let Some(name) = company {
                    self.after_company_row_delete(&name);
                }
                // The platform deletes the user's User Property row with the User, after the
                // OnBeforeDelete subscribers and before the OnAfterDelete ones
                // (access-control/TestUserDeleteCascadeOrder.al:26-84 in the OnPrem app).
                if user_row {
                    self.user_after_delete(t, rec)?;
                }
                if run {
                    let x = Self::xrec_handle(rec, rec.borrow().fields.clone());
                    self.run_ext_trigger(t, "OnAfterDelete", rec, Some(&x))?;
                }
                self.table_event(t, "onafterdeleteevent", rec, None, run, None)?;
                Ok(Value::Boolean(true))
            }
            WriteOutcome::NotFound => self.fail(t, &row, false),
        }
    }

    /// The row as the store keeps it: FlowFields and FlowFilters hold no data, whatever the
    /// buffer shows after CalcFields or an assignment.
    pub(crate) fn stored_view(&mut self, t: &TableDef, row: &[Value]) -> Vec<Value> {
        row.iter()
            .enumerate()
            .map(|(i, v)| {
                if t.fields[i].stored {
                    v.clone()
                } else {
                    self.default_value(&t.fields[i].ty)
                }
            })
            .collect()
    }

    /// The first unique secondary key whose values `row` repeats in another row. `own` is the
    /// primary key of the row being rewritten (Modify, Rename), which does not count. For an
    /// Insert, a row with the same primary key is the duplicate-key failure instead.
    fn unique_violation(
        &mut self,
        rec: &Rec,
        t: &TableDef,
        row: &[Value],
        own: Option<&[Value]>,
    ) -> Result<Option<usize>, Signal> {
        let unique: Vec<usize> = {
            let schema = self.engine.program.objects[t.object as usize]
                .table
                .as_ref()
                .expect("table");
            (1..t.keys.len())
                .filter(|k| schema.unique_keys.get(*k).copied().unwrap_or(false))
                .collect()
        };
        if unique.is_empty() {
            return Ok(None);
        }
        let cx = self.cx_for(rec, t)?;
        if own.is_none() {
            let pk = t.primary_key(row);
            if self
                .store_for(rec)
                .get(&cx, t, &pk)
                .map_err(|e| Signal::Unsupported(format!("{e}")))?
                .is_some()
            {
                return Ok(None);
            }
        }
        let current = match own {
            Some(pk) => self
                .store_for(rec)
                .get(&cx, t, pk)
                .map_err(|e| Signal::Unsupported(format!("{e}")))?,
            None => None,
        };
        for k in unique {
            let filters: Vec<FieldFilter> = t.keys[k]
                .iter()
                .map(|&f| FieldFilter {
                    field: f,
                    expr: FilterExpr::Eq(row[f].clone()),
                    group: 0,
                })
                .collect();
            let mut n = self
                .store_for(rec)
                .count(&cx, t, &filters)
                .map_err(|e| Signal::Unsupported(format!("{e}")))?;
            // The row being rewritten matches itself when its key values do not change.
            if let Some(old) = &current {
                if t.keys[k]
                    .iter()
                    .all(|&f| ops::compare_keys(&[old[f].clone()], &[row[f].clone()]))
                {
                    n = n.saturating_sub(1);
                }
            }
            if n > 0 {
                return Ok(Some(k));
            }
        }
        Ok(None)
    }

    /// The refusal of a unique index; only the start of BC's message is pinned.
    fn fail_unique(&mut self, t: &TableDef, key: usize, row: &[Value]) -> Result<Value, Signal> {
        if self.ret_used {
            return Ok(Value::Boolean(false));
        }
        // The fields of the violated key with their values (corpus
        // record/TestCalcFieldsTruncateUniqueContracts.al:UniqueIndex_Violation_FullTextAndCode_...).
        let mut parts = Vec::new();
        for &i in &t.keys[key] {
            let v = al_builtins::format::try_format(&row[i], 0, 0, self).map_err(Signal::Unsupported)?;
            parts.push(format!("{}='{}'", t.fields[i].name, v));
        }
        Err(Signal::Error(Box::new(AlError {
            message: format!(
                "There is already a record in table {} that has the same values in a unique index for the following fields: {}",
                self.caption(t),
                parts.join(",")
            ),
            code: "DB:RecordExists".into(),
            call_stack: String::new(),
            ..Default::default()
        })))
    }

    /// Whether an insert or modify of `t` would run table code (a trigger, an extension trigger
    /// or an event subscriber). XmlPort import refuses such tables: whether it runs them is not
    /// pinned (docs/probes-needed.md).
    #[allow(dead_code)] // the XmlPort import ran no hooks before; it runs them now
    pub(crate) fn has_write_hooks(&mut self, t: &TableDef) -> Result<bool, Signal> {
        let mut any = ["OnInsert", "OnModify"]
            .iter()
            .any(|n| self.table_trigger(t, n).is_some());
        for n in [
            "OnBeforeInsert",
            "OnInsert",
            "OnAfterInsert",
            "OnBeforeModify",
            "OnModify",
            "OnAfterModify",
        ] {
            any |= !self.ext_triggers(t, n).is_empty();
        }
        for e in [
            "onbeforeinsertevent",
            "onafterinsertevent",
            "onbeforemodifyevent",
            "onaftermodifyevent",
        ] {
            any |= self.listened(t, e, None)?;
        }
        Ok(any)
    }

    /// Whether field `pos` of `t` is a FlowField (not stored, not a FlowFilter, not a system field).
    pub(crate) fn is_flowfield(&self, t: &TableDef, pos: usize) -> bool {
        let f = &t.fields[pos];
        !f.stored
            && f.id < 2_000_000_000
            && !self.engine.program.objects[t.object as usize]
                .table
                .as_ref()
                .is_some_and(|s| s.fields[pos].flow_filter)
    }

    pub(crate) fn table_trigger(&self, t: &TableDef, name: &str) -> Option<usize> {
        self.engine.program.objects[t.object as usize]
            .routines
            .iter()
            .position(|r| r.is_trigger && r.field.is_none() && r.name.eq_ignore_ascii_case(name))
    }

    /// An instance of `object` whose Rec is `rec`, for a call on that record.
    ///
    /// A table or table extension runs on the globals of the record variable (ADR-021): one
    /// instance per object, created on the first call and kept in `rec`'s state. Its `Rec` and
    /// `xRec` slots are filled for the call; give the returned [`RecSlots`] to
    /// [`Session::leave_record`] when the call ends, whether it succeeded or not, so the slots go
    /// back to what they held (nothing, or the outer call's record) and the instance holds no
    /// pointer to its record at rest. Any other object (a codeunit with `TableNo`, a page) gets a
    /// fresh instance, as before.
    pub(crate) fn enter_record(
        &mut self,
        object: usize,
        rec: Rec,
        xrec: Option<&Rec>,
    ) -> Result<(Rc<RefCell<Instance>>, RecSlots), Signal> {
        let (rs, xs, per_variable, is_ext) = {
            let o = &self.engine.program.objects[object];
            (
                o.rec_slot,
                o.xrec_slot,
                !o.uncompiled && matches!(o.kind, al_ir::ObjKind::Table | al_ir::ObjKind::TableExtension),
                !o.uncompiled && o.kind == al_ir::ObjKind::TableExtension,
            )
        };
        let existing = if per_variable {
            rec.borrow().globals.instance(object as u32)
        } else {
            None
        };
        let inst = match existing {
            Some(i) => i,
            None => {
                let i = if per_variable {
                    self.table_instance(object)
                } else {
                    self.instance(object)
                };
                if per_variable {
                    rec.borrow().globals.set_instance(object as u32, i.clone());
                }
                i
            }
        };
        // A table extension's `BaseGlobal` places are the globals of the base table's instance for the same record
        // (assumption `table-extension-base-vars`): the link is made here, and made again when the base instance is
        // a new one (a Clear or a Reset).
        if is_ext {
            let table = rec.borrow().table;
            let base = match rec.borrow().globals.instance(table) {
                Some(b) => b,
                None => {
                    let b = self.table_instance(table as usize);
                    rec.borrow().globals.set_instance(table, b.clone());
                    b
                }
            };
            inst.borrow_mut().base = Some(Rc::downgrade(&base));
        }
        // xRec: the caller's handle when the operation has one (shared, so changes made by
        // event subscribers reach the trigger and the other subscribers), else a copy of Rec. Both
        // carry the variable's xRec globals (ADR-021 decision 5).
        let xr = match (xrec, xs) {
            (Some(x), _) => {
                if x.borrow().globals.is_unset() {
                    let g = rec.borrow().globals.xrec();
                    x.borrow_mut().globals = g;
                }
                Some(Value::Record(x.clone()))
            }
            (None, Some(_)) => {
                // In a procedure that no operation of the record variable called (a plain call), xRec
                // is a blank record, not a copy of the row: Get, Insert, Modify and edits of the
                // variable do not reach it; only the globals it was given stay (corpus
                // record/PWTableGlobals.al:196-260, XRecPlainProc_*).
                let mut st = rec.borrow().clone();
                let table = rec.borrow().table as usize;
                st.fields = self.initial_fields(table);
                st.globals = rec.borrow().globals.xrec();
                Some(Value::Record(Rc::new(RefCell::new(st))))
            }
            (None, None) => None,
        };
        let mut i = inst.borrow_mut();
        let mut saved = RecSlots::default();
        if let Some(r) = rs {
            saved.rec = Some((r, std::mem::replace(&mut i.globals[r], Value::Record(rec))));
        }
        if let (Some(x), Some(v)) = (xs, xr) {
            saved.xrec = Some((x, std::mem::replace(&mut i.globals[x], v)));
        }
        drop(i);
        Ok((inst, saved))
    }

    /// Puts the `Rec` and `xRec` slots back to what they held before [`Session::enter_record`].
    pub(crate) fn leave_record(inst: &Rc<RefCell<Instance>>, saved: RecSlots) {
        let mut i = inst.borrow_mut();
        if let Some((slot, v)) = saved.rec {
            i.globals[slot] = v;
        }
        if let Some((slot, v)) = saved.xrec {
            i.globals[slot] = v;
        }
    }

    /// Runs `routine` of `object` with `rec` as its Rec (and `xrec`, or a copy of `rec`, as its xRec).
    pub(crate) fn call_on_record(
        &mut self,
        object: usize,
        routine: usize,
        rec: Rec,
        xrec: Option<&Rec>,
        args: Vec<Value>,
    ) -> Result<(Value, Vec<Value>), Signal> {
        let (inst, saved) = self.enter_record(object, rec, xrec)?;
        let r = self.call(object, routine, inst.clone(), args);
        Self::leave_record(&inst, saved);
        r
    }

    /// Sets the table globals of `rec` back to their initial values, in place: the instances
    /// stay (a running call keeps its own), their variables are re-initialised. `xRec`'s set is
    /// not touched (ADR-021 decision 4 and 5).
    pub(crate) fn reset_table_globals(&mut self, rec: &Rec) {
        let instances = rec.borrow().globals.instances();
        for (object, inst) in instances {
            let (rs, xs, tys) = {
                let o = &self.engine.program.objects[object as usize];
                (
                    o.rec_slot,
                    o.xrec_slot,
                    o.globals.iter().map(|g| g.ty.clone()).collect::<Vec<_>>(),
                )
            };
            for (i, ty) in tys.iter().enumerate() {
                if Some(i) == rs || Some(i) == xs {
                    continue;
                }
                let v = self.default_value(ty);
                inst.borrow_mut().globals[i] = v;
            }
        }
    }

    fn run_trigger(&mut self, object: usize, routine: usize, rec: &Rec, xrec: Option<&Rec>) -> Result<(), Signal> {
        self.call_on_record(object, routine, rec.clone(), xrec, Vec::new())
            .map(|_| ())
    }

    /// Every write gets a new, increasing SystemRowVersion.
    /// Returns the previous value, so a refused write can give it back
    /// (record/TestRefusedWriteRowVersion.al).
    fn bump_row_version(&mut self, t: &TableDef, rec: &Rec) -> Option<(usize, Value)> {
        let i = t.fields.iter().position(|f| f.id == 2_000_000_005)?;
        if self.temp_never_stamps(t, rec) {
            return None;
        }
        self.next_row_version += 1;
        let old = std::mem::replace(
            &mut rec.borrow_mut().fields[i],
            Value::BigInteger(self.next_row_version),
        );
        Some((i, old))
    }

    /// A BigInteger field with `SQLTimestamp = true` reads the row version: above 0 after the Insert, greater after a Modify,
    /// the same without one, and an assignment to it is not stored (record/lt-x-table-props/XTablePropsTests.al:412-415).
    /// `stored` is the row before a Modify: the field gets its value back first, so an assignment to it is no change.
    /// A temporary record never stamps (see `temp_never_stamps`).
    fn sql_timestamp_before(&self, t: &TableDef, rec: &Rec, stored: Option<&[Value]>) -> Result<(), Signal> {
        let fields = &self.engine.table_facts[t.object as usize].sql_timestamp;
        if fields.is_empty() {
            return Ok(());
        }
        if t.fields.iter().all(|f| f.id != 2_000_000_005) {
            return unsupported(format!(
                "an SQLTimestamp field of {}, a table without a row version",
                t.name
            ));
        }
        if rec.borrow().temporary {
            return Ok(());
        }
        if let Some(old) = stored {
            let mut r = rec.borrow_mut();
            for &i in fields {
                r.fields[i] = old[i].clone();
            }
        }
        Ok(())
    }

    /// A temporary record of a table with an SQLTimestamp field never stamps: the field and SystemRowVersion keep what the buffer
    /// holds through Insert, Modify and Rename (0 for a new row, the database row's values for a copy of one, a value code assigned
    /// to the field), and Init keeps them (record/lt-qr-temp-sqltimestamp/QrTempTimestamp.al:76-85, :111-115, :160-170, :221-226).
    /// A temporary record of a table without one is not pinned and keeps its own row versions.
    fn temp_never_stamps(&self, t: &TableDef, rec: &Rec) -> bool {
        rec.borrow().temporary && !self.engine.table_facts[t.object as usize].sql_timestamp.is_empty()
    }

    /// The SQLTimestamp fields take the row version the write gives the row (see `sql_timestamp_before`).
    fn sql_timestamp_after(&self, t: &TableDef, rec: &Rec) {
        let fields = &self.engine.table_facts[t.object as usize].sql_timestamp;
        if fields.is_empty() || rec.borrow().temporary {
            return;
        }
        if let Some(v) = t.fields.iter().position(|f| f.id == 2_000_000_005) {
            let mut r = rec.borrow_mut();
            let version = r.fields[v].clone();
            for &i in fields {
                r.fields[i] = version.clone();
            }
        }
    }

    /// A Code field the database stores as a number (`SQLDataType = Integer`) takes the empty value and plain numbers. A
    /// Modify of `0` or a number with a leading zero fails with error code 22928070, one of letters with 22928072
    /// (record/lt-x-table-props/XTablePropsTests.al:352-353, :368-369, :384-385); the values '10' and '9' are taken
    /// (:322-325). Any other value, any such value an Insert writes, and every value of a `BigInteger` one other than those
    /// taken are not pinned: refused.
    fn check_code_storage(&self, t: &TableDef, row: &[Value], modify: bool) -> Result<(), Signal> {
        if !self.engine.table_facts[t.object as usize].code_sql_type {
            return Ok(());
        }
        let schema = self.engine.program.objects[t.object as usize]
            .table
            .as_ref()
            .expect("table");
        for (i, f) in t.fields.iter().enumerate() {
            if !matches!(f.ty, RtType::Code(_)) {
                continue;
            }
            let declared = schema.fields[i]
                .properties
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("SQLDataType"))
                .map(|(_, v)| v.trim());
            let Some(declared) = declared.filter(|v| !v.eq_ignore_ascii_case("Varchar")) else {
                continue;
            };
            if f.code_storage == al_data::CodeStorage::Text {
                return unsupported(format!("Code field {} with SQLDataType = {declared}", f.name));
            }
            if t.keys[0].contains(&i) {
                return unsupported(format!(
                    "Code field {} with SQLDataType = {declared} in the primary key: its order is not pinned",
                    f.name
                ));
            }
            let value = al_builtins::native::text_of(&row[i])
                .map(|v| v.to_string())
                .unwrap_or_default();
            let digits = !value.is_empty() && value.bytes().all(|c| c.is_ascii_digit());
            if value.is_empty() || (digits && !value.starts_with('0') && value.parse::<i32>().is_ok()) {
                continue;
            }
            let number = if digits && value.starts_with('0') {
                ("22928070", "NDBCS:BadCSideCodeData")
            } else if value.bytes().all(|c| c.is_ascii_uppercase()) {
                ("22928072", "NDBCS:BadSqlIntCodeData")
            } else {
                return unsupported(format!(
                    "the value '{value}' in the Code field {}, which the database stores as a number: the outcome is not pinned",
                    f.name
                ));
            };
            if !modify || f.code_storage != al_data::CodeStorage::Integer {
                return unsupported(format!(
                    "{} of '{value}' into the Code field {} with SQLDataType = {declared}: the outcome is not pinned",
                    if modify { "a Modify" } else { "an Insert" },
                    f.name
                ));
            }
            // Only a table named as its caption, with one Integer primary key field named as its caption, is pinned.
            let pk = &t.keys[0];
            if pk.len() != 1
                || !matches!(t.fields[pk[0]].ty, RtType::Integer)
                || self.caption(t) != t.name
                || self.field_caption(t, pk[0]) != t.fields[pk[0]].name
            {
                return unsupported(format!(
                    "the error of a value that is no integer in Code field {} of {}: the identification of this primary key is not pinned",
                    f.name, t.name
                ));
            }
            let key = match &row[pk[0]] {
                Value::Integer(n) => n.to_string(),
                _ => return unsupported("an Integer primary key field that holds no Integer"),
            };
            return Err(crate::transactions::al_error_code(
                &format!(
                    "The {} table contains a value that is not valid in an integer Code field:\\\\{}='{key}'\\Error code: {}",
                    t.name, t.fields[pk[0]].name, number.0
                ),
                number.1,
            ));
        }
        Ok(())
    }

    fn restore_row_version(rec: &Rec, old: Option<(usize, Value)>) {
        if let Some((i, v)) = old {
            rec.borrow_mut().fields[i] = v;
        }
    }

    /// Gives the row its SystemId: the one in the buffer when `keep` (`Insert(_, true)`) and it is
    /// set, otherwise a fresh one (corpus record/TestRecordInsert.al:84-110).
    fn assign_system_id(&mut self, t: &TableDef, rec: &Rec, keep: bool) {
        // A temporary record has no SystemId: it stays the null Guid
        // (record/TestSystemFieldStampsContracts.al:115, TemporaryRecord_HasNoStampsAndNoSystemId).
        if rec.borrow().temporary {
            return;
        }
        if let Some(i) = t.fields.iter().position(|f| f.id == 2_000_000_000) {
            let mut r = rec.borrow_mut();
            let unset = matches!(r.fields[i], Value::Guid(g) if g == [0; 16]);
            if unset || !keep {
                // Deterministic ids that look random (ADR-006, `fresh_guid`).
                r.fields[i] = Value::Guid(self.fresh_system_id());
            }
        }
    }

    pub(crate) fn scan(
        &mut self,
        rec: &Rec,
        t: &TableDef,
        ascending: bool,
        after: Option<Vec<Value>>,
    ) -> Result<Option<Vec<Value>>, Signal> {
        if rec.borrow().field_descending != 0 {
            return self.scan_first_field_reversed(rec, t, ascending, after);
        }
        self.scan_plain(rec, t, ascending, after)
    }

    /// A scan in the order `SetAscending(Field, false)` gives: the fields marked descending run backwards and the rest
    /// of the key, then the primary key, forwards (records/W4SortKeys.al:SetAscending_SingleFieldKey: key Txt, ids
    /// `4,2,3,1,5`, where `Ascending(false)` and a view give `4,3,2,5,1`; record/lt-filters-keys-finds/P68D_Ascending.al). The rows are read in key order and sorted
    /// again, so a walk over a big table costs more than one in the key's own order.
    fn scan_first_field_reversed(
        &mut self,
        rec: &Rec,
        t: &TableDef,
        forward: bool,
        after: Option<Vec<Value>>,
    ) -> Result<Option<Vec<Value>>, Signal> {
        use std::cmp::Ordering;
        let key = rec.borrow().key;
        let mut rows: Vec<(Vec<Value>, Vec<Value>)> = Vec::new();
        let mut cursor = None;
        while let Some(row) = self.scan_plain(rec, t, true, cursor.take())? {
            let pos = position(t, key, &row);
            cursor = Some(pos.clone());
            rows.push((pos, row));
        }
        let mask = rec.borrow().field_descending;
        let order = |a: &[Value], b: &[Value]| -> Ordering {
            for (i, (x, y)) in a.iter().zip(b).enumerate() {
                let o = ops::compare(x, y).unwrap_or(Ordering::Equal);
                let o = if i < 32 && mask & (1 << i) != 0 { o.reverse() } else { o };
                if o != Ordering::Equal {
                    return o;
                }
            }
            Ordering::Equal
        };
        rows.sort_by(|a, b| order(&a.0, &b.0));
        let hit = if forward {
            rows.into_iter()
                .find(|(p, _)| after.as_deref().is_none_or(|a| order(p, a) == Ordering::Greater))
        } else {
            rows.into_iter()
                .rev()
                .find(|(p, _)| after.as_deref().is_none_or(|a| order(p, a) == Ordering::Less))
        };
        Ok(hit.map(|(_, row)| row))
    }

    fn scan_plain(
        &mut self,
        rec: &Rec,
        t: &TableDef,
        ascending: bool,
        after: Option<Vec<Value>>,
    ) -> Result<Option<Vec<Value>>, Signal> {
        let temporary = rec.borrow().temporary;
        self.unseeded_read(t, temporary, "reading rows")?;
        self.coverage_read(t, temporary)?;
        let key = rec.borrow().key;
        // Filters on FlowFilter fields drop out; those on FlowFields are checked per row.
        let (filters, flow_field_filters) = self.split_filters(rec);
        let marked_only = rec.borrow().marked_only;
        let cx = self.cx_for(rec, t)?;
        let mut after = after;
        loop {
            let store = self.store_for(rec);
            let row = store
                .next(
                    &cx,
                    t,
                    &Scan {
                        key,
                        filters: &filters,
                        ascending,
                        after: after.as_deref(),
                    },
                )
                .map_err(|e| Signal::Unsupported(format!("store error {e}")))?;
            match row {
                // MarkedOnly narrows the set to the marked rows (corpus record/TestRecordFind.al:258-290).
                Some(row) if marked_only && !Self::is_marked(rec, t, &row) => after = Some(position(t, key, &row)),
                Some(row) if !flow_field_filters.is_empty() => {
                    if self.passes_flow_field_filters(rec, &row, &flow_field_filters)? {
                        return Ok(Some(row));
                    }
                    after = Some(position(t, key, &row));
                }
                other => return Ok(other),
            }
        }
    }

    /// Whether `row` passes the record's filters (every group), its FlowField filters and
    /// MarkedOnly.
    fn row_passes(&mut self, rec: &Rec, t: &TableDef, row: &[Value]) -> Result<bool, Signal> {
        let (filters, flow_field_filters) = self.split_filters(rec);
        if filters.iter().any(|f| !f.expr.matches(&row[f.field])) {
            return Ok(false);
        }
        if rec.borrow().marked_only && !Self::is_marked(rec, t, row) {
            return Ok(false);
        }
        if flow_field_filters.is_empty() {
            return Ok(true);
        }
        self.passes_flow_field_filters(rec, row, &flow_field_filters)
    }

    /// Whether the record last read the row with primary key `pk` (see `RecordState::position`).
    fn on_own_row(rec: &Rec, pk: &[Value]) -> bool {
        rec.borrow()
            .position
            .as_deref()
            .is_some_and(|p| ops::compare_keys(p, pk))
    }

    fn is_marked(rec: &Rec, t: &TableDef, row: &[Value]) -> bool {
        let pk = t.primary_key(row);
        rec.borrow()
            .marks
            .iter()
            .any(|m| key_cmp(m, &pk) == std::cmp::Ordering::Equal)
    }

    /// Whether another row of the record's table holds the SystemId in its buffer.
    fn system_id_taken(&mut self, rec: &Rec, t: &TableDef) -> Result<bool, Signal> {
        let Some(i) = t.fields.iter().position(|f| f.id == 2_000_000_000) else {
            return Ok(false);
        };
        let id = rec.borrow().fields[i].clone();
        let filters = [FieldFilter {
            field: i,
            expr: FilterExpr::Eq(id),
            group: 0,
        }];
        let cx = self.cx_for(rec, t)?;
        let taken = self
            .store_for(rec)
            .count(&cx, t, &filters)
            .map_err(|e| Signal::Unsupported(format!("{e}")))?;
        Ok(taken > 0)
    }

    /// Refuses an explicit SystemId another row already holds. The wording after the pinned
    /// fragments is docs/probes-needed.md (record/TestRecordSystemId.al:86-168, TestAutoIncrementEdges.al:282).
    fn check_system_id_unique(&mut self, rec: &Rec, t: &TableDef) -> Result<(), Signal> {
        let Some(i) = t.fields.iter().position(|f| f.id == 2_000_000_000) else {
            return Ok(());
        };
        if !self.system_id_taken(rec, t)? {
            return Ok(());
        }
        let id = rec.borrow().fields[i].clone();
        let shown = al_builtins::format::try_format(&id, 0, 0, self).map_err(Signal::Unsupported)?;
        Err(Signal::Error(Box::new(AlError {
            // record/TestSystemFieldStampsContracts.al:153 (SystemId_Duplicate_And_ModifyAll_Texts;
            // the caption of the field is "System ID").
            message: format!(
                "There is already a record in table {} that has the same values in a unique index for the following fields: System ID='{}'",
                self.caption(t),
                shown
            ),
            code: "DB:RecordExists".into(),
            call_stack: String::new(),
            ..Default::default()
        })))
    }

    /// Reads a row into the record, then calculates its `SetAutoCalcFields` fields. A database
    /// table hands out its own copy of every Blob, not yet loaded
    /// (streams/TestBlobUncommittedWriteIsolation.al, TestBlob.al Blob_Get_WithoutCalcFields_InStreamReadsEmpty);
    /// a temporary table's rows hold the record's own Blob objects
    /// (TempBlob_UncommittedWrite_SameInstanceReGet_KeepsWrite).
    pub(crate) fn load(&mut self, rec: &Rec, mut row: Vec<Value>) -> Result<(), Signal> {
        // A temporary table keeps whole milliseconds; only a write to a database table rounds them to 1/300 s
        // (errors/lt-ms-refusals-130/P130DateFilters.al:324, :402).
        if rec.borrow().temp_id.is_none() {
            for v in &mut row {
                if let Value::Blob(b) = v {
                    *b = al_builtins::stream::copy_blob_db_row(b);
                }
            }
        } else {
            // A Blob a Modify persisted reads empty after a Get until CalcFields, as a database record's does; one still being
            // written keeps its content (misc/lt-probes-needed-1/PnABlobAssignRename.al:Assign_Temporary `regot-source = ''`).
            for v in &mut row {
                if let Value::Blob(b) = v {
                    if b.borrow().persisted {
                        *b = al_builtins::stream::copy_blob_temp_row(b);
                    }
                }
            }
        }
        // A full load (the default) needs no bookkeeping beyond the flag; a partial one is worked
        // out in partial.rs.
        let via_get = std::mem::take(&mut self.get_load);
        // A temporary record loads every field whatever SetLoadFields asked for (record/lt-are-fields-loaded-unfetched/
        // AreFieldsLoadedUnfetched.al:174-221, partial_name and partial_qty after FindFirst and Get).
        if rec.borrow().load.request.is_some() && !rec.borrow().temporary {
            self.note_partial_fetch(rec, &row, via_get)?;
        } else {
            let mut r = rec.borrow_mut();
            r.load.buffer = Some(al_builtins::value::Buffer {
                have: al_builtins::value::FieldSet::All,
                unsure: al_builtins::value::FieldSet::Of(Vec::new()),
                row: None,
                cleared: false,
            });
            r.load.partial = false;
        }
        let t = self.table(rec)?;
        let pk = t.primary_key(&row);
        // The record remembers the row as its own only when the row passed its filters when it was read; a filter set later forgets it
        // (record/TestFindSetSortKeyModify.al:OwnFilter_ModifiedOutOfIt_FindStillFindsTheRow against misc/lt-probes-needed-3/
        // PnC02Filters.al:FindEqual_OutsideFilter and record/lt-record-read/RcAMisc.al:MarkedOnly_With_Find_Get_And_Order).
        let passes = self.row_passes(rec, &t, &row)?;
        {
            let mut r = rec.borrow_mut();
            r.position = passes.then_some(pk);
            r.fields = row;
        }
        self.auto_calculate(rec)
    }

    /// Marks the Blobs that hold content, in the stored row and in the record, as stored by an `Insert` (`inserted`) or not (a `Modify`).
    fn mark_blobs_inserted(rec: &Rec, stored: &[Value], inserted: bool) {
        for v in stored.iter().chain(rec.borrow().fields.iter()) {
            if let Value::Blob(b) = v {
                let mut b = b.borrow_mut();
                b.inserted = inserted && !b.bytes.is_empty();
            }
        }
    }

    /// The Date, Time and DateTime values of a database table's row as its SQL columns hold them (`sqltime`): `Some` error
    /// for a value the column cannot hold. A temporary table keeps the values as they are.
    fn sql_columns(&self, rec: &Rec, t: &TableDef, row: &mut [Value]) -> Result<Option<AlError>, Signal> {
        if rec.borrow().temporary {
            return Ok(None);
        }
        let caption = self.caption(t);
        match crate::sqltime::store(t, &|i| self.field_caption(t, i), &caption, row)? {
            crate::sqltime::Stored::Fits => Ok(None),
            crate::sqltime::Stored::Bad(e) => Ok(Some(e)),
        }
    }

    /// The row to hand to the store: a database table keeps its own copy of every Blob.
    fn to_store(rec: &Rec, mut row: Vec<Value>) -> Vec<Value> {
        if rec.borrow().temp_id.is_none() {
            for v in &mut row {
                if let Value::Blob(b) = v {
                    *b = al_builtins::stream::copy_blob(b, true);
                }
            }
        }
        row
    }

    /// `CalcFields` of Blob fields: a database record's Blob is fetched from the stored row unless
    /// the buffer already holds it (an in-memory write is kept:
    /// streams/TestBlobCalcFieldsUncommittedWrite.al); a temporary record's Blob is always loaded.
    pub(crate) fn load_blobs(&mut self, rec: &Rec, positions: &[usize]) -> Result<(), Signal> {
        let t = self.table(rec)?;
        let cx = self.cx_for(rec, &t)?;
        let mut stored: Option<Vec<Value>> = None;
        for &pos in positions {
            let buf = match rec.borrow().fields[pos].clone() {
                Value::Blob(buf) => buf,
                // A platform row's column no corpus test pins: its own reason.
                Value::Unknown(why) => return unsupported(why.to_string()),
                _ => return unsupported("CalcFields of a Blob field that holds no Blob"),
            };
            // A temporary record's row that was never inserted has no stored content: CalcFields empties a Blob a stream
            // wrote (P94XpDelim.al:Export_NothingDeclared exports into such a Blob and reads back nothing). A Blob of a row read
            // with `Get` and written in memory is read back from the row too (record/lt-blob-calcfields-length/
            // BlobCalcFieldsLength.al:BlobCalcFields_UnsavedContent_Result, 17 bytes in memory, 6 after CalcFields): see `BlobBuf::fetched`.
            let temporary = rec.borrow().temporary;
            let keeps_content = buf.borrow().fetched && !buf.borrow().refetch && !temporary;
            if stored.is_none() {
                let pk = t.primary_key(&rec.borrow().fields);
                stored = self
                    .store_for(rec)
                    .get(&cx, &t, &pk)
                    .map_err(|e| Signal::Unsupported(format!("{e}")))?;
            }
            // A row this variable inserted and that is gone (an error inside ASSERTERROR rolled the Insert back) leaves the Blob empty, whatever a
            // stream wrote since (misc/lt-probes-needed-5/PnE08RecordMisc.al:TestField_Blob `af219`).
            if keeps_content && (stored.is_some() || !rec.borrow().inserted) {
                continue;
            }
            // A row that is not stored (a copy of a record never inserted) leaves the Blob empty: the buffer
            // showed `*` after `Copy1 := Row`, and CalcFields reads nothing back
            // (types/ProbeTypeFormsAndFilters.al:FormatRecord_AssignmentCopiesBlobBytes, BC 27.5).
            let (bytes, pending, inserted) = match stored.as_ref().map(|row| row[pos].clone()) {
                Some(Value::Blob(src)) => {
                    let src = src.borrow();
                    (src.bytes.clone(), src.render_pending.clone(), src.inserted)
                }
                _ => (Vec::new(), None, false),
            };
            let mut b = buf.borrow_mut();
            b.inserted = inserted;
            b.bytes = bytes;
            b.written = false;
            b.fetched = true;
            // ADR-040: the stored row carries the mark of an output that was never produced.
            b.render_pending = pending;
            b.loaded = true;
            b.refetch = false;
        }
        Ok(())
    }

    /// A save makes the Blobs of the buffer what is stored: none counts as written any more.
    fn clear_blob_written(rec: &Rec) {
        for v in &rec.borrow().fields {
            if let Value::Blob(b) = v {
                let mut b = b.borrow_mut();
                b.written = false;
                // A Blob the record read without CalcFields stays as stored by the save (record/lt-loadfields-pagesource-validate/
                // P81LoadFields.al:700-711) and stays unloaded: a later CalcFields fetches it (streams/TestBlob.al
                // Blob_Get_WithoutCalcFields_InStreamReadsEmpty), it does not keep the empty view the buffer shows.
                if b.loaded {
                    b.fetched = true;
                }
            }
        }
    }

    /// The platform's virtual table `Agent Task` (2000000262): in a test session IsEmpty fails with the agent service's
    /// error, code DB:Ok (platform/lt-rh-agent-session/RhAgentSession.al:74-88 RH_AgentTask_TaskExistsUnknownAgent, through
    /// "Agent Task Impl.".TaskExists, whose only read of the table is IsEmpty after two SetRange calls and ReadIsolation).
    /// Every other read or write of it stays refused by its provider ("has no provider").
    fn agent_task_check(&self, t: &TableDef, rec: &Rec, method: RecordOp) -> Result<(), Signal> {
        if method != RecordOp::IsEmpty || rec.borrow().temporary {
            return Ok(());
        }
        let object = &self.engine.program.objects[t.object as usize];
        let platform = self
            .engine
            .program
            .apps
            .get(object.app as usize)
            .is_some_and(|a| a.id.eq_ignore_ascii_case(crate::platform::objects::al_sema_platform_app_id()));
        // Only a read from Microsoft code (the System Application) is pinned; a partner app's own read is not.
        let microsoft = self.frames.last().is_some_and(|o| {
            let app = self.engine.program.objects[*o].app as usize;
            self.engine.program.apps.get(app).is_some_and(|a| a.publisher == "Microsoft")
        });
        if !platform || object.id != Some(2000000262) || !microsoft {
            return Ok(());
        }
        Err(crate::transactions::al_error_code(
            "Agents are temporarily unavailable. Please try again later. You can check the Business Central Admin Center and Microsoft 365 Service Health dashboard for updates.",
            "DB:Ok",
        ))
    }

    /// A `LinkedObject = true` table has no SQL table behind it in a test database: Insert and Count raise the SQL error
    /// (record/lt-x-table-props/XTablePropsTests.al:720-737, text and code). The other database operations are not pinned.
    fn linked_object_check(&self, t: &TableDef, rec: &Rec, method: RecordOp) -> Result<(), Signal> {
        if rec.borrow().temporary {
            return Ok(());
        }
        let linked = self.engine.program.objects[t.object as usize]
            .properties
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("LinkedObject") && v.trim().eq_ignore_ascii_case("true"));
        if !linked {
            return Ok(());
        }
        match method {
            RecordOp::Insert if self.ret_used => unsupported(format!(
                "Insert with its result used on LinkedObject table {} (only the statement form is pinned)",
                t.name
            )),
            RecordOp::Insert | RecordOp::Count => Err(crate::transactions::al_error_code(
                "An unexpected error occurred after a database command was cancelled.",
                "Sql:Sql",
            )),
            m if m.writes()
                || matches!(
                    m,
                    RecordOp::Find
                        | RecordOp::FindFirst
                        | RecordOp::FindLast
                        | RecordOp::FindSet
                        | RecordOp::Next
                        | RecordOp::Get
                        | RecordOp::GetBySystemId
                        | RecordOp::IsEmpty
                        | RecordOp::CountApprox
                        | RecordOp::CalcSums
                        | RecordOp::CalcFields
                        | RecordOp::LockTable
                ) =>
            {
                unsupported(format!(
                    "{} on LinkedObject table {} (only Insert and Count are pinned)",
                    m.name(),
                    t.name
                ))
            }
            _ => Ok(()),
        }
    }

    /// The first use of a record of a `TableType = CRM` table fails while no CRM table connection is
    /// registered, whatever the use (Init, TableCaption, a field, IsEmpty): text and code from
    /// session/TestCrmTableNoConnectionContracts.al:15-17,111-118. With a connection registered what
    /// the table does is not pinned.
    pub(crate) fn connection_check(&self, rec: &Rec) -> Result<(), Signal> {
        // A temporary record of a CRM table is an in-memory table of its own and needs no connection (not
        // pinned by a corpus line; Base Application's `CRM Synch. Helper.ClearCache` relies on it,
        // docs/probes-needed.md).
        if !rec.borrow().crm || rec.borrow().temporary {
            return Ok(());
        }
        if self.table_connections.iter().any(|(kind, _)| *kind == 0) {
            return unsupported("a TableType = CRM table through a registered table connection (not modelled)");
        }
        Err(crate::transactions::al_error_code(
            "Table connection  for table type CRM must be registered using RegisterTableConnection or cmdlet New-NAVTableConnection before it can be used.",
            "Argument",
        ))
    }

    /// A record method on a record whose buffer is not pinned (`RecordState::unpinned_buffer`): methods that do not read the
    /// buffer run, a read that fills it again (Get, FindFirst, FindLast, FindSet, Find('-'/'+') answering true) makes it
    /// pinned again, and every other method refuses. Rec in the OnOpenPage of a page an action opens through `RunPageLink`
    /// ([`RUN_PAGE_LINK_OPEN`]) is narrower, see [`Self::run_page_link_open_op`].
    fn unpinned_buffer_op(&mut self, method: RecordOp, rec: &Rec, args: &mut [Value]) -> Result<Value, Signal> {
        let why = rec.borrow().unpinned_buffer.unwrap_or_default();
        if why == RUN_PAGE_LINK_OPEN {
            return self.run_page_link_open_op(method, rec, args);
        }
        let refill = match method {
            RecordOp::Get | RecordOp::FindFirst | RecordOp::FindLast | RecordOp::FindSet => true,
            RecordOp::Find => matches!(args.first(), Some(Value::Text(w)) if w.as_str() == "-" || w.as_str() == "+"),
            _ => false,
        };
        let independent = matches!(
            method,
            RecordOp::Reset
                | RecordOp::SetCurrentKey
                | RecordOp::SetAscending
                | RecordOp::Ascending
                | RecordOp::SetRange
                | RecordOp::SetFilter
                | RecordOp::FilterGroup
                | RecordOp::FieldNo
                | RecordOp::SetLoadFields
                | RecordOp::AddLoadFields
                | RecordOp::SetAutoCalcFields
                | RecordOp::MarkedOnly
                | RecordOp::ClearMarks
                | RecordOp::Count
                | RecordOp::CountApprox
                | RecordOp::IsEmpty
                | RecordOp::DeleteAll
                | RecordOp::ModifyAll
                | RecordOp::IsTemporary
                | RecordOp::TableName
                | RecordOp::TableCaption
                | RecordOp::LockTable
                | RecordOp::ReadIsolation
                | RecordOp::SecurityFiltering
        );
        if !refill && !independent {
            return unsupported(why);
        }
        let flag = rec.borrow_mut().unpinned_buffer.take();
        let r = self.record_op_inner(method, rec, args);
        let refilled = refill && matches!(r, Ok(Value::Boolean(true)));
        if !refilled {
            rec.borrow_mut().unpinned_buffer = flag;
        }
        r
    }

    /// A record method on Rec in the OnOpenPage of a page an action opens through `RunPageLink`. Real BC fills the linked fields
    /// there and `GetFilters()` of a copy of Rec reads '' (corpus issue 712, writer QZ), while the runtime holds the link as a
    /// filter: every read of the buffer, the filters, the view or the rows refuses. What runs: `GetFilter` of one field and
    /// `FilterGroup` (pinned per filter group, handlers/TestActionRunPageLinkFilterGroup.al:119-131), `CurrentKey` and `Ascending`
    /// (pinned with a RunPageView sorting, handlers/TestPageActionRunPageView.al:121-134, asserted at :341), the filter and key
    /// setters, the table's names, and a `Get`, which ignores the filters and fills the buffer again.
    fn run_page_link_open_op(&mut self, method: RecordOp, rec: &Rec, args: &mut [Value]) -> Result<Value, Signal> {
        let runs = matches!(
            method,
            RecordOp::Get
                | RecordOp::GetFilter
                | RecordOp::FilterGroup
                | RecordOp::CurrentKey
                | RecordOp::Ascending
                | RecordOp::Reset
                | RecordOp::SetCurrentKey
                | RecordOp::SetAscending
                | RecordOp::SetRange
                | RecordOp::SetFilter
                | RecordOp::FieldNo
                | RecordOp::SetLoadFields
                | RecordOp::AddLoadFields
                | RecordOp::SetAutoCalcFields
                | RecordOp::IsTemporary
                | RecordOp::TableName
                | RecordOp::TableCaption
                | RecordOp::LockTable
                | RecordOp::ReadIsolation
                | RecordOp::SecurityFiltering
        );
        if !runs {
            return unsupported(format!("Rec.{}() {RUN_PAGE_LINK_OPEN}", method.name()));
        }
        let flag = rec.borrow_mut().unpinned_buffer.take();
        let r = self.record_op_inner(method, rec, args);
        if !(method == RecordOp::Get && matches!(r, Ok(Value::Boolean(true)))) {
            rec.borrow_mut().unpinned_buffer = flag;
        }
        r
    }

    /// A record method: [`Session::record_op_inner`], with what it does to the snapshot `Record.IsDirty` compares with
    /// (isdirty.rs).
    pub(crate) fn record_op(&mut self, method: RecordOp, rec: &Rec, args: &mut [Value]) -> Result<Value, Signal> {
        if self.sql.active {
            return self.record_op_counted(method, rec, args);
        }
        self.record_op_tracked(method, rec, args)
    }

    /// `record_op` while the SQL counters log record methods (`sqlcount.rs`).
    #[cold]
    #[inline(never)]
    fn record_op_counted(&mut self, method: RecordOp, rec: &Rec, args: &mut [Value]) -> Result<Value, Signal> {
        let entry = self.sql.enter(self.calls_made);
        let r = self.record_op_tracked(method, rec, args);
        let (temp, table) = {
            let s = rec.borrow();
            (s.temporary, s.table)
        };
        let key = if crate::sqlcount::SqlCounters::wants_key(method) {
            self.table(rec)
                .ok()
                .map(|t| al_data::position(&t, 0, &rec.borrow().fields))
        } else {
            None
        };
        let calls = self.calls_made;
        self.sql.leave(entry, calls, method, temp, table, args, key, &r);
        r
    }

    pub(crate) fn record_op_inner(&mut self, method: RecordOp, rec: &Rec, args: &mut [Value]) -> Result<Value, Signal> {
        self.connection_check(rec)?;
        if rec.borrow().unpinned_buffer.is_some() {
            return self.unpinned_buffer_op(method, rec, args);
        }
        let t = self.table(rec)?;
        // A table declared ObsoleteState = Moved is not in the runtime app: AllObj has no row for it and RecordRef.Open
        // fails (platform/lt-aw-fresh-tenant/AwTests.al:413-442). What a record of it does is not pinned.
        if self.engine.object_facts.get(t.object as usize).is_some_and(|f| f.moved) {
            return unsupported(format!(
                "{} on table {}, which is declared ObsoleteState = Moved (what BC does is not pinned)",
                method.name(),
                t.name
            ));
        }
        self.linked_object_check(&t, rec, method)?;
        self.agent_task_check(&t, rec, method)?;
        self.note_table(t.object, method.writes());
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
            // session/TestProbeSessionMiscContracts.al:Company_Names (69815, `Count() >= 1`). Kept, not pinned.
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
        // BC counts 1 Company Information row and 36 Accounting Period rows (session/lt-company-information/
        // CompanyInformationValues.al:45 QG_CompanyInformation_CorpusRow, :89 QG_AccountingPeriod_CorpusRows); the runtime has
        // only what install code wrote (ADR-047 rejected). A test that writes its own rows is answered. Media Resources holds
        // 46 rows there (platform/lt-platform-tables/PtCloudTables.al:269 QX_MediaResources_FreshTenantCodes, read with FindSet),
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
        // stay buffered: misc/lt-protected-var-abs-query-views/P146TableExtProtected.al:P146_Triggers_BaseThenExtension, where
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
        // (install/lt-bf-install-probes/BfInstallTests.al:138-139, reads at BfInstallFixtures.al:131-146; ADR-046
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
            // Inside a TryFunction whose result is used only Insert is pinned (misc/lt-cu-odd-leads/CuTryTests.al:83-150);
            // writes to a temporary record are allowed (:153-171), and never reach here.
            if method == RecordOp::Insert {
                // Whether the check comes before the table's insert code is not pinned. Subscribers and table-extension
                // triggers that would see the order are refused. The table's own OnInsert trigger is kept: the pinned
                // table has one (misc/lt-cu-odd-leads/CuFixtures.al:20-24), and refusing it would lose 85310
                // TryInsertThenError_ExpressionForm, TryInsertThenError_IfNotForm and TryInsertOnly_ExpressionForm.
                if self.used_try_write_applies()
                    && (self.listened(&t, "onbeforeinsertevent", None)?
                        || self.listened(&t, "onafterinsertevent", None)?
                        || ["OnBeforeInsert", "OnInsert", "OnAfterInsert"]
                            .iter()
                            .any(|n| !self.ext_triggers(&t, n).is_empty()))
                {
                    return unsupported(
                        "Insert inside a TryFunction whose result is used, on a table with insert event subscribers or table-extension insert triggers (whether they run before the TryFunctionWrite error is not pinned)",
                    );
                }
                self.gate_used_try_write("INSERT")?;
            } else {
                self.refuse_used_try_write(method)?;
            }
            self.user_bulk_refusal(&t, rec.borrow().temporary, method)?;
            // A write call starts the transaction, also a DeleteAll that finds no row
            // (session/ProbeSystemEnumMembers.al:143-161, session/TestDatabaseExtended.al:
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
            // A read call starts it too (session/ProbeSystemEnumMembers.al:94-112).
            self.tx.tx_phase = crate::transactions::TX_READ;
        }
        match method {
            RecordOp::Init => {
                let mut fresh = self.initial_fields(t.object as usize);
                if self.temp_never_stamps(&t, rec) {
                    // Init of a temporary record keeps the SQLTimestamp fields and SystemRowVersion
                    // (record/lt-qr-temp-sqltimestamp/QrTempTimestamp.al:221-226, `shared copy insert`).
                    let r = rec.borrow();
                    let facts = &self.engine.table_facts[t.object as usize].sql_timestamp;
                    for (i, f) in t.fields.iter().enumerate() {
                        if f.id == 2_000_000_005 || facts.contains(&i) {
                            fresh[i] = r.fields[i].clone();
                        }
                    }
                }
                let mut r = rec.borrow_mut();
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
                    r.is_dirty_base = al_builtins::value::DirtyBase::Row { base: old, alt: None };
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
            RecordOp::Clear => {
                // Clear resets the buffer, the filters and the keys but does not empty the table of a
                // temporary record: its rows stay and it stays temporary
                // (types/ProbeTypeFormsAndFilters.al:Clear_TemporaryRecordWithRows_ThenCount, BC 27.5).
                let fresh = self.initial_fields(t.object as usize);
                let old_row_id = Some(crate::partial::row_id(&t, &rec.borrow().fields));
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
                // Clear sets ReadIsolation back to Default (misc/lt-cv-odd-leads/CvReadIsolationState.al:58, :64).
                r.read_isolation = 0;
                // A Clear leaves the record's record of what it fetched: the fields in hand stay loaded for a later
                // partial fetch (session/TestProbeSessionMiscContracts.al:PartialRecords_AreFieldsLoaded: a full Get,
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
            RecordOp::Insert => {
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
                    self.run_ext_trigger(&t, "OnInsert", rec, insert_xrec.as_ref())?;
                }
                // The SystemId the OnBeforeInsertEvent subscribers and the insert triggers left in the buffer.
                let set_by_code = id_slot.map(|i| rec.borrow().fields[i].clone());
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
                // RecordRef.Insert and on a temporary record (record/lt-qr-systemid-insert-events/QrSystemIdTests.al:56-78, :150-196;
                // record/lt-at-partner-gaps/AtSystemIdOnInsert.al:135-170; record/lt-af-record-events/AfSystemIdBeforeInsert.al).
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
                self.check_code_storage(&t, &rec.borrow().fields, false)?;
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
                    return self.fail_unique(&t, unique_key, &row);
                }
                let mut stored = Self::to_store(rec, self.stored_view(&t, &row));
                if let Some(e) = self.sql_columns(rec, &t, &mut stored)? {
                    // A value the SQL column cannot hold: the Insert does not fail, the next access of the table does
                    // (errors/lt-ms-refusals-130/P130DateFilters.al:255-275, stage `get`), as for a duplicate key. With a Blob
                    // field, an AutoIncrement field, the result used, an after-insert trigger or event, or a page write it is
                    // not pinned.
                    let has_blob = t
                        .fields
                        .iter()
                        .any(|f| matches!(&f.ty, RtType::Library(al_builtins::library_kind::LibraryKind::Blob)));
                    if self.ret_used
                        || has_blob
                        || auto.is_some()
                        || self.tx.immediate_errors
                        || (run && !self.ext_triggers(&t, "OnAfterInsert").is_empty())
                        || self.listened(&t, "onafterinsertevent", None)?
                    {
                        return unsupported(format!(
                            "an Insert into {} of a value its SQL column cannot hold, in a form the corpus does not pin",
                            t.name
                        ));
                    }
                    Self::restore_row_version(rec, old_version);
                    Self::unstamp(rec, stamped);
                    self.tx.deferred_insert.get_or_insert((t.object as usize, e));
                    return Ok(Value::Boolean(true));
                }
                Self::mark_blobs_inserted(rec, &stored, true);
                match self
                    .store_for(rec)
                    .insert(&cx, &t, stored.clone())
                    .map_err(|e| Signal::Unsupported(format!("{e}")))?
                {
                    InsertOutcome::Inserted => {
                        Self::clear_blob_written(rec);
                        rec.borrow_mut().inserted = true;
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
                        let has_blob = self.engine.program.objects[t.object as usize]
                            .table
                            .as_ref()
                            .is_some_and(|s| {
                                s.fields.iter().any(|f| {
                                    matches!(&f.ty, RtType::Library(al_builtins::library_kind::LibraryKind::Blob))
                                })
                            });
                        // A table with an AutoIncrement field needs the platform's answer at once
                        // (record/TestAutoIncrementContracts.al:AutoIncrement_SecondInsert_...).
                        if !self.ret_used
                            && !has_blob
                            && auto.is_none()
                            && !self.tx.immediate_errors
                            && !rec.borrow().temporary
                        {
                            if (run && !self.ext_triggers(&t, "OnAfterInsert").is_empty())
                                || self.listened(&t, "onafterinsertevent", None)?
                            {
                                return unsupported(format!(
                                    "deferred duplicate-key Insert into {}, which has an After insert event or trigger (whether it runs is not pinned)",
                                    t.name
                                ));
                            }
                            match self.fail(&t, &row, true) {
                                Err(Signal::Error(e)) => {
                                    self.tx.deferred_insert.get_or_insert((t.object as usize, *e));
                                }
                                // A key whose format is not pinned stays a refusal.
                                Err(other) => return Err(other),
                                Ok(_) => {}
                            }
                            return Ok(Value::Boolean(true));
                        }
                        self.fail(&t, &row, true)
                    }
                }
            }
            RecordOp::Modify => {
                let run = arg_bool(args, 0)?;
                self.tenant_license_state_refuses(&t, "modify")?;
                let pk = t.primary_key(&rec.borrow().fields);
                let stored = self
                    .store_for(rec)
                    .get(&cx, &t, &pk)
                    .map_err(|e| Signal::Unsupported(format!("{e}")))?;
                // A buffer older than a committed change to its row cannot be saved
                // (record/TestRecordStaleBufferWrite.al); before a commit the last writer wins.
                if let (Some(row), Some(i)) = (&stored, t.fields.iter().position(|f| f.id == 2_000_000_005)) {
                    let (mine, theirs) = (rec.borrow().fields[i].clone(), row[i].clone());
                    let version = |v: &Value| if let Value::BigInteger(n) = v { *n } else { 0 };
                    // Stale means the stored row is NEWER than the buffer. A buffer that is newer than the row (the
                    // transaction was rolled back to the last Commit after the buffer's own Modify, as an asserterror
                    // does) is not stale (corpus record/lt-record-conversion-assumptions/StaleBuffer.al:
                    // P98_Stale_AfterAsserterror_SameBuffer, P98_Stale_CommitThenErrorThenModify_Rec).
                    let committed = self.tx.committed_row_version;
                    let stale =
                        |m: &Value| version(m) != 0 && version(m) < version(&theirs) && version(&theirs) <= committed;
                    if !rec.borrow().temporary && stale(&mine) {
                        let identification = self.identification(&t, &rec.borrow().fields)?;
                        return Err(Signal::Error(Box::new(AlError {
                            message: format!(
                                "The changes to the {} record cannot be saved because some information on the page is not up-to-date. Close the page, reopen it, and try again.{BR}{BR}Identification fields and values:{BR}{BR}{}",
                                self.caption(&t),
                                identification
                            ),
                            code: "DB:RecordChanged".into(),
                            call_stack: String::new(),
                            ..Default::default()
                        })));
                    }
                    // The buffer still holds what its own last Modify wrote and a rollback undid that write: the Modify answers
                    // true and writes nothing, so the row keeps what the rollback restored (StaleBuffer.al:76-86); a buffer
                    // changed since writes the whole row (:167-168). The triggers, events and database triggers of such a
                    // Modify are not pinned: refused when there are any.
                    let own_image = {
                        let r = rec.borrow();
                        !r.temporary
                            && version(&mine) > version(&theirs)
                            && r.last_modify
                                .as_ref()
                                .is_some_and(|img| version(&img[i]) == version(&mine))
                    };
                    if own_image {
                        let img = rec.borrow().last_modify.clone().expect("checked");
                        if Session::modify_changes_nothing(&t, &img, rec) {
                            let mut trigger = run && self.table_trigger(&t, "OnModify").is_some();
                            for n in ["OnBeforeModify", "OnModify", "OnAfterModify"] {
                                trigger |= run && !self.ext_triggers(&t, n).is_empty();
                            }
                            if trigger
                                || self.listened(&t, "onbeforemodifyevent", None)?
                                || self.listened(&t, "onaftermodifyevent", None)?
                                || self.global_enabled(&t, GlobalOp::Modify)?
                            {
                                return unsupported(format!(
                                    "a Modify of a {} buffer that its own rolled-back write left, with triggers or events: not pinned",
                                    t.name
                                ));
                            }
                            return Ok(Value::Boolean(true));
                        }
                    }
                }
                // Modify never changes the stored SystemId, whatever the buffer carries
                // (corpus record/TestRecordSystemId.al:176-215, :285-330).
                let kept: Vec<(usize, Value)> = match &stored {
                    Some(old) => [2_000_000_000, 2_000_000_001, 2_000_000_002]
                        .iter()
                        .filter_map(|id| t.fields.iter().position(|f| f.id == *id))
                        .map(|i| (i, old[i].clone()))
                        .collect(),
                    None => Vec::new(),
                };
                // One xRec handle for the events, the triggers and the global trigger, so a change a
                // subscriber makes to it reaches the rest. When code drives the Modify, xRec equals
                // Rec (record/TestXRecContracts.al:69-87 OnModify_xRec_MirrorsRecValues_WhenCalledFromCode;
                // record/TestTableEventSequenceContracts.al:ModifyRenameDelete_EventsAndTrigger pins the
                // same for the events). A row a page saves carries the page's own previous row
                // (record/TestXRecDriverContracts.al:32-51 Record_Modify_FromPage_xRecHoldsPreviousValue).
                let stored_row = stored.clone();
                if let Some(old) = &stored {
                    self.check_modify_auto_increment(&t, rec, old, run)?;
                }
                // A Modify loads every field of a partially loaded record: the subscribers' Rec and xRec and the caller's record
                // report all fields loaded afterwards, for Modify(true) and Modify(false)
                // (record/lt-af-record-events/AfModifyLoadFields.al:92, :112, :130). A record nothing was fetched into is not pinned
                // and stays as it is.
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
                // stale write (record/lt-priority-assumptions/PriorityAssumptions.al:143-244, real BC 29: after a Modify that changes nothing and a Commit, the
                // other variable's Modify of a changed field is accepted, while a Modify that changed a field makes it fail with DB:RecordChanged).
                self.check_code_storage(&t, &rec.borrow().fields, true)?;
                self.sql_timestamp_before(&t, rec, stored_row.as_deref())?;
                // A bulk ModifyAll(.., false) without subscribers stamps SystemModifiedAt of a row that has the value already
                // (RcBTriggers.al:ModifyAll_Equal_Value_Rows_BQ) and gives it a new row version, with or without an OnModify
                // trigger (record/lt-modify-without-change/ModifyWithoutChange.al:375-448, `row1_version_changed = Yes`).
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
                let stamped = self.stamp_modify(&t, rec, stored_row.as_deref());
                let mut row = rec.borrow().fields.clone();
                for (i, v) in kept {
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
                // (record/lt-flowfield-write/DiFwWriteTests.al:606-616, `after Modify`, `after Get`, `Get after CalcFields`).
                // An Insert does not (`insert then Get` = 0).
                if rec.borrow().temporary {
                    let engine = self.engine.clone();
                    let schema = engine.program.objects[t.object as usize].table.as_ref().expect("table");
                    for (i, f) in t.fields.iter().enumerate() {
                        if !f.stored && f.id < 2_000_000_000 && !schema.fields[i].flow_filter {
                            view[i] = row[i].clone();
                        }
                    }
                }
                let mut stored = Self::to_store(rec, view);
                // Only the Insert of a value the SQL column cannot hold is pinned (errors/lt-ms-refusals-130/P130DateFilters.al:234-275).
                if self.sql_columns(rec, &t, &mut stored)?.is_some() {
                    return unsupported("a Modify of a value its SQL column cannot hold (not pinned)");
                }
                Self::mark_blobs_inserted(rec, &stored, false);
                match self
                    .store_for(rec)
                    .modify(&cx, &t, stored.clone())
                    .map_err(|e| Signal::Unsupported(format!("{e}")))?
                {
                    WriteOutcome::Done => {
                        Self::clear_blob_written(rec);
                        if !rec.borrow().temporary {
                            let image = rec.borrow().fields.clone();
                            rec.borrow_mut().last_modify = Some(image);
                        }
                        // The Modify that stores a Media field's new media removes the Tenant Media row of the one it held
                        // (record/lt-ar-records/ArRecordProbes.al:141-143). Only a media replaced by another is pinned; a
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
                self.delete_row(&t, rec, run)
            }
            RecordOp::DeleteAll => {
                let run = arg_bool(args, 0)?;
                // The platform takes no delete on Tenant License State (record/TestPlatformTablesWritableContracts.al:96-101).
                self.tenant_license_state_refuses(&t, "delete")?;
                let (filters, flow_field_filters) = self.split_filters(rec);
                if !flow_field_filters.is_empty() {
                    return unsupported("DeleteAll with a filter on a FlowField");
                }
                let temporary = rec.borrow().temporary;
                self.coverage_cleared(&t, temporary, &filters);
                // The platform asks for the table's Global Triggers setup, as the first write to the table, before it looks
                // for rows: a row the setup subscriber inserts is deleted (events/W4GlobalTriggers.al:327, :341), and a table
                // with no row asks too (session/TestEnvironmentUserTenantContracts.al:289-290, `GtTable.DeleteAll()` on an
                // empty table). A temporary record is not pinned and stays as it was.
                if !temporary {
                    self.global_setup_asked(&t)?;
                }
                let n = self
                    .store_for(rec)
                    .count(&cx, &t, &filters)
                    .map_err(|e| Signal::Unsupported(format!("{e}")))?;
                // No row, no trigger and no event.
                if n == 0 {
                    return Ok(Value::Null);
                }
                let events =
                    self.listened(&t, "onbeforedeleteevent", None)? || self.listened(&t, "onafterdeleteevent", None)?;
                let mut trigger = self.table_trigger(&t, "OnDelete").is_some();
                for n in ["OnBeforeDelete", "OnDelete", "OnAfterDelete"] {
                    trigger |= !self.ext_triggers(&t, n).is_empty();
                }
                let global = !temporary && self.global_enabled(&t, GlobalOp::Delete)?;
                // With delete events or database triggers a DeleteAll deletes row by row: the events
                // and the database trigger per row, the table's and extensions' triggers only for
                // DeleteAll(true). Pinned on real BC 27.5 by the probe codeunit 68212
                // (record/TestTableExtensionTriggerOrderContracts.al:102
                // Delete_And_DeleteAll_WithAndWithoutRunTrigger).
                // The User table has no bulk delete: the platform deletes it row by row.
                let user_rows = self.is_user_row(&t, rec.borrow().temporary);
                // MarkedOnly limits the rows to the marked ones, which the store's bulk delete does not know: delete row
                // by row what the scan finds (Post Inventory Cost to G/L ends its data item with
                // `MarkedOnly(true); DeleteAll()` and keeps the capacity entries it has not posted yet).
                let marked_only = rec.borrow().marked_only;
                if marked_only {
                    al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::DeleteAllMarkedOnly);
                }
                if events || global || user_rows || (run && trigger) || marked_only {
                    // One Delete per row (record/TestTriggerContracts.al:DeleteAll_WithTrigger_FiresForEachFilteredRecord).
                    let key = rec.borrow().key;
                    let mut rows = Vec::new();
                    let mut after = None;
                    while let Some(row) = self.scan(rec, &t, true, after.take())? {
                        after = Some(position(&t, key, &row));
                        rows.push(row);
                    }
                    // One fresh set of table globals for all rows, not the caller's
                    // (ADR-021 decision 6; corpus probe TableGlobals_DeleteAll_TriggersRunOnOneInstanceForAllRows).
                    let shared = al_builtins::value::TableGlobals::new_shared();
                    for row in rows {
                        // A row an earlier row's OnDelete deleted is not visited: BC finds the next row of the table as
                        // it is then. Not pinned; Requisition Line's OnDelete deletes the lines that follow it
                        // (Tests-SCM-Manufacturing B42538_UserIDOnReqLine; assumption delete-all-skips-rows-deleted-meanwhile).
                        let pk: Vec<Value> = t.keys[0].iter().map(|&f| row[f].clone()).collect();
                        let still_there = self
                            .store_for(rec)
                            .get(&cx, &t, &pk)
                            .map_err(|e| Signal::Unsupported(format!("{e}")))?
                            .is_some();
                        if !still_there {
                            // record/lt-delete-all-deleted-meanwhile/DeleteAllSkipsDeleted.al:DeleteAllTrue_FirstDeletesAnEarlierSibling_NoneEarlier.
                            continue;
                        }
                        let mut st = rec.borrow().clone();
                        st.fields = row;
                        st.filters.clear();
                        st.globals = shared.share();
                        self.delete_row(&t, &Rc::new(RefCell::new(st)), run)?;
                    }
                    return Ok(Value::Null);
                }
                if self.is_company_table(&t) && !temporary {
                    return unsupported(
                        "DeleteAll on the Company table without delete events (a deleted company's rows go with it; not pinned)",
                    );
                }
                self.store_for(rec)
                    .delete_all(&cx, &t, &filters)
                    .map_err(|e| Signal::Unsupported(format!("{e}")))?;
                self.deletes += 1;
                Ok(Value::Null)
            }
            RecordOp::Get => {
                // `Get(RecordId)`: the row the RecordId names, as RecordRef.Get(RecordId) does (corpus
                // recordref/TestRecordRefKeys.al:48). The Record form is pinned for a row that exists
                // (record/TestSetAutoCalcFieldsOnGet.Codeunit.al:68-79; Base Application's Item
                // Category Management does `ItemCategory.Get(RecId)`); the RecordId of another table is refused.
                let from_id: Option<Vec<Value>> = match args {
                    [Value::Library(_, l)] => match &*l.borrow() {
                        al_builtins::value::LibraryValue::RecordId { table, key, .. } if *table == t.object => {
                            Some(key.clone())
                        }
                        al_builtins::value::LibraryValue::RecordId { .. } => {
                            return unsupported("Get of a RecordId of another table (error text not pinned)");
                        }
                        // An empty RecordId finds no row: false, no error (record/lt-record-conversion-assumptions/
                        // GetRecordId.al: P98_Get_RecordId_Empty). With the result unused the outcome is not pinned.
                        al_builtins::value::LibraryValue::Empty if self.ret_used => return Ok(Value::Boolean(false)),
                        _ => None,
                    },
                    _ => None,
                };
                let args = from_id.as_deref().unwrap_or(args);
                let mut pk = Vec::new();
                for (n, &field) in t.keys[0].iter().enumerate() {
                    let ty = t.fields[field].ty.clone();
                    pk.push(match args.get(n) {
                        // A text for an Integer key reads as Evaluate reads it; a text that is no number raises
                        // (GetWithTextArgument.al:Get_TextForIntegerKey, Get_TextVariableDirect; BC 27 names the type
                        // unknown).
                        Some(Value::Text(s) | Value::Code(s)) if matches!(ty, RtType::Integer) => {
                            match s.trim_matches(' ').parse::<i32>().ok() {
                                Some(n) => Value::Integer(n),
                                None if self.engine.target.as_ref().map_or(29, |t| t.bc.major) >= 28 => {
                                    return Err(crate::transactions::al_error_code(
                                        &format!("The value \"{s}\" can't be evaluated into type Integer."),
                                        "Evaluate:Format",
                                    ));
                                }
                                None => {
                                    return Err(crate::transactions::al_error_code(
                                        "The type NavInteger is unknown.",
                                        "NotSupportedType",
                                    ));
                                }
                            }
                        }
                        // An Integer (literal or variable) for a Code key reads as its digits: found, not found and the
                        // not-found error (record/lt-at-partner-gaps/AtGetIntegerOnCodeKey.al:86-140, Integer 123 on a
                        // Code[20] key). A negative number, one longer than the field and a Text key are not pinned.
                        Some(Value::Integer(i)) if matches!(ty, RtType::Code(_)) => {
                            let digits = i.to_string();
                            let fits =
                                matches!(ty, RtType::Code(len) if len.is_none_or(|l| digits.len() <= l as usize));
                            if *i < 0 || !fits {
                                return unsupported(
                                    "Get with a negative or too long Integer on a Code key (not pinned)",
                                );
                            }
                            Value::Code(digits.into())
                        }
                        // A Time or DateTime key value the SQL column would round: what Get finds is not pinned (sqltime.rs).
                        Some(v @ (Value::Time(_) | Value::DateTime(_)))
                            if !rec.borrow().temporary
                                && !ops::compare(&crate::sqltime::rounded(v), v).is_ok_and(|o| o.is_eq()) =>
                        {
                            return unsupported(
                                "Get with a Time or DateTime key value the SQL column rounds (not pinned)",
                            );
                        }
                        Some(v) => ops::convert(v.clone(), &ty)?,
                        None => self.default_value(&ty),
                    });
                }
                if self.engine.program.objects[t.object as usize].id == Some(2_000_000_164) {
                    if let Some(why) = pk.first().and_then(crate::platform::time_zone_get_refusal) {
                        return unsupported(why);
                    }
                }
                match self
                    .store_for(rec)
                    .get(&cx, &t, &pk)
                    .map_err(|e| Signal::Unsupported(format!("{e}")))?
                {
                    Some(row) => {
                        self.get_load = true;
                        let loaded = self.load(rec, row);
                        self.get_load = false;
                        loaded?;
                        Ok(Value::Boolean(true))
                    }
                    None => {
                        // A row that is there: record/TestSetAutoCalcFieldsOnGet.Codeunit.al:68-79; a row that is not: the
                        // error of Get(key) (record/lt-record-conversion-assumptions/GetRecordId.al: P98_Get_RecordId_Missing_*).
                        let mut ident = rec.borrow().fields.clone();
                        for (n, &field) in t.keys[0].iter().enumerate() {
                            ident[field] = match (args.get(n), &t.fields[field].ty) {
                                // The message names the key as the caller gave it: an Option for an Enum key
                                // reads 'Three', not the ordinal the enum lacks
                                // (corpus types/PEOptionEnumBigText.al:225-259, probed on BC 27.5).
                                (Some(o @ Value::Option(..)), RtType::Enum(_)) => o.clone(),
                                // Whether the message names an enum of another type, or an Integer, by the
                                // caller's value or by the key's is not pinned (docs/probes-needed.md); found
                                // and not found are (types/PXEnumRecordRef.al:123,127,154,156).
                                (Some(Value::Enum(from, _)), RtType::Enum(to)) if from != to && !self.ret_used => {
                                    return Err(Signal::Unsupported(
                                        "the not-found error of a Get keyed by an enum of another type: the identification text is not pinned (docs/probes-needed.md)".into(),
                                    ));
                                }
                                (Some(Value::Integer(_)), RtType::Enum(_)) if !self.ret_used => {
                                    return Err(Signal::Unsupported(
                                        "the not-found error of a Get keyed by an Integer on an Enum field: the identification text is not pinned (docs/probes-needed.md)".into(),
                                    ));
                                }
                                _ => pk[n].clone(),
                            };
                        }
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
                        let pk = t.primary_key(&rec.borrow().fields);
                        let row = self
                            .store_for(rec)
                            .get(&cx, &t, &pk)
                            .map_err(|e| Signal::Unsupported(format!("{e}")))?;
                        match row {
                            Some(row) if self.row_passes(rec, &t, &row)? || Self::on_own_row(rec, &pk) => Some(row),
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
                    // the neighbour search of a row that is gone: pageupdate/TestPageUpdateGoneRow.al:423,
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
                                // hold the record's values and that passes the filters (pages/lt-modal-list-new-row-more/
                                // NewRowListSecondaryKey.al:P197_SecKey_PositionedAfterFindFirst, _PositionedAfterFindLast,
                                // NewRowListTemporary.al:P197_Temp_SecondaryKeyAfterFindFirst: a list over a secondary key opens on
                                // the positioned record). Base Application's Req. Wksh.-Make Order `Code` needs it as well.
                                '=' => {
                                    let pk = t.primary_key(&rec.borrow().fields);
                                    let exact = self
                                        .store_for(rec)
                                        .get(&cx, &t, &pk)
                                        .map_err(|e| Signal::Unsupported(format!("{e}")))?;
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
                        self.load(rec, row)?;
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
                        // record/lt-filters-keys-finds/P68C_EmptyFinds.al:215-250 ( passes on the BC 27 oracle
                        // too), FindSet ErTTriggerErrors.al:140-147 and P68C_EmptyFinds.al:202-204.
                        // Find('-') and Find('+') of an empty, unfiltered table say so as well
                        // (record/lt-find-empty-table/FindEmptyTable.al:Find_EmptyTable `unused_first`, `unused_last`).
                        if filters.is_empty()
                            && (matches!(method, RecordOp::FindFirst | RecordOp::FindLast | RecordOp::FindSet)
                                || (method == RecordOp::Find && matches!(which.as_str(), "-" | "+")))
                        {
                            return Err(Signal::Error(Box::new(AlError {
                                message: format!("The {} table is empty.", self.caption(&t)),
                                code: "DB:TableIsEmpty".into(),
                                call_stack: String::new(),
                                ..Default::default()
                            })));
                        }
                        // The search strings the corpus pins on an empty table: none or `=` (the key alone), `>`, `<` and `=><`
                        // (FindEmptyTable.al:Find_EmptyTable `unused_default` .. `unused_all_directions`). A table with rows
                        // that lacks the key, and the other combinations, are not pinned.
                        if filters.is_empty()
                            && method == RecordOp::Find
                            && matches!(which.as_str(), "=" | ">" | "<" | "=><")
                            && self
                                .store_for(rec)
                                .count(&cx, &t, &[])
                                .map_err(|e| Signal::Unsupported(format!("{e}")))?
                                == 0
                        {
                            let fields = rec.borrow().fields.clone();
                            let pk = t.primary_key(&fields);
                            if !pk.iter().all(|v| crate::recmeta::is_blank(v).unwrap_or(false)) {
                                return unsupported(
                                    "Find on an empty table with a key value other than the default: only default keys are pinned (FindEmptyTable.al)",
                                );
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
                            return Err(Signal::Error(Box::new(AlError {
                                message,
                                code: "DB:RecordNotFound".into(),
                                call_stack: String::new(),
                                ..Default::default()
                            })));
                        }
                        if filters.is_empty()
                            || !matches!(method, RecordOp::FindFirst | RecordOp::FindLast | RecordOp::FindSet)
                        {
                            return unsupported(format!(
                                "{method} on an empty set of table '{}' without using the result and without a filter: BC's error text is not pinned (docs/probes-needed.md)",
                                t.name
                            ));
                        }
                        Err(Signal::Error(Box::new(AlError {
                            message: format!(
                                "There is no {} within the filter.{BR}{BR}Filters: {filters}",
                                self.caption(&t)
                            ),
                            code: "DB:NothingInsideFilter".into(),
                            call_stack: String::new(),
                            ..Default::default()
                        })))
                    }
                }
            }
            RecordOp::Next => {
                let steps = match args.first() {
                    Some(Value::Integer(n)) => *n,
                    // A whole-number Decimal (`Calendar.Next(12 / CountFrequency)`); a fraction would round or
                    // truncate, which is not pinned.
                    Some(Value::Decimal(d)) if d.fract().is_zero() && i32::try_from(d.trunc()).is_ok() => {
                        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::NextWholeDecimal);
                        i32::try_from(d.trunc()).unwrap_or(0)
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
                            self.load(rec, row)?;
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
                        .map_err(|e| Signal::Unsupported(format!("{e}")))?
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
                let mut r = rec.borrow_mut();
                r.position = None;
                let group = r.filter_group;
                r.filters.retain(|f| !(f.field == pos && f.group == group));
                // A FlowField filter in the OR group is dropped: GetFilters does not show it and no row is filtered by it
                // (record/PWFilterGroupOr.al:OrGroup_FlowFieldFilter_BQ, `getFilters`, `flowFieldOnly`, `nameOrFlowField`).
                if group == al_builtins::filter::OR_GROUP && or_flowfield {
                    return Ok(Value::Null);
                }
                if let Some(from) = args.get(1) {
                    let from = ops::convert(from.clone(), &ty)?;
                    let to = match args.get(2) {
                        Some(v) => ops::convert(v.clone(), &ty)?,
                        None => from.clone(),
                    };
                    r.filters.push(FieldFilter {
                        field: pos,
                        expr: FilterExpr::Range(Some(from), Some(to)),
                        group,
                    });
                }
                Ok(Value::Null)
            }
            RecordOp::SetFilter => {
                let pos = arg_pos(args, 0)?;
                rec.borrow_mut().position = None;
                let ty = t.fields[pos].ty.clone();
                let text = al_builtins::native::text_of(args.get(1).unwrap_or(&Value::Null)).unwrap_or_default();
                // An empty filter text removes the field's filter: a Query column does so
                // (query/TestQueryRuntimeContracts.al:208-209, "an empty SetFilter clears the filter"), and the
                // documentation says the same of a Record, also in a filter group other than 0 (types/lt-filter-enum-relation-duration-guid/
                // A_BlankFilter.al: Blank_AfterSetFilterGt_Group0, Blank_Group2_AfterSetFilterGt).
                if text.trim().is_empty() {
                    let mut r = rec.borrow_mut();
                    let group = r.filter_group;
                    r.filters.retain(|f| !(f.field == pos && f.group == group));
                    return Ok(Value::Null);
                }
                let engine = self.engine.clone();
                let enums = |id: u32| crate::views::enum_filter_members(&engine.program.objects[id as usize]);
                let date_ctx = al_builtins::filter::DateFilterContext {
                    today: self.today_day_number(),
                    work_date: self.work_date_day().unwrap_or_else(|| self.today_day_number()),
                    language: self.global_language,
                };
                let parse = |text: &str| {
                    al_builtins::filter::with_date_context(date_ctx, || {
                        al_builtins::filter::parse_filter_with(text, &args[2..], &ty, &enums)
                    })
                };
                let mut parsed = parse(&text);
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
                // text made from the work date), so a DateTime text that raises it is refused.
                if matches!(&parsed, Err(e) if e.starts_with(al_builtins::filter::NOT_EVALUATED)) {
                    let event = match ty {
                        RtType::Integer => Some(("makeintfilter", "IntFilterText")),
                        RtType::Time => Some(("maketimefilter", "TimeFilterText")),
                        RtType::DateTime => Some(("makedatetimefilter", "DateTimeFilterText")),
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
                                if let Some(fixed) =
                                    self.repair_word_filter_text(event, param, &text, &|t| parse(t).is_ok())?
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
                    // record/lt-record-read/RcAFilters.al:SetFilter_Integer_Invalid, events/lt-platform-triggers/PtObserve.al:Bad_Int,
                    // Bad_Decimal, Bad_Time, Bad_DateTime m0.
                    Err(e) if e.starts_with(al_builtins::filter::NOT_EVALUATED) && self.global_language == 1033 => {
                        let (type_name, value) = e[al_builtins::filter::NOT_EVALUATED.len()..]
                            .split_once('\t')
                            .expect("type and value");
                        return Err(Signal::Error(Box::new(AlError {
                            message: format!(
                                "The filter \"{}\" is not valid for the {} field on the {} table. The value \"{value}\" can't be evaluated into type {type_name}.",
                                text.trim(),
                                self.field_caption(&t, pos),
                                self.caption(&t)
                            ),
                            code: "Filter:CSide".into(),
                            call_stack: String::new(),
                            ..Default::default()
                        })));
                    }
                    // The German error of a date no Date filter reads (P143Date_DePlain.al, _DeRanges.al, _DeFormulas.al).
                    Err(e) if e.starts_with(al_builtins::filter::INVALID_DATE) && self.global_language == 1033 => {
                        // P81DateFilters.al:SetFilter_Date_InvalidTexts.
                        let bad = &e[al_builtins::filter::INVALID_DATE.len()..];
                        return Err(Signal::Error(Box::new(AlError {
                            message: format!(
                                "The filter \"{}\" is not valid for the {} field on the {} table. The format of the Date '{bad}' does not match your device's date settings.\\\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format 'yyyy-mm-dd'.",
                                text.trim(),
                                self.field_caption(&t, pos),
                                self.caption(&t)
                            ),
                            code: "Filter:CSide".into(),
                            call_stack: String::new(),
                            ..Default::default()
                        })));
                    }
                    Err(e) if e.starts_with(al_builtins::filter::FILTER_SYNTAX) && self.global_language == 1033 => {
                        let inner = &e[al_builtins::filter::FILTER_SYNTAX.len()..];
                        return Err(Signal::Error(Box::new(AlError {
                            message: format!(
                                "The filter \"{}\" is not valid for the {} field on the {} table. {inner}",
                                text.trim(),
                                self.field_caption(&t, pos),
                                self.caption(&t)
                            ),
                            code: "Filter:CSide".into(),
                            call_stack: String::new(),
                            ..Default::default()
                        })));
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
                    Err(e) if e.starts_with(al_builtins::filter::INVALID_BOOLEAN) => {
                        let word = &e[al_builtins::filter::INVALID_BOOLEAN.len()..];
                        return Err(Signal::Error(Box::new(AlError {
                            message: format!(
                                "The filter \"{}\" is not valid for the {} field on the {} table. The value \"{word}\" can't be evaluated into type Boolean.",
                                text.trim(),
                                self.field_caption(&t, pos),
                                self.caption(&t)
                            ),
                            code: "Filter:CSide".into(),
                            call_stack: String::new(),
                            ..Default::default()
                        })));
                    }
                    // types/lt-in-precedence-date-guid-filters/P143Guid_Filters.al:P143_GuidInvalid, P143Date_FieldRef.al:P143_FieldRefGuid.
                    Err(e) if e.starts_with(al_builtins::filter::INVALID_GUID) => {
                        return Err(Signal::Error(Box::new(AlError {
                            message: format!(
                                "The filter \"{}\" is not valid for the {} field on the {} table. Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits.",
                                text.trim(),
                                self.field_caption(&t, pos),
                                self.caption(&t)
                            ),
                            code: "Filter:CSide".into(),
                            call_stack: String::new(),
                            ..Default::default()
                        })));
                    }
                    Err(e) => return Err(Signal::Unsupported(format!("SetFilter: {e}"))),
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
                // The filter group goes back to 0 (misc/lt-probes-needed-5/PnE08RecordMisc.al:RecordRef_FilterGroup `afterReset`).
                r.filter_group = 0;
                // Reset drops the marks too (record/lt-record-read/RcAMisc.al:ClearMarks_And_Reset `markAfterReset`).
                r.marks.clear();
                r.marked_only = false;
                r.key = 0;
                r.adhoc = None;
                r.ascending = true;
                r.field_descending = 0;
                // Reset sets ReadIsolation back to Default (misc/lt-cv-odd-leads/CvReadIsolationState.al:43-44, :58).
                r.read_isolation = 0;
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
                // declares that key: the view and the SetAscending error name only them (record/lt-setascending-ties/
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
                        // (record/lt-filters-keys-finds/P68D_Ascending.al:Reset_And_SetCurrentKey_Reset_Direction).
                        if r.key != k || r.adhoc.is_some() {
                            r.field_descending = 0;
                        }
                        r.key = k;
                        r.adhoc = None;
                        Ok(Value::Boolean(true))
                    }
                    // Fields no key starts with are allowed and order the rows by themselves, then
                    // by the primary key (record/TestRecordNavContracts.al:180-195,
                    // xmlport/TestXmlPortTriggersReplaceViewContracts.al:339-375).
                    None => {
                        let schema = self.engine.program.objects[t.object as usize]
                            .table
                            .as_ref()
                            .expect("table");
                        // A FlowFilter field and a Blob cannot order rows: `Sorting cannot be done based on the
                        // <field> field(s) in the <table> table.` (code DB:Ok; records/W4SortKeys.al:SetCurrentKey_NotStoredFields
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
                        // A FlowField whose value depends on the row (a `field()` condition) sorts the rows by that value and is
                        // reported as the key (record/lt-set-current-key/SetCurrentKeyFlowFieldOrder.al:109-113), against the
                        // FlowField without conditions above (W4SortKeys.al:471). The deciding rule is not pinned: refused.
                        if let Some(&p) = wanted.iter().find(|&&p| {
                            let f = &schema.fields[p];
                            f.calc_refusal.is_some()
                                || f.calc.as_ref().is_some_and(|c| {
                                    c.conds.iter().any(|k| {
                                        matches!(
                                            k.value,
                                            al_ir::CalcValue::Field(_)
                                                | al_ir::CalcValue::UpperLimit(_)
                                                | al_ir::CalcValue::FieldFilter(_)
                                        )
                                    })
                                })
                        }) {
                            return unsupported(format!(
                                "SetCurrentKey on the FlowField {}, whose value depends on the row: whether it orders the rows is not pinned (probe wanted)",
                                t.fields[p].name
                            ));
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
                let pos = arg_pos(args, 0)?;
                let engine = self.engine.clone();
                let schema = engine.program.objects[t.object as usize].table.as_ref().expect("table");
                let field = &schema.fields[pos];
                // A FlowField (record/lt-flowfield-write/DiFwWriteTests.al): flowfields.rs `validate_flowfield` raises or refuses
                // what is not a Decimal Sum; for that one the value goes in, OnValidate runs, then `write_sum_difference`.
                let sum_flowfield = !field.stored && !field.flow_filter && field.id < 2_000_000_000;
                if sum_flowfield {
                    let value = args.get(1).cloned();
                    self.validate_flowfield(rec, &t, pos, value.as_ref())?;
                }
                // A Validate nested in an OnValidate of the same record keeps the outer call's
                // before-image as xRec (corpus record/TestXRecContracts.al:133).
                // `handle` is the address of `rec`, which this call borrows for as long as the entry exists, and the
                // entry is popped below on every way out (the result is held, not `?`-ed, until then), so no other
                // record can get the address while it is on the stack (tests/store/xrec.rs).
                let handle = Rc::as_ptr(rec) as usize;
                let outer = self
                    .validating
                    .iter()
                    .find(|(h, _)| *h == handle)
                    .map(|(_, b)| b.clone());
                let before = outer.clone().unwrap_or_else(|| rec.borrow().fields.clone());
                let old_value = rec.borrow().fields[pos].clone();
                let snapshot = rec.borrow().fields.clone();
                let new_value = match args.get(1) {
                    // Validate of an Option field with a BigInteger beyond Integer raises Overflow, where an assignment raises
                    // ConversionCast (types/lt-implicit-conversions/P79BigOption.al:284-302 Field_ValidateBigToOption). An Enum
                    // field takes a BigInteger only through FieldRef.Validate (a Variant parameter), and raises the same
                    // (types/lt-bigint-enum-option/BigIntToEnumOption.al:449-463 QO_Variant_ToEnumFieldRef_Validate).
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
                if outer.is_none() {
                    self.validating.push((handle, before));
                }
                let r = self.validate_hooks(&t, rec, &xrec, field, snapshot);
                if outer.is_none() {
                    self.validating.pop();
                }
                r?;
                if sum_flowfield {
                    let target = ops::convert(args.get(1).cloned().unwrap_or(old_value), &field.ty)?;
                    if ops::compare(&rec.borrow().fields[pos], &target).ok() != Some(std::cmp::Ordering::Equal) {
                        return unsupported(
                            "an OnValidate that changes the value of the Sum FlowField it validates (not pinned)",
                        );
                    }
                    self.write_sum_difference(rec, &t, pos, target)?;
                }
                Ok(Value::Null)
            }
            RecordOp::Rename => {
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
                // The Blob fields another app's table extension adds to this table (a database record only).
                let other_app_blobs: Vec<usize> = if rec.borrow().temporary {
                    Vec::new()
                } else {
                    let program = &self.engine.program;
                    let table_app = program.objects[t.object as usize].app;
                    program.objects[t.object as usize]
                        .table
                        .as_ref()
                        .map(|schema| {
                            schema
                                .fields
                                .iter()
                                .enumerate()
                                .filter(|(_, f)| {
                                    matches!(f.ty, RtType::Library(al_builtins::library_kind::LibraryKind::Blob))
                                        && f.declared_in
                                            .is_some_and(|o| program.objects[o as usize].app != table_app)
                                })
                                .map(|(i, _)| i)
                                .collect()
                        })
                        .unwrap_or_default()
                };
                // Pinned only for content saved by the Insert (tableextension/lt-bw-cross-app/BwCrossAppTests.al:89-115).
                if other_app_blobs
                    .iter()
                    .any(|&i| matches!(&old[i], Value::Blob(b) if b.borrow().written))
                {
                    return unsupported(
                        "Rename of a record with a write to a Blob field of another app's table extension after its last Insert or Modify: what the record and the row hold then is not pinned",
                    );
                }
                // More values than the primary key has fields (recordref/lt-refs/RfTWrite.al:Rename_Variants `rename_too_many`).
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
                self.check_rename_auto_increment(&t, &old, &new)?;
                if self.is_profile_row(&t, rec.borrow().temporary) {
                    self.profile_before_rename(&t, &old, &new)?;
                }
                let old_pk = t.primary_key(&old);
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
                self.global_trigger(&t, GlobalOp::Rename, rec, Some(&xrec))?;
                let _ = self.bump_row_version(&t, rec);
                let _ = self.stamp_rename(&t, rec);
                let row = rec.borrow().fields.clone();
                if let Some(unique_key) = self.unique_violation(rec, &t, &row, Some(&old_pk))? {
                    rec.borrow_mut().fields = old.clone();
                    return self.fail_unique(&t, unique_key, &row);
                }
                let mut stored = Self::to_store(rec, self.stored_view(&t, &row));
                if self.sql_columns(rec, &t, &mut stored)?.is_some() {
                    return unsupported("a Rename with a value its SQL column cannot hold (not pinned)");
                }
                // A rename to the key the row has succeeds and writes the row again, which is what
                // stamps it (record/TestRecordSystemId.al:752-775).
                let same_key = ops::compare_keys(&old_pk, &t.primary_key(&row));
                let outcome = if same_key {
                    match self
                        .store_for(rec)
                        .modify(&cx, &t, stored.clone())
                        .map_err(|e| Signal::Unsupported(format!("{e}")))?
                    {
                        WriteOutcome::Done => RenameOutcome::Renamed,
                        WriteOutcome::NotFound => RenameOutcome::NotFound,
                    }
                } else {
                    self.store_for(rec)
                        .rename(&cx, &t, &old_pk, stored.clone())
                        .map_err(|e| Signal::Unsupported(format!("{e}")))?
                };
                match outcome {
                    RenameOutcome::Renamed => {
                        // The record itself shows no Blob value after a Rename of a temporary record; the row keeps one only when a write came
                        // after the last Insert or Modify (misc/lt-probes-needed-1/PnABlobAssignRename.al:RenameTemporary_* `A..E-rec-content`).
                        // So does a database record for a Blob field another app's table extension adds, while the renamed row
                        // keeps the bytes (tableextension/lt-bw-cross-app/BwCrossAppTests.al:89-115: `blob in variable after
                        // rename = <none>`, `NEW blob = blob of OLD`). A Blob field of the table's own app keeps what it had,
                        // as before (not pinned for a database record).
                        for &i in &other_app_blobs {
                            let empty = self.default_value(&t.fields[i].ty);
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
                        }
                        if !same_key {
                            self.propagate_rename(rec, &t, &refs, &old_fields, &row)?;
                            self.move_record_links(&t, rec, &old_fields)?;
                        }
                        self.run_ext_trigger(&t, "OnAfterRename", rec, Some(&xrec))?;
                        self.table_event(&t, "onafterrenameevent", rec, Some(&xrec), true, None)?;
                        Ok(Value::Boolean(true))
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
            RecordOp::Truncate => self.truncate(&t, rec, arg_bool(args, 0)?),
            RecordOp::CalcFields | RecordOp::CalcSums | RecordOp::SetAutoCalcFields => self.flow_op(method, rec, args),
            RecordOp::LoadFields => self.load_fields_row(rec, &t, args),
            RecordOp::SetLoadFields
            | RecordOp::AddLoadFields
            | RecordOp::AreFieldsLoaded
            | RecordOp::SetBaseLoadFields => self.load_fields_op(method, rec, &t, args),
            other => self.record_meta_op(other, rec, &t, args),
        }
    }
}

#[allow(dead_code)]
fn _uses(_: RtType, a: &[Value], b: &[Value]) -> std::cmp::Ordering {
    key_cmp(a, b)
}

/// The error BC raises when it evaluates the `InitValue` of `f`, for the two forms the corpus pins as
/// unevaluable (record/TestDateTimeInitValue.Codeunit.al): `0DT` on a DateTime, and a closing-date
/// literal (`C20260101D`) on a Date. Every other form is `None`.
pub(crate) fn unevaluable_init_value(f: &al_ir::Field) -> Option<String> {
    use al_builtins::value::RtType;
    let text = f.init_text.as_deref()?.trim();
    if f.init.is_some() {
        return None;
    }
    match &f.ty {
        RtType::DateTime if text.eq_ignore_ascii_case("0DT") => {
            Some("The value \"01/01/0001 00:00:00\" can't be evaluated into type DateTime.".into())
        }
        RtType::Date => {
            let inner = text.strip_prefix(['C', 'c'])?.strip_suffix(['D', 'd'])?;
            (inner.len() == 8 && inner.chars().all(|c| c.is_ascii_digit()))
                .then(|| format!("The format of the Date '{text}' does not match your device's date settings."))
        }
        _ => None,
    }
}

impl Session {
    /// Whether `row` holds the values of `pos` in the fields of key `key` (and the primary key after them).
    fn same_key_values(t: &al_data::TableDef, key: usize, row: &[Value], pos: &[Value]) -> bool {
        let theirs = position(t, key, row);
        theirs.len() == pos.len()
            && theirs
                .iter()
                .zip(pos.iter())
                .all(|(a, b)| al_builtins::ops::compare(a, b).is_ok_and(|o| o == std::cmp::Ordering::Equal))
    }
}

/// Field `i` of `t` is a stored Media field.
fn is_media_field(t: &TableDef, i: usize) -> bool {
    t.fields[i].stored
        && matches!(
            t.fields[i].ty,
            RtType::Library(al_builtins::library_kind::LibraryKind::Media)
        )
}
