//! A read that the code answering `Global Triggers.GetDatabaseTableTriggerSetup` makes under a restricted set, which the
//! session's sets do not grant (ADR-062). Only the measured read runs: Base Application's `API Webhook Notification Mgt.`
//! asks `IsEmpty` of "API Webhook Subscription" when the first write of an own table under a Permissions Mock set asks the
//! table's setup, and that table holds no row. BC raises no error there and the write goes on
//! (permissions/SysFlowFieldPerm.al:489-499: `Item.DeleteAll` of `Op`, the first write of "SFP Item" in each case, under
//! "SFP PA READ" and "SFP NO PA", neither of which grants the table). With no row, the table enables no database trigger
//! whichever way BC answers the read. Every other read keeps the refusal of `Session::write_denied`.

use crate::Session;
use crate::permissions::{Access, Op};
use crate::records::Rec;
use al_builtins::native::Signal;
use al_builtins::record_op::RecordOp;

/// "API Webhook Subscription".
const API_WEBHOOK_SUBSCRIPTION: u32 = 2_000_000_095;
/// Base Application's "API Webhook Notification Mgt.", whose `GetDatabaseTableTriggerSetup` reads it.
const API_WEBHOOK_NOTIFICATION_MGT: u32 = 6153;

impl Session {
    /// Whether the denied `access` by the record method `method` on `rec` is the measured read above. Every fact is
    /// required by equality: the setup of a table is being asked (`EventState::db_setup_table`), the method is `IsEmpty`,
    /// the access a read of "API Webhook Subscription", the running code is codeunit 6153 of Base Application, and the table holds no row in
    /// the record's company.
    pub(crate) fn trigger_setup_read_unchecked(
        &mut self,
        method: RecordOp,
        rec: &Rec,
        access: Access,
    ) -> Result<bool, Signal> {
        let program = &self.engine.program;
        let heads = program.objects.heads();
        let caller_is_mgt = self.frames.last().is_some_and(|&o| {
            heads[o].is_kind(al_ir::ObjKind::Codeunit)
                && heads[o].id == Some(API_WEBHOOK_NOTIFICATION_MGT)
                && program
                    .apps
                    .get(program.objects[o].app as usize)
                    .is_some_and(|a| a.id.eq_ignore_ascii_case(al_ir::BASE_APPLICATION_APP_ID))
        });
        let measured = self.events.db_setup_table.is_some()
            && method == RecordOp::IsEmpty
            && access.op == Op::Read
            && access.kind == 0
            && access.id == API_WEBHOOK_SUBSCRIPTION
            && caller_is_mgt;
        if !measured {
            return Ok(false);
        }
        let t = self.table(rec)?;
        let cx = self.cx_for(rec, &t)?;
        let rows = al_data::Store::count(&*self.store_for(rec)?, &cx, &t, &[])
            .map_err(|e| Signal::unsupported(format!("store error {e}")))?;
        Ok(rows == 0)
    }
}
