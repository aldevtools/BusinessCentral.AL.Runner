//! A company built once for every test app of the same dependency apps (`--company-data`, Microsoft's Legacy lane).
//!
//! Building the DemoTool's company takes about two minutes, and `al-runner test-microsoft` runs one test app after the
//! other, each over the same dependency apps. BCApps' CI builds the company once and runs every Legacy test app against
//! it (build/scripts/ImportTestDataInBcContainer.ps1:907-923). Here the company is built in the program of the dependency
//! apps alone, before any app under test is installed, and that start is the same for every app under test that takes
//! no part in building it ([`apps_under_test_take_no_part`]). So the result is kept (`company_key`, the compile cache's
//! folder) and every run of such an app starts from its own copy of it, read from bytes: no state passes from one app to
//! the next.
//!
//! An app under test takes part when building the company could run its code or see its shape: an extension object
//! other than a permission set extension (fields, triggers and values of the objects the company code writes), install code (BC would run it before the
//! company is built), or an event subscriber that fires without a bind. Such an app gets its company built in its own
//! program, with it installed, as before. What an app that takes no part still changes is only what the platform's
//! metadata tables list (its objects in `AllObj`, `Field`, its permission sets, ...): the company built without it does not see it there.
//! That is the one difference to a build with it installed, and it is the same for every app, as in BCApps' CI, where
//! one company serves every app. `ALRUNNER_COMPANY_REUSE_VERIFY=1` builds both and says whether they differ.
//!
//! `--no-company-reuse` (or `ALRUNNER_NO_COMPANY_REUSE=1`) builds the company again for every app, in the same program
//! of the dependency apps: the outcomes are the same, only slower.

use super::*;
use crate::baseline::BaselineRun;
use crate::company_data::CompanyData;

/// The environment variable that turns the reuse off (`RunOptions::no_company_reuse`).
pub const NO_REUSE_ENV: &str = "ALRUNNER_NO_COMPANY_REUSE";
/// The environment variable that builds the company both ways and compares them.
pub const VERIFY_ENV: &str = "ALRUNNER_COMPANY_REUSE_VERIFY";

/// `Ok` when no app under test of `ws` takes part in building the company (see the module comment), else why one does.
/// The dependency apps all take part, also those the recipe installs after the company is built
/// (`CompanyData::installed_first`): their install code runs after the company data codeunit in the program of the
/// dependency apps as it does in the run's own.
pub(crate) fn apps_under_test_take_no_part(ws: &al_sema::Workspace, _company: &CompanyData) -> Result<(), String> {
    for (i, app) in ws.apps.iter().enumerate() {
        if ws.kinds.get(i) == Some(&al_sema::AppKind::Dependency) {
            continue;
        }
        let declared = ws.objects.iter().filter(|o| o.app == i).count();
        let mut scanned = 0;
        for (path, text) in &app.files {
            for o in al_syntax::scan::scan(text, ws.symbols_of(i)).objects {
                scanned += 1;
                let what = || format!("{} \"{}\" of {} ({path})", o.kind, o.name, app.name);
                // A permission set extension changes only what the metadata tables list (the permissions of a set),
                // as a permission set of the app does: no code of it runs and no table changes shape.
                if o.kind.ends_with("extension") && o.kind != "permissionsetextension" {
                    return Err(format!("{} extends an object", what()));
                }
                if o.install {
                    return Err(format!("{} is install code", what()));
                }
                if !o.subscribers.is_empty() && !o.manual_subscribers {
                    return Err(format!("{} subscribes to events without a bind", what()));
                }
            }
        }
        // The scan must have seen every object the compile declares, or a subscriber could hide in one it missed.
        if scanned != declared {
            return Err(format!(
                "the scan of {} found {scanned} objects, the compile {declared}",
                app.name
            ));
        }
    }
    Ok(())
}

/// The company built in the program of the dependency apps, in that program's object indices, with what is needed to
/// move it into the program of a run (`place`).
pub(crate) struct Shared {
    pub run: BaselineRun,
    /// Per object index of the program it was built in: (app id, kind, lower-case name).
    pub objects: Vec<(String, String, String)>,
    /// Per object index: the field ids of the table, `None` for an object that is no table.
    pub tables: Vec<Option<Vec<u32>>>,
    /// The install code read an app's resource files (`Engine::resources_read`).
    pub resources_read: bool,
}

