//! The session's permission sets (ADR-062).
//!
//! A test lowers its permissions through `Permissions Mock` (codeunit 131006), which calls the platform's
//! `PermissionTestHelper`: `Clear`/`Dispose` and the constructor leave no set assigned, `AddEffectivePermissionSet(RoleId)`
//! adds one. With no set assigned the user's own permissions apply, and the test user is SUPER: everything is allowed
//! (permissions/TestPermissionsMockEnforcement.al:74-97). With sets assigned the session has exactly their union
//! (TestPermissionsMockEnforcement.al:28-72, lt-permissions-mock-indirect/TestMockBasics.al:20-79).
//!
//! The content of each set comes from its source (ADR-062 section 3): the grants it declares, wildcards included, and
//! the sets it includes. When the union grants every object kind at Yes by wildcard (SUPER, which System Application
//! declares as `SUPER (DATA)` = `tabledata * = RIMD` plus Execute on every object kind and on `system *`), the session is
//! **unrestricted** and holds `None`: nothing is checked, one branch per access (ADR-062 section 4).
//!
//! Any other assigned list leaves the session restricted. No access kind is enforced yet, so the first access a
//! restricted session makes outside the mock's own code refuses the test with the text the refusal had before (the
//! stages of ADR-062 section 5 turn the checks on once their probes land).

use crate::{Engine, Session};
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::native::Signal;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// The five levels of a grant, Read, Insert, Modify, Delete, Execute: 0 blank, 1 Yes (direct), 2 Indirect.
pub(crate) type Levels = [u8; 5];

/// How many object kinds a grant can name: TableData, Table, Report, Codeunit, XmlPort, Page, Query, System.
pub(crate) const KINDS: usize = 8;

/// Per object kind ([`kind_slot`]), the levels a grant on the whole kind (`tabledata * = RIMD`) gives.
pub(crate) type Wildcards = [Levels; KINDS];

/// The slot of an object kind, by the "Object Type" ordinal of the Permission tables (TableData 0, Table 1, Report 3,
/// Codeunit 5, XMLport 6, Page 8, Query 9, System 10).
pub(crate) fn kind_slot(code: i32) -> Option<usize> {
    Some(match code {
        0 => 0,
        1 => 1,
        3 => 2,
        5 => 3,
        6 => 4,
        8 => 5,
        9 => 6,
        10 => 7,
        _ => return None,
    })
}

/// The id of codeunit "Permissions Mock": its own code assigns sets one by one (`Set` is `Clear`, `Assign('All Objects')`,
/// `Assign(RoleId)`), and each `Assign` reads the `Permission Set` table first.
const PERMISSIONS_MOCK: u32 = 131_006;

/// The refusal of a test that lowers its permissions while no access kind is enforced.
pub(crate) const LOWERED: &str = "DotNet PermissionTestHelper.AddEffectivePermissionSet lowers the permissions of the test session; \
     permissions are not enforced here (every access is granted, ADR-027), so the test is not run \
     (`--ignore-permission-mock` runs it without permission checks, as a flagged pass)";

/// What a list of permission sets grants.
#[derive(Clone, Default, Debug, PartialEq)]
pub(crate) struct Content {
    pub wild: Wildcards,
    /// (object type ordinal, object id, levels), sorted by type and id.
    pub explicit: Vec<(i32, u32, Levels)>,
}

fn merge(a: u8, b: u8) -> u8 {
    if a == 1 || b == 1 { 1 } else { a.max(b) }
}

/// The order of the levels: blank, Indirect, Yes.
fn rank(level: u8) -> u8 {
    match level {
        1 => 2,
        2 => 1,
        _ => 0,
    }
}

/// `have` with `excluded` taken away, for one permission (Y19PermComposition.al:124-129, :151-162, P11): an excluded
/// Yes removes the permission, an excluded Indirect leaves Indirect of a Yes. `None` for Indirect minus Indirect, which
/// no probe pins.
fn subtract_level(have: u8, excluded: u8) -> Option<u8> {
    match (have, excluded) {
        (h, 0) => Some(h),
        (_, 1) | (0, _) => Some(0),
        (1, 2) => Some(2),
        _ => None,
    }
}

impl Content {
    /// The level the content gives permission `p` on object `(kind, id)`: the better of the kind's wildcard and the
    /// object's own grant.
    pub(crate) fn level_of(&self, kind: i32, id: u32, p: usize) -> u8 {
        let wild = kind_slot(kind).map_or(0, |slot| self.wild[slot][p]);
        let own = self
            .explicit
            .binary_search_by_key(&(kind, id), |&(k, id, _)| (k, id))
            .map_or(0, |i| self.explicit[i].2[p]);
        merge(wild, own)
    }

