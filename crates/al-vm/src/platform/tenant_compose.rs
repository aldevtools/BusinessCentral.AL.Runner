//! The Expanded Permission rows of TENANT permission sets: the stored Tenant Permission Set, Tenant Permission and Tenant
//! Permission Set Rel. rows, composed. Pinned by corpus permissions/lt-y67-perm-compose/Y67PermCompose.al and
//! permissions/TestMetadataExpandedPermission.al:150-180:
//! - a set lists its own Tenant Permission rows of Type Include under Scope Tenant (TestMetadataExpandedPermission.al:150-180);
//! - an included tenant set (relation Type Include) adds its rights, level by level, also through a further include and a
//!   diamond (Y67PermCompose.al:88 Compose_OwnAndIncluded, :161 Compose_OverlapAndDiamond);
//! - an excluded tenant set (relation Type Exclude) and the set's own Tenant Permission rows of Type Exclude remove the rights
//!   they grant from the composed set, after the includes; an object left with no right has no row (:110
//!   Compose_ExcludedSet, :139 Compose_ExcludePermissionRow);
//! - an included set that does not exist adds nothing, and a cycle of includes gives each set the rights of all of them
//!   (:191 Compose_IncludedSetMissing_And_Cycle).
//!
//! Only the shapes the corpus measured are served; a set that uses another is refused when a read can reach it: a right of
//! Indirect, a Security Filter, an object type other than Table Data, a row with no right, a relation to a System set, an
//! included set that excludes, an excluded set that is missing or composes itself, and Tenant Permission rows of a role
//! without a set. A set composed again after its rows changed is refused: the platform keeps the last composed set
//! (TestMetadataExpandedPermission.al:196-207), which is not modelled.

use super::mat::Entry;
use super::{Providers, blank, pos};
use al_builtins::filter::FieldFilter;
use al_builtins::value::Value;
use al_data::memory::MemStore;
use al_data::{AccessContext, Scan, Store, StoreError, TableDef};
use std::collections::HashMap;
use std::sync::Arc;

const TENANT_PERMISSION_SET: u32 = 2_000_000_165;
const TENANT_PERMISSION: u32 = 2_000_000_166;
const TENANT_PERMISSION_SET_REL: u32 = 2_000_000_253;

/// (App ID, upper-case Role ID).
type SetKey = ([u8; 16], String);

/// One composed right: (object type ordinal, object id) and the five levels (0 blank, 1 Yes).
type Rights = Vec<((i32, i32), [u8; 5])>;

#[derive(Default)]
struct TenantSet {
    name: Option<Value>,
    /// The own rows of Type Include.
    include: Rights,
    /// The own rows of Type Exclude.
    exclude: Rights,
    /// Related sets of Type Include and Exclude.
    includes: Vec<SetKey>,
    excludes: Vec<SetKey>,
    /// Why the set is not served.
    unpinned: Option<String>,
}

/// The sets composed so far in this session, for the refusal of a set composed again after its rows changed.
/// A stored table and its rows.
type TableRows = (Arc<TableDef>, Vec<Vec<Value>>);

pub(super) type Composed = std::cell::RefCell<HashMap<SetKey, Rights>>;

fn col(t: &TableDef, name: &str) -> Option<usize> {
    t.fields.iter().position(|f| f.name.eq_ignore_ascii_case(name))
}

fn text(v: &Value) -> String {
    match v {
        Value::Code(c) | Value::Text(c) => c.as_str().to_uppercase(),
        _ => String::new(),
    }
}

fn guid(v: &Value) -> [u8; 16] {
    match v {
        Value::Guid(g) => *g,
        _ => [0; 16],
    }
}

fn ord(v: &Value) -> i32 {
    match v {
        Value::Option(n, _) => *n,
        Value::Integer(n) => *n,
        _ => -1,
    }
}

impl Providers {
    fn stored_table(&self, id: u32) -> Option<Arc<TableDef>> {
        let engine = self.engine();
        let oi = engine
            .program
            .objects
            .heads()
            .iter()
            .position(|o| o.is_kind(al_ir::ObjKind::Table) && o.id == Some(id) && o.table.is_some())?;
        engine.tables[oi].clone()
    }

    fn all_rows(&self, store: &MemStore, cx: &AccessContext, id: u32) -> Result<Option<TableRows>, StoreError> {
        let Some(def) = self.stored_table(id) else {
            return Ok(None);
        };
        let mut rows = Vec::new();
        let mut after: Option<Vec<Value>> = None;
        loop {
            let scan = Scan {
                key: 0,
                filters: &[],
                ascending: true,
                after: after.as_deref(),
            };
            let Some(row) = store.next(cx, &def, &scan)? else { break };
            after = Some(al_data::position(&def, 0, &row));
            rows.push(row);
        }
        Ok(Some((def, rows)))
    }

