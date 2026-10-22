//! Performance budget: cold compile of the pinned corpus (issue #1, docs/architecture.md). The budget is in instructions
//! per KiB of corpus source (see below); the wall time is printed for information.
//!
//! "Compile" is everything between the app folders on disk and the lowered
//! program: reading the `.al` files, lex, preprocess, parse, workspace load,
//! member tables, type check, and lowering. It excludes executing the tests.
//!
//! Ignored by default because wall-clock budgets are only meaningful in an
//! optimised build. CI runs it explicitly:
//!
//! ```text
//! cargo test --release -p al-test-cli --test perf_budget -- --ignored
//! ```

#[path = "support/meter.rs"]
mod meter;

use al_test_cli::runtime;
use meter::Meter;
use std::path::{Path, PathBuf};
use std::time::Instant;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// The corpus app and the corpus apps it depends on (the internals fixture, the cross-app fixtures), dependencies first.
fn corpus_app_dirs() -> Vec<std::path::PathBuf> {
    let main = root().join("corpus/tests/al-language");
    let mut dirs = al_test_cli::deps::corpus_source_dirs(&main).unwrap();
    dirs.push(main);
    dirs
}

/// The bytes of corpus source the compile reads: every `.al` file of those apps. Computed once, outside the measured
/// region.
fn corpus_bytes() -> u64 {
    corpus_app_dirs()
        .iter()
        .map(|a| {
            let app = runtime::load_app(a).unwrap();
            app.files.iter().map(|(_, text)| text.len() as u64).sum::<u64>()
        })
        .sum()
}

fn compile_corpus() -> usize {
    let apps: Vec<_> = corpus_app_dirs()
        .iter()
        .map(|d| runtime::load_app(d).unwrap())
        .collect();
    let symbols = al_version::corpus_symbols(27);
    let (_ws, lowered) = al_sema::lower::compile(apps, &symbols);
    lowered.program.objects.len()
}

/// The cold compile is budgeted in instructions per KiB of corpus source, not in seconds. On a shared CI runner the same code
/// measured 828 and then 955 ms (and 2.4 and then 3.0 s of CPU) between two runs a day apart; the instruction count of
/// every commit is the same to within 1 % on every machine, and per KiB of source it does not break when the corpus grows (per object it did: the same compiler measured 1.961 million
/// per object on the old pin and 2.218 on the pin of corpus bump 12, whose objects are bigger; per KiB it is 607 346 and 629 042, +3.5 %).
/// The limit is the value of the day plus 5 %. Where the kernel refuses the counter the test falls back to the CPU time of
/// the process per KiB, with a limit three times as wide, and says so. The wall time is printed for information only.
///
/// ADR-033 step 1 added the link step (every lowered object is made symbolic and linked again, on worker threads): the
/// day's value went from 1.936 to 1.999 million (+3.3 %), so the limit went from 1 990 000 to 2 060 000. The wall time
/// did not grow (the walks run on all cores). Lowering that writes the symbolic form directly saves the first of the two
/// walks.
///
/// ADR-036 step 0 (the side table of expression types, `lower/tys.rs`) added 2.3 % and ADR-033 step 1 3.3 %. Cheaper
/// type copies (`Ty` names are shared, `Arc<str>`), Fx-hashed name maps, and no copy of the candidate list of a call took
/// the day's value from 2.106 to 1.958 million, so the limit was back at the original 1 990 000 per object (1.958 million that day).
///
/// The unit changed from the object to the KiB of source at corpus bump 12 (14 523 191 bytes, 4021 objects; 8.92 billion
/// instructions = 629 042 per KiB). The limit was that value plus 5 %, 660 000.
///
/// Corpus bump 14 (18.9 MB of source): 662 399 per KiB. Main had grown to 655 403 on the old corpus (the night's language
/// and Microsoft-test work, +4.2 %) and the bump's files add 1.1 %. Same rule, the day's value plus 5 %: 695 000. Compare
/// values only on one compiler build.
const LIMIT_INSTRUCTIONS_PER_KIB: u64 = 695_000;
const LIMIT_CPU_NS_PER_KIB: u64 = 695_000;
const RUNS: usize = 3;

