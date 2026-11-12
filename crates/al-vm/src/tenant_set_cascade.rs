//! The platform's delete of a stored Tenant Permission Set row (2000000165): its Tenant Permission rows (2000000166), the ones
//! with the same App ID and Role ID, go with it. No corpus test pins it; it runs under the assumption
//! `tenant-permission-set-delete-cascade`. Evidence: System Application's Permission Set page deletes only the set
//! (PermissionSet.Page.al, OnDeleteRecord), and Microsoft's Permission Import Export Tests
//! (ExportImportTenantPermissionSetWithAdditionalPermissions) read the set's permissions as an import wrote them after
//! `TenantPermissionSet.DeleteAll()` (al-runner issue 7854). Tenant Permission Set Rel. rows stay as before (not measured).

use crate::Session;
use crate::platform::pos;
use al_builtins::native::{Signal, unsupported};
use al_builtins::value::Value;
use al_data::{Row, Store, TableDef};

const TENANT_PERMISSION_SET: u32 = 2_000_000_165;
const TENANT_PERMISSION: u32 = 2_000_000_166;
// Field ids of both tables.
const F_APP_ID: u32 = 1;
const F_ROLE_ID: u32 = 2;

impl Session {
    /// Whether a row of `t` is a row of the stored Tenant Permission Set table.
    pub(crate) fn is_tenant_set_row(&self, t: &TableDef, temporary: bool) -> bool {
        !temporary && self.table_id(t) == Some(TENANT_PERMISSION_SET)
    }

    /// After the set row `rec` is deleted: its Tenant Permission rows go (see the module).
    pub(crate) fn tenant_set_after_delete(&mut self, t: &TableDef, rec: &crate::records::Rec) -> Result<(), Signal> {
        let Some(tp) = self.table_def(TENANT_PERMISSION) else {
            return Ok(());
        };
        let (app, role) = {
            let r = rec.borrow();
            (r.fields[pos(t, F_APP_ID)].clone(), r.fields[pos(t, F_ROLE_ID)].clone())
        };
        let cx = self.cx.clone();
        let same = |a: &Value, b: &Value| match (a, b) {
            (Value::Text(x) | Value::Code(x), Value::Text(y) | Value::Code(y)) => x.as_str() == y.as_str(),
            (Value::Guid(x), Value::Guid(y)) => x == y,
            _ => false,
        };
        let mine: Vec<Row> = self
            .all_rows(&cx, &tp)?
            .into_iter()
            .filter(|r| same(&r[pos(&tp, F_APP_ID)], &app) && same(&r[pos(&tp, F_ROLE_ID)], &role))
            .collect();
        if mine.is_empty() {
            return Ok(());
        }
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::TenantPermissionSetDeleteCascade);
        for row in mine {
            let pk = tp.primary_key(&row);
            if let Err(e) = self.store.delete(&cx, &tp, &pk) {
                return unsupported(format!(
                    "deleting a Tenant Permission row of a deleted Tenant Permission Set: {e}"
                ));
            }
        }
        Ok(())
    }
}
