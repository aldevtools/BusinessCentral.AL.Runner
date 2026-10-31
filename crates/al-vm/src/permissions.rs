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

impl Content {
    /// Adds what `other` grants: per permission, Yes beats Indirect beats blank (ADR-062 section 3).
    fn union(&mut self, other: &Content) {
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
/// (the runner assigns the same sets before thousands of tests).
pub(crate) struct SetModel {
    sets: Vec<crate::platform::RawSet>,
    by_name: HashMap<String, Vec<usize>>,
    by_role: HashMap<String, Vec<usize>>,
    composed: Mutex<HashMap<Vec<String>, Option<Arc<EffectivePermissions>>>>,
}

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
            }
        })
    }

    /// Set `i` with its included sets merged in, and whether it is complete (the reasons when not).
    fn expanded(&self, i: usize, visiting: &mut Vec<usize>, why: &mut Vec<String>) -> Result<Content, String> {
        let set = &self.sets[i];
        if visiting.contains(&i) {
            return Err(format!("permission set '{}' includes itself", set.role));
        }
        let mut all = set.declared.clone()?;
        if set.excluded {
            why.push(format!(
                "permission set '{}' excludes sets (ADR-062 probe P11)",
                set.role
            ));
        }
        if set.extended {
            why.push(format!(
                "permission set '{}' is extended by a permission set extension (ADR-062 probe P10)",
                set.role
            ));
        }
        visiting.push(i);
        for inc in &set.includes {
            match self.by_name.get(inc).map(Vec::as_slice) {
                Some([t]) => match self.expanded(*t, visiting, why) {
                    Ok(c) => all.union(&c),
                    Err(e) => why.push(e),
                },
                _ => why.push(format!(
                    "permission set '{}' includes `{inc}`, which is not one set",
                    set.role
                )),
            }
        }
        visiting.pop();
        Ok(all)
    }

    /// The session level of a session whose assigned sets are `roles` (Role IDs, in assignment order): `None` when the
    /// union grants everything (unrestricted), else what it gives.
    ///
    /// The union only adds (pin 2 of ADR-062), so a set that cannot be composed, or whose exclusions and extensions are
    /// not applied, cannot take away what the others grant: when the sets that compose grant everything, the session
    /// is unrestricted whatever the others hold. An excluding set is left out of the union (subtraction could remove
    /// what it grants), and the result is incomplete.
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
        for role in &key {
            match self.by_role.get(role).map(Vec::as_slice) {
                Some([i]) => {
                    let mut own_why = Vec::new();
                    match self.expanded(*i, &mut Vec::new(), &mut own_why) {
                        Ok(c) if !self.sets[*i].excluded => content.union(&c),
                        Ok(_) => {}
                        Err(e) => own_why.push(e),
                    }
                    why.extend(own_why);
                }
                Some(_) => why.push(format!(
                    "more than one permission set has the Role ID '{role}' (ADR-062 probe P9)"
                )),
                None => why.push(format!("no permission set of the program has the Role ID '{role}'")),
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

    /// Whether the stage is on unless `--permissions off` says otherwise. None is yet.
    pub(crate) const fn on_by_default(self) -> bool {
        match self {
            Stage::Writes | Stage::Reads | Stage::Execute => false,
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
        let Some(slot) = kind_slot(access.kind) else {
            return 0;
        };
        let p = access.op as usize;
        let wild = self.content.wild[slot][p];
        let own = self
            .content
            .explicit
            .binary_search_by_key(&(access.kind, access.id), |&(k, id, _)| (k, id))
            .map_or(0, |i| self.content.explicit[i].2[p]);
        merge(wild, own)
    }
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
        }
    }

    /// The constructor, `Clear` and `Dispose` of `PermissionTestHelper`, and the start of each test method: no set is
    /// assigned, the user's own permissions apply (unrestricted).
    pub(crate) fn clear_permission_sets(&mut self) {
        self.permission_roles.clear();
        self.permissions = None;
    }

    /// An access of a restricted session (the caller checked `self.permissions.is_some()`). The code of "Permissions
    /// Mock" itself is not checked: between its `Assign` calls the list is not final (`Set('SUPER')` assigns All Objects
    /// first), and Microsoft's mock runs on BC under every list it builds (TestMockBasics.al:20-79, 143-163). Nor is a
    /// call the runner makes from outside AL code (its steps around the test method, al-testrun `runner_reset.rs`).
    ///
    /// The gate per access kind (ADR-062 section 2): an access whose stage is not enforced refuses the test, as before
    /// the gate existed. An enforced access runs the decision; a denial is raised only in a pinned shape, so until a
    /// stage brings its pinned errors it refuses too (the safety rule of section 2).
    #[cold]
    #[inline(never)]
    pub(crate) fn permission_gate(&mut self, access: Access) -> Result<(), Signal> {
        let Some(&caller) = self.frames.last() else {
            return Ok(());
        };
        let o = &self.engine.program.objects.heads()[caller];
        if o.kind == al_ir::ObjKind::Codeunit && o.id == Some(PERMISSIONS_MOCK) {
            return Ok(());
        }
        if self.ignore_permission_mock {
            assume(Assumption::PermissionsNotEnforced);
            return Ok(());
        }
        if !self.enforces(access.op) {
            return Err(Signal::Unsupported(LOWERED.into()));
        }
        let Some(eff) = self.permissions.clone() else {
            return Ok(());
        };
        // The object grant `G` (the caller's own `Permissions` property) comes with stage 1.
        match decide(eff.level(access), false) {
            Decision::Allow => Ok(()),
            Decision::Deny { .. } if !eff.complete => Err(Signal::Unsupported(format!(
                "{LOWERED}; the assigned sets are not fully composed: {}",
                eff.why_incomplete.join("; ")
            ))),
            Decision::Deny { .. } => Err(Signal::Unsupported(format!(
                "a permission denial ({} permission on object type {} id {}) whose error is not pinned yet (ADR-062 section 2)",
                access.op.describe(),
                access.kind,
                access.id
            ))),
        }
    }

    /// Whether the gate decides accesses of this kind (`--permissions off` turns every stage off).
    fn enforces(&self, op: Op) -> bool {
        !self.permissions_off && Stage::of(op).on_by_default()
    }

    /// The gate for a call of a routine of `object` (an Execute access, stage 3).
    #[cold]
    #[inline(never)]
    pub(crate) fn permission_gate_call(&mut self, object: usize) -> Result<(), Signal> {
        let o = &self.engine.program.objects.heads()[object];
        let (kind, id) = execute_target(o.kind, o.id, o.extends_id);
        self.permission_gate(Access {
            kind,
            id,
            op: Op::Execute,
        })
    }

    /// The gate for record method `method` on a record of the table with object index `object`; a method that reads
    /// or writes no row (filters, keys, field metadata) is no access.
    #[cold]
    #[inline(never)]
    pub(crate) fn permission_gate_record(
        &mut self,
        method: al_builtins::record_op::RecordOp,
        object: usize,
    ) -> Result<(), Signal> {
        let Some(op) = record_access(method) else {
            return Ok(());
        };
        let id = self
            .engine
            .program
            .objects
            .heads()
            .get(object)
            .and_then(|o| o.id)
            .unwrap_or(0);
        self.permission_gate(Access { kind: 0, id, op })
    }

    /// At the end of a test method: the refusal when the session is still restricted (the test lowered its permissions
    /// and made no access the gate saw since). Under `--ignore-permission-mock` the test is flagged instead.
    pub fn permissions_lowered_at_end(&mut self) -> Option<String> {
        self.permissions.as_ref()?;
        if self.ignore_permission_mock {
            assume(Assumption::PermissionsNotEnforced);
            return None;
        }
        Some(LOWERED.into())
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

    /// No stage is on before its probes land (ADR-062 section 5): the skeleton changes nothing.
    #[test]
    fn no_stage_is_on_by_default_yet() {
        for op in [Op::Read, Op::Insert, Op::Modify, Op::Delete, Op::Execute] {
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