    /// Takes away what the excluded sets grant (`ExcludedPermissionSets`, ADR-062 section 3, probe P11), per permission.
    /// Where the result is not pinned or cannot be held (a wildcard minus one object), it keeps less than BC may: the
    /// content stays a lower bound, and the error names why.
    pub(crate) fn subtract(&mut self, ex: &Content) -> Result<(), String> {
        let mut why = None;
        let mut wild = self.wild;
        for (slot, lv) in wild.iter_mut().enumerate() {
            for (p, h) in lv.iter_mut().enumerate() {
                *h = subtract_level(*h, ex.wild[slot][p]).unwrap_or_else(|| {
                    why = Some("an excluded set takes an indirect permission from an indirect one (not pinned)");
                    0
                });
            }
        }
        let mut ids: Vec<(i32, u32)> = self.explicit.iter().chain(&ex.explicit).map(|&(k, id, _)| (k, id)).collect();
        ids.sort_unstable();
        ids.dedup();
        let mut explicit = Vec::new();
        for (k, id) in ids {
            let mut lv = [0u8; 5];
            for (p, l) in lv.iter_mut().enumerate() {
                *l = subtract_level(self.level_of(k, id, p), ex.level_of(k, id, p)).unwrap_or_else(|| {
                    why = Some("an excluded set takes an indirect permission from an indirect one (not pinned)");
                    0
                });
                // A wildcard that grants more than this object keeps: the wildcard cannot leave one object out.
                if let Some(slot) = kind_slot(k)
                    && rank(wild[slot][p]) > rank(*l)
                {
                    wild[slot][p] = 0;
                    why = Some("an excluded set takes one object out of a wildcard grant (not held here)");
                }
            }
            if lv != [0; 5] {
                explicit.push((k, id, lv));
            }
        }
        self.wild = wild;
        self.explicit = explicit;
        why.map_or(Ok(()), |w| Err(w.to_string()))
    }

    /// The content without entries that grant nothing, to compare two contents.
    fn normalized(mut self) -> Content {
        self.explicit.retain(|e| e.2 != [0; 5]);
        self
    }

    /// Adds what `other` grants: per permission, Yes beats Indirect beats blank (ADR-062 section 3).
    pub(crate) fn union(&mut self, other: &Content) {
        for (have, add) in self.wild.iter_mut().zip(&other.wild) {
            for (h, a) in have.iter_mut().zip(add) {
                *h = merge(*h, *a);
            }
        }
        for &(k, id, lv) in &other.explicit {
            match self.explicit.binary_search_by_key(&(k, id), |&(k, id, _)| (k, id)) {
                Ok(i) => {
                    for (h, a) in self.explicit[i].2.iter_mut().zip(lv) {
                        *h = merge(*h, a);
                    }
                }
                Err(i) => self.explicit.insert(i, (k, id, lv)),
            }
        }
    }

    /// Whether every object of every kind is granted at Yes: Read, Insert, Modify and Delete on all table data, and
    /// Execute on every table, report, codeunit, xmlport, page, query and system object. SUPER grants this.
    pub(crate) fn grants_everything(&self) -> bool {
        let [data, rest @ ..] = &self.wild;
        data[..4].iter().all(|&l| l == 1) && rest.iter().all(|l| l[4] == 1)
    }
}

/// What the assigned sets of a session give.
#[derive(Debug)]
pub(crate) struct EffectivePermissions {
    /// The union of the sets that composed. A lower bound of what BC grants when [`EffectivePermissions::complete`] is
    /// false: a set that is not composed adds nothing here, and an extension's grants are not merged.
    pub content: Content,
    /// Every set of the list composed in full. False when a set is unknown, ambiguous, uses a grant or exclusion the
    /// reader does not take, or is extended (`why_incomplete`).
    pub complete: bool,
    pub why_incomplete: Vec<String>,
}

/// The permission set objects of the program, read once per engine, and the composed lists, shared by every session
/// (the runner assigns the same sets before thousands of tests). Also the object grants and inherent attributes, read
/// on first use.
pub(crate) struct SetModel {
    sets: Vec<crate::platform::RawSet>,
    by_name: HashMap<String, Vec<usize>>,
    by_role: HashMap<String, Vec<usize>>,
    composed: Mutex<HashMap<Vec<String>, Option<Arc<EffectivePermissions>>>>,
    /// Per object index: its `Permissions` property.
    grants: Mutex<HashMap<usize, Arc<Result<Content, String>>>>,
    /// Per (object index, routine): its `[InherentPermissions]` attributes.
    inherent: Mutex<HashMap<(usize, usize), InherentGrants>>,
}

type InherentGrants = Arc<Vec<Result<Option<(i32, u32, Levels)>, String>>>;

impl SetModel {
    pub(crate) fn of(engine: &Engine) -> Arc<SetModel> {
        engine.derived(|| {
            let (sets, by_name) = crate::platform::raw_sets(engine);
            let mut by_role: HashMap<String, Vec<usize>> = HashMap::new();
            for (i, s) in sets.iter().enumerate() {
                by_role.entry(s.role.clone()).or_default().push(i);
            }
            SetModel {
                sets,
                by_name,
                by_role,
                composed: Mutex::new(HashMap::new()),
                grants: Mutex::new(HashMap::new()),
                inherent: Mutex::new(HashMap::new()),
            }
        })
    }

    fn one_set(&self, name: &str) -> Option<usize> {
        match self.by_name.get(name).map(Vec::as_slice) {
            Some([t]) => Some(*t),
            _ => None,
        }
    }

