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
pub(crate) const PERMISSIONS_MOCK: u32 = 131_006;

/// The system objects whose `GetEffectivePermission` answer the corpus measured (Y90PermSystem.al:217-241, Y93WildEffective.al):
/// Execute follows `system * = X`, and with no set assigned every one has it. An id outside the list is not measured (id 0 has
/// nothing, session-system/W6EnvironmentHelpers.al:116-117), so it refuses.
pub(crate) const SYSTEM_OBJECTS_MEASURED: [u32; 6] = [5310, 5810, 5820, 6110, 9026, 9100];

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
    /// The grants on one named system object (`system "Tools, Security, Roles" = X`), which the program holds no id
    /// for: what they give a System access is not known; they give nothing else (al-runner issue 6717).
    pub unread_system: Vec<String>,
    /// (object type ordinal, object id, levels): the levels the kind's wildcard gives one object after an excluded set took
    /// permissions of that object out of it (Y93WildScope.al:142-162, `LT Y93 W Excl`), sorted by type and id. An object not
    /// listed gets the kind's wildcard.
    pub cut: Vec<(i32, u32, Levels)>,
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
        let wild = self.wild_level(kind, id, p);
        let own = self
            .explicit
            .binary_search_by_key(&(kind, id), |&(k, id, _)| (k, id))
            .map_or(0, |i| self.explicit[i].2[p]);
        merge(wild, own)
    }

    /// The level the kind's wildcard gives permission `p` on object `(kind, id)`, after the exclusions of [`Content::cut`].
    fn wild_level(&self, kind: i32, id: u32, p: usize) -> u8 {
        if !self.cut.is_empty()
            && let Ok(i) = self.cut.binary_search_by_key(&(kind, id), |&(k, id, _)| (k, id))
        {
            return self.cut[i].2[p];
        }
        kind_slot(kind).map_or(0, |slot| self.wild[slot][p])
    }

    /// The level a grant that names object `(kind, id)` gives permission `p`, without the kind's wildcard.
    pub(crate) fn named_level(&self, kind: i32, id: u32, p: usize) -> u8 {
        self.explicit
            .binary_search_by_key(&(kind, id), |&(k, id, _)| (k, id))
            .map_or(0, |i| self.explicit[i].2[p])
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
        let mut ids: Vec<(i32, u32)> = self
            .explicit
            .iter()
            .chain(&ex.explicit)
            .chain(&self.cut)
            .chain(&ex.cut)
            .map(|&(k, id, _)| (k, id))
            .collect();
        ids.sort_unstable();
        ids.dedup();
        let mut explicit = Vec::new();
        let mut cut = Vec::new();
        for (k, id) in ids {
            let mut lv = [0u8; 5];
            let mut wild_here = kind_slot(k).map_or([0; 5], |slot| wild[slot]);
            for (p, l) in lv.iter_mut().enumerate() {
                let (have, taken) = (self.level_of(k, id, p), ex.level_of(k, id, p));
                *l = subtract_level(have, taken).unwrap_or_else(|| {
                    why = Some("an excluded set takes an indirect permission from an indirect one (not pinned)");
                    0
                });
                let Some(slot) = kind_slot(k) else { continue };
                // What the wildcard still gives this object (after an earlier cut of it).
                let w_obj = self.wild_level(k, id, p);
                let left = subtract_level(w_obj, taken).unwrap_or(0);
                if rank(left) >= rank(wild[slot][p]) {
                    continue;
                }
                if left == 0 && (w_obj == 0 || (w_obj, taken) == (1, 1)) {
                    // A Yes taken from a Yes wildcard: the wildcard no longer grants that permission on this object, the other
                    // objects keep it (Y93WildScope.al:153-160: Execute on CodeUnit LT Y93 Cu M4 and Insert on LT Y93 Row are
                    // denied, CodeUnit M6 runs and Row reads; Y93WildEffective.al: R-MD on Row).
                    wild_here[p] = 0;
                } else {
                    // Another level taken from a wildcard for one object is not pinned: the wildcard is not kept.
                    wild[slot][p] = 0;
                    why = Some("an excluded set takes one object out of a wildcard grant below Yes (not pinned)");
                }
            }
            if lv != [0; 5] {
                explicit.push((k, id, lv));
            }
            if kind_slot(k).is_some_and(|slot| wild_here != wild[slot]) {
                cut.push((k, id, wild_here));
            }
        }
        // A wildcard dropped above for a later object leaves no cut that grants more than it.
        cut.retain_mut(|(k, _, lv)| {
            let slot = kind_slot(*k).unwrap_or(0);
            for (l, w) in lv.iter_mut().zip(wild[slot]) {
                if rank(*l) > rank(w) {
                    *l = w;
                }
            }
            *lv != wild[slot]
        });
        self.wild = wild;
        self.explicit = explicit;
        self.cut = cut;
        // What an excluded named system grant takes is not known: no System grant is kept.
        if !ex.unread_system.is_empty() {
            self.wild[7] = [0; 5];
            self.explicit.retain(|e| e.0 != 10);
            self.cut.retain(|e| e.0 != 10);
            why = Some("an excluded set has a grant on a named system object (not read)");
        }
        why.map_or(Ok(()), |w| Err(w.to_string()))
    }

    /// The content without entries that grant nothing, to compare two contents.
    fn normalized(mut self) -> Content {
        self.explicit.retain(|e| e.2 != [0; 5]);
        let wild = self.wild;
        self.cut
            .retain(|e| kind_slot(e.0).is_some_and(|slot| e.2 != wild[slot]));
        self
    }

    /// Adds what `other` grants: per permission, Yes beats Indirect beats blank (ADR-062 section 3).
    pub(crate) fn union(&mut self, other: &Content) {
        // Per object cut out of a wildcard: the union's wildcard for it is the better of the two (each after its own cuts).
        let mut cut = Vec::new();
        if !self.cut.is_empty() || !other.cut.is_empty() {
            let mut ids: Vec<(i32, u32)> = self.cut.iter().chain(&other.cut).map(|&(k, id, _)| (k, id)).collect();
            ids.sort_unstable();
            ids.dedup();
            for (k, id) in ids {
                let Some(slot) = kind_slot(k) else { continue };
                let mut lv = [0u8; 5];
                for (p, l) in lv.iter_mut().enumerate() {
                    *l = merge(self.wild_level(k, id, p), other.wild_level(k, id, p));
                }
                let whole: Levels = std::array::from_fn(|p| merge(self.wild[slot][p], other.wild[slot][p]));
                if lv != whole {
                    cut.push((k, id, lv));
                }
            }
        }
        self.cut = cut;
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
        for u in &other.unread_system {
            if !self.unread_system.contains(u) {
                self.unread_system.push(u.clone());
            }
        }
    }

    /// Whether every object of every kind is granted at Yes: Read, Insert, Modify and Delete on all table data, and
    /// Execute on every table, report, codeunit, xmlport, page, query and system object. SUPER grants this.
    pub(crate) fn grants_everything(&self) -> bool {
        let [data, rest @ ..] = &self.wild;
        self.cut.is_empty() && data[..4].iter().all(|&l| l == 1) && rest.iter().all(|l| l[4] == 1)
    }
}

