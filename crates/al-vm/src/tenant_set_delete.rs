//! The delete of a stored Tenant Permission Set row (2000000165) whose App ID is zero, while Tenant Permission rows (2000000166)
//! or Tenant Permission Set Rel. rows (2000000253) name it: BC deletes those rows with the set. Corpus probe
//! permissions/permission-sets/TenantSetDeleteCascade.al (corpus pull request 1340, not yet in the pinned corpus; al-runner issue
//! 7854) measured it, so the runner deletes them through the assumption `tenant-set-delete-cascade`, for the shapes the probe
//! has only (`Session::tenant_set_cascade`). Every other shape where rows name the set is refused; a delete where no row names
//! the set keeps its result.

use crate::Session;
use crate::globaltriggers::GlobalOp;
use crate::platform::{TENANT_PERMISSION, TENANT_PERMISSION_SET, TENANT_PERMISSION_SET_REL, pos};
use crate::records::Rec;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::native::{Signal, unsupported};
use al_builtins::value::Value;
use al_data::{AccessContext, Row, Store, TableDef, position};

// Field ids of App ID and Role ID in the set table (its primary key).
const F_APP_ID: u32 = 1;
const F_ROLE_ID: u32 = 2;

/// The probe file the refusals name.
const PROBE: &str = "corpus probe TenantSetDeleteCascade.al";

/// What the Delete of one stored set row takes with it: the primary keys of its Tenant Permission and Rel. rows.
pub(crate) struct TenantSetCascade {
    set_pk: Vec<Value>,
    perms: Vec<Vec<Value>>,
    rels: Vec<Vec<Value>>,
}

fn refused<T>(why: &str) -> Result<T, Signal> {
    unsupported(format!(
        "the delete of a Tenant Permission Set with App ID zero that Tenant Permission or Tenant Permission Set Rel. rows name, \
         {why} ({PROBE} measures other shapes)"
    ))
}

fn column(t: &TableDef, name: &str) -> Option<usize> {
    t.fields.iter().position(|f| f.name == name)
}

fn columns<const N: usize>(t: &TableDef, names: [&str; N]) -> Result<[usize; N], Signal> {
    let mut out = [0; N];
    for (o, n) in out.iter_mut().zip(names) {
        match column(t, n) {
            Some(c) => *o = c,
            None => return refused(&format!("on a {} table without the field {n}", t.name)),
        }
    }
    Ok(out)
}

fn is_zero(v: &Value) -> bool {
    matches!(v, Value::Guid(g) if *g == [0; 16])
}

fn same_text(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Text(x) | Value::Code(x), Value::Text(y) | Value::Code(y)) => x.as_str() == y.as_str(),
        _ => false,
    }
}

fn ordinal_is(v: &Value, n: i32) -> bool {
    matches!(v, Value::Option(o, _) if *o == n)
}

impl Session {
    /// Whether `t` with `temporary` is the stored Tenant Permission Set table.
    pub(crate) fn is_stored_tenant_set(&self, t: &TableDef, temporary: bool) -> bool {
        !temporary && self.engine.program.objects[t.object as usize].id == Some(TENANT_PERMISSION_SET)
    }

