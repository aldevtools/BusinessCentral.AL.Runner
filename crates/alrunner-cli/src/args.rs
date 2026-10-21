//! The command line of `al-runner` (ADR-016 section 2): parsing and the help text.

use std::path::PathBuf;

/// What the command line asks for.
#[derive(Debug, PartialEq)]
pub enum Command {
    Help,
    Version,
    /// `al-runner --list-assumptions`: the registry of unprobed assumptions with the counts of the last run.
    ListAssumptions(Option<PathBuf>),
    Test(Box<TestArgs>),
    /// `al-runner serve`: the options every request's `al-runner test` run gets (ADR-028).
    Serve(Vec<String>),
    /// `al-runner warm`: find (or download) the demo backup and build its index.
    Warm(Box<TestArgs>),
    /// `al-runner cache clear [--cache-dir DIR]`: remove every entry of the compile cache.
    CacheClear(Option<PathBuf>),
    /// `al-runner cache prune [--dry-run] [--all-other-builds] [--cache-dir DIR]`.
    CachePrune {
        dir: Option<PathBuf>,
        dry_run: bool,
        all_other_builds: bool,
    },
    /// `al-runner cache info [--cache-dir DIR]`.
    CacheInfo(Option<PathBuf>),
    /// `al-runner cache --help`: the cache part of the help.
    CacheHelp,
    /// `al-runner impact-index build APP... [OPTIONS]` (ADR-043 step 2): run the tests of the APPs once and keep, per
    /// test, what it raised and touched, for the impact runs that follow.
    ImpactIndexBuild(Box<TestArgs>),
    /// `al-runner impact-index check-static APP... [OPTIONS]` (ADR-043 step 5): check that static reach covers every key
    /// each test recorded in the cached index (a key it misses is a bug in static reach), and say how much more static
    /// reach selects.
    ImpactIndexCheckStatic(Box<TestArgs>),
}

/// `al-runner test`.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct TestArgs {
    /// The apps whose tests run: an app folder (with `app.json`), a `*.Source.zip` or an `.app`.
    pub apps: Vec<PathBuf>,
    /// `--dep PATH`: a package file, an app folder, or a folder of packages.
    pub deps: Vec<PathBuf>,
    /// `--package-cache DIR`: a folder of `.app` files.
    pub package_caches: Vec<PathBuf>,
    pub bc_version: Option<String>,
    /// `--country CODE` (lower case): the BC country version whose Microsoft apps and demo data the run uses
    /// (ADR-020 amendment, owner 2026-10-17). `None`: `w1`. Never read from app.json or AL-Go settings.
    pub country: Option<String>,
    pub no_download: bool,
    pub cache_dir: Option<PathBuf>,
    /// `--no-compile-cache`: compile the dependency apps on every run, only what the tests reach.
    pub no_compile_cache: bool,
    /// `--require-compile-cache`: a run without a usable cache entry is an error (CI).
    pub require_compile_cache: bool,
    /// `--rebuild-compile-cache`: read no entry of the compile cache; build and write the ones this run needs.
    pub rebuild_compile_cache: bool,
    /// `--codeunit ID`: select a test codeunit by object id.
    pub codeunits: Vec<u32>,
    /// `--filter TEXT`: select the tests whose name contains TEXT (any case).
    pub filters: Vec<String>,
    pub junit: Option<PathBuf>,
    /// `--output-json [FILE]`: the v2-shaped JSON document. `Some(None)`: on stdout, and the human report goes to
    /// stderr. `Some(Some(file))`: written to the file, the report stays on stdout.
    pub output_json: Option<Option<PathBuf>>,
    /// `--isolation codeunit|test|disabled`: the isolation of every test codeunit (default: Codeunit).
    pub isolation: Option<al_testrun::isolation::TestIsolation>,
    /// `--exclude-test TEXT`: tests whose name matches are left out of the selection. Repeatable.
    pub excludes: Vec<String>,
    /// `--test-exact NAME`: the test whose method name or `Codeunit<ID>.<Method>` is exactly NAME. Repeatable.
    pub exact: Vec<String>,
    /// `--define SYMBOL`: a preprocessor symbol for the apps. Repeatable. `--preprocessor-symbols A,B` adds several.
    pub defines: Vec<String>,
    /// `--coverage`: count the statements of the APPs each test runs.
    pub coverage: bool,
    /// `--tdd` (ADR-051): a call in a test app of a procedure that does not exist yet runs against a stub.
    pub tdd: bool,
    /// `--coverage-out FILE`: write Cobertura XML there (implies `--coverage`; default `coverage.cobertura.xml`).
    pub coverage_out: Option<PathBuf>,
    /// `--coverage-json FILE`: write the statements and the statements of each test as JSON (implies `--coverage`).
    pub coverage_json: Option<PathBuf>,
    /// `--explain TEST`: run one test and say where it was refused or failed.
    pub explain: Option<String>,
    /// `--top-refusals [N]`: how many refusal reasons to list (default 10).
    pub top_refusals: Option<usize>,
    /// `--top-failures [N]`: how many failure messages to list, ranked (default 10).
    pub top_failures: Option<usize>,
    /// `--top-assumptions [N]`: the unprobed assumptions the tests ran through, ranked by flagged passes.
    pub top_assumptions: Option<usize>,
    /// `--refusal-detail TEXT`: the reasons that contain TEXT, grouped finer than `--top-refusals`.
    pub refusal_detail: Option<String>,
    pub allow_not_run: bool,
    /// Not a command line option: `al-runner serve`'s `execute` runs the `OnRun` trigger of one codeunit per app
    /// instead of the tests.
    pub execute: bool,
    /// Not a command line option: `captureValues` and `iterationTracking` of a server request.
    pub capture: al_vm::CaptureConfig,
    pub verbose: bool,
    /// `--color auto|always|never` (default auto).
    pub color: crate::style::ColorMode,
    pub quiet: bool,
    pub timings: bool,
    pub dotnet_sidecar: Option<PathBuf>,
    pub dotnet_assembly_dirs: Vec<PathBuf>,
    pub dotnet_allow_roll_forward: bool,
    pub dotnet_timeout_secs: Option<u64>,
    /// `--test-timeout-secs N`: a test that uses more CPU time is stopped and not run (default 20; 0: no limit).
    pub test_timeout_secs: Option<u64>,
    pub require_dotnet: bool,
    pub files_dir: Option<PathBuf>,
    /// `--environment saas|onprem`: what Environment Information answers (default saas, as probed).
    pub environment: Option<al_host::Environment>,
    /// `--server-url URL` (else `$ALRUNNER_SERVER_URL`): the base address `GetUrl` answers with (ADR-042).
    pub server_url: Option<String>,
    /// Not a command line option: the name `CompanyName()` answers when no company is restored (a lane of the recipe).
    pub company_name: Option<String>,
    /// `--test-data [PATH]`: restore a CRONUS company from a BC backup (ADR-020).
    pub test_data: bool,
    /// `--no-test-data`: run without company data. Test data is on by default (ADR-020, owner
    /// decision of 2026-10-14).
    pub no_test_data: bool,
    /// The `.bak` or `.bacpac` named; else the cached artifact for the selected BC build.
    pub test_data_path: Option<PathBuf>,
    /// `--test-data-company NAME`: which company of the backup (default: the demo company).
    pub test_data_company: Option<String>,
    /// `--company-data package` (or `--package PATH`): build the company from the artifact's RapidStart package instead of
    /// restoring the backup's company, as Microsoft's Legacy test lane does (Company-Initialize, then the package).
    pub company_package: bool,
    /// `--package PATH`: the `.rapidstart` file; else the one of the artifact that holds the backup.
    pub package_path: Option<PathBuf>,
    /// `--disabled-tests DIR`: BCApps' `src/DisabledTests` (or one app's folder); tests Microsoft turns off are marked.
    pub disabled_tests_dir: Option<PathBuf>,
    /// `--parity`: skip the tests Microsoft turns off instead of running and marking them.
    pub parity: bool,
    /// `--recipe microsoft`: run the way Microsoft's CI does: one run per lane (the company each test codeunit gets),
    /// Microsoft's disabled tests skipped. The files come from BCApps (`--bcapps DIR`, else the cache or a fetch).
    pub recipe: bool,
    /// `--bcapps DIR`: a BCApps checkout (or its `src/DisabledTests`, `build/projects`, `build/scripts` parts) for `--recipe`.
    pub bcapps_dir: Option<PathBuf>,
    /// `--impact-of APP` (ADR-043): the partner extensions whose effect on the tests of the APPs is measured. Repeatable.
    pub impact_of: Vec<PathBuf>,
    /// `--impact MODE`: how an impact run selects tests: `dynamic` (default with an index), `static` (default
    /// without one) or `all`.
    pub impact: Option<String>,
    /// `--impact-accept FILE` (ADR-043 step 3): the changes the partner accepts, in the shape `--impact-json` writes.
    pub impact_accept: Option<PathBuf>,
    /// `--impact-json FILE` (ADR-043 step 3): write the per-test diff of an impact run as JSON.
    pub impact_json: Option<PathBuf>,
    /// Not a command line option: only the test codeunits of the first N APPs are selected (the "with" side of an
    /// impact run gives the extensions as further APPs).
    pub test_apps: Option<usize>,
}

