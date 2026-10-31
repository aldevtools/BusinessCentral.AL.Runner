//! The permission set tables: Metadata Permission Set, Permission Set and Aggregate Permission Set. Their rows
//! are the permission set objects of every loaded app (compiled or not: the scan reads `Caption` and
//! `Assignable`), and for the Aggregate table also the tenant's own sets, read live from the stored
//! Tenant Permission Set table.
//!
//! Pinned by corpus permissions/TestMetadataPermissionSet.al, TestPermissionSetTable.al,
//! TestPermissionSetUndeclaredAssignable.al and access-control/TestAggregatePermissionSetTable.al:
//! - "Role ID" is the object name, upper-cased (a Code field) and cut to the field length; "Name" is the
//!   Caption with its own casing, else the upper-case Role ID;
//! - "Assignable" is the declared property and false when it is not declared;
//! - SUPER and SECURITY carry the null App ID (the object is declared in System Application);
//!   from BC 29 a lookup by key under any App ID finds that row (`Gate::SuperPermissionSetIgnoresAppId`);
//! - the legacy Permission Set table lists only assignable sets when it is walked, but a read by the
//!   whole primary key finds a non-assignable one too;
//! - the System scope of the Aggregate table is the same set of assignable sets, and its Tenant scope
//!   is the Tenant Permission Set rows that are assignable, as they are now.

use super::mat::{Entry, Mat};
use super::objects::{Catalog, prop, string_lit, truthy};
use super::{Providers, blank, pos};
use al_builtins::filter::{FieldFilter, FilterExpr};
use al_builtins::value::Value;
use al_data::memory::MemStore;
use al_data::{AccessContext, Scan, Store, StoreError, TableDef};
use std::rc::Rc;
use std::sync::Arc;

const TENANT_PERMISSION_SET: u32 = 2_000_000_165;

/// A permission set object of the program.
pub(super) struct PSet {
    pub app: [u8; 16],
    /// Upper-case object name.
    pub role: String,
    pub name: String,
    pub app_name: String,
    pub assignable: bool,
}

fn upper_trunc(s: &str, max: usize) -> String {
    s.to_uppercase().chars().take(max).collect()
}

pub(super) fn sets(p: &Providers) -> Vec<PSet> {
    let engine = p.engine();
    let c = Catalog::new(&engine);
    c.objects
        .iter()
        .filter(|o| o.kind == al_ir::ObjKind::PermissionSet && o.id.is_some())
        .map(|o| {
            let role = o.name.to_uppercase();
            let null_app = role == "SUPER" || role == "SECURITY";
            let app = if null_app { [0; 16] } else { c.owner(o) };
            let caption = prop(o, "Caption").and_then(string_lit);
            PSet {
                app,
                name: caption.unwrap_or_else(|| role.clone()),
                app_name: c.apps.get(o.app as usize).map(|a| a.name.clone()).unwrap_or_default(),
                role,
                // An undeclared Assignable is false (TestPermissionSetUndeclaredAssignable.al:41-80).
                assignable: prop(o, "Assignable").is_some_and(truthy),
            }
        })
        .collect()
}

/// Metadata Permission Set: "App ID" (1), "Role ID" (2), Name (3), Assignable (4).
pub(super) fn metadata_permission_set_rows(p: &Providers, t: &TableDef) -> Vec<Entry> {
    sets(p)
        .into_iter()
        .map(|s| {
            let mut r = blank(t);
            r[pos(t, 1)] = Value::Guid(s.app);
            r[pos(t, 2)] = Value::Code(upper_trunc(&s.role, 30).into());
            r[pos(t, 3)] = Value::Text(s.name.chars().take(30).collect());
            r[pos(t, 4)] = Value::Boolean(s.assignable);
            Entry::ok(r)
        })
        .collect()
}

/// Permission Set: "Role ID" (1, Code[20]), Name (2), Hash (3, blank for a set served from metadata).
pub(super) fn permission_set_rows(p: &Providers, t: &TableDef, assignable_only: bool) -> Vec<Entry> {
    sets(p)
        .into_iter()
        .filter(|s| s.assignable || !assignable_only)
        .map(|s| {
            let mut r = blank(t);
            r[pos(t, 1)] = Value::Code(upper_trunc(&s.role, 20).into());
            r[pos(t, 2)] = Value::Text(s.name.chars().take(30).collect());
            Entry::ok(r)
        })
        .collect()
}

