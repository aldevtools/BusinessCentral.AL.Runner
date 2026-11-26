# Code map

Read this before you open a source file. One line per module: the path, what it owns, the entry point.
The full description of a module is the `//!` header at the top of its file. Read that header before you
change the file. Call chains of the main flows: `docs/flows.md`. Test layout and the ratchets:
`docs/agents/workflow.md`.

A test (`crates/al-test-cli/tests/suite/codemap.rs`) fails when a listed file or directory no longer
exists, and when this file passes 40,000 bytes. A new module gets one line here, short; detail goes in its
`//!` header, never here. Line numbers drift, so none are listed: find an entry with `rust-analyzer`
(`documentSymbol`, `workspaceSymbol`) or `grep -n "fn name"`.

Line format: `- \`path\`: what it owns. Entry: \`symbol\`.` A directory line ends in `/`, counts its files and
lists them after `Files:`.

## Layers

Dependencies point down. Core crates do no OS access (ADR-006); the host seam is `al-host`.

```
al-test-cli    corpus job: oracle, corpus, not-passing lists, pass-rate report           alrunner-cli  user command line `al-runner test`
al-testrun     test runner (runtime), install baseline, dependency loading, sidecar host, explain, refusal ranking
al-vm          interpreter over the IR: Engine (immutable), Session (all mutable state), test client
al-sema        workspace -> members -> check (+ demand) -> lower; produces al-ir; takes dependency objects from a compile cache (provided.rs)
al-cache       the compile cache file (ADR-022): key, per-object zstd blocks, index; bytes in, bytes out
al-builtins    values, builtin functions by catalog overload id, filters, JSON/XML, platform table registry
al-data        Store trait and the in-memory store         al-host   clock, identity, files, DotNet seam
al-ir          the typed IR (plain data)                   al-version  BC versions, run target, gates
al-syntax      lexer, preprocessor, parser, rowan tree, AST views, cheap declaration scan
al-deps        dependency apps: .alpackages, Microsoft artifact feed, cache, package/zip reading
al-testkit     dev only: compile AL from a string and run a routine     al-testfiles  dev only: Microsoft's files of the pinned build, for tests
```

## Crates and modules
### al-bcdb

- `bacpac.rs`: The `.bacpac` container (port of `Bacpac.cs`). Entry: `BacpacTable`.
- `bak.rs`: The SQL Server backup path. Entry: `BakSource`.
- `bcp.rs`: Reads rows out of a bacpac's native-BCP data stream. Entry: `BacpacColumn`.
- `catalog.rs`: `to_index_bytes`/`from_index_bytes`. Entry: `PagePtr`.
- `error.rs`: Errors. Entry: `Error`.
- `io.rs`: The byte source the readers work on. Entry: `ByteSource`.
- `lib.rs`: Reads Business Central data straight out of a SQL…
- `lob.rs`: Resolves off-row values. Entry: `LobReader`.
- `model.rs`: Reading `model.xml`. Entry: `ModelScan`.
- `mtf.rs`: Structural parser for the Microsoft Tape Format… Entry: `DataRegion`.
- `pagefile.rs`: `IndexStore`, `open_indexed`. Entry: `Page`.
- `query.rs`: `plan_read_table_in`. Entry: `BcTable`.
- `records.rs`: Record parsers (port of `Records.cs`). Entry: `record_type`.
- `scsu.rs`: Decoder for SCSU, Unicode Technical Standard #6. Entry: `decode`.
- `serve.rs`: Serve mode (port of `Program.Serve`). Entry: `serve`.
- `source.rs`: The query surface both containers implement. Entry: `SourceTable`.
- `symbols.rs`: The AL meaning layer (port of `Symbols.cs`). Entry: `AlField`.
- `types.rs`: Storage-form cells to typed values. Entry: `SysColumn`.
- `value.rs`: Decoded values. Entry: `Value`.
- `zip.rs`: A zip central-directory reader over a [`ByteSource`]. Entry: `ZipEntry`.

### al-bcdb-cli

- `cmd.rs`: The al-bcdb commands. Entry: `run`.
- `main.rs`: The al-bcdb binary: reads BC database backups. Entry: `main`.
- `opts.rs`: Option parsing, strict the way bcdb's is. Entry: `command_keys`.

### al-bcdb-std

