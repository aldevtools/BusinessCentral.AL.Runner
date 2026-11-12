//! The platform table Tenant Report Layout (2000000232), a stored table with these pinned limits
//! (reports/runtime/Y6TenantReportLayoutRows.al and TrlResourceRows.al, corpus master, BC 27, 28 and 29):
//!
//! - A fresh tenant holds no row (TrlResourceRows.al:84-97 FreshTenant_AllRows: Count 0 and an empty FindSet, no filter), so
//!   the reads (Find*, Next, Count, IsEmpty, Get) and Delete or DeleteAll answer from the rows the session inserted.
//!   ModifyAll stays limited to a filter on the Report ID of one report of an app that is not Microsoft's; Modify, Rename and
//!   the other writes are not pinned: refused.
//! - Insert of a row whose Layout holds a stream, for a report that has layouts, with Layout Format RDLC, Word, Excel or
//!   Custom: true for Microsoft report 1305 and own reports (TrlResourceRows.al:228-388, Insert_*); the row reads back with
//!   a blank Company Name, MIME Type and Description, the null App ID, and Layout.HasValue Yes. A second Insert of the key
//!   answers false (Y6TenantReportLayoutRows.al:76). Custom with an empty or unknown Custom Type reads back as Layout Format
//!   External (the table's own name for the option's fourth member) with the bytes unchanged (:360-388); RDLC, Word and Excel store bytes BC rewrites (the length differs from
//!   the file and by version, :243-343): the Layout media is marked and any look at its bytes is refused.
//! - Every other Insert is refused: a blank row and one with an empty layout raise errors whose text the corpus pins only in
//!   part (ScReportMetadataViews.al:95-134, Y6TenantReportLayoutRows.al:457-467), a processing-only or unknown report
//!   (:474-509), and layout bytes that are no XML (RDLC) or no zip package (Word, Excel), which BC answers with an uncatchable
//!   exception (Y6TenantReportLayoutRows.al:11-13).

use crate::Session;
use al_builtins::native::{Signal, unsupported};
use al_builtins::record_op::RecordOp;
use al_builtins::value::{RecordState, Value};
use al_data::TableDef;
use std::cell::RefCell;
use std::rc::Rc;

type Rec = Rc<RefCell<RecordState>>;

const TENANT_REPORT_LAYOUT: u32 = 2_000_000_232;
/// The `Layout Format` option of an RDLC layout (the System app's table: `RDLC,Word,Excel,Custom`).
const RDLC: i32 = 0;
const WORD: i32 = 1;
const EXCEL: i32 = 2;
const CUSTOM: i32 = 3;

impl Session {
    pub(crate) fn tenant_report_layout_check(
        &mut self,
        t: &TableDef,
        rec: &Rec,
        method: RecordOp,
    ) -> Result<(), Signal> {
        if rec.borrow().temporary
            || self.install_scope
            || self.engine.program.objects[t.object as usize].id != Some(TENANT_REPORT_LAYOUT)
        {
            return Ok(());
        }
        let refuse = |what: &str| {
            unsupported(format!(
                "{what} on Tenant Report Layout: the rows a fresh tenant holds there and this operation are not pinned (only reads, DeleteAll and an Insert of the report's own RDLC layout, for a report of the app under test)"
            ))
        };
        match method {
            RecordOp::Find
            | RecordOp::FindFirst
            | RecordOp::FindLast
            | RecordOp::FindSet
            | RecordOp::Next
            | RecordOp::Count
            | RecordOp::IsEmpty
            | RecordOp::Get
            | RecordOp::Delete
            | RecordOp::DeleteAll => Ok(()),
            RecordOp::ModifyAll => {
                if self.own_report_filter(t, rec).is_some() {
                    Ok(())
                } else {
                    refuse(method.name())
                }
            }
            RecordOp::Insert => self.tenant_report_layout_insert(t, rec),
            m if m.writes() || matches!(m, RecordOp::GetBySystemId | RecordOp::CalcSums | RecordOp::CountApprox) => {
                refuse(m.name())
            }
            _ => Ok(()),
        }
    }

