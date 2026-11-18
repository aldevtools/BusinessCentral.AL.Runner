//! The workspace layer in the compile of a run (ADR-022 amendment 2026-11-15, prototype): the key of the run, the read of
//! an entry in place of the compile (decision 4), and the store of a fresh compile (decisions 6 and 9). The key, the
//! entry's extra section and the report are `crate::workspace_cache`; the format is `al_cache::build_workspace`.

use super::*;
use crate::compile_cache::{self as cc, CompileCache};
use crate::workspace_cache::{self as wc, Decided, WorkspaceExtra, WorkspaceKey, WorkspaceReport, WorkspaceState};
use std::time::Instant;

/// What decides the workspace layer of one compile, as `compile_suites_once` has it.
pub(super) struct LayerInputs<'a> {
    pub(super) opts: &'a RunOptions,
    pub(super) apps: &'a [al_sema::workspace::AppSource],
    pub(super) kinds: &'a [al_sema::AppKind],
    pub(super) manifests: &'a [Manifest],
    pub(super) symbols: &'a BTreeSet<String>,
    pub(super) target: Option<&'a al_version::Target>,
    pub(super) roots: &'a al_sema::check::Roots,
    /// How many apps, from the first, the dependency layers hold.
    pub(super) n_cached: usize,
    pub(super) use_cache: bool,
    pub(super) identities: &'a [Option<[u8; 32]>],
    /// Program app -> caller app.
    pub(super) order: &'a [usize],
    pub(super) suites: &'a [SuiteRun],
    pub(super) lean: bool,
    pub(super) try_skip: bool,
    pub(super) baseline_wanted: bool,
    pub(super) growing: bool,
}

/// The workspace layer of one compile: the key and the entry file, while it is used, and what it did.
pub(super) struct Layer {
    plan: Option<Plan>,
    report: Option<WorkspaceReport>,
}

struct Plan {
    cc: CompileCache,
    key: WorkspaceKey,
    write_now: bool,
}

/// A compile read from the entry.
pub(super) struct Hit {
    pub(super) ws: al_sema::Workspace,
    pub(super) lowered: Lowered,
    pub(super) cache_report: Option<cc::CacheReport>,
    /// Where the scan of the lazily read apps came from, when no dependency layer report says it.
    pub(super) source_index: Option<&'static str>,
    pub(super) layers: Option<cc::Layers>,
    pub(super) decided: Decided,
}

impl Layer {
    /// The layer of a compile with `i`: off (with the reason) when the run did not ask for it or the compile is one the
    /// layer does not cover (decision 7).
    pub(super) fn plan(i: &LayerInputs<'_>) -> Layer {
        let Some(options) = i.opts.workspace_cache.as_ref() else {
            return Layer {
                plan: None,
                report: None,
            };
        };
        let off = |why: String| Layer {
            plan: None,
            report: Some(WorkspaceReport {
                state: WorkspaceState::Off,
                notes: vec![why],
                ..Default::default()
            }),
        };
        let Some(cache) = i.opts.compile_cache.clone() else {
            return off("the compile cache is off".into());
        };
        if let Some(why) = wc::switched_off() {
            return off(why);
        }
        if i.growing {
            return off("a compile that a later pass grows (ADR-063)".into());
        }
        let has_deps = i.kinds.contains(&al_sema::AppKind::Dependency);
        if has_deps && !i.use_cache {
            return off("dependency apps compiled without the dependency layers".into());
        }
        let started = Instant::now();
        let below = if i.use_cache {
            let refs: Vec<&al_sema::workspace::AppSource> = i.apps[..i.n_cached].iter().collect();
            let ids: Vec<Option<[u8; 32]>> = (0..i.n_cached)
                .map(|a| i.identities.get(a).copied().flatten())
                .collect();
            cc::layer_keys(&refs, &i.manifests[..i.n_cached], i.target, &ids, &cache.country)
                .last()
                .map_or([0; 32], |(_, k)| k.digest())
        } else {
            [0; 32]
        };
        let apps = (i.n_cached..i.apps.len())
            .map(|a| {
                let json = options.app_json.get(i.order[a]).map_or("", String::as_str);
                let identity = i.identities.get(a).copied().flatten();
                wc::app_key(&i.apps[a], &i.manifests[a], json, identity)
            })
            .collect();
        let suites: Vec<String> = i.suites.iter().map(|s| format!("{s:?}")).collect();
        // Whether the compile weighs the install baseline on a compile of the selected tests first (`compile_suites_once`):
        // only then does `lean` change the compile.
        let weighed = i.try_skip
            && !cache.require
            && crate::observe::text_looks_pure(i.apps, i.kinds, i.symbols, i.roots.only_tests.as_deref());
        let key = WorkspaceKey {
            format: wc::KEY_FORMAT,
            runtime_build: cc::runtime_build(),
            below,
            apps,
            target: format!("{:?}", i.target),
            symbols: i.symbols.iter().cloned().collect(),
            compile_options: options.compile_options.clone(),
            roots: wc::roots_text(i.roots),
            env: wc::env_values(),
            path: format!(
                "try_skip {} weighed {weighed} lean {} baseline_wanted {} n_cached {} suites {suites:?}",
                i.try_skip,
                i.lean && weighed,
                i.baseline_wanted,
                i.n_cached
            ),
        };
        Layer {
            plan: Some(Plan {
                cc: cache,
                key,
                write_now: options.write_now,
            }),
            report: Some(WorkspaceReport {
                state: WorkspaceState::Miss,
                open: started.elapsed(),
                ..Default::default()
            }),
        }
    }

