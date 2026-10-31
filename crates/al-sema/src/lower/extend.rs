//! A compile that grows by apps (ADR-063): a later pass of the same run adds apps to a compile it keeps, and only the
//! new apps, the objects they change and what the demand newly reaches are checked and lowered.
//!
//! The kept compile's objects come first in the grown workspace (`Workspace::append_apps_of`), so an object index means
//! the same in both. An object of the kept compile is taken from its program, as a compile cache supplies an object
//! (`provided.rs`), when the grown compile would lower it the same way: the demand reached the same routines of it, it is
//! left out (or not) as before, and no new app extends or implements what its lowered form embeds (`relower_set`). The
//! rest is lowered from source. The check continues the kept one (`check::check_grown`). The result is what a compile of
//! all the apps from the start gives, except that it may compile more of the kept apps than that one needs (what the
//! kept compile's roots reached): code the tests of the grown compile do not reach, which runs the same either way, as
//! in the eager compile (ADR-013).

use super::*;
use crate::provided::{DepBlock, DepFacts, DepSource};
use crate::workspace::{AppKind, AppSource};

/// What a compile keeps for a later pass that grows it by apps ([`grow`]).
pub struct Growable {
    prev: crate::check::PrevCheck,
    skipped: Vec<bool>,
    left_out: Vec<bool>,
    /// Per object: its lowered form embeds what apps that left the compile added (`truncate`), so it is lowered again.
    stale: Vec<bool>,
    /// The members the kept compile computed, by object (`MemberTable::with_known`).
    members: Vec<std::sync::OnceLock<crate::members::Members>>,
    source: std::sync::Arc<KeptProgram>,
}

impl Growable {
    /// How many objects the kept compile has.
    pub fn objects(&self) -> usize {
        self.prev.objects
    }

    /// Cuts the kept compile and `ws`, the workspace it was kept with, back to the first `apps` apps
    /// ([`Workspace::truncate_apps`]), and gives the others back: the main pass's own test apps, which a later pass
    /// does not have. What the demand reached through them stays reached (a grown compile may compile more than it
    /// needs, see the module comment); an object whose lowered form embeds what they add (an extension, an
    /// implementer: `provided::relower_set`) is lowered again when the compile grows.
    pub fn truncate(&mut self, ws: &mut Workspace, apps: usize) -> Vec<AppSource> {
        let n_obj = ws.objects.iter().position(|o| o.app >= apps).unwrap_or(ws.objects.len());
        let n_file = ws.files.iter().position(|f| f.app >= apps).unwrap_or(ws.files.len());
        let src = std::sync::Arc::get_mut(&mut self.source).expect("the kept program is not shared yet");
        src.facts.truncate(n_obj);
        src.heads.truncate(n_obj);
        src.pos_ranges.truncate(n_obj);
        let stale = crate::provided::relower_set(ws, n_obj, &*self.source);
        self.stale.truncate(n_obj);
        for (s, t) in self.stale.iter_mut().zip(stale) {
            *s |= t;
        }
        let prev = &mut self.prev;
        prev.objects = n_obj;
        prev.demand.level.truncate(n_obj);
        prev.demand.reached.retain(|&(o, _)| o < n_obj);
        prev.kept.truncate(n_obj);
        prev.types.truncate_files(n_file);
        prev.resolutions.truncate_files(n_file);
        prev.from_checker.retain(|d| d.app < apps);
        prev.panicked.retain(|&(o, _), _| o < n_obj);
        prev.codes.truncate(apps);
        self.skipped.truncate(n_obj);
        self.left_out.truncate(n_obj);
        self.members.truncate(n_obj);
        forget_extended(&mut self.members, ws, n_obj);
        ws.truncate_apps(apps)
    }
}

/// The objects of a kept compile, as a source of supplied objects (`provided.rs`): their facts as the lowering exported
/// them, and the objects themselves from the kept program.
struct KeptProgram {
    facts: Vec<DepFacts>,
    heads: Vec<ir::ObjectHead>,
    program: std::sync::Arc<ir::Program>,
    pos_ranges: Vec<(u32, u32)>,
}

