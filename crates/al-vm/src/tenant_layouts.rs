//! The platform table Tenant Report Layout (2000000232), a stored table with these pinned limits
//! (report/lt-sc-report-metadata-views/Y6TenantReportLayoutRows.al, corpus master, BC 27, 28 and 29):
//!
//! - A fresh tenant holds rows here that nothing pins (12 on the SaaS Dev tenant), none of a report of the app under test
//!   (:16-33: Count 0, also for the current company, FindFirst No). So a read, `DeleteAll` or `ModifyAll` is answered only
//!   when a filter keeps the `Report ID` of one report of an app that is not Microsoft's; any other is refused.
//! - Insert of a row of such a report whose `Layout Format` is RDLC and whose Layout holds the report's own RDLC layout
//!   (Report.RdlcLayout) is stored as given: true, Company Name and MIME Type blank, the App ID null (:35-89). A second
//!   Insert of the key answers false (an ordinary duplicate). Every other Insert is refused: a blank row and one with an empty
//!   RDLC layout raise errors whose text the corpus pins only in part (ScReportMetadataViews.al:95-134).
//! - Get, Modify, Delete and Rename are not pinned: refused.

use crate::Session;
use al_builtins::native::Signal;
use al_builtins::record_op::RecordOp;
use al_builtins::value::{RecordState, Value};
use al_data::TableDef;
use std::cell::RefCell;
use std::rc::Rc;

type Rec = Rc<RefCell<RecordState>>;

const TENANT_REPORT_LAYOUT: u32 = 2_000_000_232;
/// The `Layout Format` option of an RDLC layout (the System app's table: `RDLC,Word,Excel,Custom`).
const RDLC: i32 = 0;

fn unsupported<T>(what: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(what.into()))
}

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
            | RecordOp::DeleteAll
            | RecordOp::ModifyAll => {
                if self.own_report_filter(t, rec).is_some() {
                    Ok(())
                } else {
                    refuse(method.name())
                }
            }
            RecordOp::Insert => self.tenant_report_layout_insert(t, rec),
            m if m.writes()
                || matches!(
                    m,
                    RecordOp::Get | RecordOp::GetBySystemId | RecordOp::CalcSums | RecordOp::CountApprox
                ) =>
            {
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
        let refuse = || {
            unsupported(
                "Insert on Tenant Report Layout of another row than one of a report of the app under test whose Layout is the report's own RDLC layout (not pinned)",
            )
        };
        let field = |id: u32| t.fields.iter().position(|f| f.id == id);
        let (Some(report), Some(layout), Some(format)) = (field(1), field(7), field(8)) else {
            return refuse();
        };
        let (report_id, media, format) = {
            let r = rec.borrow();
            (
                r.fields[report].clone(),
                r.fields[layout].clone(),
                r.fields[format].clone(),
            )
        };
        let (Value::Integer(report_id), Value::Option(RDLC, _)) = (report_id, format) else {
            return refuse();
        };
        let Some(object) = self.own_report(report_id) else {
            return refuse();
        };
        let Some(file) = self.default_rdlc_layout_file(object) else {
            return refuse();
        };
        let Value::Guid(id) = media else {
            return refuse();
        };
        let same = self.media.objects.borrow().get(&id).is_some_and(|m| m.bytes == file);
        if !same {
            return refuse();
        }
        Ok(())
    }
}
