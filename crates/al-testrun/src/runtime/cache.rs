//! Compiling through the compile cache.
use super::*;

/// The index of the lazily read dependency apps from the cache folder, as the `LazyDep`s a workspace
/// loads from (by app; `None` for an app that has its text). `Err(key)`: there is none that fits, and
/// `key` is where to save one.
pub(crate) fn lazy_index(
    cc: &crate::compile_cache::CompileCache,
    apps: &[al_sema::workspace::AppSource],
    manifests: &[Manifest],
    lazy: &[Option<al_sema::LazySource>],
    identities: &[Option<[u8; 32]>],
) -> Result<Vec<Option<al_sema::LazyDep>>, Box<al_cache::Key>> {
    let which: Vec<usize> = (0..apps.len()).filter(|&i| lazy[i].is_some()).collect();
    let refs: Vec<&al_sema::workspace::AppSource> = which.iter().map(|&i| &apps[i]).collect();
    let mans: Vec<Manifest> = which
        .iter()
        .map(|&i| manifests.get(i).cloned().unwrap_or_default())
        .collect();
    let ids: Vec<Option<[u8; 32]>> = which.iter().map(|&i| identities[i]).collect();
    let key = crate::compile_cache::index_key(&refs, &mans, &ids);
    let fits = |index: &Vec<Vec<al_sema::FileIndex>>| {
        index.len() == which.len() && index.iter().zip(&which).all(|(f, &i)| f.len() == apps[i].files.len())
    };
    match (!cc.rebuild)
        .then(|| crate::compile_cache::read_index(&cc.dir, &key))
        .flatten()
    {
        Some(index) if fits(&index) => {
            let mut index = index.into_iter();
            Ok((0..apps.len())
                .map(|i| {
                    lazy[i].as_ref().map(|l| al_sema::LazyDep {
                        index: index.next().expect("one index per lazy app"),
                        loader: l.loader.clone(),
                    })
                })
                .collect())
        }
        _ => Err(Box::new(key)),
    }
}

/// Saves the scan of the lazily read apps of `ws` (loaded with their text) as the index for `key`.
pub(crate) fn save_index(
    cc: &crate::compile_cache::CompileCache,
    ws: &al_sema::Workspace,
    lazy: &[Option<al_sema::LazySource>],
    key: &al_cache::Key,
) {
    let mut all = ws.export_index();
    let index: Vec<Vec<al_sema::FileIndex>> = (0..lazy.len())
        .filter(|&i| lazy[i].is_some())
        .map(|i| std::mem::take(&mut all[i]))
        .collect();
    crate::compile_cache::write_index(&cc.dir, key, &index);
}

/// What the compile with the cache gives: the workspace, its lowering, the report, and the opened layers
/// when the entry was used (the incremental compile reads the dependency facts from them).
pub(crate) type CachedCompile = (
    al_sema::Workspace,
    Lowered,
    Option<crate::compile_cache::CacheReport>,
    Option<crate::compile_cache::Layers>,
);

