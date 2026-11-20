//! The Object Options row a report request page with SaveValues keeps what an OK confirmed in (al-runner issue 8250), and
//! the values the next run of the report reads from it. Pinned by corpus tests/al-language/reports/
//! ReportObjectOptionsSaveValues.al in the shapes it measures (`measured_save`); every other save keeps its values outside
//! the rows, as before, and a later change to the rows refuses its next read. The values themselves stay in
//! `PageClients::saved_values` (`values.rs`); the row decides whether and which of them a run shows.

use super::*;
use crate::pages::pinned_body::{Names, report_globals_are};
use crate::platform::{blank, pos};
use std::sync::Arc;

/// The platform table "Object Options".
const OBJECT_OPTIONS: u32 = 2_000_000_196;
/// The Parameter Name of the row an OK writes (ReportObjectOptionsSaveValues.al:203 `Parameter Name`).
const LAST_USED: &str = "Last used options and filters";
const F_PARAMETER_NAME: u32 = 1;
const F_OBJECT_ID: u32 = 2;
const F_OBJECT_TYPE: u32 = 3;
const F_COMPANY_NAME: u32 = 4;
const F_USER_NAME: u32 = 5;
const F_OPTION_DATA: u32 = 6;
const F_CREATED_BY: u32 = 9;
/// The system fields of the written row the probe does not read: SystemCreatedAt, SystemCreatedBy, SystemModifiedAt and
/// SystemModifiedBy.
const UNREAD_STAMPS: [u32; 4] = [2_000_000_001, 2_000_000_002, 2_000_000_003, 2_000_000_004];

/// The declared globals of the probe report, one Text (:37-38), under any name (`pinned_body.rs`).
const PROBE_GLOBALS: &[&str] = &["var «V»: Text"];

/// What a SaveValues page stored (`PageClients::saved_values`): (owner, global slot, value).
type Stored = Vec<(Option<usize>, usize, Value)>;

/// Whether the values a report's request page stored stand in an Object Options row.
#[derive(Clone, Debug)]
pub(crate) struct SavedRow {
    /// The runtime wrote the row: the mark its Option Data carries (`stream::SAVED_OPTION_DATA`, one allocation per write, so a
    /// copy of the row keeps it and no other row has it). `None`: the values are kept outside the rows.
    written: Option<Arc<str>>,
    /// The columns the runtime wrote other than Option Data and the system fields, as (position, value): the key, Created By,
    /// Public Visible and Temporary (:178-214). Empty when `written` is `None`.
    columns: Vec<(usize, Value)>,
    /// The store's change counter after the save, or after the runtime's latest own write of a row.
    since: u64,
    /// AL changed the Object Options rows after the save, before a later own write moved `since`.
    al_changed: bool,
}

impl Session {
    /// After a request page stored `saved` (`store_page_values`): the Object Options row of the save, written when the
    /// report run commits (`settle_saved_row`). `by_ok`: the OK action closed the page.
    pub(in crate::pages) fn mirror_saved_row(&mut self, st: &St, saved: &[(Option<usize>, usize, Value)], by_ok: bool) {
        // Nothing in the program can read the rows: the stored values are all there is.
        if self.table_def(OBJECT_OPTIONS).is_none() {
            return;
        }
        // The request page of RunRequestPage stores nothing (reports/TestReportRequestPageSaveValues.al:272
        // SourceReport_SaveValues_RunRequestPageReturnsTheValueWithoutSavingIt; `report_request_only` puts the values back).
        if self.report.report_preview_pinned.last() == Some(&crate::state::ReportStart::RequestOnly) {
            return;
        }
        let object = st.borrow().object;
        let counter = self.store.change_counter();
        if self.measured_save(st, saved, by_ok) {
            self.pages.saved_row_pending.push((object, self.tx.tx_world, counter));
        } else {
            self.pages.saved_rows.insert(
                object,
                SavedRow {
                    written: None,
                    columns: Vec::new(),
                    since: counter,
                    al_changed: false,
                },
            );
        }
    }

