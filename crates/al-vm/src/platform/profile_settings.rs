//! The Tenant Profile Setting table (2000000083, Scope = OnPrem): a stored table whose rows set "Default Role Center" (6) and
//! "Disable Personalization" (13) of the All Profile row of the same App ID and Profile ID.
//!
//! Pinned by corpus al-language-onprem/platform/lt-y35-stored-tables/Y35StoredTables.al:483-620 and
//! lt-y59-tenant-profile-setting/Y59AllProfileGhostSetting.al (corpus master 9cf26907):
//! - a fresh tenant with the Base Application holds two rows (O365 SALES: Default Role Center No, Disable Personalization Yes;
//!   ORDER PROCESSOR: both No; :581); Insert of any App ID and Profile ID answers true with no check, a duplicate Insert false,
//!   ModifyAll and DeleteAll work (:582-587);
//! - an Insert of a setting for an existing profile with Default Role Center Yes shows in All Profile, and DeleteAll turns it off
//!   again (:610-617);
//! - a setting whose Profile ID names no profile adds no All Profile row (Y59AllProfileGhostSetting.al:82-139).
//!
//! The link is applied before the write, in the same transaction: an error that rolls the write back rolls the All Profile
//! change back with it. What a Rename of a setting does to its profile, and what a removed setting gives back to a profile that
//! declares `DefaultRoleCenter`, are not pinned: refused.

use crate::Session;
use crate::platform::profiles::ALL_PROFILE;
use crate::platform::{Ident, pos};
use crate::records::Rec;
use al_builtins::native::Signal;
use al_builtins::record_op::RecordOp;
use al_builtins::value::Value;
use al_data::{Store, TableDef};
use std::sync::Arc;

pub(crate) const TENANT_PROFILE_SETTING: u32 = 2_000_000_083;
const BASE_APPLICATION_APP_ID: &str = "437dbf0e-84ff-417a-965d-ed2bb9650972";

/// The rows of a fresh tenant with the Base Application (Y35StoredTables.al:581): (Profile ID, Default Role Center, Disable
/// Personalization), App ID the Base Application's.
const SEED: &[(&str, bool, bool)] = &[("O365 SALES", false, true), ("ORDER PROCESSOR", false, false)];

fn unsupported<T>(what: &str) -> Result<T, Signal> {
    Err(Signal::Unsupported(format!("Tenant Profile Setting: {what}")))
}

/// The two linked values of a setting: (Default Role Center, Disable Personalization).
type Linked = (Value, Value);

impl Session {
    fn has_base_application(&self) -> bool {
        self.engine
            .program
            .apps
            .iter()
            .any(|a| a.id.eq_ignore_ascii_case(BASE_APPLICATION_APP_ID))
    }

    /// The seed rows of Tenant Profile Setting (none without the Base Application). Returns whether it wrote.
    pub(super) fn seed_profile_settings(&mut self, t: &TableDef, _i: &Ident) -> bool {
        if !self.has_base_application() {
            return false;
        }
        let app = al_builtins::ops::parse_guid(BASE_APPLICATION_APP_ID).expect("a literal guid");
        for &(profile, default, disable) in SEED {
            let mut r = super::blank(t);
            r[pos(t, 1)] = Value::Guid(app);
            r[pos(t, 2)] = Value::Code(profile.into());
            r[pos(t, 3)] = Value::Boolean(default);
            r[pos(t, 4)] = Value::Boolean(disable);
            if let Some(p) = t.fields.iter().position(|f| f.id == 2_000_000_000) {
                r[p] = Value::Guid(self.fresh_system_id());
            }
            let _ = self.store.insert(&self.cx, t, r);
        }
        true
    }

    /// The linked values a seed setting gives the All Profile row of (`app`, `profile`).
    pub(super) fn seeded_profile_setting(&self, app: &Value, profile: &str) -> Option<(bool, bool)> {
        let base = al_builtins::ops::parse_guid(BASE_APPLICATION_APP_ID).expect("a literal guid");
        if !self.has_base_application() || !matches!(app, Value::Guid(g) if *g == base) {
            return None;
        }
        SEED.iter()
            .find(|(p, _, _)| p.eq_ignore_ascii_case(profile))
            .map(|&(_, d, n)| (d, n))
    }

    fn all_profile_def(&self) -> Option<Arc<TableDef>> {
        let program = &self.engine.program;
        let oi = program
            .objects
            .heads()
            .iter()
            .position(|o| o.is_kind(al_ir::ObjKind::Table) && o.id == Some(ALL_PROFILE) && o.table.is_some())?;
        self.engine.tables[oi].clone()
    }

