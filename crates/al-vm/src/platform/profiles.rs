//! The All Profile table (2000000178): the profiles the apps declare, and the tenant's own.
//!
//! The table is stored. A session seeds one row per `profile` object of every loaded app, and AL may
//! insert, change and delete tenant-owned rows like any stored row. The platform's own rules on the
//! rows of installed apps are native triggers (below).
//!
//! Pinned by corpus profile/TestAllProfileTable.al and TestCopyProfile.al:
//! - an app's profile is a row of Scope `Tenant` under the app's id (Scope `System` is empty:
//!   TestAllProfileTable.al:81-92), with the declared `Caption`, the declared `ProfileDescription`
//!   as "Description" (a profile that declares only `Description` has an empty one: :52-66), the page
//!   id of its `RoleCenter`, `Enabled`, `Promoted` and the app's name;
//! - a row of an installed app cannot be deleted (`Cannot delete '<id>' profile from an Installed
//!   Application.`, :96-107), and a rename that changes the key of such a row, or moves a tenant
//!   row onto an installed app's id, is refused with `Cannot modify the Scope, Profile ID, or
//!   RoleCenter fields on the <new id> profile because it is part of an installed app.` (:109-148);
//!   a rename onto the row's own key succeeds (:150-166);
//! - a tenant-owned row (empty App ID) can be inserted, read back and deleted (:200-232).
//!
//! Not pinned, so documented defaults or refused columns: `Enabled` of a profile that does not declare
//! it (true, the AL default), `Caption` of a profile that declares none (its name), "Use Comments"
//! and the other obsolete columns (not read).

use crate::Session;
use crate::platform::{Ident, pos};
use al_builtins::native::{AlError, Ctx, Signal};
use al_builtins::value::Value;
use al_data::{Store, TableDef};

pub(crate) const ALL_PROFILE: u32 = 2_000_000_178;
/// Tenant Profile Page Metadata: App ID (1), Profile ID (2), Page ID (3), ...
pub(crate) const TENANT_PROFILE_PAGE_METADATA: u32 = 2_000_000_187;

/// The Tenant Profile Page Metadata table and some of its rows.
type PageMetadataRows = (std::sync::Arc<TableDef>, Vec<Vec<Value>>);

fn text_of_literal(v: &str) -> Option<String> {
    super::objects::string_lit(v)
}

impl Session {
    /// One All Profile row per profile object of the program.
    pub(super) fn seed_profiles(&mut self, t: &TableDef, _i: &Ident) -> bool {
        let engine = self.engine.clone();
        let mut wrote = false;
        // Which profiles get a row, and the role center page each names, depend on the program
        // only: found once per engine, not by every session scanning every object for each profile.
        let seeds = engine.profile_seeds.get_or_init(|| profile_seeds(&engine.program));
        for &(oi, role_center_id) in seeds {
            let o = &engine.program.objects[oi];
            let prop = |n: &str| super::objects::prop(o, n);
            let app = engine.program.apps.get(o.app as usize);
            let owner = app
                .and_then(|a| al_builtins::ops::parse_guid(&a.id))
                .filter(|_| !app.is_some_and(|a| a.id.eq_ignore_ascii_case("8874ed3a-0643-4247-9ced-7a7002f7135d")))
                .unwrap_or([0; 16]);
            let mut r = super::blank(t);
            r[pos(t, 1)] = super::option(t, 1, 1);
            r[pos(t, 2)] = Value::Guid(owner);
            r[pos(t, 3)] = Value::Code(o.name.to_uppercase().chars().take(30).collect());
            r[pos(t, 4)] = Value::Text(
                prop("ProfileDescription")
                    .and_then(text_of_literal)
                    .unwrap_or_default()
                    .into(),
            );
            r[pos(t, 5)] = Value::Integer(role_center_id);
            r[pos(t, 6)] = Value::Boolean(prop("DefaultRoleCenter").is_some_and(super::objects::truthy));
            r[pos(t, 14)] = Value::Text(app.map(|a| a.name.clone()).unwrap_or_default().into());
            r[pos(t, 15)] = Value::Boolean(prop("Enabled").is_none_or(super::objects::truthy));
            r[pos(t, 16)] = Value::Text(
                prop("Caption")
                    .and_then(text_of_literal)
                    .unwrap_or_else(|| o.name.clone())
                    .chars()
                    .take(100)
                    .collect(),
            );
            r[pos(t, 17)] = Value::Boolean(prop("Promoted").is_some_and(super::objects::truthy));
            if let Some(p) = t.fields.iter().position(|f| f.id == 2_000_000_000) {
                r[p] = Value::Guid(self.new_guid());
            }
            let _ = self.store.insert(&self.cx, t, r);
            wrote = true;
        }
        wrote
    }