    /// Set `i` with its included sets merged in and, with `exclude`, its excluded sets taken away (P11: an excluded set's
    /// own includes are excluded too, Y19PermComposition.al:126-127, :151-156). A reason goes to `why` wherever the
    /// result is only a lower bound of what BC grants.
    fn expanded(
        &self,
        i: usize,
        exclude: bool,
        visiting: &mut Vec<usize>,
        why: &mut Vec<String>,
    ) -> Result<Content, String> {
        let set = &self.sets[i];
        if visiting.contains(&i) {
            return Err(format!("permission set '{}' includes itself", set.role));
        }
        let mut all = set.declared.clone()?;
        if let Some(w) = &set.incomplete {
            why.push(w.clone());
        }
        visiting.push(i);
        for inc in &set.includes {
            match self.one_set(inc) {
                // An included set that excludes sets: whether its exclusion reaches beyond it is not pinned, so it adds
                // nothing here.
                Some(t) if !self.sets[t].excludes.is_empty() => why.push(format!(
                    "permission set '{}' includes `{inc}`, which excludes sets (not pinned)",
                    set.role
                )),
                Some(t) => match self.expanded(t, true, visiting, why) {
                    Ok(c) => all.union(&c),
                    Err(e) => why.push(e),
                },
                None => why.push(format!(
                    "permission set '{}' includes `{inc}`, which is not one set",
                    set.role
                )),
            }
        }
        if exclude {
            for ex in &set.excludes {
                let taken = match self.one_set(ex) {
                    Some(t) if self.sets[t].excludes.is_empty() => self.expanded(t, true, visiting, why),
                    Some(_) => Err(format!("permission set '{}' excludes `{ex}`, which excludes sets too (not pinned)", set.role)),
                    None => Err(format!("permission set '{}' excludes `{ex}`, which is not one set", set.role)),
                };
                match taken {
                    Ok(c) => {
                        if let Err(w) = all.subtract(&c) {
                            why.push(format!("permission set '{}': {w}", set.role));
                        }
                    }
                    // What is excluded is not known: nothing of the set is kept.
                    Err(e) => {
                        why.push(e);
                        all = Content::default();
                    }
                }
            }
        }
        visiting.pop();
        Ok(all)
    }

    /// The session level of a session whose assigned sets are `roles` (Role IDs, in assignment order): `None` when the
    /// union grants everything (unrestricted), else what it gives.
    ///
    /// The union of the sets (pin 2 of ADR-062), each with its own exclusions taken away. Whether one assigned set's
    /// exclusions also take from another assigned set is not pinned; where the two readings differ, the session holds the
    /// smaller (exclusions taken from the union) and is incomplete. A set that cannot be composed adds nothing, so the
    /// content is always a lower bound of what BC grants, and complete only when every set composed in full.
    pub(crate) fn compose(&self, roles: &[String]) -> Option<Arc<EffectivePermissions>> {
        let mut key = roles.to_vec();
        key.sort_unstable();
        key.dedup();
        let lock = || self.composed.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(known) = lock().get(&key) {
            return known.clone();
        }
        let mut content = Content::default();
        let mut why = Vec::new();
        let mut found = Vec::new();
        for role in &key {
            match self.by_role.get(role).map(Vec::as_slice) {
                Some([i]) => {
                    found.push(*i);
                    match self.expanded(*i, true, &mut Vec::new(), &mut why) {
                        Ok(c) => content.union(&c),
                        Err(e) => why.push(e),
                    }
                }
                Some(_) => why.push(format!(
                    "more than one permission set has the Role ID '{role}' (ADR-062 probe P9)"
                )),
                None => why.push(format!("no permission set of the program has the Role ID '{role}'")),
            }
        }
        if found.len() > 1 && found.iter().any(|&i| !self.sets[i].excludes.is_empty()) {
            let mut whole = Content::default();
            let mut taken = Content::default();
            let mut ignored = Vec::new();
            for &i in &found {
                if let Ok(c) = self.expanded(i, false, &mut Vec::new(), &mut ignored) {
                    whole.union(&c);
                }
                for ex in &self.sets[i].excludes {
                    if let Some(Ok(c)) = self.one_set(ex).map(|t| self.expanded(t, true, &mut Vec::new(), &mut ignored)) {
                        taken.union(&c);
                    }
                }
            }
            let _ = whole.subtract(&taken);
            let whole = whole.normalized();
            if whole != content.clone().normalized() {
                why.push("an assigned set excludes what another assigned set grants (not pinned)".into());
                content = whole;
            }
        }
        let result = (!content.grants_everything()).then(|| {
            Arc::new(EffectivePermissions {
                content,
                complete: why.is_empty(),
                why_incomplete: why,
            })
        });
        lock().insert(key, result.clone());
        result
    }

    /// The `Permissions` property of object `object` (read once per engine).
    fn object_grants(&self, engine: &Engine, object: usize) -> Arc<Result<Content, String>> {
        let lock = || self.grants.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(g) = lock().get(&object) {
            return g.clone();
        }
        let g = Arc::new(crate::platform::object_grants(engine, object));
        lock().insert(object, g.clone());
        g
    }