impl TestArgs {
    /// The country version of the run: `--country`, else `w1`.
    pub fn country(&self) -> &str {
        self.country.as_deref().unwrap_or("w1")
    }

    /// Whether the run restores a company: unless `--no-test-data` says otherwise.
    pub fn wants_test_data(&self) -> bool {
        !self.no_test_data
    }

    /// Whether the command line asked for test data (a backup or `--test-data`), so a failure to
    /// find it stops the run instead of turning the data off with a message.
    pub fn test_data_asked_for(&self) -> bool {
        self.test_data || self.test_data_path.is_some() || self.company_package
    }
}

/// Whether `v` names a database backup, which is how `--test-data PATH` tells a path from an APP.
fn is_backup(v: &str) -> bool {
    let v = v.to_ascii_lowercase();
    v.ends_with(".bak") || v.ends_with(".bacpac")
}

pub const DEFAULT_TOP: usize = 10;

pub const HELP: &str = "\
al-runner: run AL tests without a service tier (AL Runner, alpha)

USAGE
    al-runner test [OPTIONS] APP...
    al-runner serve [--protocol 2] [OPTIONS]
    al-runner impact-index build|check-static [OPTIONS] APP...
    al-runner cache clear|prune|info [--cache-dir DIR]
    al-runner warm [PATH.bak] [OPTIONS]
    al-runner --list-assumptions [--cache-dir DIR]
    al-runner --version | --help

APP is an app folder (with app.json), a source archive (`Tests-SMB.Source.zip`) or an `.app`
that carries its source. Every `Subtype = Test` codeunit of every APP runs, each in its own
session, under TestIsolation = Codeunit (the default of Microsoft's test runner).

DEPENDENCIES
    Looked up for the closure of the APPs' app.json dependencies, in this order:
    --dep, --package-cache, each APP folder's .alpackages, then Microsoft's artifact feed.
    A Microsoft app in .alpackages that is older than the BC version of the run (--bc-version,
    or the build taken from the feed) gives way to the feed's copy of that version; it is used
    only when the feed does not have the app. With --no-download it is used as before. Apps
    from --dep and --package-cache, and other publishers' apps, are always used as found.
    --dep PATH           A package file (.app, *.Source.zip), an app folder or a folder of
                         packages. Repeatable.
    --package-cache DIR  A folder of .app files. Repeatable.
    --bc-version V       BC major, major.minor or an exact build (four numbers). An exact build
                         also fills Microsoft's $(app_...) variables in source archives that
                         lie outside a folder named for their build.
    --no-download        Never fetch from the artifact feed (apps, System.app, demo data).
    Microsoft's apps, and the platform's System.app, come from the artifact feed. Without
    --bc-version they come from the build of the tests when the tests are Microsoft's (a
    Tests-*.Source.zip or .app in a folder named for its build, or with the build as its
    version); if that build is gone from the feed the newest match is used and the run says so.
    A build named exactly (four numbers) that is in ~/.bcartifacts.cache/sandbox is read from
    there, without the feed; any matching build there is the fallback when the feed has none or
    is offline.
    Without --bc-version, the build of the run is, in this order: the build of the tests when
    they are Microsoft's; the build of the Microsoft packages in the project's package folders
    (.alpackages, --dep, --package-cache) when it is of the target BC major and not below the
    application and platform versions of app.json; else the newest published build of the
    target major at or above those versions. The target major is that of app.json's application
    version, else the newest BC major the runtime is verified against. --verbose prints the rule.
    --cache-dir DIR      Where downloaded artifacts and the compile cache are kept. Else
                         $ALRUNNER_CACHE, else $XDG_CACHE_HOME/al-runtime, else ~/.cache/al-runtime.

