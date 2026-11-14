//! Record operations on the store (ADR-003) for the `Record.*` builtins.
//!
//! Pinned by the corpus or the docs as noted; behaviour neither pins yet
//! raises `Signal::Unsupported` instead of guessing.

use crate::Session;
use crate::globaltriggers::GlobalOp;
use al_builtins::date_filter_forms::datetime_rejected as dt_rejected;
use al_builtins::describe::Describe;
use al_builtins::filter::{FieldFilter, FilterExpr, key_cmp};
use al_builtins::native::{AlError, Signal, unsupported};
use al_builtins::ops;
use al_builtins::record_op::RecordOp;
use al_builtins::value::{Instance, RecordState, RtType, Value};
use al_data::{InsertOutcome, RenameOutcome, Scan, Store, TableDef, WriteOutcome, memory::MemStore, position};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

mod blob_filter;
mod boolean_filter_error;
mod delete_all;
mod delete_all_flow;
mod empty_find_origin;
mod filter_error;
mod flowfield_key;
mod insert_buffer;
mod setfilter_empty_or_side;
mod setrange_filter;
pub(crate) use insert_buffer::{InsertKind, InsertQueue};
mod nested_validate;
pub(crate) use nested_validate::ValidateSite;
mod op_inner;
mod own_rolled_back_modify;
mod platform_rows;
mod stale;
mod stale_rule;
mod tenant_media_rows;
pub(crate) use stale_rule::{TrappedRollback, same_key};
mod guarded_run_body;
pub(crate) use guarded_run_body::GuardedBody;
mod locking_read_cells;
pub(crate) use locking_read_cells::{LockingReadNote, StaleWrite};
mod write_notes;

pub(crate) type Rec = Rc<RefCell<RecordState>>;

/// The `TxState::lock_unpinned` text of a read other than FindFirst under a locking ReadIsolation.
pub(crate) const UNPINNED_LOCKING_READ: &str = "a FindFirst, FindSet, FindLast or Find('-'/'+'/'>'/'<') under a ReadIsolation RepeatableRead or UpdLock set in an earlier transaction";
/// The `TxState::lock_unpinned` text of a ReadIsolation RepeatableRead assigned again to a record that already had it, with
/// no write transaction open.
pub(crate) const REPEATABLE_READ_ASSIGNED_AGAIN: &str =
    "a ReadIsolation RepeatableRead assigned again to a record that had it";

/// BC's line break inside error texts: a carriage return and a backslash.
pub(crate) const BR: &str = "\\";

/// The `RecordState::unpinned_buffer` reason of Rec while the OnOpenPage of a page an action opens through `RunPageLink` runs
/// (set in pages/runobject.rs, its methods in [`Session::run_page_link_open_op`]).
pub(crate) const RUN_PAGE_LINK_OPEN: &str = "in the OnOpenPage of a page an action opens through RunPageLink: real BC fills the linked fields of Rec and GetFilters() reads '' there, the runtime holds the link as a filter (corpus issue 712)";

/// The `RecordState::unpinned_buffer` reason of Rec of a part page reached through a page the test client closed (set in
/// pages/held_page_part.rs): its fields read and `Count()` counts (pages/parts/HeldSenderPartAccess.al:286-323 HS01, HS02
/// `read=id=1 name=one count=1`); no other method of it is measured there, so they refuse. A field write runs and marks the
/// assumption `held-page-part-after-testpage-close` ([`held_part_field_write`]).
pub(crate) const HELD_PART_REC: &str =
    "on Rec of a part of a page the test client closed: only its fields and Count() are measured there";

/// The `RecordState::unpinned_buffer` reason of xRec of such a part: HeldSenderPartAccess.al does not read it, so a read, a
/// write and every method of it refuse.
pub(crate) const HELD_PART_XREC: &str = "xRec of a part of a page the test client closed is not measured there";

/// A field write on a record whose buffer is `why`: on Rec of a held part it runs and marks the assumption (not measured); on
/// its xRec it refuses. Every other buffer writes as before.
pub(crate) fn held_part_field_write(why: &str) -> Result<(), Signal> {
    if why == HELD_PART_REC {
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::HeldPagePartAfterTestPageClose);
    } else if why == HELD_PART_XREC {
        return Err(Signal::unsupported(HELD_PART_XREC));
    }
    Ok(())
}

/// The `RecordState::unpinned_buffer` reason of a table element record after an export under `SourceTableView` (set in
/// xmlports.rs `xp_export_table`). The export order is pinned (xmlports/shapes-properties/RaXmlPortExport.al:416, :448, :474,
/// metadata/platform-tables/AgXmlPortImport.al:312); the key, order, filters and position the record has afterwards are not,
/// so every read of them refuses until `Reset` or `SetView` gives the record a view of its own ([`XP_BUFFER_AFTER_EXPORT`]
/// then). Its methods in [`Session::xp_view_after_export_op`].
pub(crate) const XP_VIEW_AFTER_EXPORT: &str = "the table element record's view after an export under SourceTableView is not pinned (its key, order, filters and position)";

/// The `RecordState::unpinned_buffer` reason of that record after `Reset` or `SetView`: the buffer still holds the row the export
/// left there, which is not pinned either (generic unpinned buffer rules, [`Session::unpinned_buffer_op`]).
pub(crate) const XP_BUFFER_AFTER_EXPORT: &str = "the table element record's buffer after an export under SourceTableView is not pinned (the row the export left there)";

/// The `RecordState::unpinned_buffer` reason of a table element record with MaxOccurs = Once after an export that found three
/// or more rows with export triggers: only the first row is written (QrXpContainerOccurs.al TS_Again_TrMember on branch
/// al-runtime-rs-staging-pub-ts), the row the record holds afterwards is not pinned (generic rules,
/// [`Session::unpinned_buffer_op`]).
pub(crate) const XP_ROW_AFTER_ONCE: &str = "the row a MaxOccurs = Once table element record holds after an export of three or more rows with export triggers, or below another table element without LinkTable, is not pinned";

/// The `RecordState::unpinned_buffer` reason, while an export runs, of a table element record with LinkTable, no LinkFields and
/// MaxOccurs = Once below another table element after a pass that found two or more rows: only the first row is written
/// (QrXpContainerOccurs.al TT_Confirm_Export_NoPreFilter on corpus writer TT's branch al-runtime-rs-staging-tt), the row the record
/// holds afterwards is not pinned. Its view is the export's own again at the next pass, so the view reads of [`XP_BUFFER_AFTER_EXPORT`]
/// run; after the export it becomes [`XP_ROW_AFTER_ONCE`].
pub(crate) const XP_NESTED_ROW_AFTER_ONCE: &str =
    "the row a linked MaxOccurs = Once table element record holds after a pass of two or more rows is not pinned";

/// Whether `rec` carries one of the marks of a table element record after an export under `SourceTableView`.
pub(crate) fn xp_export_marked(rec: &Rec) -> Option<&'static str> {
    rec.borrow()
        .unpinned_buffer
        .filter(|w| *w == XP_VIEW_AFTER_EXPORT || *w == XP_BUFFER_AFTER_EXPORT)
}

/// [`RUN_PAGE_LINK_OPEN`] in the shape the corpus pins (see `run_page_link_fill`, pages/runobject.rs): the linked fields are
/// filled and the fields of Rec read; the other uses are as narrow as with [`RUN_PAGE_LINK_OPEN`].
pub(crate) const RUN_PAGE_LINK_OPEN_FILLED: &str = "in the OnOpenPage of a page an action opens through RunPageLink: real BC hands a by-value copy of Rec no filters (pinned), what Rec itself answers is not pinned";

/// The `RecordState::unpinned_buffer` reason of a by-value copy of Rec of a page an action opened through `RunPageLink`
/// (`RecordState::run_page_link`): the copy holds Rec's fields and no link filter; only its fields and `GetFilters` are pinned
/// (testpage/TestActionRunPageLinkFilterGroup.al:292-369, `Describe(Child: Record ...)` in QzRunPageLinkOnOpen.al).
pub(crate) const RUN_PAGE_LINK_COPY: &str = "a by-value copy of Rec of a page an action opened through RunPageLink: real BC hands it Rec's fields and no link filter, whether it reads rows without the link is not pinned";

/// [`RUN_PAGE_LINK_COPY`] for a copy taken in the OnOpenPage of the page: it holds Rec's fields and no link filter
/// (TestActionRunPageLinkFilterGroup.al:292-369), but whether it reads rows without the link was probed only after the page opened
/// (R7138RunPageLinkCopyGet.al, OnAfterGetCurrRecord), so its `Count` and `Get` stay refused.
pub(crate) const RUN_PAGE_LINK_OPEN_COPY: &str = "a by-value copy of Rec in the OnOpenPage of a page an action opened through RunPageLink: real BC hands it Rec's fields and no link filter, whether it reads rows without the link is not pinned";

/// The `RecordState::unpinned_buffer` reason of Rec of a page an action opened through `RunPageLink` handed to a Variant parameter:
/// the Variant holds Rec with the link filter. Only `RecordRef.GetTable` of it is pinned (the RecordRef carries the link filter and
/// answers as any RecordRef, corpus testpage/R7138RunPageLinkCopyGet.al:117-134 `rr[Project Code: P2] n=2`); every record
/// method and field read on it refuses.
pub(crate) const RUN_PAGE_LINK_VARIANT: &str =
    "Rec of a page an action opened through RunPageLink handed to a Variant: only RecordRef.GetTable of it is pinned";

/// The `RecordState::unpinned_buffer` reason of a record after `SetCurrentKey` on Media or MediaSet fields that no key holds, and
/// of a by-value copy of it. Only `CurrentKey` is pinned: the record answers the fields (streams/ArRecordProbes.al:198-229
/// SetCurrentKey_Media `Pic`, SetCurrentKey_MediaSet `Pics`), a by-value copy answers the primary key (record/W4SortKeys.al:440-471
/// SetCurrentKey_NotStoredFields `media` `ok:[Id]`, read in `Outcome(Ok, R: Record)`). Its methods in [`Session::media_key_op`].
pub(crate) const MEDIA_KEY: &str = "after SetCurrentKey on a Media or MediaSet field only CurrentKey is pinned (the record answers the field, a by-value copy the primary key): the order of the rows and the rest of the view are not";

/// The `RecordState::unpinned_buffer` reason of a record after `SetCurrentKey` on a FlowField whose value depends on the row (a
/// `field()` condition), alone or after normal fields. Only `CurrentKey` is pinned: it names the fields
/// (record/Y8SetCurrentKeyNotStored.al:84-106 `flowfield` `key=Total`, `name_then_flowfield` `key=Name,Total`;
/// record/keys-sorting/SetCurrentKeyFlowFieldOrder.al:109-113 `currentKey` `Cnt`). The finds and the order of the rows
/// run for a sum or count FlowField first in the key (`flowfield_key`); the rest refuses, as for [`MEDIA_KEY`].
pub(crate) const FLOWFIELD_KEY: &str = "after SetCurrentKey on a FlowField whose value depends on the row only CurrentKey is pinned: the order of the rows by the FlowField's value is not built";

/// Whether `rec` carries [`MEDIA_KEY`] or [`FLOWFIELD_KEY`].
pub(crate) fn media_key_marked(rec: &Rec) -> bool {
    matches!(rec.borrow().unpinned_buffer, Some(w) if w == MEDIA_KEY || w == FLOWFIELD_KEY)
}

/// Whether field reads of a record whose buffer has the reason `why` run.
pub(crate) fn run_page_link_fields_read(why: &str) -> bool {
    why == RUN_PAGE_LINK_OPEN_FILLED
        || why == RUN_PAGE_LINK_COPY
        || why == RUN_PAGE_LINK_OPEN_COPY
        || why == MEDIA_KEY
        || why == FLOWFIELD_KEY
        || why == XP_VIEW_AFTER_EXPORT
}