    /// The `[InherentPermissions]` attributes of routine `routine` of object `object`.
    fn inherent_of(&self, engine: &Engine, object: usize, routine: usize) -> InherentGrants {
        let lock = || self.inherent.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(g) = lock().get(&(object, routine)) {
            return g.clone();
        }
        let list: Vec<_> = engine.program.objects[object]
            .routines
            .get(routine)
            .map(|r| {
                r.attributes
                    .iter()
                    .filter(|a| a.name.eq_ignore_ascii_case("InherentPermissions"))
                    .map(|a| crate::platform::inherent_attribute(engine, &a.text))
                    .collect()
            })
            .unwrap_or_default();
        let g = Arc::new(list);
        lock().insert((object, routine), g.clone());
        g
    }
}

/// The five permissions of a grant, in the order of [`Levels`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Op {
    Read = 0,
    Insert = 1,
    Modify = 2,
    Delete = 3,
    Execute = 4,
}

impl Op {
    /// The permission in plain words, for messages.
    fn describe(self) -> &'static str {
        match self {
            Op::Read => "read",
            Op::Insert => "insert",
            Op::Modify => "modify",
            Op::Delete => "delete",
            Op::Execute => "execute",
        }
    }
}

/// One access a check decides: permission `op` on object `(kind, id)`, `kind` the "Object Type" ordinal
/// ([`kind_slot`]).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Access {
    pub kind: i32,
    pub id: u32,
    pub op: Op,
}

/// The stages of ADR-062 section 5 that check accesses. Each turns on only once its probes are merged; until then a
/// restricted session refuses the test at the first access of that stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Stage {
    /// TableData Insert, Modify, Delete (probes P1-P13).
    Writes,
    /// TableData Read, FlowFields, ReadPermission/WritePermission (P14-P19).
    Reads,
    /// Execute on every object kind, AccessByPermission, effective-permission queries (P20-P22).
    Execute,
}

impl Stage {
    pub(crate) fn of(op: Op) -> Stage {
        match op {
            Op::Insert | Op::Modify | Op::Delete => Stage::Writes,
            Op::Read => Stage::Reads,
            Op::Execute => Stage::Execute,
        }
    }

    /// Whether the stage raises the pinned denials of its access kind unless `--permissions off` says otherwise.
    /// Writes is on: its probes P1-P13 are merged in the corpus (master, PR 914). An access the sets allow runs in every
    /// stage (an allowed access needs no probe, ADR-062 section 2); a denial of a stage that is off refuses the test.
    pub(crate) const fn on_by_default(self) -> bool {
        match self {
            Stage::Writes => true,
            Stage::Reads | Stage::Execute => false,
        }
    }
}

/// The result of the decision function.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Decision {
    Allow,
    /// Denied; `indirect` when the object whose code runs holds an object grant for the access, so BC names the
    /// operation `Indirect<Op>` (ADR-062 "Error texts and codes").
    Deny {
        indirect: bool,
    },
}

/// ADR-062 section 1, rule 4, for a session level `u` (0 blank, 1 Yes, 2 Indirect) and whether the object whose code
/// runs grants the access in its `Permissions` property (`object_grant`): allowed when `u` is Yes, or when `u` is
/// Indirect and the object grant holds (pin 4: lt-permissions-mock-indirect/TestIndirect.al:80-103,
/// lt-cp-permission-leads/CpPermLeads.al:65-117). The license level is Yes everywhere under the Cronus license and is
/// not part of this skeleton (ADR-060 decision 6).
pub(crate) fn decide(u: u8, object_grant: bool) -> Decision {
    match (u, object_grant) {
        (1, _) | (2, true) => Decision::Allow,
        (_, indirect) => Decision::Deny { indirect },
    }
}

/// The access of a record method; `None` for one that touches no row.
pub(crate) fn record_access(method: al_builtins::record_op::RecordOp) -> Option<Op> {
    use al_builtins::record_op::RecordOp as R;
    Some(match method {
        R::Insert => Op::Insert,
        R::Modify | R::ModifyAll | R::Rename => Op::Modify,
        R::Delete | R::DeleteAll | R::Truncate => Op::Delete,
        R::Get
        | R::GetBySystemId
        | R::Find
        | R::FindFirst
        | R::FindLast
        | R::FindSet
        | R::Next
        | R::Count
        | R::CountApprox
        | R::IsEmpty
        | R::CalcFields
        | R::CalcSums => Op::Read,
        _ => return None,
    })
}

/// The object an Execute access names for a call of a routine of an object of `kind`: an extension's code counts as
/// its base object's. Kinds without an Execute permission (enums, interfaces, permission sets) map to type ordinal -1,
/// which no grant names.
fn execute_target(kind: al_ir::ObjKind, id: Option<u32>, extends: Option<u32>) -> (i32, u32) {
    use al_ir::ObjKind as K;
    let (code, own) = match kind {
        K::Table => (1, true),
        K::TableExtension => (1, false),
        K::Report => (3, true),
        K::ReportExtension => (3, false),
        K::Codeunit => (5, true),
        K::XmlPort => (6, true),
        K::Page => (8, true),
        K::PageExtension => (8, false),
        K::Query => (9, true),
        _ => (-1, true),
    };
    let id = if own { id } else { extends };
    (code, id.unwrap_or(0))
}

impl EffectivePermissions {
    /// The session level for `access`: the better of the kind's wildcard and the object's own grant.
    pub(crate) fn level(&self, access: Access) -> u8 {
        if kind_slot(access.kind).is_none() {
            return 0;
        }
        self.content.level_of(access.kind, access.id, access.op as usize)
    }
}