/// The first bytes of a kept company: what it is and which layout it has.
const MAGIC: &[u8; 8] = b"ALCOMP\0\x01";

/// The bytes of `shared` and the instant the run that built it began, or `None` when its baseline has none.
pub(crate) fn encode(shared: &Shared, started_at: i64) -> Option<Vec<u8>> {
    use al_builtins::value_codec::*;
    let mut out = MAGIC.to_vec();
    let run = crate::baseline_cache::encode(&shared.run, started_at)?;
    put_bytes(&mut out, &run);
    put_u8(&mut out, shared.resources_read as u8);
    put_u32(&mut out, shared.objects.len() as u32);
    for ((app, kind, name), table) in shared.objects.iter().zip(&shared.tables) {
        put_str(&mut out, app);
        put_str(&mut out, kind);
        put_str(&mut out, name);
        match table {
            None => put_u32(&mut out, u32::MAX),
            Some(fields) => {
                put_u32(&mut out, fields.len() as u32);
                for f in fields {
                    put_u32(&mut out, *f);
                }
            }
        }
    }
    Some(out)
}

/// What [`encode`] wrote, or `None` for anything else.
pub(crate) fn decode(bytes: &[u8]) -> Option<(Shared, i64)> {
    use al_builtins::value_codec::Reader;
    let mut r = Reader::new(bytes.strip_prefix(&MAGIC[..])?);
    let (run, started_at) = crate::baseline_cache::decode(r.bytes()?)?;
    let resources_read = r.u8()? != 0;
    let n = r.u32()? as usize;
    let mut objects = Vec::with_capacity(n.min(1 << 20));
    let mut tables = Vec::with_capacity(n.min(1 << 20));
    for _ in 0..n {
        objects.push((r.str()?, r.str()?, r.str()?));
        tables.push(match r.u32()? {
            u32::MAX => None,
            k => Some((0..k).map(|_| r.u32()).collect::<Option<Vec<_>>>()?),
        });
    }
    if !r.is_empty() {
        return None;
    }
    Some((
        Shared {
            run,
            objects,
            tables,
            resources_read,
        },
        started_at,
    ))
}

fn identity(program: &al_ir::Program, o: &al_ir::Object) -> (String, String, String) {
    (
        program
            .apps
            .get(o.app as usize)
            .map(|a| a.id.clone())
            .unwrap_or_default(),
        o.kind.label().to_string(),
        o.name.to_lowercase(),
    )
}

/// Builds the company in the program of the dependency apps of `compiled` alone (compiled again, from the compile
/// cache when the run has one). `trace`: what the install code raised and touched (`BaselineRun::trace`).
pub(crate) fn build(
    compiled: &Compiled,
    target: Option<&al_version::Target>,
    dotnet: &al_vm::dotnet::DotNetConfig,
    opts: &RunOptions,
    trace: bool,
) -> Result<Shared, String> {
    let ws = &compiled.ws;
    let deps: Vec<usize> = (0..ws.apps.len())
        .filter(|&i| ws.kinds.get(i) == Some(&al_sema::AppKind::Dependency))
        .collect();
    let apps: Vec<al_sema::workspace::AppSource> = deps.iter().map(|&i| ws.apps[i].clone()).collect();
    let manifests: Vec<Manifest> = deps
        .iter()
        .map(|&i| compiled.manifests.get(i).cloned().unwrap_or_default())
        .collect();
    let s_opts = RunOptions {
        compile_cache: opts.compile_cache.clone(),
        lazy: deps.iter().map(|&i| compiled.lazy.get(i).cloned().flatten()).collect(),
        company_data: opts.company_data.clone(),
        test_data: opts.test_data.clone(),
        quiet_baseline: true,
        ..Default::default()
    };
    let s = CorpusRuntime::compile_suites(
        apps,
        vec![al_sema::AppKind::Dependency; deps.len()],
        &manifests,
        &Default::default(),
        target,
        &[],
        &s_opts,
    )?;
    let lowered = &s.lowered;
    let blocked: Vec<Vec<Option<String>>> = lowered
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let mut engine = Engine::with_config(lowered.program.clone(), blocked, target.cloned(), dotnet.clone());
    global_triggers_find_unavailable(&mut engine, lowered);
    if let Some(e) = Arc::get_mut(&mut engine) {
        e.test_data = opts.test_data.clone();
    }
    let run = crate::baseline::run_traced(&engine, lowered, trace, opts.company_data.as_ref())
        .ok_or("the dependency apps have no install code")?;
    let program = &lowered.program;
    let objects = program.objects.iter().map(|o| identity(program, o)).collect();
    let tables = (0..program.objects.len())
        .map(|oi| {
            engine
                .tables
                .get(oi)
                .and_then(|t| t.as_ref())
                .map(|t| t.fields.iter().map(|f| f.id).collect())
        })
        .collect();
    let resources_read = engine.resources_read.load(std::sync::atomic::Ordering::Relaxed);
    drop(engine);
    retire(s);
    Ok(Shared {
        run,
        objects,
        tables,
        resources_read,
    })
}