    pub(crate) fn is_profile_row(&self, t: &TableDef, temporary: bool) -> bool {
        !temporary && self.engine.program.objects[t.object as usize].id == Some(ALL_PROFILE)
    }

    /// The Tenant Profile Page Metadata table and the rows of profile `profile_id` under app `app` (a store scan).
    fn page_metadata_rows(&self, app: &Value, profile_id: &Value) -> Result<Option<PageMetadataRows>, Signal> {
        let Some(oi) = self
            .engine
            .program
            .objects
            .heads()
            .iter()
            .position(|o| o.kind == al_ir::ObjKind::Table && o.id == Some(TENANT_PROFILE_PAGE_METADATA))
        else {
            return Ok(None);
        };
        let Some(m) = self.engine.tables[oi].clone() else {
            return Ok(None);
        };
        let cx = self.cx.clone();
        let mut filters = Vec::new();
        for (id, v) in [(1, app), (2, profile_id)] {
            let field = pos(&m, id);
            filters.push(al_builtins::filter::FieldFilter {
                field,
                expr: al_builtins::filter::FilterExpr::Eq(al_builtins::ops::convert(v.clone(), &m.fields[field].ty)?),
                group: 0,
            });
        }
        let mut rows = Vec::new();
        let mut after: Option<Vec<Value>> = None;
        loop {
            let scan = al_data::Scan {
                key: 0,
                filters: &filters,
                ascending: true,
                after: after.as_deref(),
            };
            let Some(row) = self
                .store
                .next(&cx, &m, &scan)
                .map_err(|e| Signal::Unsupported(format!("{e}")))?
            else {
                break;
            };
            after = Some(al_data::position(&m, 0, &row));
            rows.push(row);
        }
        Ok(Some((m, rows)))
    }

    /// After a rename of an All Profile row: the Tenant Profile Page Metadata rows of a tenant profile (empty App ID) follow
    /// it to the new Profile ID, the rows of another profile stay (al-language-onprem/profile/TestTenantProfileRenameCascade.al:
    /// 83-118). Rows of a profile whose rename is not of that shape are not pinned: refused.
    pub(crate) fn profile_after_rename(&mut self, t: &TableDef, old: &[Value], new: &[Value]) -> Result<(), Signal> {
        let (scope, app, id) = (pos(t, 1), pos(t, 2), pos(t, 3));
        let Some((m, rows)) = self.page_metadata_rows(&old[app], &old[id])? else {
            return Ok(());
        };
        if rows.is_empty() {
            return Ok(());
        }
        let tenant = |r: &[Value]| matches!(&r[scope], Value::Option(1, _)) && is_empty_guid(&r[app]);
        if !tenant(old) || !tenant(new) {
            return Err(Signal::Unsupported(
                "Rename of a profile that has Tenant Profile Page Metadata, other than of a tenant profile: not pinned"
                    .into(),
            ));
        }
        let cx = self.cx.clone();
        let target = pos(&m, 2);
        let new_id = al_builtins::ops::convert(new[id].clone(), &m.fields[target].ty)?;
        for row in rows {
            let old_pk = m.primary_key(&row);
            let mut changed = row.clone();
            changed[target] = new_id.clone();
            match self
                .store
                .rename(&cx, &m, &old_pk, changed)
                .map_err(|e| Signal::Unsupported(format!("{e}")))?
            {
                al_data::RenameOutcome::Renamed => {}
                _ => {
                    return Err(Signal::Unsupported(
                        "Rename of a profile: its Tenant Profile Page Metadata collides with an existing row (not pinned)".into(),
                    ));
                }
            }
        }
        Ok(())
    }