/// AllObj and Field: a narrow set reads them (Y19PermSystemTables.al, P13).
const ALWAYS_READABLE: [u32; 2] = [2_000_000_038, 2_000_000_041];
/// Tenant Media: the one system table whose write denial is pinned (Y19PermSystemTables.al, P13: Insert error).
const TENANT_MEDIA: u32 = 2_000_000_184;

/// What the decision function says about one access of a restricted session.
enum Verdict {
    Allow,
    /// Denied by complete inputs: BC raises its permission error. `by` is the object whose code makes the access.
    Deny { indirect: bool, by: usize },
    /// The outcome is not known: the test is not run.
    Refuse(String),
}

impl Session {
    /// `PermissionTestHelper.AddEffectivePermissionSet(RoleId)`: adds the set to the session's assigned sets.
    pub(crate) fn assign_permission_set(&mut self, role: &str) {
        let role: String = role.to_uppercase().chars().take(20).collect();
        if !self.permission_roles.contains(&role) {
            self.permission_roles.push(role);
        }
        self.permissions = SetModel::of(&self.engine).compose(&self.permission_roles);
        if self.permissions.is_none() {
            assume(Assumption::PermissionSetsGrantingEverythingUnrestricted);
        } else if self.runner_permission_sets {
            // The runner's sets are not composed: the session holds at least what the test's own sets give, and an
            // access they do not grant may be granted by the runner's (ADR-062 stage 4, probe P25).
            let eff = self.permissions.take().expect("restricted");
            let mut why = eff.why_incomplete.clone();
            why.push(
                "the sets were assigned on top of the test runner's permission sets, which are not composed (ADR-062 probe P25, stage 4)"
                    .into(),
            );
            self.permissions = Some(Arc::new(EffectivePermissions {
                content: eff.content.clone(),
                complete: false,
                why_incomplete: why,
            }));
        }
    }

    /// The constructor, `Clear` and `Dispose` of `PermissionTestHelper`, and the start of each test method: no set is
    /// assigned, the user's own permissions apply (unrestricted).
    pub(crate) fn clear_permission_sets(&mut self) {
        self.permission_roles.clear();
        self.permissions = None;
        self.runner_permission_sets = false;
    }

    /// The decision function of ADR-062 section 1 for an access of a restricted session (the caller checked
    /// `self.permissions.is_some()`). `target` is the object index of the accessed object (the table of a TableData
    /// access, the called object of an Execute access); `flowfield` makes
    /// the object grant hold (a FlowField calculation counts as an indirect read, CpPermLeads.al:29-63).
    ///
    /// The code of "Permissions Mock" itself is not checked: between its `Assign` calls the list is not final
    /// (`Set('SUPER')` assigns All Objects first), and Microsoft's mock runs on BC under every list it builds
    /// (TestMockBasics.al:20-79, 143-163). Nor is a call the runner makes from outside AL code (its steps around the
    /// test method, al-testrun `runner_reset.rs`).
    fn permission_verdict(&mut self, access: Access, target: Option<usize>, flowfield: bool) -> Verdict {
        let Some(&caller) = self.frames.last() else {
            return Verdict::Allow;
        };
        let engine = self.engine.clone();
        let o = &engine.program.objects.heads()[caller];
        if o.kind == al_ir::ObjKind::Codeunit && o.id == Some(PERMISSIONS_MOCK) {
            return Verdict::Allow;
        }
        if self.ignore_permission_mock {
            assume(Assumption::PermissionsNotEnforced);
            return Verdict::Allow;
        }
        if self.permissions_off {
            return Verdict::Refuse(LOWERED.into());
        }
        let Some(eff) = self.permissions.clone() else {
            return Verdict::Allow;
        };
        let p = access.op as usize;
        let mut u = eff.level(access);
        if u == 1 {
            return Verdict::Allow;
        }
        let model = SetModel::of(&engine);
        // Inherent permissions merge into the session level (pin 5): the table's property and the attributes of every
        // routine on the stack, each for that call and the calls it makes (CpPermLeads.al:155-189). Only Yes is pinned.
        let mut unpinned: Option<String> = None;
        // The level the accessed object's own InherentPermissions adds where no probe pins it (an indirect letter on a
        // table, Execute on any object): used only when the access is not allowed without it, under assumption
        // `inherent-permissions-indirect-and-objects`.
        let mut assumed = 0;
        if let Some(t) = target {
            match crate::platform::table_inherent(&engine, t) {
                Some(Ok(l)) if access.kind == 0 && l[p] == 1 => u = merge(u, 1),
                Some(Ok(l)) => assumed = l[p],
                Some(Err(why)) => unpinned = Some(why),
                None => {}
            }
        }
        for (&object, frame) in self.frames.iter().zip(&self.call_stack) {
            for g in model.inherent_of(&engine, object, frame.routine).iter() {
                match g {
                    Ok(Some((k, id, l))) if *k == access.kind && *id == access.id => {
                        if l[p] == 2 {
                            unpinned = Some("an indirect [InherentPermissions] attribute".into());
                        } else {
                            u = merge(u, l[p]);
                        }
                    }
                    Ok(_) => {}
                    Err(why) => unpinned = Some(why.clone()),
                }
            }
        }
        if u == 1 {
            return Verdict::Allow;
        }
        // The object grant `G`: the caller's own `Permissions` property (pin 4: Y19PermComposition.al:45-58, the frame's
        // object only). Code of an extension object: which property applies is not pinned.
        let grant = if flowfield {
            true
        } else if o.kind.extended().is_some() {
            return Verdict::Refuse(format!(
                "the code of {} {} makes an access the assigned permission sets do not grant directly; which Permissions property covers an extension object's code is not pinned (ADR-062)",
                o.kind,
                o.name
            ));
        } else {
            match &*model.object_grants(&engine, caller) {
                Ok(c) => c.level_of(access.kind, access.id, p) != 0,
                Err(why) => return Verdict::Refuse(format!("the Permissions property of {} {} is not read: {why}", o.kind, o.name)),
            }
        };
        let decision = match decide(u, grant) {
            Decision::Deny { .. } if assumed != 0 && decide(merge(u, assumed), grant) == Decision::Allow => {
                assume(Assumption::InherentPermissionsIndirectAndObjects);
                Decision::Allow
            }
            d => d,
        };
        match decision {
            Decision::Allow => Verdict::Allow,
            Decision::Deny { .. } if unpinned.is_some() => Verdict::Refuse(format!(
                "a permission denial ({}) where an inherent permission is not read: {}",
                access.describe(),
                unpinned.unwrap_or_default()
            )),
            Decision::Deny { .. } if !eff.complete => Verdict::Refuse(format!(
                "a permission denial ({}) under assigned permission sets that are not fully composed, so BC may allow it: {}",
                access.describe(),
                eff.why_incomplete.join("; ")
            )),
            Decision::Deny { indirect } => Verdict::Deny { indirect, by: caller },
        }
    }