/// Whether two filter lists are the same filters, in any order.
pub(crate) fn same_filters(a: &[FieldFilter], b: &[FieldFilter]) -> bool {
    a.len() == b.len()
        && a.iter().all(|f| {
            b.iter()
                .any(|g| f.group == g.group && f.field == g.field && same_expr(&f.expr, &g.expr))
        })
        && b.iter().all(|g| {
            a.iter()
                .any(|f| f.group == g.group && f.field == g.field && same_expr(&f.expr, &g.expr))
        })
}

fn same_expr(a: &FilterExpr, b: &FilterExpr) -> bool {
    let v = |x: &Value, y: &Value| ops::compare(x, y).is_ok_and(|o| o.is_eq());
    let ov = |x: &Option<Value>, y: &Option<Value>| match (x, y) {
        (None, None) => true,
        (Some(x), Some(y)) => v(x, y),
        _ => false,
    };
    match (a, b) {
        (FilterExpr::Range(a1, a2), FilterExpr::Range(b1, b2)) => ov(a1, b1) && ov(a2, b2),
        (FilterExpr::Eq(x), FilterExpr::Eq(y))
        | (FilterExpr::RangeSame(x), FilterExpr::RangeSame(y))
        | (FilterExpr::Ne(x), FilterExpr::Ne(y))
        | (FilterExpr::Lt(x), FilterExpr::Lt(y))
        | (FilterExpr::Le(x), FilterExpr::Le(y))
        | (FilterExpr::Gt(x), FilterExpr::Gt(y))
        | (FilterExpr::Ge(x), FilterExpr::Ge(y)) => v(x, y),
        (FilterExpr::Or(x), FilterExpr::Or(y)) | (FilterExpr::And(x), FilterExpr::And(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(x, y)| same_expr(x, y))
        }
        (FilterExpr::Like(x, i), FilterExpr::Like(y, j)) | (FilterExpr::NotLike(x, i), FilterExpr::NotLike(y, j)) => {
            x == y && i == j
        }
        _ => false,
    }
}

fn arg_bool(args: &[Value], i: usize) -> Result<bool, Signal> {
    match args.get(i) {
        None => Ok(false),
        Some(v) => ops::truthy(v),
    }
}

pub(crate) fn arg_pos(args: &[Value], i: usize) -> Result<usize, Signal> {
    match args.get(i) {
        Some(Value::Integer(p)) => Ok(*p as usize),
        other => unsupported(format!("field argument {}", other.describe())),
    }
}

/// The error an Insert into the platform table Device raises (al-language-onprem/metadata/PtOnPremTables.al:184).
pub(crate) const DEVICE_LICENSE_ERROR: &str = "The new devices do not meet the terms of the current license. The current \
     license allows 0 full devices, 0 limited devices, 0 ISV devices, 0 ISV functional devices and 1 devices.";

/// What the `Rec` and `xRec` slots of a table instance held before a call filled them.
#[derive(Default)]
pub(crate) struct RecSlots {
    rec: Option<(usize, Value)>,
    xrec: Option<(usize, Value)>,
}

impl Session {
    /// The platform takes Insert on Tenant License State and refuses Modify and Delete (al-language-onprem/session-system/PZPlatformSurvey.al:
    /// PlatformTable_TenantLicenseState: `It is not possible to modify any record in the Tenant License State table.`, code `DB:Ok`).
    /// Modify, Delete and DeleteAll of Document Service with triggers: what the platform's triggers do is not pinned
    /// (al-language-onprem/metadata/PtOnPremTables.al:103-123 pins them without triggers only).
    fn document_service_trigger_refused(&self, t: &TableDef, rec: &Rec, run: bool, verb: &str) -> Result<(), Signal> {
        if run && !rec.borrow().temporary && self.engine.program.objects[t.object as usize].id == Some(2_000_000_114) {
            return unsupported(format!(
                "{verb}(true) of Document Service: what the platform's trigger does is not pinned"
            ));
        }
        Ok(())
    }

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

    /// The error of a write `method` that a read-only platform table raises (`read_only_tables::raises`).
    fn read_only_platform_table(&self, t: &TableDef, rec: &Rec, method: RecordOp) -> Option<AlError> {
        let id = self.engine.program.objects[t.object as usize].id?;
        if rec.borrow().temporary || !crate::read_only_tables::raises(id, method) {
            return None;
        }
        Some(AlError {
            message: format!("The {} table is read-only.", t.name),
            code: "DB:Ok".into(),
            call_stack: String::new(),
            ..Default::default()
        })
    }

    /// A Modify of a Feature Key row that changes one read-only column (every column but Enabled) raises the error naming the
    /// column's caption, code `DB:Ok` (record/load-fields/P81FeatureKeyModify.al:51-63;
    /// record/TestFeatureKeyTable.al:64-78). A change to two of them, or to one together with Enabled, is not pinned: the store
    /// refuses it. The error comes before the table events: P81FeatureKeyModify.al passes on real BC with Microsoft's
    /// subscribers to the Feature Key events loaded.
    fn feature_key_read_only_change(&self, t: &TableDef, rec: &Rec, stored: Option<&[Value]>) -> Result<(), Signal> {
        let Some(old) = stored else { return Ok(()) };
        let id = self.engine.program.objects[t.object as usize].id.unwrap_or(0);
        if rec.borrow().temporary
            || al_builtins::platform::virtual_table(id) != Some(al_builtins::platform::Virtual::FeatureKey)
        {
            return Ok(());
        }
        let changed: Vec<usize> = {
            let r = rec.borrow();
            (0..t.fields.len())
                .filter(|&i| t.fields[i].id < 2_000_000_000)
                .filter(|&i| match (&old[i], &r.fields[i]) {
                    (Value::Unknown(_), Value::Unknown(_)) => false,
                    (a, b) => !ops::compare(a, b).is_ok_and(|o| o == std::cmp::Ordering::Equal),
                })
                .collect()
        };
        match changed.as_slice() {
            [i] if t.fields[*i].id != 2 => Err(Signal::Error(Box::new(AlError {
                message: format!(
                    "Sorry, but the task couldn't be completed because it tried to change the \"{}\" field of the feature key, which cannot be changed. Contact your administrator or application developer.",
                    self.field_caption(t, *i)
                ),
                code: "DB:Ok".into(),
                call_stack: String::new(),
                ..Default::default()
            }))),
            _ => Ok(()),
        }
    }

    /// A write of the platform table Device (2000000130): an Insert with triggers raises the license error of a fresh
    /// tenant and stores nothing (al-language-onprem/metadata/PtOnPremTables.al:172-190, Device_InsertModifyDelete: the
    /// text at :184; its code is not pinned). A guarded Codeunit.Run does not trap it (:177, `transactions.rs`); whether a
    /// TryFunction does is not pinned. Every other write, and an Insert that event subscribers would see, is not pinned.
    /// Answers the signal the write ends with.
    fn device_write(&mut self, t: &TableDef, method: RecordOp, args: &[Value]) -> Result<Signal, Signal> {
        let listened =
            self.listened(t, "onbeforeinsertevent", None)? || self.listened(t, "onafterinsertevent", None)?;
        if method != RecordOp::Insert || !arg_bool(args, 0)? || listened || self.tx.try_depth > 0 {
            return Ok(Signal::unsupported(format!(
                "{method} on the platform table Device (only an Insert with triggers, outside a TryFunction and with no \
                 event subscribers, is pinned: al-language-onprem/metadata/PtOnPremTables.al:172-190)"
            )));
        }
        Ok(Signal::Error(Box::new(AlError {
            message: DEVICE_LICENSE_ERROR.into(),
            code: al_builtins::native::UNPINNED_ERROR.into(),
            call_stack: String::new(),
            ..Default::default()
        })))
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
        // The values are the same for every record of the table, so they are worked out once per session and copied
        // (a text shares its buffer, copy on write). A value that is a shared object (a Blob's buffer) is built fresh
        // for each record in its slot of the copy (issue 6133: a local record variable of a table with a Blob field
        // built every field anew on each call).
        if let Some(template) = self.initial_cache.get(&table) {
            let mut fields = template.values.clone();
            if !template.fresh.is_empty() {
                let fresh = Rc::clone(&template.fresh);
                let engine = self.engine.clone();
                if let Some(t) = engine.program.objects[table].table.as_ref() {
                    for &i in fresh.iter() {
                        fields[i] = self.initial_field(&t.fields[i]);
                    }
                }
            }
            self.clear_uninstalled_init_values(table, &mut fields);
            return fields;
        }
        let engine = self.engine.clone();
        let Some(t) = engine.program.objects[table].table.as_ref() else {
            return Vec::new();
        };
        let fields: Vec<Value> = t.fields.iter().map(|f| self.initial_field(f)).collect();
        let fresh: Rc<[usize]> = fields
            .iter()
            .enumerate()
            .filter(|(_, v)| {
                !matches!(
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
            })
            .map(|(i, _)| i)
            .collect();
        // The cache keeps no shared object of this record: its slots are built fresh on every copy, and a Blob buffer the
        // cache held would count as one more holder of this record's buffer (`temp_blob_share.rs`).
        let mut values = fields.clone();
        for &i in fresh.iter() {
            values[i] = Value::Null;
        }
        self.initial_cache.insert(table, InitialFields { values, fresh });
        let mut fields = fields;
        self.clear_uninstalled_init_values(table, &mut fields);
        fields
    }

    /// Whether object `o` belongs to an app that is not installed yet while an app installs (`Session::set_apps_not_installed`).
    pub(crate) fn object_app_not_installed(&self, o: usize) -> bool {
        let app = self.engine.program.objects[o].app;
        self.events
            .not_installed_apps
            .as_ref()
            .is_some_and(|a| a.contains(&app))
    }

    /// Whether object `o` belongs to an app with no known install order to the app that installs
    /// (`Session::set_apps_install_order_unknown`).
    pub(crate) fn object_app_order_unknown(&self, o: usize) -> bool {
        let app = self.engine.program.objects[o].app;
        self.events
            .order_unknown_apps
            .as_ref()
            .is_some_and(|a| a.contains(&app))
    }

    /// The InitValue of a table extension field of an app not installed yet does not apply: the field holds its type's
    /// default (install-upgrade/install-order/Y8InstallOrderTests.al:88-110, row 1 that XApp A's install writes has an empty
    /// "Y8 XB Mark", XApp B's InitValue 'B init' applies to B's own install row and at test time).
    fn clear_uninstalled_init_values(&mut self, table: usize, fields: &mut [Value]) {
        if self.events.not_installed_apps.as_ref().is_none_or(|a| a.is_empty()) {
            return;
        }
        let engine = self.engine.clone();
        let Some(t) = engine.program.objects[table].table.as_ref() else {
            return;
        };
        for (i, f) in t.fields.iter().enumerate() {
            if f.init.is_some()
                && let Some(d) = f.declared_in
                && d as usize != table
                && self.object_app_not_installed(d as usize)
                && let Some(slot) = fields.get_mut(i)
            {
                *slot = self.default_value(&f.ty);
            }
        }
    }

    /// The value of field `f` in a fresh record.
    fn initial_field(&mut self, f: &al_ir::Field) -> Value {
        match &f.init {
            Some(c) => ops::convert(crate::constant(c), &f.ty).unwrap_or_else(|_| self.default_value(&f.ty)),
            None => self.default_value(&f.ty),
        }
    }

    pub(crate) fn store_for(&mut self, rec: &Rec) -> &mut MemStore {
        // A CRM table's rows are its default table connection's (crm.rs).
        if let Some(name) = self.crm_connection_of(rec)
            && let Some(s) = self.crm_stores.get_mut(&name)
        {
            return &mut s.store;
        }
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
        self.source_caption(t)
    }

    /// The table's caption as its source declares it: a tableextension's Caption over its own, its Caption, or its name.
    fn source_caption(&self, t: &TableDef) -> String {
        let o = &self.engine.program.objects[t.object as usize];
        if let Some((_, v)) = o.properties.iter().find(|(k, _)| k == al_ir::EXT_CAPTION_OVER_OWN) {
            return v.trim().trim_matches('\'').to_string();
        }
        o.properties
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Caption"))
            .map(|(_, v)| v.trim().trim_matches('\'').to_string())
            .unwrap_or_else(|| t.name.clone())
    }

    /// The related table's caption in the TableRelation error of a Validate: [`Self::caption`] (a table one tableextension gives a
    /// Caption over its own is named by the extension's caption: al-runner issue 5955; corpus writer Y5, branch
    /// al-runtime-rs-staging-y5, recordref-fieldref/Y5TestFieldExtCaption.al, codeunit 90986).
    pub(crate) fn relation_error_caption(&self, t: &TableDef) -> String {
        self.caption(t)
    }

    /// The caption a read of it gives (TableCaption, RecordRef.Caption, Format of a RecordId): refused where a
    /// tableextension sets it over another caption (`al_ir::CAPTION_REFUSED`).
    pub(crate) fn caption_checked(&self, t: &TableDef) -> Result<String, Signal> {
        let Some(why) = self.caption_refused(t.object as usize) else {
            return Ok(self.caption(t));
        };
        // A caption a translation file of its app gives in the language in force (ADR-053 step 2).
        match self.table_caption_translation(t.object as usize, why) {
            Ok(Some(text)) => Ok(text),
            Ok(None) => Ok(self.source_caption(t)),
            Err(why) => Err(Signal::unsupported(why)),
        }
    }

    /// Why the caption of the table with object index `object` is not pinned, if it is not.
    /// `Get` with the RecordId of table `other` (record/crud/GetRecordId.al:123-164, al-runner
    /// issue 5795): with the result used it answers false and raises nothing (P98_Get_RecordId_OtherTable_ResultUsed) and
    /// sets no last error: after ClearLastError both texts stay empty, an earlier error stays the last one
    /// (Y8GetRecordIdCaption.al:65-86). Unused, also in a TryFunction, it raises the error below with code `DB:Ok`
    /// (P98_Get_RecordId_OtherTable), naming both tables by their captions (Y8GetRecordIdCaption.al:48-63).
    fn get_other_tables_record_id(&mut self, t: &TableDef, other: usize) -> Result<Value, Signal> {
        if self.ret_used {
            return Ok(Value::Boolean(false));
        }
        let Some(o) = self.engine.tables.get(other).cloned().flatten() else {
            return unsupported("Get of a RecordId of a table that was not compiled");
        };
        let mut names = Vec::new();
        for d in [&*o, t] {
            names.push(self.caption_checked(d)?);
        }
        Err(Signal::Error(Box::new(AlError {
            message: format!(
                "A RecordID from table '{}' cannot be used with a record from table '{}'.",
                names[0], names[1]
            ),
            code: "DB:Ok".into(),
            ..Default::default()
        })))
    }

    pub(crate) fn caption_refused(&self, object: usize) -> Option<String> {
        let o = &self.engine.program.objects[object];
        // One tableextension that sets a Caption over the table's own: every text that names the table, TableCaption and
        // RecordRef.Caption give the extension's Caption (al-runner issues 5955 and 6378; corpus writer Y5, branch
        // al-runtime-rs-staging-y5, recordref-fieldref/Y5TestFieldExtCaption.al, codeunit 90986, measured on Base
        // Application's Machine Center). AllObjWithCaption of such a table is not measured and stays refused there.
        let ext_over_own = o.properties.iter().any(|(k, _)| k == al_ir::EXT_CAPTION_OVER_OWN);
        if !ext_over_own && let Some((_, v)) = o.properties.iter().find(|(k, _)| k == al_ir::CAPTION_REFUSED) {
            return Some(v.clone());
        }
        // A declared caption a translation file of its app could change in the language in force (ADR-053 step 1).
        if al_builtins::format::FormatCtx::language(self) == 1033 {
            return None;
        }
        let captioned = o
            .properties
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("Caption") || k.eq_ignore_ascii_case("CaptionML"));
        if !captioned {
            return None;
        }
        self.translated_read_refusal(object, || format!("the caption of {} {}", o.kind.label(), o.name))
    }

