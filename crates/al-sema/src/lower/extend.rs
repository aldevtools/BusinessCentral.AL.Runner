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
    /// What the lowering exported of each object. The program itself stays with the caller, which gives it back to
    /// [`grow`]: the runner changes the program it runs in place (it sets the manifest facts), and a program held here
    /// too would make that change copy it.
    facts: KeptFacts,
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
        let n_obj = ws
            .objects
            .iter()
            .position(|o| o.app >= apps)
            .unwrap_or(ws.objects.len());
        let n_file = ws.files.iter().position(|f| f.app >= apps).unwrap_or(ws.files.len());
        let src = &mut self.facts;
        src.facts.truncate(n_obj);
        src.heads.truncate(n_obj);
        src.pos_ranges.truncate(n_obj);
        let stale = crate::provided::relower_set(ws, n_obj, &self.facts);
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

/// The facts of the objects of a kept compile, as the lowering exported them.
struct KeptFacts {
    facts: Vec<DepFacts>,
    heads: Vec<ir::ObjectHead>,
    pos_ranges: Vec<(u32, u32)>,
}

impl KeptFacts {
    fn from(exported: crate::provided::DepSet, lowered: &Lowered) -> KeptFacts {
        KeptFacts {
            facts: exported.facts,
            heads: exported.heads,
            pos_ranges: lowered.pos_ranges.clone(),
        }
    }
}

/// The facts alone, for what reads only them (`relower_set` when the kept compile is cut back). They hold no object.
impl DepSource for KeptFacts {
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
        Err(format!(
            "object {object}: the facts of a kept compile hold no object; `grow` takes it from the program"
        ))
    }
}

/// The objects of a kept compile, as a source of supplied objects (`provided.rs`): their facts as the lowering exported
/// them, and the objects themselves from the kept compile's program.
struct KeptProgram {
    kept: KeptFacts,
    program: std::sync::Arc<ir::Program>,
}

