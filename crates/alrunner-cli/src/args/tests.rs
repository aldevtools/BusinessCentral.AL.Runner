// The unit tests of `args.rs`, a `#[cfg(test)]` module; the version-literal scan skips everything from the marker on.
// #[cfg(test)]
use super::*;

fn p(s: &str) -> Result<Command, String> {
    parse(s.split_whitespace().map(String::from))
}

#[test]
fn skip_after_timeout_is_off_by_default_and_a_flag_of_test_and_serve() {
    assert!(!test_args("test A").skip_after_timeout);
    assert!(test_args("test A --skip-after-timeout").skip_after_timeout);
    assert!(test_args("test A --affected --skip-after-timeout").skip_after_timeout);
    match p("serve --protocol 2 --skip-after-timeout").unwrap() {
        Command::Serve(d) => assert_eq!(d, ["--skip-after-timeout"]),
        other => panic!("{other:?}"),
    }
}

fn test_args(s: &str) -> TestArgs {
    match p(s).unwrap() {
        Command::Test(a) => *a,
        other => panic!("{other:?}"),
    }
}

/// Issue 5996: `--ignore-permission-mock` is off unless given, and `serve` takes it only per request.
#[test]
fn ignore_permission_mock_is_opt_in() {
    assert!(!test_args("test A").ignore_permission_mock);
    assert!(test_args("test A --ignore-permission-mock").ignore_permission_mock);
}