impl DepSource for KeptProgram {
    fn len(&self) -> usize {
        self.facts.len()
    }
    fn facts(&self, object: usize) -> &DepFacts {
        &self.facts[object]
    }
    fn head(&self, object: usize) -> &ir::ObjectHead {
        &self.heads[object]
    }
    fn block(&self, object: usize) -> Result<DepBlock, String> {
        let o = self
            .program
            .objects
            .get(object)
            .ok_or_else(|| format!("the kept program has no object {object}"))?;
        let (start, len) = self.pos_ranges[object];
        Ok(DepBlock {
            object: Some(o.clone()),
            pos_start: start,
            positions: (start..start + len).map(|id| self.program.positions[id as usize]).collect(),
        })
    }
}

/// Clears in `known` (the members of the first objects of `ws`) those of an object that an object from `from` on extends:
/// its members hold its extensions'.
fn forget_extended(known: &mut [std::sync::OnceLock<crate::members::Members>], ws: &Workspace, from: usize) {
    for e in from..ws.objects.len() {
        if let Some(t) = ws.extension_target(e) {
            if let Some(cell) = known.get_mut(t) {
                *cell = std::sync::OnceLock::new();
            }
        }
    }
}

/// [`compile_loaded`] that also keeps what a later pass needs to grow the compile by apps ([`grow`]). The program is the
/// one `compile_loaded` lowers. `None` instead of the kept part when the compile cannot be grown: without dependency apps
/// (no demand), or with a selection, a lean compile or `--tdd` (the kept compile must have checked every object it has).
pub fn compile_loaded_growable(
    ws: Workspace,
    load: std::time::Duration,
    catalog: &al_builtins::catalog::Catalog,
    roots: crate::check::Roots,
) -> (Workspace, Lowered, Option<Growable>) {
    let growable = roots.only_tests.is_none()
        && !roots.lean
        && !roots.reach_only
        && !roots.tdd
        && !roots.eager
        && !roots.object_hashes;
    if !growable {
        let (ws, lowered) = compile_loaded(ws, load, catalog, roots);
        return (ws, lowered, None);
    }
    let started = std::time::Instant::now();
    let members = crate::members::build(&ws);
    let members_time = started.elapsed();
    let roots = crate::check::Roots {
        keep_demand: true,
        ..roots
    };
    let mut checked = crate::check::check_with(&ws, &members, catalog, roots);
    let started = std::time::Instant::now();
    let mut lowered = lower_core(&ws, &members, &checked, true, lower_workers(), None, Export::Facts)
        .expect("no cache, no cache error");
    lowered.timings = CompileTimings {
        load,
        members: members_time,
        check: checked.timings,
        lower: started.elapsed(),
        cache_blocks: Default::default(),
    };
    let exported = lowered.exported.take().expect("the facts were exported");
    let kept = match (checked.demand.take(), checked.kept_demand.take(), checked.kept_codes.take()) {
        (Some(demand), Some(kept), Some(codes)) => Some(Growable {
            prev: crate::check::PrevCheck {
                objects: ws.objects.len(),
                demand,
                kept,
                types: std::mem::take(&mut checked.types),
                resolutions: std::mem::take(&mut checked.resolutions),
                from_checker: std::mem::take(&mut checked.from_checker),
                panicked: std::mem::take(&mut checked.panicked),
                codes,
            },
            skipped: (0..ws.objects.len()).map(|oi| checked.is_skipped(oi)).collect(),
            stale: vec![false; ws.objects.len()],
            members: Vec::new(),
            left_out: (0..ws.objects.len()).map(|oi| checked.is_left_out(oi)).collect(),
            source: std::sync::Arc::new(KeptProgram {
                facts: exported.facts,
                heads: exported.heads,
                program: lowered.program.clone(),
                pos_ranges: lowered.pos_ranges.clone(),
            }),
        }),
        _ => None,
    };
    drop(checked);
    let known = members.into_known();
    let kept = kept.map(|k| Growable { members: known, ..k });
    (ws, lowered, kept)
}

