//! Entry points that load, check and lower in one go, and the cached layer variants.
use super::*;

/// Load, check and lower in one go, against the newest catalog (nothing hidden), with source
/// positions.
pub fn compile(
    apps: Vec<crate::workspace::AppSource>,
    symbols: &std::collections::BTreeSet<String>,
) -> (Workspace, Lowered) {
    let kinds = vec![crate::workspace::AppKind::Workspace; apps.len()];
    compile_apps(apps, kinds, symbols)
}

/// Like [`compile`], with the kind of each app (dependency apps, ADR-013).
pub fn compile_apps(
    apps: Vec<crate::workspace::AppSource>,
    kinds: Vec<crate::workspace::AppKind>,
    symbols: &std::collections::BTreeSet<String>,
) -> (Workspace, Lowered) {
    compile_with(apps, kinds, symbols, &al_builtins::catalog::Catalog::load())
}

/// Like [`compile`], for a run target: builtins newer than the target's runtime are
/// refused (ADR-015 3.3).
pub fn compile_for(
    apps: Vec<crate::workspace::AppSource>,
    symbols: &std::collections::BTreeSet<String>,
    target: &al_version::Target,
) -> (Workspace, Lowered) {
    let kinds = vec![crate::workspace::AppKind::Workspace; apps.len()];
    compile_apps_for(apps, kinds, symbols, target)
}

/// [`compile_for`] with the kind of each app.
pub fn compile_apps_for(
    apps: Vec<crate::workspace::AppSource>,
    kinds: Vec<crate::workspace::AppKind>,
    symbols: &std::collections::BTreeSet<String>,
    target: &al_version::Target,
) -> (Workspace, Lowered) {
    compile_with(apps, kinds, symbols, &al_builtins::catalog::Catalog::for_target(target))
}

/// [`compile_apps_for`] (or the newest catalog without a target) with extra demand roots: the
/// install baseline (ADR-013 decision 5).
pub fn compile_apps_with(
    apps: Vec<crate::workspace::AppSource>,
    kinds: Vec<crate::workspace::AppKind>,
    symbols: &std::collections::BTreeSet<String>,
    target: Option<&al_version::Target>,
    roots: crate::check::Roots,
) -> (Workspace, Lowered) {
    let catalog = match target {
        Some(t) => al_builtins::catalog::Catalog::for_target(t),
        None => al_builtins::catalog::Catalog::load(),
    };
    compile_with_roots(apps, kinds, symbols, &catalog, roots)
}

/// The lowered form of the dependency apps `apps`, built by a complete compile of them alone,
/// for a compile cache (ADR-022). The apps are in the order a run puts them: dependencies first.
pub fn lower_for_cache(
    apps: Vec<crate::workspace::AppSource>,
    symbols: &std::collections::BTreeSet<String>,
    target: Option<&al_version::Target>,
) -> crate::provided::DepSet {
    lower_for_cache_with(apps, &[], symbols, target)
}

/// [`lower_for_cache`] where app `i` with `app_symbols[i]` set is preprocessed with those symbols.
pub fn lower_for_cache_with(
    apps: Vec<crate::workspace::AppSource>,
    app_symbols: &[Option<std::collections::BTreeSet<String>>],
    symbols: &std::collections::BTreeSet<String>,
    target: Option<&al_version::Target>,
) -> crate::provided::DepSet {
    let catalog = match target {
        Some(t) => al_builtins::catalog::Catalog::for_target(t),
        None => al_builtins::catalog::Catalog::load(),
    };
    let kinds = vec![crate::workspace::AppKind::Dependency; apps.len()];
    let ws = Workspace::load_apps_symbols(apps, kinds, symbols, app_symbols, Vec::new());
    let members = crate::members::build(&ws);
    let roots = crate::check::Roots {
        eager: true,
        ..Default::default()
    };
    let checked = crate::check::check_with(&ws, &members, &catalog, roots);
    let mut set = lower_core(&ws, &members, &checked, true, lower_workers(), None, true)
        .expect("no cache, no cache error")
        .exported
        .expect("exported");
    set.parse_problems = ws.lazy_parse_problems();
    set
}

/// The blocks a run takes from `src` for the first `n_prov` objects of `ws`: the identity of the
/// objects is checked, and a block the apps above them change (`relower_set`) is left out, to be
/// lowered again from source.
pub(crate) fn provided_blocks(
    ws: &Workspace,
    n_prov: usize,
    src: &dyn crate::provided::DepSource,
    decode: bool,
) -> Result<Vec<Option<crate::provided::DepBlock>>, String> {
    use rayon::prelude::*;
    crate::provided::check_identity(ws, n_prov, src)?;
    let relower = crate::provided::relower_set(ws, n_prov, src);
    (0..ws.objects.len())
        .into_par_iter()
        .map(|oi| {
            if oi < n_prov && !relower[oi] {
                if decode {
                    src.block(oi).map(Some)
                } else {
                    Ok(Some(crate::provided::DepBlock::placeholder()))
                }
            } else {
                Ok(None)
            }
        })
        .collect()
}