/// What the assigned sets of a session give.
#[derive(Debug, Clone)]
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
        // A set whose own grants are not read adds none of them, but its included sets still add theirs: the content
        // stays a lower bound of what BC grants (al-runner issue 6717).
        let mut all = set.declared.clone().unwrap_or_else(|e| {
            why.push(e);
            Content::default()
        });
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
                let before = why.len();
                let taken = match self.one_set(ex) {
                    Some(t) if self.sets[t].excludes.is_empty() => self.expanded(t, true, visiting, why),
                    Some(_) => Err(format!(
                        "permission set '{}' excludes `{ex}`, which excludes sets too (not pinned)",
                        set.role
                    )),
                    None => Err(format!(
                        "permission set '{}' excludes `{ex}`, which is not one set",
                        set.role
                    )),
                };
                // An excluded set known only as a lower bound would take away too little.
                let taken = match taken {
                    Ok(_) if why.len() > before => Err(format!(
                        "permission set '{}' excludes `{ex}`, which is not composed in full",
                        set.role
                    )),
                    t => t,
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
            let mut exact_exclusions = true;
            for &i in &found {
                if let Ok(c) = self.expanded(i, false, &mut Vec::new(), &mut ignored) {
                    whole.union(&c);
                }
                for ex in &self.sets[i].excludes {
                    let mut partial = Vec::new();
                    match self
                        .one_set(ex)
                        .map(|t| self.expanded(t, true, &mut Vec::new(), &mut partial))
                    {
                        Some(Ok(c)) if partial.is_empty() => taken.union(&c),
                        // An exclusion known only in part would take away too little from the union.
                        _ => exact_exclusions = false,
                    }
                }
            }
            let _ = whole.subtract(&taken);
            let whole = whole.normalized();
            if !exact_exclusions {
                why.push("an assigned set excludes a set that is not composed in full (not read)".into());
            } else if whole != content.clone().normalized() {
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
    pub(crate) fn object_grants(&self, engine: &Engine, object: usize) -> Arc<Result<Content, String>> {
        let lock = || self.grants.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(g) = lock().get(&object) {
            return g.clone();
        }
        let g = Arc::new(crate::platform::object_grants(engine, object));
        lock().insert(object, g.clone());
        g
    }

    /// The `[InherentPermissions]` attributes of routine `routine` of object `object`.
    pub(crate) fn inherent_of(&self, engine: &Engine, object: usize, routine: usize) -> InherentGrants {
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
    /// Writes is on: its probes P1-P13 are merged in the corpus (master, PR 914). Reads is on for the record methods and
    /// FlowFields its probes pin (P14, P15: Y24PermReads.al); the other read shapes refuse. An access the sets allow runs in every
    /// stage (an allowed access needs no probe, ADR-062 section 2); a denial of a stage that is off refuses the test.
    pub(crate) const fn on_by_default(self) -> bool {
        match self {
            Stage::Writes | Stage::Reads | Stage::Execute => true,
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

/// The "Object Type" ordinal of a System object in the Permission tables.
const SYSTEM: i32 = 10;

/// AllObj, Field and Page Control Field: a narrow set reads them (Y19PermSystemTables.al, P13; Page Control Field: Y84PermAbp.al
/// Y84_Abp_PageControlFieldRows, which lists the page's 19 field controls under a set with no table data grant).
const ALWAYS_READABLE: [u32; 3] = [2_000_000_038, 2_000_000_041, 2_000_000_192];
/// Company and Tenant Media: the system tables whose ReadPermission and WritePermission under a set that does not name them are
/// pinned (`permission_query`).
const ANSWER_FALSE_SYSTEM_TABLES: [u32; 2] = [crate::companies::COMPANY_TABLE, TENANT_MEDIA];
/// Tenant Media: the one system table whose write denial is pinned (Y19PermSystemTables.al, P13: Insert error).
pub(crate) const TENANT_MEDIA: u32 = 2_000_000_184;

/// What the decision function says about one access of a restricted session.
pub(crate) enum Verdict {
    Allow,
    /// Denied by complete inputs: BC raises its permission error. `by` is the object whose code makes the access.
    Deny {
        indirect: bool,
        by: usize,
    },
    /// The outcome is not known: the test is not run.
    Refuse(String),
}

impl Session {
    /// `PermissionTestHelper.AddEffectivePermissionSet(RoleId)`: adds the set to the session's assigned sets.
    pub(crate) fn assign_permission_set(&mut self, role: &str) {
        // A set the test assigns ends the shadow run's evaluation of the runner's sets (ADR-062 probe P25 stays refused).
        self.shadow_active = false;
        let role: String = role.to_uppercase().chars().take(20).collect();
        if !self.permission_roles.contains(&role) {
            self.permission_roles.push(role);
        }
        self.permissions = SetModel::of(&self.engine).compose(&self.permission_roles);
        al_builtins::value::set_born_restricted(self.permissions.is_some());
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
        self.shadow_clear();
        al_builtins::value::set_born_restricted(false);
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
    ///
    /// The license caps what the sets allow (ADR-062 amendment 1 point 5): an access the sets allow goes on to
    /// [`Session::license_verdict`] when the license limits some table (license_ceiling.rs).
    fn permission_verdict(&mut self, access: Access, target: Option<usize>, flowfield: bool) -> Verdict {
        match self.sets_verdict(access, target, flowfield) {
            Verdict::Allow if self.engine.license_caps() => self.license_verdict(access, target, flowfield),
            v => v,
        }
    }

    /// The permission sets' part of [`Session::permission_verdict`]: Allow in an unrestricted session.
    fn sets_verdict(&mut self, access: Access, target: Option<usize>, flowfield: bool) -> Verdict {
        let Some(caller) = self.permission_caller.or_else(|| self.frames.last().copied()) else {
            return Verdict::Allow;
        };
        let engine = self.engine.clone();
        let o = &engine.program.objects.heads()[caller];
        if o.kind == al_ir::ObjKind::Codeunit && o.id == Some(PERMISSIONS_MOCK) {
            return Verdict::Allow;
        }
        // An unrestricted session reaches the check only for the license (license_ceiling.rs): the sets allow everything.
        if self.permissions.is_none() {
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
        if access.kind == SYSTEM && !eff.content.unread_system.is_empty() {
            return Verdict::Refuse(format!(
                "a permission denial ({}) where an assigned set grants on a named system object, which is not read: {}",
                access.describe(),
                eff.content.unread_system.join("; ")
            ));
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
                // Upper-case letters on a table (RiskyInherentPerms.al:53-122) and `InherentPermissions = X` on a codeunit
                // (Y90PermMore1.al:100-103: callable without Execute, the codeunit without it is denied).
                Some(Ok(l)) if (access.kind == 0 || (access.kind == 5 && access.op == Op::Execute)) && l[p] == 1 => {
                    u = merge(u, 1)
                }
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
                o.kind, o.name
            ));
        } else {
            match &*model.object_grants(&engine, caller) {
                Ok(c) => c.level_of(access.kind, access.id, p) != 0,
                Err(why) => {
                    return Verdict::Refuse(format!(
                        "the Permissions property of {} {} is not read: {why}",
                        o.kind, o.name
                    ));
                }
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

    /// `Record.ReadPermission` and `Record.WritePermission` (also through a RecordRef): the decision function for the
    /// access the name stands for. Read is the Read permission; Write is Insert, Modify and Delete together, each decided
    /// as an access (direct Yes, or Indirect completed by the calling object's `Permissions` property).
    /// Pinned: Y24PermReads.al Y24_P19_ReadWritePermission (indirect `r`, `i`, `im`, `imd`, `rimd`, and inside an object
    /// whose Permissions names the table), and the Y75 probe for R, RI, RIM, RIMD and RecordRef (Y75PermReadWrite.al).
    /// Not pinned, so refused when the answer would be false: a temporary record and a system table other than Company and
    /// Tenant Media.
    pub(crate) fn permission_answer_decided(
        &mut self,
        rec: &crate::records::Rec,
        method: al_builtins::record_op::RecordOp,
    ) -> Result<al_builtins::value::Value, Signal> {
        if !self.permission_checks() {
            return Ok(al_builtins::value::Value::Boolean(true));
        }
        let write = method == al_builtins::record_op::RecordOp::WritePermission;
        self.permission_query(rec, write)
            .map(al_builtins::value::Value::Boolean)
    }

    #[cold]
    #[inline(never)]
    fn permission_query(&mut self, rec: &crate::records::Rec, write: bool) -> Result<bool, Signal> {
        let (object, temporary) = {
            let r = rec.borrow();
            (r.table as usize, r.temporary)
        };
        let id = self
            .engine
            .program
            .objects
            .heads()
            .get(object)
            .and_then(|o| o.id)
            .unwrap_or(0);
        let ops: &[Op] = if write {
            &[Op::Insert, Op::Modify, Op::Delete]
        } else {
            &[Op::Read]
        };
        let mut refused: Option<String> = None;
        for &op in ops {
            if op == Op::Read && ALWAYS_READABLE.contains(&id) {
                continue;
            }
            match self.permission_verdict(Access { kind: 0, id, op }, Some(object), false) {
                Verdict::Allow => {}
                Verdict::Refuse(why) => refused = refused.or(Some(why)),
                Verdict::Deny { .. } if temporary => {
                    refused = refused.or(Some(format!(
                        "{} on a temporary record under sets that do not grant it is not pinned (ADR-062 probe P19)",
                        if write { "WritePermission" } else { "ReadPermission" }
                    )));
                }
                // Company and Tenant Media under a set that names neither: both answers false (metadata/Y19PermSystemTables.al:36-40,
                // :58-63, RecordRef.Open(TableId) under `LT Y19 R`).
                Verdict::Deny { indirect: false, .. } if ANSWER_FALSE_SYSTEM_TABLES.contains(&id) => return Ok(false),
                Verdict::Deny { .. } if id >= 2_000_000_000 => {
                    refused = refused.or(Some(format!(
                        "{} on a system table under sets that do not grant it is not pinned (ADR-062 probe P19)",
                        if write { "WritePermission" } else { "ReadPermission" }
                    )));
                }
                // One certain denial makes the answer false, whatever the other accesses would say.
                Verdict::Deny { .. } => return Ok(false),
            }
        }
        match refused {
            Some(why) => Err(Signal::unsupported(why)),
            None => Ok(true),
        }
    }

    /// The gate for a call of routine `routine` of `object` on `instance` (an Execute access, stage 3, probe P20).
    /// BC checks Execute when an object made under restricted permissions is used (Y24PermExecute.al, BC 27.5, 28.5, 29.0):
    /// - a call from the object's own code is free (the test codeunit's local procedures, a codeunit's own procedures);
    /// - an instance made before the permissions were lowered is not checked (a global of the test codeunit, the Permissions
    ///   Mock: `Mock.ClearAssignments()` runs under an exact set that grants it no Execute);
    /// - Execute on the calling codeunit does not cover the objects it runs.
    ///
    /// The triggers of a table, page or extension run for record operations and page opens, which are not a call of the
    /// object; the code of a subscriber is not pinned. A denial there refuses the test.
    #[cold]
    #[inline(never)]
    pub(crate) fn permission_gate_call_decided(
        &mut self,
        object: usize,
        routine: usize,
        instance: &std::rc::Rc<std::cell::RefCell<al_builtins::value::Instance>>,
    ) -> Result<(), Signal> {
        if !instance.borrow().born.0 {
            return Ok(());
        }
        if self.permission_caller.or_else(|| self.frames.last().copied()) == Some(object) {
            return Ok(());
        }
        if self.permission_caller_is_subscriber() {
            return Ok(());
        }
        if let Some(abort) = self.permission_open_page_abort(object) {
            return Err(abort);
        }
        let rt = &self.engine.program.objects[object].routines[routine];
        let kind = self.engine.program.objects.heads()[object].kind;
        // The triggers of a table run for the record write that starts them; Execute on the table is checked there first.
        if rt.is_trigger && kind == al_ir::ObjKind::Table {
            return Ok(());
        }
        let unpinned_site = rt
            .attributes
            .iter()
            .any(|a| a.name.eq_ignore_ascii_case("EventSubscriber"))
            || (rt.is_trigger
                && !matches!(
                    kind,
                    al_ir::ObjKind::Codeunit | al_ir::ObjKind::Report | al_ir::ObjKind::XmlPort | al_ir::ObjKind::Query
                ));
        if unpinned_site {
            return match self.execute_verdict(object) {
                Verdict::Refuse(why) => Err(Signal::unsupported(why)),
                Verdict::Deny { .. } => Err(Signal::unsupported(unpinned_denial(
                    self.execute_access(object),
                    "an Execute denial on a trigger or an event subscriber",
                ))),
                Verdict::Allow => Ok(()),
            };
        }
        self.permission_gate_execute(object)
    }

    /// Whether opening page `object` is denied (no Execute on it for the code that opens it).
    pub(crate) fn permission_page_open_denied_decided(&mut self, object: usize) -> Result<bool, Signal> {
        if self.permission_caller.or_else(|| self.frames.last().copied()) == Some(object) {
            return Ok(false);
        }
        match self.execute_verdict(object) {
            Verdict::Allow => Ok(false),
            Verdict::Refuse(why) => Err(Signal::unsupported(why)),
            Verdict::Deny { .. } => Ok(true),
        }
    }

    fn execute_access(&self, object: usize) -> Access {
        let o = &self.engine.program.objects.heads()[object];
        let (kind, id) = execute_target(o.kind, o.id, o.extends_id);
        Access {
            kind,
            id,
            op: Op::Execute,
        }
    }

    fn execute_verdict(&mut self, object: usize) -> Verdict {
        let access = self.execute_access(object);
        if access.kind < 0 {
            return Verdict::Allow;
        }
        self.permission_verdict(access, Some(object), false)
    }

    /// Execute on `object`, used by code of another object (Codeunit.Run, a report run, XmlPort.Export, Query.Open, a call on a
    /// variable). The error is Y24PermExecute.al's: `(<Kind> <id> <name> Execute: <app of the calling code>)`, code
    /// `DB:ExecuteObjDenied`. Pages end with FormAbort:Permission when opened (not built yet: refused).
    #[cold]
    #[inline(never)]
    pub(crate) fn permission_gate_execute_decided(&mut self, object: usize) -> Result<(), Signal> {
        if self.permission_caller.or_else(|| self.frames.last().copied()) == Some(object) {
            return Ok(());
        }
        let access = self.execute_access(object);
        match self.execute_verdict(object) {
            Verdict::Allow => Ok(()),
            Verdict::Refuse(why) => Err(Signal::unsupported(why)),
            Verdict::Deny { by, .. } => {
                let heads = self.engine.program.objects.heads();
                let o = &heads[object];
                let word = match access.kind {
                    1 => "Table",
                    3 => "Report",
                    5 => "CodeUnit",
                    6 => "XmlPort",
                    9 => "Query",
                    _ => {
                        return Err(Signal::unsupported(unpinned_denial(
                            access,
                            "an Execute denial on a page: BC ends with FormAbort:Permission when it opens (probe P20)",
                        )));
                    }
                };
                let app = self
                    .engine
                    .program
                    .apps
                    .get(heads[by].app as usize)
                    .map(|a| a.name.as_str())
                    .unwrap_or_default();
                Err(crate::transactions::al_error_code(
                    &format!(
                        "Sorry, the current permissions prevented the action. ({word} {} {} Execute: {app})",
                        access.id, o.name
                    ),
                    "DB:ExecuteObjDenied",
                ))
            }
        }
    }

    /// `User Permissions.GetEffectivePermission` (through `NavUserAccountHelper.GetEffectivePermissionForObject`): the five levels
    /// (Read, Insert, Modify, Delete, Execute; 0 blank, 1 Yes) of the session on object `(kind, id)`, `kind` the "Object Type"
    /// ordinal. Pinned (Y24PermExecute.al Y24_P22_EffectivePermission, BC 27.5, 28.5, 29.0; Y84PermEffective.al for Query, XmlPort and the Indirect level, which comes back as 2: the lower-case letters of the set) for TableData, Table, Codeunit, Page, XmlPort, Query and
    /// Report: the assigned sets give what they declare, and `Set` adds All Objects (Execute on every object kind), so a narrow
    /// set gives Execute on objects and `SetExactPermissionSet` gives none. `None` when no restricted session answers it (the
    /// caller answers for the unrestricted user). Sets that are not fully composed and the other object types are not pinned and
    /// refuse.
    pub(crate) fn effective_permission_levels_decided(&self, kind: i32, id: u32) -> Result<Option<Levels>, Signal> {
        let Some(eff) = self.permissions.clone() else {
            return Ok(None);
        };
        if self.ignore_permission_mock {
            assume(Assumption::PermissionsNotEnforced);
            return Ok(None);
        }
        if self.permissions_off {
            return Err(Signal::Unsupported(LOWERED.into()));
        }
        // System objects (kind 10): the wildcard `system * = X` gives Execute on each, a set without it none (Y93WildEffective.al:97-100,
        // 118-121, ids 6110, 5810, 9026; Y90PermSystem.al:234-237 `wild exact`). A grant that names one system object is read by name
        // only (`unread_system`), so what it gives an id is not known and refuses.
        let system_by_wildcard =
            kind == 10 && SYSTEM_OBJECTS_MEASURED.contains(&id) && eff.content.unread_system.is_empty();
        if !matches!(kind, 0 | 1 | 3 | 5 | 6 | 8 | 9) && !system_by_wildcard {
            return Err(Signal::unsupported(format!(
                "GetEffectivePermission of an object of type {kind} under assigned permission sets (only TableData, Table, Report, Codeunit, XmlPort, Page and Query are pinned, ADR-062 probe P22)"
            )));
        }
        let mut out = [0u8; 5];
        for (slot, op) in [Op::Read, Op::Insert, Op::Modify, Op::Delete, Op::Execute]
            .into_iter()
            .enumerate()
        {
            out[slot] = eff.level(Access { kind, id, op });
        }
        // A codeunit's own `InherentPermissions = X` gives its Execute whatever the sets say: `cuS` (System Application Base64
        // Convert, InherentPermissions = X) reads `----X` under every exact set (Y93WildEffective.al:35, :56, :77, :98, :140), and the
        // codeunit runs without a grant (Y90PermMore1.al:100-101, Y93WildScope.al:259-262).
        if kind == 5 && out[4] != 1 {
            let engine = self.engine.clone();
            let heads = engine.program.objects.heads();
            if let Some(t) = heads
                .iter()
                .position(|h| h.kind == al_ir::ObjKind::Codeunit && h.id == Some(id))
                && let Some(Ok(l)) = crate::platform::table_inherent(&engine, t)
                && l[4] == 1
            {
                out[4] = 1;
            }
        }
        if !eff.complete && out.contains(&0) {
            return Err(Signal::unsupported(format!(
                "GetEffectivePermission under assigned permission sets that are not fully composed, so BC may grant more: {}",
                eff.why_incomplete.join("; ")
            )));
        }
        Ok(Some(out))
    }

    /// Whether the control `guard` (the value of its AccessByPermission property) is left off a page that opened under the
    /// permission sets `opened_under`. Pinned (Y24PermExecute.al Y24_P21_AccessByPermissionPage and ...RequestPage, BC 27.5, 28.5,
    /// 29.0): `tabledata T = R` and `= I` and `codeunit C = X`; the control is off when the session's level of that permission
    /// is blank and there when it is Yes. An Indirect level, a session whose sets are not fully composed, a change of the
    /// assigned sets since the page opened and every other guard are not pinned: they refuse.
    pub(crate) fn access_by_permission_hidden_decided(
        &self,
        guard: &str,
        opened_under: &[String],
    ) -> Result<bool, Signal> {
        let Some(eff) = self.permissions.clone() else {
            return Ok(false);
        };
        if self.ignore_permission_mock {
            assume(Assumption::PermissionsNotEnforced);
            return Ok(false);
        }
        if self.permissions_off {
            return Err(Signal::Unsupported(LOWERED.into()));
        }
        if opened_under != self.permission_roles.as_slice() {
            return Err(Signal::Unsupported(
                "the permission sets changed while a page with AccessByPermission was open (not pinned, ADR-062 probe P21)".into(),
            ));
        }
        let g = crate::platform::parse_guard(&self.engine, guard).map_err(Signal::unsupported)?;
        let ops = [Op::Read, Op::Insert, Op::Modify, Op::Delete, Op::Execute];
        // The control is there when the session holds at least one letter of the mask at level Yes; a level of Indirect does
        // not count (Y84PermAbp.al Y84_Abp_LettersA/B: guard RIMD under a set with only R is there, an indirect-only session
        // leaves every guarded control off).
        let held = g.slots.iter().any(|&slot| match g.id {
            Some(id) => {
                eff.level(Access {
                    kind: g.kind,
                    id,
                    op: ops[slot],
                }) == 1
            }
            None => {
                eff.level(Access {
                    kind: g.kind,
                    id: u32::MAX,
                    op: ops[slot],
                }) == 1
                    || eff
                        .content
                        .unread_system
                        .iter()
                        .any(|e| crate::platform::system_entry_level(e, &g.name, slot) == 1)
            }
        });
        if held {
            return Ok(false);
        }
        if eff.complete {
            Ok(true)
        } else {
            Err(Signal::unsupported(format!(
                "AccessByPermission under assigned permission sets that are not fully composed, so BC may grant it: {}",
                eff.why_incomplete.join("; ")
            )))
        }
    }

    /// The gate for the calculation of a FlowField of `rec` over table `table` (object index): an indirect read the session
    /// level allows runs (pin 6). A denial with rows in the source table is `IndirectRead` (Y24PermReads.al
    /// Y24_P15_FlowFieldsNothingOnSource: CalcFields of Lookup, Exist, Sum and Count). On an empty source table the shape is
    /// not pinned, so it refuses.
    #[cold]
    #[inline(never)]
    pub(crate) fn permission_gate_flowfield_decided(
        &mut self,
        rec: &crate::records::Rec,
        table: usize,
    ) -> Result<(), Signal> {
        if self.permission_autocalc {
            return Ok(());
        }
        let id = self.engine.program.objects.heads()[table].id.unwrap_or(0);
        let access = Access {
            kind: 0,
            id,
            op: Op::Read,
        };
        match self.permission_verdict(access, Some(table), true) {
            Verdict::Allow => Ok(()),
            Verdict::Refuse(why) => Err(Signal::unsupported(why)),
            Verdict::Deny { .. } if self.permission_page_calc => Err(Signal::unsupported(unpinned_denial(
                access,
                "a page control over a FlowField the session cannot read: BC leaves the control off the page",
            ))),
            Verdict::Deny { by, .. } => {
                if id >= 2_000_000_000 {
                    return Err(Signal::unsupported(unpinned_denial(
                        access,
                        "a FlowField over a system table",
                    )));
                }
                let src_t = self.engine.tables[table].clone().expect("table");
                let cx = self.cx_for(rec, &src_t)?;
                let rows = al_data::Store::count(&*self.store_for(rec), &cx, &src_t, &[])
                    .map_err(|e| Signal::unsupported(format!("store error {e}")))?;
                if rows == 0 {
                    return Err(Signal::unsupported(unpinned_denial(
                        access,
                        "a FlowField over an empty source table; Y24PermReads.al pins it with rows",
                    )));
                }
                Err(self.denial_error(access, "IndirectRead", "Read", by))
            }
        }
    }

    /// Whether the session, restricted by the permission sets a test assigned, may not execute the page `part_page` (the page of a
    /// part). False without a restricted session.
    ///
    /// `unguarded`: the part has no AccessByPermission. It also needs Execute on the source table of the part page (Y90PermParts.al:20-62,
    /// 64-78: an exact set granting the card, the part page and even every page and codeunit leaves the part off; `table "LT Y90 Row" = X`
    /// or all wildcards puts it on; Y90_Part_ExactB `b_table_x`).
    pub(crate) fn permission_part_page_off(&self, part_page: Option<usize>, unguarded: bool) -> Result<bool, Signal> {
        let (Some(eff), Some(page)) = (self.permissions.as_ref(), part_page) else {
            return Ok(false);
        };
        if self.ignore_permission_mock {
            return Ok(false);
        }
        let mut accesses = vec![self.execute_access(page)];
        if unguarded {
            let o = &self.engine.program.objects[page];
            let table = o.rec_slot.and_then(|s| match o.globals.get(s).map(|g| &g.ty) {
                Some(al_builtins::value::RtType::Record(t, _)) => Some(*t as usize),
                _ => None,
            });
            if let Some(t) = table {
                accesses.push(Access {
                    kind: 1,
                    id: self.engine.program.objects.heads()[t].id.unwrap_or(0),
                    op: Op::Execute,
                });
            }
        }
        if accesses.iter().all(|a| a.kind < 0 || eff.level(*a) == 1) {
            return Ok(false);
        }
        if eff.complete {
            Ok(true)
        } else {
            Err(Signal::unsupported(format!(
                "a part page under assigned permission sets that are not fully composed, so BC may execute it: {}",
                eff.why_incomplete.join("; ")
            )))
        }
    }

    /// Whether a page control over a FlowField whose source table is object `table` is left off the page: the session may not
    /// calculate it (the same decision as `CalcFields`, an indirect read the session level allows runs). BC leaves it off whether
    /// the source table has rows or not (Y84PermAbp.al Y84_Abp_FlowFieldControl `empty, no read`, `rows, no read`: "The field with
    /// ID = n is not found on the page."), where `CalcFields` of the same FlowField on an empty table raises nothing.
    #[cold]
    #[inline(never)]
    pub(crate) fn permission_flowfield_off_page(&mut self, table: usize) -> Result<bool, Signal> {
        let id = self.engine.program.objects.heads()[table].id.unwrap_or(0);
        let access = Access {
            kind: 0,
            id,
            op: Op::Read,
        };
        match self.permission_verdict(access, Some(table), true) {
            Verdict::Allow => Ok(false),
            Verdict::Refuse(why) if self.shadow_active => {
                self.shadow_note("unknown", "flowfield page control", &why);
                Ok(false)
            }
            Verdict::Refuse(why) => Err(Signal::unsupported(why)),
            Verdict::Deny { .. } if id >= 2_000_000_000 => Err(Signal::unsupported(unpinned_denial(
                access,
                "a page control over a FlowField of a system table",
            ))),
            Verdict::Deny { .. } if self.shadow_active => {
                self.shadow_note("would deny", "flowfield page control", &access.describe());
                Ok(false)
            }
            Verdict::Deny { .. } => Ok(true),
        }
    }

    /// The read check of an object that reads tables without an AL frame of its own making the access: a query, an
    /// xmlport, a report. The object's `Permissions` property and app decide it, as for code (Y24PermObjectReads.al
    /// Y24_P16_*: without a property the object needs Read and the error says `Read`; with `Permissions = r` it needs the
    /// indirect grant and the error says `IndirectRead`). Like a record read, it raises only when the table has rows.
    #[cold]
    #[inline(never)]
    pub(crate) fn permission_gate_object_read_decided(
        &mut self,
        object: usize,
        tables: &[usize],
    ) -> Result<(), Signal> {
        // Execute on the query or xmlport itself, by the code that runs it (Y24PermExecute.al tdonly_query_open, tdonly_xmlport_export).
        self.permission_gate_execute(object)?;
        self.permission_caller = Some(object);
        let r = self.object_read_inner(tables);
        self.permission_caller = None;
        r
    }

    fn object_read_inner(&mut self, tables: &[usize]) -> Result<(), Signal> {
        for &table in tables {
            let id = self.engine.program.objects.heads()[table].id.unwrap_or(0);
            let access = Access {
                kind: 0,
                id,
                op: Op::Read,
            };
            if ALWAYS_READABLE.contains(&id) {
                continue;
            }
            match self.permission_verdict(access, Some(table), false) {
                Verdict::Allow => {}
                Verdict::Refuse(why) => return Err(Signal::unsupported(why)),
                Verdict::Deny { indirect, by } => {
                    let t = self.engine.tables[table].clone().expect("table");
                    let rows = al_data::Store::count(&self.store, &self.cx, &t, &[])
                        .map_err(|e| Signal::unsupported(format!("store error {e}")))?;
                    if id >= 2_000_000_000 {
                        return Err(Signal::unsupported(unpinned_denial(
                            access,
                            "an object reading a system table",
                        )));
                    }
                    if rows > 0 {
                        let word = if indirect { "IndirectRead" } else { "Read" };
                        return Err(self.denial_error(access, word, "Read", by));
                    }
                }
            }
        }
        Ok(())
    }

    /// The gate for record method `method` on `rec`; a method that reads or writes no row (filters, keys, field
    /// metadata) is no access. `from_al`: AL code calls the method (a Record or RecordRef method), not the runtime on
    /// behalf of a page, report, xmlport or validation.
    #[cold]
    #[inline(never)]
    pub(crate) fn permission_gate_record_decided(
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
        let id = self
            .engine
            .program
            .objects
            .heads()
            .get(object)
            .and_then(|o| o.id)
            .unwrap_or(0);
        if op == Op::Read && ALWAYS_READABLE.contains(&id) {
            return Ok(());
        }
        let access = Access { kind: 0, id, op };
        if from_al {
            self.permission_table_trigger_execute(method, rec, args, object)?;
        }
        // The reads the runtime makes for a running xmlport count as the xmlport's.
        self.permission_caller = if from_al { None } else { self.permission_run_object };
        let verdict = self.permission_verdict(access, Some(object), false);
        self.permission_caller = None;
        match verdict {
            Verdict::Allow => Ok(()),
            Verdict::Refuse(why) => Err(Signal::unsupported(why)),
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
        let refuse = |why: &str| Err(Signal::unsupported(unpinned_denial(access, why)));
        if !Stage::of(access.op).on_by_default() {
            return refuse("ADR-062 stage 3, probes P20-P22");
        }
        if !from_al && access.op == Op::Read && self.page_open_read_allowed(rec)? {
            return Ok(());
        }
        if !from_al {
            return refuse("made by the runtime for a page, report, xmlport or validation; ADR-062 probes P16-P18");
        }
        if access.id >= 2_000_000_000 && access.id != TENANT_MEDIA {
            return refuse("on a system table; probe P13 pins Tenant Media only");
        }
        if access.op == Op::Read {
            if access.id >= 2_000_000_000 {
                return refuse("on a system table; probe P13 pins Tenant Media writes only");
            }
            return self.read_denied(method, rec, args, access, indirect, by);
        }
        let word = match access.op {
            Op::Insert => "Insert",
            Op::Modify => "Modify",
            Op::Delete => "Delete",
            _ => return refuse("ADR-062 section 2"),
        };
        match method {
            R::Insert if self.row_exists(rec)? => {
                return refuse("an Insert of a key that exists (which error comes first is not pinned)");
            }
            R::Modify | R::Delete | R::Rename if !self.row_exists(rec)? => {
                return refuse("on a row that does not exist (which error comes first is not pinned)");
            }
            R::Insert | R::Modify | R::Delete | R::Rename => {}
            R::ModifyAll | R::DeleteAll => {
                let t = self.table(rec)?;
                let cx = self.cx_for(rec, &t)?;
                let rows = al_data::Store::count(&*self.store_for(rec), &cx, &t, &[])
                    .map_err(|e| Signal::unsupported(format!("store error {e}")))?;
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
        let op = if indirect {
            format!("Indirect{word}")
        } else {
            word.to_string()
        };
        Err(crate::transactions::al_error_code(
            &format!(
                "Sorry, the current permissions prevented the action. (TableData {} {name} {op}: {app})",
                access.id
            ),
            &format!("DB:Client{word}Denied"),
        ))
    }

    /// BC's permission error: `text_word` in the message (`Read`, `IndirectRead`, `Insert`, ...), `code_word` in the code
    /// (`DB:Client<code_word>Denied`); `by` is the object whose code makes the access (its app names the error).
    fn denial_error(&self, access: Access, text_word: &str, code_word: &str, by: usize) -> Signal {
        // A test that reaches a report the runtime cannot run is refused at its first denied read: its report would
        // refuse it later, after the code between had changed what a later test of the codeunit observes
        // (reports/runtime/UPropertyTests.al Report_RequestPagePermissions_AndInherentPermissions and the test
        // after it, Query_InherentPermissions_UnderTheMock). Report permissions are not pinned (P17).
        if let Some(why) = &self.report_refusal_after_denial {
            return Signal::unsupported(unpinned_denial(
                access,
                &format!("the test goes on to a report that cannot run ({why})"),
            ));
        }
        let heads = self.engine.program.objects.heads();
        let name = heads
            .iter()
            .find(|o| o.kind == al_ir::ObjKind::Table && o.id == Some(access.id))
            .map(|o| o.name.as_str())
            .unwrap_or_default();
        let app = self
            .engine
            .program
            .apps
            .get(heads[by].app as usize)
            .map(|a| a.name.as_str())
            .unwrap_or_default();
        crate::transactions::al_error_code(
            &format!(
                "Sorry, the current permissions prevented the action. (TableData {} {name} {text_word}: {app})",
                access.id
            ),
            &format!("DB:Client{code_word}Denied"),
        )
    }

    /// A denied Read of a record method AL code calls (the checks of `write_denied` have passed). Pinned
    /// (Y24PermReads.al Y24_P14_ReadShapesWithRows and Y24_P14_ReadShapesEmptyTable, BC 27.5, 28.5, 29.0):
    /// - on a table with rows, Get of an existing key, Find, FindLast, FindSet, IsEmpty, CountApprox, FindFirst after
    ///   SetLoadFields, and RecordRef.FindFirst raise `Read`; CalcSums raises `IndirectRead` (code `DB:ClientReadDenied`
    ///   for both); Get of a missing key raises RecordNotFound, which the method does itself;
    /// - on an empty table nothing raises a permission error and the method answers as usual.
    ///
    /// A method that finds no row on a table that has rows, a Next, and an indirect CalcSums are not pinned: they refuse.
    /// Whether a denied read the runtime makes of `rec` raises nothing: a page that opens reads its own source table, a table of
    /// the app under test with no rows. Y94AbpActionRef.al Y94_Ref_GrantsB ('set d365 basic', 'exact d365 basic', 'exact d365 full
    /// access': none of the sets grants Read on "LT Y94 Row", which holds no row; OpenView opens the card and its controls answer),
    /// the page form of the empty-table rule of AL reads (Y24PermReads.al Y24_P14_ReadShapesEmptyTable). With rows (probe P18), on
    /// a table of a dependency app (Cronus may hold rows there) and for every other read the runtime makes, it stays refused.
    fn page_open_read_allowed(&mut self, rec: &crate::records::Rec) -> Result<bool, Signal> {
        let table = rec.borrow().table as usize;
        if self.permission_opening_source != Some(table) {
            return Ok(false);
        }
        let app = self.engine.program.objects.heads()[table].app as usize;
        if self.engine.program.apps.get(app).is_none_or(|a| a.dependency) {
            return Ok(false);
        }
        let t = self.table(rec)?;
        let cx = self.cx_for(rec, &t)?;
        let total = al_data::Store::count(&*self.store_for(rec), &cx, &t, &[])
            .map_err(|e| Signal::unsupported(format!("store error {e}")))?;
        Ok(total == 0)
    }

    fn read_denied(
        &mut self,
        method: al_builtins::record_op::RecordOp,
        rec: &crate::records::Rec,
        args: &[al_builtins::value::Value],
        access: Access,
        indirect: bool,
        by: usize,
    ) -> Result<(), Signal> {
        use al_builtins::record_op::RecordOp as R;
        let refuse = |why: &str| Err(Signal::unsupported(unpinned_denial(access, why)));
        let t = self.table(rec)?;
        let cx = self.cx_for(rec, &t)?;
        let total = al_data::Store::count(&*self.store_for(rec), &cx, &t, &[])
            .map_err(|e| Signal::unsupported(format!("store error {e}")))?;
        if total == 0 {
            // Pinned for a table of the app under test (Y24PermReads.al Y24_P14_ReadShapesEmptyTable). A table of a dependency
            // app that is empty here may hold rows in BC's Cronus database (Y69SystemGrants.al Y69_SY_ReporterShapes reads
            // Customer under a set without Read and BC denies): the answer is not known.
            let heads = self.engine.program.objects.heads();
            let app = heads[t.object as usize].app as usize;
            // With test data loaded (the restored company, ADR-020) the store holds the database's full content for every table
            // the backup offers, so an empty table is empty in BC's database too: AEDependencyTableReads.al
            // Read_StdCost_SetOwnOnly_Part1 and _Part2 (Standard Cost Worksheet, empty in Cronus): typed reads raise nothing.
            if self.engine.program.apps.get(app).is_none_or(|a| a.dependency) && self.engine.test_data.is_none() {
                return refuse(
                    "a read of an empty table of a dependency app, which BC's database may fill with demo rows (no test data is loaded)",
                );
            }
            return Ok(());
        }
        if method == R::Get {
            if args.iter().any(|v| matches!(v, al_builtins::value::Value::Library(..))) || args.len() > t.keys[0].len()
            {
                return refuse("Get with a RecordId or more values than the primary key has fields");
            }
            let key = self.get_key(rec, &t, args, false)?;
            if key.given < key.pk.len() {
                return refuse("Get with fewer values than the primary key has fields");
            }
            let pk = key.pk;
            if crate::sqltime::key_rounds(&pk) {
                return refuse("Get with a Time or DateTime key value the SQL column rounds");
            }
            let cx = self.cx_for(rec, &t)?;
            let found = al_data::Store::get(&*self.store_for(rec), &cx, &t, &pk)
                .map_err(|e| Signal::unsupported(format!("store error {e}")))?;
            return if found.is_some() {
                Err(self.denial_error(access, "Read", "Read", by))
            } else {
                Ok(())
            };
        }
        if method == R::GetBySystemId {
            // An existing id is denied, a missing id raises nothing (AEDependencyTableReads.al `Customer_getbysystemid_existing`,
            // `Customer_getbysystemid_missing`).
            let Some(al_builtins::value::Value::Guid(g)) = args.first() else {
                return refuse("GetBySystemId with an argument that is not a Guid");
            };
            let Some(i) = t.fields.iter().position(|f| f.id == 2_000_000_000) else {
                return refuse("GetBySystemId on a table without SystemId");
            };
            let filters = [al_builtins::filter::FieldFilter {
                lang: al_builtins::filter::filter_language(),
                field: i,
                expr: al_builtins::filter::FilterExpr::Eq(al_builtins::value::Value::Guid(*g)),
                group: 0,
            }];
            let cx = self.cx_for(rec, &t)?;
            let found = al_data::Store::count(&*self.store_for(rec), &cx, &t, &filters)
                .map_err(|e| Signal::unsupported(format!("store error {e}")))?;
            return if found > 0 && *g != [0; 16] {
                Err(self.denial_error(access, "Read", "Read", by))
            } else {
                Ok(())
            };
        }
        let exact = match args.first() {
            None => true,
            Some(al_builtins::value::Value::Text(s)) => s.as_str() == "=",
            _ => false,
        };
        if method == R::Find && exact {
            return if self.row_exists(rec)? {
                Err(self.denial_error(access, "Read", "Read", by))
            } else if self.key_is_blank(rec, &t)? {
                // A blank key finds no row and raises nothing (AEDependencyTableReads.al `Customer_find_eq`).
                Ok(())
            } else {
                refuse("Find('=') of a non-blank key that does not exist on a table with rows")
            };
        }
        let (filters, flow_field_filters) = self.split_filters(rec);
        if !flow_field_filters.is_empty() || rec.borrow().marked_only {
            return refuse("a read with a FlowField filter or marked records only");
        }
        let n = al_data::Store::count(&*self.store_for(rec), &cx, &t, &filters)
            .map_err(|e| Signal::unsupported(format!("store error {e}")))?;
        if n == 0 {
            // FindFirst with a filter that matches no row is denied (AEDependencyTableReads.al `Customer_filter_no_match`); the
            // other methods are not pinned for it.
            if method == R::FindFirst {
                return Err(self.denial_error(access, "Read", "Read", by));
            }
            return refuse("a read other than FindFirst that finds no row on a table that has rows");
        }
        match method {
            // Next on a fresh record: no filter and a blank key (`Customer_next`).
            R::Next if !filters.is_empty() || !self.key_is_blank(rec, &t)? => {
                refuse("Next on a record with a filter or a position")
            }
            R::Next => Err(self.denial_error(access, "Read", "Read", by)),
            R::Find | R::FindFirst | R::FindLast | R::FindSet | R::Count | R::CountApprox | R::IsEmpty => {
                Err(self.denial_error(access, "Read", "Read", by))
            }
            R::CalcSums if !indirect && self.session_read_level(access) == 0 => {
                Err(self.denial_error(access, "IndirectRead", "Read", by))
            }
            _ => refuse("a read shape other than the ones Y24PermReads.al pins"),
        }
    }

    /// `calc_fields` for the controls of a page: a denial refuses (BC leaves such a control off the page, Y24_P15).
    pub(crate) fn page_calc_fields(&mut self, rec: &crate::records::Rec, positions: &[usize]) -> Result<(), Signal> {
        let saved = std::mem::replace(&mut self.permission_page_calc, true);
        let r = self.calc_fields(rec, positions);
        self.permission_page_calc = saved;
        r
    }

    /// `calc_value` for a control of a page, as [`Self::page_calc_fields`].
    pub(crate) fn page_calc_value(
        &mut self,
        rec: &crate::records::Rec,
        pos: usize,
    ) -> Result<al_builtins::value::Value, Signal> {
        let saved = std::mem::replace(&mut self.permission_page_calc, true);
        let r = self.calc_value(rec, pos);
        self.permission_page_calc = saved;
        r
    }

    /// `CountApprox` answers as `Count`. The gate ran for CountApprox, so `Count` passes it again with the same answer.
    pub(crate) fn count_approx(&mut self, rec: &crate::records::Rec) -> Result<al_builtins::value::Value, Signal> {
        self.permission_from_al = self.permission_checks();
        self.record_op(al_builtins::record_op::RecordOp::Count, rec, &mut [])
    }

    /// The level the assigned sets give `access` before inherent permissions and object grants.
    fn session_read_level(&self, access: Access) -> u8 {
        // An unrestricted session (checked only for the license) holds Yes from its sets.
        self.permissions.as_ref().map_or(1, |e| e.level(access))
    }

    /// Whether every field of `rec`'s primary key holds its blank value.
    fn key_is_blank(&mut self, rec: &crate::records::Rec, t: &al_data::TableDef) -> Result<bool, Signal> {
        let r = rec.borrow();
        for &i in &t.keys[0] {
            if !crate::recmeta::is_blank(&r.fields[i])? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Whether the row with `rec`'s primary key is stored.
    pub(crate) fn row_exists(&mut self, rec: &crate::records::Rec) -> Result<bool, Signal> {
        let t = self.table(rec)?;
        let cx = self.cx_for(rec, &t)?;
        let pk: Vec<al_builtins::value::Value> = {
            let r = rec.borrow();
            t.keys[0].iter().map(|&i| r.fields[i].clone()).collect()
        };
        let found = al_data::Store::get(&*self.store_for(rec), &cx, &t, &pk)
            .map_err(|e| Signal::unsupported(format!("store error {e}")))?;
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
    pub(crate) fn describe(self) -> String {
        format!(
            "{} permission on object type {} id {}",
            self.op.describe(),
            self.kind,
            self.id
        )
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
            unread_system: vec![],
            cut: vec![],
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
            unread_system: vec![],
            cut: vec![],
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

    /// Writes is on once probes P1-P13 landed, Reads once P14-P15 did, Execute once P20 did (ADR-062 section 5).
    #[test]
    fn every_access_stage_is_on_by_default() {
        for op in [Op::Insert, Op::Modify, Op::Delete, Op::Read, Op::Execute] {
            assert!(Stage::of(op).on_by_default(), "{op:?}");
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
            unread_system: vec![],
            cut: vec![],
        };
        let b = Content {
            wild: Wildcards::default(),
            explicit: vec![(0, 3, [0, 1, 0, 0, 0]), (0, 5, [1, 2, 0, 0, 0])],
            unread_system: vec![],
            cut: vec![],
        };
        a.union(&b);
        assert_eq!(a.explicit, vec![(0, 3, [0, 1, 0, 0, 0]), (0, 5, [1, 2, 0, 0, 0])]);
    }

    /// `LT Y93 W Excl` (Y93WildObjects.al:533-538): every wildcard at Yes, minus `codeunit "LT Y93 Cu M4" = X` and
    /// `tabledata "LT Y93 Row" = I`. Y93WildScope.al:153-160: M4 denied, M6 runs, Row insert denied, Row read allowed.
    #[test]
    fn an_excluded_yes_takes_one_object_out_of_a_yes_wildcard() {
        let mut all = Content {
            wild: all_kinds([1, 1, 1, 1, 1]),
            ..Default::default()
        };
        let ex = Content {
            explicit: vec![(0, 94980, [0, 1, 0, 0, 0]), (5, 94983, [0, 0, 0, 0, 1])],
            ..Default::default()
        };
        assert_eq!(all.subtract(&ex), Ok(()));
        assert_eq!(all.level_of(5, 94983, 4), 0, "M4 Execute excluded");
        assert_eq!(all.level_of(5, 94985, 4), 1, "M6 keeps the wildcard");
        let row: Vec<u8> = (0..4).map(|p| all.level_of(0, 94980, p)).collect();
        assert_eq!(row, vec![1, 0, 1, 1], "R-MD (Y93WildEffective.al)");
        assert_eq!(all.level_of(0, 94981, 1), 1, "another table keeps Insert");
        assert!(!all.grants_everything());
        // A union with a set that names the object explicitly at Yes gives it back; one without leaves the cut.
        let mut again = all.clone();
        again.union(&Content {
            explicit: vec![(5, 94983, [0, 0, 0, 0, 1])],
            ..Default::default()
        });
        assert_eq!(again.level_of(5, 94983, 4), 1);
        let mut other = all.clone();
        other.union(&Content {
            wild: all_kinds([0, 0, 0, 0, 0]),
            ..Default::default()
        });
        assert_eq!(other.level_of(5, 94983, 4), 0);
        assert_eq!(other.level_of(0, 94980, 1), 0);
    }

    /// An excluded Indirect from a Yes wildcard is not measured: the wildcard is not kept and the reason is named.
    #[test]
    fn an_excluded_indirect_from_a_yes_wildcard_stays_unpinned() {
        let mut all = Content {
            wild: all_kinds([1, 1, 1, 1, 1]),
            ..Default::default()
        };
        let ex = Content {
            explicit: vec![(0, 94980, [0, 2, 0, 0, 0])],
            ..Default::default()
        };
        assert!(all.subtract(&ex).is_err());
        assert_eq!(all.level_of(0, 94981, 1), 0, "a lower bound");
    }

    /// Y93WildScope.al:127-138 (`LT Y93 W Mix Data`, `LT Y93 W Mix Cu`): an explicit lower grant does not narrow a wildcard.
    #[test]
    fn an_explicit_lower_grant_keeps_the_wildcard_level() {
        let mut c = Content::default();
        c.union(&Content {
            wild: all_kinds([1, 1, 1, 1, 1]),
            explicit: vec![(0, 94981, [1, 0, 0, 0, 0]), (5, 94982, [0, 0, 0, 0, 2])],
            ..Default::default()
        });
        assert_eq!(c.level_of(0, 94981, 1), 1);
        assert_eq!(c.level_of(5, 94982, 4), 1);
    }
}