#[test]
#[ignore = "perf budget: run with `cargo test --release -p al-test-cli --test perf_budget -- --ignored`"]
fn cold_compile_of_the_pinned_corpus_stays_within_its_instruction_budget() {
    if cfg!(debug_assertions) {
        panic!("perf budgets are only meaningful in --release; this is a debug build");
    }
    assert!(
        root().join("corpus/tests/al-language/app.json").exists(),
        "corpus submodule not checked out: run `git submodule update --init`"
    );
    // The warm-up (file cache, allocator, the thread pools) is not counted.
    let bytes = corpus_bytes();
    let mut objects = compile_corpus();
    // Open before the compile starts its threads: they inherit the counter, so the whole compile is counted.
    let meter = Meter::new(true);
    let counting = matches!(meter, Meter::Instructions(_));
    let mut best = u64::MAX;
    let mut samples = Vec::new();
    for _ in 0..RUNS {
        let t = Instant::now();
        let (n, cost) = meter.measure(compile_corpus);
        objects = n;
        samples.push(format!("{} ({:.0} ms)", cost, t.elapsed().as_secs_f64() * 1e3));
        best = best.min(cost);
    }
    let per_object = best / objects as u64;
    // Budgeted per KiB of source: the objects of a corpus grow without the compiler getting slower.
    let per_kib = best * 1024 / bytes;
    let limit = if counting {
        LIMIT_INSTRUCTIONS_PER_KIB
    } else {
        LIMIT_CPU_NS_PER_KIB
    };
    meter::print_mode(counting);
    eprintln!(
        "cold compile: {objects} objects, {} ({}), least of {RUNS} = {best}, {per_object} per object, {per_kib} per KiB of source ({bytes} bytes), limit {limit} per KiB; samples: {}",
        meter.name(),
        if counting { "all threads" } else { "process" },
        samples.join(", ")
    );
    assert!(
        objects > 1500,
        "compiled only {objects} objects: the corpus was not loaded"
    );
    meter::enforce(
        counting,
        limit != 0 && per_kib > limit,
        &format!(
            "the cold compile used {per_kib} {} per KiB of corpus source, over the limit of {limit}",
            meter.name()
        ),
    );
}

/// A full corpus run with Microsoft's apps (Base Application and the rest of `.alpackages`, the
/// test libraries from the artifact cache). Not the owner's target of one second: a guard against
/// the big regressions found by profiling it (a run took 14 to 27 s before; see the commit log of
/// branch perf-1). It is on the runtime phases of `--timings` (compile, install baseline, run tests), not on
/// the whole process: "load apps and dependencies" reads the artifact cache and the network, and took 9.9 s of a
/// 14.7 s run on a runner while it takes about 1 s otherwise (issue 16). The load time is printed and
/// not budgeted.
///
/// The budget is in instructions of the whole process tree (the meter is opened before the run starts, so the worker
/// threads and a .NET helper count), not in seconds: the seconds moved with the load of the machine (12 s was
/// exceeded at 21.5 s on a machine with a load of 25 and the same code), the instructions do not. Where the kernel refuses
/// the counter, the CPU limit below applies. The seconds are printed for information. A run over the limit is repeated,
/// at most `WITH_APPS_RUNS` times in all, and the best counts: a real regression is over the limit every time.
///
/// The limit grows with the corpus (owner, 2026-10-19): a fixed part for the Microsoft apps plus a part per KiB of the
/// corpus source the run compiles (`run_source_bytes`: the corpus app, the internals fixture and the suites of the BC 29
/// leg). A fixed limit broke at every corpus bump (bump 26 alone added 7.3 G of real work).
///
/// Why two parts, measured on 2026-10-19 with one binary (main 4047556e) and only the corpus pin and `oracle/` changed,
/// BC 29.0.54011.56262 at every pin (instructions, best of 2; KiB of run source):
///
/// | pin (bump)    | KiB    | instructions | per KiB | (minus fixed part) per KiB |
/// |---------------|--------|--------------|---------|----------------------------|
/// | 4e6d142 (14)  | 18 738 | 145.9 G      | 7 786 k | 3 577 k                    |
/// | 81244ca (20)  | 19 989 | 157.4 G      | 7 874 k | 3 927 k                    |
/// | 4b6435b (23)  | 20 644 | 160.1 G      | 7 754 k | 3 933 k                    |
/// | 92aeaa7 (24)  | 21 213 | 162.0 G      | 7 637 k | 3 918 k                    |
/// | be87ec9 (25)  | 21 498 | 163.1 G      | 7 586 k | 3 917 k                    |
/// | db082fc (26)  | 22 263 | 168.5 G      | 7 570 k | 4 026 k                    |
///
/// The fixed part is measured, not fitted: the same run with every `.al` file of the corpus app and the suites deleted
/// (the fixture kept, no test runs) costs 78.88 G (79.27 G in a second run): loading, parsing and the demand compile of
/// the Microsoft apps. The install baseline (about 0.5 s of the full run) does not run there, so it counts in the corpus
/// part. A line fitted through the six points puts the fixed part anywhere from 23 to 69 G, depending on the size
/// measure and on whether bump 14 is in. Per KiB of all source, the cost fell 3.9 % from bump 20 to bump 26, so a limit
/// per KiB alone would gain slack at every bump.
/// With the fixed part taken off, bumps 20 to 25 cost 3 917 k to 3 933 k per KiB (0.4 % apart); bump 26 is 2.6 % above
/// (its new tests do more per KiB), bump 14 9 % below. Bytes of all compiled corpus apps fit a little better than the two
/// apps of the cold-compile budget (bump 14 to 26 drifts 6.3 % against 7.2 %).
///
/// The limit is the value of the day plus 5 %, on both parts: on 2026-10-19 (corpus db082fc, 22 797 007 bytes =
/// 22 262.7 KiB) the run took 168.52 G (and 169.23 G), so the corpus part is (168.52 - 78.88) G / 22 262.7 KiB =
/// 4 026 k per KiB. Fixed: 78.88 G x 1.05 = 82.8 G. Per KiB: 4 026 k x 1.05 = 4 230 k. The limit at today's size is
/// 82.8 G + 4 230 k x 22 262.7 = 177.0 G = 168.52 G + 5.0 %. Re-measure the fixed part (the empty-corpus run above) when
/// the BC build of the leg moves or the load of the Microsoft apps changes. Compare values only on one compiler build.
const WITH_APPS_LIMIT_FIXED_INSTRUCTIONS: u64 = 82_800_000_000;
const WITH_APPS_LIMIT_INSTRUCTIONS_PER_KIB: u64 = 4_230_000;
/// The BC major of the budgeted run, and of the suites `run_source_bytes` counts.
const WITH_APPS_MAJOR: &str = "29";