    /// When the run whose request page stored values returns (`reports.rs` `report_in_world`, after its world ended):
    /// `commits`, the run commits, as a run that enters its world does (record/TestTxModelReportRun.al:177-196): the row
    /// is written just before that commit, so it is committed with the save. A run that does not commit, or whose rows AL
    /// changed since the OK, writes no row; its values are kept outside the rows, as before.
    pub(crate) fn settle_saved_row(&mut self, commits: bool) -> Result<(), Signal> {
        // The run that just returned is the one whose request page asked for the row.
        let world = self.tx.tx_world + 1;
        let Some(at) = self.pages.saved_row_pending.iter().rposition(|(_, w, _)| *w == world) else {
            return Ok(());
        };
        let (object, _, saved_at) = self.pages.saved_row_pending.remove(at);
        let kept_outside = SavedRow {
            written: None,
            columns: Vec::new(),
            since: saved_at,
            al_changed: false,
        };
        let t = match self.table_def(OBJECT_OPTIONS) {
            Some(t) if commits && !self.store.tables_changed_since(saved_at).contains(&t.object) => t,
            _ => {
                self.pages.saved_rows.insert(object, kept_outside);
                return Ok(());
            }
        };
        // The written row (:178-214): its key, Created By, Public Visible and Temporary No (the blank row's), Option Data.
        let mut row = blank(&t);
        let key = self.saved_row_key(&t, object as usize);
        for (id, v) in &key {
            row[pos(&t, *id)] = v.clone();
        }
        row[pos(&t, F_CREATED_BY)] = key[3].1.clone();
        // Only that Option Data holds the value as UTF-8 text is pinned (:197), not the text: every look at it refuses.
        let mark: Arc<str> = Arc::from(al_builtins::stream::SAVED_OPTION_DATA);
        let blob = al_builtins::stream::new_blob();
        blob.borrow_mut().render_pending = Some(mark.clone());
        row[pos(&t, F_OPTION_DATA)] = Value::Blob(blob);
        let columns: Vec<(usize, Value)> = t
            .fields
            .iter()
            .enumerate()
            .filter(|(_, f)| f.id < 2_000_000_000 && f.id != F_OPTION_DATA)
            .map(|(p, _)| (p, row[p].clone()))
            .collect();
        let cx = self.cx.clone();
        let pk: Vec<Value> = key.iter().map(|(_, v)| v.clone()).collect();
        let standing = self.store.get(&cx, &t, &pk)?.is_some();
        // The first write gets a SystemId, as every stored row has (platform/seed.rs `insert_seed`). A second OK while a row
        // stands under the key is not measured (every probe test deletes the rows first, :232 writes after a delete): that
        // row's SystemId is Unknown, so a read refuses. The stamps are not pinned for either.
        if let Some(p) = t.fields.iter().position(|f| f.id == crate::hostvalues::SYSTEM_ID) {
            row[p] = match standing {
                false => Value::Guid(self.fresh_system_id()),
                true => crate::platform::unknown(
                    &t,
                    crate::hostvalues::SYSTEM_ID,
                    "an Object Options row saved over again",
                ),
            };
        }
        for id in UNREAD_STAMPS {
            if t.fields.iter().any(|f| f.id == id) {
                row[pos(&t, id)] =
                    crate::platform::unknown(&t, id, "the Object Options row of saved request page values");
            }
        }
        // AL's changes to the rows before this write still count for the other saves.
        for entry in self.pages.saved_rows.values_mut() {
            entry.al_changed |= self.store.tables_changed_since(entry.since).contains(&t.object);
        }
        if standing {
            self.store.modify(&cx, &t, row)?;
        } else {
            self.store.insert(&cx, &t, row)?;
        }
        let counter = self.store.change_counter();
        for entry in self.pages.saved_rows.values_mut() {
            entry.since = counter;
        }
        self.pages.saved_rows.insert(
            object,
            SavedRow {
                written: Some(mark),
                columns,
                since: counter,
                al_changed: false,
            },
        );
        Ok(())
    }

    /// Whether a save is in the shapes ReportObjectOptionsSaveValues.al measures. Each fact is required by equality: the
    /// request page of the static `Report.RunModal(Id)` with no other argument and no substitution (:106-112 `RunSet`), closed
    /// by OK (:394-398; Cancel writes no row, :374), no write transaction open (`RunSet` commits first), a ProcessingOnly
    /// report with no data item, no report extension and no trigger or procedure (:20-40), its globals exactly the probe
    /// report's one Text (`PROBE_GLOBALS`), and one stored control, bound to that global (:33, :39).
    fn measured_save(&mut self, st: &St, saved: &[(Option<usize>, usize, Value)], by_ok: bool) -> bool {
        let object = st.borrow().object as usize;
        let o = &self.engine.program.objects[object];
        let report = o.routines.is_empty()
            && o.report.as_ref().is_some_and(|r| {
                r.processing_only && r.items.is_empty() && r.extensions.is_empty() && r.output.is_empty()
            });
        let one_text = matches!(saved, [(None, slot, Value::Text(_))]
            if o.globals.get(*slot).is_some_and(|g| g.ty == RtType::Text(None)));
        let globals = report_globals_are(&self.engine.program, object, PROBE_GLOBALS, &mut Names::default());
        self.is_request_page(st)
            && by_ok
            && report
            && one_text
            && globals
            && self.static_run_modal_read()
            && matches!(self.in_write_transaction(), Ok(false))
    }

