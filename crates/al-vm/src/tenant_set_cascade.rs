//! The platform's `DeleteAll` of stored Tenant Permission Set rows (2000000165) in the one shape Microsoft's tests show: the
//! Tenant Permission rows (2000000166) of each deleted set whose App ID is zero, the ones with the same App ID and Role ID, go
//! with it. No corpus test pins it; it runs under the assumption `tenant-permission-set-delete-cascade`. Evidence: System
//! Application's Permission Import Export Tests read a set's permissions as an import wrote them after
//! `TenantPermissionSet.SetFilter("Role ID", NewRoleId)` and `TenantPermissionSet.DeleteAll()` on a Record variable, App ID
//! zero (PermissionImportExportTests.Codeunit.al:195, :282, :383; asserts :300-301, :402; al-runner issue 7854).
//!
//! Every other shape keeps main's result, which deletes the set rows only: Delete, DeleteAll(true), RecordRef.DeleteAll, other
//! filters, a set whose App ID is not zero. The cascade deletes the permission rows in the store, after the set row's own
//! delete: no Tenant Permission delete event runs for them, and the Tenant Permission Set Rel. rows stay (not measured).

use crate::Session;
use crate::platform::{TENANT_PERMISSION, TENANT_PERMISSION_SET, pos};
use crate::records::Rec;
use al_builtins::native::{Signal, unsupported};
use al_builtins::value::Value;
use al_data::{Row, Store, TableDef};

// Field ids of both tables.
const F_APP_ID: u32 = 1;
const F_ROLE_ID: u32 = 2;

impl Session {
    /// Whether `DeleteAll(run)` of `rec` on `t` is the shape the module names: a Record variable (`via_recordref` false) on the
    /// stored Tenant Permission Set table, `run` false, no MarkedOnly, and exactly one filter, on Role ID, in filter group 0
    /// (PermissionImportExportTests.Codeunit.al:195, :202).
    pub(crate) fn tenant_set_cascade_shape(&self, t: &TableDef, rec: &Rec, run: bool, via_recordref: bool) -> bool {
        let r = rec.borrow();
        let one_role_filter = matches!(r.filters.as_slice(), [f] if f.field == pos(t, F_ROLE_ID) && f.group == 0);
        self.table_id(t) == Some(TENANT_PERMISSION_SET)
            && !r.temporary
            && !run
            && !via_recordref
            && !r.marked_only
            && one_role_filter
    }

    /// After the set row `rec` is deleted by a DeleteAll of [`Self::tenant_set_cascade_shape`]: when its App ID is zero, its
    /// Tenant Permission rows go (see the module).
    pub(crate) fn tenant_set_after_delete(&mut self, t: &TableDef, rec: &Rec) -> Result<(), Signal> {
        let (app, role) = {
            let r = rec.borrow();
            (r.fields[pos(t, F_APP_ID)].clone(), r.fields[pos(t, F_ROLE_ID)].clone())
        };
        let zero_app = matches!(app, Value::Guid(g) if g == [0; 16]);
        let (true, Some(tp)) = (zero_app, self.table_def(TENANT_PERMISSION)) else {
            return Ok(());
        };
        let cx = self.cx.clone();
        let same_role = |v: &Value| match (v, &role) {
            (Value::Text(x) | Value::Code(x), Value::Text(y) | Value::Code(y)) => x.as_str() == y.as_str(),
            _ => false,
        };
        let mine: Vec<Row> = self
            .all_rows(&cx, &tp)?
            .into_iter()
            .filter(|r| {
                matches!(r[pos(&tp, F_APP_ID)], Value::Guid(g) if g == [0; 16]) && same_role(&r[pos(&tp, F_ROLE_ID)])
            })
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