/// Whether the filters name one "Role ID" exactly: such a read is served by the key and finds a set that is
/// not assignable too (TestPermissionSetUndeclaredAssignable.al:82-92).
pub(super) fn pins_the_primary_key(t: &TableDef, filters: &[FieldFilter]) -> bool {
    let role = pos(t, 1);
    filters.iter().any(|f| {
        f.field == role
            && match &f.expr {
                FilterExpr::Eq(_) => true,
                FilterExpr::Range(Some(a), Some(b)) => al_builtins::ops::compare(a, b).is_ok_and(|o| o.is_eq()),
                _ => false,
            }
    })
}

/// Aggregate Permission Set: Scope (1), "App ID" (2), "Role ID" (3), Name (4), "App Name" (5).
pub(super) fn aggregate_rows(
    p: &Providers,
    store: &MemStore,
    cx: &AccessContext,
    t: &TableDef,
) -> Result<Vec<Entry>, StoreError> {
    let mut out = Vec::new();
    for s in sets(p).into_iter().filter(|s| s.assignable) {
        let mut r = blank(t);
        r[pos(t, 1)] = super::option(t, 1, 0);
        r[pos(t, 2)] = Value::Guid(s.app);
        r[pos(t, 3)] = Value::Code(upper_trunc(&s.role, 20).into());
        r[pos(t, 4)] = Value::Text(s.name.chars().take(30).collect());
        r[pos(t, 5)] = Value::Text(s.app_name.into());
        out.push(Entry::ok(r));
    }
    if let Some(tenant) = p.tenant_permission_set_def() {
        let mut after: Option<Vec<Value>> = None;
        loop {
            let scan = Scan {
                key: 0,
                filters: &[],
                ascending: true,
                after: after.as_deref(),
            };
            let Some(row) = store.next(cx, &tenant, &scan)? else {
                break;
            };
            after = Some(al_data::position(&tenant, 0, &row));
            let at = |id: u32| row[pos(&tenant, id)].clone();
            if !matches!(at(4), Value::Boolean(true)) {
                continue;
            }
            let mut r = blank(t);
            r[pos(t, 1)] = super::option(t, 1, 1);
            r[pos(t, 2)] = at(1);
            r[pos(t, 3)] = at(2);
            r[pos(t, 4)] = at(3);
            out.push(Entry::ok(r));
        }
    }
    Ok(out)
}

impl Providers {
    /// The schema of the stored Tenant Permission Set table, if the program compiled it.
    pub(super) fn tenant_permission_set_def(&self) -> Option<Arc<TableDef>> {
        let engine = self.engine();
        let oi = engine.program.objects.heads().iter().position(|o| {
            o.is_kind(al_ir::ObjKind::Table) && o.id == Some(TENANT_PERMISSION_SET) && o.table.is_some()
        })?;
        engine.tables[oi].clone()
    }
}

pub(super) type Cache = std::cell::RefCell<Option<(u64, Rc<Mat>)>>;

// ---- Grants: Metadata Permission, Expanded Permission, Permission -----------------------------------------
//
// Pinned by permissions/TestMetadataExpandedPermission.al (Metadata Permission lists what a set declares,
// Expanded Permission adds what its IncludedPermissionSets grant; `RIMD` is Yes in four columns and nothing
// in Execute; a grant of `R` leaves the others blank; Scope is System for a metadata set) and
// permissions/TestPermissionSetGrants.al (one Permission row per `tabledata` grant of an assignable set,
// filed under Object Type Table Data, not Table). An uppercase letter is a direct grant (Yes), a lowercase
// one an indirect grant (Indirect): the language documents it, no corpus test fixes it.

use std::cell::{OnceCell, RefCell};
use std::collections::HashMap;

/// One declared grant: the object type (the "Object Type" option ordinal), the object id and the five
/// levels (0 blank, 1 Yes, 2 Indirect).
#[derive(Clone)]
pub(super) struct Grant {
    kind: i32,
    id: u32,
    level: [u8; 5],
}

fn merge(a: u8, b: u8) -> u8 {
    if a == 1 || b == 1 { 1 } else { a.max(b) }
}