/// The instruction limit of the full run for `bytes` of corpus source.
fn with_apps_limit_instructions(bytes: u64) -> u64 {
    WITH_APPS_LIMIT_FIXED_INSTRUCTIONS + WITH_APPS_LIMIT_INSTRUCTIONS_PER_KIB * bytes / 1024
}

/// The bytes of corpus source the full run compiles: every `.al` file of the corpus app, the corpus apps it depends on
/// and the suites of the `WITH_APPS_MAJOR` leg of `oracle/oracle.toml`.
fn run_source_bytes() -> u64 {
    let root = root();
    let oracle = std::fs::read_to_string(root.join("oracle/oracle.toml")).unwrap();
    let oracle = al_test_cli::config::OracleFile::parse(&oracle).unwrap();
    let leg = oracle.select(Some(WITH_APPS_MAJOR)).unwrap();
    let mut apps = al_test_cli::deps::corpus_source_dirs(&root.join(&oracle.corpus_app)).unwrap();
    apps.push(root.join(&oracle.corpus_app));
    apps.extend(leg.suites.iter().map(|s| root.join(&s.app)));
    apps.iter()
        .map(|a| {
            let app = runtime::load_app(a).unwrap();
            app.files.iter().map(|(_, text)| text.len() as u64).sum::<u64>()
        })
        .sum()
}
/// Where the kernel gives no instruction counter (a GitHub runner), the cost is the CPU time of the child process (user and
/// system, `getrusage(RUSAGE_CHILDREN)`: the helper processes it waited for count too), not the wall time: the seconds of
/// the runtime phases moved with the load of a shared runner (13.6 s against 12 s on main with the same code), CPU time
/// moves much less, but still varies between runners: main CI measured 25.8 s (53fcf0cc) and 34.7 s (4342a744). The
/// limit catches a run several times slower or one that never ends, not a few percent: the instruction limit does that.
/// Owner decision 2026-10-20: in this fallback mode an excess only warns (a GitHub Actions warning line); it does not fail.
const WITH_APPS_LIMIT_CPU_NS: u64 = 45_000_000_000;
const WITH_APPS_RUNS: usize = 3;
/// No single test codeunit may take longer: one codeunit that walked 2.9 million computed Date
/// rows took 5 s of the run before.
const SLOWEST_CODEUNIT_SECS: f64 = 2.0;
/// The longest start of one .NET sidecar process (ADR-014), reported by `--timings` apart from the
/// codeunit times. A start is about 0.05 s on a dev box and 1 s on a cold CI runner; one that
/// takes this long is a broken start, not a slow machine.
const SIDECAR_START_SECS: f64 = 10.0;