/// `shared` in the object indices of `lowered` (the run's program): every object it was built with is found there by
/// app, kind and name, and every table has the same fields there. Else why not, and the company is built in the run's
/// own program.
pub(crate) fn place(shared: &Shared, lowered: &Lowered, engine: &Engine) -> Result<BaselineRun, String> {
    let program = &lowered.program;
    let here: std::collections::HashMap<(String, String, String), usize> = program
        .objects
        .iter()
        .enumerate()
        .map(|(oi, o)| (identity(program, o), oi))
        .collect();
    let mut map: Vec<Option<u32>> = Vec::with_capacity(shared.objects.len());
    for (id, fields) in shared.objects.iter().zip(&shared.tables) {
        let at = here.get(id).copied();
        if let (Some(fields), Some(oi)) = (fields, at) {
            let there: Option<Vec<u32>> = engine
                .tables
                .get(oi)
                .and_then(|t| t.as_ref())
                .map(|t| t.fields.iter().map(|f| f.id).collect());
            if there.as_ref() != Some(fields) {
                return Err(format!(
                    "table {} of app {} has other fields in the run's program",
                    id.2, id.0
                ));
            }
        }
        map.push(at.map(|oi| oi as u32));
    }
    let r = &shared.run;
    let baseline = r
        .baseline
        .remapped(&map)
        .ok_or("the company names an object the run's program does not have")?;
    let unseeded = r
        .unseeded
        .iter()
        .map(|&t| map.get(t as usize).copied().flatten())
        .collect::<Option<_>>()
        .ok_or("an unseeded table the run's program does not have")?;
    Ok(BaselineRun {
        baseline,
        unseeded,
        filled: r.filled,
        notes: r.notes.clone(),
        ran: r.ran,
        total: r.total,
        restored: r.restored.clone(),
        trace: r.trace.clone(),
    })
}

/// The key under which the company of this run is kept: the world of `baseline_key` without the apps under test and
/// their test selection, which take no part in it. `None`: it is not kept (see `world_key`).
pub(crate) fn company_key(
    report: Option<&crate::compile_cache::CacheReport>,
    target: Option<&al_version::Target>,
    dotnet: &al_vm::dotnet::DotNetConfig,
    opts: &RunOptions,
) -> Option<String> {
    let mut key = crate::baseline_cache::KeyBuilder::new();
    key.text("kind", "company built by the dependency apps alone");
    super::corpus::world_key(&mut key, report?, target, dotnet, opts)?;
    Some(key.finish())
}

/// The two baselines hold the same data and say the same about the install roots; else the first difference.
pub(crate) fn same(a: &BaselineRun, b: &BaselineRun) -> Result<(), String> {
    if a.notes != b.notes {
        let first = a.notes.iter().zip(&b.notes).find(|(x, y)| x != y);
        return Err(match first {
            Some((x, y)) => format!("the notes differ: '{x}' / '{y}'"),
            None => format!("{} notes / {} notes", a.notes.len(), b.notes.len()),
        });
    }
    if (a.ran, a.total, a.filled) != (b.ran, b.total, b.filled) {
        return Err(format!(
            "the roots differ: {}/{} roots, {} tables / {}/{} roots, {} tables",
            a.ran, a.total, a.filled, b.ran, b.total, b.filled
        ));
    }
    if a.unseeded != b.unseeded {
        return Err("the unseeded tables differ".into());
    }
    let (Some(x), Some(y)) = (a.baseline.to_bytes(), b.baseline.to_bytes()) else {
        return Err("a baseline has no bytes".into());
    };
    if x != y {
        let tables = |b: &al_vm::Baseline| {
            b.exported_tables()
                .unwrap_or_default()
                .into_iter()
                .map(|t| {
                    let mut bytes = Vec::new();
                    for (k, r) in &t.rows {
                        al_builtins::value_codec::encode_row(k, &mut bytes);
                        al_builtins::value_codec::encode_row(r, &mut bytes);
                    }
                    ((t.company, t.object), bytes)
                })
                .collect::<std::collections::BTreeMap<_, _>>()
        };
        let (ta, tb) = (tables(&a.baseline), tables(&b.baseline));
        let differ: Vec<String> = ta
            .keys()
            .chain(tb.keys())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .filter(|k| ta.get(k) != tb.get(k))
            .take(10)
            .map(|(c, o)| format!("{c}/{o}"))
            .collect();
        return Err(format!(
            "the data differs (tables by company/object index: {})",
            differ.join(", ")
        ));
    }
    Ok(())
}