    /// What deleting the stored set row `row` of `t` deletes with it. `None`: no row names it (the App ID is not zero, or
    /// no Tenant Permission row with App ID zero and its Role ID, and no Rel. row with App ID zero and its Role ID or Related
    /// App ID zero and its Related Role ID), and the delete keeps its result. Otherwise every fact of the probe's cell must
    /// hold, else it refuses (TenantSetDeleteCascade.al, corpus pull request 1340):
    /// - no subscriber of the delete events of the three tables, no delete trigger of the set table, and the database delete
    ///   trigger known to be off for the three (the probe binds a subscriber only in TSDC_DeleteAllPermSubscriber, :234-250,
    ///   whose event arguments it does not log);
    /// - the session grants everything (`TestPermissions = Disabled`, :16);
    /// - every Tenant Permission row with the set's Role ID has App ID zero, Object Type Table Data and Type Include (:41-50);
    /// - every Rel. row with the set's Role ID as Role ID or Related Role ID has App ID zero, Related App ID zero, Related
    ///   Scope Tenant and Type Include, and names on its other side another stored set with App ID zero (:53-63, :82-88).
    ///
    /// Those rows go with the set (:121, :135, :149, :163, :179, :195: `perms gone=0 perms kept=1 rel of gone=0 rel of
    /// kept=0`).
    fn tenant_set_cascade(
        &mut self,
        cx: &AccessContext,
        t: &TableDef,
        row: &[Value],
    ) -> Result<Option<TenantSetCascade>, Signal> {
        if !is_zero(&row[pos(t, F_APP_ID)]) {
            return Ok(None);
        }
        let role = row[pos(t, F_ROLE_ID)].clone();
        let tp = self.table_def(TENANT_PERMISSION);
        let rel = self.table_def(TENANT_PERMISSION_SET_REL);
        let perm_rows = match &tp {
            Some(tp) => {
                let [rid] = columns(tp, ["Role ID"])?;
                let rows: Vec<Row> = self.all_rows(cx, tp)?;
                rows.into_iter().filter(|r| same_text(&r[rid], &role)).collect()
            }
            None => Vec::new(),
        };
        let rel_rows = match &rel {
            Some(rel) => {
                let [rid, rrid] = columns(rel, ["Role ID", "Related Role ID"])?;
                let rows: Vec<Row> = self.all_rows(cx, rel)?;
                rows.into_iter()
                    .filter(|r| same_text(&r[rid], &role) || same_text(&r[rrid], &role))
                    .collect()
            }
            None => Vec::new(),
        };
        let names_it = match (&tp, &rel) {
            (Some(tp), _) if perm_rows.iter().any(|r| is_zero(&r[pos(tp, F_APP_ID)])) => true,
            (_, Some(rel)) => {
                let [app, rid, rapp, rrid] = columns(rel, ["App ID", "Role ID", "Related App ID", "Related Role ID"])?;
                rel_rows.iter().any(|r| {
                    (is_zero(&r[app]) && same_text(&r[rid], &role)) || (is_zero(&r[rapp]) && same_text(&r[rrid], &role))
                })
            }
            _ => false,
        };
        if !names_it {
            return Ok(None);
        }
        self.tenant_set_cascade_hooks_off(t, tp.as_deref(), rel.as_deref())?;
        if self.permissions.is_some() {
            return refused("in a session whose permission sets do not grant everything");
        }
        let mut perms = Vec::new();
        if let Some(tp) = &tp {
            let [app, kind, ty] = columns(tp, ["App ID", "Object Type", "Type"])?;
            for r in &perm_rows {
                if !is_zero(&r[app]) {
                    return refused("while a Tenant Permission row with another App ID has its Role ID");
                }
                if !ordinal_is(&r[kind], 0) || !ordinal_is(&r[ty], 0) {
                    return refused("while one of its Tenant Permission rows is not an Include grant of Table Data");
                }
                perms.push(tp.primary_key(r));
            }
        }
        let mut rels = Vec::new();
        if let Some(rel) = &rel {
            let [app, rid, rapp, rrid, scope, ty] = columns(
                rel,
                [
                    "App ID",
                    "Role ID",
                    "Related App ID",
                    "Related Role ID",
                    "Related Scope",
                    "Type",
                ],
            )?;
            for r in &rel_rows {
                // Related Scope: System (0), Tenant (1); Type: Include (0), Exclude (1).
                if !is_zero(&r[app]) || !is_zero(&r[rapp]) || !ordinal_is(&r[scope], 1) || !ordinal_is(&r[ty], 0) {
                    return refused(
                        "while one of its Rel. rows is not an Include relation between tenant sets with App ID zero",
                    );
                }
                let other = if same_text(&r[rid], &role) { &r[rrid] } else { &r[rid] };
                if same_text(other, &role) || !self.tenant_set_stored(cx, t, other)? {
                    return refused(
                        "while one of its Rel. rows names on its other side the set itself or no stored set",
                    );
                }
                rels.push(rel.primary_key(r));
            }
        }
        Ok(Some(TenantSetCascade {
            set_pk: t.primary_key(row),
            perms,
            rels,
        }))
    }

    /// Refuses unless nothing can see the order of the cascade: no subscriber of the delete events of the set, Tenant
    /// Permission and Rel. tables, no delete trigger of the set table, and the database delete trigger known to be off for
    /// all three (see `tenant_set_cascade`).
    fn tenant_set_cascade_hooks_off(
        &mut self,
        t: &TableDef,
        tp: Option<&TableDef>,
        rel: Option<&TableDef>,
    ) -> Result<(), Signal> {
        let mut trigger = self.table_trigger(t, "OnDelete").is_some();
        for n in ["OnBeforeDelete", "OnDelete", "OnAfterDelete"] {
            trigger |= !self.ext_triggers(t, n).is_empty();
        }
        if trigger {
            return refused("on a set table with a delete trigger");
        }
        for table in [Some(t), tp, rel].into_iter().flatten() {
            let events = self.listened(table, "onbeforedeleteevent", None)?
                || self.listened(table, "onafterdeleteevent", None)?;
            if events || !self.global_known_off(table, GlobalOp::Delete) {
                return refused(&format!(
                    "while the delete of a {} row raises events or database triggers",
                    table.name
                ));
            }
        }
        Ok(())
    }