/// One layer of the dependency set (ADR-022, owner decision 2026-10-15). `apps` are the apps of
/// the layers below followed by the layer's own, in program order; `n_below_apps` is how many
/// belong to the layers below; `below` describes their objects, which come first in the program.
/// The layer's objects are compiled completely, the objects below are taken from `below` except
/// those the layer changes, which are lowered again. The result holds the layer's own objects and
/// those lowered again, so that the layers below and this one give, object for object, what one
/// complete compile of all the apps gives.
pub fn lower_layer(
    apps: Vec<crate::workspace::AppSource>,
    n_below_apps: usize,
    symbols: &std::collections::BTreeSet<String>,
    target: Option<&al_version::Target>,
    below: &dyn crate::provided::DepSource,
) -> Result<crate::provided::DepLayer, String> {
    lower_layer_with(apps, n_below_apps, &[], symbols, target, below)
}

/// [`lower_layer`] where app `i` with `app_symbols[i]` set is preprocessed with those symbols.
pub fn lower_layer_with(
    apps: Vec<crate::workspace::AppSource>,
    n_below_apps: usize,
    app_symbols: &[Option<std::collections::BTreeSet<String>>],
    symbols: &std::collections::BTreeSet<String>,
    target: Option<&al_version::Target>,
    below: &dyn crate::provided::DepSource,
) -> Result<crate::provided::DepLayer, String> {
    let catalog = match target {
        Some(t) => al_builtins::catalog::Catalog::for_target(t),
        None => al_builtins::catalog::Catalog::load(),
    };
    let kinds = vec![crate::workspace::AppKind::Dependency; apps.len()];
    let ws = Workspace::load_apps_symbols(apps, kinds, symbols, app_symbols, Vec::new());
    let n_prov = below.len();
    if n_prov > ws.objects.len() {
        return Err(format!(
            "the layers below hold {n_prov} objects, the apps have {}",
            ws.objects.len()
        ));
    }
    // A layer's result holds only the objects lowered here, so the blocks below are not decoded.
    let blocks = provided_blocks(&ws, n_prov, below, false)?;
    let provided = Provided { src: below, blocks };
    let members = crate::members::build(&ws);
    let mut roots = crate::check::Roots {
        eager: true,
        ..Default::default()
    };
    roots.provided = (0..ws.objects.len()).map(|oi| provided.has(oi)).collect();
    roots.provided_queries = (0..ws.objects.len())
        .map(|oi| provided.has(oi).then(|| below.facts(oi).query.clone()).flatten())
        .collect();
    let checked = crate::check::check_with(&ws, &members, &catalog, roots);
    let lowered = lower_core(&ws, &members, &checked, true, lower_workers(), Some(provided), true)?;
    let all = lowered.exported.expect("exported");
    let mut layer = crate::provided::DepLayer {
        base_len: n_prov,
        ..Default::default()
    };
    // Facts and direct calls of an object taken from `below` are not complete in the export, and
    // are not needed: only the objects lowered here are kept.
    for (oi, (facts, block)) in all.facts.into_iter().zip(all.blocks).enumerate() {
        if !lowered.provided[oi] {
            layer.objects.push(oi as u32);
            layer.set.facts.push(facts);
            layer.set.blocks.push(block);
        }
    }
    layer.set.parse_problems = ws.lazy_parse_problems_from(n_below_apps);
    Ok(layer)
}

/// [`compile_apps_with`] for a run whose dependency apps (first in `apps`) are described by `src`:
/// their objects come from it, except the ones the workspace changes (see `provided.rs`). The
/// result is the complete (eager) compile. An error says the source does not fit the apps or is
/// damaged; nothing was compiled then.
pub fn compile_apps_cached(
    apps: Vec<crate::workspace::AppSource>,
    kinds: Vec<crate::workspace::AppKind>,
    symbols: &std::collections::BTreeSet<String>,
    target: Option<&al_version::Target>,
    roots: crate::check::Roots,
    src: &dyn crate::provided::DepSource,
) -> Result<(Workspace, Lowered), Box<(Workspace, String)>> {
    let catalog = match target {
        Some(t) => al_builtins::catalog::Catalog::for_target(t),
        None => al_builtins::catalog::Catalog::load(),
    };
    let started = std::time::Instant::now();
    let ws = Workspace::load_apps(apps, kinds, symbols);
    let load = started.elapsed();
    compile_loaded_cached(ws, load, &catalog, roots, src)
}