/// Entries are split at commas outside double quotes; `//` comments are dropped first.
fn split_entries(text: &str) -> Vec<String> {
    let text: String = text
        .lines()
        .map(|l| l.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join(" ");
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    for c in text.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                cur.push(c);
            }
            ',' if !quoted => out.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
    }
    out.push(cur);
    out.into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// The last name of a possibly namespace-qualified, possibly quoted name: `A.B."C D"` is `C D`.
fn last_name(s: &str) -> String {
    let mut parts = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    for c in s.trim().chars() {
        match c {
            '"' => quoted = !quoted,
            '.' if !quoted => parts.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
    }
    parts.push(cur);
    parts.pop().unwrap_or_default().trim().to_string()
}

fn parse_grants<O: al_ir::ObjectInfo + ?Sized>(
    o: &O,
    ids: &dyn Fn(al_ir::ObjKind, &str) -> Result<u32, String>,
) -> Result<Vec<Grant>, String> {
    parse_grants_with(o, ids, None)
}

/// The grants of `o`'s `Permissions` property. With `wildcards`, a grant on a whole object type (`tabledata * = RIMD`,
/// `system * = X`) merges its levels into the slot of that type ([`crate::permissions::kind_slot`]); without, it is an
/// error, as for the grants tables, which do not list such a grant row by row.
fn parse_grants_with<O: al_ir::ObjectInfo + ?Sized>(
    o: &O,
    ids: &dyn Fn(al_ir::ObjKind, &str) -> Result<u32, String>,
    mut wildcards: Option<&mut crate::permissions::Wildcards>,
) -> Result<Vec<Grant>, String> {
    let Some(text) = prop(o, "Permissions") else {
        return Ok(Vec::new());
    };
    let mut out: Vec<Grant> = Vec::new();
    for entry in split_entries(text) {
        let bad = |why: &str| format!("permission set '{}': `{entry}` {why}", o.name());
        let (head, mask) = entry.rsplit_once('=').ok_or_else(|| bad("has no permission mask"))?;
        let head = head.trim();
        let (kind_word, name) = head
            .split_once(char::is_whitespace)
            .ok_or_else(|| bad("names no object"))?;
        let name = name.trim();
        let kind_word = kind_word.to_ascii_lowercase();
        if name == "*" {
            let Some(w) = wildcards.as_deref_mut() else {
                return Err(bad("grants a whole object type (not listed row by row)"));
            };
            let code = match kind_word.as_str() {
                "tabledata" => 0,
                "table" => 1,
                "report" => 3,
                "codeunit" => 5,
                "xmlport" => 6,
                "page" => 8,
                "query" => 9,
                "system" => 10,
                other => return Err(bad(&format!("is a `{other}` grant (not pinned)"))),
            };
            let slot = crate::permissions::kind_slot(code).expect("every listed type has a slot");
            let level = parse_mask(mask).ok_or_else(|| bad("has a letter that is not R, I, M, D or X"))?;
            for (have, add) in w[slot].iter_mut().zip(level) {
                *have = merge(*have, add);
            }
            continue;
        }
        let (kind, ty) = match kind_word.as_str() {
            "tabledata" => (0, al_ir::ObjKind::Table),
            "table" => (1, al_ir::ObjKind::Table),
            "report" => (3, al_ir::ObjKind::Report),
            "codeunit" => (5, al_ir::ObjKind::Codeunit),
            "xmlport" => (6, al_ir::ObjKind::XmlPort),
            "page" => (8, al_ir::ObjKind::Page),
            "query" => (9, al_ir::ObjKind::Query),
            other => return Err(bad(&format!("is a `{other}` grant (not pinned)"))),
        };
        let id = ids(ty, &last_name(name)).map_err(|why| bad(&why))?;
        let level = parse_mask(mask).ok_or_else(|| bad("has a letter that is not R, I, M, D or X"))?;
        match out.iter_mut().find(|g| g.kind == kind && g.id == id) {
            Some(g) => {
                for (have, add) in g.level.iter_mut().zip(level) {
                    *have = merge(*have, add);
                }
            }
            None => out.push(Grant { kind, id, level }),
        }
    }
    Ok(out)
}

/// The five levels a mask such as `RIMD` or `rX` gives (0 blank, 1 Yes, 2 Indirect); `None` for a letter that is not
/// R, I, M, D or X.
fn parse_mask(mask: &str) -> Option<[u8; 5]> {
    let mut level = [0u8; 5];
    for c in mask.trim().chars() {
        let (slot, lvl) = match c {
            'R' => (0, 1),
            'r' => (0, 2),
            'I' => (1, 1),
            'i' => (1, 2),
            'M' => (2, 1),
            'm' => (2, 2),
            'D' => (3, 1),
            'd' => (3, 2),
            'X' => (4, 1),
            'x' => (4, 2),
            _ => return None,
        };
        level[slot] = merge(level[slot], lvl);
    }
    Some(level)
}

pub(super) struct GSet {
    app: [u8; 16],
    role: String,
    name: String,
    assignable: bool,
    includes: Vec<String>,
    excluded: bool,
    declared: Result<Vec<Grant>, String>,
}

pub(super) struct Built {
    sets: Vec<GSet>,
    by_name: HashMap<String, Vec<usize>>,
}

type Expanded = Result<Rc<Vec<Grant>>, String>;
/// (grants table, the sets the read could select).
type MatKey = (u8, Vec<usize>);

#[derive(Default)]
pub(super) struct Grants {
    built: OnceCell<Rc<Built>>,
    expanded: RefCell<HashMap<usize, Expanded>>,
    mats: RefCell<HashMap<MatKey, Rc<Mat>>>,
}

impl Grants {
    fn built(&self, p: &Providers) -> Rc<Built> {
        self.built
            .get_or_init(|| {
                let engine = p.engine();
                let c = Catalog::new(&engine);
                let ids = |ty: al_ir::ObjKind, name: &str| c.id_of(ty, name);
                let mut sets = Vec::new();
                let mut by_name: HashMap<String, Vec<usize>> = HashMap::new();
                for o in c
                    .objects
                    .iter()
                    .filter(|o| o.kind == al_ir::ObjKind::PermissionSet && o.id.is_some())
                {
                    let role = o.name.to_uppercase();
                    let app = if role == "SUPER" || role == "SECURITY" {
                        [0; 16]
                    } else {
                        c.owner(o)
                    };
                    let includes = prop(o, "IncludedPermissionSets")
                        .map(|t| split_entries(t).iter().map(|n| last_name(n).to_lowercase()).collect())
                        .unwrap_or_default();
                    by_name.entry(o.name.to_lowercase()).or_default().push(sets.len());
                    sets.push(GSet {
                        app,
                        name: prop(o, "Caption").and_then(string_lit).unwrap_or_else(|| role.clone()),
                        role,
                        assignable: prop(o, "Assignable").is_some_and(truthy),
                        includes,
                        excluded: prop(o, "ExcludedPermissionSets").is_some(),
                        declared: parse_grants(o, &ids),
                    });
                }
                Rc::new(Built { sets, by_name })
            })
            .clone()
    }

    /// The grants of set `i` with its included sets' grants merged in.
    fn expanded(&self, b: &Built, i: usize, visiting: &mut Vec<usize>) -> Result<Rc<Vec<Grant>>, String> {
        if let Some(known) = self.expanded.borrow().get(&i) {
            return known.clone();
        }
        let set = &b.sets[i];
        let result = (|| {
            if set.excluded {
                return Err(format!("permission set '{}' excludes sets (not pinned)", set.role));
            }
            if visiting.contains(&i) {
                return Err(format!("permission set '{}' includes itself", set.role));
            }
            visiting.push(i);
            let mut all: Vec<Grant> = set.declared.clone()?;
            for inc in &set.includes {
                let target = match b.by_name.get(inc).map(Vec::as_slice) {
                    Some([t]) => *t,
                    _ => {
                        return Err(format!(
                            "permission set '{}' includes `{inc}`, which is not one set",
                            set.role
                        ));
                    }
                };
                for g in self.expanded(b, target, visiting)?.iter() {
                    match all.iter_mut().find(|x| x.kind == g.kind && x.id == g.id) {
                        Some(x) => {
                            for k in 0..5 {
                                x.level[k] = merge(x.level[k], g.level[k]);
                            }
                        }
                        None => all.push(g.clone()),
                    }
                }
            }
            visiting.pop();
            Ok(Rc::new(all))
        })();
        self.expanded.borrow_mut().insert(i, result.clone());
        result
    }
}

/// Which grants table is being read.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum GrantTable {
    Metadata,
    Expanded,
    Legacy,
}