COMPILE CACHE
    The Microsoft apps and the other dependency apps are compiled completely once, and the result
    is kept in <cache dir>/compile/ (about 20 MB per set of apps). Later runs with the same apps,
    versions, sources, target and the same al-runner build load it instead of compiling them.
    The first run of a new set pays for the build (about 10 s for Business Central).
    --no-compile-cache   Compile the dependency apps on every run, only what the tests reach.
                         Same as ALRUNNER_NO_COMPILE_CACHE=1.
    --require-compile-cache  Fail when there is no usable cache entry instead of building one (CI).
    --rebuild-compile-cache  Read no cache entry: build the entries this run needs again and write
                         them over the old ones (the compiled dependency apps, the scan of their
                         sources, the install baseline). Entries of other app sets stay.
    The cache also holds the install baseline: what the install code of the dependency apps and
    Company-Initialize leave in the database, with the demo data if the run has it. It is kept
    under a key of the al-runner build, the dependency apps, the app's sources, the BC target, the
    environment, the demo backup and the day, and a run that finds it starts its clock at the
    instant the run that made it began.
    Old entries go by themselves: at the end of `al-runner test`, at most once a day and for at most
    200 ms, the folder is cleaned of entries of other al-runner builds unused for 7 days, entries of
    any build unused for 30 days, and leftover temporary files older than a day. A cache hit
    renews the entry. ALRUNNER_NO_CACHE_PRUNE=1 turns this off.
    al-runner cache prune [--dry-run] [--all-other-builds] [--cache-dir DIR]
                         Run that cleanup now and print how many files and bytes went (--dry-run:
                         only print). --all-other-builds also removes entries of other builds that
                         are younger than 7 days.
    al-runner cache info [--cache-dir DIR]
                         Print the entries, their size, and the split by al-runner build.
    al-runner cache clear [--cache-dir DIR]
                         Remove every entry of the compile cache (<cache dir>/compile/): use it
                         when a cached result is in doubt. The next run builds what it needs.

SELECTION (a test is selected when any option matches it)
    --codeunit ID        A test codeunit by object id. Repeatable.
    --filter TEXT        (`--test TEXT` is the same) Tests whose name contains TEXT, any case.
                         TEXT with a dot (`Codeunit50100.Foo`) is matched against
                         `Codeunit<ID>.<Method>`, TEXT without a dot against the method name.
                         Repeatable.
    --test-exact NAME    The test whose method name or `Codeunit<ID>.<Method>` is exactly NAME
                         (any case). Repeatable.
    --exclude-test TEXT  Leave out the tests TEXT matches (matched like --filter). Wins over
                         every selecting option. Repeatable.
    --explain TEST       Run one test (its name, or a part of it that only one test has) and print
                         its outcome and, for a failure or a refusal, the routines and source
                         lines it left from. Selects that test like --filter.
    A selected test's codeunit runs whole (tests of a codeunit share state); only the selected
    tests are reported. Only the selected codeunits are compiled.

OUTPUT
    Default: the version line, one line per failing test with the AL call stack of its error
    below it (a frame whose format is not known reads as such), the top refusal reasons, the
    Tests line, the totals table, one time line (total, compile, run, tests/s) and the Result
    line. A warning that changes how to read the result is one short line, only
    when it applies (for example: baseline incomplete, 16 of 17 install roots ran to their end).
    Errors go to stderr. --junit also holds the stack as the text of each <failure>.
    --verbose, -v        Everything else: first the setup (dependency notes, target, apps, build
                         chosen by, dependencies, threads, isolation, server address, test data,
                         baseline details), then one line per test (PASS, FAIL, NOT RUN with its
                         reason) with its run time in ms, the test data rows line and the full Time
                         line (it replaces the short time line). There is one level.
    --quiet, -q          Summary only.
    --color auto|always|never
                         Color the report: PASS green, FAIL red, NOT RUN yellow, flagged cyan,
                         headings bold. auto (default): only when stdout is a terminal and
                         NO_COLOR is not set. JUnit and JSON output are never colored.
    --top-refusals [N]   List the N most common reasons tests were not run (default 10).
    --top-failures [N]   List the N most common failure messages (default 10), numbers and quoted
                         values folded.
    --top-assumptions [N]  List the N unprobed assumptions most passes ran through (default 10). A pass
                         that went through one is a FLAGGED pass: the summary counts it (N passed (M
                         flagged: rely on unprobed assumptions)), --junit lists its assumption ids in a
                         property, --explain names them. --list-assumptions (alone) prints the registry with the
                         counts of the last run.
    --refusal-detail TEXT  List the not-run reasons that contain TEXT (site and call arguments
                         folded, names kept), most common first (default 30 lines).
    --junit FILE         Write JUnit XML. A test that did not run is <skipped> with its reason.
    --output-json [FILE] Write one JSON document of the shape of AL Runner v2: `tests[]` with
                         `name` (`Codeunit<ID>.<Method>`), `status` (`pass`, `fail`, `error`),
                         `durationMs`, `message`, `stackTrace`, and `passed`, `failed`, `errors`,
                         `skipped`, `total`, `notRun`, `exitCode`, `wallSeconds`. A test that did not
                         run is `error` with the message `NotRun: out-of-scope: <reason>`; one the
                         time limit stopped is `error` with `Test exceeded <N>s timeout.`. Without
                         FILE the document is the only thing on stdout and the report goes to
                         stderr. `--output-json=FILE` also works. See docs/integrations/migrating-tools-to-v3.md.
    --timings            Compile phases and the slowest test codeunits.

TEST-FIRST
    --tdd                A test that calls a procedure that does not exist yet runs against a stub
                         instead of failing to compile. Only calls in test apps (apps that declare a
                         test codeunit) on a codeunit variable become stubs. A stub returns the
                         default value of the type its use asks for (0, '', false, an empty record),
                         writes nothing and never raises; `var` arguments stay unchanged. A site
                         whose type its use does not tell is refused (its test is not run). Every
                         test that ran a stub is stubbed, pass or fail: the text names the stubs,
                         `Tests:` counts stubbed passes and failures apart, --output-json adds
                         `stubbed` and `generatedStubs` per test and in the summary, --junit a
                         `stubbed` property. A stubbed pass counts as a pass for the exit code.