/// The compile of `ws` (the workspace of the compile `kept` was kept from) with `apps` added after its apps, of the
/// kinds `kinds`, for `roots` (ADR-063). The result is what [`compile_loaded`] gives for the grown workspace, except that
/// it may compile more of the kept apps than that one needs (see the module comment). An error gives `apps` back, with
/// the reason: an app already compiled would see one of them (its compile would change), so the caller compiles all
/// the apps from source.
#[allow(clippy::too_many_arguments)]
pub fn grow(
    mut ws: Workspace,
    kept: Growable,
    apps: Vec<AppSource>,
    kinds: Vec<AppKind>,
    symbols: &std::collections::BTreeSet<String>,
    app_symbols: &[Option<std::collections::BTreeSet<String>>],
    catalog: &al_builtins::catalog::Catalog,
    roots: crate::check::Roots,
) -> Result<(Workspace, Lowered), (Vec<AppSource>, String)> {
    let n_old = kept.prev.objects;
    if ws.objects.len() != n_old {
        return Err((apps, "the workspace is not the one the compile was kept from".into()));
    }
    if let Err(why) = ws.can_append(&apps) {
        return Err((apps, why));
    }
    let started = std::time::Instant::now();
    ws.append_apps_of(apps, kinds, symbols, app_symbols)
        .expect("checked by can_append");
    let load = started.elapsed();
    let started = std::time::Instant::now();
    let mut known = kept.members;
    forget_extended(&mut known, &ws, n_old);
    let members = crate::members::MemberTable::with_known(&ws, known);
    let members_time = started.elapsed();
    // What the kept demand reached of each object: the grown one reaches at least that (it continues it).
    let level_before = kept.prev.demand.level.clone();
    let mut reached_before = vec![0u32; n_old];
    for &(o, _) in &kept.prev.demand.reached {
        reached_before[o] += 1;
    }
    let checked = crate::check::check_grown(&ws, &members, catalog, roots, kept.prev);
    let demand = checked.demand.as_ref().expect("a grown check has a demand");
    let mut reached_now = vec![0u32; n_old];
    for &(o, _) in &demand.reached {
        if o < n_old {
            reached_now[o] += 1;
        }
    }
    let src = kept.source;
    crate::provided::check_identity_prefix(&ws, n_old, &*src).map_err(|why| (Vec::new(), why))?;
    let relower = crate::provided::relower_set(&ws, n_old, &*src);
    let blocks: Vec<Option<DepBlock>> = (0..ws.objects.len())
        .map(|oi| {
            let same = oi < n_old
                && !relower[oi]
                && !kept.stale[oi]
                && demand.level[oi] == level_before[oi]
                && reached_now[oi] == reached_before[oi]
                && checked.is_skipped(oi) == kept.skipped[oi]
                && checked.is_left_out(oi) == kept.left_out[oi];
            same.then(DepBlock::placeholder)
        })
        .collect();
    let shared: std::sync::Arc<dyn DepSource + Send> = src.clone();
    let check_time = checked.timings;
    let started = std::time::Instant::now();
    let provided = Provided {
        src: &*src,
        blocks,
        shared: Some(shared),
    };
    let mut lowered = lower_core(&ws, &members, &checked, true, lower_workers(), Some(provided), Export::No)
        .map_err(|why| (Vec::new(), why))?;
    lowered.timings = CompileTimings {
        load,
        members: members_time,
        check: check_time,
        lower: started.elapsed(),
        cache_blocks: Default::default(),
    };
    drop(checked);
    drop(members);
    Ok((ws, lowered))
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::check::Roots;
    use crate::workspace::{AppKind, AppSource};
    use std::collections::BTreeSet;

    fn app(id: &str, deps: &[&str], src: &str) -> AppSource {
        AppSource {
            id: id.into(),
            name: id.into(),
            dependencies: deps.iter().map(|s| s.to_string()).collect(),
            files: vec![("f.al".into(), src.into())],
        }
    }

    const BASE: &str = r#"
table 50000 T
{
    fields { field(1; A; Integer) { } field(2; B; Integer) { } }
    keys { key(PK; A) { } }
    trigger OnInsert() begin B := 1; end;
}
codeunit 50001 Pub
{
    [IntegrationEvent(false, false)]
    procedure OnDone(var N: Integer) begin end;
    procedure Run(): Integer var N: Integer; begin OnDone(N); exit(N); end;
}
interface IFoo { procedure Bump(): Integer; }
codeunit 50002 DImpl implements IFoo { procedure Bump(): Integer begin exit(1); end; }
enum 50003 E implements IFoo { value(0; X) { Implementation = IFoo = DImpl; } }
codeunit 50004 DUse
{
    procedure Call(F: Interface IFoo): Integer begin exit(F.Bump()); end;
    procedure ViaEnum(En: Enum E): Integer var F: Interface IFoo; begin F := En; exit(F.Bump()); end;
    procedure Ins() var R: Record T; begin R.A := 1; R.Insert(true); end;
}
codeunit 50005 Unused { procedure Nothing(): Integer begin exit(5); end; }
codeunit 50006 Later { procedure Called(): Integer begin exit(6); end; }
table 50007 T2 { fields { field(1; K; Integer) { } } keys { key(PK; K) { } } }
codeunit 50008 DUse2 { procedure Touch() var R: Record T2; begin R.K := 1; R.Insert(); end; }
"#;

    const MAIN: &str = r#"
codeunit 70000 AUse
{
    procedure Go(): Integer
    var P: Codeunit Pub; U: Codeunit DUse; U2: Codeunit DUse2;
    begin
        U.Ins();
        U2.Touch();
        exit(P.Run() + U.ViaEnum(Enum::E::X));
    end;
}
codeunit 70001 ATest
{
    Subtype = Test;
    [Test]
    procedure T1() var A: Codeunit AUse; begin A.Go(); end;
}
codeunit 70002 ABad { procedure P() var R: Record T; begin R.NoSuchField := 1; end; }
"#;

    const EXTRA_DEP: &str = r#"
tableextension 60000 XT extends T
{
    fields { field(10; XC; Integer) { } }
    trigger OnBeforeInsert() begin XC := 2; end;
}
codeunit 60001 XSub
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnDone', '', false, false)]
    local procedure OnDone(var N: Integer) begin N := 7; end;
}
codeunit 60002 XImpl implements IFoo { procedure Bump(): Integer begin exit(2); end; }
enumextension 60003 XE extends E { value(1; Y) { Implementation = IFoo = XImpl; } }
codeunit 60004 XTableSub
{
    [EventSubscriber(ObjectType::Table, Database::T, 'OnAfterInsertEvent', '', false, false)]
    local procedure AfterIns(var Rec: Record T) begin end;
}
"#;

    const SUITE: &str = r#"
