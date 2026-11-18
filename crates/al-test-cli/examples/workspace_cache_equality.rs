//! The equality check of the workspace layer (ADR-022 amendment 2026-11-15, "Equality requirement") on the corpus: the
//! corpus apps compiled with Microsoft's apps as dependency layers, three times in one process over one fresh cache
//! folder: without the workspace layer (the reference), with it (a miss that writes the entry), and with it again (a
//! read). The canonical dumps (`al_testrun::workspace_cache::canonical_dump`) of the reference and the read compile must
//! be identical. Exits 1 when they differ or the third compile did not read the entry.
//!
//! `cargo run --release -p al-test-cli --example workspace_cache_equality -- ROOT MAJOR OUT_DIR [APP_DIR]`
//!
//! The Microsoft apps of the pinned build come from the artifact cache (`al-test-cli --root . --all-versions
//! --fetch-deps`). Run it twice to cover the thread counts: as is, and with `AL_CHECK_THREADS=1 AL_LOWER_THREADS=1`.

use al_testrun::runtime::{self, CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::workspace_cache::{self as wc, WorkspaceCacheOptions, WorkspaceState};
use std::path::PathBuf;
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let root = PathBuf::from(&args[1]);
    let major: u16 = args[2].parse().expect("a BC major");
    let out = PathBuf::from(&args[3]);
    std::fs::create_dir_all(&out).unwrap();
    // A fourth argument: one app folder instead of the corpus apps (the measurement of issue 8143's app).
    let main_app = args
        .get(4)
        .map_or_else(|| root.join("corpus/tests/al-language"), PathBuf::from);
    let publish = if args.len() > 4 {
        Vec::new()
    } else {
        al_test_cli::deps::corpus_publish_dirs(&main_app).unwrap()
    };
    let mut apps: Vec<_> = publish.iter().map(|d| runtime::load_app(d).unwrap()).collect();
    apps.push(runtime::load_app(&main_app).unwrap());
    let mut kinds = vec![al_sema::AppKind::Workspace; apps.len()];
    let mut manifests = vec![runtime::Manifest::default(); apps.len()];
    let mut lazy = vec![None; apps.len()];
    let text = std::fs::read_to_string(root.join("oracle/oracle.toml")).unwrap();
    let oracle = al_test_cli::config::OracleFile::parse(&text).unwrap();
    let version = oracle
        .versions
        .iter()
        .find(|v| v.major == major)
        .expect("the major in oracle.toml");
    let build = version.bc_build.as_deref().expect("a pinned build");
    let provision = al_test_cli::deps::Provision {
        cache_dir: None,
        download: false,
        query: al_deps::build::BuildQuery::new(Some(build), major, None),
        local_artifacts: None,
        country: String::new(),
    };
    let deps = al_test_cli::deps::load_corpus(&main_app, &publish, &[], Some(&provision), &[]).unwrap();
    kinds.extend(deps.apps.iter().map(|_| al_sema::AppKind::Dependency));
    manifests.extend(deps.manifests);
    lazy.extend(deps.lazy);
    apps.extend(deps.apps);
    let symbols = al_version::corpus_symbols(major);
    let target = al_version::Target::for_bc(al_version::BcVersion::parse(build).unwrap());
    let cache = out.join("cache");
    let _ = std::fs::remove_dir_all(&cache);
    let compile = |name: &str, workspace: bool| {
        let opts = RunOptions {
            compile_cache: Some(al_testrun::compile_cache::CompileCache {
                dir: cache.clone(),
                require: false,
                rebuild: false,
                country: String::new(),
            }),
            workspace_cache: workspace.then(|| WorkspaceCacheOptions {
                compile_options: String::new(),
                app_json: Vec::new(),
                write_now: true,
            }),
            lazy: lazy.clone(),
            ..Default::default()
        };
        let started = Instant::now();
        let c = CorpusRuntime::compile_suites(
            apps.clone(),
            kinds.clone(),
            &manifests,
            &symbols,
            Some(&target),
            &[SuiteRun::everything()],
            &opts,
        )
        .unwrap();
        let took = started.elapsed();
        let dump = wc::canonical_dump(&c);
        std::fs::write(out.join(format!("{name}.txt")), &dump).unwrap();
        let report = c.workspace_cache().cloned();
        eprintln!(
            "{name}: compile {:.3} s, workspace layer {:?}, dump {} lines",
            took.as_secs_f64(),
            report
                .as_ref()
                .map(|r| (r.state, r.bytes, r.open, r.store, r.notes.clone())),
            dump.lines().count()
        );
        (dump, report.map(|r| r.state))
    };
    let (fresh, _) = compile("fresh", false);
    let (built, built_state) = compile("built", true);
    let (read, read_state) = compile("read", true);
    let mut ok = true;
    if built_state != Some(WorkspaceState::Built) || read_state != Some(WorkspaceState::Read) {
        eprintln!("FAIL: the states were {built_state:?} and {read_state:?}, not built and read");
        ok = false;
    }
    for (name, dump) in [("built", &built), ("read", &read)] {
        if *dump != fresh {
            let at = fresh
                .lines()
                .zip(dump.lines())
                .position(|(a, b)| a != b)
                .unwrap_or(fresh.lines().count().min(dump.lines().count()));
            eprintln!("FAIL: the {name} dump differs from the fresh one at line {at}");
            ok = false;
        }
    }
    if ok {
        eprintln!(
            "OK: the read compile's dump equals the fresh one ({} bytes)",
            fresh.len()
        );
    }
    std::process::exit(if ok { 0 } else { 1 });
}