    /// The report object whose id a filter of group 0 on `Report ID` keeps alone, when it belongs to an app that is not
    /// Microsoft's, and no filter of another group is set.
    fn own_report_filter(&self, t: &TableDef, rec: &Rec) -> Option<usize> {
        let pos = t.fields.iter().position(|f| f.id == 1)?;
        let r = rec.borrow();
        if r.filters.iter().any(|f| f.group != 0) || r.marked_only {
            return None;
        }
        let id = r
            .filters
            .iter()
            .filter(|f| f.field == pos)
            .find_map(|f| crate::platform::mat::single_value_of(&f.expr))?;
        let Value::Integer(id) = id else { return None };
        self.own_report(id)
    }

    /// Report `id`, when an app that is not Microsoft's declares it.
    fn own_report(&self, id: i32) -> Option<usize> {
        let program = &self.engine.program;
        let object = program
            .objects
            .iter()
            .position(|o| o.report.is_some() && o.id == Some(id as u32))?;
        let app = program.apps.get(program.objects[object].app as usize)?;
        (!app.publisher.eq_ignore_ascii_case("Microsoft")).then_some(object)
    }

    fn tenant_report_layout_insert(&mut self, t: &TableDef, rec: &Rec) -> Result<(), Signal> {
        const WHAT: &str = "Insert on Tenant Report Layout of";
        let refuse = |why: &str| unsupported(format!("{WHAT} {why} (not pinned)"));
        let field = |id: u32| t.fields.iter().position(|f| f.id == id);
        let (Some(report), Some(layout), Some(format)) = (field(1), field(7), field(8)) else {
            return refuse("a table without the pinned fields");
        };
        let (report_id, media, format) = {
            let r = rec.borrow();
            (
                r.fields[report].clone(),
                r.fields[layout].clone(),
                r.fields[format].clone(),
            )
        };
        let (Value::Integer(report_id), Value::Option(format, _)) = (report_id, format) else {
            return refuse("a row without a report id or layout format");
        };
        if !(RDLC..=CUSTOM).contains(&format) {
            return refuse("a row with a Layout Format other than RDLC, Word, Excel and Custom");
        }
        let program = &self.engine.program;
        let known = program
            .objects
            .iter()
            .find(|o| o.report.is_some() && o.id == u32::try_from(report_id).ok());
        match known.and_then(|o| o.report.as_ref()) {
            Some(r) if !r.processing_only => {}
            Some(_) => return refuse("a processing-only report"),
            None => return refuse("a report the app and its dependencies do not declare"),
        }
        let Value::Guid(id) = media else {
            return refuse("a row without a Layout");
        };
        if id == [0; 16] {
            return refuse("a row without a Layout");
        }
        let mut objects = self.media.objects.borrow_mut();
        let Some(object) = objects.get_mut(&id) else {
            return refuse("a Layout the session does not hold");
        };
        // A second Insert of the same row finds the mark the first one put on the media.
        let marked = object.pending.as_deref() == Some(al_builtins::stream::TENANT_LAYOUT_STORED);
        // The layout file the compiler generated, of the row's format, as `Report.<Kind>Layout` gave it: the Insert succeeds
        // (reports/layouts-saveas/UserDefinedWordExcelLayoutRows.al:61, :95 `insert Yes`; UserDefinedLayoutAppIdAndNames.al:180
        // for RDLC, Word and Excel), and the stored media refuses any look like another stored layout.
        let kind = match format {
            RDLC => "RDLC",
            WORD => "Word",
            EXCEL => "Excel",
            _ => "",
        };
        if !kind.is_empty()
            && object.pending.as_deref() == Some(al_builtins::stream::layout_file_generated_mark(kind).as_str())
        {
            object.pending = Some(al_builtins::stream::TENANT_LAYOUT_STORED.into());
            return Ok(());
        }
        if (object.pending.is_some() && !marked) || object.bytes.is_empty() {
            return refuse("an empty Layout, or one whose content was never produced");
        }
        let bytes = &object.bytes;
        let text_start = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF][..]).unwrap_or(bytes);
        let shaped = match format {
            RDLC => text_start.first() == Some(&b'<'),
            WORD | EXCEL => bytes.starts_with(b"PK"),
            _ => true,
        };
        if !shaped {
            return refuse("a Layout whose bytes do not match its format (BC raises an uncatchable exception there)");
        }
        if format != CUSTOM {
            object.pending = Some(al_builtins::stream::TENANT_LAYOUT_STORED.into());
        }
        Ok(())
    }
}