    /// The gate for a call of a routine of `object` (an Execute access, stage 3): an allowed call runs; a denial is not
    /// pinned yet (probe P20), so it refuses the test.
    #[cold]
    #[inline(never)]
    pub(crate) fn permission_gate_call(&mut self, object: usize) -> Result<(), Signal> {
        let o = &self.engine.program.objects.heads()[object];
        let (kind, id) = execute_target(o.kind, o.id, o.extends_id);
        if kind < 0 {
            return Ok(());
        }
        let access = Access {
            kind,
            id,
            op: Op::Execute,
        };
        match self.permission_verdict(access, Some(object), false) {
            Verdict::Allow => Ok(()),
            Verdict::Refuse(why) => Err(Signal::Unsupported(why)),
            Verdict::Deny { .. } => Err(Signal::Unsupported(unpinned_denial(access, "ADR-062 stage 3, probe P20"))),
        }
    }

    /// The gate for the calculation of a FlowField over table `table` (object index): an indirect read the session level
    /// allows runs (pin 6); a denial is stage 2 (probe P15), so it refuses the test.
    #[cold]
    #[inline(never)]
    pub(crate) fn permission_gate_flowfield(&mut self, table: usize) -> Result<(), Signal> {
        let id = self.engine.program.objects.heads()[table].id.unwrap_or(0);
        let access = Access {
            kind: 0,
            id,
            op: Op::Read,
        };
        match self.permission_verdict(access, Some(table), true) {
            Verdict::Allow => Ok(()),
            Verdict::Refuse(why) => Err(Signal::Unsupported(why)),
            Verdict::Deny { .. } => Err(Signal::Unsupported(unpinned_denial(access, "ADR-062 stage 2, probe P15"))),
        }
    }

    /// The gate for record method `method` on `rec`; a method that reads or writes no row (filters, keys, field
    /// metadata) is no access. `from_al`: AL code calls the method (a Record or RecordRef method), not the runtime on
    /// behalf of a page, report, xmlport or validation.
    #[cold]
    #[inline(never)]
    pub(crate) fn permission_gate_record(
        &mut self,
        method: al_builtins::record_op::RecordOp,
        rec: &crate::records::Rec,
        args: &[al_builtins::value::Value],
        from_al: bool,
    ) -> Result<(), Signal> {
        let Some(op) = record_access(method) else {
            return Ok(());
        };
        let (object, temporary) = {
            let r = rec.borrow();
            (r.table as usize, r.temporary)
        };
        // A temporary record is never checked, its writes nor its Count: Y19PermWrites.al Y19_P6_Temporary (Temporary
        // variable, TableType = Temporary, RecordRef.Open(.., true); ops 64-70 under a set without a grant).
        if temporary {
            return Ok(());
        }
        let id = self.engine.program.objects.heads().get(object).and_then(|o| o.id).unwrap_or(0);
        if op == Op::Read && ALWAYS_READABLE.contains(&id) {
            return Ok(());
        }
        let access = Access { kind: 0, id, op };
        match self.permission_verdict(access, Some(object), false) {
            Verdict::Allow => Ok(()),
            Verdict::Refuse(why) => Err(Signal::Unsupported(why)),
            Verdict::Deny { indirect, by } => self.write_denied(method, rec, args, access, indirect, by, from_al),
        }
    }