codeunit 80000 STest
{
    Subtype = Test;
    [Test]
    procedure T2() var A: Codeunit AUse; L: Codeunit Later; begin A.Go(); L.Called(); end;
}
codeunit 80001 SBad { procedure P() var R: Record T; begin R.AlsoNoField := 1; end; }
"#;

    fn calls(l: &Lowered) -> Vec<Vec<Vec<(usize, usize)>>> {
        (0..l.program.objects.len())
            .map(|o| {
                (0..l.program.objects[o].routines.len())
                    .map(|r| {
                        let mut v = l.calls_of(o, r);
                        v.sort_unstable();
                        v.dedup();
                        v
                    })
                    .collect()
            })
            .collect()
    }

    fn assert_same(fresh: &Lowered, grown: &Lowered) {
        grown.program.objects.make_all();
        assert_eq!(fresh.program.objects.len(), grown.program.objects.len());
        for (i, (a, b)) in fresh.program.objects.iter().zip(&grown.program.objects).enumerate() {
            assert!(a == b, "object {i} `{}` differs", a.name);
        }
        assert!(*fresh.program == *grown.program, "the programs differ");
        assert!(fresh.unsupported == grown.unsupported, "the refusals differ");
        assert!(calls(fresh) == calls(grown), "the call graphs differ");
        assert_eq!(fresh.dependency, grown.dependency);
    }

    fn diags(l: &Lowered, apps: std::ops::Range<usize>) -> Vec<String> {
        let mut v: Vec<String> = l
            .al_diagnostics
            .iter()
            .filter(|d| apps.contains(&d.app))
            .map(|d| format!("{d:?}"))
            .collect();
        v.sort();
        v
    }

    /// The main pass's apps, then the apps a later pass adds, as one list with the kind of each.
    fn passes() -> (Vec<AppSource>, Vec<AppKind>, Vec<AppSource>, Vec<AppKind>) {
        (
            vec![app("a", &["d1"], MAIN), app("d1", &[], BASE)],
            vec![AppKind::Workspace, AppKind::Dependency],
            vec![app("x", &["d1"], EXTRA_DEP), app("s", &["a", "d1"], SUITE)],
            vec![AppKind::Dependency, AppKind::Workspace],
        )
    }

    fn fresh(apps: Vec<AppSource>, kinds: Vec<AppKind>, roots: Roots) -> (Workspace, Lowered) {
        let symbols = BTreeSet::new();
        let ws = Workspace::load_apps(apps, kinds, &symbols);
        compile_loaded(ws, Default::default(), &al_builtins::catalog::Catalog::load(), roots)
    }

    #[test]
    fn a_compile_kept_for_growth_lowers_the_program_a_plain_compile_lowers() {
        let (apps, kinds, _, _) = passes();
        let symbols = BTreeSet::new();
        let catalog = al_builtins::catalog::Catalog::load();
        let (_, plain) = fresh(apps.clone(), kinds.clone(), Roots::default());
        let ws = Workspace::load_apps(apps, kinds, &symbols);
        let (_, kept, _) = compile_loaded_growable(ws, Default::default(), &catalog, Roots::default());
        assert_same(&plain, &kept);
    }

    #[test]
    fn a_grown_compile_equals_a_fresh_compile_of_all_its_apps() {
        let (main, main_kinds, more, more_kinds) = passes();
        let symbols = BTreeSet::new();
        let catalog = al_builtins::catalog::Catalog::load();
        let ws = Workspace::load_apps(main.clone(), main_kinds.clone(), &symbols);
        let (ws, main_pass, kept) = compile_loaded_growable(ws, Default::default(), &catalog, Roots::default());
        let kept = kept.expect("the compile can be grown");
        let (_, grown) = grow(ws, kept, more.clone(), more_kinds.clone(), &symbols, &[], &catalog, Roots::default())
            .map_err(|e| e.1)
            .expect("the apps can be added");
        let mut all = main;
        all.extend(more);
        let mut kinds = main_kinds;
        kinds.extend(more_kinds);
        let (ws, fresh) = fresh(all, kinds, Roots::default());
        // What the added apps set off is in both: the subscriber, the table extension's trigger, the implementer the
        // enum extension names, and the routine only the added test calls.
        for name in ["XSub", "XT", "XImpl", "XTableSub", "Later"] {
            let oi = ws.objects.iter().position(|o| o.name == name).expect(name);
            assert!(
                fresh.unsupported[oi]
                    .iter()
                    .any(|r| r.iter().all(|why| !why.contains("not compiled"))),
                "`{name}` is compiled"
            );
        }
        assert_same(&fresh, &grown);
        // The compile errors: those of the added apps are the fresh compile's; those of the kept apps are what their own
        // compile found. (A fresh compile of all apps does not judge `R.NoSuchField` on a table an added app extends:
        // AL0132 skips a table any extension extends, also one of an app the code does not see. BC compiles the main
        // app without the added apps and reports it.)
        assert_eq!(diags(&grown, 2..4), diags(&fresh, 2..4), "the added apps' compile errors");
        assert_eq!(diags(&grown, 0..2), diags(&main_pass, 0..2), "the kept apps' compile errors");
        assert_eq!(diags(&main_pass, 0..2).len(), 1, "{:?}", main_pass.al_diagnostics);
        // The objects the added apps do not change are taken from the kept compile, not lowered again.
        let unused = ws.objects.iter().position(|o| o.name == "Unused").unwrap();
        assert!(grown.provided[unused], "an object nothing changed is taken from the kept compile");
    }

    /// An app of the main pass that the later pass does not have: extends a table, implements an interface the code
    /// uses, subscribes to a reached event. It reaches nothing in the dependency apps that the other code does not.
    const OWN_SUITE: &str = r#"