impl GrantTable {
    fn code(self) -> u8 {
        self as u8
    }
}

fn level_option(t: &TableDef, id: u32, n: u8) -> Value {
    super::option(t, id, i32::from(n))
}

/// The rows of one grants table over the sets that the key filters could select.
pub(super) fn grants_mat(
    p: &Providers,
    store: &MemStore,
    cx: &AccessContext,
    t: &TableDef,
    which: GrantTable,
    filters: &[FieldFilter],
) -> Result<Rc<Mat>, StoreError> {
    if which != GrantTable::Metadata && p.tenant_sets_present(store, cx)? {
        return Err(super::refused(
            "permissions of tenant sets: the platform keeps the last composed set per setup version (TestMetadataExpandedPermission.al:196-207), which is not modelled",
        ));
    }
    let g = &p.grants;
    let b = g.built(p);
    // The sets whose (App ID, Role ID) can pass the key filters. The legacy table keys on the role alone.
    let (app_pos, role_pos) = match which {
        GrantTable::Legacy => (None, pos(t, 1)),
        _ => (Some(pos(t, 1)), pos(t, 2)),
    };
    let candidates: Vec<usize> = (0..b.sets.len())
        .filter(|&i| {
            let s = &b.sets[i];
            if which == GrantTable::Legacy && !s.assignable {
                return false;
            }
            filters.iter().all(|f| {
                if Some(f.field) == app_pos {
                    f.expr.matches(&Value::Guid(s.app))
                } else if f.field == role_pos {
                    f.expr.matches(&Value::Code(if which == GrantTable::Legacy {
                        upper_trunc(&s.role, 20).into()
                    } else {
                        upper_trunc(&s.role, 30).into()
                    }))
                } else {
                    true
                }
            })
        })
        .collect();
    let key = (which.code(), candidates.clone());
    if let Some(m) = g.mats.borrow().get(&key) {
        return Ok(m.clone());
    }
    let mut entries = Vec::new();
    for &i in &candidates {
        let s = &b.sets[i];
        let role = if which == GrantTable::Legacy {
            upper_trunc(&s.role, 20)
        } else {
            upper_trunc(&s.role, 30)
        };
        let key_only = |t: &TableDef| {
            let mut k = blank(t);
            if let Some(a) = app_pos {
                k[a] = Value::Guid(s.app);
            }
            k[role_pos] = Value::Code(role.clone().into());
            k
        };
        let grants = match which {
            GrantTable::Metadata => s.declared.clone().map(Rc::new),
            _ => g.expanded(&b, i, &mut Vec::new()),
        };
        match grants {
            Err(why) => entries.push(Entry::poisoned(key_only(t), why)),
            Ok(list) => {
                for gr in list.iter() {
                    let mut r = key_only(t);
                    let (rn, ty, id, lv, flags) = match which {
                        GrantTable::Legacy => (2, 3, 4, 6, 0),
                        _ => (3, 4, 5, 7, 0),
                    };
                    let _ = flags;
                    r[pos(t, rn)] = Value::Text(s.name.chars().take(30).collect());
                    r[pos(t, ty)] = super::option(t, ty, gr.kind);
                    r[pos(t, id)] = Value::Integer(gr.id as i32);
                    for (k, l) in gr.level.iter().enumerate() {
                        r[pos(t, lv + k as u32)] = level_option(t, lv + k as u32, *l);
                    }
                    match which {
                        // Type (12): Include; Assignable (13).
                        GrantTable::Metadata => {
                            r[pos(t, 12)] = super::option(t, 12, 0);
                            r[pos(t, 13)] = Value::Boolean(s.assignable);
                        }
                        // Scope (14): System for a metadata set.
                        GrantTable::Expanded => r[pos(t, 14)] = super::option(t, 14, 0),
                        GrantTable::Legacy => {}
                    }
                    entries.push(Entry::ok(r));
                }
            }
        }
    }
    let m = Rc::new(Mat::new(t, entries));
    g.mats.borrow_mut().insert(key, m.clone());
    Ok(m)
}