    /// An Insert of Tenant Profile Page Metadata with an empty Profile ID raises an uncatchable error on the tier
    /// (al-language-onprem/session/PZPlatformSurvey.al:12): refused.
    pub(crate) fn page_metadata_before_insert(&self, t: &TableDef, row: &[Value]) -> Result<(), Signal> {
        if self.engine.program.objects[t.object as usize].id == Some(TENANT_PROFILE_PAGE_METADATA)
            && al_builtins::native::text_of(&row[pos(t, 2)]).is_some_and(|s| s.trim().is_empty())
        {
            return Err(Signal::Unsupported(
                "Insert of Tenant Profile Page Metadata with an empty Profile ID: the tier raises an uncatchable error"
                    .into(),
            ));
        }
        Ok(())
    }

    /// A row that an installed app declares cannot be deleted. What a Delete of a tenant row that still has Tenant Profile
    /// Page Metadata does to those rows is not pinned: refused.
    pub(crate) fn profile_before_delete(&self, t: &TableDef, row: &[Value]) -> Result<(), Signal> {
        if is_empty_guid(&row[pos(t, 2)]) {
            if self
                .page_metadata_rows(&row[pos(t, 2)], &row[pos(t, 3)])?
                .is_some_and(|(_, r)| !r.is_empty())
            {
                return Err(Signal::Unsupported(
                    "Delete of a profile that has Tenant Profile Page Metadata: what happens to those rows is not pinned".into(),
                ));
            }
            return Ok(());
        }
        Err(error(format!(
            "Cannot delete '{}' profile from an Installed Application.",
            text(&row[pos(t, 3)])
        )))
    }

    /// A rename that changes the key of an installed app's row, or puts a tenant row under an app's id,
    /// is refused; a rename onto the row's own key is not.
    pub(crate) fn profile_before_rename(&self, t: &TableDef, old: &[Value], new: &[Value]) -> Result<(), Signal> {
        let key = [pos(t, 1), pos(t, 2), pos(t, 3)];
        let same = key
            .iter()
            .all(|&k| al_builtins::ops::compare(&old[k], &new[k]).is_ok_and(|o| o.is_eq()));
        if same || (is_empty_guid(&old[pos(t, 2)]) && is_empty_guid(&new[pos(t, 2)])) {
            return Ok(());
        }
        Err(error(format!(
            "Cannot modify the Scope, Profile ID, or RoleCenter fields on the {} profile because it is part of an installed app.",
            text(&new[pos(t, 3)])
        )))
    }
}

fn is_empty_guid(v: &Value) -> bool {
    matches!(v, Value::Guid(g) if *g == [0; 16])
}

fn text(v: &Value) -> String {
    al_builtins::native::text_of(v).unwrap_or_default().to_string()
}

/// The error the server raises from the table trigger. Its code is not pinned.
fn error(message: String) -> Signal {
    Signal::Error(Box::new(AlError {
        message,
        code: "Dialog".into(),
        call_stack: String::new(),
        ..Default::default()
    }))
}

/// The profile objects that get an All Profile row, as (object index, id of the page named as the
/// role center; 0 for none). A profile whose role center name is not exactly one page gets no row:
/// the row would be wrong.
pub(crate) fn profile_seeds(program: &al_ir::Program) -> Vec<(usize, i32)> {
    let mut pages: std::collections::HashMap<String, Vec<u32>> = std::collections::HashMap::new();
    for p in program.objects.heads() {
        if let (true, Some(id)) = (p.kind == al_ir::ObjKind::Page, p.id) {
            pages.entry(p.name.to_ascii_lowercase()).or_default().push(id);
        }
    }
    let mut out = Vec::new();
    for (oi, o) in program.objects.heads().iter().enumerate() {
        if o.kind != al_ir::ObjKind::Profile {
            continue;
        }
        // `RoleCenter = 9006;` names the page by id (Base Application's profiles do), `RoleCenter = Name;` by name.
        let role_center = o
            .properties
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("RoleCenter"))
            .map(|(_, v)| v.trim())
            .map(|v| match v.trim().parse::<u32>() {
                Ok(id) => vec![id],
                Err(_) => {
                    let want = super::objects::ident(v).to_ascii_lowercase();
                    pages.get(&want).cloned().unwrap_or_default()
                }
            });
        match role_center.as_deref() {
            None => out.push((oi, 0)),
            Some([id]) => out.push((oi, *id as i32)),
            Some(_) => {}
        }
    }
    out
}