    /// Why the entry was not used or not written, for the report.
    fn note(&mut self, why: String) {
        if let Some(r) = self.report.as_mut() {
            r.notes.push(why);
        }
    }

    /// An entry that cannot be used: removed, and said on stderr (decision 10). The run compiles fresh.
    fn refuse(&mut self, path: &std::path::Path, why: String) {
        cc::remove_entry(path);
        let why = format!("{} ignored and removed: {why}", path.display());
        eprintln!("workspace cache: {why}");
        self.note(why);
    }

    /// The compile read from the entry of the key, or `None` (a miss: the apps are as they were). A file that cannot be
    /// used is removed, and the reason noted (decision 10).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn read(
        &mut self,
        apps: &mut Vec<al_sema::workspace::AppSource>,
        kinds: &mut Vec<al_sema::AppKind>,
        manifests: &[Manifest],
        symbols: &BTreeSet<String>,
        target: Option<&al_version::Target>,
        n_cached: usize,
        lazy: &[Option<al_sema::LazySource>],
        identities: &[Option<[u8; 32]>],
        catalog: &al_builtins::catalog::Catalog,
    ) -> Option<Hit> {
        let plan = self.plan.as_ref()?;
        if plan.cc.rebuild {
            return None;
        }
        let started = Instant::now();
        let name = plan.key.file_name();
        let (bytes, path) = match cc::read_workspace_entry(&plan.cc.dir, &name) {
            Ok(Some(found)) => found,
            Ok(None) => return None,
            Err(why) => {
                self.note(why);
                return None;
            }
        };
        let size = bytes.len();
        let opened = (|| {
            let (entry, extra): (al_cache::Entry, WorkspaceExtra) =
                al_cache::open_workspace(bytes, plan.key.digest()).map_err(|e| e.to_string())?;
            if let Some((field, _)) = plan.key.field_digests().iter().find(|f| !extra.key.contains(f)) {
                return Err(format!("made for other inputs (the key's {field} differs)"));
            }
            Ok((entry, extra))
        })();
        let (entry, extra) = match opened {
            Ok(o) => o,
            Err(why) => {
                self.refuse(&path, why);
                return None;
            }
        };
        // The dependency layers the compile stands on: those of the key, opened (and built if one is missing).
        // Not for the compile of selected tests free of data, which used none (`Decided::compile_cache_skipped`).
        let (layers, mut report) = if n_cached > 0 && !extra.decided.compile_cache_skipped {
            let cache = &plan.cc;
            let fill = |a: &mut [al_sema::workspace::AppSource]| {
                for (a, l) in a.iter_mut().zip(lazy) {
                    if let Some(l) = l {
                        l.fill(a);
                    }
                }
            };
            match cc::open_or_build_with(
                cache,
                &mut apps[..n_cached],
                &manifests[..n_cached],
                target,
                &identities[..n_cached.min(identities.len())],
                &mut |a| fill(a),
            ) {
                Ok((l, r)) => (Some(l), Some(r)),
                Err(why) => {
                    self.note(why);
                    return None;
                }
            }
        } else {
            (None, None)
        };
        let made = (|| {
            let stack = match &layers {
                Some(l) => {
                    if l.top_digest() != Some(plan.key.below) {
                        return Err("the dependency layers the run opened are not the entry's".to_string());
                    }
                    Some(l.shared_stack()?)
                }
                None => None,
            };
            if extra.base_len as usize != stack.as_ref().map_or(0, |s| s.len()) {
                return Err("the entry stands on another number of dependency objects".into());
            }
            use al_sema::provided::LayerSource;
            use rayon::prelude::*;
            let blocks = (0..entry.objects().len())
                .into_par_iter()
                .map(|slot| Ok((entry.objects()[slot], entry.block_at(slot)?)))
                .collect::<Result<Vec<_>, String>>()?;
            al_sema::lower::stored::restore(extra.lowered, blocks, stack)
        })();
        let mut lowered = match made {
            Ok(l) => l,
            Err(why) => {
                self.refuse(&path, why);
                return None;
            }
        };
        let loaded = load_stored_workspace(
            &plan.cc,
            std::mem::take(apps),
            kinds.clone(),
            manifests,
            symbols,
            lazy,
            identities,
            extra.workspace,
        );
        let (mut ws, source_index) = match loaded {
            Ok(w) => w,
            Err((back, why)) => {
                *apps = back;
                self.refuse(&path, why);
                return None;
            }
        };
        std::mem::take(kinds);
        ws.set_user_name_fields_for(catalog.bc_major());
        lowered.timings.load = started.elapsed();
        if let (Some(r), Some(l)) = (report.as_mut(), layers.as_ref()) {
            r.objects = l.len();
            r.parse_problems = l.parse_problems();
            r.source_index = source_index;
            r.relowered = (0..r.objects.min(lowered.provided.len()))
                .filter(|&i| lowered.dependency[i] && !lowered.provided[i])
                .count();
        }
        if let Some(r) = self.report.as_mut() {
            r.state = WorkspaceState::Read;
            r.file = Some(path);
            r.bytes = size;
            r.open += started.elapsed();
        }
        Some(Hit {
            ws,
            lowered,
            source_index: if report.is_some() { None } else { source_index },
            cache_report: report,
            layers,
            decided: extra.decided,
        })
    }

    /// Whether a fresh compile of this run is stored: the program's apps and sources are taken before the run changes
    /// them (`StoredLowered::apps`).
    pub(super) fn wants_store(&self) -> bool {
        self.plan.is_some() && self.report.as_ref().is_some_and(|r| r.state != WorkspaceState::Read)
    }

    /// Stores the fresh compile `c` (decision 6) when it is one the layer covers (a complete compile of the workspace on
    /// the dependency layers, of a workspace without dependency apps, or of selected tests free of data, which used no
    /// layer), and sets its report. The parts are taken now;
    /// the entry is encoded and written on a thread of its own unless the options ask to write it now.
    pub(super) fn finish(mut self, c: &mut Compiled, pre_apply: Option<(Vec<al_ir::AppInfo>, Vec<al_ir::SourceFile>)>) {
        if let (Some(plan), Some((apps, sources))) = (self.plan.take(), pre_apply) {
            let storable = match (plan.key.below != [0; 32], c.cache_report.is_some(), c.incr.is_some()) {
                (false, false, false) | (true, true, true) => Ok(()),
                // The compile of selected tests free of data, without the dependency layers, is the compile.
                (true, false, false) if c.compile_cache_skipped => Ok(()),
                _ => Err("the dependency layers did not fit; compiled without them".to_string()),
            };
            match storable.and_then(|()| Self::store(&plan, c, apps, sources)) {
                Ok(job) => {
                    let started = Instant::now();
                    if let Some(r) = self.report.as_mut() {
                        r.state = WorkspaceState::Built;
                        r.file = Some(plan.cc.dir.join(plan.key.file_name()));
                    }
                    if plan.write_now {
                        let bytes = job();
                        if let Some(r) = self.report.as_mut() {
                            r.bytes = bytes;
                        }
                    } else {
                        wc::spawn_write(move || {
                            job();
                        });
                    }
                    if let Some(r) = self.report.as_mut() {
                        r.store += started.elapsed();
                    }
                }
                Err(why) => self.note(format!("not stored: {why}")),
            }
        }
        c.workspace_cache = self.report;
    }

    /// The parts of `c` the entry holds, and the job that encodes and writes them (it gives the size written).
    fn store(
        plan: &Plan,
        c: &Compiled,
        apps: Vec<al_ir::AppInfo>,
        sources: Vec<al_ir::SourceFile>,
    ) -> Result<impl FnOnce() -> usize + Send + 'static, String> {
        let (lowered, blocks) = al_sema::lower::stored::store(&c.lowered, apps, sources)?;
        let layers = c.incr.as_ref().map(|i| &i.layers);
        let base_len = layers.map_or(0, |l| l.len());
        let dictionary = layers.and_then(|l| l.base_dictionary()).map(<[u8]>::to_vec);
        let extra = WorkspaceExtra {
            key: plan.key.field_digests(),
            base_len: base_len as u64,
            lowered,
            workspace: c.ws.stored(),
            decided: Decided {
                baseline: c.baseline,
                baseline_note: c.baseline_note.clone(),
                compiles: c.compiles,
                compile_cache_skipped: c.compile_cache_skipped,
            },
        };
        let layer = al_sema::provided::DepLayer {
            base_len,
            objects: blocks.iter().map(|(o, _)| *o).collect(),
            set: al_sema::provided::DepSet {
                facts: vec![Default::default(); blocks.len()],
                heads: blocks
                    .iter()
                    .map(|(_, b)| al_ir::ObjectHead::of(b.object.as_deref().expect("a stored block has its object")))
                    .collect(),
                blocks: blocks.into_iter().map(|(_, b)| b).collect(),
                parse_problems: Vec::new(),
            },
        };
        let (dir, name, digest) = (plan.cc.dir.clone(), plan.key.file_name(), plan.key.digest());
        Ok(move || {
            let bytes = al_cache::build_workspace(&layer, digest, dictionary.as_deref(), &extra);
            let written = cc::write_named(&dir, &name, &bytes);
            if let Err(e) = &written {
                eprintln!("workspace cache: {} not written: {e}", dir.join(&name).display());
            }
            crate::cache_gc::limit_workspace_entries(&dir, false);
            written.map_or(0, |_| bytes.len())
        })
    }
}