    /// A denied TableData access: BC's error where its shape is pinned, else the refusal (the safety rule of ADR-062
    /// section 2).
    ///
    /// `Sorry, the current permissions prevented the action. (TableData <id> <name> <Op>: <app>)` with the code
    /// `DB:Client<Op>Denied`: Insert, Modify and Delete (TestMockBasics.al:72-78), Rename as Modify (Y19PermWrites.al
    /// Y19_P1_Rename), DeleteAll and ModifyAll as Delete and Modify on a table with rows, whatever the filter matches
    /// (Y19_P2_SetR, Y19_P2_SetRI), RecordRef writes the same (Y19_P6_RecordRef), trigger writes (Y19_P3_*);
    /// `Indirect<Op>` when the code's object grants the access (Y19_P4_IndirectDeleteAndModify, TestIndirect.al:80-103);
    /// `<app>` the app of the code (Y19PermCrossApp.al). DeleteAll and ModifyAll without triggers on an empty table raise
    /// nothing (TestMockBasics.al:47-50).
    #[allow(clippy::too_many_arguments)]
    fn write_denied(
        &mut self,
        method: al_builtins::record_op::RecordOp,
        rec: &crate::records::Rec,
        args: &[al_builtins::value::Value],
        access: Access,
        indirect: bool,
        by: usize,
        from_al: bool,
    ) -> Result<(), Signal> {
        use al_builtins::record_op::RecordOp as R;
        use al_builtins::value::Value;
        let refuse = |why: &str| Err(Signal::Unsupported(unpinned_denial(access, why)));
        if !Stage::of(access.op).on_by_default() {
            return refuse(match access.op {
                Op::Read => "ADR-062 stage 2, probes P14-P19",
                _ => "ADR-062 stage 3, probes P20-P22",
            });
        }
        if !from_al {
            return refuse("made by the runtime for a page, report, xmlport or validation; ADR-062 probes P16-P18");
        }
        if access.id >= 2_000_000_000 && access.id != TENANT_MEDIA {
            return refuse("on a system table; probe P13 pins Tenant Media only");
        }
        let word = match access.op {
            Op::Insert => "Insert",
            Op::Modify => "Modify",
            Op::Delete => "Delete",
            _ => return refuse("ADR-062 section 2"),
        };
        match method {
            R::Insert if self.row_exists(rec)? => return refuse("an Insert of a key that exists (which error comes first is not pinned)"),
            R::Modify | R::Delete | R::Rename if !self.row_exists(rec)? => {
                return refuse("on a row that does not exist (which error comes first is not pinned)");
            }
            R::Insert | R::Modify | R::Delete | R::Rename => {}
            R::ModifyAll | R::DeleteAll => {
                let t = self.table(rec)?;
                let cx = self.cx_for(rec, &t)?;
                let rows = al_data::Store::count(&*self.store_for(rec), &cx, &t, &[])
                    .map_err(|e| Signal::Unsupported(format!("store error {e}")))?;
                let triggers = match method {
                    R::ModifyAll => args.len() > 2 && matches!(args.last(), Some(Value::Boolean(true))),
                    _ => matches!(args.last(), Some(Value::Boolean(true))),
                };
                if rows == 0 {
                    return if triggers {
                        refuse("on an empty table with triggers (TestMockBasics.al:47-50 pins it without)")
                    } else {
                        Ok(())
                    };
                }
            }
            _ => return refuse("ADR-062 section 2"),
        }
        let heads = self.engine.program.objects.heads();
        let name = &heads[rec.borrow().table as usize].name;
        let app = self
            .engine
            .program
            .apps
            .get(heads[by].app as usize)
            .map(|a| a.name.as_str())
            .unwrap_or_default();
        let op = if indirect { format!("Indirect{word}") } else { word.to_string() };
        Err(crate::transactions::al_error_code(
            &format!(
                "Sorry, the current permissions prevented the action. (TableData {} {name} {op}: {app})",
                access.id
            ),
            &format!("DB:Client{word}Denied"),
        ))
    }

    /// Whether the row with `rec`'s primary key is stored.
    fn row_exists(&mut self, rec: &crate::records::Rec) -> Result<bool, Signal> {
        let t = self.table(rec)?;
        let cx = self.cx_for(rec, &t)?;
        let pk: Vec<al_builtins::value::Value> = {
            let r = rec.borrow();
            t.keys[0].iter().map(|&i| r.fields[i].clone()).collect()
        };
        let found = al_data::Store::get(&*self.store_for(rec), &cx, &t, &pk)
            .map_err(|e| Signal::Unsupported(format!("store error {e}")))?;
        Ok(found.is_some())
    }

    /// At the end of a test method: the refusal when the session is still restricted and `--permissions off` turned the
    /// checks off (the test lowered its permissions and made no access the gate refused since). With the checks on,
    /// every access the test made was decided, so it ends as it ran. Under `--ignore-permission-mock` the test is
    /// flagged instead.
    pub fn permissions_lowered_at_end(&mut self) -> Option<String> {
        self.permissions.as_ref()?;
        if self.ignore_permission_mock {
            assume(Assumption::PermissionsNotEnforced);
            return None;
        }
        if !self.permissions_off && Stage::Writes.on_by_default() {
            return None;
        }
        Some(LOWERED.into())
    }
}