    /// `No.='A'` style identification of a row's primary key. A key value whose format is not
    /// pinned refuses.
    pub(crate) fn identification(&self, t: &TableDef, fields: &[Value]) -> Result<String, Signal> {
        let mut parts = Vec::new();
        for &i in &t.keys[0] {
            let v = al_builtins::format::try_format(&fields[i], 0, 0, self).map_err(Signal::unsupported)?;
            // The field's caption names it (recordref-fieldref/recordref + records/lt-record-meta: `Code Caption='A1'`).
            parts.push(format!("{}='{}'", self.field_caption(t, i), v));
        }
        Ok(parts.join(","))
    }

    /// Refuses a read whose answer depends on the rows of table `t` when the install baseline could
    /// not create the rows BC has there (`Engine::unseeded`, ADR-013 decision 5): the answer would
    /// be "empty" or "fewer rows", which is not BC's. Rows this session wrote itself do not make
    /// the answer BC's, so they are not an exception. Temporary records are private to the session.
    /// Whether a `Get` that finds nothing, or a scan, on table object `object` is not BC's answer.
    /// A platform table of `UNSEEDED_ROWS` stops being one once the session's own DeleteAll without filters emptied it (and
    /// while that stands, `MemStore::emptied`): BC has then removed its fresh-tenant rows too, and what it holds is what the
    /// session wrote (Power BI Blob: Microsoft's Power BI tests clear it before they insert, al-language-onprem/metadata/
    /// PtOnPremTables.al Y17_PowerBIBlob_BlobRoundTrip, corpus master 97c29f86).
    pub(crate) fn is_unseeded(&self, t: &TableDef) -> bool {
        let o = t.object as usize;
        self.engine.unseeded.contains(&o)
            || (!self.install_scope && self.engine.platform_unseeded.contains(&o) && !self.store.emptied(&self.cx, t))
    }

    /// Why the rows install code left in table `object` are not known (ADR-046: `InstallFacts::unknown`). Install code
    /// itself reads them (it runs in an order of the runtime's own; the tables it writes are in the set).
    pub(crate) fn install_unknown(&self, object: u32) -> Option<&str> {
        if self.install_scope {
            return None;
        }
        let facts = self.engine.install_facts.get()?;
        facts.unknown.get(&(object as usize)).map(String::as_str)
    }

    /// Marks the assumption the install rows of table `object` rely on (`InstallFacts::assumed`), outside install code.
    pub(crate) fn install_assumed(&self, object: u32) {
        if self.install_scope {
            return;
        }
        let Some(facts) = self.engine.install_facts.get() else {
            return;
        };
        if let Some((_, a)) = facts.assumed.get(&(object as usize)) {
            al_builtins::assumptions::assume(*a);
        }
    }