/// Compile, install baseline and run tests of the `timings:` line of `--timings`, in seconds: the runtime,
/// without the load of the apps and dependencies (artifact cache and network).
fn runtime_phases_secs(err: &str) -> Option<f64> {
    let line = err.lines().find(|l| l.starts_with("timings: "))?;
    let part = |label: &str| -> Option<f64> {
        let at = line.find(label)? + label.len();
        line[at..].split(' ').next()?.parse().ok()
    };
    Some(part("compile ")? + part("install baseline ")? + part("run tests ")?)
}

/// One entry of the `slowest test codeunits:` line of `--timings`.
#[derive(Debug, PartialEq)]
struct CodeunitLine {
    name: String,
    took: f64,
    sidecar_start: f64,
}

/// The number that ends right before `marker` in `text`: `number_before("a 1.5 s at most", " s at most")`.
fn number_before(text: &str, marker: &str) -> Option<f64> {
    let head = &text[..text.find(marker)?];
    head.rsplit(' ').next()?.parse().ok()
}

/// `69934 Name 7.39 s (sidecar start 7.30 s); 60130 Other 0.50 s`: the name, the time and the part of
/// it spent starting a sidecar (0 when the entry has none).
fn parse_slowest(line: &str) -> Vec<CodeunitLine> {
    line.split("; ")
        .filter_map(|entry| {
            let (entry, start) = match entry.rfind(" (sidecar start ") {
                Some(i) => (&entry[..i], number_before(&entry[i..], " s)")?),
                None => (entry, 0.0),
            };
            let took = entry.trim().strip_suffix(" s")?;
            let (name, took) = took.rsplit_once(' ')?;
            Some(CodeunitLine {
                name: name.to_string(),
                took: took.parse().ok()?,
                sidecar_start: start,
            })
        })
        .collect()
}

#[test]
fn the_runtime_phases_leave_out_the_load_of_the_apps() {
    // Run 38598436035 of CI: 14.7 s in all, 9.9 s of it loading.
    let line = "timings: BC 29.0: load apps and dependencies 9.90 s, compile 3.36 s, install baseline 0.30 s, run tests 1.04 s, total 14.62 s";
    let secs = runtime_phases_secs(line).unwrap();
    assert!((secs - 4.70).abs() < 1e-9, "{secs}");
    assert_eq!(runtime_phases_secs("nothing"), None);
}

#[test]
fn the_full_run_limit_is_a_fixed_part_plus_a_part_per_kib_of_corpus_source() {
    // Corpus db082fc: 22 797 007 bytes, measured 168.52 G; the limit is that plus 5 %.
    let limit = with_apps_limit_instructions(22_797_007);
    assert_eq!(limit, 82_800_000_000 + 4_230_000 * 22_797_007 / 1024);
    let today = 168_523_144_041f64;
    assert!((limit as f64 / today - 1.05).abs() < 0.001, "{limit}");
    // The fixed part does not grow with the corpus.
    assert_eq!(with_apps_limit_instructions(0), WITH_APPS_LIMIT_FIXED_INSTRUCTIONS);
}

#[test]
fn the_slowest_line_is_read_with_and_without_a_sidecar_start() {
    let l =
        parse_slowest("69934 TPF Media Filled Tests 7.39 s (sidecar start 7.30 s); 60130 Test Media Png Import 0.07 s");
    assert_eq!(
        l,
        vec![
            CodeunitLine {
                name: "69934 TPF Media Filled Tests".into(),
                took: 7.39,
                sidecar_start: 7.30
            },
            CodeunitLine {
                name: "60130 Test Media Png Import".into(),
                took: 0.07,
                sidecar_start: 0.0
            },
        ]
    );
    assert_eq!(
        number_before(
            "2 .NET sidecar process(es) started: starting 0.12 s in total, 0.06 s at most; 35 request(s) 0.03 s",
            " s at most"
        ),
        Some(0.06)
    );
}

