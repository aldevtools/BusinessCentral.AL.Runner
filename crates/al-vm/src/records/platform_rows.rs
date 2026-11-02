//! Writes to platform tables whose stored rows BC changes or guards (corpus master onprem
//! platform/lt-y35-stored-tables/Y35StoredTables.al).

use super::*;

impl Session {
    /// The checks of `record_op_inner` that refuse or fail an operation on one table before it runs.
    pub(super) fn table_op_checks(
        &mut self,
        t: &TableDef,
        rec: &Rec,
        method: RecordOp,
        args: &[Value],
    ) -> Result<(), Signal> {
        self.linked_object_check(t, rec, method)?;
        self.crm_method_check(t, rec, method, args)?;
        self.agent_task_check(t, rec, method)?;
        self.mcp_configuration_delete_check(t, rec, method)?;
        self.tenant_report_layout_check(t, rec, method)?;
        self.tenant_layout_cfg_check(t, rec, method, args)?;
        self.tenant_profile_setting_check(t, rec, method, args)
    }

    /// The platform table Webhook Subscription (2000000199): an Insert stores a blank "Application ID" (field 7) as 'NAV'
    /// (corpus master onprem platform/lt-y35-stored-tables/Y35StoredTables.al C303_Webhook_RowsAndWrites, the rows read back).
    /// Another value AL set is not pinned: refused. What the variable holds afterwards is not pinned either: only the stored row changes.
    pub(super) fn webhook_application_id(&self, t: &TableDef, rec: &Rec, stored: &mut [Value]) -> Result<(), Signal> {
        if rec.borrow().temporary || self.engine.program.objects[t.object as usize].id != Some(2000000199) {
            return Ok(());
        }
        let Some(pos) = t.fields.iter().position(|f| f.id == 7) else {
            return Ok(());
        };
        match &stored[pos] {
            // 'NAV' (a row read back and inserted again) is stored as 'NAV' whether the platform fills or keeps it.
            Value::Text(v) | Value::Code(v) if v.as_str() == "NAV" => Ok(()),
            Value::Text(v) | Value::Code(v) if v.is_empty() => {
                stored[pos] = Value::Text("NAV".into());
                Ok(())
            }
            _ => unsupported(
                "an Insert into Webhook Subscription with an Application ID: whether the platform keeps it is not pinned",
            ),
        }
    }

    /// A Delete or DeleteAll of the platform table MCP Configuration (2000000292) while MCP Configuration Tool (2000000293)
    /// holds rows: BC 27.5 deletes the configuration's tools with it, BC 28.5 and 29.0 keep them (corpus master onprem
    /// platform/lt-y35-stored-tables/Y35StoredTables.al C303_McpTool_ParentChecks). The BC 27 table set is the one whose tool
    /// table has no "API Version" field (BC 28 and later); there the delete is refused, as the cascade is not followed.
    pub(super) fn mcp_configuration_delete_check(
        &self,
        t: &TableDef,
        rec: &Rec,
        method: RecordOp,
    ) -> Result<(), Signal> {
        if rec.borrow().temporary
            || !matches!(method, RecordOp::Delete | RecordOp::DeleteAll)
            || self.engine.program.objects[t.object as usize].id != Some(2000000292)
        {
            return Ok(());
        }
        let program = &self.engine.program;
        let Some(oi) = program
            .objects
            .iter()
            .position(|o| o.kind == al_ir::ObjKind::Table && o.id == Some(2000000293))
        else {
            return Ok(());
        };
        let Some(Some(tools)) = self.engine.tables.get(oi) else {
            return Ok(());
        };
        if tools.fields.iter().any(|f| f.name == "API Version") || self.store.stored_rows(&self.cx, tools).is_empty() {
            return Ok(());
        }
        unsupported(
            "a Delete of an MCP Configuration while MCP Configuration Tool holds rows, on a major whose tool table has no API Version field: there the platform deletes the configuration's tools with it (not followed)",
        )
    }
}