- `file.rs`: A file as a byte source, the OS side of `al-bcdb`. Entry: `FileSource`.
- `lib.rs`: The OS side of `al-bcdb`.
- `provider.rs`: `BackupProvider` (`open_with_index_dir` keeps… Entry: `BackupProvider`.

### al-cache

- `format.rs`: The cache entry file: header, dictionary. Entry: `build`.
- `index.rs`: The index entry: the scan of the dependency sources. Entry: `build_index`.
- `key.rs`: The key of an entry: every input of the compile… Entry: `Key`.
- `lib.rs`: Crate root of the compile cache format (ADR-022).

### al-builtins

- `catalog.rs`: Generating the signature catalog from the docs scrape. Entry: `Catalog`.
- `collation.rs`: Text order for `<`, `>`, `<=` and `>=`. Entry: `compare`.
- `collections.rs`: List and Dictionary methods.
- `describe.rs`: How a value or type reads in a refusal reason… Entry: `Describe`.
- `refusal.rs`: Typed refusals: `Refusal` (a key and a text. Entry: `Refusal`.
- `datatransfer.rs`: DataTransfer (install and upgrade code only). Entry: `native_for`.
- `datefns.rs`: Date, Time and DateTime builtins. Entry: `formula_text`.
- `datefns/brackets.rs`: Angle brackets inside a term of a date formula. Entry: `open`.
- `datefns/numbers.rs`: A formula that is only a number above 9999. Entry: `too_large`.
- `datefns/range.rs`: CalcDate at the ends of the date range. Entry: `low_base`.
- `autoformat11.rs`: `Format` in code of an AutoFormatType 11 field… Entry: `format`.
- `errorinfo.rs`: ErrorInfo: the property bag, Error, collected errors. Entry: `raise`.
- `features.rs`: `catalog/alc-features.txt` (what the AL compiler… Entry: `feature`.
- `evaluate.rs`: `Evaluate(var Variable, String [, Number])`. Entry: `evaluate`.
- `evaluate/short_us_date.rs`: `Evaluate` into a Date or DateTime local or parameter. Entry: `measured`.
- `evaluate/option_blank.rs`: `Evaluate` of a blank text into an Option field with an OptionCaption: which member it names. Entry: `blank_text`.
- `file.rs`: The `File` data type, on the host file system capability. Entry: `FileState`.
- `filter.rs`: Record filters as data. Entry: `FilterExpr`.
- `filter_at_sign.rs`: `@` alone and `<>@` alone answer as BC reads the rows. Entry: `AtSignRows`.
- `filter_collation.rs`: Where a filter's answer for a text rests on `Æ`/`æ`… Entry: `collation_refusal`.
- `format.rs`: `Format(Value [, Length] [, FormatNumber])`. Entry: `FormatCtx`.
- `format/length.rs`: What a length does in `Format`.
- `format_json.rs`: `Format` of a JsonObject, JsonArray, JsonValue or JsonToken. Entry: `format_json`.
- `formatstr.rs`: `Format(Value, Length, FormatString)`. Entry: `format_with_string`.
- `http/mod.rs`: HttpClient, HttpRequestMessage, HttpResponseMessage. Entry: `native_for`.
- `iso.rs`: ISO 8601 text to AL date/time values. Entry: `days_in_month`.
- `json.rs`: JsonObject, JsonArray, JsonToken and JsonValue. Entry: `Handle`.
- `json/yaml.rs`: `JsonObject.WriteToYaml` and `ReadFromYaml`. Entry: `read_from_yaml`.
- `lib.rs`: Crate root of the AL standard library: values.
- `decimal_scale.rs`: The decimals a Decimal shows where its scale is seen. Entry: `shown`.
- `mathfns.rs`: Round and Power. Entry: `round`.
- `media.rs`: `Media` and `MediaSet` fields (corpus media/). Entry: `Id`.
- `media_export.rs`: Which media `Media.ExportStream` gives back byte for byte. Entry: `exports_unchanged`.
- `media_word_package.rs`: What the ImportFile and ImportStream of Media… Entry: `media_import_file`.
- `moduleinfo.rs`: `NavApp.GetCurrentModuleInfo` and friends. Entry: `native_for`.
- `native/mod.rs`: Native builtin implementations.
- `ops.rs`: Value operations: conversion, arithmetic, comparison. Entry: `convert`.
- `ops/quotient.rs`: The one quotient rule of `/` for a Decimal, an Integer. Entry: `whole_quotient`.
- `option_members.rs`: The one test for an Option member list declared…
- `closing_date_days.rs`: A closing date plus or minus a whole number. Entry: `closing_date_moved`.
- `date_difference.rs`: The difference of two Dates or DateTimes…
- `conv/`: Implicit conversions at run time from the run…
- `conv/variant_records.rs`: A Variant holding a RecordId, Record or RecordRef. Entry: `record_slot_code`.
- `conv/text_constant.rs`: What a reader gets from a Label or TextConst… Entry: `read`.
- `page_handle.rs`: The opaque handles `Value::TestPage`… Entry: `PageHandle`.
- `pagebuiltins.rs`: `TestPage`, `TestPart`, `TestField`.
- `pagefmt.rs`: The text a page control shows for a value, en-US. Entry: `Blank`.
- `platform.rs`: The registry of platform tables that have a native… Entry: `Provider`.
- `query_reopen_step.rs`: `ReopenStep`, the step a query variable reached…
- `querybuiltins.rs`: `Query` variable methods.
- `refs.rs`: `RecordRef`, `FieldRef` and `KeyRef` methods.
- `recordref_closed.rs`: A RecordRef that is not open, never opened or closed. Entry: `not_open`.
- `sessionfns.rs`: Builtins that act on session state.
- `sessiontypes.rs`: `SessionSettings` and `Notification`. Entry: `native_for`.
- `sqlcount.rs`: The taint of `Value::SqlCount`. Entry: `arith`.
- `signature.rs`: Parsing the `syntax` line of a documented builtin… Entry: `Param`.
- `stream.rs`: In-memory streams over Blob fields. Entry: `BlobBuf`.
- `blob_form.rs`: Every field of a Blob buffer in one place. Entry: `put`.
- `placeholder_output.rs`: The mark of a placeholder report output. Entry: `mark`.
- `rendered_text.rs`: Rendered Html content in a Text value.
- `sysenum.rs`: Platform option types whose values AL writes… Entry: `members`.
- `textfns.rs`: Text functions and methods, TextBuilder and BigText. Entry: `secret_or_text`.
- `instance_cycles.rs`: Frees the object instances only a reference cycle…
- `record_fields.rs`: `Fields`, a record's field buffer. Entry: `Fields`.
- `value.rs`: Runtime values (ADR-010). Entry: `Value`.
- `version.rs`: The Version type. Entry: `native_for`.
- `wsac.rs`: `WebServiceActionContext`. Entry: `native_for`.
- `xml/mod.rs`: XmlDocument, XmlElement, XmlAttribute, XmlText. Entry: `native_for`.
- `xmlport.rs`: The document side of XmlPort import and export. Entry: `Elem`.

### al-data

- `contract.rs`: The conformance suite every `Store` must pass. Entry: `table`.
- `lib.rs`: The Store trait and table definitions. Entry: `Store`.
- `memory.rs`: In-memory store on persistent maps. Entry: `MemStore`.
- `memory/` (8 files): In-memory store on persistent maps. Entry: `MemStore`. Files: `key.rs`, `held_cols.rs`, `alt_cache.rs`, `band.rs`, `footprint.rs`, `order_carry.rs`, `source_stamps.rs`, `shape.rs`

### al-deps

- `artifact.rs`: Dependency apps from Microsoft's artifact feed. Entry: `Cdn`.
- `build.rs`: Which Business Central build supplies the Microsoft apps. Entry: `BuildId`.
- `test_apps.rs`: The Microsoft test apps of a build. Entry: `list`.
- `demo.rs`: The demo database backup of a build. Entry: `ensure_demo_backup`.
- `cache.rs`: The artifact cache (ADR-013 decision 3). Entry: `cache_root`.
- `lib.rs`: Dependency apps for the runtime (ADR-013).
- `license.rs`: The license that ships with the BC artifacts… Entry: `artifact_license`.
- `service_tier.rs`: The BC service-tier assemblies a DotNet type needs. Entry: `ServiceTier::ensure`.
- `service_runtime.rs`: The `Microsoft.NETCore.App` major a build's service… Entry: `netcore_major`.
- `render_set.rs`: What the report render process needs.
- `assembly_refs.rs`: The `AssemblyRef` names of a .NET assembly. Entry: `assembly_refs`.
- `emit_version.rs`: The emit version of a BC build. Entry: `emit_version`.
- `lazy.rs`: A package whose files are not read yet. Entry: `read_lazy`.
- `load.rs`: Finding, choosing and reading the dependency apps… Entry: `parse_app_json`.
- `package.rs`: Reading an .app package: manifest, sources, resources. Entry: `Manifest`.
- `remote.rs`: A ZIP archive read by byte ranges. Entry: `RemoteError`.
- `select.rs`: Which package satisfies each dependency. Entry: `DepDecl`.
- `version.rs`: App version numbers. Entry: `Version`.
- `zip.rs`: A small ZIP reader over a byte slice. Entry: `ZipError`.

### al-host

- `inproc.rs`: In-process stand-ins for a closed set of .NET… Entry: `wrap`.
- `inproc/math.rs`: Exact decimal `System.Math` members and `TimeSpan.FromTicks`. Entry: `math`.
- `inproc/regex.rs`: Which `Regex` constructions may be built again… Entry: `key`.
- `dotnet.rs`: The `DotNet` provider seam (ADR-014 section 2). Entry: `DotNetType`.
- `render.rs`: The report renderer seam: `ReportRenderer`.
- `lib.rs`: The host seam: clock, identity, client type, file system. Entry: `Clock`.
- `testdata.rs`: Test data capability: `TableProvider`, `Cell`. Entry: `TableProvider`.
- `license.rs`: A BC license file: `parse`. Entry: `parse`.
- `random.rs`: .NET's seeded `System.Random` for `Random` and `Randomize`. Entry: `NetRandom`.
- `seed.rs`: The run seed's derivations per test method: `derive`.

### al-ir

- `body.rs`: `Body`, the statements of a routine. Entry: `Body`.
- `head.rs`: `ObjectHead`, what a pass over all objects reads… Entry: `ObjectHead::of`.
- `lib.rs`: The typed IR: Program, Object, Routine, Stmt, Expr. Entry: `Program`.
- `sym.rs`: The symbolic form of an object: the keys. Entry: `ObjKey`.
- `walk.rs`: Every reference to another object, routine. Entry: `walk_object`.

### al-link

- `lib.rs`: The link step: symbolic objects in, the dense `Program` out. Entry: `link`.

### al-sema

- `alcodes.rs`: Compile errors with the AL compiler's own codes.
- `alcodes/` (6 files): Compile errors with the AL compiler's own codes. Entry: `not_checked_reason`. Files: `enum_variables.rs`, `calc_filter_at_sign.rs`, `keyword_member_names.rs`, `keyword_section_names.rs`, `keyword_option_members.rs`, `keyword_variable_names.rs`
- `features.rs`: AL0666 for a property, data type. Entry: `check`.
- `property_values.rs`: AL0169, a property set to a name that is none of its values. Entry: `check`.
- `base_access.rs`: Page and report extension code that names the base… Entry: `PINNED`.
- `calcformula.rs`: The `CalcFormula` property of a FlowField, as syntax. Entry: `Formula`.
- `check/` (14 files): Resolves names, member access and calls. Entry: `check_with`. Files: `mod.rs`, `tdd.rs`, `tdd_decl.rs`, `posmap.rs`, `object.rs`, `infer.rs`, `calls.rs`, `member.rs`, `protected_member.rs`, `fit.rs`, `conv.rs`, `field_enum.rs`, `diag_index.rs`, `demand.rs`
- `lib.rs`: Crate root of the semantic model.
- `lower/` (23 files): Lowers resolved syntax to the typed IR (ADR-011). Entry: `Lowered`. Files: `mod.rs`, `assemble.rs`, `ext_order.rs`, `ext_request_page.rs`, `localize.rs`, `routine_lists.rs`, `shared.rs`, `tables.rs`, `table_no_rec.rs`, `pages.rs`, `page_client_expr.rs`, `convert.rs`, `copy_array.rs`, `conv_matrix.rs`, `stmt.rs`, `expr.rs`, `dotnet.rs`, `with_events.rs`, `call.rs`, `evaluate_id.rs`, `var_option_arg.rs`, `compile.rs`, `extend.rs`
- `members.rs`: Members of objects: table fields, enum values. Entry: `MemberTable`.
- `provided.rs`: Dependency objects whose lowered form comes… Entry: `DepSet`.
- `own_named_page_methods.rs`: A page method named like one of the page's own… Entry: `save_record_names_platform`.
- `pages.rs`: Pages for the test client (ADR-012). Entry: `expand`.
- `queries.rs`: Query objects (corpus query/). Entry: `QueryLayout`.
- `relation.rs`: `TableRelation` property text → arms. Entry: `ParsedArm`.
- `reports.rs`: Report objects: dataset walk, columns. Entry: `layout`.
- `types.rs`: Semantic types (Ty) and their display names. Entry: `Ty`.
- `workspace.rs`: A set of apps: parse every file. Entry: `Workspace`.
- `workspace/moved_tables.rs`: A workspace table kept as the… Entry: `moved_table_of_dependency`.
- `workspace/subscriber_publisher.rs`: The object an `[EventSubscriber]` names… Entry: `subscriber_publisher`.
- `xmlports.rs`: XmlPort objects (Format = Xml). Entry: `RawNode`.

### al-syntax

- `ast.rs`: Typed views over the untyped tree. Entry: `routines_in`.
- `builder.rs`: Green tree construction for the parser. Entry: `TreeCache`.
- `grammar/` (5 files): AL grammar: declarations, expressions, objects, statements. Files: `declarations.rs`, `expressions.rs`, `mod.rs`, `objects.rs`, `statements.rs`
- `lexer.rs`: Lossless lexer. Entry: `lex`.
- `lib.rs`: Crate root of the AL syntax: lexer, parser, tree, AST views.
- `parser.rs`: Parser infrastructure and the parse entry points. Entry: `parse`.
- `preprocessor.rs`: Token-level preprocessor (#if and friends). Entry: `preprocess`.
- `splice.rs`: Adds the routines the compiler generates… Entry: `splice_routines`.
- `scan.rs`: A cheap declaration scan: objects, ids, subscribers. Entry: `scan`.
- `syntax_kind.rs`: Every token and node kind. Entry: `SyntaxKind`.

### al-test-cli

- `args.rs`: Command-line parsing and the --help text. Entry: `parse`.
- `awaiting.rs`: `--awaiting-assumptions [--summary]`. Entry: `with_extra`.
- `config.rs`: Oracle/oracle.toml: versions, suites, corpus pin. Entry: `OracleFile`.
- `corpus.rs`: Corpus discovery: test codeunits, their area folder and file. Entry: `discover`.
- `coverage.rs`: The corpus coverage map: which builtin overloads… Entry: `collect_calls`.
- `behavior_coverage.rs`: The behavior coverage map: triggers, properties. Entry: `collect`.
- `cell_report.rs`: The cell report: per test that uses the observer… Entry: `build`.
- `language_catalog.rs`: The AL language catalog `docs/language-catalog.json`.
- `drift.rs`: Comparing case sets across BC versions (ADR-015 3.6). Entry: `differences`.
- `mnc.rs`: The must-not-compile corpus: parse a case header. Entry: `parse_case`.
- `insight.rs`: --top-refusals (folded reason ranking). Entry: `top_refusals`.
- `refusal_keys.rs`: The refusal key lists of every crate that refuses. Entry: `LISTS`.
- `not_passing.rs`: ADR-065, the lists of the cases the runtime does… Entry: `check`.
- `base_oracle.rs`: The base branch's oracle and lists. Entry: `Base`.
- `lib.rs`: Runs the corpus's `[Test]` codeunits and reports…
- `main.rs`: The binary: run each oracle version, report. Entry: `main`.
- `oracle.rs`: The oracle: JUnit and xUnit cases of real BC, identified by. Entry: `CaseId`.
- `output.rs`: Renders a report as Markdown and the runtime's… Entry: `markdown`.
- `pin.rs`: The per-version corpus pin check. Entry: `check`.
- `report.rs`: Joins the oracle with the runtime's outcomes into… Entry: `Outcome`.
- `runner.rs`: The seam between the harness and the runtime. Entry: `Runner`.
- `select.rs`: Test selection: --codeunit, --test, --area, --file. Entry: `Selection`.
- `strict_order.rs`: The corpus order probes…

### al-testrun

- `affected.rs`: `--affected`: the stored result of the last run… Entry: `Stored`.
- `baseline.rs`: The install baseline: the dependency apps' install… Entry: `BaselineRun`.
- `baseline_roots.rs`: One root of the dependency baseline. Entry: `RootRun`.
- `company_data.rs`: A company built by a dependency codeunit… Entry: `CompanyData`.
- `compile_cache.rs`: The OS side of the compile cache: the key of a run. Entry: `CompileCache`.
- `cache_gc.rs`: Cleanup of the compile cache folder. Entry: `prune`.
- `crash.rs`: Names the test a crashed process was running. Entry: `install`.
- `deps.rs`: Dependency apps for a corpus run (ADR-013). Entry: `Dependencies`.
- `discover.rs`: Finds `Subtype = Test` codeunits and their `[Test]`… Entry: `test_codeunits`.
- `dotnet.rs`: The .NET sidecar host: one process per session. Entry: `SidecarConfig`.
- `dotnet_runtime.rs`: Which `Microsoft.NETCore.App` the sidecar runs on. Entry: `ServiceRuntime`.
- `emit_version.rs`: The emit version of the run's build for al-vm… Entry: `of_build`.
- `report_render.rs`: The report render process. Entry: `RenderHost`.
- `report_render_pieces.rs`: The `PrepareFn` of a run. Entry: `prepare`.
- `explain.rs`: --explain: where a test was refused or failed. Entry: `describe`.
- `host.rs`: The host the command line gives a session (ADR-006), and the clock of a run. Entry: `SystemClock`, `RunClock`.
- `impact_trace.rs`: The impact index's keys, their numbering. Entry: `keys`.
- `install.rs`: The install code of each app. Entry: `plan`.
- `install_orders.rs`: The install order of apps with no dependency path: Microsoft's first, partner groups recorded and proven. Entry: `split_microsoft`, `install_partner_apps`.
- `install_proof.rs`: The influence graph of a partner group's installs and its units. Entry: `Graph::of`.
- `install_subsets.rs`: The runs that prove a group's install independent of order: per order of a unit's apps and per place of its inputs among them. Entry: `Proof::settle`.
- `impact_static.rs`: Static selection: the graph of routines, objects. Entry: `Graph`.
- `login.rs`: The test session's login. Entry: `decide`.
- `login_replay.rs`: The check that a replay of the raised login would… Entry: `Key::of`, `StartKey` (the key with the texts the login read).
- `login_replay/` (3 files): What the login replay check's report names: where two states differ, and the parts a new key differs in, never a value. Entry: `differences`, `KeyParts::of`. Files: `difference.rs`, `key_parts.rs`, `rows_digest.rs`
- `login_start.rs`: Replaying the raised login from a kept start (opt-in). Entry: `replay`.
- `partner_login.rs`: The partner dependency apps' "System… Entry: `subscribers`.
- `license.rs`: Which BC license a run uses: `--license FILE`. Entry: `LicenseUse`.
- `isolation.rs`: TestIsolation of the runner codeunit. Entry: `TestIsolation`.
- `isolation/failed_test.rs`: What a failed test under TestIsolation = Codeunit… Entry: `CodeunitSequences`.
- `lib.rs`: Running test codeunits and reading the result. Entry: `Outcome`.
- `install_reach.rs`: Whether an incremental compile of `al-runner serve`… Entry: `before`.
- `install_rows.rs`: The rows the install code of an app under test may… Entry: `install_rows`.
- `observe.rs`: Whether a selected test can observe the install… Entry: `may_observe_baseline`.
- `profiling/mod.rs`: `alcpuprofile.rs`, `summary.rs`, `write.rs`. Entry: `start`.
- `reach.rs`: The reach report: which roots and edges pull… Entry: `report`.
- `refusals.rs`: Ranking not-run reasons with names and numbers folded. Entry: `normalise_reason`.
- `stability.rs`: Stability mode: the config (`parse_config`. Entry: `parse_config`.
- `stopped_state.rs`: What code the runner stopped leaves of the session… Entry: `roll_back_refused_test`.
- `test_suites.rs`: The `--test-suites` file. Entry: `TestSuites`.
- `runtime/` (28 files): Runs test codeunits the way BC's test runner does. Entry: `CorpusRuntime`. Files: `mod.rs`, `compiled.rs`, `baseline_key.rs`, `grow.rs`, `corpus.rs`, `chain.rs`, `carry_after.rs`, `discovery.rs`, `chain_walk.rs`, `strict_walk.rs`, `single_lane.rs`, `narrowing.rs`, `not_run_link.rs`, `single_fresh.rs`, `order_hint.rs`, `memory.rs`, `compile_halt.rs`, `threads.rs`, `install_once.rs`, `cache.rs`, `run.rs`, `data_driven.rs`, `test_handlers.rs`, `written.rs`, `runner_reset.rs`, `handlers.rs`, `blocked.rs`, `table_reads.rs`
- `target.rs`: Reads the version fields of an app.json. Entry: `build_query`.

### alrunner-cli

- `affected.rs`: `al-runner test --affected` and serve `affectedOnly`. Entry: `Affected::prepare`.
- `apps.rs`: Reads the APPs given on the command line and builds… Entry: `load`.
- `args.rs`: Command-line parsing and the `--help` text of `al-runner`. Entry: `parse`.
- `demotool.rs`: `--company-data demotool`. Entry: `resolve`.
- `bcapps.rs`: Where the Microsoft recipe files come from. Entry: `resolve`.
- `path_note.rs`: The note about another `al-runner` on PATH. Entry: `maybe_note`.
- `service_tier.rs`: The .NET sidecar's probe folders and fetch of a run. Entry: `dotnet_dirs`.
- `bcapps_apps.rs`: The test apps Microsoft's CI runs that the artifact… Entry: `project_folders`.
- `recipe.rs`: BCApps' settings as lanes. Entry: `Recipe::load`.
- `recipe_run.rs`: `al-runner test --recipe microsoft`. Entry: `test`.
- `sources.rs`: Test sources (ADR-052). Entry: `test_microsoft`.
- `matrix.rs`: Matrix runs: `cells`, `stability_cells`. Entry: `run`.
- `sources_report.rs`: The exit code shared by runs of several parts (`worst`.
- `stability.rs`: `al-runner test --stability`. Entry: `plan`.
- `stability_si.rs`: The order dependence part of a stability report.
- `memory_budget.rs`: `--max-memory` and serve `maxMemory`: `parse_size`. Entry: `budget`.
- `test_suites.rs`: `--test-suites FILE` and serve `testSuites`. Entry: `load`.
- `testdata.rs`: Which demo backup a run restores: disk, cache. Entry: `locate`.
- `debug/mod.rs`: `debug/wire.rs`, `debug/breakpoints.rs`. Entry: `debug::run`.
- `impact.rs`: `al-runner test --impact-of APP`. Entry: `run`.
- `impact_index.rs`: The impact index: format, cache key and path. Entry: `Index`.
- `run_inputs.rs`: The inputs of a run, classified once.
- `main.rs`: The `al-runner` binary: runs on a thread… Entry: `main`.
- `profile.rs`: `--profile PATH`, `--profile-mode`. Entry: `Flags`.
- `output.rs`: Counts, exit codes, the summary line. Entry: `exit_code`.
- `run.rs`: `al-runner test`: load APPs, select test codeunits. Entry: `open_backup`.
- `setup_timings.rs`: The parts of a run's setup before the compile: `Part`. Entry: `Laps::start`.
- `kept_apps.rs`: What a warm server keeps between requests while… Entry: `load`.

### al-version

- `awaited.rs`: Behavior that differs between BC versions… Entry: `journal_temp_part_reads_after_refresh`.
- `choose.rs`: How the target of a run is chosen (ADR-015 3.2, 3.8). Entry: `AppVersions`.
- `gates.rs`: Behaviour that differs between BC versions. Entry: `Evidence`.
- `lib.rs`: BC versions and the run target (ADR-015). Entry: `VersionError`.
- `symbols.rs`: The preprocessor symbols the corpus CI defines… Entry: `ReleasedMinors`.

### al-vm

- `autoinc.rs`: AutoIncrement fields: taking the next value, explicit keys. Entry: `auto_increment_field`.
- `baseline.rs`: The install baseline, with the install routines… Entry: `Baseline`.
- `baseline/install_orders.rs`: What installing partner apps in several orders needs. Entry: `install_order_differences`.
- `install_footprint.rs`: The footprint of a partner app's install session, and the write set of an install session that took install-session-can-create-task. Entry: `Session::install_footprint_start`, `Session::install_yes_session_bounds`.
- `order_trial.rs`: A subscriber whose install order is not known runs… Entry: `trial_start`.
- `callstack.rs`: The AL call stack of an error and the live stack. Entry: `add_frame`.
- `caught_errors.rs`: The count of errors AL code caught: asserterror. Entry: `catch_error`.
- `clear_shared_codeunit.rs`: Clear and ClearAll of a codeunit variable whose instance another holder shares. Entry: `Session::clear_shared_codeunit`.
- `crm.rs`: A `TableType = CRM` table through the `@@test@@`… Entry: `connection_check`.
- `companies.rs`: The company a record works in, ChangeCompany. Entry: `find_company`.
- `dataset.rs`: The dataset XML `Report.SaveAs` writes: header. Entry: `dataset_name`.
- `dispatch.rs`: Event dispatch: publishers run their subscribers. Entry: `publish`.
- `emit_version.rs`: The emit version of the run's build… Entry: `EmitVersion`.
- `pages/` (114 files): Test client for pages (ADR-012) and page state. Entry: `page_call`. Files: `server_row_lag.rs`, `state/after_go_to_key.rs`, `state/close.rs`, `state/list_view.rs`, `state/mode_actions.rs`, `state/lookup_left.rs`, `state/refresh.rs`, `state/stages.rs`, `state/mod.rs`, `pageops.rs`, `open/mod.rs`, `rows.rs`, `go_to_key_read_up.rs`, `pinned_shape.rs`, `long_view.rs`, `open/temp_ws_preloaded.rs`, `currpage_run_typed_key.rs`, `action_enabled_row.rs`, `record_trigger_answer.rs`, `temp_list_gone_row.rs`, `draft.rs`, `new_document_insert.rs`, `empty_view_update.rs`, `controls.rs`, `builtin_mode_action.rs`, `list_view_switch.rs`, `action_tally.rs`, `actions.rs`, `relation_lookup.rs`, `relation_text_entry.rs`, `empty_after_action.rs`, `relation_lookup_close.rs`, `document_handler_error.rs`, `page_run_close.rs`, `modal_handler_error.rs`, `caption_runs.rs`, `own_area_caption.rs`, `read_backwards.rs`, `text_control_value.rs`, `modal_close_save_error.rs`, `code_close_veto.rs`, `code_inserted_new_line.rs`, `code_inserted_pinned.rs`, `code_inserted_extra.rs`, `code_inserted_coded.rs`, `gone_new_line.rs`, `modify_denied.rs`, `format.rs`, `ext_column_visible.rs`, `format_procedure.rs`, `format_type11.rs`, `get_field.rs`, `cue_value.rs`, `areas.rs`, `background.rs`, `filter.rs`, `filter_eq_text.rs`, `filter_option_ne.rs`, `parts.rs`, `part_filter_find.rs`, `part_update_left_view.rs`, `part_refill_pins.rs`, `split_key_view.rs`, `split_part_second_control_xrec.rs`, `part_view_host_new.rs`, `end_move_no_read.rs`, `pinned_body.rs`, `part_refresh_find_forward.rs`, `post_left_new_line.rs`, `part_unsettled.rs`, `row_var_outside_call.rs`, `provider_hidden.rs`, `provider_no_row.rs`, `hidden_part_journal.rs`, `hidden_part_journal_code.rs`, `action_body.rs`, `card_part_new_host.rs`, `empty_part_rec.rs`, `empty_part_rec_list.rs`, `walk_parts.rs`, `empty_part_rec_list_triggers.rs`, `held_page_part.rs`, `request.rs`, `request_ok_only.rs`, `runobject.rs`, `dirty_step.rs`, `caption_relation.rs`, `caption_expr_list.rs`, `ws_trigger_order.rs`, `blank_enum_rows.rs`, `empty_list_one_row.rs`, `data_caption_get_open.rs`, `data_caption_get_body.rs`, `data_caption_kept_open.rs`, `data_caption_open.rs`, `delayed_blank_key_save.rs`, `leave_new_line.rs`, `list_factbox_new_line.rs`, `modify_record_xrec.rs`, `validate_xrec.rs`, `no_source_events.rs`, `lookup_editable.rs`, `next_held_row_read.rs`, `list_card_action.rs`, `modal_answer.rs`, `page_var_modeless_temp.rs`, `refresh_new_line.rs`, `validate_view_cells.rs`, `validate_view_refresh.rs`, `open/temp_list_key.rs`, `open/temp_navigate_new.rs`, `open/saved_row.rs`, `open/or_group.rs`, `shown.rs`
- `page_held_bindings.rs`: Manual bindings a page's globals hold (al-runner issue 6924. Entry: `note_page_instance`.
- `dotnet/` (15 files): `DotNet` calls (ADR-014): router and marshalling. Entry: `DotNetConfig`. Files: `mod.rs`, `blob_stream.rs`, `not_created.rs`, `pinned_exceptions.rs`, `zip_entry_streams.rs`, `last_error.rs`, `invoke_exception.rs`, `policy.rs`, `zone_rules.rs`, `pure.rs`, `registry.rs`, `query_metadata.rs`, `business_chart_data.rs`, `compression.rs`, `gzip_inflate.rs`
- `errorcollect.rs`: ErrorBehavior::Collect scopes. Entry: `declares_collect`.
- `object_ids.rs`: The declared id of an object where an error text… Entry: `declared_id`.
- `events.rs`: Event publishers and subscribers. Entry: `Registry`.
- `auto_format_writer.rs`: A subscriber outside Microsoft's apps that can… Entry: `Session::auto_format_writer_refusal`.
- `faultsite.rs`: Where a refusal or a failure was raised. Entry: `FaultFrame`.
- `filterpage.rs`: FilterPageBuilder. Entry: `filter_page_builder_op`.
- `flowfields.rs`: FlowFields, FlowFilters, `CalcFields`.
- `group1_sessions.rs`: The filters in filter group 1 a session holds. Entry: `Session::set_global_group_filter`.
- `group1_sessions/worker.rs`: What a page background task's worker may do… Entry: `Session::worker_call_measured`.
- `geturl.rs`: `GetUrl` for `ClientType::Web`, Default, Windows. Entry: `Session::get_url`.
- `globaltriggers.rs`: The events of the platform codeunit `Global Triggers`. Entry: `global_setup`.
- `hostvalues.rs`: Builtins that answer from the host. Entry: `datetime_of_unix_millis`.
- `assigned_audit.rs`: What a Modify and a Rename write after AL code…
- `impact.rs`: The per-test record for the impact index.
- `observer_tap.rs`: The cell tap: counts `Expect` calls of corpus… Entry: `compare`.
- `interfaces.rs`: Interface variables: dispatch through… Entry: `interface_test`.
- `lib.rs`: Engine and Session, construction…
- `interp.rs`: Helpers the bytecode calls: `read`, `write`, `element`.
- `blob_transfer.rs`: The Blob value a record field or a Variant takes… Entry: `moved`.
- `table_facts.rs`: `TableFacts`, what the VM asks of a table on every… Entry: `of_heads`.
- `isolated_storage_reads.rs`: What an `IsolatedStorage` read answers beyond…
- `bg_readisolation_lock.rs`: A page background task's record read under… Entry: `Session::row_locks_read`.
- `exec/mod.rs`: `exec/compile.rs`, `exec/run.rs`, `exec/types.rs`. Entry: `ExecProgram::chunk`.
- `profile.rs`: The AL profiler's call tree. Entry: `ProfileMode`.
- `debug.rs`: The debugger seam: `DebugHook`, `DebugView`. Entry: `DebugHook`.
- `ctx.rs`: `Session` as the context natives call back into.
- `single_track.rs`: The notes of a session that watches SingleInstance…
- `single_unmeasured.rs`: SingleInstance codeunits whose state the run found…
- `var_option.rs`: An Option handed to a `var` parameter whose… Entry: `var_binding`.
- `var_option/carried.rs`: A memberless `var` Option parameter handed on to one with members: the measured origins, lists and callee uses, and a parameter with members handed to a memberless one that only formats it (assumption var-option-carried-members). Entry: `carried_args`, `hand_on_args`.
- `session_language.rs`: The session language and its one writer…
- `state.rs`: Sub-states of `Session`: `TxState` (what ends…
- `pages.rs`: The page test client: shared helpers.
- `permissions.rs`: The session's permission sets. Entry: `SetModel::compose`.
- `permissions/` (4 files): The session's permission sets (ADR-062). Entry: `decide`. Files: `permission_probed/inherent_letter.rs`, `permission_probed/reads.rs`, `permission_probed/runner_sets_insert.rs`, `permission_probed/extension_code.rs`
- `permission_trigger_setup.rs`: The read that the code answering… Entry: `Session::trigger_setup_read_unchecked`.
- `permission_system_flowfield.rs`: The read "Published Application"."Tenant Visible"… Entry: `Session::system_flowfield_reads_unchecked`.
- `license_ceiling.rs`: The license as a ceiling of the permission check. Entry: `Session::license_verdict`.
- `partial.rs`: Partial records (SetLoadFields, LoadFields). Entry: `note_partial_fetch`.
- `query_cache.rs`: The query cache page background tasks and the test… Entry: `cached_get`.
- `platform/` (18 files): Row providers for the platform's system tables. Entry: `Providers`. Files: `apps.rs`, `date.rs`, `fields.rs`, `field_relation.rs`, `relation_field_no_readers.rs`, `field_key_lookup.rs`, `host_tables.rs`, `identity.rs`, `ids.rs`, `integer.rs`, `links.rs`, `mat.rs`, `mod.rs`, `objects.rs`, `pages.rs`, `permissions.rs`, `ranges.rs`, `seed.rs`
- `queries.rs`: Query objects: Open, Read, column values, SaveAs. Entry: `query_op`.
- `query_save_as.rs`: The bytes Query.SaveAsXml and SaveAsJson write. Entry: `render`.
- `query_reopen.rs`: The pinned second Query.Open while rows are left. Entry: `query_reopen_step`.
- `recmeta.rs`: Record builtins that read table metadata or guard… Entry: `string_literal`.
- `records.rs`: Record.* on the store: Insert, Modify, Delete, Get. Entry: `record_op`.
- `get_key_value.rs`: The primary key a `Record.Get` looks for (each… Entry: `get_key`.
- `records/` (24 files): Record operations on the store for `Record.*`. Files: `initial.rs`, `op_inner.rs`, `setfilter_empty_or_side.rs`, `filter_error.rs`, `measured_setfilter.rs`, `placeholder_wildcard_edge.rs`, `option_filter_error.rs`, `boolean_filter_error.rs`, `setrange_filter.rs`, `stale.rs`, `restamped.rs`, `empty_find_origin.rs`, `stale_rule.rs`, `guarded_run_body.rs`, `insert_buffer.rs`, `locking_read_cells.rs`, `sql_range.rs`, `time_rollover.rs`, `delete_all.rs`, `by_value_view.rs`, `nested_validate.rs`, `running_call_xrec.rs`, `validate_event_xrec.rs`, `flowfield_key.rs`
- `refs.rs`: `RecordRef`, `FieldRef` and `KeyRef` on the VM. Entry: `record_ref_value`.
- `unopened_refs.rs`: Members of a RecordRef that is not open… Entry: `record_not_open`.
- `fieldref_media_filter.rs`: The error of FieldRef.SetFilter with an argument… Entry: `media_fieldref_filter_argument`.
- `recordid_table_name.rs`: The table a RecordId text names beyond the exact name. Entry: `record_id_tables_bracketed`.
- `relations.rs`: TableRelation: lookups, Validate checks… Entry: `field_relation`.
- `rename.rs`: Rename propagation to related tables. Entry: `rename_references`.
- `rename_blank_key.rs`: A Rename of a database record in which a primary…
- `stale_rename.rs`: A Rename through a stale record.
- `reports.rs`: Report objects at run time: dataset walk, triggers. Entry: `report_builtin`.
- `reports/` (4 files): Report objects: Run, SaveAs, data items, layouts. Files: `item_view.rs`, `dependency_items.rs`, `transaction_world.rs`, `property_gates.rs`
- `report_placeholder/`: `mod.rs` writes the output of a rendering save.
- `report_render/`: The Html of a Word layout made by BC's own merger…
- `report_save/`: The answer of a rendering save as one table of cells.
- `sessionops.rs`: Builtins that act on session state. Entry: `Sequence`.
- `raise_to.rs`: `Session::raise_to`, an event raised to some…
- `storage_checkpoint.rs`: The isolated storage and number sequences at a test… Entry: `storage_checkpoint`.
- `sequence_start.rs`: The number sequences as a test codeunit's tests… Entry: `SequenceStart`.
- `setrange_convert.rs`: A value of another type than the field's given to SetRange or SetFilter from AL code… Entries: `fieldref_setrange_convert`, `record_setrange_convert`.
- `state_stamp.rs`: The session language, WorkDate. Entry: `Session::state_stamp`.
- `state_render.rs`: The one owner of how a value reads… Entry: `Render::new`.
- `state_facts.rs`: The facts `Session::state_lines` compares besides…
- `slot_default.rs`: The value a routine's local or an object's global… Entry: `slot_default`.
- `install_left.rs`: `install_left/clients.rs`. Entry: `Session::install_left`.
- `session_fields.rs`: Every `Session` field in one class of one table. Entry: `Session::at_rest_why`.
- `login_image.rs`: The session image after the raised login, and a new session made to hold it. Entry: `Session::login_image`.
- `resource_reads.rs`: The resource files of the apps AL code of a session read, and the one owner of "their texts are still the same" (install-once, login starts). Entry: `Session::resource_reads`, `TextsRead`.
- `sqlcount.rs`: The model behind `SqlStatementsExecuted` /… Entry: `SqlCounters::read`.
- `tasks.rs`: `TaskScheduler.CreateTask`, `TaskExists`, `CancelTask`. Entry: `Session::task_scheduler_op`.
- `tenant_set_delete.rs`: The Delete or DeleteAll of a stored Tenant… Entry: `tenant_set_delete_one`.
- `testdata.rs`: Test data: `TestBacking` maps a provider's columns… Entry: `TestBacking`.
- `transaction_type.rs`: `Database.CurrentTransactionType`. Entry: `current_transaction_type`.
- `transaction_type_saved.rs`: A `CurrentTransactionType` read while the type is not known whose value only sets it back. Entry: `Session::read_saved_for_set_back`.
- `transactions.rs`: Transactions as AL code sees them. Entry: `TxModel`.
- `consistent_marks.rs`: `Record.Consistent` marks and how a run or Commit ends on one. Entry: `Session::set_consistent`.
- `ui.rs`: Dialogs raised by the code under test and the test… Entry: `UiKind`.
- `dev_endpoint_ui.rs`: `--dev-endpoint-ui`: the answer of the dev-endpoint…
- `clientless.rs`: What a dialog does in a session without a client.
- `variant_copy.rs`: A temporary record going from a Variant into a Variant. Entry: `source_of`.
- `views.rs`: Filter and view text: SetView, GetFilters. Entry: `parse_filter_text`.
- `xmlports/mod.rs`: XmlPort import and export (Format = Xml).
- `xmlports/linked_view.rs`: Import of a linked element whose SourceTableView has one `const`. Entry: `xp_linked_view_entry`.