impl DepSource for KeptProgram {
    fn len(&self) -> usize {
        self.kept.len()
    }
    fn facts(&self, object: usize) -> &DepFacts {
        self.kept.facts(object)
    }
    fn head(&self, object: usize) -> &ir::ObjectHead {
        self.kept.head(object)
    }
    fn block(&self, object: usize) -> Result<DepBlock, String> {
        let o = self
            .program
            .objects
            .get_shared(object)
            .ok_or_else(|| format!("the kept program has no object {object}"))?;
        let (start, len) = self.kept.pos_ranges[object];
        Ok(DepBlock {
            object: Some(std::sync::Arc::clone(o)),
            pos_start: start,
            positions: (start..start + len)
                .map(|id| self.program.positions[id as usize])
                .collect(),
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
    mut ws: Workspace,
    load: std::time::Duration,
    catalog: &al_builtins::catalog::Catalog,
    roots: crate::check::Roots,
) -> (Workspace, Lowered, Option<Growable>) {
    ws.set_user_name_fields_for(catalog.bc_major());
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
    let kept = match (
        checked.demand.take(),
        checked.kept_demand.take(),
        checked.kept_codes.take(),
    ) {
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
            facts: KeptFacts::from(exported, &lowered),
        }),
        _ => None,
    };
    drop(checked);
    let known = members.into_known();
    let kept = kept.map(|k| Growable { members: known, ..k });
    (ws, lowered, kept)
}

/// The compile of `ws` (the workspace of the compile `kept` was kept from) with `apps` added after its apps, of the
/// kinds `kinds`, for `roots` (ADR-063). `program` is the program of the compile `kept` was kept from (the caller may
/// have set the facts of its apps on it; the objects are as lowered). The result is what [`compile_loaded`] gives for the grown workspace, except that
/// it may compile more of the kept apps than that one needs (see the module comment). An error gives `apps` back, with
/// the reason: an app already compiled would see one of them (its compile would change), so the caller compiles all
/// the apps from source.
#[allow(clippy::too_many_arguments)]
pub fn grow(
    ws: Workspace,
    kept: Growable,
    program: std::sync::Arc<ir::Program>,
    apps: Vec<AppSource>,
    kinds: Vec<AppKind>,
    symbols: &std::collections::BTreeSet<String>,
    app_symbols: &[Option<std::collections::BTreeSet<String>>],
    catalog: &al_builtins::catalog::Catalog,
    roots: crate::check::Roots,
) -> Result<(Workspace, Lowered), (Vec<AppSource>, String)> {
    grow_inner(
        ws,
        kept,
        program,
        apps,
        kinds,
        symbols,
        app_symbols,
        catalog,
        roots,
        false,
    )
    .map(|(ws, l, _)| (ws, l))
}

/// What [`grow_growable`] gives: the grown compile and what it keeps for a later pass, or the apps back with the reason
/// it could not grow (see [`grow`]).
pub type Grown = Result<(Workspace, Lowered, Option<Growable>), (Vec<AppSource>, String)>;

/// [`grow`] that also keeps the grown compile for a later pass, as [`compile_loaded_growable`] keeps the main pass's: a
/// third pass of a version grows it (after [`Growable::truncate`] cut it back to the apps that pass also has) instead
/// of compiling everything again. `None` instead of the kept part when the grown compile cannot be grown (the
/// conditions of `compile_loaded_growable`, except a test selection). The program is the one `grow` lowers.
///
/// A test selection is allowed here (a later pass leaves out the main app's test codeunits, `Roots::only_tests`): the
/// chain starts from a compile without one (`compile_loaded_growable`), whose demand reached what every test codeunit of
/// the kept apps reaches, and a grown demand only adds to that. A test codeunit of a kept app that this pass leaves out
/// and a later pass selects again reaches nothing new; its lowered form changes, so it is lowered again.
#[allow(clippy::too_many_arguments)]
pub fn grow_growable(
    ws: Workspace,
    kept: Growable,
    program: std::sync::Arc<ir::Program>,
    apps: Vec<AppSource>,
    kinds: Vec<AppKind>,
    symbols: &std::collections::BTreeSet<String>,
    app_symbols: &[Option<std::collections::BTreeSet<String>>],
    catalog: &al_builtins::catalog::Catalog,
    roots: crate::check::Roots,
) -> Grown {
    let keep = !roots.lean && !roots.reach_only && !roots.tdd && !roots.eager && !roots.object_hashes;
    grow_inner(
        ws,
        kept,
        program,
        apps,
        kinds,
        symbols,
        app_symbols,
        catalog,
        roots,
        keep,
    )
}

#[allow(clippy::too_many_arguments)]
fn grow_inner(
    mut ws: Workspace,
    kept: Growable,
    program: std::sync::Arc<ir::Program>,
    apps: Vec<AppSource>,
    kinds: Vec<AppKind>,
    symbols: &std::collections::BTreeSet<String>,
    app_symbols: &[Option<std::collections::BTreeSet<String>>],
    catalog: &al_builtins::catalog::Catalog,
    roots: crate::check::Roots,
    keep: bool,
) -> Grown {
    let n_old = kept.prev.objects;
    if ws.objects.len() != n_old || program.objects.len() < n_old {
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
    let roots = crate::check::Roots {
        keep_demand: keep,
        ..roots
    };
    let mut checked = crate::check::check_grown(&ws, &members, catalog, roots, kept.prev);
    let demand = checked.demand.as_ref().expect("a grown check has a demand");
    let mut reached_now = vec![0u32; n_old];
    for &(o, _) in &demand.reached {
        if o < n_old {
            reached_now[o] += 1;
        }
    }
    let src = std::sync::Arc::new(KeptProgram {
        kept: kept.facts,
        program,
    });
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
    let supplied: Vec<bool> = blocks.iter().map(Option::is_some).collect();
    let shared: std::sync::Arc<dyn DepSource + Send> = src.clone();
    let check_time = checked.timings;
    let started = std::time::Instant::now();
    let provided = Provided {
        src: &*src,
        blocks,
        shared: Some(shared),
        kept: true,
    };
    let mut lowered = lower_core(
        &ws,
        &members,
        &checked,
        true,
        lower_workers(),
        Some(provided),
        if keep { Export::Facts } else { Export::No },
    )
    .map_err(|why| (Vec::new(), why))?;
    lowered.timings = CompileTimings {
        load,
        members: members_time,
        check: check_time,
        lower: started.elapsed(),
        cache_blocks: Default::default(),
    };
    let grown = if keep {
        kept_grown(&ws, &mut checked, &mut lowered, &supplied, &src)
    } else {
        None
    };
    drop(checked);
    let known = members.into_known();
    let grown = grown.map(|g| Growable { members: known, ..g });
    Ok((ws, lowered, grown))
}

/// What a grown compile keeps for a later pass ([`grow_growable`]); the members are the caller's to add. An object the
/// grown compile took from the kept one (`supplied`) keeps the kept compile's facts: the export of a supplied object
/// holds only its key. Its block comes from the grown program, which makes it from the kept one on first touch.
fn kept_grown(
    ws: &Workspace,
    checked: &mut crate::check::CheckResult,
    lowered: &mut Lowered,
    supplied: &[bool],
    before: &KeptProgram,
) -> Option<Growable> {
    let mut exported = lowered.exported.take()?;
    for (oi, facts) in exported.facts.iter_mut().enumerate() {
        if supplied[oi] {
            *facts = before.kept.facts[oi].clone();
        }
    }
    let (demand, kept, codes) = (
        checked.demand.take()?,
        checked.kept_demand.take()?,
        checked.kept_codes.take()?,
    );
    Some(Growable {
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
        // Every object is lowered for the grown workspace: what a later cut takes away marks it (`truncate`).
        stale: vec![false; ws.objects.len()],
        members: Vec::new(),
        left_out: (0..ws.objects.len()).map(|oi| checked.is_left_out(oi)).collect(),
        facts: KeptFacts::from(exported, lowered),
    })
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

    // The runner sets the manifest facts on the program it runs in place (`Arc::make_mut`, al-testrun
    // `apply_manifests`). A kept compile that also held the program made that a copy of the whole program (1.1 GB on
    // the corpus), so the kept compile leaves the program to its caller, which gives it back to `grow`.
    #[test]
    fn a_kept_compile_does_not_share_the_program_it_lowered() {
        let (main, main_kinds, more, more_kinds) = passes();
        let symbols = BTreeSet::new();
        let catalog = al_builtins::catalog::Catalog::load();
        let ws = Workspace::load_apps(main, main_kinds, &symbols);
        let (ws, mut main_pass, kept) = compile_loaded_growable(ws, Default::default(), &catalog, Roots::default());
        assert!(
            std::sync::Arc::get_mut(&mut main_pass.program).is_some(),
            "the main pass's program is shared"
        );
        let (_, mut grown, kept) = grow_growable(
            ws,
            kept.expect("growable"),
            main_pass.program.clone(),
            more,
            more_kinds,
            &symbols,
            &[],
            &catalog,
            Roots::default(),
        )
        .map_err(|e| e.1)
        .expect("the apps can be added");
        assert!(kept.is_some(), "the grown compile is kept");
        assert!(
            std::sync::Arc::get_mut(&mut grown.program).is_some(),
            "the grown program is shared"
        );
    }

    #[test]
    fn a_grown_compile_equals_a_fresh_compile_of_all_its_apps() {
        let (main, main_kinds, more, more_kinds) = passes();
        let symbols = BTreeSet::new();
        let catalog = al_builtins::catalog::Catalog::load();
        let ws = Workspace::load_apps(main.clone(), main_kinds.clone(), &symbols);
        let (ws, main_pass, kept) = compile_loaded_growable(ws, Default::default(), &catalog, Roots::default());
        let kept = kept.expect("the compile can be grown");
        let (_, grown) = grow(
            ws,
            kept,
            main_pass.program.clone(),
            more.clone(),
            more_kinds.clone(),
            &symbols,
            &[],
            &catalog,
            Roots::default(),
        )
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
        assert_eq!(
            diags(&grown, 2..4),
            diags(&fresh, 2..4),
            "the added apps' compile errors"
        );
        assert_eq!(
            diags(&grown, 0..2),
            diags(&main_pass, 0..2),
            "the kept apps' compile errors"
        );
        assert_eq!(diags(&main_pass, 0..2).len(), 1, "{:?}", main_pass.al_diagnostics);
        // The objects the added apps do not change are taken from the kept compile, not lowered again.
        let unused = ws.objects.iter().position(|o| o.name == "Unused").unwrap();
        assert!(
            grown.provided[unused],
            "an object nothing changed is taken from the kept compile"
        );
        // ... and shared with the kept compile's program, not copied.
        assert!(
            std::ptr::eq(&grown.program.objects[unused], &main_pass.program.objects[unused]),
            "an object taken from the kept compile is the kept program's object"
        );
        // The work ratchet counts them apart from what a compile cache supplies.
        assert_eq!(
            grown.work.objects_from_kept,
            grown.provided.iter().filter(|&&p| p).count()
        );
        assert_eq!(grown.work.objects_from_cache, 0);
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
        let (mut ws, main_pass, kept) = compile_loaded_growable(ws, Default::default(), &catalog, Roots::default());
        let mut kept = kept.expect("the compile can be grown");
        let gone = kept.truncate(&mut ws, main.len());
        assert_eq!(gone.len(), 1);
        let (_, grown) = grow(
            ws,
            kept,
            main_pass.program,
            more.clone(),
            more_kinds.clone(),
            &symbols,
            &[],
            &catalog,
            Roots::default(),
        )
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

    /// The suite app of a third pass: a test that reaches only what the main app reaches.
    const THIRD: &str = r#"
codeunit 85000 TTest
{
    Subtype = Test;
    [Test]
    procedure T3() var A: Codeunit AUse; begin A.Go(); end;
}
"#;

    /// The suite app of a second pass that reaches nothing the main app does not: a grown compile keeps what the roots of
    /// every earlier pass reached (ADR-063 as built, point 4), so a fresh compile of the third pass's apps would leave
    /// out what only the second pass's tests reach.
    const SECOND: &str = r#"
codeunit 82000 S2Test
{
    Subtype = Test;
    [Test]
    procedure T2() var A: Codeunit AUse; begin A.Go(); end;
}
"#;

    /// The apps of a second pass: `with_dep` adds the dependency app that extends, implements and subscribes.
    fn second(with_dep: bool) -> (Vec<AppSource>, Vec<AppKind>) {
        let (_, _, more, more_kinds) = passes();
        let suite = app("s2", &["a", "d1"], SECOND);
        if with_dep {
            (vec![more[0].clone(), suite], vec![more_kinds[0], AppKind::Workspace])
        } else {
            (vec![suite], vec![AppKind::Workspace])
        }
    }

    /// Grows the kept compile of the main pass (`main`, cut back from `main` plus `own`) by `more`, keeps the grown
    /// compile, cuts it back to the main pass's apps and grows that by the app of a third pass.
    fn third_pass(more: Vec<AppSource>, more_kinds: Vec<AppKind>) -> (Workspace, Lowered, Lowered) {
        third_pass_with(more, more_kinds, Roots::default(), Roots::default())
    }

    /// [`third_pass`] with the roots of the second and the third pass (the fresh compile has the third's).
    fn third_pass_with(
        more: Vec<AppSource>,
        more_kinds: Vec<AppKind>,
        second_roots: Roots,
        third_roots: Roots,
    ) -> (Workspace, Lowered, Lowered) {
        let (main, main_kinds, _, _) = passes();
        let symbols = BTreeSet::new();
        let catalog = al_builtins::catalog::Catalog::load();
        let mut with_own = main.clone();
        with_own.push(app("own", &["a", "d1"], OWN_SUITE));
        let mut own_kinds = main_kinds.clone();
        own_kinds.push(AppKind::Workspace);
        let ws = Workspace::load_apps(with_own, own_kinds, &symbols);
        let (mut ws, main_pass, kept) = compile_loaded_growable(ws, Default::default(), &catalog, Roots::default());
        let mut kept = kept.expect("the compile can be grown");
        kept.truncate(&mut ws, main.len());
        let n_more = more.len();
        let (mut ws, second, kept) = grow_growable(
            ws,
            kept,
            main_pass.program,
            more,
            more_kinds,
            &symbols,
            &[],
            &catalog,
            second_roots,
        )
        .map_err(|e| e.1)
        .expect("the apps can be added");
        let mut kept = kept.expect("the grown compile can be grown again");
        let gone = kept.truncate(&mut ws, main.len());
        assert_eq!(gone.len(), n_more, "the second pass's apps leave the kept compile");
        let third = vec![app("t", &["a", "d1"], THIRD)];
        let (_, grown) = grow(
            ws,
            kept,
            second.program,
            third.clone(),
            vec![AppKind::Workspace],
            &symbols,
            &[],
            &catalog,
            third_roots.clone(),
        )
        .map_err(|e| e.1)
        .expect("the third pass's app can be added");
        let mut all = main;
        all.extend(third);
        let mut kinds = main_kinds;
        kinds.push(AppKind::Workspace);
        let (ws, fresh) = fresh(all, kinds, third_roots);
        (ws, fresh, grown)
    }

    // ADR-063, a third pass (the corpus's TryWrites suite after the testlibraries one): the second pass keeps its grown
    // compile, cut back to the main pass's apps, and the third pass grows it as the second pass grew the main one.
    #[test]
    fn a_grown_compile_kept_and_cut_back_grows_as_a_fresh_compile_of_the_apps() {
        let (more, more_kinds) = second(true);
        let (ws, fresh, grown) = third_pass(more, more_kinds);
        assert_same(&fresh, &grown);
        let at = |name: &str| ws.objects.iter().position(|o| o.name == name).unwrap();
        // The table the second pass's dependency app extended is lowered again; an untouched object is not.
        assert!(!grown.provided[at("T")]);
        assert!(grown.provided[at("Unused")]);
    }

    // The third pass of a version whose apps add no dependency app and reach nothing new in the dependency apps (the
    // TryWrites suite) lowers no dependency routine: every dependency object comes from the kept compile.
    #[test]
    fn a_third_pass_that_reaches_nothing_new_lowers_no_dependency_routine() {
        // Only the second pass's suite app: no dependency app of its own.
        let (more, more_kinds) = second(false);
        let (ws, fresh, grown) = third_pass(more, more_kinds);
        assert_same(&fresh, &grown);
        assert_eq!(grown.work.dependency_routines_lowered, 0, "{:?}", grown.work);
        assert_eq!(grown.work.dependency_objects_lowered, 0, "{:?}", grown.work);
        let deps = (0..ws.objects.len()).filter(|&o| ws.is_dependency(o)).count();
        assert!(grown.work.objects_from_kept >= deps, "{:?}", grown.work);
    }

    // The corpus's later passes select their own suites' tests (`Roots::only_tests`): the main app's test codeunits are
    // left out. A second pass with a selection still keeps its compile, and the third pass, with its own selection,
    // equals a fresh compile of its apps with that selection.
    #[test]
    fn a_grown_compile_with_a_test_selection_is_kept_for_a_third_pass() {
        let (more, more_kinds) = second(false);
        let only = |app: usize, id: u32| Roots {
            only_tests: Some(vec![(app, id)]),
            ..Roots::default()
        };
        // The main pass's apps are `a` and `d1`: the second and the third pass's suite app is app 2.
        let (_, fresh, grown) = third_pass_with(more, more_kinds, only(2, 82000), only(2, 85000));
        assert_same(&fresh, &grown);
        assert_eq!(grown.work.dependency_routines_lowered, 0, "{:?}", grown.work);
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
        let (ws, lowered, kept) = compile_loaded_growable(ws, Default::default(), &catalog, Roots::default());
        let late = vec![app("late", &[], "codeunit 61000 Late { }")];
        let refused = grow(
            ws,
            kept.expect("growable"),
            lowered.program,
            late,
            vec![AppKind::Dependency],
            &symbols,
            &[],
            &catalog,
            Roots::default(),
        );
        let (apps, why) = refused.err().expect("refused");
        assert_eq!(apps.len(), 1, "the apps are given back");
        assert!(why.contains("would see"), "{why}");
    }
}