/// ADR-058 amendment 1, decision 7: the value after `--stability` is a preset name or an existing regular file
/// (any name), never a folder; a missing `.json` file is an error, not the next argument.
#[test]
fn stability_takes_an_existing_file_of_any_name_but_never_a_folder() {
    let d = tempfile::tempdir().unwrap();
    let cfg = d.path().join("stability.cfg");
    std::fs::write(&cfg, r#"{"format":1,"dimensions":{"order.methods":["reverse"]}}"#).unwrap();
    let a = test_args(&format!("test A --stability {}", cfg.display()));
    assert_eq!(a.stability.unwrap().source, cfg.display().to_string());
    assert_eq!(a.apps, vec![PathBuf::from("A")]);
    let app = d.path().join("app");
    std::fs::create_dir(&app).unwrap();
    let a = test_args(&format!("test --stability {}", app.display()));
    assert_eq!(a.stability.unwrap().source, "all");
    assert_eq!(a.apps, vec![app.clone()]);
    let missing = d.path().join("missing.json");
    let e = p(&format!("test A --stability {}", missing.display())).unwrap_err();
    assert!(e.contains("config file not found"), "{e}");
}

/// ADR-066 amendment 4: `--strict-order` is off unless given.
#[test]
fn strict_order_is_opt_in() {
    assert!(!test_args("test A").strict_order);
    assert!(test_args("test A --strict-order").strict_order);
}

/// ADR-066 amendment 7 (owner, 2026-11-08): no replay cap unless `--max-replay N` is given.
#[test]
fn max_replay_is_opt_in() {
    assert_eq!(test_args("test A").max_replay, None);
    assert_eq!(test_args("test A --max-replay 50").max_replay, Some(50));
}

/// Issue 7143: `--allow-write-in-try-functions` is off unless given.
#[test]
fn allow_write_in_try_functions_is_opt_in() {
    assert!(!test_args("test A").allow_write_in_try_functions);
    assert!(test_args("test A --allow-write-in-try-functions").allow_write_in_try_functions);
}

/// Issue 7169: `--dependencies-published-global` is off unless given.
#[test]
fn dependencies_published_global_is_opt_in() {
    assert!(!test_args("test A").dependencies_published_global);
    assert!(test_args("test A --dependencies-published-global").dependencies_published_global);
    assert!(parse(["serve".to_string(), "--dependencies-published-global".to_string()]).is_err());
}

/// Issue 3697: `--dev-endpoint-ui` is off unless given, and `serve` takes it only per request.
#[test]
fn dev_endpoint_ui_is_opt_in() {
    assert!(!test_args("test A").dev_endpoint_ui);
    assert!(test_args("test A --dev-endpoint-ui").dev_endpoint_ui);
    assert!(parse(["serve".to_string(), "--dev-endpoint-ui".to_string()]).is_err());
}

/// ADR-062 section 5: `--permissions on|off`, `on` (the stages that are on by default) unless given; any other
/// value is an error.
#[test]
fn permissions_takes_on_or_off() {
    assert!(!test_args("test A").permissions_off);
    assert!(!test_args("test A --permissions on").permissions_off);
    assert!(test_args("test A --permissions off").permissions_off);
    assert!(test_args("test A --permissions OFF").permissions_off);
    let e = p("test A --permissions some").unwrap_err();
    assert!(
        e.contains("--permissions") && e.contains("on") && e.contains("off"),
        "{e}"
    );
}

#[test]
fn test_microsoft_needs_no_app_and_turns_the_recipe_on() {
    match p("test-microsoft --bc-version 29.0 --app Tests-ERM --microsoft-app tests-bank --junit r.xml").unwrap() {
        Command::TestMicrosoft(a) => {
            assert!(a.apps.is_empty() && a.recipe);
            assert_eq!(a.microsoft_apps, vec!["Tests-ERM", "tests-bank"]);
            assert_eq!(a.bc_version.as_deref(), Some("29.0"));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn test_microsoft_refuses_what_the_recipe_decides() {
    for flag in [
        "--recipe microsoft",
        "--disabled-tests d",
        "--parity",
        "--company-data package",
        "--test-data",
        "--no-test-data",
        "--test-data-company X",
        "--explain T",
    ] {
        let e = p(&format!("test-microsoft {flag}")).unwrap_err();
        assert!(
            e.contains("test-microsoft") && e.contains("cannot be combined"),
            "{flag}: {e}"
        );
    }
    match p("test-microsoft MyApp --impact static").unwrap() {
        Command::TestMicrosoft(a) => {
            assert_eq!(a.apps, vec![PathBuf::from("MyApp")]);
            assert_eq!(a.affected_from, vec!["microsoft"]);
            assert!(!a.recipe, "the recipe is the source's business, not the extension's");
        }
        other => panic!("{other:?}"),
    }
    // Without an extension there is nothing to measure.
    assert!(p("test-microsoft --impact static").unwrap_err().contains("--impact"));
    for flag in ["--no-compile-cache", "--coverage"] {
        let e = p(&format!("test-microsoft MyApp {flag}")).unwrap_err();
        assert!(e.contains(flag.split(' ').next().unwrap()), "{flag}: {e}");
    }
    for flag in ["--output-json r.json", "--impact-json d.json", "--impact-accept a.json"] {
        assert!(p(&format!("test-microsoft MyApp {flag}")).is_ok(), "{flag}");
    }
    assert!(p("test-microsoft --affected-from microsoft").is_err());
    // `--affected` selects from the stored result of a `test` run; test-microsoft has none (ADR-050 decision 1).
    assert!(
        p("test Ext --affected --affected-from microsoft")
            .unwrap_err()
            .contains("--affected cannot be combined with --affected-from")
    );
    assert!(
        p("test-microsoft --affected")
            .unwrap_err()
            .contains("--affected cannot be combined")
    );
    assert!(
        p("test-microsoft --include-failing")
            .unwrap_err()
            .contains("needs --affected")
    );
    assert!(p("test-microsoft --output-json").unwrap_err().contains("FILE"));
    match p("test-microsoft --list-lanes --app Tests-ERM").unwrap() {
        Command::TestMicrosoft(a) => assert!(a.list_lanes),
        other => panic!("{other:?}"),
    }
    for extra in ["MyApp", "--junit r.xml", "--output-json r.json"] {
        let e = p(&format!("test-microsoft --list-lanes {extra}")).unwrap_err();
        assert!(e.contains("--list-lanes"), "{extra}: {e}");
    }
    assert!(p("test --list-lanes").unwrap_err().contains("unknown option"));
    assert!(p("test-microsoft --output-json r.json").is_ok());
    // `--app` is the short form in this command only; `--microsoft-app` needs the command too.
    assert!(p("test A --app Tests-ERM").unwrap_err().contains("unknown option"));
    assert!(
        p("test A --microsoft-app Tests-ERM")
            .unwrap_err()
            .contains("test-microsoft")
    );
}

#[test]
fn affected_from_microsoft_parses_and_refuses_what_it_cannot_be_combined_with() {
    let a = test_args("test Ext --affected-from microsoft --microsoft-app Tests-ERM --impact static");
    assert_eq!(a.affected_from, vec!["microsoft"]);
    assert_eq!(a.microsoft_apps, vec!["Tests-ERM"]);
    assert_eq!(
        test_args("test Ext --affected-from Microsoft").affected_from,
        vec!["microsoft"]
    );
    assert!(p("test Ext --affected-from microsoft --impact all --junit r.xml").is_ok());
    // An ISV test app is a path; one that does not exist, the same twice or an APP as a source exit 2.
    let e = p("test Ext --affected-from ./no-such-vendor-tests").unwrap_err();
    assert!(e.contains("./no-such-vendor-tests") && e.contains("no such"), "{e}");
    let dir = tempfile::tempdir().unwrap();
    let v = dir.path().join("vendor");
    std::fs::create_dir_all(&v).unwrap();
    let v = v.to_string_lossy();
    assert_eq!(
        test_args(&format!("test Ext --affected-from {v} --affected-from microsoft")).affected_from,
        vec![v.to_string(), "microsoft".into()]
    );
    let e = p(&format!("test Ext --affected-from {v} --affected-from {v}/")).unwrap_err();
    assert!(e.contains("twice"), "{e}");
    let e = p(&format!("test {v} --affected-from {v}")).unwrap_err();
    assert!(e.contains("also an APP"), "{e}");
    let e = p(&format!("test Ext --affected-from {v} --microsoft-app Tests-ERM")).unwrap_err();
    assert!(e.contains("--affected-from microsoft"), "{e}");
    let e = p("test Ext --affected-from microsoft --affected-from MICROSOFT").unwrap_err();
    assert!(e.contains("microsoft") && e.contains("twice"), "{e}");
    for flag in [
        "--recipe microsoft",
        "--no-compile-cache",
        "--explain T",
        "--coverage",
        "--impact-of E",
        "--output-json",
        "--tdd",
    ] {
        let e = p(&format!("test Ext --affected-from microsoft {flag}")).unwrap_err();
        assert!(
            e.contains("--affected-from") && e.contains(flag.split(' ').next().unwrap()),
            "{flag}: {e}"
        );
    }
    let e = p("test Ext --microsoft-app Tests-ERM").unwrap_err();
    assert!(e.contains("--affected-from microsoft"), "{e}");
    // Build step 3: the files of a run with sources.
    for flag in ["--output-json r.json", "--impact-json d.json", "--impact-accept a.json"] {
        assert!(
            p(&format!("test Ext --affected-from microsoft {flag}")).is_ok(),
            "{flag}"
        );
    }
}

#[test]
fn impact_index_build_from_microsoft_takes_no_app() {
    match p("impact-index build --from microsoft --microsoft-app Tests-ERM --bc-version 29.0").unwrap() {
        Command::ImpactIndexBuild(a) => {
            assert!(a.apps.is_empty());
            assert_eq!(a.affected_from, vec!["microsoft"]);
            assert_eq!(a.microsoft_apps, vec!["Tests-ERM"]);
        }
        other => panic!("{other:?}"),
    }
    assert!(p("impact-index build --from microsoft A").unwrap_err().contains("APP"));
    let e = p("impact-index build --from ./x").unwrap_err();
    assert!(e.contains("microsoft") && e.contains("impact-index build ./x"), "{e}");
    assert!(p("impact-index check-static --from microsoft").is_err());
    assert!(p("impact-index build A --microsoft-app Tests-ERM").is_err());
}

#[test]
fn warm_takes_a_backup_or_a_build_and_nothing_else() {
    match p("warm x.bak --test-data-company TP --cache-dir c").unwrap() {
        Command::Warm(a) => {
            assert_eq!(a.test_data_path, Some(PathBuf::from("x.bak")));
            assert_eq!(a.test_data_company.as_deref(), Some("TP"));
            assert_eq!(a.cache_dir, Some(PathBuf::from("c")));
        }
        other => panic!("{other:?}"),
    }
    assert!(matches!(
        p("warm --bc-version 27.5.46862.48827 --no-download").unwrap(),
        Command::Warm(_)
    ));
    assert!(p("warm").unwrap_err().contains("--bc-version"));
    assert!(p("warm App").unwrap_err().contains("unknown"));
}

#[test]
fn cache_clear_takes_a_cache_dir_and_nothing_else() {
    assert_eq!(p("cache clear").unwrap(), Command::CacheClear(None));
    assert_eq!(
        p("cache clear --cache-dir /c").unwrap(),
        Command::CacheClear(Some(PathBuf::from("/c")))
    );
    assert!(p("cache").unwrap_err().contains("clear"));
    assert_eq!(
        p("cache prune --dry-run --all-other-builds --cache-dir /c").unwrap(),
        Command::CachePrune {
            dir: Some(PathBuf::from("/c")),
            dry_run: true,
            all_other_builds: true
        }
    );
    assert_eq!(p("cache info").unwrap(), Command::CacheInfo(None));
    assert_eq!(p("cache --help").unwrap(), Command::CacheHelp);
    assert_eq!(p("cache prune --help").unwrap(), Command::CacheHelp);
    assert!(cache_help().starts_with("COMPILE CACHE") && cache_help().contains("al-runner cache clear"));
    assert!(p("cache info --dry-run").is_err(), "--dry-run belongs to prune");
    assert!(p("cache wipe").unwrap_err().contains("wipe"));
    assert!(p("cache clear --all").unwrap_err().contains("--all"));
}

/// Issue 5618: `-v` and `-q` are listed in `--help` and must not be read as app paths.
#[test]
fn short_verbose_and_quiet_flags_are_flags_not_apps() {
    let a = test_args("test -v App Tests");
    assert!(a.verbose);
    assert_eq!(a.apps, vec![PathBuf::from("App"), PathBuf::from("Tests")]);
    let a = test_args("test App -q Tests");
    assert!(a.quiet);
    assert_eq!(a.apps, vec![PathBuf::from("App"), PathBuf::from("Tests")]);
    assert!(p("test -v -q App").unwrap_err().contains("cannot be combined"));
}

#[test]
fn rebuild_compile_cache_is_a_test_flag_that_excludes_the_other_two() {
    assert!(test_args("test A --rebuild-compile-cache").rebuild_compile_cache);
    assert!(!test_args("test A").rebuild_compile_cache);
    assert!(p("test A --rebuild-compile-cache --no-compile-cache").is_err());
    assert!(p("test A --rebuild-compile-cache --require-compile-cache").is_err());
}

#[test]
fn recipe_flags_parse_and_check_each_other() {
    assert!(!test_args("test A").recipe);
    let a = test_args("test A --recipe microsoft --bcapps /b");
    assert!(a.recipe);
    assert_eq!(a.bcapps_dir.as_deref(), Some(std::path::Path::new("/b")));
    assert!(p("test A --recipe other").unwrap_err().contains("microsoft"));
    assert!(p("test A --bcapps /b").unwrap_err().contains("--recipe"));
    for flag in [
        "--company-data package",
        "--no-test-data",
        "--disabled-tests d",
        "--test-data-company X",
    ] {
        let e = p(&format!("test A --recipe microsoft {flag}")).unwrap_err();
        assert!(e.contains("decides the company"), "{flag}: {e}");
    }
    assert!(p("test A --recipe microsoft --codeunit 5 --junit x.xml").is_ok());
}

#[test]
fn company_data_and_parity_flags_parse_and_check_each_other() {
    assert!(!test_args("test A").company_package);
    assert!(test_args("test A --company-data package").company_package);
    assert!(!test_args("test A --company-data bak").company_package);
    let a = test_args("test A --package p.rapidstart");
    assert!(a.company_package && a.package_path == Some(PathBuf::from("p.rapidstart")));
    assert!(p("test A --company-data tape").unwrap_err().contains("tape"));
    assert!(p("test A --company-data package --no-test-data").is_err());
    let a = test_args("test A --disabled-tests d --parity");
    assert!(a.parity && a.disabled_tests_dir == Some(PathBuf::from("d")));
    assert!(p("test A --parity").unwrap_err().contains("--disabled-tests"));
    assert!(test_args("test A --company-data package").test_data_asked_for());
    for needle in ["--company-data", "--package", "--disabled-tests", "--parity"] {
        assert!(HELP.contains(needle), "{needle}");
    }
}

#[test]
fn company_data_contoso_builds_the_company_without_a_backup() {
    let a = test_args("test A --experimental-company-builds --company-data contoso-setup");
    assert_eq!(a.company_contoso, Some(ContosoData::Setup));
    assert!(!a.wants_test_data() && !a.company_demotool);
    assert_eq!(
        test_args("test A --experimental-company-builds --company-data contoso-all").company_contoso,
        Some(ContosoData::All)
    );
    assert!(
        test_args("test A --experimental-company-builds --company-data contoso-all --company-data bak")
            .company_contoso
            .is_none()
    );
    assert!(
        test_args("test A --experimental-company-builds --company-data contoso-all --company-data demotool")
            .company_contoso
            .is_none()
    );
    for bad in [
        "test A --experimental-company-builds --company-data contoso-setup --test-data x.bak",
        "test A --experimental-company-builds --company-data contoso-all --no-test-data",
        "test A --experimental-company-builds --company-data contoso-all --country us",
        "test A --experimental-company-builds --company-data contoso-all --recipe microsoft",
    ] {
        assert!(p(bad).is_err(), "{bad}");
    }
}

#[test]
fn company_data_demotool_builds_the_company_without_a_backup() {
    let a = test_args("test A --experimental-company-builds --company-data demotool");
    assert!(a.company_demotool && !a.company_package);
    assert!(!a.wants_test_data(), "no backup is restored under the DemoTool company");
    let a = test_args("test A --experimental-company-builds --demotool d");
    assert!(a.company_demotool && a.demotool_dir == Some(PathBuf::from("d")));
    assert!(
        !test_args("test A --experimental-company-builds --company-data demotool --company-data bak").company_demotool
    );
    for bad in [
        "test A --experimental-company-builds --company-data demotool --test-data x.bak",
        "test A --experimental-company-builds --company-data demotool --no-test-data",
        "test A --experimental-company-builds --company-data demotool --country us",
        "test A --experimental-company-builds --company-data demotool --recipe microsoft",
    ] {
        assert!(p(bad).is_err(), "{bad}");
    }
    assert!(HELP.contains("--company-data bak|package|demotool") && HELP.contains("--demotool DIR"));
}

// Owner decision 2026-10-23: the company builds of Microsoft's lanes (DemoTool, Contoso) are experimental and off by
// default; without the opt-in they are refused with a reason that names it.
#[test]
fn company_builds_are_refused_without_the_experimental_opt_in() {
    for bad in [
        "test A --company-data demotool",
        "test A --demotool d",
        "test A --company-data contoso-setup",
        "test A --company-data contoso-all",
    ] {
        let e = p(bad).unwrap_err();
        assert!(
            e.contains("experimental") && e.contains("off by default") && e.contains("--experimental-company-builds"),
            "{bad}: {e}"
        );
    }
    // The other company options need no opt-in.
    assert!(test_args("test A --company-data package").company_package);
    assert!(!test_args("test A --company-data bak").experimental_company_builds);
    assert!(test_args("test A --experimental-company-builds").experimental_company_builds);
    // test-microsoft takes the opt-in for its lanes, and still refuses --company-data.
    assert!(p("test-microsoft --experimental-company-builds").is_ok());
    assert!(p("test-microsoft --experimental-company-builds --company-data demotool").is_err());
    assert!(HELP.contains("--experimental-company-builds"));
}

#[test]
fn server_url_is_checked_when_parsed() {
    assert_eq!(
        test_args("test A --server-url http://h:1/BC/").server_url.as_deref(),
        Some("http://h:1/BC/")
    );
    assert!(test_args("test A").server_url.is_none());
    for bad in ["h", "ftp://h/", "http://h/?a=1"] {
        assert!(
            p(&format!("test A --server-url {bad}"))
                .unwrap_err()
                .contains("--server-url"),
            "{bad}"
        );
    }
    assert!(HELP.contains("--server-url"));
}

#[test]
fn the_help_documents_the_flush_and_the_rebuild_and_the_baseline_entry() {
    for needle in ["al-runner cache clear", "--rebuild-compile-cache", "install baseline"] {
        assert!(HELP.contains(needle), "{needle}");
    }
}

#[test]
fn no_arguments_and_help_print_the_help() {
    assert_eq!(p("").unwrap(), Command::Help);
    assert_eq!(p("--help").unwrap(), Command::Help);
    assert_eq!(p("test --help").unwrap(), Command::Help);
    assert_eq!(p("--version").unwrap(), Command::Version);
}

#[test]
fn test_takes_apps_and_repeatable_options() {
    let a = test_args(
        "test A B.Source.zip --dep d1 --dep d2 --package-cache pc --codeunit 5 --codeunit 6 --filter x --test y",
    );
    assert_eq!(a.apps, vec![PathBuf::from("A"), PathBuf::from("B.Source.zip")]);
    assert_eq!(a.deps.len(), 2);
    assert_eq!(a.package_caches, vec![PathBuf::from("pc")]);
    assert_eq!(a.codeunits, vec![5, 6]);
    assert_eq!(a.filters, vec!["x", "y"]);
}

#[test]
fn the_missing_dependency_hint_names_an_al_runner_option() {
    // The error for a dependency nobody provides ends in "pass <flag> DIR". The flag must be one
    // `al-runner test` takes (it is `--dep`; `--deps` is al-test-cli's spelling).
    use al_deps::select::{DepDecl, SelectError};
    use al_deps::version::Version;
    let err = SelectError::Missing {
        dep: DepDecl {
            id: "a".into(),
            name: "A".into(),
            publisher: "P".into(),
            min: Version(1, 0, 0, 0),
            optional: false,
        },
        searched: Box::new([]),
        symbol_only: Box::default(),
    }
    .to_string();
    let flag = err
        .split("pass ")
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
        .unwrap_or_else(|| panic!("no `pass FLAG` hint in: {err}"));
    let a = test_args(&format!("test A {flag} d1"));
    assert_eq!(a.deps, vec![PathBuf::from("d1")], "{err}");
}

#[test]
fn the_flags_of_the_v2_tools_parse() {
    use al_testrun::isolation::TestIsolation as I;
    let a = test_args(
        "test A --isolation test --exclude-test Codeunit1.X --exclude-test Y --test-exact Codeunit1.Z --define FOO \
         --preprocessor-symbols A,B --auto-provision --test-timeout 7 --test Codeunit1.Z",
    );
    assert_eq!(a.isolation, Some(I::Function));
    assert_eq!(a.excludes, vec!["Codeunit1.X", "Y"]);
    assert_eq!(a.exact, vec!["Codeunit1.Z"]);
    assert_eq!(a.defines, vec!["FOO", "A", "B"]);
    assert_eq!(a.test_timeout_secs, Some(7));
    assert_eq!(a.filters, vec!["Codeunit1.Z"]);
    assert_eq!(test_args("test A --isolation codeunit").isolation, Some(I::Codeunit));
    let r = test_args("test A --isolation Required");
    assert_eq!((r.isolation, r.required_isolation), (None, true));
    assert_eq!(
        test_args("test A --test-isolation disabled").isolation,
        Some(I::Disabled)
    );
    assert!(
        p("test A --isolation nope")
            .unwrap_err()
            .contains("codeunit, test, disabled or required")
    );
}

#[test]
fn serve_takes_the_options_that_do_not_depend_on_a_request() {
    match p("serve --protocol 2 --package-cache pc --define S --test-timeout 5 --no-download").unwrap() {
        Command::Serve(d) => assert_eq!(
            d,
            [
                "--package-cache",
                "pc",
                "--define",
                "S",
                "--test-timeout",
                "5",
                "--no-download"
            ]
        ),
        other => panic!("{other:?}"),
    }
    assert!(matches!(p("serve").unwrap(), Command::Serve(_)));
    assert!(p("serve --protocol 1").unwrap_err().contains("only protocol 2"));
    assert!(p("serve --nope").unwrap_err().contains("unknown option"));
    assert!(p("serve --test Codeunit1.X").unwrap_err().contains("do not depend"));
    assert!(p("serve --coverage").unwrap_err().contains("do not depend"));
    assert!(p("serve OtherApp").unwrap_err().contains("do not depend"));
    // Hard rule 4: a request runs one plain test run, so the options that make a run of another kind are refused at
    // the start, not ignored by every request.
    let e = p("serve --recipe microsoft").unwrap_err();
    assert!(e.contains("--recipe microsoft") && e.contains("serve"), "{e}");
    let e = p("serve --impact-of ext").unwrap_err();
    assert!(e.contains("--impact-of") && e.contains("serve"), "{e}");
    // ADR-052 section 4 (owner answer 5): serve keeps refusing test sources.
    for flag in ["--affected-from microsoft", "--microsoft-app Tests-ERM"] {
        let e = p(&format!("serve {flag}")).unwrap_err();
        assert!(
            e.contains(flag.split(' ').next().unwrap()) && e.contains("serve"),
            "{flag}: {e}"
        );
    }
}

#[test]
fn coverage_flags_turn_coverage_on() {
    assert!(!test_args("test A").coverage);
    assert!(test_args("test A --coverage").coverage);
    let a = test_args("test A --coverage-out c.xml --coverage-json c.json");
    assert!(a.coverage);
    assert_eq!(a.coverage_out, Some(PathBuf::from("c.xml")));
    assert_eq!(a.coverage_json, Some(PathBuf::from("c.json")));
}

#[test]
fn impact_runs_take_junit_and_a_json_file_but_not_the_json_document_on_stdout() {
    assert!(p("test A --impact-of E --junit x.xml --output-json o.json --impact-json d.json").is_ok());
    assert!(p("test A --impact-of E --impact-accept a.json").is_ok());
    assert!(p("test A --impact-of E --output-json").is_err());
    assert!(p("test A --impact-of E --coverage").is_err());
    assert!(p("test A --impact-json d.json").is_err());
    assert!(p("test A --impact-accept a.json").is_err());
    assert!(p("impact-index build A --impact-json d.json").is_err());
    let a = test_args("test A --impact-of E --impact-accept a.json --impact-json d.json");
    assert_eq!(a.impact_accept, Some(PathBuf::from("a.json")));
    assert_eq!(a.impact_json, Some(PathBuf::from("d.json")));
}

#[test]
fn output_json_takes_a_file_only_when_the_name_ends_in_json() {
    assert_eq!(test_args("test A --output-json").output_json, Some(None));
    assert_eq!(
        test_args("test A --output-json r.json").output_json,
        Some(Some(PathBuf::from("r.json")))
    );
    assert_eq!(
        test_args("test A --output-json=out/r.txt").output_json,
        Some(Some(PathBuf::from("out/r.txt")))
    );
    let a = test_args("test --output-json A");
    assert_eq!(a.output_json, Some(None));
    assert_eq!(a.apps, vec![PathBuf::from("A")]);
}

#[test]
fn refusal_detail_takes_a_text() {
    assert_eq!(
        test_args("test A --refusal-detail is_not_lowered")
            .refusal_detail
            .as_deref(),
        Some("is_not_lowered")
    );
}

#[test]
fn top_failures_count_is_optional_like_top_refusals() {
    assert_eq!(test_args("test A --top-failures").top_failures, Some(DEFAULT_TOP));
    assert_eq!(test_args("test A --top-failures 15").top_failures, Some(15));
    let a = test_args("test --top-failures A");
    assert_eq!(a.apps, vec![PathBuf::from("A")]);
    assert!(p("test A --top-failures 0").is_err());
}

#[test]
fn assumption_options_parse() {
    assert_eq!(test_args("test A --top-assumptions").top_assumptions, Some(DEFAULT_TOP));
    assert_eq!(test_args("test A --top-assumptions 3").top_assumptions, Some(3));
    assert_eq!(p("--list-assumptions").unwrap(), Command::ListAssumptions(None));
    assert_eq!(
        p("--list-assumptions --cache-dir /c").unwrap(),
        Command::ListAssumptions(Some("/c".into()))
    );
    assert!(p("--list-assumptions --nope").is_err());
}

#[test]
fn top_refusals_count_is_optional() {
    assert_eq!(test_args("test A --top-refusals").top_refusals, Some(DEFAULT_TOP));
    assert_eq!(test_args("test A --top-refusals 25").top_refusals, Some(25));
    // The app after a bare --top-refusals is an app, not a count.
    let a = test_args("test --top-refusals A");
    assert_eq!(a.top_refusals, Some(DEFAULT_TOP));
    assert_eq!(a.apps, vec![PathBuf::from("A")]);
    assert!(p("test A --top-refusals 0").is_err());
}

#[test]
fn test_data_takes_an_optional_backup_path_that_cannot_be_mistaken_for_an_app() {
    let a = test_args("test A --test-data");
    assert!(a.test_data && a.test_data_path.is_none());
    let a = test_args("test A --test-data x/y.bak");
    assert_eq!(a.test_data_path, Some(PathBuf::from("x/y.bak")));
    assert_eq!(a.apps, vec![PathBuf::from("A")]);
    let a = test_args("test --test-data B.Source.zip");
    assert!(
        a.test_data && a.test_data_path.is_none(),
        "an app after a bare --test-data is an app"
    );
    assert_eq!(a.apps, vec![PathBuf::from("B.Source.zip")]);
    let a = test_args("test A --test-data=some/Demo.BACPAC");
    assert_eq!(a.test_data_path, Some(PathBuf::from("some/Demo.BACPAC")));
    let a = test_args("test A --test-data --test-data-company CRONUS");
    assert_eq!(a.test_data_company.as_deref(), Some("CRONUS"));
    // Test data is on by default, so a company can be named without --test-data.
    let a = test_args("test A --test-data-company X");
    assert!(!a.no_test_data && a.test_data_company.as_deref() == Some("X"));
}

#[test]
fn the_country_is_w1_unless_the_flag_names_another() {
    let a = test_args("test A");
    assert_eq!(a.country(), "w1");
    assert!(a.country.is_none());
    let a = test_args("test A --country US --bc-version 28");
    assert_eq!(a.country(), "us");
    let e = p("test A --country").unwrap_err();
    assert!(e.contains("--country needs a value"), "{e}");
    let e = p("test A --country u/s").unwrap_err();
    assert!(e.contains("not a country code"), "{e}");
    match p("warm --bc-version 28 --country us").unwrap() {
        Command::Warm(w) => assert_eq!(w.country(), "us"),
        other => panic!("{other:?}"),
    }
    let e = p("test A --country us --company-data package").unwrap_err();
    assert!(e.contains("only the w1 country"), "{e}");
}

#[test]
fn test_data_is_on_unless_no_test_data_turns_it_off() {
    let a = test_args("test A");
    assert!(
        !a.no_test_data && !a.test_data,
        "no flag: the default decides, not the user"
    );
    assert!(a.wants_test_data() && !a.test_data_asked_for());
    let a = test_args("test A --no-test-data");
    assert!(a.no_test_data && !a.wants_test_data());
    let a = test_args("test A --test-data");
    assert!(a.wants_test_data() && a.test_data_asked_for());
    assert!(
        p("test A --test-data --no-test-data")
            .unwrap_err()
            .contains("cannot be combined")
    );
    assert!(
        p("test A --test-data=x.bak --no-test-data")
            .unwrap_err()
            .contains("cannot be combined")
    );
    assert!(
        p("test A --no-test-data --test-data-company X")
            .unwrap_err()
            .contains("--no-test-data")
    );
}

#[test]
fn bad_invocations_say_what_is_wrong() {
    assert!(p("test").unwrap_err().contains("at least one APP"));
    assert!(p("test A --nope").unwrap_err().contains("unknown option `--nope`"));
    assert!(p("test A --codeunit x").unwrap_err().contains("object id"));
    assert!(p("test A --dep").unwrap_err().contains("needs a value"));
    assert!(p("frobnicate").unwrap_err().contains("unknown command"));
    assert!(p("test A --verbose --quiet").is_err());
    assert_eq!(
        test_args("test A --color always").color,
        crate::style::ColorMode::Always
    );
    assert_eq!(test_args("test A").color, crate::style::ColorMode::Auto);
    assert!(p("test A --color blue").is_err());
}

#[test]
fn deferred_and_unavailable_options_are_refused_by_name() {
    assert!(
        p("test A --test-data-normalize-company")
            .unwrap_err()
            .contains("not available")
    );
    assert!(p("test A --watch").unwrap_err().contains("deferred"));
    assert!(p("run").unwrap_err().contains("not implemented yet"));
}

#[test]
fn a_matrix_parses_its_cells_and_refuses_what_cannot_be_combined_with_it() {
    let a = test_args("test App --bc-version 27.5,28.5 --country w1,us --matrix-parallel 2 --matrix-memory 12");
    assert_eq!(a.matrix.as_ref().map(Vec::len), Some(4));
    assert_eq!((a.bc_version.as_deref(), a.country.as_deref()), (None, None));
    assert_eq!(a.matrix_parallel, Some(2));
    assert_eq!(a.matrix_memory_gb, Some(12.0));
    assert_eq!(test_args("test App --country w1,de").matrix.map(|m| m.len()), Some(2));
    match p("test-microsoft --matrix 27.5:w1,29.0:us").unwrap() {
        Command::TestMicrosoft(a) => assert_eq!(a.matrix.map(|m| m.len()), Some(2)),
        other => panic!("{other:?}"),
    }
    assert!(test_args("test App --bc-version 29.0 --country us").matrix.is_none());
    for (cmd, says) in [
        ("test App --matrix 27.5 --bc-version 28.5,29.0", "--matrix"),
        ("test App --matrix 27.5,27.5", "twice"),
        ("test App --bc-version 27.5,28.5 --matrix-parallel 0", "at least 1"),
        ("test App --matrix-parallel 2", "need a matrix"),
        ("test App --bc-version 27.5,28.5 --explain T", "--explain"),
        ("test App --bc-version 27.5,28.5 --test-data x.bak", "one backup"),
        ("test App --bc-version 27.5,28.5 --output-json", "FILE"),
        ("impact-index build App --bc-version 27.5,28.5", "test-microsoft only"),
    ] {
        let e = p(cmd).unwrap_err();
        assert!(e.contains(says), "{cmd}: {e}");
    }
    // ADR-052 section 4: serve refuses matrix options.
    for cmd in [
        "serve --bc-version 27.5,28.5",
        "serve --matrix 27.5",
        "serve --matrix-parallel 2",
    ] {
        let e = p(cmd).unwrap_err();
        assert!(e.contains("al-runner serve cannot be combined"), "{cmd}: {e}");
    }
}

/// ADR-074: `--test-driver` picks whose way of starting the tests the run models; bc-linux unless asked.
#[test]
fn test_driver_is_bc_linux_unless_asked() {
    use al_testrun::runtime::TestDriver;
    assert_eq!(test_args("test A").test_driver, None);
    assert_eq!(
        test_args("test A --test-driver bccontainerhelper").test_driver,
        Some(TestDriver::BcContainerHelper)
    );
    assert_eq!(
        test_args("test A --test-driver bc-linux").test_driver,
        Some(TestDriver::BcLinux)
    );
    let e = p("test A --test-driver al-go").unwrap_err();
    assert!(e.contains("bc-linux or bccontainerhelper"), "{e}");
}