/// The company a run of `compiled` starts from when no app under test takes part in building it: read from the
/// cache, or built in the program of the dependency apps (and kept), then moved into the run's program. Also what
/// `Phases::company` says and whether it was read. When it cannot be moved (or not built that way), it is built in the
/// run's own program, and the note says why.
#[allow(clippy::too_many_arguments)]
pub(crate) fn start(
    compiled: &Compiled,
    engine: &Arc<Engine>,
    report: Option<&crate::compile_cache::CacheReport>,
    target: Option<&al_version::Target>,
    dotnet: &al_vm::dotnet::DotNetConfig,
    opts: &RunOptions,
) -> (Option<BaselineRun>, String, bool) {
    let lowered = &compiled.lowered;
    // The impact index and an impact run's selection need what the install code raised and touched: it runs.
    let trace = opts.impact_trace || opts.narrow.is_some();
    let reuse = !opts.no_company_reuse && std::env::var_os(NO_REUSE_ENV).is_none() && !trace;
    let kept = (if reuse {
        company_key(report, target, dotnet, opts)
    } else {
        None
    })
    .zip(opts.compile_cache.as_ref())
    .filter(|(_, cc)| !cc.rebuild);
    let own = |why: String| {
        let run = crate::baseline::run_traced(engine, lowered, trace, opts.company_data.as_ref());
        (run, format!("built with the apps under test: {why}"), false)
    };
    // As for a kept install baseline: the rows carry the instant the run that built them began, so the run clock
    // must start there.
    let read = kept
        .as_ref()
        .and_then(|(key, cc)| crate::baseline_cache::read_with(&cc.dir, key, decode))
        .filter(|(_, at)| {
            crate::host::SystemClock::begin_at(*at)
                || (crate::host::SystemClock::started() && crate::host::SystemClock::run_start() == *at)
        });
    let (shared, from_cache) = match read {
        Some((shared, _)) => (shared, true),
        None => match build(compiled, target, dotnet, opts, trace) {
            Ok(shared) => {
                if let Some((key, cc)) = &kept {
                    // The clock has started by now if the install code read it; if not, no row carries it.
                    if let Some(bytes) = encode(&shared, crate::host::SystemClock::run_start()) {
                        crate::baseline_cache::write_bytes(&cc.dir, key, &bytes);
                    }
                }
                (shared, false)
            }
            Err(why) => return own(format!("it was not built for the dependency apps alone: {why}")),
        },
    };
    let placed = match place(&shared, lowered, engine) {
        Ok(run) => run,
        Err(why) => return own(why),
    };
    if shared.resources_read {
        engine.resources_read.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    if std::env::var_os(VERIFY_ENV).is_some() {
        match crate::baseline::run_traced(engine, lowered, trace, opts.company_data.as_ref()) {
            Some(with_apps) => match same(&placed, &with_apps) {
                Ok(()) => eprintln!("company: verify: the company built with the apps under test is the same"),
                Err(d) => eprintln!("company: verify: the company built with the apps under test differs: {d}"),
            },
            None => eprintln!("company: verify: no baseline with the apps under test"),
        }
    }
    let how = if from_cache {
        "read from the company an earlier run of the same dependency apps built"
    } else if reuse {
        "built for the dependency apps alone, and kept"
    } else {
        "built for the dependency apps alone (reuse is off)"
    };
    (Some(placed), how.to_string(), from_cache)
}