tableextension 90000 OT extends T { fields { field(30; OC; Integer) { } } }
tableextension 90003 OT2 extends T2 { fields { field(5; OK; Integer) { } } }
codeunit 90001 OImpl implements IFoo { procedure Bump(): Integer begin exit(3); end; }
codeunit 90002 OSub
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnDone', '', false, false)]
    local procedure OnDone(var N: Integer) begin N := 9; end;
}
"#;

    #[test]
    fn a_compile_cut_back_to_its_shared_apps_grows_as_a_fresh_compile_of_the_apps() {
        let (main, main_kinds, more, more_kinds) = passes();
        let symbols = BTreeSet::new();
        let catalog = al_builtins::catalog::Catalog::load();
        let mut with_own = main.clone();
        with_own.push(app("own", &["a", "d1"], OWN_SUITE));
        let mut own_kinds = main_kinds.clone();
        own_kinds.push(AppKind::Workspace);
        let ws = Workspace::load_apps(with_own, own_kinds, &symbols);
        let (mut ws, _, kept) = compile_loaded_growable(ws, Default::default(), &catalog, Roots::default());
        let mut kept = kept.expect("the compile can be grown");
        let gone = kept.truncate(&mut ws, main.len());
        assert_eq!(gone.len(), 1);
        let (_, grown) = grow(ws, kept, more.clone(), more_kinds.clone(), &symbols, &[], &catalog, Roots::default())
            .map_err(|e| e.1)
            .expect("the apps can be added");
        let mut all = main;
        all.extend(more);
        let mut kinds = main_kinds;
        kinds.extend(more_kinds);
        let (ws, fresh) = fresh(all, kinds, Roots::default());
        assert_same(&fresh, &grown);
        // The table the cut app extended is lowered again; an object it did not touch is taken from the kept compile.
        let at = |name: &str| ws.objects.iter().position(|o| o.name == name).unwrap();
        assert!(!grown.provided[at("T")]);
        assert!(!grown.provided[at("T2")], "only the cut app extended T2");
        assert!(grown.provided[at("Unused")]);
    }

    #[test]
    fn apps_an_app_already_compiled_would_see_are_refused() {
        let (main, main_kinds, _, _) = passes();
        let symbols = BTreeSet::new();
        let catalog = al_builtins::catalog::Catalog::load();
        // `a` names `late`, which the main compile does not have: adding it would change what `a` sees.
        let mut main = main;
        main[0].dependencies.push("late".into());
        let ws = Workspace::load_apps(main, main_kinds, &symbols);
        let (ws, _, kept) = compile_loaded_growable(ws, Default::default(), &catalog, Roots::default());
        let late = vec![app("late", &[], "codeunit 61000 Late { }")];
        let refused = grow(ws, kept.expect("growable"), late, vec![AppKind::Dependency], &symbols, &[], &catalog, Roots::default());
        let (apps, why) = refused.err().expect("refused");
        assert_eq!(apps.len(), 1, "the apps are given back");
        assert!(why.contains("would see"), "{why}");
    }
}