impl Providers {
    /// Whether the stored Tenant Permission Set or Tenant Permission table holds a row.
    fn tenant_sets_present(&self, store: &MemStore, cx: &AccessContext) -> Result<bool, StoreError> {
        let engine = self.engine();
        for id in [TENANT_PERMISSION_SET, 2_000_000_166] {
            let Some(oi) = engine
                .program
                .objects
                .heads()
                .iter()
                .position(|o| o.is_kind(al_ir::ObjKind::Table) && o.id == Some(id) && o.table.is_some())
            else {
                continue;
            };
            let Some(def) = engine.tables[oi].clone() else { continue };
            let scan = Scan {
                key: 0,
                filters: &[],
                ascending: true,
                after: None,
            };
            if store.next(cx, &def, &scan)?.is_some() {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

pub(super) fn grant_table(v: al_builtins::platform::Virtual) -> GrantTable {
    use al_builtins::platform::Virtual;
    match v {
        Virtual::MetadataPermission => GrantTable::Metadata,
        Virtual::ExpandedPermission => GrantTable::Expanded,
        _ => GrantTable::Legacy,
    }
}

/// Equality filters on the key columns that name a set, for a read by key.
pub(super) fn key_filters(t: &TableDef, which: GrantTable, pk: &[Value]) -> Vec<FieldFilter> {
    let cols: &[u32] = if which == GrantTable::Legacy { &[1] } else { &[1, 2] };
    cols.iter()
        .zip(pk)
        .map(|(&id, v)| FieldFilter {
            field: pos(t, id),
            expr: FilterExpr::Eq(v.clone()),
            group: 0,
        })
        .collect()
}

// ---- The session's permission sets (ADR-062 section 3) ----------------------------------------------------------------

/// One permission set object of the program as the composition of a session's sets reads it.
pub(crate) struct RawSet {
    /// The Role ID: the upper-case object name cut to 20 characters (the `Permission Set` table's key, which
    /// `Permissions Mock.Assign` passes on).
    pub role: String,
    /// The grants the set declares itself, wildcards included; an error for a grant the reader does not take.
    pub declared: Result<crate::permissions::Content, String>,
    /// `IncludedPermissionSets`, as lower-case object names.
    pub includes: Vec<String>,
    /// The set declares `ExcludedPermissionSets` (subtraction is not composed yet, ADR-062 probe P11).
    pub excluded: bool,
    /// A permission set extension of the program extends the set, or names a set the program does not hold (its
    /// grants are not merged yet, ADR-062 probe P10).
    pub extended: bool,
}

/// The permission set objects of the program, and their indexes by lower-case object name.
pub(crate) fn raw_sets(engine: &crate::Engine) -> (Vec<RawSet>, HashMap<String, Vec<usize>>) {
    let c = Catalog::new(engine);
    let ids = |ty: al_ir::ObjKind, name: &str| c.id_of(ty, name);
    let extended: Vec<Option<u32>> = c
        .objects
        .iter()
        .filter(|o| o.kind == al_ir::ObjKind::PermissionSetExtension)
        .map(|o| o.extends_id)
        .collect();
    let mut sets = Vec::new();
    let mut by_name: HashMap<String, Vec<usize>> = HashMap::new();
    for o in c
        .objects
        .iter()
        .filter(|o| o.kind == al_ir::ObjKind::PermissionSet && o.id.is_some())
    {
        let includes = prop(o, "IncludedPermissionSets")
            .map(|t| split_entries(t).iter().map(|n| last_name(n).to_lowercase()).collect())
            .unwrap_or_default();
        let mut wild = crate::permissions::Wildcards::default();
        let declared = parse_grants_with(o, &ids, Some(&mut wild)).map(|list| {
            let mut explicit: Vec<(i32, u32, [u8; 5])> = list.into_iter().map(|g| (g.kind, g.id, g.level)).collect();
            explicit.sort_unstable_by_key(|&(k, id, _)| (k, id));
            crate::permissions::Content { wild, explicit }
        });
        by_name.entry(o.name.to_lowercase()).or_default().push(sets.len());
        sets.push(RawSet {
            role: upper_trunc(&o.name, 20),
            declared,
            includes,
            excluded: prop(o, "ExcludedPermissionSets").is_some(),
            extended: extended.iter().any(|e| e.is_none_or(|id| Some(id) == o.id)),
        });
    }
    (sets, by_name)
}
