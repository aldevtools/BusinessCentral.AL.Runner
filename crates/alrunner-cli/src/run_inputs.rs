//! The inputs of a run, classified once (al-runner issue 7610). Three keys over the command line options of a run are
//! made here: the options hash of the `--affected` stored result (ADR-050), the key of the impact index (ADR-043) and
//! the options part of the key of `al-runner serve`'s kept compile. The other keys still list their inputs by hand
//! (the kept dependencies of `serve`, the manifest part of `run::warm_key`, `kept_engine_key`); issue 7650 has them.
//!
//! [`RunInputs::of`] destructures `TestArgs` without `..`, as `al_cache::Key::digest` does with its key: a field added
//! to `TestArgs` does not compile until it is put into a class here. The classes:
//!
//! - **outcome**: changes what a test answers. In the stored result's hash and in the index key.
//! - **stored result only**: changes outcomes, but the callers of the impact index set it themselves on one side
//!   (`impact::side_args`, `impact_index::build_run`), so the index key cannot hold it.
//! - **compile**: changes what the compile holds beyond the apps, the symbols and the target (`run::warm_key` has
//!   those). In the key of the kept compile.
//! - **content**: a file or folder whose bytes change outcomes. Hashed by content, named by file name only, so a key
//!   does not depend on where a checkout is (ADR-050 decision 10) and a file that keeps its name but changes its
//!   content is another key (decision 7).
//! - **not an input**: named in an arm that says why.

use crate::args::TestArgs;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Which key a digest is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    /// The options hash of the stored result of `--affected`.
    StoredResult,
    /// The key of the impact index.
    ImpactIndex,
}

/// A file or folder whose content is an input.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Content {
    name: &'static str,
    /// The command line flag, for an error message.
    flag: &'static str,
    /// `None`: the option is not given.
    path: Option<PathBuf>,
    /// A path that cannot be read is an error of the run; else it counts as empty.
    must_exist: bool,
}