COVERAGE
    --coverage           Count how often each statement of the APPs runs, per test. Writes Cobertura XML to
                         `coverage.cobertura.xml` (see --coverage-out). Off, a run pays one branch per statement.
    --coverage-out FILE  Write the Cobertura XML to FILE (implies --coverage). Files are named relative to
                         the working directory when they lie below it; a line has the runs of the
                         statements that start on it.
    --coverage-json FILE Write a JSON document with `coverage` ([{file, statements: [{id, scope, line,
                         column, endLine, hits}]}]) and `perTestCoverage` ([{test, coverage: [...]}]) (implies --coverage): every statement with
                         its runs, and for each test the statements it ran. File names are absolute with
                         forward slashes (`archive.zip!/inner/path.al` inside a package). Statements are the
                         ones the lowering marks, one per AL statement.

IMPACT (ADR-043)
    --impact-of APP      Measure what an extension changes in the tests of the APPs (for example
                         Microsoft's test apps): run the selected tests without the extension, then
                         with it, and list per test what changed and which extension procedures ran.
                         Repeatable. With --disabled-tests, the tests Microsoft turns off are left out.
                         --junit FILE writes the run with the extension, each test with the properties
                         impact_class (its change class), impact_without (its outcome without the
                         extension) and impact_accepted; --output-json FILE writes that run in AL
                         Runner v2's shape, with the exit code of the impact run.
    --impact MODE        How the tests are chosen. `dynamic` (the default when an impact index of the
                         same APPs and options is cached): only the test codeunits that reached one of
                         the extension's impact points in the index's run, once, with the extension;
                         the outcomes without it come from the index. `all`: every selected test runs
                         without and with the extension. `static`: the test codeunits whose tests can
                         reach an impact point over the call graph, both sides; with no index cached,
                         the run warns and does what `static` does.
    --impact-json FILE   Write the per-test diff as JSON: `format` (1), `mode`, `extensions`, `testApps`,
                         `counts` (per class), `accepted`, `notSelected` ({tests, codeunits}, dynamic
                         and static selection), `exitCode`, and `tests[]` with `codeunit`, `codeunitName`,
                         `test`, `class`, `accepted`, `without` and `with` ({outcome: pass, fail, not
                         run or missing; flagged; message}, or null), `reached` (impact points) and
                         `ran` ([{routine, calls}]).
    --impact-accept FILE Changes the extension makes on purpose. FILE has the shape --impact-json
                         writes (the file of an earlier run works as it is; delete the tests you do
                         not accept). A test is accepted when its codeunit, name, outcome with the
                         extension and message equal an entry whose class is a change. An accepted
                         change is listed, marked (accepted), and does not fail the run; an entry
                         that no longer matches is listed below the report.
    Exit codes of an impact run: 0 when no test changed for the worse, or every such change is
                         accepted; 1 when a test is newly failing, fails differently, is newly refused
                         or is a selection miss and not accepted; 2 for bad options or an unreadable
                         accept file; 6 when a side selected no test. Tests not run on a side do not
                         give 7 or 8: they are the class not verified.
    al-runner impact-index build [OPTIONS] APP...
                         Run the tests of the APPs once (the same options as `test`, without
                         --impact-of) and keep in the cache, per test, the events it raised and the
                         tables, pages and reports it used. An impact run with the same APPs and
                         options reads it. Never built implicitly.
    al-runner impact-index check-static [OPTIONS] APP...
                         Check that static reach covers everything each test recorded in the cached
                         index of the same APPs and options (exit 1 on a miss), and how much more
                         static reach selects.

ISOLATION
    --isolation MODE     (`--test-isolation MODE` is the same) `codeunit` (default): the database
                         is rolled back when a test codeunit ends. `test` (also `function`, `method`): it is rolled back to its state after OnRun
                         after every test method; the codeunit's AL variables are kept, as BC's
                         TestIsolation = Function does. `disabled`: not pinned, so every test is
                         reported as not run.

EXIT CODES
    0  every selected test passed
    1  a test failed
    2  bad invocation or the run could not start
    3  the APPs do not compile: an error the AL compiler gives (its code, such as AL0118);
       no test ran. The errors are on stderr and in `compilationErrors` of --output-json.
       The check covers the AL compiler's errors that the must-not-compile corpus pins,
       not all of them. Source our parser does not accept is not such an error: the
       tests that need the file are not run (exit 7), with the parse error as the reason
    6  the selection matched no test
    7  some tests were not run (a refusal); --allow-not-run makes this 0
    8  some tests were not run because the .NET helper could not be used (not found, no .NET 8
       runtime): a problem of the machine; --allow-not-run makes this 0
    Order when several apply: 2, 6, 1, 8, 7.
    --allow-not-run      Return 0 instead of 7 or 8.

SERVE
    al-runner serve      AL Runner v2's server protocol (ALchemist, LethAL): one JSON object per line on
                         stdin, answers on stdout (the first line is {\"ready\":true}), logs on stderr.
                         Commands: runTests, execute, cancel, shutdown. Requests run in this process,
                         one at a time: the compile of the last request is kept, so a request with
                         the same sources and test codeunits reuses it (`cached`), and one that only
                         changes routine code compiles just those objects again (`incremental`).
                         Every request starts from the same database. runTests writes one `test` line
                         per test as it finishes, then a `summary`; execute runs one OnRun trigger per
                         app, or inline `code`, and answers with one line. cancel is answered at once
                         and stops the run after the current test; shutdown stops a run the same way
                         and exits. The start options are those of `test` that do not depend on a
                         request (--package-cache, --dep, --define, --test-timeout, --bc-version,
                         --cache-dir, --no-download, --test-data, ...); --recipe and --impact-of are
                         refused. A runTests request carries
                         sourcePaths, packagePaths, test, excludeTests, testIsolation, coverage,
                         perTestCoverage, captureValues, iterationTracking, announceStart, serverUrl,
                         tdd (`serve --tdd` makes it the default).
                         Fields and answers: docs/integrations/migrating-tools-to-v3.md, section 4b.

WARM
    al-runner warm [PATH.bak] [--bc-version BUILD] [--country CODE] [--cache-dir DIR] [--no-download]
                  [--test-data-company NAME]
        Finds the demo backup the way `test` does (downloading it if need be), opens it and
        keeps its page map and catalog in the cache. The first open of a backup verifies the
        whole file (2 to 3 s on a 900 MB backup); later opens take milliseconds. Run it once
        before a demo on a new machine.

COUNTRY
    --country CODE       The BC country version: `us`, `gb`, `de`, ... (the names of the artifact feed).
                         Default `w1`. Only this flag selects it; app.json and AL-Go settings are not read.
                         The country's own Microsoft apps (Base Application and the others under
                         Applications.<CODE>/ of its artifact) replace the w1 ones, and the demo data is that
                         country's backup (BusinessCentral-<CODE>.bak, about 600 MB the first time). The
                         platform artifact stays the same. Not combinable with --company-data package.

TEST DATA
    On by default for a run that uses Microsoft's apps: the CRONUS demo company of the build is
    restored. Microsoft's tests expect it, and many of them fail on missing setup data without it.
    A table is read the first time a test touches it; the rest is never read. The backup is found
    in this order: the build of the Microsoft apps in ~/.bcartifacts.cache/sandbox/<build>/w1,
    then the artifact cache, then it is downloaded once from the artifact feed (about 520 MB,
    progress on stderr) into the cache, then a backup of another build of the same BC version
    (the line says which). If none can be had, test data is off and the run says why.
    --no-test-data       Run without company data.
    --test-data [PATH]   Name a backup (.bak or .bacpac), or insist on test data: a missing
                         backup then stops the run. Write `--test-data=PATH` or end the name in
                         .bak/.bacpac: a bare name after --test-data is an APP.
    --test-data-company NAME  The company to restore (exact name or prefix). Default: the only
                         company, else 'CRONUS International Ltd.'.
    The summary says what was loaded and every exclusion: tables the runtime could not rebuild,
    tables the reader failed on (with its text), and columns left out.
    --company-data bak|package  `bak` (default) restores the CRONUS company of the backup. `package`
                         builds the company the way Microsoft's Legacy test lane does: a fresh
                         company, Company-Initialize, then the RapidStart package of the artifact
                         (platform/ConfigurationPackages/*.rapidstart, the DemoTool's Extended data).
                         Opt-in; nothing else changes.
    --package PATH       The .rapidstart file to use (implies --company-data package).
    --disabled-tests DIR  Microsoft's list of disabled tests: BCApps' src/DisabledTests, or one
                         app's folder (every *.json below DIR is read). Those tests still run and
                         the summary and the JUnit file mark them (property disabled_by_microsoft).
    --parity             With --disabled-tests: do not run those tests.
    --recipe microsoft   Run the way Microsoft's CI does. BCApps' settings decide the company of each test
                         codeunit (a lane): the Legacy test apps get the RapidStart package company, a codeunit
                         with TestType = UnitTest an empty company, IntegrationTest the Contoso setup company,
                         the rest the Contoso company with all demo data. One run per lane, each with its own
                         summary and, with --junit FILE, its own file (FILE.<lane>.xml). Microsoft's disabled
                         tests are skipped and counted. The BCApps files (MIT) of the release (releases/<major>.<minor>)
                         are fetched with git, sparse, into <cache dir>/bcapps/<major>.<minor>/ once; --no-download
                         uses only that cache. Not combinable with the company and disabled-test options.
    --bcapps DIR         With --recipe: use this BCApps checkout instead of the cache. $ALRUNNER_BCAPPS_URL
                         replaces the GitHub address the fetch uses.

OTHER
    --server-url URL     The base address of the web client that GetUrl(ClientType::Web) answers with
                         (default http://localhost:8080/BC, else $ALRUNNER_SERVER_URL). Absolute http or
                         https URL without a query. The summary prints the address. Other GetUrl forms are refused.
    --environment ENV    The server Environment Information describes: `saas` (default; the probed
                         sandbox: IsSaaS and IsSandbox Yes) or `onprem` (IsSaaS and IsSandbox No
                         until a test library's SetTestability* calls change them; Microsoft's own
                         test suites expect this, for example codeunit 134614). The on-prem values
                         are an unprobed assumption (docs/probes-needed.md).
    --files-dir DIR      Let the File data type read and write real files under DIR.
    --define SYMBOL      A preprocessor symbol (`#if SYMBOL`) for the APPs. Repeatable.
                         `--preprocessor-symbols A,B` names several.
    --auto-provision     Accepted and ignored (v2 downloaded platform apps with it; v3 always does,
                         unless --no-download).
    --test-timeout-secs N  (`--test-timeout N` is the same) A test that uses more than N seconds of CPU time (default 20; the
                         load of the machine does not count; a test that hangs without
                         computing is stopped at 10 times N of wall time) is stopped at its
                         next loop iteration and reported as not run, with the limit as the
                         reason; the codeunit's later failures are then not trusted. 0: no limit.
    --dotnet-sidecar PATH, --dotnet-assembly-dir DIR, --dotnet-allow-roll-forward,
    --dotnet-timeout-secs N, --require-dotnet     The optional .NET helper (ADR-014).

ENVIRONMENT
    ALRUNNER_CACHE       The cache folder when --cache-dir is not given.
    ALRUNNER_SERVER_URL  The address --server-url sets, when it is not given.
    ALRUNNER_NO_COMPILE_CACHE=1, ALRUNNER_NO_CACHE_PRUNE=1  See COMPILE CACHE.
    ALRUNNER_BCAPPS_URL  The address --recipe fetches BCApps from.
    ALRUNNER_NO_PATH_NOTE=1  No note about another al-runner on PATH (AL Runner v2's dotnet tool has
                         the same name), and no list of them under --version.
    NO_COLOR             No color with --color auto.
    ALRUNNER_TEST_TIMEOUT, AL_RUNNER_TEST_TIMEOUT_SEC  The test time limit in seconds when
    --test-timeout-secs is not given (the second is AL Runner v2's name).
    AL_CHECK_THREADS, AL_LOWER_THREADS, AL_TEST_THREADS  Worker threads of the check, the
    lowering and the test codeunits (default: one per core).
";

fn number(flag: &str, v: &str) -> Result<usize, String> {
    v.parse().map_err(|_| format!("{flag} needs a number, got `{v}`"))
}

/// The COMPILE CACHE section of the help: what `al-runner cache --help` prints.
pub fn cache_help() -> &'static str {
    let start = HELP.find("COMPILE CACHE\n").unwrap_or(0);
    let end = HELP[start..].find("\n\n").map_or(HELP.len(), |e| start + e + 1);
    &HELP[start..end]
}

/// `al-runner cache clear|prune|info`.
fn parse_cache(mut it: impl Iterator<Item = String>) -> Result<Command, String> {
    let sub = match it.next().as_deref() {
        Some("--help" | "-h") => return Ok(Command::CacheHelp),
        Some(s @ ("clear" | "prune" | "info")) => s.to_string(),
        Some(other) => {
            return Err(format!(
                "unknown cache command `{other}` (use `al-runner cache clear`, `prune` or `info`)"
            ));
        }
        None => return Err("al-runner cache needs a command: `clear`, `prune` or `info`".into()),
    };
    let (mut dir, mut dry_run, mut all_other_builds) = (None, false, false);
    while let Some(f) = it.next() {
        match f.as_str() {
            "--help" | "-h" => return Ok(Command::CacheHelp),
            "--cache-dir" => dir = Some(it.next().ok_or("--cache-dir needs a value")?.into()),
            "--dry-run" if sub == "prune" => dry_run = true,
            "--all-other-builds" if sub == "prune" => all_other_builds = true,
            other => {
                return Err(format!(
                    "unknown option `{other}` for `al-runner cache {sub}` (see --help)"
                ));
            }
        }
    }
    Ok(match sub.as_str() {
        "clear" => Command::CacheClear(dir),
        "prune" => Command::CachePrune {
            dir,
            dry_run,
            all_other_builds,
        },
        _ => Command::CacheInfo(dir),
    })
}

/// `al-runner warm [PATH.bak] [--bc-version V] [--country CODE] [--cache-dir DIR] [--no-download] [--test-data-company NAME]`.
fn parse_warm(mut it: impl Iterator<Item = String>) -> Result<Command, String> {
    let mut a = TestArgs {
        test_data: true,
        ..TestArgs::default()
    };
    while let Some(flag) = it.next() {
        let mut text = || it.next().ok_or_else(|| format!("{flag} needs a value"));
        match flag.as_str() {
            "--help" | "-h" => return Ok(Command::Help),
            "--bc-version" => a.bc_version = Some(text()?),
            "--country" => a.country = Some(al_deps::artifact::normalize_country(&text()?)?),
            "--cache-dir" => a.cache_dir = Some(text()?.into()),
            "--no-download" => a.no_download = true,
            "--test-data-company" => a.test_data_company = Some(text()?),
            f if !f.starts_with("--") && is_backup(f) => a.test_data_path = Some(PathBuf::from(f)),
            f => {
                return Err(format!(
                    "unknown option or argument `{f}` for `al-runner warm` (see --help)"
                ));
            }
        }
    }
    if a.test_data_path.is_none() && a.bc_version.is_none() {
        return Err("al-runner warm needs a backup (PATH.bak) or a build (--bc-version BUILD)".into());
    }
    Ok(Command::Warm(Box::new(a)))
}

/// `al-runner serve [--protocol 2] [OPTIONS]`: the options are those of `al-runner test` that do not depend on a request;
/// they are checked with its parser and handed to every run.
fn parse_serve(args: Vec<String>) -> Result<Command, String> {
    let mut daemon = Vec::new();
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--help" | "-h" => return Ok(Command::Help),
            "--protocol" => match it.next().as_deref() {
                Some("2") => {}
                other => {
                    return Err(format!(
                        "--protocol: only protocol 2 exists, not `{}`",
                        other.unwrap_or("(nothing)")
                    ));
                }
            },
            _ => daemon.push(a),
        }
    }
    // A throw-away parse tells a good option from a bad one, and keeps out what only a request decides.
    let mut probe = vec!["test".to_string(), "APP".to_string()];
    probe.extend(daemon.iter().cloned());
    let Command::Test(t) = parse(probe)? else {
        return Err("al-runner serve: options of `al-runner test` only".into());
    };
    let t = *t;
    if t.apps.len() != 1
        || !t.codeunits.is_empty()
        || !t.filters.is_empty()
        || !t.excludes.is_empty()
        || !t.exact.is_empty()
        || t.junit.is_some()
        || t.output_json.is_some()
        || t.coverage
        || t.explain.is_some()
        || t.isolation.is_some()
    {
        return Err("al-runner serve takes only the options that do not depend on a request: --package-cache, --dep, --define, --preprocessor-symbols, --test-timeout, --bc-version, --cache-dir, --no-download, --test-data and the like; the rest comes in each `runTests` request".into());
    }
    // Hard rule 4: every request is one plain test run (`run::test_with`). A recipe run (one run per lane) and an
    // impact run (a run without and one with the extensions) are other kinds of run, so they are refused here, not
    // ignored by every request.
    for (set, flag) in [
        (t.recipe, "--recipe microsoft"),
        (!t.impact_of.is_empty(), "--impact-of"),
    ] {
        if set {
            return Err(format!(
                "al-runner serve cannot be combined with {flag}: a request runs the tests once, the plain way; use `al-runner test {flag} ...`"
            ));
        }
    }
    Ok(Command::Serve(daemon))
}

pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Command, String> {
    let mut it = args.into_iter().peekable();
    let Some(first) = it.next() else {
        return Ok(Command::Help);
    };
    let mut index_build = false;
    let mut check_static = false;
    match first.as_str() {
        "--help" | "-h" | "help" => return Ok(Command::Help),
        "--version" | "-V" | "version" => return Ok(Command::Version),
        "--list-assumptions" => {
            // `--list-assumptions [--cache-dir DIR]`: the counts come from the last run that left them in the cache.
            let mut dir = None;
            while let Some(f) = it.next() {
                match f.as_str() {
                    "--cache-dir" => dir = it.next().map(PathBuf::from),
                    other => return Err(format!("unknown option `{other}` for --list-assumptions")),
                }
            }
            return Ok(Command::ListAssumptions(dir));
        }
        "test" => {}
        "impact-index" => match it.next().as_deref() {
            Some("build") => index_build = true,
            Some("check-static") => {
                index_build = true;
                check_static = true;
            }
            other => {
                return Err(format!(
                    "`al-runner impact-index` needs `build` or `check-static`, got `{}`",
                    other.unwrap_or_default()
                ));
            }
        },
        "cache" => return parse_cache(it),
        "warm" => return parse_warm(it),
        "serve" => return parse_serve(it.collect()),
        "run" | "debug" | "deps" | "package" | "helper" | "doctor" | "migrate" | "guide" => {
            return Err(format!(
                "`al-runner {first}` is not implemented yet (ADR-016 lists it for a later step); see --help for the commands that exist"
            ));
        }
        other => return Err(format!("unknown command `{other}` (see --help)")),
    }
    let mut a = TestArgs::default();
    while let Some(flag) = it.next() {
        if matches!(flag.as_str(), "--help" | "-h") {
            return Ok(Command::Help);
        }
        if flag == "--top-refusals" {
            // The count is optional: `--top-refusals` or `--top-refusals 5`.
            let n = match it.peek() {
                Some(v) if !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()) => {
                    let v = it.next().unwrap_or_default();
                    let n = number("--top-refusals", &v)?;
                    if n == 0 {
                        return Err("--top-refusals needs a count of at least 1".into());
                    }
                    n
                }
                _ => DEFAULT_TOP,
            };
            a.top_refusals = Some(n);
            continue;
        }
        if flag == "--list-assumptions" {
            return Ok(Command::ListAssumptions(a.cache_dir.clone()));
        }
        if flag == "--top-assumptions" {
            let n = match it.peek() {
                Some(v) if !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()) => {
                    let v = it.next().unwrap_or_default();
                    let n = number("--top-assumptions", &v)?;
                    if n == 0 {
                        return Err("--top-assumptions needs a count of at least 1".into());
                    }
                    n
                }
                _ => DEFAULT_TOP,
            };
            a.top_assumptions = Some(n);
            continue;
        }
        if flag == "--top-failures" {
            let n = match it.peek() {
                Some(v) if !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()) => {
                    let v = it.next().unwrap_or_default();
                    let n = number("--top-failures", &v)?;
                    if n == 0 {
                        return Err("--top-failures needs a count of at least 1".into());
                    }
                    n
                }
                _ => DEFAULT_TOP,
            };
            a.top_failures = Some(n);
            continue;
        }
        if flag == "--output-json" {
            // The file is optional and an APP is positional: only a name that ends in `.json` is the file.
            a.output_json = Some(match it.peek() {
                Some(v) if v.to_ascii_lowercase().ends_with(".json") => it.next().map(PathBuf::from),
                _ => None,
            });
            continue;
        }
        if let Some(path) = flag.strip_prefix("--output-json=") {
            a.output_json = Some(Some(PathBuf::from(path)));
            continue;
        }
        if flag == "--test-data" {
            // The path is optional and an APP is positional, so only a name that ends in a
            // backup's extension is taken as the path: `--test-data Tests.Source.zip` is an APP.
            a.test_data = true;
            if it.peek().is_some_and(|v| is_backup(v)) {
                a.test_data_path = it.next().map(PathBuf::from);
            }
            continue;
        }
        if let Some(path) = flag.strip_prefix("--test-data=") {
            a.test_data = true;
            a.test_data_path = Some(PathBuf::from(path));
            continue;
        }
        if !flag.starts_with("--") {
            a.apps.push(PathBuf::from(flag));
            continue;
        }
        let mut text = || it.next().ok_or_else(|| format!("{flag} needs a value"));
        match flag.as_str() {
            "--dep" => a.deps.push(text()?.into()),
            "--package-cache" => a.package_caches.push(text()?.into()),
            "--bc-version" => a.bc_version = Some(text()?),
            "--no-download" => a.no_download = true,
            "--no-test-data" => a.no_test_data = true,
            "--cache-dir" => a.cache_dir = Some(text()?.into()),
            "--no-compile-cache" => a.no_compile_cache = true,
            "--require-compile-cache" => a.require_compile_cache = true,
            "--rebuild-compile-cache" => a.rebuild_compile_cache = true,
            "--codeunit" => {
                let v = text()?;
                a.codeunits
                    .push(v.parse().map_err(|_| format!("{flag} needs an object id, got `{v}`"))?);
            }
            "--filter" | "--test" => a.filters.push(text()?),
            "--coverage" => a.coverage = true,
            "--impact-of" => a.impact_of.push(text()?.into()),
            "--impact" => a.impact = Some(text()?),
            "--impact-accept" => a.impact_accept = Some(text()?.into()),
            "--impact-json" => a.impact_json = Some(text()?.into()),
            "--coverage-out" => {
                a.coverage = true;
                a.coverage_out = Some(text()?.into());
            }
            "--coverage-json" => {
                a.coverage = true;
                a.coverage_json = Some(text()?.into());
            }
            "--test-exact" => a.exact.push(text()?),
            "--exclude-test" => a.excludes.push(text()?),
            "--define" => a.defines.push(text()?),
            "--preprocessor-symbols" => a.defines.extend(
                text()?
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(String::from),
            ),
            "--auto-provision" => {}
            "--isolation" | "--test-isolation" => {
                let v = text()?;
                a.isolation = Some(match v.to_ascii_lowercase().as_str() {
                    "codeunit" => al_testrun::isolation::TestIsolation::Codeunit,
                    "test" | "function" | "method" => al_testrun::isolation::TestIsolation::Function,
                    "disabled" => al_testrun::isolation::TestIsolation::Disabled,
                    _ => return Err(format!("{flag} needs codeunit, test or disabled, got `{v}`")),
                });
            }
            "--color" => {
                let v = text()?;
                a.color = crate::style::ColorMode::parse(&v)
                    .ok_or_else(|| format!("--color needs auto, always or never, got `{v}`"))?;
            }
            "--refusal-detail" => a.refusal_detail = Some(text()?),
            "--junit" => a.junit = Some(text()?.into()),
            "--explain" => a.explain = Some(text()?),
            "--allow-not-run" => a.allow_not_run = true,
            "--verbose" | "-v" => a.verbose = true,
            "--quiet" | "-q" => a.quiet = true,
            "--timings" => a.timings = true,
            "--files-dir" => a.files_dir = Some(text()?.into()),
            "--environment" => a.environment = Some(text()?.parse().map_err(|e| format!("--environment: {e}"))?),
            "--server-url" => {
                a.server_url = Some(al_host::check_server_url(&text()?).map_err(|e| format!("--server-url: {e}"))?)
            }
            "--dotnet-sidecar" => a.dotnet_sidecar = Some(text()?.into()),
            "--dotnet-assembly-dir" => a.dotnet_assembly_dirs.push(text()?.into()),
            "--dotnet-allow-roll-forward" => a.dotnet_allow_roll_forward = true,
            "--require-dotnet" => a.require_dotnet = true,
            "--dotnet-timeout-secs" => {
                let v = text()?;
                a.dotnet_timeout_secs = Some(number(&flag, &v)? as u64);
            }
            "--test-timeout-secs" | "--test-timeout" => {
                let v = text()?;
                a.test_timeout_secs = Some(number(&flag, &v)? as u64);
            }
            "--test-data-company" => a.test_data_company = Some(text()?),
            "--country" => a.country = Some(al_deps::artifact::normalize_country(&text()?)?),
            "--company-data" => {
                let v = text()?;
                match v.as_str() {
                    "bak" | "backup" => a.company_package = false,
                    "package" | "rapidstart" => a.company_package = true,
                    other => return Err(format!("--company-data takes `bak` or `package`, not `{other}`")),
                }
            }
            "--package" => {
                a.package_path = Some(text()?.into());
                a.company_package = true;
            }
            "--disabled-tests" => a.disabled_tests_dir = Some(text()?.into()),
            "--parity" => a.parity = true,
            "--recipe" => {
                let v = text()?;
                if v != "microsoft" {
                    return Err(format!("--recipe takes `microsoft`, not `{v}`"));
                }
                a.recipe = true;
            }
            "--bcapps" => a.bcapps_dir = Some(text()?.into()),
            "--test-data-normalize-company" => {
                return Err(format!(
                    "{flag} is not available: AL Runner v2's company normalization rules are not ported (ADR-020)"
                ));
            }
            "--tdd" => a.tdd = true,
            "--watch" | "--affected" | "--per-suite" | "--include-failing" | "--strict-environment" => {
                return Err(format!(
                    "{flag} is deferred in v3 (ADR-016, owner direction 2026-10-13): features that only hide start-up or run cost wait until they are measured to matter"
                ));
            }
            _ => return Err(format!("unknown option `{flag}` (see --help)")),
        }
    }
    if a.apps.is_empty() {
        return Err("al-runner test needs at least one APP (an app folder or a *.Source.zip)".into());
    }
    if a.rebuild_compile_cache && a.no_compile_cache {
        return Err("--rebuild-compile-cache and --no-compile-cache cannot be combined".into());
    }
    if a.rebuild_compile_cache && a.require_compile_cache {
        return Err("--rebuild-compile-cache and --require-compile-cache cannot be combined".into());
    }
    if a.require_compile_cache && a.no_compile_cache {
        return Err("--require-compile-cache and --no-compile-cache cannot be combined".into());
    }
    if a.no_test_data && a.test_data_asked_for() {
        return Err("--test-data and --no-test-data cannot be combined".into());
    }
    if a.no_test_data && a.test_data_company.is_some() {
        return Err("--test-data-company cannot be combined with --no-test-data".into());
    }
    if a.parity && a.disabled_tests_dir.is_none() {
        return Err("--parity needs --disabled-tests DIR".into());
    }
    if a.company_package && !al_deps::artifact::is_w1(a.country()) {
        return Err("--company-data package supports only the w1 country (its RapidStart package comes from the platform artifact)".into());
    }
    if a.company_package && a.no_test_data {
        return Err("--company-data package cannot be combined with --no-test-data".into());
    }
    if a.bcapps_dir.is_some() && !a.recipe {
        return Err("--bcapps needs --recipe microsoft".into());
    }
    if a.recipe {
        for (given, flag) in [
            (a.company_package, "--company-data package / --package"),
            (a.test_data_asked_for(), "--test-data"),
            (a.no_test_data, "--no-test-data"),
            (a.disabled_tests_dir.is_some(), "--disabled-tests"),
            (a.parity, "--parity"),
            (a.test_data_company.is_some(), "--test-data-company"),
        ] {
            if given {
                return Err(format!(
                    "--recipe microsoft decides the company and the disabled tests itself: {flag} cannot be combined with it"
                ));
            }
        }
        if a.explain.is_some() {
            return Err("--explain cannot be combined with --recipe microsoft; explain one test with --codeunit and the lane's company options".into());
        }
    }
    if a.verbose && a.quiet {
        return Err("--verbose and --quiet cannot be combined".into());
    }
    if index_build {
        if !a.impact_of.is_empty() || a.impact.is_some() || a.impact_accept.is_some() || a.impact_json.is_some() {
            return Err("al-runner impact-index build runs the tests without any extension: --impact-of, --impact, --impact-accept and --impact-json do not apply".into());
        }
        if a.no_compile_cache {
            return Err("al-runner impact-index build cannot be combined with --no-compile-cache: impact runs compile the dependency apps completely from the compile cache, and so must the index's run (ADR-043 section 2.2)".into());
        }
        for (set, flag) in [
            (a.explain.is_some(), "--explain"),
            (a.coverage, "--coverage, --coverage-out and --coverage-json"),
            (a.execute, "a scratch run"),
            (a.recipe, "--recipe microsoft"),
            (a.tdd, "--tdd"),
        ] {
            if set {
                return Err(format!("al-runner impact-index build cannot be combined with {flag}"));
            }
        }
        return Ok(if check_static {
            Command::ImpactIndexCheckStatic(Box::new(a))
        } else {
            Command::ImpactIndexBuild(Box::new(a))
        });
    }
    impact_args(&a)?;
    Ok(Command::Test(Box::new(a)))
}

/// The checks of an impact run's options (ADR-043).
fn impact_args(a: &TestArgs) -> Result<(), String> {
    // An impact run compares Microsoft's tests with and without an extension: nothing to stub (ADR-051).
    if a.tdd && !a.impact_of.is_empty() {
        return Err("--tdd cannot be combined with --impact-of: an impact run compares the tests with and without an extension, and has nothing to stub".into());
    }
    if a.impact_of.is_empty() {
        for (set, flag) in [
            (a.impact.is_some(), "--impact"),
            (a.impact_accept.is_some(), "--impact-accept"),
            (a.impact_json.is_some(), "--impact-json"),
        ] {
            if set {
                return Err(format!(
                    "{flag} needs --impact-of APP: the extension whose impact the run measures"
                ));
            }
        }
        return Ok(());
    }
    match a.impact.as_deref() {
        None | Some("all" | "dynamic" | "static") => {}
        Some(other) => return Err(format!("--impact needs all, dynamic or static, got `{other}`")),
    }
    if a.no_compile_cache {
        return Err("--impact-of cannot be combined with --no-compile-cache: both sides must compile the dependency apps completely from the compile cache, never only what the tests reach (ADR-043 section 2.2)".into());
    }
    if matches!(a.output_json, Some(None)) {
        return Err("--impact-of needs a FILE for --output-json: the impact report goes to stdout, so the JSON document cannot (`--output-json FILE.json`; --impact-json FILE writes the diff itself)".into());
    }
    for (set, flag) in [
        (a.explain.is_some(), "--explain"),
        (a.coverage, "--coverage, --coverage-out and --coverage-json"),
        (a.execute, "a scratch run"),
    ] {
        if set {
            return Err(format!(
                "--impact-of cannot be combined with {flag}: an impact run counts the extension's statements for its own report and runs two sides"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> Result<Command, String> {
        parse(s.split_whitespace().map(String::from))
    }

    fn test_args(s: &str) -> TestArgs {
        match p(s).unwrap() {
            Command::Test(a) => *a,
            other => panic!("{other:?}"),
        }
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
        assert_eq!(
            test_args("test A --test-isolation disabled").isolation,
            Some(I::Disabled)
        );
        assert!(
            p("test A --isolation nope")
                .unwrap_err()
                .contains("codeunit, test or disabled")
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
}