    /// The refusal of a read of field `pos` of a row of table `table` (`fields`, the record's values) when that field is
    /// `SystemRowVersion`, `SystemCreatedAt` or `SystemModifiedAt`, the row was written by install code, and the order of
    /// the install routines that write the table is not known (ADR-046 amendment 1, A2 item d; the row version orders
    /// rows across tables: record/BfInstallTests.al:8-9, :196-241).
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
            Some(Value::BigInteger(v)) if *v > self.restored_row_version && *v <= self.install_row_version => Some(
                format!("{name} of a row of {} that install code wrote: {why}", def.name),
            ),
            _ => None,
        }
    }

    /// ADR-046 decision 4a: in the default mode (one company) a read of the Company table that could list a company the
    /// runtime does not know is refused. The corpus database holds a second company the runtime does not have
    /// (record/BfInstallTests.al:60-62). A read filtered to one known name (a single-value filter on
    /// Name: the session company or a company row the run holds) is answered.
    pub(crate) fn company_table_read(&self, rec: &Rec, what: &str) -> Result<(), Signal> {
        // Under the `--companies` opt-in the declared list is the database's complete company list: answered.
        if !self.companies.is_empty() {
            return Ok(());
        }
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
            // Every read of a table's rows passes here, the indirect ones too (FlowField, TableRelation, list caption).
            self.note_indirect_read(t.object);
            self.install_assumed(t.object);
            if let Some(why) = self.install_unknown(t.object) {
                return unsupported(format!("{what} on {}: {why}", t.name));
            }
        }
        if !temporary && self.is_unseeded(t) {
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
        if !exists && self.is_unseeded(t) {
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
    /// A table extension of an app that is not installed yet (an app installs after the one whose install code runs) is not
    /// active: its table triggers do not run (install-upgrade/install-order/Y8InstallOrderTests.al:88-110, row 1 that XApp A's
    /// install writes with Insert(true) has Note 'A', without XApp B's '+BX').
    pub(crate) fn ext_triggers(&self, t: &TableDef, name: &str) -> Vec<(usize, usize)> {
        let mut found = Vec::new();
        if let Some(schema) = &self.engine.program.objects[t.object as usize].table {
            for &e in &schema.extensions {
                if self.object_app_not_installed(e) {
                    continue;
                }
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

    /// Whether table extension `e`, whose install order is not known, can change what trigger `name` does (`run_ext_trigger`).
    fn ext_trigger_matters(&self, e: usize, name: &str) -> bool {
        name.to_ascii_lowercase().contains("insert")
            || self.engine.program.objects[e]
                .routines
                .iter()
                .enumerate()
                .any(|(ri, r)| {
                    r.is_trigger
                        && r.field.is_none()
                        && r.name.eq_ignore_ascii_case(name)
                        && !self.inert_in_either_order(e, ri)
                })
    }

    /// Runs the extension trigger `name`, if any, with the record and `xrec`.
    fn run_ext_trigger(&mut self, t: &TableDef, name: &str, rec: &Rec, xrec: Option<&Rec>) -> Result<(), Signal> {
        // While an app installs, a table extension of an app with no dependency path to it may or may not be installed yet:
        // whether its triggers and InitValues apply is not known. Its InitValues matter to an insert only; another trigger
        // matters only when the extension declares it and it has an effect in either order (db_event_no_op.rs, empty_reads.rs;
        // corpus install-upgrade/install-triggers/Y79InstallUnrelated.al:6-12, al-runner issue 6924).
        if let Some(schema) = &self.engine.program.objects[t.object as usize].table
            && let Some(&e) = schema
                .extensions
                .iter()
                .find(|&&e| self.object_app_order_unknown(e) && self.ext_trigger_matters(e, name))
        {
            self.note_order_unknown_hit(self.engine.program.objects[e].app);
            return unsupported(format!(
                "{name} of table {}: a table extension in app {} while an app installs that it has no dependency path to; the \
                 install order of the two is not known",
                self.engine.program.objects[t.object as usize].name,
                self.engine
                    .program
                    .apps
                    .get(self.engine.program.objects[e].app as usize)
                    .map(|a| a.name.as_str())
                    .unwrap_or("?")
            ));
        }
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
    pub(crate) fn xrec_handle(rec: &Rec, fields: Vec<Value>) -> Rec {
        let mut st = state_with_fields(rec, fields);
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
        // NavApp.LoadPackageData raises no table event (corpus install-upgrade/install-triggers/Y88PackageData.al,
        // Y88_Events_LoadRaisesNoTableEvent).
        if self.package_load {
            return Ok(());
        }
        self.note_event(t.object as usize, event, element);
        if !self.listened(t, event, element)? {
            return Ok(());
        }
        // A temporary record raises the table events and runs the triggers like any other
        // (corpus record/TestTableEventSequenceContracts.al:TemporaryRecord_RaisesTheEventsAndRunsTheTriggers).
        let (names, tail): (&[&str], Value) = match event {
            "onbeforeinsertevent" | "onafterinsertevent" if self.run_trigger_unknown.is_some() => (
                &["Rec", "RunTrigger"],
                Value::Unknown(self.run_trigger_unknown.unwrap_or_default().into()),
            ),
            "onbeforeinsertevent" | "onafterinsertevent" | "onbeforedeleteevent" | "onafterdeleteevent" => {
                (&["Rec", "RunTrigger"], Value::Boolean(run_trigger))
            }
            "onbeforemodifyevent" | "onaftermodifyevent" | "onbeforerenameevent" | "onafterrenameevent" => {
                (&["Rec", "xRec", "RunTrigger"], Value::Boolean(run_trigger))
            }
            // CurrFieldNo is 0 for a Validate called from code (the field's own OnValidate
            // sees 0: testpage/TestPageCurrFieldNo_Tests.al); docs/probes-needed.md for the event.
            // Through a TestPage write it is the field's number (record/load-fields/P81PageValidateEvents.al:
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
            self.note_xrec(x);
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
        // A by-value subscriber of a measured table event takes Rec as `blob_transfer::Argument` says.
        self.events.blob_publisher = crate::blob_transfer::measured_table_event(event, rec.borrow().temporary);
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
        // An error in an OnBeforeValidateEvent or OnAfterValidateEvent subscriber puts the record back too, as one in the
        // field's OnValidate below (events/binding-senders/DbtEvents.al:623 Validate_ErrorInSubscriber, `name`, `name3`).
        let restore = |rec: &Rec, e: Signal, snapshot: Vec<Value>| {
            if matches!(e, Signal::Error(_)) {
                rec.borrow_mut().fields = snapshot;
            }
            e
        };
        if let Err(e) = self.table_event(t, "onbeforevalidateevent", rec, Some(xrec), false, Some(&field.name)) {
            return Err(restore(rec, e, snapshot));
        }
        // The triggers table extensions attach to the field bracket its own OnValidate
        // (record/TestTriggerDispatchOrder.al:TriggerOrder_Validate_FieldPipeline_IsStable).
        let (before, after) = self.ext_validate_order(t, field)?;
        // An error in an OnBeforeValidate or OnAfterValidate trigger a table extension adds to the field puts the whole record
        // back too (al-runner issue 6978; corpus
        // record/TestValidateRaisesAndHandlerAsserterrorContracts.al:167-274, ExtOnBeforeValidate_Raises_RecordAfterwards,
        // ExtOnAfterValidate_Raises_RecordAfterwards, ExtOnAfterValidate_Raises_FieldOnValidateChangesUndone).
        for &(object, routine) in &before {
            if let Err(e) = self.run_trigger(object, routine, rec, Some(xrec)) {
                return Err(restore(rec, e, snapshot));
            }
        }
        if let Some((object, routine)) = field.on_validate {
            if let Err(e) = self.run_trigger(object, routine, rec, Some(xrec)) {
                let snapshot = snapshot.clone();
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
        for &(object, routine) in &after {
            if let Err(e) = self.run_trigger(object, routine, rec, Some(xrec)) {
                return Err(restore(rec, e, snapshot));
            }
        }
        self.table_event(t, "onaftervalidateevent", rec, Some(xrec), false, Some(&field.name))
            .map_err(|e| restore(rec, e, snapshot))
    }

    /// `Record.Delete(run)` of the row `rec` holds: Before event, OnDelete, the
    /// delete, After event.
    /// A Modify or Delete let go of media `id` that field `pos` of table `t` held: the media leaves Tenant Media unless another
    /// row of the same field still holds it (streams/media/TestMediaPngImport.al:297-334: kept while the other row holds it, removed with
    /// the Delete of that row; streams/ArRecordProbes.al:141-143: removed when no row holds it). A media another
    /// table or field holds is not pinned and refused. The removal is undone by a rollback (`Session::rollback_transaction`).
    fn release_media(
        &mut self,
        cx: &al_data::AccessContext,
        t: &TableDef,
        pos: usize,
        id: [u8; 16],
    ) -> Result<(), Signal> {
        if self.is_stored_tenant_media_id(id) {
            return unsupported(
                "a Modify or Delete that lets go of the ID of a Tenant Media row an AL Insert stored (not pinned)",
            );
        }
        let held = |s: &mut Self, table: &TableDef, field: usize| -> Result<bool, Signal> {
            let filters = [FieldFilter {
                lang: al_builtins::filter::filter_language(),
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
                .map_err(|e| Signal::unsupported(format!("{e}")))?
                .is_some())
        };
        // Another row of the same table holding it in the same field keeps it (streams/media/TestMediaPngImport.al:297-334
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

    /// `Delete()` and `Delete(true)` of a Tenant Media row (streams/media/TestMediaPngImport.al TP_TenantMedia_Delete,
    /// TQ_TenantMedia_DeleteTrue_Statement, _Used, _SharedMedia, _Missing_Statement): the media leaves Tenant Media and Delete
    /// returns true (the statement form raises no error); every row that holds it, also two rows, keeps the MediaId with
    /// HasValue false and ExportStream false. A missing id returns false, or in the statement form fails with
    /// DB:RecordNotFound. With event subscribers or global triggers, and for a media a MediaSet holds, it is not pinned.
    fn delete_tenant_media(&mut self, t: &TableDef, rec: &Rec, run: bool) -> Result<Value, Signal> {
        if (run && self.table_trigger(t, "OnDelete").is_some())
            || self.listened(t, "onbeforedeleteevent", None)?
            || self.listened(t, "onafterdeleteevent", None)?
            || self.global_enabled(t, GlobalOp::Delete)?
        {
            return unsupported("Delete of a Tenant Media row with triggers, events or global triggers (not pinned)");
        }
        let Some(&key) = t.keys.first().and_then(|k| k.first()) else {
            return unsupported("Delete of a Tenant Media row: the table has no primary key");
        };
        let Value::Guid(id) = rec.borrow().fields[key].clone() else {
            return unsupported("Delete of a Tenant Media row whose key is not a Guid");
        };
        if self.media.sets.borrow().values().any(|items| items.contains(&id))
            || self.media.outside_tenant.borrow().contains(&id)
        {
            return unsupported("Delete of a Tenant Media row a MediaSet or Media Resources holds (not pinned)");
        }
        let removed = self.media.objects.borrow_mut().remove(&id);
        match removed {
            Some(obj) => {
                self.media.removed.borrow_mut().insert(id);
                self.media_removed.0.push((id, obj));
                Ok(Value::Boolean(true))
            }
            // A missing row: false when the return value is used (TP_TenantMedia_Delete, TQ_TenantMedia_DeleteTrue_Used), else
            // `The Tenant Media does not exist. Identification fields and values: ID='{..}'`, DB:RecordNotFound
            // (TQ_TenantMedia_DeleteTrue_Missing_Statement).
            None => {
                let row = rec.borrow().fields.clone();
                self.fail(t, &row, false)
            }
        }
    }

    /// The Delete of a row whose MediaSet holds items: their Tenant Media rows go (TestMediaPngImport.al:340-378). What happens
    /// when another row still holds the same set is not pinned: refused.
    fn release_media_set(
        &mut self,
        cx: &al_data::AccessContext,
        t: &TableDef,
        pos: usize,
        set: [u8; 16],
    ) -> Result<(), Signal> {
        let filters = [FieldFilter {
            lang: al_builtins::filter::filter_language(),
            field: pos,
            expr: FilterExpr::Eq(Value::Guid(set)),
            group: 0,
        }];
        let scan = Scan {
            key: 0,
            filters: &filters,
            ascending: true,
            after: None,
        };
        let shared = self
            .store
            .next(cx, t, &scan)
            .map_err(|e| Signal::unsupported(format!("{e}")))?
            .is_some();
        if shared {
            return unsupported(
                "a Delete of a row whose MediaSet another row still holds (what happens to its Tenant Media rows is not pinned)",
            );
        }
        let items = self.media.sets.borrow().get(&set).cloned().unwrap_or_default();
        for id in items {
            if let Some(obj) = self.media.objects.borrow_mut().remove(&id) {
                self.media.removed.borrow_mut().insert(id);
                self.media_removed.0.push((id, obj));
            }
        }
        Ok(())
    }

    fn delete_row(&mut self, t: &TableDef, rec: &Rec, run: bool) -> Result<Value, Signal> {
        let cx = self.cx_for(rec, t)?;
        // A Delete through a stale buffer follows the rule of a Modify (record/TestRecordStaleBufferWrite.al:662, :676); with
        // triggers or events, where the check falls among them is pinned for the shapes below only.
        if !rec.borrow().temporary && t.fields.iter().any(|f| f.id == 2_000_000_005) {
            let pk = Self::sql_key(rec, t, &rec.borrow().fields);
            let stored = self
                .store_for(rec)
                .get(&cx, t, &pk)
                .map_err(|e| Signal::unsupported(format!("{e}")))?;
            let i = t.fields.iter().position(|f| f.id == 2_000_000_005).expect("checked");
            if let Some(Value::BigInteger(theirs)) = stored.as_ref().map(|r| r[i].clone()) {
                let verdict = self.stale_verdict(t, rec, &cx, &pk, theirs, StaleWrite::Delete)?;
                if verdict != StaleVerdict::Fresh {
                    let mut ext = false;
                    for n in ["OnBeforeDelete", "OnDelete", "OnAfterDelete"] {
                        ext |= run && !self.ext_triggers(t, n).is_empty();
                    }
                    let hooks = run && self.table_trigger(t, "OnDelete").is_some()
                        || self.listened(t, "onbeforedeleteevent", None)?
                        || self.listened(t, "onafterdeleteevent", None)?
                        || self.global_enabled(t, GlobalOp::Delete)?;
                    // A stale own-Insert buffer deleted after the new transaction deleted other rows of the table only (the
                    // neighbour shape, written): the row is deleted and the table's OnDelete trigger, OnBeforeDeleteEvent,
                    // OnAfterDeleteEvent and the OnDatabaseDelete global trigger run as for a fresh buffer
                    // (al-language-testlibraries/test-framework/stale-delete-triggers/WbStaleDeleteTests.al:13-53, al-runner issue 6286). A table
                    // extension's delete triggers and a RecordChanged verdict are not pinned.
                    let company = rec.borrow().company.clone();
                    let pinned = verdict == StaleVerdict::Written
                        && !ext
                        && self.stale_after_other_deletes_only(t, &company, &pk);
                    // A stale buffer from the variable's own Insert two or three Commits back, deleted in a transaction that
                    // wrote nothing and took no lock: the OnBeforeDeleteEvent subscribers run, then (Delete(true)) the table's
                    // OnDelete trigger, then DB:RecordChanged; OnAfterDeleteEvent never runs and the row stays. When another
                    // variable Modified that row in the new transaction (the Modify rule writes the buffer), the row is deleted
                    // and the trigger and the events run as for a fresh buffer (record/crud/
                    // Y32StaleDeleteTests.al Y32_2C_*, Y32_3C_*, al-runner issue 6546). A table
                    // extension's delete triggers and the OnDatabaseDelete global trigger are not pinned.
                    let own_insert = {
                        let r = rec.borrow();
                        let mine = if let Value::BigInteger(n) = &r.fields[i] { *n } else { 0 };
                        r.row_origin.filter(|o| o.version == mine).is_some_and(|o| {
                            o.own_write && (o.commits + 2 == self.commits || o.commits + 3 == self.commits)
                        })
                    };
                    let y32 = own_insert && !ext && !self.global_enabled(t, GlobalOp::Delete)?;
                    let untouched_tx = self.tx.writes.is_empty()
                        && !self.tx.rolled_back
                        && !self.tx.lock_tx
                        && self.tx.lock_unpinned.is_none();
                    // record/crud/Y32StaleDeleteTests.al (al-runner issue 6546).
                    if y32 && verdict == StaleVerdict::Refused && untouched_tx {
                        self.table_event(t, "onbeforedeleteevent", rec, None, run, None)?;
                        if run && let Some(tr) = self.table_trigger(t, "OnDelete") {
                            let x = Self::xrec_handle(rec, rec.borrow().fields.clone());
                            self.run_trigger(t.object as usize, tr, rec, Some(&x))?;
                        }
                        return Err(self.record_changed(t, rec)?);
                    }
                    let row_written =
                        y32 && verdict == StaleVerdict::Written && self.stale_row_modified_only(t, &company, &pk);
                    let pinned = pinned || row_written;
                    if (hooks || ext) && !pinned {
                        return unsupported(format!(
                            "a Delete through a stale {} buffer, with triggers or events: not pinned",
                            t.name
                        ));
                    }
                }
                if verdict == StaleVerdict::Refused {
                    return Err(self.record_changed(t, rec)?);
                }
            }
        }
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
        // A key value the column rounds finds the rounded row (P130DateFilters.al:653-658, `delete unrounded`).
        let pk = Self::sql_key(rec, t, &row);
        let company = if self.is_company_table(t) && !rec.borrow().temporary {
            let name = al_builtins::native::text_of(&row[0]).unwrap_or_default();
            self.before_company_row_delete(&name, run)?;
            Some(name)
        } else {
            None
        };
        // The Media fields of the stored row: the Delete of the last row that holds a media removes it from Tenant Media
        // (streams/media/TestMediaPngImport.al:326-332).
        let mut media_sets: Vec<(usize, [u8; 16])> = Vec::new();
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
                        .map_err(|e| Signal::unsupported(format!("{e}")))?;
                    // The Delete removes the Tenant Media rows of the items still in a MediaSet; an item removed from the set
                    // earlier keeps its row (streams/media/TestMediaPngImport.al:340-378 MediaSet_Remove_KeepsTheTenantMediaRow:
                    // 'removed_item_after_delete' Yes, 'kept_item_after_delete' No).
                    media_sets = sets
                        .iter()
                        .filter_map(|&i| match stored.as_ref().map(|r| &r[i]) {
                            Some(Value::Guid(g)) if *g != [0; 16] => Some((i, *g)),
                            _ => None,
                        })
                        .collect();
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
        let before = self.store.change_counter();
        match self
            .store_for(rec)
            .delete(&cx, t, &pk)
            .map_err(|e| Signal::unsupported(format!("{e}")))?
        {
            WriteOutcome::Done => {
                self.deletes += 1;
                self.note_write(rec, t, crate::state::TxWriteKind::Delete, &pk, before);
                for (i, g) in media {
                    self.release_media(&cx, t, i, g)?;
                }
                for (i, set) in media_sets {
                    self.release_media_set(&cx, t, i, set)?;
                }
                if let Some(name) = company {
                    self.after_company_row_delete(&name);
                }
                // The platform deletes the user's User Property row with the User, after the
                // OnBeforeDelete subscribers and before the OnAfterDelete ones
                // (permissions/access-control/TestUserDeleteCascadeOrder.al:26-84 in the OnPrem app).
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
    /// The FlowFilter fields of `t`. A temporary row keeps the FlowFilter values its Insert wrote, and a Modify keeps them as
    /// the Insert left them (record/flowfields/DiFwWriteTests.al:937 SF_FlowFilter_TemporaryTableType_InsertThenRead,
    /// :975 SF_FlowFilter_TemporaryVariable_InsertThenRead; al-runner issue 5857).
    fn flow_filter_positions(&self, t: &TableDef) -> Vec<usize> {
        let schema = self.engine.program.objects[t.object as usize]
            .table
            .as_ref()
            .expect("table");
        (0..t.fields.len())
            .filter(|&i| t.fields[i].id < 2_000_000_000 && schema.fields[i].flow_filter)
            .collect()
    }

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
            let pk = Self::sql_key(rec, t, row);
            if self
                .store_for(rec)
                .get(&cx, t, &pk)
                .map_err(|e| Signal::unsupported(format!("{e}")))?
                .is_some()
            {
                return Ok(None);
            }
        }
        let current = match own {
            Some(pk) => self
                .store_for(rec)
                .get(&cx, t, pk)
                .map_err(|e| Signal::unsupported(format!("{e}")))?,
            None => None,
        };
        for k in unique {
            let filters: Vec<FieldFilter> = t.keys[k]
                .iter()
                .map(|&f| FieldFilter {
                    lang: al_builtins::filter::filter_language(),
                    field: f,
                    expr: FilterExpr::Eq(row[f].clone()),
                    group: 0,
                })
                .collect();
            let mut n = self
                .store_for(rec)
                .count(&cx, t, &filters)
                .map_err(|e| Signal::unsupported(format!("{e}")))?;
            // The row being rewritten matches itself when its key values do not change.
            if let Some(old) = &current {
                if t.keys[k].iter().try_fold(true, |all, &f| {
                    Ok::<_, Signal>(all && ops::compare_keys(&[old[f].clone()], &[row[f].clone()])?)
                })? {
                    n = n.saturating_sub(1);
                }
            }
            if n > 0 {
                return Ok(Some(k));
            }
        }
        Ok(None)
    }

    /// Whether table `t` declares a Blob field (a duplicate Insert into it fails at once).
    fn has_blob_field(&self, t: &TableDef) -> bool {
        self.engine.program.objects[t.object as usize]
            .table
            .as_ref()
            .is_some_and(|s| {
                s.fields
                    .iter()
                    .any(|f| matches!(&f.ty, RtType::Library(al_builtins::library_kind::LibraryKind::Blob)))
            })
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
            let v = al_builtins::format::try_format(&row[i], 0, 0, self).map_err(Signal::unsupported)?;
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
                self.note_xrec(x);
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
                let table = rec.borrow().table as usize;
                let fields = self.initial_fields(table);
                let mut st = state_with_fields(&rec, fields);
                st.globals = rec.borrow().globals.xrec();
                // The blank xRec of a record that a partial fetch filled loads nothing from the row: every field reads empty, the
                // key too (record/load-fields/XRecUnloadedFieldRead.al PartialGet_XRecFields_ReadInTableMethod,
                // PartialGet_RowChangedAfterGet_XRecReadFirst, PartialGet_KeyAssignedAfterGet_ReadInTableMethod, al-runner issue
                // 7375). It is marked as after an Init, which loads no field from the row.
                crate::partial::note_init(&mut st);
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
        let engine = self.engine.clone();
        for (object, inst) in instances {
            let o = &engine.program.objects[object as usize];
            for (i, slot) in o.globals.iter().enumerate() {
                if Some(i) == o.rec_slot || Some(i) == o.xrec_slot {
                    continue;
                }
                let v = self.slot_default(slot);
                inst.borrow_mut().globals[i] = v;
            }
        }
    }

    pub(crate) fn run_trigger(
        &mut self,
        object: usize,
        routine: usize,
        rec: &Rec,
        xrec: Option<&Rec>,
    ) -> Result<(), Signal> {
        let entered = !self.dirty_frames.is_empty() && self.dirty_trigger_enter(rec, xrec);
        let r = self
            .call_on_record(object, routine, rec.clone(), xrec, Vec::new())
            .map(|_| ());
        self.dirty_trigger_exit(entered);
        r
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
        if rec.borrow().adhoc.is_some() {
            if let Some(hit) = self.scan_flowfield_key(rec, t, ascending, &after)? {
                return Ok(hit);
            }
        }
        if rec.borrow().field_descending != 0 {
            return self.scan_first_field_reversed(rec, t, ascending, after);
        }
        self.scan_plain(rec, t, ascending, after)
    }

    /// A scan in the order `SetAscending(Field, false)` gives: the fields marked descending run backwards and the rest
    /// of the key, then the primary key, forwards (record/W4SortKeys.al:SetAscending_SingleFieldKey: key Txt, ids
    /// `4,2,3,1,5`, where `Ascending(false)` and a view give `4,3,2,5,1`; record/filters/P68D_Ascending.al). The rows are read in key order and sorted
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
        let (mask, fields) = (rec.borrow().field_descending, Session::position_fields(t, key));
        let order = |a: &[Value], b: &[Value]| -> Ordering {
            for (i, (x, y)) in a.iter().zip(b).enumerate() {
                let o = Session::compare_in_position(t, &fields, i, x, y);
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
                .map_err(|e| Signal::unsupported(format!("store error {e}")))?;
            match row {
                // MarkedOnly narrows the set to the marked rows (corpus record/TestRecordFind.al:258-290).
                Some(row) if marked_only && !Self::is_marked(rec, t, &row)? => after = Some(position(t, key, &row)),
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
        if !al_builtins::filter::all_match(&filters, row)? {
            return Ok(false);
        }
        if rec.borrow().marked_only && !Self::is_marked(rec, t, row)? {
            return Ok(false);
        }
        if flow_field_filters.is_empty() {
            return Ok(true);
        }
        self.passes_flow_field_filters(rec, row, &flow_field_filters)
    }

    /// Whether the record last read the row with primary key `pk` (see `RecordState::position`).
    fn on_own_row(rec: &Rec, pk: &[Value]) -> Result<bool, Signal> {
        match rec.borrow().position.as_deref() {
            Some(p) => ops::compare_keys(p, pk),
            None => Ok(false),
        }
    }

    /// Whether `row` is marked. A mark whose key does not compare with the row's refuses (al-runner issue 7644).
    fn is_marked(rec: &Rec, t: &TableDef, row: &[Value]) -> Result<bool, Signal> {
        let pk = t.primary_key(row);
        for m in &rec.borrow().marks {
            if ops::compare_keys(m, &pk)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Whether another row of the record's table holds the SystemId in its buffer.
    fn system_id_taken(&mut self, rec: &Rec, t: &TableDef) -> Result<bool, Signal> {
        let Some(i) = t.fields.iter().position(|f| f.id == 2_000_000_000) else {
            return Ok(false);
        };
        let id = rec.borrow().fields[i].clone();
        let filters = [FieldFilter {
            lang: al_builtins::filter::filter_language(),
            field: i,
            expr: FilterExpr::Eq(id),
            group: 0,
        }];
        let cx = self.cx_for(rec, t)?;
        let taken = self
            .store_for(rec)
            .count(&cx, t, &filters)
            .map_err(|e| Signal::unsupported(format!("{e}")))?;
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
        let shown = al_builtins::format::try_format(&id, 0, 0, self).map_err(Signal::unsupported)?;
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
        // (record/filters/P130DateFilters.al:324, :402).
        let t = self.table(rec)?;
        if rec.borrow().temp_id.is_none() {
            for (pos, v) in row.iter_mut().enumerate() {
                if let Value::Blob(b) = v {
                    *b = self.copy_blob_db_row(&t, pos, b);
                }
            }
        } else {
            // A Blob a Modify persisted reads empty after a Get until CalcFields, as a database record's does; one still being
            // written keeps its content (record/PnABlobAssignRename.al:Assign_Temporary `regot-source = ''`).
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
        let via_sysid = std::mem::take(&mut self.sysid_load);
        let via_find = std::mem::take(&mut self.find_load);
        // A temporary record loads every field whatever SetLoadFields asked for (record/load-fields/
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
                assigned: Vec::new(),
                missing_loaded: false,
                fetched: Default::default(),
                init: false,
            });
            r.load.partial = false;
        }
        let pk = t.primary_key(&row);
        // The record remembers the row as its own only when the row passed its filters when it was read; a filter set later forgets it
        // (record/TestFindSetSortKeyModify.al:OwnFilter_ModifiedOutOfIt_FindStillFindsTheRow against misc/lt-probes-needed-3/
        // PnC02Filters.al:FindEqual_OutsideFilter and record/crud/RcAMisc.al:MarkedOnly_With_Find_Get_And_Order).
        let passes = self.row_passes(rec, &t, &row)?;
        let version = t.fields.iter().position(|f| f.id == 2_000_000_005).map(|i| &row[i]);
        let origin = self.load_origin(version, [via_get, via_sysid, via_find]);
        let own_read = self.own_read_after_load(&t, rec, &row, via_get && !via_sysid && !via_find && passes);
        let temporary = rec.borrow().temporary;
        if via_get && !temporary && self.tx.locked_tables.contains(&t.object) {
            let company = rec.borrow().company.clone();
            self.tx.rows_touched.push((t.object, company, pk.clone()));
        }
        {
            let mut r = rec.borrow_mut();
            r.position = passes.then_some(pk);
            r.fields = row;
            r.row_origin = origin;
            r.own_read = own_read;
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

    /// The primary key the row of `fields` stands under in the record's store: for a database table with its Time and DateTime
    /// values rounded as the SQL columns hold them (`sqltime::rounded_key`).
    pub(crate) fn sql_key(rec: &Rec, t: &TableDef, fields: &[Value]) -> Vec<Value> {
        let pk = t.primary_key(fields);
        if rec.borrow().temporary {
            pk
        } else {
            crate::sqltime::rounded_key(&pk)
        }
    }

    /// The primary key of `fields` where a key value the SQL column rounds is not pinned (`what` names the operation).
    fn exact_key(rec: &Rec, t: &TableDef, fields: &[Value], what: &str) -> Result<Vec<Value>, Signal> {
        let pk = t.primary_key(fields);
        if !rec.borrow().temporary && crate::sqltime::key_rounds(&pk) {
            return Err(Signal::unsupported(format!(
                "{what} of a record whose Time or DateTime primary key value the SQL column rounds (not pinned)"
            )));
        }
        Ok(pk)
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
            // with `Get` and written in memory is read back from the row too (record/flowfields/
            // BlobCalcFieldsLength.al:BlobCalcFields_UnsavedContent_Result, 17 bytes in memory, 6 after CalcFields): see `BlobBuf::fetched`.
            let temporary = rec.borrow().temporary;
            // A record assignment's copy (`refetch`) of a buffer that holds an unsaved stream write keeps the write when the
            // row is stored, as the buffer it was copied from does (al-runner issue 7360). A copy of a row that is not stored
            // reads empty (types/ProbeTypeFormsAndFilters.al:228-250 FormatRecord_AssignmentCopiesBlobBytes): see `copied_write`.
            let copied_write = buf.borrow().refetch && buf.borrow().written;
            let keeps_content = buf.borrow().fetched && (!buf.borrow().refetch || copied_write) && !temporary;
            if stored.is_none() {
                let fields = rec.borrow().fields.clone();
                let pk = Self::exact_key(rec, &t, &fields, "a Blob read")?;
                stored = self
                    .store_for(rec)
                    .get(&cx, &t, &pk)
                    .map_err(|e| Signal::unsupported(format!("{e}")))?;
            }
            // A row this variable inserted and that is gone (an error inside ASSERTERROR rolled the Insert back) leaves the Blob empty, whatever a
            // stream wrote since (recordref-fieldref/PnE08RecordMisc.al:TestField_Blob `af219`).
            // A buffer that holds no unsaved write and differs from the stored row was left behind by another variable's
            // Modify (Copy Document's report writes the new order's Work Description through its own copy of the header;
            // Tests-ERM ERM Copy Purch/Sales Doc.CopyWorkDescriptionSalesArchiveShipment reads it back through the caller's
            // record). CalcFields then reads the row. Pinned by streams/CalcFieldsBlobStaleBuffer.al.
            // An unsaved stream write is replaced the same way when the row changed since the write began
            // (streams/CalcFieldsBlobStaleBuffer.al:75-98); `BlobBuf::written_base` says what the row held then.
            let stale = keeps_content
                && match stored.as_ref().map(|row| &row[pos]) {
                    Some(Value::Blob(src)) => {
                        let (src, buf) = (src.borrow(), buf.borrow());
                        match (&buf.written_base, buf.written) {
                            (_, false) => src.bytes != buf.bytes,
                            (Some(base), true) => src.bytes != *base && src.bytes != buf.bytes,
                            (None, true) => false,
                        }
                    }
                    _ => false,
                };
            // An assigned Blob that nothing saved or wrote since is kept, as an unsaved stream write is, when the stored row's
            // Blob is empty (assumption `blob-assign-counts-as-write`: al-runner issue 7961, the reporter's repro on real BC).
            let stored_empty =
                matches!(stored.as_ref().map(|row| &row[pos]), Some(Value::Blob(src)) if src.borrow().bytes.is_empty());
            if keeps_content && stored_empty && buf.borrow().assigned_unsaved {
                al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::BlobAssignCountsAsWrite);
                continue;
            }
            if stale {
                // Pinned: streams/CalcFieldsBlobStaleBuffer.al (empty, loaded-earlier and unsaved-write buffers).
            } else if copied_write {
                if keeps_content && stored.is_some() {
                    // Pinned: streams/BlobWriteThenRecordCopy.al:48-155 (Stored_*), the unsaved write survives the copy.
                    continue;
                }
            } else if keeps_content && (stored.is_some() || !rec.borrow().inserted) {
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
            b.written_base = None;
            b.assigned_unsaved = false;
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
                b.written_base = None;
                b.assigned_unsaved = false;
                // A Blob the record read without CalcFields stays as stored by the save (record/load-fields/
                // P81LoadFields.al:700-711) and stays unloaded: a later CalcFields fetches it (streams/TestBlob.al
                // Blob_Get_WithoutCalcFields_InStreamReadsEmpty), it does not keep the empty view the buffer shows.
                if b.loaded {
                    b.fetched = true;
                }
            }
        }
    }

    /// The platform's agent tables Agent (2000000260), Agent Access Control (2000000261), Agent Task (2000000262) and Agent
    /// Task Message (2000000263): in a test session on the corpus containers every read and every insert fails with the agent
    /// service's error, code DB:Ok, and leaves no write transaction, whatever app calls it (onprem
    /// metadata/session-system/Y35AgentDirect.al C302_AgentDirect_FindSetFiltered, _Reads, _Inserts: FindSet with a filter,
    /// FindFirst, IsEmpty, Count, FindLast, Get, Insert(true), Insert() on Agent; metadata/session-system/Y35AgentApi.al
    /// C302_AgentTaskDirect_Reads, _Inserts: FindSet, Count, Get and Insert on Agent Task, FindSet and Insert on Agent Task
    /// Message, FindSet on Agent Access Control; C302_AgentApi_Create, _Activate, _TaskCreate through the System Application;
    /// metadata/session-system/RhAgentSession.al:74-145 for IsEmpty on Agent Task and Get on Agent). Every other
    /// operation stays refused by the provider ("has no provider").
    fn agent_task_check(&self, t: &TableDef, rec: &Rec, method: RecordOp) -> Result<(), Signal> {
        if rec.borrow().temporary {
            return Ok(());
        }
        let id = self.engine.program.objects[t.object as usize].id;
        if !matches!(id, Some(2000000260..=2000000263)) {
            return Ok(());
        }
        let pinned = match id {
            Some(2000000260) => matches!(
                method,
                RecordOp::FindSet
                    | RecordOp::FindFirst
                    | RecordOp::FindLast
                    | RecordOp::IsEmpty
                    | RecordOp::Count
                    | RecordOp::Get
                    | RecordOp::Insert
            ),
            Some(2000000262) => matches!(
                method,
                RecordOp::FindSet | RecordOp::Count | RecordOp::Get | RecordOp::IsEmpty | RecordOp::Insert
            ),
            Some(2000000263) => matches!(method, RecordOp::FindSet | RecordOp::Insert),
            _ => method == RecordOp::FindSet,
        };
        if !pinned {
            return Ok(());
        }
        let object = &self.engine.program.objects[t.object as usize];
        let platform = self.engine.program.apps.get(object.app as usize).is_some_and(|a| {
            a.id.eq_ignore_ascii_case(crate::platform::objects::al_sema_platform_app_id())
        });
        if !platform {
            return Ok(());
        }
        Err(crate::transactions::al_error_code(
            "Agents are temporarily unavailable. Please try again later. You can check the Business Central Admin Center and Microsoft 365 Service Health dashboard for updates.",
            "DB:Ok",
        ))
    }

    /// A `LinkedObject = true` table has no SQL table behind it in a test database: Insert and Count raise the SQL error
    /// (record/table-properties/XTablePropsTests.al:720-737, text and code). The other database operations are not pinned.
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

    /// A record method on a record whose buffer is not pinned (`RecordState::unpinned_buffer`): methods that do not read the
    /// buffer run, a read that fills it again (Get, FindFirst, FindLast, FindSet, Find('-'/'+') answering true) makes it
    /// pinned again, and every other method refuses. Rec in the OnOpenPage of a page an action opens through `RunPageLink`
    /// ([`RUN_PAGE_LINK_OPEN`]) is narrower, see [`Self::run_page_link_open_op`].
    fn unpinned_buffer_op(&mut self, method: RecordOp, rec: &Rec, args: &mut [Value]) -> Result<Value, Signal> {
        let why = rec.borrow().unpinned_buffer.unwrap_or_default();
        if why == RUN_PAGE_LINK_OPEN || why == RUN_PAGE_LINK_OPEN_FILLED {
            return self.run_page_link_open_op(method, rec, args);
        }
        if why == XP_VIEW_AFTER_EXPORT {
            return self.xp_view_after_export_op(method, rec, args);
        }
        if why == RUN_PAGE_LINK_COPY || why == RUN_PAGE_LINK_OPEN_COPY {
            return self.run_page_link_copy_op(method, rec, args, why);
        }
        if why == RUN_PAGE_LINK_VARIANT {
            return unsupported(format!("Record.{}(): {RUN_PAGE_LINK_VARIANT}", method.name()));
        }
        if why == HELD_PART_XREC {
            return unsupported(format!("Record.{}(): {HELD_PART_XREC}", method.name()));
        }
        if why == HELD_PART_REC {
            if method != RecordOp::Count || !args.is_empty() {
                return unsupported(format!("Record.{}(): {HELD_PART_REC}", method.name()));
            }
            let flag = rec.borrow_mut().unpinned_buffer.take();
            let r = self.record_op_inner(method, rec, args);
            rec.borrow_mut().unpinned_buffer = flag;
            return r;
        }
        if why == MEDIA_KEY || why == FLOWFIELD_KEY {
            return self.media_key_op(method, rec, args, why);
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
        // After Reset or SetView the view of a table element record is the code's own again; only its buffer is unpinned
        // ([`XP_BUFFER_AFTER_EXPORT`]).
        let own_view = (why == XP_BUFFER_AFTER_EXPORT || why == XP_NESTED_ROW_AFTER_ONCE)
            && matches!(
                method,
                RecordOp::CurrentKey
                    | RecordOp::GetFilter
                    | RecordOp::GetFilters
                    | RecordOp::GetRangeMin
                    | RecordOp::GetRangeMax
                    | RecordOp::GetView
                    | RecordOp::HasFilter
                    | RecordOp::SetView
            );
        if !refill && !independent && !own_view {
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

    /// A record method on a table element record after an export under `SourceTableView` ([`XP_VIEW_AFTER_EXPORT`]). What corpus
    /// writer TS pinned runs (former assumption `xmlport-export-again` (pinned on corpus master since bump 1030); QrXpContainerOccurs.al TS_Again_* and TS_View_* on branch
    /// al-runtime-rs-staging-pub-ts): `Count` and `GetView(false)` (the view's key, the code's filters; the view's own filters
    /// count but do not show), `SetCurrentKey`, `SetRange` and `SetFilter` in filter group 0 (kept by a later export), `Init`
    /// (primary key kept, SystemId null), `TransferFields(From, true)` from the same table (SystemId not copied), and `Reset`
    /// (primary key, no filters: the mark goes). `SetView` gives it a view of its own with the buffer still marked
    /// ([`XP_BUFFER_AFTER_EXPORT`]); the table's names and `FieldNo` run; every other method refuses.
    fn xp_view_after_export_op(&mut self, method: RecordOp, rec: &Rec, args: &mut [Value]) -> Result<Value, Signal> {
        let group0 = rec.borrow().filter_group == 0;
        // GetFilters reads '' while only the view's own filters are set (corpus PR 896, streams/Np4XmlPort.al
        // Y9_XmlPort_LinkedOnceOnce_ZeroOneTwoRows `held 3/10/A/h3l10|filters=`: a linked element with a SourceTableView).
        if method == RecordOp::GetFilters
            && args.is_empty()
            && self.xp_link_hidden.contains(&(Rc::as_ptr(rec) as usize))
            && rec
                .borrow()
                .filters
                .iter()
                .all(|f| f.group == crate::reports::ITEM_VIEW_GROUP)
        {
            return Ok(Value::Text("".into()));
        }
        let pinned = match method {
            RecordOp::Count | RecordOp::Reset | RecordOp::SetCurrentKey => true,
            RecordOp::GetView => matches!(args, [Value::Boolean(false)]),
            RecordOp::SetRange | RecordOp::SetFilter => group0,
            RecordOp::Init => args.is_empty(),
            RecordOp::TransferFields => {
                matches!(args.first(), Some(Value::Record(src)) if src.borrow().table == rec.borrow().table)
                    && args.get(1).is_none_or(|v| matches!(v, Value::Boolean(true)))
            }
            _ => false,
        };
        let runs = pinned
            || matches!(
                method,
                RecordOp::SetView
                    | RecordOp::FieldNo
                    | RecordOp::IsTemporary
                    | RecordOp::TableName
                    | RecordOp::TableCaption
            );
        if !runs {
            return unsupported(format!("Record.{}(): {XP_VIEW_AFTER_EXPORT}", method.name()));
        }
        // Pinned by xmlports/shapes-properties/QrXpContainerOccurs.al TS_Again_SameData, TS_Again_AssignBetween, TS_Again_TransferFieldsBetween, TS_Again_InitBetween, TS_Again_KeyBetween, TS_Again_LineFilter, TS_View_SecondExport, TS_Again_TrBooking, TS_Again_TrMember (was assumption `xmlport-export-again`).
        let flag = rec.borrow_mut().unpinned_buffer.take();
        let r = self.record_op_inner(method, rec, args);
        rec.borrow_mut().unpinned_buffer = match method {
            RecordOp::Reset if r.is_ok() => None,
            RecordOp::SetView if r.is_ok() => Some(XP_BUFFER_AFTER_EXPORT),
            _ => flag,
        };
        r
    }

    /// A record method on a by-value copy of Rec of a page an action opened through `RunPageLink` ([`RUN_PAGE_LINK_COPY`]):
    /// `GetFilters` of a copy that holds no filter reads '' (testpage/TestActionRunPageLinkFilterGroup.al:292-369, `filters=`),
    /// the names of its table run. On such a copy `Count` counts every row of the table and `Get` finds the current row and a
    /// row outside the link (testpage/R7138RunPageLinkCopyGet.al:136-151, :218 `copy[] n=4;copy.get own O3;copy.get O1 O1`, in
    /// OnAfterGetCurrRecord; not for a copy taken in OnOpenPage, [`RUN_PAGE_LINK_OPEN_COPY`]). `TestField` reads only the copy's
    /// fields, which TestActionRunPageLinkFilterGroup.al:292-369 and R7138RunPageLinkCopyGet.al pin as Rec's. Every other
    /// method refuses.
    fn run_page_link_copy_op(
        &mut self,
        method: RecordOp,
        rec: &Rec,
        args: &mut [Value],
        why: &'static str,
    ) -> Result<Value, Signal> {
        let runs = match method {
            RecordOp::GetFilters => rec.borrow().filters.is_empty(),
            RecordOp::Count | RecordOp::Get => why == RUN_PAGE_LINK_COPY && rec.borrow().filters.is_empty(),
            RecordOp::FieldNo | RecordOp::IsTemporary | RecordOp::TableName | RecordOp::TableCaption => true,
            // TestField reads only the copy's own field, and the copy holds Rec's fields (al-runner issue 7444).
            RecordOp::TestField => true,
            _ => false,
        };
        if !runs {
            return unsupported(format!("Rec.{}() on {why}", method.name()));
        }
        let flag = rec.borrow_mut().unpinned_buffer.take();
        let r = self.record_op_inner(method, rec, args);
        rec.borrow_mut().unpinned_buffer = flag;
        r
    }

    /// A record method on a record with a key on a Media or MediaSet field, or on a by-value copy of it ([`MEDIA_KEY`]):
    /// and on a FlowField key ([`FLOWFIELD_KEY`], `why`): `CurrentKey` and the methods that do not depend on the key or the order
    /// of the rows run; `SetCurrentKey`, `Reset` and `Clear` give the record a key of its own again; every other method refuses.
    fn media_key_op(&mut self, method: RecordOp, rec: &Rec, args: &mut [Value], why: &str) -> Result<Value, Signal> {
        let rekeys = matches!(method, RecordOp::Reset | RecordOp::SetCurrentKey | RecordOp::Clear);
        let runs = rekeys || flowfield_key::runs(method, why, args) || self.auto_calc_on_lookup_key(method, rec, why);
        if !runs {
            return unsupported(format!("Record.{}(): {why}", method.name()));
        }
        let flag = rec.borrow_mut().unpinned_buffer.take();
        let r = self.record_op_inner(method, rec, args);
        // SetCurrentKey sets the mark again itself when the new key is on Media fields too.
        if !(rekeys && r.is_ok()) {
            rec.borrow_mut().unpinned_buffer = flag;
        }
        r
    }

    /// A record method: [`Session::record_op_inner`], with what it does to the snapshot `Record.IsDirty` compares with
    /// (isdirty.rs).
    pub(crate) fn record_op(&mut self, method: RecordOp, rec: &Rec, args: &mut [Value]) -> Result<Value, Signal> {
        self.probed_source_before(method, rec, args);
        let inserted = self.own_read_before(method, rec, args);
        let r = if self.coverage.profiler.is_some() {
            self.record_op_profiled(method, rec, args)
        } else {
            self.record_op_unprofiled(method, rec, args)
        };
        self.own_read_after(method, rec, inserted, matches!(r, Ok(Value::Boolean(true))));
        r
    }

    /// [`Session::record_op`] without the profiler's count (`profile.rs`).
    pub(crate) fn record_op_unprofiled(
        &mut self,
        method: RecordOp,
        rec: &Rec,
        args: &mut [Value],
    ) -> Result<Value, Signal> {
        let insert_from_al = std::mem::take(&mut self.tx.insert_from_al);
        // ADR-062: a record method of a restricted session, or of any session when the license limits a table (license_ceiling.rs).
        if self.permission_checks() {
            let from_al = std::mem::take(&mut self.permission_from_al);
            self.permission_gate_record(method, rec, args, from_al)?;
        }
        // A statement-form Insert of AL code is queued by the platform (`insert_buffer.rs`); a table with a Blob or AutoIncrement
        // field is not, and an Insert the platform makes itself stays out of the queue.
        let from_al = insert_from_al;
        let insert = match method {
            RecordOp::Insert if !from_al => InsertKind::Platform,
            RecordOp::Insert if !self.ret_used && self.insert_is_queued(rec) => InsertKind::Queued,
            _ => InsertKind::Other,
        };
        self.note_read_after_rollback(method, rec, args);
        if self.sql.active {
            let r = self.record_op_counted(method, rec, args);
            self.note_locking_read(method, rec, args, &r);
            self.guarded_body_record_op(method, rec, args, &r);
            return self.note_insert_buffer(method, rec, insert, r);
        }
        let r = self.record_op_tracked(method, rec, args);
        self.note_locking_read(method, rec, args, &r);
        self.guarded_body_record_op(method, rec, args, &r);
        let r = self.note_insert_buffer(method, rec, insert, r);
        if matches!(r, Ok(Value::Boolean(false)))
            && matches!(method, RecordOp::FindSet | RecordOp::FindFirst | RecordOp::FindLast)
        {
            self.sql_found_nothing(method, rec);
        }
        r
    }

    /// A find on `rec` found no row while the SQL counters are inactive (sqlcount.rs, `SqlCounters::found_nothing`).
    #[cold]
    #[inline(never)]
    fn sql_found_nothing(&mut self, method: RecordOp, rec: &Rec) {
        let read = self.sql_read_shape(rec);
        if read.readable {
            let table = rec.borrow().table;
            self.sql.note_found_nothing(method, table, read.filters);
        }
    }

    /// `record_op` while the SQL counters log record methods (`sqlcount.rs`).
    #[cold]
    #[inline(never)]
    fn record_op_counted(&mut self, method: RecordOp, rec: &Rec, args: &mut [Value]) -> Result<Value, Signal> {
        let entry = self.sql.enter(self.calls_made);
        let rows_before = self.sql_sized_rows(rec);
        let copy = self.sql_copy_shape(rec);
        let buffer_shape = self.sql_buffer_shape(rec, copy);
        let read = self.sql_read_shape(rec);
        let setup_key = self.table(rec).ok().map(|t| t.object as usize);
        let asked_before = setup_key.is_none_or(|k| self.events.global_setups.contains_key(&k));
        let r = self.record_op_tracked(method, rec, args);
        let first_write = match setup_key {
            Some(k) if !asked_before => self
                .events
                .global_setups
                .get(&k)
                .map(|s| (s.insert, s.modify || s.delete || s.rename || s.global_rename)),
            _ => None,
        };
        let dependency_code = self.frames.last().is_some_and(|&o| {
            let app = self.engine.program.objects[o].app as usize;
            self.engine.program.apps.get(app).is_some_and(|a| a.dependency)
        });
        let info = crate::sqlcount::OpInfo {
            buffer_shape,
            first_write,
            dependency_code,
            read,
        };
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
        self.sql.leave(
            entry,
            calls,
            method,
            temp,
            table,
            args,
            key,
            rows_before,
            copy,
            &r,
            info,
        );
        r
    }

    /// What the cost of one record method on `rec` depends on (`sqlcount::ReadShape`): a database record with no state set but
    /// filters in filter group 0 and load fields (the records of session-system/session/SkSqlBudget.al, writers TW and TX).
    fn sql_read_shape(&mut self, rec: &Rec) -> crate::sqlcount::ReadShape {
        let mut shape = crate::sqlcount::ReadShape::default();
        let filters = {
            let s = rec.borrow();
            let plain = !s.temporary
                && !s.crm
                && s.company.is_none()
                && !s.marked_only
                && s.read_isolation == 0
                && s.security_filtering == 0
                && s.key == 0
                && s.adhoc.is_none()
                && s.ascending
                && s.field_descending == 0
                && s.auto_calc.is_empty()
                && s.run_page_link.is_none()
                && s.unpinned_buffer.is_none()
                && s.filters.iter().all(|f| f.group == 0 && f.field < 64);
            if !plain {
                return shape;
            }
            s.filters.iter().fold(0u64, |m, f| m | (1 << f.field))
        };
        let Ok(t) = self.table(rec) else {
            return shape;
        };
        let Ok(auto) = self.auto_increment_field(&t) else {
            return shape;
        };
        let user: Vec<&al_data::FieldDef> = t.fields.iter().filter(|f| f.id < 2_000_000_000).collect();
        let plain_keys = t
            .keys
            .iter()
            .skip(1)
            .all(|k| k.iter().all(|&i| t.fields[i].id >= 2_000_000_000));
        // The table of SkSqlBudget.al:969-985 (TX: Code[20], Integer, Text[50], Integer keyed on the first two), filtered on its
        // key fields only (:987-1095).
        let tx_table = matches!(
            &user[..],
            [a, b, c, d] if [a, b, c, d].iter().all(|f| f.stored)
                && a.ty == RtType::Code(Some(20))
                && b.ty == RtType::Integer
                && c.ty == RtType::Text(Some(50))
                && d.ty == RtType::Integer
        ) && t.keys.first().is_some_and(|k| k[..] == [0, 1])
            && plain_keys
            && filters & !0b11 == 0;
        shape.readable = true;
        shape.measured = tx_table && auto.is_none();
        shape.auto_increment = auto.is_some();
        shape.filters = filters;
        shape
    }

    /// Whether `rec` is of a shape whose buffered Inserts and Count the corpus measured: the shape of
    /// [`Session::sql_copy_shape`] (session-system/session/SkSqlBudget.al:14-29), or a database table of one Integer key
    /// field and one Text field (SkSqlBudget.al:380-400, SqlCounters.al:7-20) and a record with no state set but at most one
    /// filter, on the key field (SkSqlBudget.al:474).
    fn sql_buffer_shape(&mut self, rec: &Rec, copy: crate::sqlcount::CopyShape) -> bool {
        if copy != crate::sqlcount::CopyShape::No {
            return true;
        }
        {
            let s = rec.borrow();
            let plain = !s.temporary
                && !s.crm
                && s.company.is_none()
                && !s.marked_only
                && s.read_isolation == 0
                && s.security_filtering == 0
                && s.key == 0
                && s.adhoc.is_none()
                && s.ascending
                && s.field_descending == 0
                && s.load.request.is_none()
                && s.auto_calc.is_empty()
                && s.run_page_link.is_none()
                && s.unpinned_buffer.is_none();
            let filters_ok = match &s.filters[..] {
                [] => true,
                [f] => f.field == 0 && f.group == 0,
                _ => false,
            };
            if !plain || !filters_ok {
                return false;
            }
        }
        let Ok(t) = self.table(rec) else {
            return false;
        };
        let user: Vec<&al_data::FieldDef> = t.fields.iter().filter(|f| f.id < 2_000_000_000).collect();
        matches!(
            &user[..],
            [k, v] if k.stored && v.stored && k.ty == RtType::Integer && matches!(v.ty, RtType::Text(_))
        ) && t.keys.first().is_some_and(|k| k[..] == [0])
            && t.keys
                .iter()
                .skip(1)
                .all(|k| k.iter().all(|&i| t.fields[i].id >= 2_000_000_000))
    }

    /// Whether `rec` is of the shape whose copy sizes the corpus measured (session-system/session/SkSqlBudget.al:14-29,
    /// `sqlcount::CopyShape`): a database table of the fields Code[20], Integer, Text[50], Integer, keyed on the first two,
    /// and a record with no state set but at most one filter, on the first field.
    fn sql_copy_shape(&mut self, rec: &Rec) -> crate::sqlcount::CopyShape {
        use crate::sqlcount::CopyShape;
        let filtered = {
            let s = rec.borrow();
            let plain = !s.temporary
                && !s.crm
                && s.company.is_none()
                && !s.marked_only
                && s.read_isolation == 0
                && s.security_filtering == 0
                && s.key == 0
                && s.adhoc.is_none()
                && s.ascending
                && s.field_descending == 0
                && s.load.request.is_none()
                && s.auto_calc.is_empty()
                && s.run_page_link.is_none()
                && s.unpinned_buffer.is_none();
            match &s.filters[..] {
                _ if !plain => return CopyShape::No,
                [] => false,
                [f] if f.field == 0 && f.group == 0 => true,
                _ => return CopyShape::No,
            }
        };
        let Ok(t) = self.table(rec) else {
            return CopyShape::No;
        };
        let user: Vec<&al_data::FieldDef> = t.fields.iter().filter(|f| f.id < 2_000_000_000).collect();
        let shape = matches!(
            &user[..],
            [a, b, c, d] if [a, b, c, d].iter().all(|f| f.stored)
                && a.ty == RtType::Code(Some(20))
                && b.ty == RtType::Integer
                && c.ty == RtType::Text(Some(50))
                && d.ty == RtType::Integer
        ) && t.keys.first().is_some_and(|k| k[..] == [0, 1])
            && t.keys
                .iter()
                .skip(1)
                .all(|k| k.iter().all(|&i| t.fields[i].id >= 2_000_000_000));
        match (shape, filtered) {
            (false, _) => CopyShape::No,
            (true, false) => CopyShape::Plain,
            (true, true) => CopyShape::Grouped,
        }
    }

    /// The rows of `rec`'s table, when the record and the table are of the shape whose SQL counter sizes the corpus measured
    /// (session-system/session/SqlCounters.al:7-20, :204-276): a database table of one Integer key field and one Text
    /// field, and a record with no filter, mark, key, order, load fields, auto-calculated fields, isolation or company set.
    fn sql_sized_rows(&mut self, rec: &Rec) -> Option<usize> {
        {
            let s = rec.borrow();
            let plain = !s.temporary
                && !s.crm
                && s.company.is_none()
                && s.filters.is_empty()
                && !s.marked_only
                && s.read_isolation == 0
                && s.security_filtering == 0
                && s.key == 0
                && s.adhoc.is_none()
                && s.ascending
                && s.field_descending == 0
                && s.load.request.is_none()
                && s.auto_calc.is_empty()
                && s.run_page_link.is_none()
                && s.unpinned_buffer.is_none();
            if !plain {
                return None;
            }
        }
        let t = self.table(rec).ok()?;
        let user: Vec<&al_data::FieldDef> = t.fields.iter().filter(|f| f.id < 2_000_000_000).collect();
        let shape = matches!(
            &user[..],
            [k, v] if k.stored && v.stored && k.ty == RtType::Integer && matches!(v.ty, RtType::Text(_))
        ) && t.keys.first().is_some_and(|k| k[..] == [0])
            && t.keys
                .iter()
                .skip(1)
                .all(|k| k.iter().all(|&i| t.fields[i].id >= 2_000_000_000));
        if !shape {
            return None;
        }
        let cx = self.cx_for(rec, &t).ok()?;
        self.store_for(rec).count(&cx, &t, &[]).ok()
    }

    /// A record method AL code calls (a Record or a RecordRef): [`Session::record_op`], with the global filter group checked
    /// (`global_group_check`). The runtime's own record calls (page refills, probes) are not AL code and pass straight on.
    pub(crate) fn record_op_from_al(
        &mut self,
        method: RecordOp,
        rec: &Rec,
        args: &mut [Value],
    ) -> Result<Value, Signal> {
        let (table, group) = {
            let r = rec.borrow();
            (r.table, r.filter_group)
        };
        if matches!(method, RecordOp::SetFilter | RecordOp::SetRange) && group == 1 {
            return self.set_global_group_filter(rec, table, method, args);
        }
        if !self.global_group1.is_empty() {
            self.global_group_check(rec, table, method.name())?;
        }
        // ADR-062: a denial of a method AL code calls raises BC's error where its shape is pinned.
        if self.permission_checks() {
            self.permission_from_al = true;
        }
        self.record_op_of_al(method, rec, args)
    }

    /// The filters of `rec` in filter group 1, as text to compare.
    fn group1_signature(rec: &Rec) -> String {
        let r = rec.borrow();
        let mut v: Vec<String> = r
            .filters
            .iter()
            .filter(|f| f.group == 1)
            .map(|f| format!("{}={:?}", f.field, f.expr))
            .collect();
        v.sort();
        v.join(";")
    }

    /// SetFilter or SetRange in filter group 1 (the global group) on a database record: BC keeps the filter on every record
    /// of the table for the rest of the session, through Reset and Clear (types/text-code-char/
    /// P145PageRunView.al:232-235, SfSeed :236-268, seed_before = 4 at :283). The session notes what group 1 holds now;
    /// `global_group_check` refuses the records that do not carry it.
    #[cold]
    #[inline(never)]
    fn set_global_group_filter(
        &mut self,
        rec: &Rec,
        table: u32,
        method: RecordOp,
        args: &mut [Value],
    ) -> Result<Value, Signal> {
        if rec.borrow().temporary {
            return unsupported(format!(
                "{} in filter group 1 (the global group) on a temporary record (how BC scopes it is not pinned)",
                method.name()
            ));
        }
        self.global_group_check(rec, table, method.name())?;
        let r = self.record_op(method, rec, args)?;
        let sig = Self::group1_signature(rec);
        if sig.is_empty() {
            self.global_group1.remove(&table);
        } else {
            self.global_group1.insert(table, sig);
        }
        Ok(r)
    }

    /// A record of a table whose filter group 1 (the global group) holds a filter another record set: BC applies that
    /// filter to this record too (P145PageRunView.al:232-235), the runtime keeps filters on the record that set them.
    /// Answered when the record carries the same group 1 filters (the setter, or a copy of it such as the record of
    /// Page.Run(Page, Rec)); refused otherwise.
    pub(crate) fn global_group_check(&self, rec: &Rec, table: u32, what: &str) -> Result<(), Signal> {
        let Some(global) = self.global_group1.get(&table) else {
            return Ok(());
        };
        if what == "FilterGroup" {
            return Ok(());
        }
        if !rec.borrow().temporary && Self::group1_signature(rec) == *global {
            return Ok(());
        }
        unsupported(format!(
            "{what} on a record of a table with a filter in filter group 1 (the global group) that another record set: BC \
             applies it to every record of the table for the session, the runtime does not"
        ))
    }

    /// What a filter does in a negative filter group other than -1, the OR group, is not pinned (only reading the group back is:
    /// P68F_FilterGroup.al RecordRef_Setter_Negative_And_Large).
    fn refuse_filter_in_unpinned_group(method: RecordOp, rec: &Rec) -> Result<(), Signal> {
        let sets_filter = matches!(
            method,
            RecordOp::SetRange
                | RecordOp::SetFilter
                | RecordOp::SetView
                | RecordOp::SetRecFilter
                | RecordOp::CopyFilter
                | RecordOp::CopyFilters
        );
        let group = rec.borrow().filter_group;
        if sets_filter && group < 0 && group != al_builtins::filter::OR_GROUP {
            return unsupported(format!(
                "{} in filter group {group}, a negative group other than -1 (not pinned)",
                method.name()
            ));
        }
        Ok(())
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

/// What [`Session::stale_verdict`] decided for a write through a buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StaleVerdict {
    /// The buffer is not stale.
    Fresh,
    /// Stale, and written all the same.
    Written,
    /// Stale: the write fails with DB:RecordChanged.
    Refused,
}

/// `s` with the text `GetLastErrorText(true)` answers for it, when it is an AL error.
fn redact_error(s: Signal, redacted: String) -> Signal {
    match s {
        Signal::Error(mut e) => {
            e.origin = al_builtins::native::ErrorOrigin::Redacted(redacted);
            Signal::Error(e)
        }
        other => other,
    }
}

/// The field values of a fresh record of one table (`Session::initial_fields`): the values to copy, and the positions of
/// those that are shared objects and are built fresh for each record instead.
pub(crate) struct InitialFields {
    values: Vec<Value>,
    fresh: Rc<[usize]>,
}

/// A copy of `rec`'s state with `fields` as its field values. Rec's own fields are left out of the copy instead of being
/// copied only to be replaced (issue 6133: NP Retail's tables are wide).
pub(crate) fn state_with_fields(rec: &Rec, fields: Vec<Value>) -> RecordState {
    let mut st = match rec.try_borrow_mut() {
        Ok(mut r) => {
            let own = std::mem::take(&mut r.fields);
            let st = r.clone();
            r.fields = own;
            st
        }
        Err(_) => rec.borrow().clone(),
    };
    st.fields = fields;
    st
}