    /// Sets the linked columns of the All Profile row (Scope Tenant, `app`, `profile`), if there is one. `None` gives the row
    /// back what it holds without a setting: refused for a profile that declares DefaultRoleCenter (not pinned).
    fn link_profile(&mut self, app: &Value, profile: &Value, linked: Option<Linked>) -> Result<(), Signal> {
        let Some(m) = self.all_profile_def() else {
            return Ok(());
        };
        let mut key = vec![super::option(&m, 1, 1), app.clone(), profile.clone()];
        key[2] = al_builtins::ops::convert(key[2].clone(), &m.fields[pos(&m, 3)].ty)?;
        let cx = self.cx.clone();
        let store_err = |e| Signal::Unsupported(format!("{e}"));
        let Some(mut row) = self.store.get(&cx, &m, &key).map_err(store_err)? else {
            return Ok(());
        };
        let (default, disable) = match linked {
            Some(v) => v,
            None => {
                if self.declares_default_role_center(&row[pos(&m, 3)]) {
                    return unsupported(
                        "removing the setting of a profile that declares DefaultRoleCenter: what All Profile shows then is not pinned",
                    );
                }
                (Value::Boolean(false), Value::Boolean(false))
            }
        };
        row[pos(&m, 6)] = default;
        row[pos(&m, 13)] = disable;
        self.store.modify(&cx, &m, row).map_err(store_err)?;
        Ok(())
    }

    fn declares_default_role_center(&self, profile: &Value) -> bool {
        let name = al_builtins::native::text_of(profile).unwrap_or_default();
        self.engine.program.objects.heads().iter().any(|o| {
            o.kind == al_ir::ObjKind::Profile
                && o.name.eq_ignore_ascii_case(&name)
                && super::objects::prop(o, "DefaultRoleCenter").is_some()
        })
    }

    /// The All Profile link of a write to Tenant Profile Setting (module comment), applied before the write.
    pub(crate) fn tenant_profile_setting_check(
        &mut self,
        t: &TableDef,
        rec: &Rec,
        method: RecordOp,
        args: &[Value],
    ) -> Result<(), Signal> {
        if rec.borrow().temporary || self.engine.program.objects[t.object as usize].id != Some(TENANT_PROFILE_SETTING) {
            return Ok(());
        }
        let (app, id, default, disable) = (pos(t, 1), pos(t, 2), pos(t, 3), pos(t, 4));
        let fields = rec.borrow().fields.clone();
        let cx = self.cx.clone();
        let stored = self
            .store
            .get(&cx, t, &t.primary_key(&fields))
            .map_err(|e| Signal::Unsupported(format!("{e}")))?;
        match method {
            // A duplicate Insert answers false and changes nothing.
            RecordOp::Insert if stored.is_none() => self.link_profile(
                &fields[app],
                &fields[id],
                Some((fields[default].clone(), fields[disable].clone())),
            ),
            RecordOp::Modify if stored.is_some() => self.link_profile(
                &fields[app],
                &fields[id],
                Some((fields[default].clone(), fields[disable].clone())),
            ),
            RecordOp::Delete if stored.is_some() => self.link_profile(&fields[app], &fields[id], None),
            RecordOp::DeleteAll | RecordOp::ModifyAll => {
                let set = match method {
                    RecordOp::ModifyAll => {
                        let p = crate::records::arg_pos(args, 0)?;
                        if p != default && p != disable {
                            return Ok(());
                        }
                        let v = args.get(1).cloned().unwrap_or(Value::Null);
                        Some((p, al_builtins::ops::convert(v, &t.fields[p].ty)?))
                    }
                    _ => None,
                };
                for row in self.setting_rows(rec, t)? {
                    let linked = set.as_ref().map(|(p, v)| {
                        let mut r = row.clone();
                        r[*p] = v.clone();
                        (r[default].clone(), r[disable].clone())
                    });
                    self.link_profile(&row[app], &row[id], linked)?;
                }
                Ok(())
            }
            RecordOp::Rename => {
                if self.profile_exists(&fields[app], &fields[id])? {
                    return unsupported(
                        "Rename of the setting of an existing profile (what All Profile shows then is not pinned)",
                    );
                }
                if args.len() >= 2 && self.profile_exists(&args[0], &args[1])? {
                    return unsupported(
                        "Rename of a setting onto an existing profile (what All Profile shows then is not pinned)",
                    );
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn profile_exists(&mut self, app: &Value, profile: &Value) -> Result<bool, Signal> {
        let Some(m) = self.all_profile_def() else {
            return Ok(false);
        };
        let p = al_builtins::ops::convert(profile.clone(), &m.fields[pos(&m, 3)].ty)?;
        let key = [super::option(&m, 1, 1), app.clone(), p];
        let cx = self.cx.clone();
        Ok(self
            .store
            .get(&cx, &m, &key)
            .map_err(|e| Signal::Unsupported(format!("{e}")))?
            .is_some())
    }

    /// The rows of Tenant Profile Setting that `rec`'s filters keep.
    fn setting_rows(&mut self, rec: &Rec, t: &TableDef) -> Result<Vec<Vec<Value>>, Signal> {
        let (_, flow) = self.split_filters(rec);
        if !flow.is_empty() || rec.borrow().marked_only {
            return unsupported("a DeleteAll or ModifyAll with a FlowField filter or MarkedOnly");
        }
        let key = rec.borrow().key;
        let mut rows = Vec::new();
        let mut after = None;
        while let Some(row) = self.scan(rec, t, true, after.take())? {
            after = Some(al_data::position(t, key, &row));
            rows.push(row);
        }
        Ok(rows)
    }
}