    /// The run is the static `Report.RunModal(Id)` the probe makes (:111, :119): the report id is its only argument and no
    /// SubstituteReport subscriber swapped the report (`ReportState::run_bare`).
    fn static_run_modal_read(&self) -> bool {
        self.report.run_modal
            && self.report.run_bare
            && self.report.report_preview_pinned.last() == Some(&crate::state::ReportStart::Run)
    }

    /// The primary key of the row of report `object` for this session: Parameter Name, Object ID, Object Type Report, User Name =
    /// UserId and Company Name = CompanyName (:203-207), as (field id, value).
    fn saved_row_key(&self, t: &al_data::TableDef, object: usize) -> [(u32, Value); 5] {
        let members: std::sync::Arc<[String]> = match &t.fields[pos(t, F_OBJECT_TYPE)].ty {
            RtType::Option(m) => m.clone(),
            _ => std::sync::Arc::from(Vec::new()),
        };
        let report = members.iter().position(|n| n == "Report").unwrap_or(0) as i32;
        let user = self.host.identity.user_id().to_uppercase();
        [
            (F_PARAMETER_NAME, Value::Text(LAST_USED.into())),
            (
                F_OBJECT_ID,
                Value::Integer(self.engine.program.objects[object].id.unwrap_or(0) as i32),
            ),
            (F_OBJECT_TYPE, Value::Option(report, members)),
            (F_USER_NAME, Value::Code(user.as_str().into())),
            (F_COMPANY_NAME, Value::Text(self.cx.company.to_string().into())),
        ]
    }

    /// Before a request page shows what `saved` holds (`restore_page_values`): what the Object Options row of the save leaves of
    /// it. `None`: nothing is restored.
    pub(in crate::pages) fn saved_through_row(
        &mut self,
        st: &St,
        saved: Option<Stored>,
    ) -> Result<Option<Stored>, Signal> {
        let object = st.borrow().object;
        let (Some(entry), Some(t)) = (
            self.pages.saved_rows.get(&object).cloned(),
            self.table_def(OBJECT_OPTIONS),
        ) else {
            return Ok(saved);
        };
        let name = &self.engine.program.objects[object as usize].name;
        let refuse = |what: &str| {
            unsupported(format!(
                "the request page of report \"{name}\" after {what} (Object Options rows of saved values, not measured)"
            ))
        };
        // AL changed no row since the save: the stored values, as before. The store tracks changes per table, so a change to
        // any Object Options row counts. A row gone without a change, as when the rollback that ends an isolated test puts the
        // store back (`Checkpoint::restore`), is this case too: main's result, not measured.
        if !entry.al_changed && !self.store.tables_changed_since(entry.since).contains(&t.object) {
            return Ok(saved);
        }
        let Some(mark) = entry.written else {
            return refuse("a change to the rows that follows a save the runtime keeps outside them");
        };
        let key = self.saved_row_key(&t, object as usize);
        let cx = self.cx.clone();
        let pk: Vec<Value> = key.iter().map(|(_, v)| v.clone()).collect();
        let row = self.store.get(&cx, &t, &pk)?;
        let carries_mark = |r: &al_data::Row| match &r[pos(&t, F_OPTION_DATA)] {
            Value::Blob(b) => b
                .borrow()
                .render_pending
                .as_ref()
                .is_some_and(|m| Arc::ptr_eq(m, &mark)),
            _ => false,
        };
        let as_written = |r: &al_data::Row| {
            carries_mark(r)
                && entry
                    .columns
                    .iter()
                    .all(|(p, v)| Self::same_values(std::slice::from_ref(&r[*p]), std::slice::from_ref(v)))
        };
        match row {
            // The row the runtime wrote, with every column as written, also after it was deleted and inserted again: what
            // was stored (:218, :277).
            Some(r) if as_written(&r) => Ok(saved),
            // No row under this session's key, read by the static RunModal with no write transaction open (:114-121
            // `RunRead`): the page's own values (:232 delete, :259 another report keeps its row, :321 another User Name, :339
            // another Company Name, :357 another Parameter Name, :374 after a Cancel).
            None if self.static_run_modal_read() && matches!(self.in_write_transaction(), Ok(false)) => Ok(None),
            None => {
                refuse("a deleted or moved row, read by a run other than the static RunModal or in a write transaction")
            }
            Some(_) => refuse("an edit of the row, or another row under its key"),
        }
    }
}
