//! `al-runner`: the user command line of AL Runner v3 (ADR-016). Today: `al-runner test`.

mod affected;
mod apps;
mod args;
mod bcapps;
mod capture;
mod coverage;
mod debug;
mod impact;
mod impact_index;
mod output;
mod path_note;
mod recipe;
mod recipe_run;
mod run;
mod serve;
mod sources;
mod sources_report;
mod style;
mod testdata;
mod version;

use args::Command;
use std::process::ExitCode;

#[cfg(all(feature = "mimalloc", not(target_family = "wasm")))]
#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() -> ExitCode {
    // A crash (SIGSEGV and the like) names the test it hit instead of ending the run without a word.
    al_testrun::crash::install();
    // An AL call nests native frames of the bytecode loop; give the runner thread room.
    let worker = std::thread::Builder::new()
        .stack_size(512 << 20)
        .spawn(real_main)
        .expect("spawn runner thread");
    let joined = worker.join();
    if std::env::var_os("AL_EXEC_REPORT").is_some() {
        eprintln!("{}", al_vm::exec_report());
    }
    match joined {
        Ok(code) => code,
        Err(_) => {
            eprintln!("error: the runner thread panicked (a bug)");
            ExitCode::from(2)
        }
    }
}

fn real_main() -> ExitCode {
    let command = match args::parse(std::env::args().skip(1)) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    // Another `al-runner` on PATH (AL Runner v2's dotnet tool): one line on stderr, at most once a day. `--version`
    // lists the copies instead; `serve` talks a protocol on stdout and stderr and gets no note.
    if !matches!(
        command,
        Command::Help | Command::CacheHelp | Command::Version | Command::Serve(_)
    ) {
        path_note::maybe_note();
    }
    match command {
        Command::Help => {
            print!("{}", args::HELP);
            ExitCode::SUCCESS
        }
        Command::CacheHelp => {
            print!("{}", args::cache_help());
            ExitCode::SUCCESS
        }
        Command::Version => {
            println!("{}", version::line());
            // A tool that reads only the first line may close the pipe before the list: not an error.
            use std::io::Write;
            let mut out = std::io::stdout();
            for l in path_note::version_lines() {
                if writeln!(out, "{l}").is_err() {
                    break;
                }
            }
            ExitCode::SUCCESS
        }
        Command::ListAssumptions(dir) => {
            let root = al_deps::cache::cache_root_from_env(dir.as_deref());
            print!("{}", al_testrun::flagged::list(&root));
            ExitCode::SUCCESS
        }
        Command::CacheClear(dir) => {
            let folder =
                al_testrun::compile_cache::CompileCache::under(&al_deps::cache::cache_root_from_env(dir.as_deref()));
            let (files, bytes) = al_testrun::compile_cache::clear(&folder);
            println!(
                "compile cache {}: removed {files} file(s), {:.1} MB",
                folder.display(),
                bytes as f64 / 1e6
            );
            ExitCode::SUCCESS
        }
        Command::CachePrune {
            dir,
            dry_run,
            all_other_builds,
        } => {
            let folder =
                al_testrun::compile_cache::CompileCache::under(&al_deps::cache::cache_root_from_env(dir.as_deref()));
            let mut o = al_testrun::cache_gc::Options::new(al_testrun::cache_gc::this_build());
            o.dry_run = dry_run;
            o.all_other_builds = all_other_builds;
            let r = al_testrun::cache_gc::prune(&folder, &o);
            use al_testrun::cache_gc::Why;
            println!(
                "compile cache {}: {} {} file(s), {:.1} MB",
                folder.display(),
                if dry_run { "would remove" } else { "removed" },
                r.count(),
                r.bytes() as f64 / 1e6
            );
            for (why, label) in [
                (
                    Why::OtherBuild,
                    "another build, unused for 7 days (or any age with --all-other-builds)",
                ),
                (Why::Unused, "unused for 30 days"),
                (Why::Temp, "leftover temporary file"),
            ] {
                let (n, b) = r.count_of(why);
                if n > 0 {
                    println!("  {n} file(s), {:.1} MB: {label}", b as f64 / 1e6);
                }
            }
            ExitCode::SUCCESS
        }
        Command::CacheInfo(dir) => {
            let folder =
                al_testrun::compile_cache::CompileCache::under(&al_deps::cache::cache_root_from_env(dir.as_deref()));
            let build = al_testrun::cache_gc::this_build();
            let i = al_testrun::cache_gc::info(&folder, &build);
            let mb = |b: u64| b as f64 / 1e6;
            println!(
                "compile cache {}: {} entries, {:.1} MB",
                folder.display(),
                i.entries,
                mb(i.bytes)
            );
            println!(
                "  this build ({build}): {} entries, {:.1} MB",
                i.this_build.0,
                mb(i.this_build.1)
            );
            let mut others: Vec<_> = i.other_builds.iter().collect();
            others.sort_by_key(|(_, (_, b))| std::cmp::Reverse(*b));
            let shown = 10;
            for (tag, (n, b)) in others.iter().take(shown) {
                println!("  build {tag}: {n} entries, {:.1} MB", mb(*b));
            }
            if others.len() > shown {
                let (n, b) = others[shown..]
                    .iter()
                    .fold((0, 0), |(n, b), (_, (cn, cb))| (n + cn, b + cb));
                println!("  {} more builds: {n} entries, {:.1} MB", others.len() - shown, mb(b));
            }
            if i.temp.0 > 0 {
                println!("  temporary files: {}, {:.1} MB", i.temp.0, mb(i.temp.1));
            }
            ExitCode::SUCCESS
        }
        Command::Warm(a) => match run::warm(&a) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::from(2)
            }
        },
        Command::Serve(daemon) => match serve::serve(&daemon) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::from(2)
            }
        },
        Command::ImpactIndexCheckStatic(a) => match impact_index::check_static(&a) {
            Ok(code) => ExitCode::from(code),
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::from(2)
            }
        },
        Command::ImpactIndexBuild(a) => match if a.affected_from.is_empty() {
            impact_index::build(&a)
        } else {
            sources::build_index(&a)
        } {
            Ok(code) => ExitCode::from(code),
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::from(2)
            }
        },
        Command::Debug(a, transport) => match debug::run(&a, transport) {
            Ok(code) => ExitCode::from(code),
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::from(2)
            }
        },
        Command::TestMicrosoft(a) => match sources::test_microsoft(&a) {
            Ok(code) => ExitCode::from(code),
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::from(2)
            }
        },
        Command::Test(a) => match if !a.affected_from.is_empty() {
            sources::affected(&a, false)
        } else if a.recipe {
            recipe_run::test(&a)
        } else {
            run::test(&a)
        } {
            Ok(code) => ExitCode::from(code),
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::from(2)
            }
        },
    }
}