/// The classified inputs of one run: (option, value as text) per class, in the order of `TestArgs`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RunInputs {
    outcome: Vec<(&'static str, String)>,
    stored_result_only: Vec<(&'static str, String)>,
    compile: Vec<(&'static str, String)>,
    content: Vec<Content>,
}

/// The file name of a path, which is what a key holds of where a file is.
fn file_name(p: &Option<PathBuf>) -> Option<String> {
    p.as_ref()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned())
}

/// Hashes a file's bytes, or a folder's files by relative path and bytes, in a stable order. A file is streamed: a
/// backup is about 1 GB (about half a second).
fn hash_tree(path: &Path, rel: &str, h: &mut Sha256) -> Result<(), String> {
    let meta = std::fs::metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if meta.is_dir() {
        let mut kids: Vec<PathBuf> = std::fs::read_dir(path)
            .map_err(|e| format!("{}: {e}", path.display()))?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .collect();
        kids.sort();
        for k in kids {
            let name = k
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            hash_tree(&k, &format!("{rel}/{name}"), h)?;
        }
    } else {
        h.update(rel.as_bytes());
        h.update([0]);
        let mut file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
        std::io::copy(&mut file, h).map_err(|e| format!("{}: {e}", path.display()))?;
        h.update([0]);
    }
    Ok(())
}

fn put(h: &mut Sha256, bytes: &[u8]) {
    h.update((bytes.len() as u64).to_le_bytes());
    h.update(bytes);
}

impl RunInputs {
    /// The inputs of the run `a`. Every field of `TestArgs` is named: an input in its class, the rest in an arm that
    /// says why it is none.
    pub fn of(a: &TestArgs) -> RunInputs {
        let mut r = RunInputs::default();
        let text = |v: &dyn std::fmt::Debug| format!("{v:?}");
        let TestArgs {
            // The apps and their dependencies count by content, not as options: the stored result holds each
            // workspace app's manifest and object hashes and each dependency app's content (`affected::Header`); the
            // index key hashes the test apps' source and the index holds the apps of its compile (`Index::apps`);
            // `run::warm_key` hashes both for the kept compile.
            apps: _,
            deps: _,
            package_caches: _,
            // Whether a dependency may be fetched or be missing: the apps that are found count by content.
            no_download: _,
            allow_missing_dependencies: _,
            bc_version,
            // Read through `country()`, which names the default.
            country: _,
            // Where caches are and whether they are read: a cache gives what the compile gives.
            cache_dir: _,
            no_compile_cache: _,
            require_compile_cache: _,
            rebuild_compile_cache: _,
            // The selection. `--affected` refuses one; the index key never held it (a lane's index run passes its
            // codeunits); a kept compile takes a new selection in as an edit (`run::warm_key`).
            codeunits: _,
            filters: _,
            excludes: _,
            exact: _,
            run_selected_only: _,
            single_test_alone: _,
            test_apps: _,
            microsoft_apps: _,
            // Where reports go and what is printed.
            junit: _,
            output_json: _,
            coverage_out: _,
            coverage_json: _,
            profile_flags: _,
            profile: _,
            explain: _,
            top_refusals: _,
            top_failures: _,
            top_assumptions: _,
            refusal_detail: _,
            allow_not_run: _,
            verbose: _,
            list_lanes: _,
            color: _,
            quiet: _,
            timings: _,
            // Observation of the run: statements counted, values captured. The outcomes are the same.
            coverage: _,
            capture: _,
            isolation,
            required_isolation,
            test_runner,
            test_driver,
            defines,
            tdd,
            ignore_permission_mock,
            dev_endpoint_ui,
            no_client,
            allow_write_in_try_functions,
            dependencies_published_global,
            permissions_off,
            strict_order,
            max_replay,
            test_suites,
            // The memory budget: the number of test threads, and in a plain run a hard limit (ADR-066 amendment 10,
            // decision 6) that stops a run which cannot fit, its tests not reached not run. Not a key: the budget also
            // follows the free memory of the machine, which no key holds. A halted run must not leave a stored result
            // (follow-up to issue 7610).
            max_memory: _,
            affected,
            // The stored result itself and how it is used.
            affected_store: _,
            include_failing: _,
            strict_environment: _,
            execute,
            dotnet_sidecar,
            dotnet_assembly_dirs,
            dotnet_allow_roll_forward,
            dotnet_timeout_secs,
            test_timeout_secs,
            skip_after_timeout,
            // Without a usable .NET the run stops: no outcome is stored.
            require_dotnet: _,
            files_dir,
            license,
            environment,
            companies,
            server_url,
            company_name,
            // Whether a backup that is not found stops the run. Whether a company is restored is `wants_test_data()`.
            test_data: _,
            no_test_data,
            test_data_path,
            test_data_company,
            company_package,
            package_path,
            company_demotool,
            demotool_dir,
            company_contoso,
            // A gate: without it the company builds are refused before anything runs.
            experimental_company_builds: _,
            disabled_tests_dir,
            disabled_tests_apps,
            parity,
            // A recipe is several runs, one per lane, each with options of its own.
            recipe: _,
            bcapps_dir: _,
            // An impact run: its two sides are runs of their own (`impact::side_args`).
            impact_of: _,
            impact: _,
            impact_accept: _,
            impact_json: _,
            affected_from: _,
            index_command: _,
            accept_source: _,
            // A matrix run: each cell is a run of its own, with its build and country set. A cell's stored result
            // has a file of its own (`affected::store_path`).
            matrix: _,
            matrix_parallel: _,
            matrix_memory_gb: _,
            matrix_cell: _,
            // A stability run is refused with `--affected` and with an impact run (ADR-058 section 6). A replay is
            // an outcome input: it runs without `--stability` as a normal run with one variant's values.
            stability: _,
            stability_replay,
            stability_max: _,
            stability_allow_flips: _,
            // The run seed is recorded with each stored codeunit and is no part of a key (ADR-061 section 5 and
            // decision 5): a changed seed alone is no reason to run unchanged tests again.
            seed: _,
            run_seed: _,
            // The switch that seeds `Library - Random` from the run seed: the same reason (ADR-061 decision 5).
            seed_library: _,
        } = a;

        // ---- outcome ----
        r.outcome.push(("country", a.country().to_string()));
        r.outcome.push(("bc_version", text(bc_version)));
        r.outcome.push(("isolation", text(isolation)));
        r.outcome.push(("required_isolation", text(required_isolation)));
        // The runner the tests run through (ADR-066).
        r.outcome.push(("test_runner", text(test_runner)));
        // Whose way of starting the tests the run models (ADR-074).
        r.outcome.push(("test_driver", text(test_driver)));
        r.outcome.push(("defines", text(defines)));
        r.outcome.push(("tdd", text(tdd)));
        // Not run becomes a flagged pass (issue 5996).
        r.outcome.push(("ignore_permission_mock", text(ignore_permission_mock)));
        // A UI call no handler takes is answered (issue 3697).
        r.outcome.push(("dev_endpoint_ui", text(dev_endpoint_ui)));
        // Message, Confirm, GuiAllowed, refused dialogs (issue 7564).
        r.outcome.push(("no_client", text(no_client)));
        // A write inside a TryFunction raises TryFunctionWrite or not (issue 7143).
        r.outcome
            .push(("allow_write_in_try_functions", text(allow_write_in_try_functions)));
        // "Published As" of a dependency app (issue 7169).
        r.outcome
            .push(("dependencies_published_global", text(dependencies_published_global)));
        // ADR-062 section 5.
        r.outcome.push(("permissions_off", text(permissions_off)));
        // The serial run is reported (ADR-066 amendments 4, 7 and 12).
        r.outcome.push(("strict_order", text(strict_order)));
        r.outcome.push(("max_replay", text(max_replay)));
        // The values of one stability variant (ADR-058): company name, work date, order and more. Without
        // `--stability` the text holds the values themselves; an id needs `--stability`, which `--affected` and an
        // impact run refuse.
        r.outcome.push(("stability_replay", text(stability_replay)));
        // Which codeunits are order dependent (ADR-066 amendment 6): the file's text counts.
        r.content.push(Content {
            name: "test_suites",
            flag: "--test-suites",
            path: test_suites.clone(),
            must_exist: false,
        });
        // Whether the `DotNet` calls of the tests can run, and what runs them. A sidecar found without these options
        // (the environment, the folder of the executable) is not seen here.
        r.outcome.push(("dotnet_sidecar", text(&file_name(dotnet_sidecar))));
        let dirs: Vec<String> = dotnet_assembly_dirs
            .iter()
            .map(|d| file_name(&Some(d.clone())).unwrap_or_default())
            .collect();
        r.outcome.push(("dotnet_assembly_dirs", text(&dirs)));
        r.outcome
            .push(("dotnet_allow_roll_forward", text(dotnet_allow_roll_forward)));
        r.outcome.push(("dotnet_timeout_secs", text(dotnet_timeout_secs)));
        // A test over the limit is not run.
        r.outcome.push(("test_timeout_secs", text(test_timeout_secs)));
        r.outcome.push(("skip_after_timeout", text(skip_after_timeout)));
        // `File` reads and writes a real folder.
        r.outcome.push(("files_dir", text(&file_name(files_dir))));
        // What License Permission answers (ADR-060).
        r.content.push(Content {
            name: "license",
            flag: "--license",
            path: license.clone(),
            must_exist: true,
        });
        r.outcome.push(("environment", text(environment)));
        // The companies the install code of the apps runs in (ADR-046 decision 2).
        r.outcome.push(("companies", text(companies)));
        r.outcome.push(("server_url", text(server_url)));
        r.outcome.push(("company_name", text(company_name)));
        // The company of the run: restored from a backup, built from a package or by a demo tool, or none.
        r.outcome.push(("wants_test_data", text(&a.wants_test_data())));
        r.outcome.push(("no_test_data", text(no_test_data)));
        r.outcome.push(("test_data_path", text(&file_name(test_data_path))));
        r.content.push(Content {
            name: "test_data_path",
            flag: "--test-data",
            path: test_data_path.clone(),
            must_exist: true,
        });
        r.outcome.push(("test_data_company", text(test_data_company)));
        r.outcome.push(("company_package", text(company_package)));
        r.outcome.push(("package_path", text(&file_name(package_path))));
        r.content.push(Content {
            name: "package_path",
            flag: "--package",
            path: package_path.clone(),
            must_exist: true,
        });
        r.outcome.push(("company_demotool", text(company_demotool)));
        // A BCApps checkout: named, not hashed (its commit is the user's to keep).
        r.outcome.push(("demotool_dir", text(&file_name(demotool_dir))));
        r.outcome.push(("company_contoso", text(company_contoso)));
        // The tests Microsoft turns off are marked or skipped.
        r.content.push(Content {
            name: "disabled_tests_dir",
            flag: "--disabled-tests",
            path: disabled_tests_dir.clone(),
            must_exist: true,
        });

        // ---- stored result only ----
        // Skipped instead of run and marked. Both sides of an impact run set it from `disabled_tests_dir`.
        r.stored_result_only.push(("parity", text(parity)));
        // Which folders of `disabled_tests_dir` are read (the recipe's test apps).
        r.stored_result_only
            .push(("disabled_tests_apps", text(disabled_tests_apps)));

        // ---- compile ----
        // A scratch run compiles the `OnRun` of one codeunit per app instead of the tests.
        r.compile.push(("execute", text(execute)));
        // What compiles (ADR-051 decision 9).
        r.compile.push(("tdd", text(tdd)));
        // A compile that hashed each object (ADR-050).
        r.compile.push(("affected", text(affected)));
        // The runner codeunits are roots of the compile, and the runner decides whether the install baseline runs.
        r.compile.push(("test_runner", text(test_runner)));
        // A company built by a codeunit keeps the install baseline whatever the tests read.
        r.compile.push(("company_package", text(company_package)));
        r.compile.push(("company_demotool", text(company_demotool)));
        r.compile.push(("company_contoso", text(company_contoso)));
        r
    }

    /// The names of the inputs a digest for `purpose` holds, for the test that every one has a miss test.
    #[cfg(test)]
    fn names(&self, purpose: Purpose) -> Vec<&'static str> {
        let mut names: Vec<&'static str> = self.outcome.iter().map(|(n, _)| *n).collect();
        if purpose == Purpose::StoredResult {
            names.extend(self.stored_result_only.iter().map(|(n, _)| *n));
        }
        names.extend(self.content.iter().map(|c| c.name));
        names.sort_unstable();
        names.dedup();
        names
    }

    /// The hash of everything that changes outcomes, for `purpose`. A run whose digest differs from the one a result
    /// was stored with cannot take that result.
    pub fn outcome_digest(&self, purpose: Purpose) -> Result<[u8; 32], String> {
        let mut h = Sha256::new();
        h.update(b"al-runner run inputs 1");
        let only: &[(&'static str, String)] = match purpose {
            Purpose::StoredResult => &self.stored_result_only,
            Purpose::ImpactIndex => &[],
        };
        for (name, value) in self.outcome.iter().chain(only) {
            put(&mut h, name.as_bytes());
            put(&mut h, value.as_bytes());
        }
        // `ALRUNNER_REPORT_UNPINNED_ERRORS=1` (issue 6487): not run becomes failed. Not an option, so not a field.
        put(
            &mut h,
            if crate::unpinned::enabled() {
                b"report_unpinned_errors"
            } else {
                b""
            },
        );
        for c in &self.content {
            put(&mut h, c.name.as_bytes());
            let Some(path) = &c.path else {
                put(&mut h, b"");
                continue;
            };
            let mut tree = Sha256::new();
            match hash_tree(path, "", &mut tree) {
                Ok(()) => {}
                Err(e) if c.must_exist => return Err(format!("{} {e}", c.flag)),
                Err(_) => tree = Sha256::new(),
            }
            put(&mut h, &tree.finalize());
        }
        Ok(h.finalize().into())
    }

    /// The options the compile depends on, as text.
    fn compile_text(&self) -> String {
        self.compile
            .iter()
            .map(|(name, value)| format!("{name}={value}"))
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// The target and the options the compile depends on, for the key of a kept compile (`run::warm_key`).
    pub fn compile_key(&self, target: &al_version::Target) -> String {
        format!("{target:?} {}", self.compile_text())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type Change = fn(&mut TestArgs, &Path);

    /// One change per outcome input, by the name `RunInputs::of` gives it. The path is a file that exists.
    const CHANGES: [(&str, Change); 41] = [
        ("country", |a, _| a.country = Some("de".into())),
        ("bc_version", |a, _| a.bc_version = Some("27.5".into())),
        ("isolation", |a, _| {
            a.isolation = Some(al_testrun::isolation::TestIsolation::Disabled)
        }),
        ("required_isolation", |a, _| a.required_isolation = true),
        ("test_runner", |a, _| {
            a.test_runner = Some(al_testrun::runtime::TestRunner::Builtin)
        }),
        ("test_driver", |a, _| {
            a.test_driver = Some(al_testrun::runtime::TestDriver::BcContainerHelper)
        }),
        ("defines", |a, _| a.defines = vec!["CLEAN27".into()]),
        ("tdd", |a, _| a.tdd = true),
        ("ignore_permission_mock", |a, _| a.ignore_permission_mock = true),
        ("dev_endpoint_ui", |a, _| a.dev_endpoint_ui = true),
        ("no_client", |a, _| a.no_client = true),
        ("allow_write_in_try_functions", |a, _| {
            a.allow_write_in_try_functions = true
        }),
        ("dependencies_published_global", |a, _| {
            a.dependencies_published_global = true
        }),
        ("permissions_off", |a, _| a.permissions_off = true),
        ("strict_order", |a, _| a.strict_order = true),
        ("max_replay", |a, _| a.max_replay = Some(3)),
        ("stability_replay", |a, _| {
            a.stability_replay = Some("company.name=Other Company".into())
        }),
        ("test_suites", |a, f| a.test_suites = Some(f.to_path_buf())),
        ("dotnet_sidecar", |a, f| a.dotnet_sidecar = Some(f.to_path_buf())),
        ("dotnet_assembly_dirs", |a, f| {
            a.dotnet_assembly_dirs = vec![f.to_path_buf()]
        }),
        ("dotnet_allow_roll_forward", |a, _| a.dotnet_allow_roll_forward = true),
        ("dotnet_timeout_secs", |a, _| a.dotnet_timeout_secs = Some(5)),
        ("test_timeout_secs", |a, _| a.test_timeout_secs = Some(5)),
        ("skip_after_timeout", |a, _| a.skip_after_timeout = true),
        ("files_dir", |a, f| a.files_dir = Some(f.to_path_buf())),
        ("license", |a, f| a.license = Some(f.to_path_buf())),
        ("environment", |a, _| a.environment = Some(al_host::Environment::OnPrem)),
        ("companies", |a, _| a.companies = Some(al_host::Companies::All)),
        ("server_url", |a, _| a.server_url = Some("https://bc.example/".into())),
        ("company_name", |a, _| a.company_name = Some("Lane".into())),
        // With `no_test_data`: one change moves both.
        ("wants_test_data", |a, _| a.no_test_data = true),
        ("no_test_data", |a, _| a.no_test_data = true),
        ("test_data_path", |a, f| a.test_data_path = Some(f.to_path_buf())),
        ("test_data_company", |a, _| a.test_data_company = Some("CRONUS".into())),
        ("company_package", |a, _| a.company_package = true),
        ("package_path", |a, f| a.package_path = Some(f.to_path_buf())),
        ("company_demotool", |a, _| a.company_demotool = true),
        ("demotool_dir", |a, f| a.demotool_dir = Some(f.to_path_buf())),
        ("company_contoso", |a, _| {
            a.company_contoso = Some(crate::args::ContosoData::Setup)
        }),
        ("disabled_tests_dir", |a, f| {
            a.disabled_tests_dir = Some(f.to_path_buf())
        }),
        ("parity", |a, _| a.parity = true),
    ];

    /// The change of the one stored result only input.
    const STORED_ONLY: [(&str, Change); 1] = [("disabled_tests_apps", |a, _| {
        a.disabled_tests_apps = vec!["Tests-ERM".into()]
    })];

    fn scratch_file(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("alrunner-run-inputs-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("file.txt");
        std::fs::write(&file, "text").unwrap();
        file
    }

    fn digest(a: &TestArgs, purpose: Purpose) -> [u8; 32] {
        RunInputs::of(a).outcome_digest(purpose).unwrap()
    }

    fn all_names(purpose: Purpose) -> Vec<&'static str> {
        RunInputs::of(&TestArgs::default()).names(purpose)
    }

    #[test]
    fn every_outcome_input_has_a_miss_test() {
        let mut tested: Vec<&str> = CHANGES.iter().chain(&STORED_ONLY).map(|(n, _)| *n).collect();
        tested.sort_unstable();
        assert_eq!(tested, all_names(Purpose::StoredResult));
    }

    #[test]
    fn every_outcome_input_changes_the_digest_of_the_stored_result() {
        let file = scratch_file("stored");
        let plain = digest(&TestArgs::default(), Purpose::StoredResult);
        assert_eq!(plain, digest(&TestArgs::default(), Purpose::StoredResult));
        for (name, change) in CHANGES.iter().chain(&STORED_ONLY) {
            let mut a = TestArgs::default();
            change(&mut a, &file);
            assert_ne!(digest(&a, Purpose::StoredResult), plain, "{name}");
        }
    }

    #[test]
    fn every_outcome_input_changes_the_key_of_the_impact_index_but_what_its_callers_set() {
        let file = scratch_file("index");
        let plain = digest(&TestArgs::default(), Purpose::ImpactIndex);
        let stored_only = ["parity", "disabled_tests_apps"];
        for (name, change) in CHANGES.iter().chain(&STORED_ONLY) {
            let mut a = TestArgs::default();
            change(&mut a, &file);
            let same = digest(&a, Purpose::ImpactIndex) == plain;
            assert_eq!(same, stored_only.contains(name), "{name}");
        }
        assert!(!all_names(Purpose::ImpactIndex).iter().any(|n| stored_only.contains(n)));
    }

    #[test]
    fn a_file_counts_by_its_content_and_its_name_not_by_where_it_is() {
        let (one, two) = (scratch_file("where-1"), scratch_file("where-2"));
        let with = |f: &Path| {
            let a = TestArgs {
                test_data_path: Some(f.to_path_buf()),
                license: Some(f.to_path_buf()),
                ..Default::default()
            };
            digest(&a, Purpose::StoredResult)
        };
        assert_eq!(with(&one), with(&two), "the same file in another folder");
        std::fs::write(&two, "other text").unwrap();
        assert_ne!(with(&one), with(&two), "other content under the same name");
    }

    #[test]
    fn a_content_file_that_is_missing_is_an_error_but_for_the_test_suites() {
        let missing = std::env::temp_dir().join("alrunner-run-inputs-no-such-file");
        let a = TestArgs {
            license: Some(missing.clone()),
            ..Default::default()
        };
        let e = RunInputs::of(&a).outcome_digest(Purpose::StoredResult).unwrap_err();
        assert!(e.starts_with("--license "), "{e}");
        let a = TestArgs {
            test_suites: Some(missing),
            ..Default::default()
        };
        assert!(RunInputs::of(&a).outcome_digest(Purpose::StoredResult).is_ok());
    }

    #[test]
    fn the_compile_text_holds_the_runner_and_the_modes_that_change_the_compile() {
        let plain = RunInputs::of(&TestArgs::default()).compile_text();
        type Set = fn(&mut TestArgs);
        let changes: [(&str, Set); 7] = [
            ("execute", |a| a.execute = true),
            ("tdd", |a| a.tdd = true),
            ("affected", |a| a.affected = true),
            ("test_runner", |a| {
                a.test_runner = Some(al_testrun::runtime::TestRunner::Builtin)
            }),
            ("company_package", |a| a.company_package = true),
            ("company_demotool", |a| a.company_demotool = true),
            ("company_contoso", |a| {
                a.company_contoso = Some(crate::args::ContosoData::All)
            }),
        ];
        let inputs = RunInputs::of(&TestArgs::default());
        let names: Vec<&str> = inputs.compile.iter().map(|(n, _)| *n).collect();
        assert_eq!(names, changes.iter().map(|(n, _)| *n).collect::<Vec<_>>());
        for (name, change) in changes {
            let mut a = TestArgs::default();
            change(&mut a);
            assert_ne!(RunInputs::of(&a).compile_text(), plain, "{name}");
        }
        // An option that changes outcomes only does not cost a compile.
        let a = TestArgs {
            no_client: true,
            ..Default::default()
        };
        assert_eq!(RunInputs::of(&a).compile_text(), plain);
    }
}