/// [`compile_apps_cached`] on a workspace that is loaded. An error gives the workspace back, so
/// the caller can compile it without the cache.
pub fn compile_loaded_cached(
    ws: Workspace,
    load: std::time::Duration,
    catalog: &al_builtins::catalog::Catalog,
    mut roots: crate::check::Roots,
    src: &dyn crate::provided::DepSource,
) -> Result<(Workspace, Lowered), Box<(Workspace, String)>> {
    let blocks_started = std::time::Instant::now();
    let prepared = (|| {
        let n_dep = crate::provided::dependency_prefix(&ws).ok_or("a dependency app follows a workspace app")?;
        provided_blocks(&ws, n_dep, src, true)
    })();
    let blocks = match prepared {
        Ok(b) => b,
        Err(why) => return Err(Box::new((ws, why))),
    };
    let provided = Provided { src, blocks };
    let cache_blocks = blocks_started.elapsed();
    let started = std::time::Instant::now();
    let members = crate::members::build(&ws);
    let members_time = started.elapsed();
    roots.eager = true;
    roots.provided = (0..ws.objects.len()).map(|oi| provided.has(oi)).collect();
    roots.provided_queries = (0..ws.objects.len())
        .map(|oi| provided.has(oi).then(|| src.facts(oi).query.clone()).flatten())
        .collect();
    let checked = crate::check::check_with(&ws, &members, catalog, roots);
    let started = std::time::Instant::now();
    let mut lowered = lower_core(&ws, &members, &checked, true, lower_workers(), Some(provided), false)
        .expect("a cache that passed its checks lowers");
    lowered.timings = CompileTimings {
        load,
        members: members_time,
        check: checked.timings,
        lower: started.elapsed(),
        cache_blocks,
    };
    drop(members);
    Ok((ws, lowered))
}

pub(crate) fn compile_with(
    apps: Vec<crate::workspace::AppSource>,
    kinds: Vec<crate::workspace::AppKind>,
    symbols: &std::collections::BTreeSet<String>,
    catalog: &al_builtins::catalog::Catalog,
) -> (Workspace, Lowered) {
    compile_with_roots(apps, kinds, symbols, catalog, Default::default())
}

pub(crate) fn compile_with_roots(
    apps: Vec<crate::workspace::AppSource>,
    kinds: Vec<crate::workspace::AppKind>,
    symbols: &std::collections::BTreeSet<String>,
    catalog: &al_builtins::catalog::Catalog,
    roots: crate::check::Roots,
) -> (Workspace, Lowered) {
    let started = std::time::Instant::now();
    let ws = Workspace::load_apps(apps, kinds.clone(), symbols);
    let load = started.elapsed();
    let (mut ws, mut lowered) = compile_loaded(ws, load, catalog, roots.clone());
    // `--tdd` (ADR-051 decision 10): the declarations the code needs are added to the apps, which are compiled again.
    let mut seen = Vec::new();
    for _ in 0..crate::check::tdd_decl::MAX_RECOMPILES {
        if !roots.tdd || !crate::check::tdd_decl::take_new(&mut seen, &lowered.tdd_needs) {
            break;
        }
        let files = crate::check::tdd_decl::stub_files(&ws, &seen, &roots.tdd_id_ranges);
        let mut apps = std::mem::take(&mut ws.apps);
        crate::check::tdd_decl::set_stub_files(&mut apps, files);
        let started = std::time::Instant::now();
        let again = Workspace::load_apps(apps, kinds.clone(), symbols);
        (ws, lowered) = compile_loaded(again, started.elapsed(), catalog, roots.clone());
    }
    (ws, lowered)
}

/// The compile of a workspace that is loaded (`load` is how long that took, for the timings).
pub fn compile_loaded(
    ws: Workspace,
    load: std::time::Duration,
    catalog: &al_builtins::catalog::Catalog,
    roots: crate::check::Roots,
) -> (Workspace, Lowered) {
    let started = std::time::Instant::now();
    let members = crate::members::build(&ws);
    let members_time = started.elapsed();
    let checked = crate::check::check_with(&ws, &members, catalog, roots);
    let started = std::time::Instant::now();
    let mut lowered = lower(&ws, &members, &checked);
    lowered.timings = CompileTimings {
        load,
        members: members_time,
        check: checked.timings,
        lower: started.elapsed(),
        cache_blocks: Default::default(),
    };
    drop(checked);
    drop(members);
    (ws, lowered)
}