/// The refusal of a denial whose error is not pinned.
fn unpinned_denial(access: Access, probe: &str) -> String {
    format!(
        "a permission denial ({}) whose error is not pinned yet ({probe}); the test is not run",
        access.describe()
    )
}

impl Access {
    /// The access in plain words, for messages.
    fn describe(self) -> String {
        format!("{} permission on object type {} id {}", self.op.describe(), self.kind, self.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_kinds(levels: Levels) -> Wildcards {
        [levels; KINDS]
    }

    #[test]
    fn rimd_on_table_data_and_execute_on_every_kind_grant_everything() {
        let mut c = Content::default();
        c.wild[0] = [1, 1, 1, 1, 0];
        assert!(!c.grants_everything(), "SUPER (DATA) alone grants no Execute");
        for slot in 1..KINDS {
            c.wild[slot][4] = 1;
        }
        assert!(c.grants_everything());
        c.wild[7][4] = 0;
        assert!(!c.grants_everything(), "without system * = X");
    }

    #[test]
    fn an_indirect_wildcard_does_not_grant_everything() {
        let c = Content {
            wild: all_kinds([2, 2, 2, 2, 2]),
            explicit: vec![],
        };
        assert!(!c.grants_everything());
    }

    /// ADR-062 section 1 rule 4, one row per pinned case of pin 4.
    #[test]
    fn the_decision_allows_yes_and_indirect_with_an_object_grant() {
        // User with `rim`, codeunit with the property: allowed (CpPermLeads.al:65-117).
        assert_eq!(decide(2, true), Decision::Allow);
        // Direct grant: allowed with or without the property.
        assert_eq!(decide(1, false), Decision::Allow);
        assert_eq!(decide(1, true), Decision::Allow);
        // User with nothing, codeunit with `i`: IndirectInsert (TestIndirect.al:80-103).
        assert_eq!(decide(0, true), Decision::Deny { indirect: true });
        // Codeunit without the property: Insert, also when the user has only the indirect grant.
        assert_eq!(decide(0, false), Decision::Deny { indirect: false });
        assert_eq!(decide(2, false), Decision::Deny { indirect: false });
    }

    #[test]
    fn the_level_is_the_better_of_the_wildcard_and_the_object_grant() {
        let mut content = Content {
            wild: Wildcards::default(),
            explicit: vec![(0, 50100, [1, 2, 0, 0, 0])],
        };
        content.wild[0] = [2, 0, 0, 0, 0];
        let e = EffectivePermissions {
            content,
            complete: true,
            why_incomplete: vec![],
        };
        let at = |id, op| e.level(Access { kind: 0, id, op });
        assert_eq!(at(50100, Op::Read), 1);
        assert_eq!(at(50100, Op::Insert), 2);
        assert_eq!(at(50100, Op::Modify), 0);
        assert_eq!(at(7, Op::Read), 2, "the wildcard");
        assert_eq!(
            e.level(Access {
                kind: 5,
                id: 7,
                op: Op::Execute
            }),
            0
        );
        assert_eq!(
            e.level(Access {
                kind: 99,
                id: 7,
                op: Op::Read
            }),
            0,
            "no such type"
        );
    }

    /// Writes is on once probes P1-P13 landed; reads and Execute wait for theirs (ADR-062 section 5).
    #[test]
    fn only_the_write_stage_is_on_by_default() {
        for op in [Op::Insert, Op::Modify, Op::Delete] {
            assert!(Stage::of(op).on_by_default(), "{op:?}");
        }
        for op in [Op::Read, Op::Execute] {
            assert!(!Stage::of(op).on_by_default(), "{op:?}");
        }
    }

    #[test]
    fn record_methods_map_to_their_access() {
        use al_builtins::record_op::RecordOp as R;
        assert_eq!(record_access(R::Insert), Some(Op::Insert));
        assert_eq!(record_access(R::ModifyAll), Some(Op::Modify));
        assert_eq!(record_access(R::DeleteAll), Some(Op::Delete));
        assert_eq!(record_access(R::FindSet), Some(Op::Read));
        assert_eq!(record_access(R::SetRange), None);
        assert_eq!(record_access(R::Init), None);
    }

    #[test]
    fn an_extension_executes_as_its_base_object() {
        use al_ir::ObjKind as K;
        assert_eq!(execute_target(K::PageExtension, Some(50100), Some(21)), (8, 21));
        assert_eq!(execute_target(K::Codeunit, Some(80), None), (5, 80));
        assert_eq!(execute_target(K::Enum, Some(3), None).0, -1);
    }

    #[test]
    fn the_union_keeps_the_best_level_per_permission() {
        let mut a = Content {
            wild: Wildcards::default(),
            explicit: vec![(0, 5, [2, 0, 0, 0, 0])],
        };
        let b = Content {
            wild: Wildcards::default(),
            explicit: vec![(0, 3, [0, 1, 0, 0, 0]), (0, 5, [1, 2, 0, 0, 0])],
        };
        a.union(&b);
        assert_eq!(a.explicit, vec![(0, 3, [0, 1, 0, 0, 0]), (0, 5, [1, 2, 0, 0, 0])]);
    }
}