    fn tenant_sets(&self, store: &MemStore, cx: &AccessContext) -> Result<HashMap<SetKey, TenantSet>, StoreError> {
        let mut sets: HashMap<SetKey, TenantSet> = HashMap::new();
        let missing = || super::refused("the tenant permission tables are not all in the program");
        let (st, rows) = self.all_rows(store, cx, TENANT_PERMISSION_SET)?.ok_or_else(missing)?;
        let (app, role, name) = (col(&st, "App ID"), col(&st, "Role ID"), col(&st, "Name"));
        let (Some(app), Some(role), Some(name)) = (app, role, name) else {
            return Err(missing());
        };
        for r in rows {
            let set = sets.entry((guid(&r[app]), text(&r[role]))).or_default();
            set.name = Some(r[name].clone());
        }
        let (pt, rows) = self.all_rows(store, cx, TENANT_PERMISSION)?.ok_or_else(missing)?;
        let names = [
            "App ID",
            "Role ID",
            "Object Type",
            "Object ID",
            "Read Permission",
            "Insert Permission",
            "Modify Permission",
            "Delete Permission",
            "Execute Permission",
            "Security Filter",
            "Type",
        ];
        let c: Vec<usize> = names.iter().filter_map(|n| col(&pt, n)).collect();
        if c.len() != names.len() {
            return Err(missing());
        }
        for r in rows {
            let key = (guid(&r[c[0]]), text(&r[c[1]]));
            let Some(set) = sets.get_mut(&key) else {
                // Rows of a role with no set: not measured. A set row added later makes the role a set again.
                let set = sets.entry(key).or_default();
                set.unpinned = Some("Tenant Permission rows of a role that has no Tenant Permission Set row".into());
                continue;
            };
            let mut level = [0u8; 5];
            for (k, l) in level.iter_mut().enumerate() {
                *l = ord(&r[c[4 + k]]) as u8;
            }
            // Object Type: Table Data (0).
            let why = if ord(&r[c[2]]) != 0 {
                Some("a tenant grant of an object type other than Table Data")
            } else if level.iter().any(|l| *l > 1) {
                Some("a tenant grant of Indirect")
            } else if level.iter().all(|l| *l == 0) {
                Some("a tenant grant with no right")
            } else if !text(&r[c[9]]).is_empty() {
                Some("a tenant grant with a Security Filter")
            } else {
                None
            };
            if let Some(why) = why {
                set.unpinned.get_or_insert_with(|| why.to_string());
                continue;
            }
            let right = (
                (
                    ord(&r[c[2]]),
                    match &r[c[3]] {
                        Value::Integer(n) => *n,
                        _ => 0,
                    },
                ),
                level,
            );
            match ord(&r[c[10]]) {
                0 => set.include.push(right),
                1 => set.exclude.push(right),
                _ => {
                    set.unpinned
                        .get_or_insert_with(|| "a tenant grant of another Type".into());
                }
            }
        }
        let (rt, rows) = self
            .all_rows(store, cx, TENANT_PERMISSION_SET_REL)?
            .ok_or_else(missing)?;
        let names = [
            "App ID",
            "Role ID",
            "Related App ID",
            "Related Role ID",
            "Related Scope",
            "Type",
        ];
        let c: Vec<usize> = names.iter().filter_map(|n| col(&rt, n)).collect();
        if c.len() != names.len() {
            return Err(missing());
        }
        for r in rows {
            let Some(set) = sets.get_mut(&(guid(&r[c[0]]), text(&r[c[1]]))) else {
                continue;
            };
            let related = (guid(&r[c[2]]), text(&r[c[3]]));
            // Related Scope: System (0), Tenant (1).
            match (ord(&r[c[4]]), ord(&r[c[5]])) {
                (1, 0) => set.includes.push(related),
                (1, 1) => set.excludes.push(related),
                _ => {
                    set.unpinned
                        .get_or_insert_with(|| "a relation to a System permission set or of another Type".into());
                }
            }
        }
        Ok(sets)
    }
}

fn add(into: &mut Rights, from: &Rights) {
    for (obj, level) in from {
        match into.iter_mut().find(|(o, _)| o == obj) {
            Some((_, l)) => {
                for k in 0..5 {
                    l[k] = l[k].max(level[k]);
                }
            }
            None => into.push((*obj, *level)),
        }
    }
}

