//! The Metadata Permission Set Rel. table (2000000252): one row per include and per exclude of a permission set object.
//!
//! Pinned by corpus permissions/lt-y59-metadata-permission/Y59PermissionSetRel.al (corpus master 9cf26907) and
//! codeunit/lt-x-object-props/XObjectProps.al:335-360:
//! - a row per `IncludedPermissionSets` entry (Type Include) and per `ExcludedPermissionSets` entry (Type Exclude), keyed
//!   (App ID, Role ID, Related App ID, Related Role ID), Role IDs upper case (:192-197);
//! - an include that a permission set extension adds is a row of the extended set (:193); a set with no includes has no rows,
//!   a set that is only excluded elsewhere has none either, a non-assignable set has its rows (:194-196);
//! - "Related App ID" is the id of the app that declares the related set (System Application for "Company - Read", :197);
//! - every write ends in "The Metadata Permission Set Rel. table is read-only." (:290-297): the computed tables' write rule.
//!
//! Not pinned, so the row is refused when read: an entry that names no set or more than one set, an exclude added by an
//! extension, an extension of a set of another app.

use super::mat::Entry;
use super::objects::{Catalog, prop};
use super::permissions::name_list;
use super::{Providers, blank, option, pos};
use al_builtins::value::Value;
use al_data::TableDef;
use al_ir::{ObjKind, ObjectHead};
use std::collections::HashMap;

fn role_id(name: &str) -> Value {
    Value::Code(name.to_uppercase().chars().take(30).collect())
}

pub(super) fn metadata_permission_set_rel_rows(p: &Providers, t: &TableDef) -> Vec<Entry> {
    let engine = p.engine();
    let c = Catalog::new(&engine);
    let sets: Vec<&ObjectHead> = c
        .objects
        .iter()
        .filter(|o| o.kind == ObjKind::PermissionSet && o.id.is_some())
        .collect();
    // SUPER and SECURITY carry the null App ID (permissions.rs, TestMetadataPermissionSet.al).
    let app_of = |o: &ObjectHead| {
        let role = o.name.to_uppercase();
        if role == "SUPER" || role == "SECURITY" {
            [0; 16]
        } else {
            c.owner(o)
        }
    };
    let mut by_name: HashMap<String, Vec<&ObjectHead>> = HashMap::new();
    for o in &sets {
        by_name.entry(o.name.to_lowercase()).or_default().push(o);
    }
    let mut out = Vec::new();
    for o in &sets {
        let app = app_of(o);
        let key = |related_app: [u8; 16], related: Value| {
            let mut r = blank(t);
            r[pos(t, 1)] = Value::Guid(app);
            r[pos(t, 2)] = role_id(&o.name);
            r[pos(t, 3)] = Value::Guid(related_app);
            r[pos(t, 4)] = related;
            r
        };
        let mut includes = name_list(*o, "IncludedPermissionSets");
        let mut unpinned = None;
        for e in c
            .objects
            .iter()
            .filter(|e| e.kind == ObjKind::PermissionSetExtension && e.extends_id == o.id)
        {
            if prop(e, "ExcludedPermissionSets").is_some() {
                unpinned = Some(format!(
                    "Metadata Permission Set Rel.: permission set '{}' has an extension that excludes sets (not pinned)",
                    o.name
                ));
            } else if c.owner(e) != c.owner(*o) {
                unpinned = Some(format!(
                    "Metadata Permission Set Rel.: permission set '{}' has an extension in another app (not pinned)",
                    o.name
                ));
            }
            includes.extend(name_list(e, "IncludedPermissionSets"));
        }
        if let Some(why) = unpinned {
            out.push(Entry::poisoned(key([0; 16], Value::Code(Default::default())), why));
            continue;
        }
        let excludes = name_list(*o, "ExcludedPermissionSets");
        for (ty, names) in [(0, includes), (1, excludes)] {
            for n in names {
                match by_name.get(&n).map(Vec::as_slice) {
                    Some([target]) => {
                        let mut r = key(app_of(target), role_id(&target.name));
                        r[pos(t, 5)] = option(t, 5, ty);
                        out.push(Entry::ok(r));
                    }
                    _ => out.push(Entry::poisoned(
                        key([0; 16], role_id(&n)),
                        format!(
                            "Metadata Permission Set Rel.: permission set '{}' names `{n}`, which is not one set (not pinned)",
                            o.name
                        ),
                    )),
                }
            }
        }
    }
    out
}