/// The compile of a run with the compile cache (ADR-022): the key of the dependency apps (the
/// first `kinds` entries), the entry from the cache folder or built now, then the cached compile.
/// An entry that does not fit or is damaged is ignored (and removed), and the run compiles without
/// it; with `require` it is an error instead.
#[allow(clippy::too_many_arguments)]
pub(crate) fn compile_with_cache(
    cc: &crate::compile_cache::CompileCache,
    mut apps: Vec<al_sema::workspace::AppSource>,
    kinds: Vec<al_sema::AppKind>,
    manifests: &[Manifest],
    symbols: &BTreeSet<String>,
    target: Option<&al_version::Target>,
    lazy: &[Option<al_sema::LazySource>],
    identities: &[Option<[u8; 32]>],
    roots: al_sema::check::Roots,
) -> Result<CachedCompile, String> {
    use crate::compile_cache as cache;
    let n_dep = kinds.iter().take_while(|k| **k == al_sema::AppKind::Dependency).count();
    let dep_symbols: Vec<Option<BTreeSet<String>>> = kinds
        .iter()
        .enumerate()
        .map(|(i, k)| {
            (*k == al_sema::AppKind::Dependency).then(|| {
                manifests
                    .get(i)
                    .map(|m| m.symbols.iter().cloned().collect())
                    .unwrap_or_default()
            })
        })
        .collect();
    // The text of a lazily read app is read when something needs it: building an entry does, a hit
    // does not.
    let filled = std::cell::Cell::new(false);
    let fill = |apps: &mut [al_sema::workspace::AppSource]| {
        if !filled.replace(true) {
            for (a, l) in apps.iter_mut().zip(lazy) {
                if let Some(l) = l {
                    l.fill(a);
                }
            }
        }
    };
    let (layers, mut report) = cache::open_or_build_with(
        cc,
        &mut apps[..n_dep],
        &manifests[..n_dep],
        target,
        &identities[..n_dep.min(identities.len())],
        &mut |a| fill(a),
    )?;
    // `open_or_build_with` checked that the layers fit each other.
    let stack = layers.shared_stack().map_err(|why| format!("compile cache: {why}"))?;
    report.objects = stack.len();
    report.parse_problems = layers.parse_problems();
    let catalog = match target {
        Some(t) => al_builtins::catalog::Catalog::for_target(t),
        None => al_builtins::catalog::Catalog::load(),
    };
    let started = std::time::Instant::now();
    // A lazily read app is loaded from its index when there is one: its sources are neither read nor
    // scanned. Otherwise they are read now (once), scanned, and the index is saved.
    let ws = if lazy.iter().any(Option::is_some) {
        match lazy_index(cc, &apps, manifests, lazy, identities) {
            Ok(deps) => {
                report.source_index = Some("read");
                al_sema::Workspace::load_apps_symbols(apps, kinds, symbols, &dep_symbols, deps)
            }
            Err(key) => {
                fill(&mut apps);
                let ws = al_sema::Workspace::load_apps_symbols(apps, kinds, symbols, &dep_symbols, Vec::new());
                save_index(cc, &ws, lazy, &key);
                report.source_index = Some("built");
                ws
            }
        }
    } else {
        al_sema::Workspace::load_apps_symbols(apps, kinds, symbols, &dep_symbols, Vec::new())
    };
    let load = started.elapsed();
    match al_sema::lower::compile_loaded_cached(ws, load, &catalog, roots.clone(), stack) {
        Ok((ws, lowered)) => {
            report.relowered = (0..lowered.provided.len())
                .filter(|&i| lowered.dependency[i] && !lowered.provided[i])
                .count();
            Ok((ws, lowered, Some(report), Some(layers)))
        }
        Err(failed) => {
            let (ws, why) = *failed;
            let note = format!("the entry is not used: {why}");
            eprintln!("compile cache: {note}");
            report.notes.push(note);
            report.hit = false;
            for l in &report.layers {
                let _ = std::fs::remove_file(&l.file);
            }
            if cc.require {
                return Err(format!("--require-compile-cache: {}", report.notes.join("; ")));
            }
            let (ws, lowered) = al_sema::lower::compile_loaded(ws, load, &catalog, roots);
            Ok((ws, lowered, Some(report), None))
        }
    }
}

/// Finds the subscribers of the platform codeunit `Global Triggers` in dependency apps that cannot
/// run (Base Application's `GlobalTriggerManagement` reaches platform tables that have no row
/// provider yet, and `Database::User`) and tells the engine, which refuses to raise an event one
/// of them would receive (`dispatch.rs`). Delete this when nothing is unavailable.
pub fn global_triggers_find_unavailable(engine: &mut Arc<Engine>, lowered: &Lowered) {
    let key = engine.events.global_key;
    let mut unavailable = std::collections::HashMap::new();
    for ((object, event), subs) in &engine.events.subscribers {
        if *object != key {
            continue;
        }
        for s in subs {
            if !lowered.dependency[s.object] {
                continue;
            }
            let Some(why) = blocked_reason(lowered, s.object, s.routine) else {
                continue;
            };
            // A subscriber of an operation nothing used was never compiled; it is never asked.
            if why.contains("not compiled") {
                continue;
            }
            // What its decision reads: record variables of the routines within two calls.
            let mut config = BTreeSet::new();
            let mut level = vec![(s.object, s.routine)];
            for _ in 0..3 {
                let mut next = Vec::new();
                for (o, r) in level {
                    for slot in &lowered.program.objects[o].routines[r].locals {
                        if let al_builtins::value::RtType::Record(t, false) = &slot.ty {
                            config.insert(*t);
                        }
                    }
                    lowered.calls_into(o, r, &mut next);
                }
                level = next;
            }
            let _ = event;
            unavailable.insert(
                (s.object, s.routine),
                al_vm::events::UnavailableGlobal {
                    why,
                    config: config.into_iter().collect(),
                },
            );
        }
    }
    if !unavailable.is_empty() {
        eprintln!(
            "Global Triggers: {} dependency subscriber(s) cannot run; a write that would reach one is not run",
            unavailable.len()
        );
    }
    if let Some(e) = Arc::get_mut(engine) {
        e.events.unavailable_global = unavailable;
    }
}
