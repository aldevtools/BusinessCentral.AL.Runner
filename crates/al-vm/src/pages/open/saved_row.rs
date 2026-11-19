//! The Object Options row a report request page with SaveValues keeps what an OK confirmed in (al-runner issue 8250), and
//! the values the next run of the report reads from it. Corpus PR 1462 (merged on the corpus master, not pinned yet),
//! tests/al-language/reports/ReportObjectOptionsSaveValues.al, measures it; until it is pinned it runs through assumption
//! `report-saved-values-object-options` in the measured shapes, and the shapes it did not measure refuse. The values
//! themselves stay in `PageClients::saved_values` (`values.rs`); the row decides whether and which of them a run shows.

use super::*;
use crate::platform::{blank, pos};
use al_builtins::assumptions::{Assumption, assume};

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

/// What a SaveValues page stored (`PageClients::saved_values`): (owner, global slot, value).
type Stored = Vec<(Option<usize>, usize, Value)>;

/// Whether the values a report's request page stored stand in an Object Options row.
#[derive(Clone, Debug)]
pub(crate) enum SavedRow {
    /// The runtime wrote the row: its Option Data text, and the store's change counter after the write.
    Written(String, u64),
    /// The values are kept outside the rows (a save outside the measured shapes); the store's change counter after that
    /// save. A change to the Object Options rows since is refused at the next read.
    Unmirrored(u64),
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
        match self.measured_save(st, saved, by_ok) {
            Some(text) => {
                let counter = self.store.change_counter();
                self.pages.saved_row_pending = Some((object, text, self.tx.tx_world, counter));
            }
            None => {
                let counter = self.store.change_counter();
                self.pages.saved_rows.insert(object, SavedRow::Unmirrored(counter));
            }
        }
    }

    /// When the run whose request page stored values returns (`reports.rs` `report_in_world`, after its world ended):
    /// `commits`, the run commits, as a run that enters its world does (record/TestTxModelReportRun.al:177-196): the row
    /// is written just before that commit, so it is committed with the save. A run that does not commit writes no row; its
    /// values are kept outside the rows, as before.
    pub(crate) fn settle_saved_row(&mut self, commits: bool) -> Result<(), Signal> {
        // The run that just returned is the one whose request page asked for the row.
        let world = self.tx.tx_world + 1;
        let Some((object, text, _, saved_at)) = self.pages.saved_row_pending.take_if(|(_, _, w, _)| *w == world) else {
            return Ok(());
        };
        let t = match self.table_def(OBJECT_OPTIONS) {
            Some(t) if commits => t,
            _ => {
                self.pages.saved_rows.insert(object, SavedRow::Unmirrored(saved_at));
                return Ok(());
            }
        };
        assume(Assumption::ReportSavedValuesObjectOptions);
        let mut row = blank(&t);
        let key = self.saved_row_key(&t, object as usize);
        for (id, v) in &key {
            row[pos(&t, *id)] = v.clone();
        }
        row[pos(&t, F_CREATED_BY)] = key[3].1.clone();
        let blob = al_builtins::stream::new_blob();
        blob.borrow_mut().bytes = text.as_bytes().to_vec();
        row[pos(&t, F_OPTION_DATA)] = Value::Blob(blob);
        let cx = self.cx.clone();
        let pk: Vec<Value> = key.iter().map(|(_, v)| v.clone()).collect();
        if self.store.get(&cx, &t, &pk)?.is_some() {
            self.store.modify(&cx, &t, row)?;
        } else {
            self.store.insert(&cx, &t, row)?;
        }
        // The runtime's own write is no change by AL to the rows of the other saves.
        let counter = self.store.change_counter();
        for entry in self.pages.saved_rows.values_mut() {
            match entry {
                SavedRow::Unmirrored(c) | SavedRow::Written(_, c) => *c = counter,
            }
        }
        self.pages.saved_rows.insert(object, SavedRow::Written(text, counter));
        Ok(())
    }

    /// The Option Data text of a save in the shapes ReportObjectOptionsSaveValues.al measured, `None` for any other save. Each
    /// fact is required by equality: the request page of `Report.RunModal` (:106-112 `RunSet`), closed by OK (:394-398; Cancel writes
    /// no row, :374), no write transaction open (`RunSet` commits first), and every stored control bound to a Text global of the
    /// report itself (:27 `SaveValues`, :39 `SomeFilter: Text`).
    fn measured_save(&mut self, st: &St, saved: &[(Option<usize>, usize, Value)], by_ok: bool) -> Option<String> {
        let request = self.is_request_page(st);
        let run_modal =
            self.report.run_modal && self.report.report_preview_pinned.last() == Some(&crate::state::ReportStart::Run);
        let committed = matches!(self.in_write_transaction(), Ok(false));
        let texts = !saved.is_empty()
            && saved
                .iter()
                .all(|(owner, _, v)| owner.is_none() && matches!(v, Value::Text(_)));
        if !(request && by_ok && run_modal && committed && texts) {
            return None;
        }
        Some(self.option_data(st.borrow().object as usize, saved))
    }

    /// The Option Data text the runtime writes for `saved`: the parameters form of `Report.RunRequestPage`, one Field per variable.
    /// Only that it holds each value as UTF-8 text is measured (:197 `Option Data holds the value (UTF-8)`, :305-313 an edit of the
    /// value inside it); the rest of the form is the runtime's own.
    fn option_data(&self, object: usize, saved: &[(Option<usize>, usize, Value)]) -> String {
        let o = &self.engine.program.objects[object];
        let fields: String = saved
            .iter()
            .map(|(_, slot, v)| {
                let name = o.globals.get(*slot).map_or("", |g| g.name.as_str());
                let text = match v {
                    Value::Text(t) => t.to_string(),
                    _ => String::new(),
                };
                crate::pages::request::option_field(name, &text)
            })
            .collect();
        format!(
            "{}<ReportParameters name=\"{}\" id=\"{}\"><Options>{fields}</Options></ReportParameters>",
            crate::pages::request_options::PARAMETERS_DECLARATION,
            crate::pages::request::xml_escape(&o.name),
            o.id.unwrap_or(0)
        )
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
        let (SavedRow::Unmirrored(counter) | SavedRow::Written(_, counter)) = entry;
        let changed = self.store.tables_changed_since(counter).contains(&t.object);
        // AL changed no row since the save: the stored values, as before. A row gone without a change, as when the
        // rollback that ends an isolated test puts the store back (`Checkpoint::restore`), is this case too.
        if !changed {
            return Ok(saved);
        }
        let SavedRow::Written(written, _) = entry else {
            return refuse("a change to the rows that follows a save the runtime keeps outside them");
        };
        let key = self.saved_row_key(&t, object as usize);
        let cx = self.cx.clone();
        let pk: Vec<Value> = key.iter().map(|(_, v)| v.clone()).collect();
        let row = self.store.get(&cx, &t, &pk)?;
        let data = row.map(|r| match &r[pos(&t, F_OPTION_DATA)] {
            Value::Blob(b) => String::from_utf8_lossy(&b.borrow().bytes).into_owned(),
            _ => String::new(),
        });
        let stored = saved.as_ref().map(|s| self.option_data(object as usize, s));
        // The row as the runtime wrote it, over the values it was written for: what was stored (:218, :277).
        if data.is_some() && data == stored {
            return Ok(saved);
        }
        // The read the probe measured: the request page of RunModal with no write transaction open (:114-121 `RunRead`).
        let run_modal =
            self.report.run_modal && self.report.report_preview_pinned.last() == Some(&crate::state::ReportStart::Run);
        let committed = matches!(self.in_write_transaction(), Ok(false));
        if !(run_modal && committed) {
            return refuse(
                "a deleted, moved or edited row, read by a run other than RunModal or in a write transaction",
            );
        }
        match data {
            // No row under this session's key: the page's own values (:232 delete, :259 another report keeps its row, :321
            // another User Name, :339 another Company Name, :357 another Parameter Name, :374 after a Cancel).
            None => {
                assume(Assumption::ReportSavedValuesObjectOptions);
                Ok(None)
            }
            // An edit of the Option Data the runtime wrote for the values still stored (:294).
            Some(edited) => {
                let edit_of_written = saved.as_ref().filter(|_| stored.as_deref() == Some(written.as_str()));
                match edit_of_written.and_then(|s| self.edited_texts(object as usize, s, &edited)) {
                    Some(texts) => {
                        assume(Assumption::ReportSavedValuesObjectOptions);
                        Ok(Some(texts))
                    }
                    None => refuse("an edit of the row's Option Data that changes its form"),
                }
            }
        }
    }

    /// `saved` with the texts an edited Option Data gives each variable, `None` when the edit changed anything but the values: the
    /// same declaration, root and Field names in the same order.
    fn edited_texts(&self, object: usize, saved: &[(Option<usize>, usize, Value)], edited: &str) -> Option<Stored> {
        let empty: Vec<_> = saved.iter().map(|(o, s, _)| (*o, *s, Value::Text("".into()))).collect();
        let blank = self.option_data(object, &empty);
        let (head, tail) = blank.split_once("<Options>")?;
        let options = edited.strip_prefix(head)?.strip_prefix("<Options>")?;
        let options = options
            .strip_suffix(tail.split_once("</Options>")?.1)?
            .strip_suffix("</Options>")?;
        let fields = crate::pages::request_options::option_fields(options).ok()?;
        let o = &self.engine.program.objects[object];
        if fields.len() != saved.len() {
            return None;
        }
        saved
            .iter()
            .zip(fields)
            .map(|((owner, slot, _), (name, text))| {
                let own = o.globals.get(*slot).map_or("", |g| g.name.as_str());
                (name == own).then(|| (*owner, *slot, Value::Text(text.as_str().into())))
            })
            .collect()
    }
}