fn remove(from: &mut Rights, what: &Rights) {
    for (obj, level) in what {
        if let Some((_, l)) = from.iter_mut().find(|(o, _)| o == obj) {
            for k in 0..5 {
                if level[k] > 0 {
                    l[k] = 0;
                }
            }
        }
    }
    from.retain(|(_, l)| l.iter().any(|x| *x > 0));
}

/// The included rights of `key` and of every set it includes, each set once (a cycle ends there).
fn included(
    sets: &HashMap<SetKey, TenantSet>,
    key: &SetKey,
    top: bool,
    seen: &mut Vec<SetKey>,
) -> Result<Rights, String> {
    seen.push(key.clone());
    let Some(set) = sets.get(key) else {
        return Ok(Vec::new());
    };
    if let Some(why) = &set.unpinned {
        return Err(why.clone());
    }
    if !top && (!set.exclude.is_empty() || !set.excludes.is_empty()) {
        return Err("an included tenant set that excludes rights".into());
    }
    let mut rights = set.include.clone();
    for inc in &set.includes {
        if !seen.contains(inc) {
            add(&mut rights, &included(sets, inc, false, seen)?);
        }
    }
    Ok(rights)
}

fn compose(sets: &HashMap<SetKey, TenantSet>, key: &SetKey) -> Result<Rights, String> {
    let mut rights = included(sets, key, true, &mut Vec::new())?;
    let set = &sets[key];
    for ex in &set.excludes {
        let Some(other) = sets.get(ex) else {
            return Err("an excluded tenant set that does not exist".into());
        };
        if other.unpinned.is_some()
            || !other.includes.is_empty()
            || !other.excludes.is_empty()
            || !other.exclude.is_empty()
        {
            return Err("an excluded tenant set that composes other sets or excludes rights".into());
        }
        remove(&mut rights, &other.include);
    }
    remove(&mut rights, &set.exclude);
    rights.sort_by_key(|(o, _)| *o);
    Ok(rights)
}

/// The Expanded Permission rows of the tenant sets whose key can pass `filters` (on "App ID" and "Role ID").
pub(super) fn tenant_expanded_entries(
    p: &Providers,
    store: &MemStore,
    cx: &AccessContext,
    t: &TableDef,
    filters: &[FieldFilter],
) -> Result<Vec<Entry>, StoreError> {
    let sets = p.tenant_sets(store, cx)?;
    let (app_pos, role_pos) = (pos(t, 1), pos(t, 2));
    let mut out = Vec::new();
    let mut keys: Vec<&SetKey> = sets.keys().collect();
    keys.sort();
    for key in keys {
        let passes = filters.iter().all(|f| {
            if f.field == app_pos {
                f.expr.matches(&Value::Guid(key.0))
            } else if f.field == role_pos {
                f.expr.matches(&Value::Code(key.1.clone().into()))
            } else {
                true
            }
        });
        if !passes {
            continue;
        }
        let mut key_only = blank(t);
        key_only[app_pos] = Value::Guid(key.0);
        key_only[role_pos] = Value::Code(key.1.clone().into());
        let composed = compose(&sets, key).and_then(|rights| {
            let mut seen = p.tenant_composed.borrow_mut();
            match seen.get(key) {
                Some(before) if *before != rights => Err(
                    "a tenant set composed again after its rows changed: the platform keeps the last composed set (TestMetadataExpandedPermission.al:196-207), which is not modelled"
                        .to_string(),
                ),
                _ => {
                    seen.insert(key.clone(), rights.clone());
                    Ok(rights)
                }
            }
        });
        match composed {
            Err(why) => out.push(Entry::poisoned(
                key_only,
                format!("Expanded Permission of tenant set '{}': {why} (not pinned)", key.1),
            )),
            Ok(rights) => {
                for ((kind, id), level) in rights {
                    let mut r = key_only.clone();
                    if let Some(name) = &sets[key].name {
                        r[pos(t, 3)] = name.clone();
                    }
                    r[pos(t, 4)] = super::option(t, 4, kind);
                    r[pos(t, 5)] = Value::Integer(id);
                    for (k, l) in level.iter().enumerate() {
                        r[pos(t, 7 + k as u32)] = super::option(t, 7 + k as u32, i32::from(*l));
                    }
                    // Scope (14): Tenant.
                    r[pos(t, 14)] = super::option(t, 14, 1);
                    out.push(Entry::ok(r));
                }
            }
        }
    }
    Ok(out)
}