    /// Whether a set row with App ID zero and Role ID `role` is stored.
    fn tenant_set_stored(&mut self, cx: &AccessContext, t: &TableDef, role: &Value) -> Result<bool, Signal> {
        let mut pk = vec![Value::Null; t.keys[0].len()];
        for (k, &f) in t.keys[0].iter().enumerate() {
            pk[k] = match t.fields[f].id {
                F_APP_ID => Value::Guid([0; 16]),
                F_ROLE_ID => role.clone(),
                _ => return Ok(false),
            };
        }
        self.store
            .get(cx, t, &pk)
            .map(|r| r.is_some())
            .map_err(|e| Signal::unsupported(format!("{e}")))
    }

    /// Deletes what `c` names once its set row is gone (the delete that planned it ran), through the assumption
    /// `tenant-set-delete-cascade`. A set row still stored (the delete did not delete it) takes nothing with it.
    pub(crate) fn tenant_set_cascade_apply(
        &mut self,
        cx: &AccessContext,
        t: &TableDef,
        c: TenantSetCascade,
    ) -> Result<(), Signal> {
        let still = self
            .store
            .get(cx, t, &c.set_pk)
            .map_err(|e| Signal::unsupported(format!("{e}")))?
            .is_some();
        if still {
            return Ok(());
        }
        assume(Assumption::TenantSetDeleteCascade);
        for (id, keys) in [(TENANT_PERMISSION, c.perms), (TENANT_PERMISSION_SET_REL, c.rels)] {
            let Some(def) = self.table_def(id) else { continue };
            for pk in keys {
                self.store
                    .delete(cx, &def, &pk)
                    .map_err(|e| Signal::unsupported(format!("{e}")))?;
            }
        }
        Ok(())
    }

    /// For a Delete of `rec`: what the stored set row it deletes takes with it (`tenant_set_cascade`). A Delete that finds no
    /// stored row, or of another table, takes nothing.
    pub(crate) fn tenant_set_delete_one(
        &mut self,
        cx: &AccessContext,
        t: &TableDef,
        rec: &Rec,
    ) -> Result<Option<TenantSetCascade>, Signal> {
        if !self.is_stored_tenant_set(t, rec.borrow().temporary) {
            return Ok(None);
        }
        let pk: Vec<Value> = t.keys[0].iter().map(|&f| rec.borrow().fields[f].clone()).collect();
        let stored = self
            .store_for(rec)
            .get(cx, t, &pk)
            .map_err(|e| Signal::unsupported(format!("{e}")))?;
        match stored {
            Some(row) => self.tenant_set_cascade(cx, t, &row),
            None => Ok(None),
        }
    }

    /// For a DeleteAll of `rec`: what the rows its filters select take with them. The probe filters its DeleteAll to one set
    /// (TenantSetDeleteCascade.al:145, :159, :190): a selection with more than one row that takes rows with it is refused.
    pub(crate) fn tenant_set_delete_all(
        &mut self,
        cx: &AccessContext,
        t: &TableDef,
        rec: &Rec,
    ) -> Result<Option<TenantSetCascade>, Signal> {
        if !self.is_stored_tenant_set(t, rec.borrow().temporary) {
            return Ok(None);
        }
        let key = rec.borrow().key;
        let mut after = None;
        let mut found = None;
        while let Some(row) = self.scan(rec, t, true, after.take())? {
            if let Some(c) = self.tenant_set_cascade(cx, t, &row)? {
                if found.is_some() {
                    return refused("by a DeleteAll that selects more than one such set");
                }
                found = Some(c);
            }
            after = Some(position(t, key, &row));
        }
        Ok(found)
    }

    /// For a Truncate of `rec`: refuses when a stored set row it removes has rows naming it. Truncate is not in the probe, so
    /// the rows it would leave behind are not known to stay (BC deletes them with the set for Delete and DeleteAll). A Truncate
    /// where no row names a set keeps its result. Truncate ignores filters: every stored row is a selected set.
    pub(crate) fn tenant_set_truncate(&mut self, cx: &AccessContext, t: &TableDef, rec: &Rec) -> Result<(), Signal> {
        if !self.is_stored_tenant_set(t, rec.borrow().temporary) {
            return Ok(());
        }
        for row in self.all_rows(cx, t)? {
            if self.tenant_set_cascade(cx, t, &row)?.is_some() {
                return refused("by a Truncate");
            }
        }
        Ok(())
    }
}