#[test]
#[ignore = "perf budget: run with `cargo test --release -p al-test-cli --test perf_budget -- --ignored`"]
fn a_full_corpus_run_with_the_microsoft_apps_stays_within_its_budget() {
    if cfg!(debug_assertions) {
        panic!("perf budgets are only meaningful in --release; this is a debug build");
    }
    // Outside the measured region, like the cold compile's `corpus_bytes`.
    let bytes = run_source_bytes();
    let limit_instructions = with_apps_limit_instructions(bytes);
    let meter = Meter::new(true);
    let counting = matches!(meter, Meter::Instructions(_));
    let mut runs: Vec<(f64, f64, String)> = Vec::new();
    let mut costs: Vec<u64> = Vec::new();
    for _ in 0..WITH_APPS_RUNS {
        let started = Instant::now();
        let children_before = meter::children_cpu_ns();
        let (out, instructions) = meter.measure(|| {
            std::process::Command::new(env!("CARGO_BIN_EXE_al-test-cli"))
                .arg("--root")
                .arg(root())
                .args(["--bc-version", WITH_APPS_MAJOR, "--no-ratchet", "--timings"])
                .output()
                .unwrap()
        });
        // With the counter: instructions. Without it: the CPU time of the child.
        let cost = if counting {
            instructions
        } else {
            meter::children_cpu_ns() - children_before
        };
        costs.push(cost);
        let wall = started.elapsed().as_secs_f64();
        let err = String::from_utf8_lossy(&out.stderr).into_owned();
        assert!(out.status.success(), "{err}");
        let runtime = runtime_phases_secs(&err).expect("a `timings:` line in --timings");
        runs.push((runtime, wall, err));
        let within = cost
            <= if counting {
                limit_instructions
            } else {
                WITH_APPS_LIMIT_CPU_NS
            };
        if within {
            break;
        }
    }
    let best = costs.iter().copied().min().unwrap();
    let (unit, limit) = if counting {
        ("instructions", limit_instructions)
    } else {
        ("ns of child CPU time", WITH_APPS_LIMIT_CPU_NS)
    };
    meter::print_mode(counting);
    eprintln!(
        "full run with the apps: {best} {unit} (limit {limit}; with the counter {WITH_APPS_LIMIT_FIXED_INSTRUCTIONS} + \
         {WITH_APPS_LIMIT_INSTRUCTIONS_PER_KIB} per KiB x {bytes} bytes of corpus source = {limit_instructions}), samples: {costs:?}"
    );
    let (runtime, wall, err) = runs
        .iter()
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(r, w, e)| (*r, *w, e.clone()))
        .unwrap();
    eprintln!(
        "full run with the apps: runtime phases {runtime:.1} s (whole process {wall:.1} s), samples: {}\n{}",
        runs.iter()
            .map(|(r, w, _)| format!("{r:.1} s ({w:.1} s)"))
            .collect::<Vec<_>>()
            .join(", "),
        err.lines()
            .filter(|l| l.starts_with("timings")
                || l.starts_with("compile:")
                || l.starts_with("slowest")
                || l.starts_with("dotnet:"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert!(
        err.contains("dependency: Base Application"),
        "the Microsoft apps were not loaded:\n{err}"
    );
    // The limit is on the runtime. Starting a .NET helper process is not the runtime and varies
    // with the machine (a cold runner took 1 to 7 s for it), so it is taken off a codeunit's time
    // and checked on its own.
    let line = err
        .lines()
        .find_map(|l| l.strip_prefix("slowest test codeunits: "))
        .expect("a slowest codeunit in --timings");
    let slowest = parse_slowest(line);
    assert!(!slowest.is_empty(), "no codeunit time in: {line}");
    let (name, runtime_secs, start_secs) = slowest
        .iter()
        .map(|c| (c.name.as_str(), c.took - c.sidecar_start, c.sidecar_start))
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .unwrap();
    assert!(
        runtime_secs <= SLOWEST_CODEUNIT_SECS,
        "test codeunit {name} took {runtime_secs:.1} s without its sidecar start ({start_secs:.1} s), \
         over {SLOWEST_CODEUNIT_SECS} s:\n{err}"
    );
    if let Some(d) = err.lines().find_map(|l| l.strip_prefix("dotnet: ")) {
        let max = number_before(d, " s at most").expect("the longest sidecar start in the dotnet line");
        assert!(
            max <= SIDECAR_START_SECS,
            "a .NET sidecar took {max:.1} s to start, over {SIDECAR_START_SECS} s:\n{err}"
        );
    }
    meter::enforce(
        counting,
        best > limit,
        &format!(
            "the run used {best} {unit} at best of {WITH_APPS_RUNS}, over the limit of {limit} \
             (runtime phases {runtime:.1} s, for information):\n{err}"
        ),
    );
}
