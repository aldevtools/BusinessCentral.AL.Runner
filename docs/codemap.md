# Code map

Read this before you open a source file. It says which module owns what, where the entry points
are, and how the main flows call through the crates. File names are checked by a test
(`crates/al-test-cli/tests/codemap.rs`): a listed file that no longer exists fails `cargo test`.
Line numbers are a snapshot and drift; find the current line with `rust-analyzer` (`documentSymbol`,
`workspaceSymbol`) or `grep -n "fn name"`. The bullets in "Crates and modules" must stay one per file:
`- `path.rs` (lines): what it owns. Entry: ...`.

## Layers

Dependencies point down. Core crates do no OS access (ADR-006); the host seam is `al-host`.

```
al-test-cli    corpus job: oracle, corpus, baseline file, ratchet, pass-rate report      alrunner-cli  user command line `al-runner test`
al-testrun     test runner (runtime), install baseline, dependency loading, sidecar host, explain, refusal ranking
al-vm          interpreter over the IR: Engine (immutable), Session (all mutable state), test client
al-sema        workspace -> members -> check (+ demand) -> lower; produces al-ir; takes dependency objects from a compile cache (provided.rs)
al-cache       the compile cache file (ADR-022): key, per-object zstd blocks, index; bytes in, bytes out
al-builtins    values, builtin functions by catalog overload id, filters, JSON/XML, platform table registry
al-data        Store trait and the in-memory store         al-host   clock, identity, files, DotNet seam
al-ir          the typed IR (plain data)                   al-version  BC versions, run target, gates
al-syntax      lexer, preprocessor, parser, rowan tree, AST views, cheap declaration scan
al-deps        dependency apps: .alpackages, Microsoft artifact feed, cache, package/zip reading
```

## Crates and modules

### al-bcdb

- `bacpac.rs` (417): The `.bacpac` container (port of `Bacpac.cs`). Entry: `BacpacTable` 30, `BacpacSource` 45.
- `bak.rs` (524): The SQL Server backup path (port of `BakSource` and `TableReader.cs`); `open_indexed` keeps the page map and catalog between runs. Entry: `BakSource` 29.
- `bcp.rs` (551): Reads rows out of a bacpac's native-BCP data stream (port of `Bcp.cs`). Entry: `BacpacColumn` 31, `decimal_storage_bytes` 148, `BcpRowReader` 157.
- `catalog.rs` (924): `to_index_bytes`/`from_index_bytes`. Reads the SQL Server system catalog base tables directly from page images (port of `Catalog.cs` and `ClusteredSeek.cs`). Entry: `PagePtr` 34, `AllocUnit` 37, `RowSet` 47, `SysObject` 56.
- `error.rs` (56): Errors. Entry: `Error` 10, `Result` 21.
- `io.rs` (91): The byte source the readers work on. Entry: `ByteSource` 9.
- `lib.rs` (27): Reads Business Central data straight out of a SQL Server native backup (`.bak`) or a cloud export (`.bacpac`).
- `lob.rs` (217): Resolves off-row values. Entry: `LobReader` 29.
- `model.rs` (530): Reading `model.xml`. Entry: `ModelScan` 17, `scan_streaming` 77, `split_name` 267.
- `mtf.rs` (154): Structural parser for the Microsoft Tape Format (MTF) container a SQL Server native backup (`.bak`) is written in (port. Entry: `DataRegion` 36, `MtfFile` 44.
- `pagefile.rs` (713): `IndexStore`, `open_indexed` (a kept page map, checked against the file's length and data regions). Maps SQL Server 8 KB pages inside a native full backup (`.bak`) to their file offsets by parsing the backup's structure,. Entry: `Page` 50, `PageFile` 52, `CrossCheck` 110.
- `query.rs` (814): `plan_read_table_in` (table list parsed once by the caller). The BC meaning layer over a [`BcSource`] (port of the shared part of `Program.cs`). Entry: `BcTable` 15, `is_guid` 30, `parse_table_name` 50, `collate_cmp` 126.
- `records.rs` (425): Record parsers (port of `Records.cs`). Entry: `record_type` 10, `is_ghost_type` 14, `live_slot` 20, `FixedVar` 41.
- `scsu.rs` (157): Decoder for SCSU (Standard Compression Scheme for Unicode), Unicode Technical Standard #6. Entry: `decode` 60.
- `serve.rs` (297): Serve mode (port of `Program.Serve`). Entry: `serve` 33.
- `source.rs` (80): The query surface both containers implement (port of `IBcSource`). Entry: `SourceTable` 16, `BcSource` 26, `read_all` 54, `is_bacpac` 64.
- `symbols.rs` (431): The AL meaning layer (port of `Symbols.cs`). Entry: `AlField` 17, `AlTable` 25, `AlTableExtension` 37, `SymbolStore` 48.
- `types.rs` (482): Storage-form cells to typed values (port of `Values.cs`). Entry: `SysColumn` 8, `type_name` 31, `Cell` 75, `LobResolver` 83.
- `value.rs` (426): Decoded values. Entry: `Value` 8, `json_string` 100, `format_binary` 294, `format_guid` 307.
- `zip.rs` (186): A zip central-directory reader over a [`ByteSource`]. Entry: `ZipEntry` 12, `ZipIndex` 21.

### al-bcdb-cli

- `cmd.rs` (636): The al-bcdb commands. Entry: `run` 128.
- `main.rs` (28): The al-bcdb binary: reads BC database backups (.bak, .bacpac). Entry: `main` 17.
- `opts.rs` (108): Option parsing, strict the way bcdb's is. Entry: `command_keys` 20, `parse` 68.

### al-bcdb-std

- `file.rs` (66): A file as a byte source (pread), the OS side of `al-bcdb` (ADR-006). Entry: `FileSource` 17.
- `lib.rs` (9): The OS side of `al-bcdb`: file byte source and the backup table provider for `--test-data` (ADR-020).
- `provider.rs` (455): `BackupProvider` (`open_with_index_dir` keeps the backup's index in the cache dir; `cost` for `--timings`): the rows of a restored company from a `.bak` or `.bacpac`, one table per `read`; picks the company, finds the table by AL name and declaring app id, merges `$ext` companions; `default_backup` finds the cached artifact. Entry: `BackupProvider` 23, `default_backup` 214.

### al-cache

- `format.rs` (330): The cache entry file: header, dictionary, per-object blocks, index; open checks the header, the index and the dictionary, a block is checked when read. Entry: `build` 82, `Entry::open` 205, `Entry::verify` 263, `CacheError` 39.
- `index.rs` (140): The index entry (ADR-022, pure tests): the scan of the dependency sources, keyed, for a run that reads none of them. Entry: `build_index`, `open_index`, `index_file_name`.
- `key.rs` (170): The key of an entry: every input of the compile of the dependency set, hashed without `..`. Entry: `Key` 41, `Key::digest` 80, `AppKey` 21, `FORMAT` 10.
- `lib.rs` (14): Crate root of the compile cache format (ADR-022). Core crate: no OS access.

### al-builtins

- `catalog.rs` (432): Generating the signature catalog from the docs scrape (ADR-008). Entry: `Catalog` 54, `load` 65, `for_target` 72, `generate` 219.
- `collation.rs` (135): Text order for `<`, `>`, `<=` and `>=`. Entry: `compare` 74.
- `collections.rs` (286): List and Dictionary methods.
- `describe.rs` (330): How a value or type reads in a refusal reason or internal error (AL type and a short value, never a debug dump). Entry: `Describe` 12.
- `datatransfer.rs` (74): DataTransfer (install and upgrade code only). Entry: `native_for` 58.
- `datefns.rs` (769): Date, Time and DateTime builtins. Entry: `formula_text` 237, `check_formula` 323.
- `errorinfo.rs` (307): ErrorInfo: the property bag, Error(ErrorInfo), collected errors. Entry: `raise` 239, `copy_of` 263, `native_for` 283.
- `evaluate.rs` (265): `Evaluate(var Variable, String [, Number])`. Entry: `evaluate` 205.
- `file.rs` (362): The `File` data type, on the host file system capability (ADR-006, `al-host::FileSystem`). Entry: `FileState` 32, `native_for` 331.
- `filter.rs` (626): Record filters as data (shared by the VM and the store). Entry: `FilterExpr` 8, `parse_filter` 70, `parse_filter_with` 79, `FieldFilter` 371.
- `format.rs` (619): `Format(Value [, Length] [, FormatNumber])`. Entry: `FormatCtx` 8, `FormatFail` 15, `try_format` 29, `format_value` 37.
- `formatstr.rs` (389): `Format(Value, Length, FormatString)`. Entry: `format_with_string` 50.
- `http.rs` (85): `HttpRequestMessage`: `Method`, `SetRequestUri`, `GetRequestUri`; nothing is sent. Entry: `native_for` 71.
- `iso.rs` (121): ISO 8601 text to AL date/time values (JSON strings, XML-format Evaluate). Entry: `days_in_month` 11, `parse_date` 21, `parse_time` 36, `parse_datetime` 56.
- `json.rs` (1415): JsonObject, JsonArray, JsonToken and JsonValue. Entry: `Handle` 17, `JsonCell` 21, `JsonNode` 27, `Scalar` 34.
- `lib.rs` (39): Crate root of the AL standard library: values, builtins, catalog.
- `mathfns.rs` (245): Round and Power. Entry: `round` 31.
- `media.rs` (197): `Media` and `MediaSet` fields (corpus media/). Entry: `Id` 14, `MediaObject` 18, `MediaStore` 26.
- `moduleinfo.rs` (287): `NavApp.GetCurrentModuleInfo` and friends, and the `ModuleInfo` type. Entry: `native_for` 169.
- `native.rs` (1274): Native builtin implementations, registered by catalog overload id (ADR-008). Entry: `Signal` 26, `AlError` 9, `Ctx` 48, `lookup` 435, `raise` 16.
- `ops.rs` (650): Value operations: conversion, arithmetic, comparison, truthiness. Entry: `convert` 16, `arith` 180, `compare_order` 301, `truthy` 384.
- `pagebuiltins.rs` (123): `TestPage`, `TestPart`, `TestField`, `TestAction` and `TestFilter` methods (ADR-012).
- `pagefmt.rs` (683): The text a page control shows for a value (ADR-012), en-US. Entry: `Blank` 11, `ControlFormat` 37, `parse_decimal_places` 50, `parse_precision` 62.
- `platform.rs` (259): The registry of platform tables that have a native row provider (ADR-013 decision 7, ADR-019). Entry: `Provider` 14, `Virtual` 23, `provider` 157, `virtual_table` 162.
- `querybuiltins.rs` (31): `Query` variable methods.
- `refs.rs` (259): `RecordRef`, `FieldRef` and `KeyRef` methods.
- `sessionfns.rs` (66): Builtins that act on session state.
- `sessiontypes.rs` (287): `SessionSettings` and `Notification`. Entry: `native_for` 265.
- `signature.rs` (457): Parsing the `syntax` line of a documented builtin into a signature. Entry: `Param` 4, `Signature` 14, `DocEntry` 33, `DocParam` 47.
- `stream.rs` (549): In-memory streams over Blob fields, plus the array helpers that share their corpus folder (`streams/`). Entry: `BlobBuf` 23, `BlobRef` 33, `new_blob` 35, `copy_blob` 44.
- `sysenum.rs` (119): Platform option types whose values AL writes `FieldClass::Normal`. Entry: `members` 9, `option_members` 116.
- `textfns.rs` (724): Text functions and methods, TextBuilder and BigText. Entry: `secret_or_text` 493, `native_for` 679.
- `value.rs` (859): Runtime values (ADR-010). Entry: `Value` 70, `RtType` 26, `Instance` 605, `DotNetObject` 133.
- `version.rs` (33): The Version type. Entry: `native_for` 20.
- `wsac.rs` (119): `WebServiceActionContext`. Entry: `native_for` 104.
- `xml.rs` (2642): XmlDocument, XmlElement, XmlAttribute, XmlText, XmlCData, XmlComment, XmlProcessingInstruction, XmlDeclaration, XmlNode,. Entry: `XmlCell` 23, `XData` 29, `escape_text` 774, `escape_attr` 786.
- `xmlport.rs` (286): The document side of XmlPort import and export (Format = Xml). Entry: `Elem` 12, `Node` 19, `ReadFailure` 58, `parse` 66.

### al-data

- `contract.rs` (220): The conformance suite every `Store` must pass (ADR-003). Entry: `table` 7, `cx` 36, `row` 43, `scan_all` 59.
- `lib.rs` (153): The Store trait and table definitions (TableDef, Row, AccessContext). Entry: `Store` 120, `TableDef` 21, `FieldDef` 12, `AccessContext` 40, `InsertOutcome` 58.
- `memory.rs` (640): In-memory store on persistent maps; a `Backing` gives untouched tables their restored rows (test data). Entry: `MemStore` 33, `with_virtual_tables` 63, `detached` 72, `attached` 83.

### al-deps

- `artifact.rs` (1153): Dependency apps from Microsoft's artifact feed, as the fallback behind the project's `.alpackages` and `--deps` (ADR-013. Entry: `Cdn` 28, `HttpCdn` 38, `Fetch` 79, `ArtifactUse` 89.
- `build.rs` (424): Which Business Central build supplies the Microsoft apps (ADR-013 decision 2, "Layer A"). Entry: `BuildId` 14, `parse_index` 91, `BuildQuery` 106, `Origin` 160.
- `test_apps.rs` (330): The Microsoft test apps of a build (ADR-052 1.3): the `*.Source.zip` entries whose path names a test (a country's replace the w1 ones of the same name), listed from the artifact's central directory or an extracted artifact, fetched into the cache only when asked; and the Legacy lane's RapidStart package. Entry: `list`, `Listing::get`, `Listing::rapidstart_package`, `candidates`.
- `demo.rs` (200): The demo database backup of a build: streamed from the country artifact into the artifact cache (ADR-020). Entry: `ensure_demo_backup`, `cached_backup`, `DemoBackup`.
- `cache.rs` (349): The artifact cache (ADR-013 decision 3). Entry: `cache_root` 21, `cache_root_from_env` 44, `Store` 48, `BuildDir` 86.
- `lib.rs` (17): Dependency apps for the runtime (ADR-013).
- `lazy.rs` (200): A package whose files are not read yet: zip directory and manifest only, files read by position on demand, a hash of the directory (ADR-022, pure tests). Entry: `read_lazy`, `LazyPackage`.
- `load.rs` (577): Finding, choosing and reading the dependency apps of a project (ADR-013). Entry: `parse_app_json` 26, `read_package_at` 96, `scan_dir` 103.
- `package.rs` (531): Reading an .app package: manifest, sources, resources. Entry: `Manifest` 9, `parse_manifest` 51, `Sources` 48.
- `remote.rs` (517): A ZIP archive read by byte ranges. Entry: `RemoteError` 14, `RangeSource` 65, `FileSource` 73, `HttpSource` 114.
- `select.rs` (644): Which package satisfies each dependency (ADR-013 decision 2, "Layer B"). Entry: `DepDecl` 27, `Candidate` 40, `Project` 65, `MissingPolicy` 72.
- `version.rs` (62): App version numbers. Entry: `Version` 7.
- `zip.rs` (499): A small ZIP reader over a byte slice, and the NAVX wrapper of `.app` files. Entry: `ZipError` 11, `Entry` 55, `Zip` 64, `EndRecord` 89.

### al-host

- `dotnet.rs` (238): The `DotNet` provider seam (ADR-014 section 2). Entry: `DotNetType` 10, `DotNetHandle` 26, `DotNetValue` 30, `DotNetArg` 65.
- `lib.rs` (350): The host seam: clock, identity, client type, file system (ADR-006). Entry: `Clock` 21, `Identity` 70, `ClientType` 54, `Host` 258.
- `testdata.rs` (330): Test data capability (ADR-020): `TableProvider`, `Cell`, `TestData` (reads each table once for all threads) and `LoadStats` (the counts behind the summary line). Entry: `TableProvider`, `TestData`, `LoadStats`, `Summary`.
- `random.rs` (112): .NET's seeded `System.Random` for `Random` and `Randomize`. Entry: `NetRandom` 15.

### al-ir

- `body.rs` (270): `Body`, the statements of a routine: a compact form (postcard) stores a body as its own byte string, and a body read from it is decoded on first read, with the link work kept for that moment (`BodyFixup`). Entry: `Body`, `Body::fix`, `BodyFixup`.
- `lib.rs` (1067): The typed IR (ADR-011): Program, Object, Routine, Stmt, Expr. Plain serde data, no syntax nodes. Entry: `Program` 8, `Object` 150, `Routine` 747, `Stmt` 937, `Expr` 844, `SrcPos` 85, `NO_POS` 79.

- `sym.rs` (160): The symbolic form of an object (ADR-033): the keys (`ObjKey`, `LocalKey`, `RoutineKey`, `FieldKey`, `ControlKey`), the per-object tables of references (`Refs`) and `SymObject`. Entry: `ObjKey`, `Refs`, `SymObject`.
- `walk.rs` (992): Every reference to another object, routine, field or control that an `Object` holds (`RefVisitor`, `walk_object`, `TableOf`); the one place that lists the reference sites of the IR. Entry: `walk_object`, `RefVisitor`.

### al-link

- `lib.rs` (851): The link step (ADR-033): symbolic objects in, the dense `Program` out. `link` for a whole program, `relink` for the incremental compile, `first_difference` for the `AL_CHECK_LINK` proof, `par_map` for the worker threads. Entry: `link`, `relink`, `Unit`, `LinkError`.

### al-sema

- `alcodes.rs` (520): Compile errors with the AL compiler's own codes (must-not-compile corpus): `IMPLEMENTED`, `NOT_CHECKED`, `check_workspace` (source-only checks, run by `check_with`), `check_context` (target and app.json checks, the caller supplies the `AppContext`). The typed ones (AL0118, AL0122, AL0132, AL0161) are in `check.rs` (`al_unresolved_name`, `al_enum_assignment`, `ABSENT_ENUM_MEMBERS`).
- `base_access.rs` (78): Page and report extension code that names the base object's members (ADR-023): which probes are pinned (`PINNED`), the `AL_UNPINNED_BASE_ACCESS` switch, the base's visible variables and procedures, and the slot of a base variable from syntax. Entry: `PINNED`, `pinned`, `scope_enabled`, `visible_vars`, `visible_procs`, `slot_of`.
- `calcformula.rs` (513): The `CalcFormula` property of a FlowField, as syntax. Entry: `Formula` 18, `Cond` 28, `CondValue` 35, `parse` 48.
- `check/mod.rs` (755): Resolves every name, member access and call in routine bodies, inferring types. Holds `CheckResult`, `Res`, `Roots`, the checker state and the entry points; `lean_left_out` (the workspace objects a lean compile of a selection leaves out, `Roots::lean`, `LEFT_OUT`). Entry: `check_with` 175, `Checker` 506, `CheckResult` 29, `Res` 82, `Roots` 124.
- `check/tdd.rs` (950): `--tdd` (ADR-051): call sites in test apps of procedures a codeunit, table, page or report does not declare (`tdd_member`), bare calls of unknown names (`tdd_bare`) and calls no overload takes by count (`tdd_overload`) become stub sites (`Res::TddStub`), their return type from the use (`tdd_settle*`), the refusals, and the signatures (`signature`; `sites_by_object` and `stub_list` for `Program::tdd_stubs`, which `relower_objects` renews from `Lowered::tdd_sites` for the objects it compiles again). Lowering makes `ir::Expr::TddStub` (`lower/call.rs` `tdd_stub`); the VM yields the type's initial value and notes the stub in `Session::take_tdd_reached`. Entry: `TddSite`, `TddRet`, `test_apps`, `signature`.
- `check/tdd_decl.rs` (630): `--tdd` stub declarations (ADR-051 decision 10). The checker notes field sites (`tdd_field`, a `TddKind::Field` site typed by the use) and enum value sites (`tdd_enum_value`); `needs` turns them, and the codeunits, tables and enums the test apps name that no app declares, into `TddNeed`s (`Lowered::tdd_needs`). `stub_files` writes the stub file of each test app (`TDD_FILE`: table and enum extensions, stub objects with the first free id of the app's `idRanges`), and the apps are compiled again (`lower/compile.rs` `compile_with_roots`, al-testrun `compile_suites_with`, at most `MAX_RECOMPILES` times). `decls` reads the declarations back (`CheckResult::tdd_decls`): lowering's `res` notes a use and `stmt` puts an `ir::Stmt::TddMark` before the statement, a routine with a variable of a stub object starts with one, a statement that names a global of a stub object gets one (`Lowerer::tdd_globals`), and a write to a stub Text or Code field is wrapped in `ir::Expr::TddFit`; a dynamic write (FieldRef, TransferFields, Validate by number) checks `ir::Field::tdd_length_unknown` (al-vm `recmeta::tdd_fits`). A bare unknown name in a table's or table extension's own code is a field site too (`tdd_bare_field`). The incremental compile refuses a compile with stub declarations (a complete compile makes them again).
- `check/posmap.rs` (123): The position-keyed maps the checker fills: FxHash and the sharded `PosMap`. Entry: `PosMap` 42, `union` 88.
- `check/object.rs` (549): The checker's object context, name resolution, and the walk over an object's routines. Entry: `object_ctx` 126, `check_object` 353, `check_routine_in` 423, `walk` 522.
- `check/infer.rs` (800): Type inference for expressions and the diagnostics that hang off it. Entry: `infer` 235, `al_unresolved_name` 92, `name` 723.
- `check/calls.rs` (375): Overload picking and the lookup of callables. Entry: `pick` 8, `lookup_callable` 74, `lookup_name` 228.
- `check/member.rs` (291): Members of a type: page controls, record fields, sections. Entry: `member` 40, `field_of` 217.
- `check/fit.rs` (178): Candidates and whether an argument type fits a parameter type. Entry: `fits` 22, `sig_cand` 110, `proc_cand` 4.
- `check/demand.rs` (1417): Demand-driven checking of dependency apps (ADR-013 decision 6): what a test reaches is compiled, the rest is not. Entry: `Demand` 44, `run_demand` 765, `reach` 247, `reach_baseline` 261, `activate` 351, `activate_table` 398, `activate_ops` 451, `activate_global` 498.
- `lib.rs` (16): Crate root of the semantic model.
- `lower/mod.rs` (352): Lowers resolved syntax to the typed IR (ADR-011). Anything inexpressible becomes a refusal reason on the routine. Holds `Lowered`, the `Lowerer` state and the module list. Entry: `Lowered` 26, `Lowerer` 127.
- `lower/assemble.rs` (1162): Lowering a whole workspace: drives the objects (in parallel), builds the shared data, relowers the objects an edit touched. Entry: `lower_with_positions` 114, `build_shared` 168, `lower_core` 763, `relower_objects` 1008.
- `lower/localize.rs` (267): From a lowered object (positions) to a symbolic one (keys): routine keys from the syntax, the walk that moves every reference into the object's tables. Entry: `localize`, `routine_keys`, `Keys`.
- `lower/shared.rs` (880): What the lowering of an object reads (`Shared`) and the per-object state (`Nodes`, `Local`, `Seq`), `lower_object`, the constants and literal helpers. Entry: `Shared` 6, `make_lowerer` 173, `lower_object` 211.
- `lower/tables.rs` (518): Table schemas: relations, field triggers, CalcFormulas. Entry: `table_schema` 311, `relation_arms` 10, `lower_calc` 445.
- `lower/pages.rs` (458): Page control trees, `RunObject`, part links. Entry: `page` 8, `run_object` 292.
- `lower/convert.rs` (359): Name resolution helpers, runtime types, conversion of a value to a target type. Entry: `rt` 67, `convert_to` 186.
- `lower/stmt.rs` (737): Routines and statements. Entry: `routine` 5, `stmt` 277, `assign` 425, `place` 670.
- `lower/expr.rs` (636): Expressions, constants, operands, controls. Entry: `expr` 280, `constant` 228.
- `lower/dotnet.rs` (76): Arguments and members of `DotNet` values (ADR-014). Entry: `dotnet_arg` 7, `dotnet_member` 30.
- `lower/call.rs` (404): Calls. Entry: `call` 6.
- `lower/compile.rs` (306): Load, check and lower in one go, and the cached layer variants. Entry: `compile` 6, `compile_apps_with` 46, `compile_with_roots` 270.
- `members.rs` (708): Members of objects: table fields (extensions merged), enum values, procedure signatures. Entry: `MemberTable` 108, `build` 147, `FieldInfo` 15, `ProcSig` 48.
- `provided.rs` (450): Dependency objects whose lowered form comes from a compile cache (ADR-022): `DepFacts`, `DepBlock`, `DepSource`, layers (`DepLayer`, `LayerSource`, `Stack`), which blocks the workspace or a layer invalidates (`relower_set`). Entry: `DepSet` 76, `relower_set` 150, `touched` 120, `check_identity` 100.
- `pages.rs` (932): Pages for the test client (ADR-012). Entry: `expand` 153, `expand_ranges` 159, `control_sections` 143, `classify_source` 81.
- `queries.rs` (760): Query objects (corpus query/). Entry: `QueryLayout` 16, `layout` 56.
- `relation.rs` (359): `TableRelation` property text → arms (before names are resolved). Entry: `ParsedArm` 13, `ParsedWhere` 23, `parse` 262.
- `reports.rs` (1035): Report objects: dataset walk, columns, and the IR it lowers to. Entry: `layout` 159, `request_layout` 318, `build` 794, `build_ext` 848.
- `types.rs` (240): Semantic types (Ty) and their display names. Entry: `Ty` 7, `display_name` 66, `from_type_ref` 71.
- `workspace.rs` (1561): A set of apps: parse every file, build the object table, resolve names across apps. Entry: `Workspace` 192, `load_apps` 236, `ObjKind` 10, `ObjectSym` 155, `AppSource` 30, `AppKind` 40, `restrict_to` 535, `subscribers_of` 433, `resolve_ref` 682.
- `xmlports.rs` (712): XmlPort objects (Format = Xml). Entry: `RawNode` 24, `Layout` 33, `layout` 97, `Context` 248.

### al-syntax

- `ast.rs` (904): Typed views over the untyped tree. Entry: `routines_in` 45, `sections_in` 57, `Routine` 33.
- `builder.rs` (204): Green tree construction for the parser. Entry: `TreeCache` 19, `Checkpoint` 64, `TreeBuilder` 66.
- `grammar/declarations.rs` (409): Variables, types, parameters, procedures, triggers, attributes. Entry: `var_section` 8, `type_ref` 92, `property_value` 166, `attributes` 184.
- `grammar/expressions.rs` (432): Expression grammar. Entry: `expr` 63, `arg_list` 185.
- `grammar/mod.rs` (38): Grammar entry module.
- `grammar/objects.rs` (270): Files, objects and object bodies. Entry: `source_file` 39.
- `grammar/statements.rs` (311): Statements and code blocks. Entry: `block` 7, `stmt` 47.
- `lexer.rs` (491): Lossless lexer. Entry: `lex` 12, `Token` 6.
- `lib.rs` (16): Crate root of the AL syntax: lexer, parser, tree, AST views.
- `parser.rs` (352): Parser infrastructure and the parse entry points. Entry: `parse` 32, `parse_with_cache` 45, `Parse` 20.
- `preprocessor.rs` (287): Token-level preprocessor (#if and friends). Entry: `preprocess` 23.
- `scan.rs` (497): A cheap declaration scan: objects, ids, subscribers, without a tree (used for dependency apps). Entry: `scan` 127, `ScannedObject` 29, `ScannedSubscriber` 53.
- `syntax_kind.rs` (177): Every token and node kind. Entry: `SyntaxKind` 7, `AlLanguage` 155, `SyntaxNode` 169, `SyntaxToken` 170.

### al-test-cli

- `args.rs` (305): Command-line parsing and the --help text. Entry: `parse` 79, `Args` 7, `HELP` 177.
- `config.rs` (292): oracle/oracle.toml: versions, suites, corpus pin. Entry: `OracleFile` 7, `VersionEntry` 16, `SuiteEntry` 51.
- `corpus.rs` (210): Corpus discovery: test codeunits, their area folder and file. Entry: `discover` 32, `CorpusIndex` 20, `TestCodeunit` 12.
- `coverage.rs` (330): The corpus coverage map (`--coverage-map`, `docs/corpus-coverage.md` and `.json`): which builtin overloads of the catalog some corpus test calls, from the checker's `Res::Builtin` resolutions. Entry: `collect_calls`, `build`, `to_markdown`, `to_json`, `corpus_coverage`. Also the view against the language catalog (`language_view`: every catalog overload with pinned, implemented, refused or unknown and its Microsoft use, `microsoft_calls`).
- `behavior_coverage.rs` (~900): The behavior coverage map (`--behavior-coverage-map`, `docs/corpus-behavior-coverage.md` and `.json`): triggers, properties, platform events, statement/expression kinds and runtime error sites against the corpus; catalog = runtime source mentions union Microsoft-app use; pinned by static reach over resolved `Res::Proc` calls. Entry: `collect`, `classify`, `match_errors`, `corpus_behavior_coverage`. Also attributes, and the view against the language catalog (`language_view`, `REFUSING_OWNERS`).
- `language_catalog.rs` (~600): The AL language catalog `docs/language-catalog.json` (generated by `tools/language-catalog/generate.sh` from the answers of the AL language server of BC 27-29 to probe files): loading, matching catalog overloads to the runtime catalog (`match_methods`), and the maps from coverage groups to catalog owners (`trigger_owners`, `property_owners`, `event_owner`, `syntax_items`).
- `drift.rs` (119): Comparing case sets across BC versions (ADR-015 3.6). Entry: `differences` 11, `common` 26, `restrict` 39, `table` 46.
- `mnc.rs` (300): The must-not-compile corpus (`--must-not-compile [DIR]`): parse a case header, compile it alone, compare the ALnnnn codes, report match / mismatch / not checked / skipped. Entry: `parse_case`, `read_cases`, `run_case`, `render`.
- `insight.rs` (388): --top-refusals (folded reason ranking) and --baseline-diff. Entry: `normalise_reason` 22, `top_refusals` 116, `baseline_diff` 194, `render_top` 145, `render_diff` 222.
- `lib.rs` (22): Runs the corpus's `[Test]` codeunits and reports them against the oracle.
- `main.rs` (806): The binary: run each oracle version, report, ratchet. Exit 0 ok, 1 regression, 2 setup error. Entry: `main` 17, `run` 40, `run_version` 135, `find_explained` 670, `drift_summary` 124.
- `oracle.rs` (288): The oracle: JUnit and xUnit cases of real BC, identified by (codeunit, name). Entry: `CaseId` 7, `OracleCase` 13, `parse_junit` 30, `parse_xunit` 74.
- `output.rs` (238): Renders a report as Markdown (CI step summary) and the runtime's outcomes as JUnit in the same shape the corpus CI emits. Entry: `markdown` 7, `junit` 69.
- `pin.rs` (169): The per-version corpus pin check (ADR-007, "Decided without owner review (2026-10-13/14)"). Entry: `check` 13.
- `report.rs` (461): Joins the oracle with the runtime's outcomes into the pass-rate report, and enforces the ratchet. Entry: `Outcome` 9, `build` 108, `ratchet` 177, `decide` 199, `parse_baseline` 221.
- `runner.rs` (18): The seam between the harness and the runtime. Entry: `Runner` 6, `NoRuntime` 11.
- `select.rs` (143): Test selection: --codeunit, --test, --area, --file (union). Entry: `Selection` 16, `matches` 34, `filter` 57.

### al-testrun

- `affected.rs` (490): `--affected` (ADR-050): the stored result of the last run (`Stored`, `Header`, per workspace object its `ObjectFacts` with the symbolic IR hash, per test codeunit its trace keys, entered objects and outcomes; JSON, `FORMAT`), the rule that picks the test codeunits a change can affect or forces a full run with a reason (`Stored::select`), and the update after a run (`Stored::merge`); the facts of a compile made with `Roots::object_hashes` (`facts`: hash from `Lowered::object_hashes`, trace keys, subscribed events, install code, references) and the objects a test entered (`entered_keys`, from `Touches::entered`). Pure; the file is the command line's. Entry: `Stored`, `select`, `merge`, `facts`, `entered_keys`.
- `baseline.rs` (440): The install baseline (ADR-013 decision 5, ADR-046): the dependency apps' install code (`install::plan`), Company-Initialize, then the apps under test (`install::run_apps_under_test`), captured once. Entry: `BaselineRun` 23, `run` 99, `tables_written` 238.
- `compile_cache.rs` (220): The OS side of the compile cache (ADR-022): the key of a run (hash of the executable and of each app's source), reading, writing (temporary file, rename) and building the entry file. Entry: `CompileCache` 14, `key_for` 83, `read_entry` 108, `write_entry` 128, `open_or_build` 170.
- `cache_gc.rs` (412): Cleanup of the compile cache folder (ADR-022 amendment): other-build entries after 7 days, any entry after 30, old temp files after 1 day; automatic once a day within 200 ms (`al-runner test`), by hand with `al-runner cache prune|info`. Stat only, never reads an entry. Entry: `prune`, `auto_prune`, `info`.
- `crash.rs` (246): Names the test a crashed process was running: each worker records its codeunit and test (`running`, from `events::starting` and `run_unit`); a handler for SIGSEGV, SIGBUS, SIGILL, SIGFPE and SIGABRT, installed by both command lines (`install`), writes it to stderr and hands the signal on (Rust's stack-overflow handler or the default action). Entry: `install`, `running`, `idle`.
- `deps.rs` (158): Dependency apps for a corpus run (ADR-013). Entry: `Dependencies` 11, `Provision` 26, `load` 46, `load_with` 54.
- `discover.rs` (90): Finds `Subtype = Test` codeunits and their `[Test]` procedure names in AL text, without a syntax tree. Entry: `test_codeunits`.
- `dotnet.rs` (1084): The .NET sidecar host (ADR-014): one process per session, standby processes started at run start (`prewarm`) or beside the first process a session needs (`prewarm_on_first_use`, `al-runner` with the compile cache), start-up stats. Entry: `SidecarConfig` 26, `SidecarStats` 72, `SidecarHost` 247, `prewarm` 279, `encode` 944, `decode` 964.
- `explain.rs` (137): --explain: where a test was refused or failed, with file and line. Entry: `describe` 38, `detail` 77, `render` 90, `TestDetail` 27.
- `host.rs` (112): The host the command line gives a session (ADR-006). Entry: `SystemClock` 10, `set_files_dir` 26, `system_host` 32, `DirFileSystem` 43.
- `impact_trace.rs` (370): The impact index's keys (`event <kind> <id> <event>[/<element>]`, `table`, `write`, `page`, `report`, `enum` by object id), their numbering, and the impact points of an extension from the compiled program (subscribers, table/page/report/enum extensions, install code by the tables it can write; uncovered events and kinds are `Unsupported`), each with its `Matcher` (index) and `Site` (static graph). Entry: `keys`, `Interner`, `impact_points`, `ImpactPoint`, `Matcher`, `Site`.
- `install.rs` (495): The install code of each app (ADR-046 step A): install codeunit triggers and `Extension Triggers` subscribers per app and part (`plan`), the tables whose install rows tests may not read (`facts`: decision 4c, amendment 1 A2; `AL_INSTALL_FACTS=1` prints them), and the run of the apps under test one after the other (`run_apps_under_test`: install session id, SingleInstance reset, the app's own Company Triggers subscribers). Entry: `plan` 84, `facts` 186, `ensure_facts` 305, `run_apps_under_test` 352.
- `impact_static.rs` (900): Static selection (ADR-043 step 5): the graph of routines, objects, tables' written nodes, events and the nodes for what the program does not name (`Graph::build`), one reverse search per impact point with the verdicts reaches / cannot rule out (`select`), the install rule over the baseline's trace (`Install`), the index-key-to-node map for the cross-check (`key_node`), test roots, and `path`/`describe` for `AL_IMPACT_WHY`. Entry: `Graph`, `select`, `Install`, `Verdict`, `key_node`, `test_roots`, `baseline_roots`.
- `isolation.rs` (171): TestIsolation of the runner codeunit; Checkpoint for Function isolation. Entry: `TestIsolation` 27, `runner_isolation` 65, `Checkpoint` 99.
- `lib.rs` (32): Running test codeunits and reading the result, shared by `al-test-cli` and `alrunner-cli` (ADR-016 section 2). `Outcome` and `CaseId` live here. Entry: `Outcome`, `CaseId`.
- `observe.rs` (680): Whether a selected test can observe the install baseline: a whitelist over the lowered IR (no record, page, query, `DotNet`, `CreateGuid`, `IsolatedStorage`, `NumberSequence` or event publisher in the reach); the text pre-check of the demand-driven path. Entry: `may_observe_baseline`, `text_looks_pure`.
- `reach.rs` (415): The reach report (`AL_REACH_REPORT=1`): which roots and edges pull the routines in, how many are refused, how many only matter through a refused routine. Entry: `report`.
- `refusals.rs` (222): Ranking not-run reasons with names and numbers folded (`--top-refusals` of both command lines). Entry: `normalise_reason`, `rank`, `render_top`.
- `runtime/mod.rs` (491): Runs test codeunits the way BC's test runner does; one Session per codeunit. Holds the app and manifest loading, `CorpusRuntime`, `TestResult`, `Phases` and the single-unit run. Entry: `CorpusRuntime` 112, `load_app` 17, `Phases` 162.
- `runtime/compiled.rs` (418): What a compile produces and keeps (`Compiled`, `into_apps` for another compile of the same run), the warm slot (`Warm`), and what a run asks for. Entry: `RunOptions` 6, `Compiled` 42, `Warm` 228, `SuiteRun` 398.
- `runtime/corpus.rs` (927): `CorpusRuntime`: compiling suites of apps and running them. A selection of pure tests is compiled lean (`compile_suites_with`, `Roots::lean`) and run again on the complete compile when a test met what was left out (`met_left_out`). Entry: `run_suites_with` 89, `compile_suites` 127, `run_compiled` 448.
- `runtime/cache.rs` (226): Compiling through the compile cache. Entry: `compile_with_cache` 70, `lazy_index` 7.
- `runtime/run.rs` (741): Running the test codeunits of one compiled program, each test on a session of its own. Entry: `run_codeunit` 5, `run_codeunit_isolated` 21, `run_codeunit_observed` 46, `run_codeunit_core` 156.
- `runtime/written.rs` (824): What a test that did not run to its end can change that a later test of the codeunit reads (static reach: tables, codeunit globals, session state; `unknown` for a refused construct or a run by a run-time id), for the trust of later failures. Entry: `ReachFacts`, `changes_read_by`, `ReachIndex`, `reach_facts`, `settled_flags`.
- `runtime/runner_reset.rs` (130): What BC's AL test runner does around each test method: runs codeunits 131006, 130301, 130302 (TestPermissions not Disabled) and 132553 when loaded, in Microsoft's order. Entry: `RunnerReset::of`, `before`, `after`, `test_permissions`.
- `runtime/handlers.rs` (92): The handler functions a test declares, bound for the length of the test. Entry: `resolve_handlers` 8, `bind_handlers` 77.
- `runtime/blocked.rs` (159): What a test cannot run for: install routines that could not run, tables a test touches, blocked routines. Entry: `blocked_site` 114, `run_install_codeunits` 11.
- `target.rs` (99): Reads the version fields of an app.json (ADR-015 step 4). Entry: `build_query` 13, `read_app_versions` 19, `app_versions_from_json` 25.

### alrunner-cli

- `apps.rs` (128): Reads the APPs given on the command line (folder, `*.Source.zip`, `.app`) and builds the project the dependency resolution works for. Entry: `load`, `combined_project`.
- `args.rs` (262): Command-line parsing and the `--help` text of `al-runner`. Entry: `parse`, `HELP`.
- `bcapps.rs` (330): Where the Microsoft recipe files come from: a sparse `git` checkout of BCApps `releases/<major>.<minor>` kept in `<cache>/bcapps/<major>.<minor>/`; a release without the BaseApp test recipe (before 29.0) uses `releases/29.0`. Entry: `resolve`, `git_fetch`, `has_recipe`.
- `path_note.rs` (210): The note about another `al-runner` on PATH (AL Runner v2's dotnet tool): one stderr line at most once a day, the list after `--version`, off with `ALRUNNER_NO_PATH_NOTE=1` (ADR-016). Entry: `maybe_note`, `version_lines`, `others`.
- `recipe.rs` (290): BCApps' settings as lanes (Legacy, UnitTest, Integration, Uncategorized): which company each test codeunit gets. Pure; reads `settings.json` and `TestConfiguration.json`. Entry: `Recipe::load`, `Lane`, `plan`.
- `recipe_run.rs` (330): `al-runner test --recipe microsoft`: one run per lane with the lane's company and Microsoft's disabled tests skipped, then a by-lane summary. Entry: `test`, `run_lanes`, `lane_args`.
- `sources.rs` (560): test sources (ADR-052). The `microsoft` source: the Microsoft test apps of a build (candidates from `al_deps::test_apps`, kept when they hold a test codeunit; `_Exclude_` apps only by name), `--app` narrowing, and `al-runner test-microsoft`: one recipe run per test app. Step 2: `affected` (`test APP --affected-from microsoft`, `test-microsoft APP`): `split` the APPs into extension and own tests, own tests by `run::test_collect`, then `microsoft_source`: per test app `recipe_run::plan_lanes`, per lane `impact::run_part` with the extension as `--impact-of`; one JUnit file (`output::junit_parts`). `build_index`: `impact-index build --from microsoft`. Entry: `test_microsoft`, `affected`, `build_index`, `choose`, `test_apps_of`.
- `sources_report.rs` (440): the exit code shared by runs of several parts (`worst`, ADR-052 1.7; the recipe uses it too), the per-test-app summary table of `test-microsoft`, and the `Sources` table of a run with test sources (`PartRow`, `table_lines`).
- `testdata.rs` (300): Which demo backup a run restores: disk, cache, download, other build; or why test data is off (ADR-020). Entry: `locate`, `Where`, `Located`.
- `debug/mod.rs`, `debug/wire.rs`, `debug/breakpoints.rs` (1400): `al-runner debug` (ADR-048): the DAP session (reader thread answers requests; the test worker blocks in the `DebugHook` at a stop; stack and variables come from a snapshot taken at the stop), Content-Length framing over stdio (stdout carries only frames) or TCP, breakpoint resolution against the program's position table (every statement on the line, column narrowing, unverified reasons) in every app compiled from source (`Sources`/`Origin` per app: a folder, or a source package whose files are written to `<cache>/debug-sources/` at a stop; a checkout of a package binds when its text is the compiled text). Entry: `debug::run`, `debug::sources_of`, `wire::read_message`, `breakpoints::Index`.
- `impact.rs` (1280): `al-runner test --impact-of APP` (ADR-043 steps 1, 2 and 5): dynamic selection with a cached index (`decide` narrows the "with" run after its compile and install baseline; the "without" outcomes come from the index; tests the index's run refused get their static verdict, `refused_static`), static selection (`--impact static`, or no index cached: `run_static`/`decide_static`, both sides run the selection, with the warning), else both runs (`--impact all`); the change class of each test (selection miss included), the impact points it reached, the extension routines it entered, the report and exit code. Entry: `run`, `run_dynamic`, `run_static`, `run_all`, `decide`, `decide_static`, `classify`, `Routines`, `diff_reached`, `report_with`.
- `impact_index.rs` (660): The impact index (ADR-043 step 2): format (gzip JSON, `FORMAT`), cache key and path (`<cache>/impact/<key>/index.json.gz`), `al-runner impact-index build`, and `al-runner impact-index check-static` (step 5: every key a test recorded must be in its static reach; ratios per kind of key; `AL_IMPACT_WHY=<key>` prints the paths). Entry: `Index`, `key`, `lookup`, `build`, `check_static`.
- `main.rs` (51): The `al-runner` binary: runs on a thread with a large stack, maps results to exit codes. Entry: `main`.
- `output.rs` (182): Counts, exit codes (0, 1, 6, 7), the summary line, the one-line warnings shown by default (`baseline_warning`, `dependency_line_is_problem`; the rest is `--verbose`) and JUnit XML of a run. Entry: `exit_code`, `junit`.
- `run.rs` (433): `al-runner test`: load APPs, select test codeunits, resolve dependencies, compile, run, print. Entry: `test`.

### al-version

- `choose.rs` (238): How the target of a run is chosen (ADR-015 3.2, 3.8). Entry: `AppVersions` 10, `TargetError` 58, `Chosen` 84, `choose_target` 98.
- `gates.rs` (115): Behaviour that differs between BC versions, as named facts in one table (ADR-015 3.4). Entry: `Evidence` 12, `Gate` 18, `has` 69.
- `lib.rs` (291): BC versions and the run target (ADR-015). Entry: `VersionError` 21, `BcVersion` 42, `Runtime` 50, `Target` 56.

### al-vm

- `autoinc.rs` (198): AutoIncrement fields: taking the next value, explicit keys. Entry: `auto_increment_field` 28, `take_auto_increment` 58.
- `baseline.rs` (66): The install baseline (ADR-013 decision 5, ADR-004). Entry: `Baseline` 12.
- `callstack.rs` (130): The AL call stack of an error (`GetLastErrorCallStack`) and the live stack (`SessionInformation.Callstack`). Entry: `add_frame`, `add_top_frame`, `live_call_stack`, `set_runner_frames`, `frame_line`.
- `companies.rs` (147): The company a record works in, ChangeCompany, the Company table's rows per company. Entry: `find_company`, `cx_for`, `change_company`.
- `dataset.rs` (332): The dataset XML `Report.SaveAs(.., Xml, ..)` writes: header, `DataItem` rows, name and text escaping. Entry: `dataset_name` 23, `Header` 91, `Dataset` 217.
- `dispatch.rs` (338): Event dispatch: publishers run their subscribers; EventCall; isolated events. Entry: `publish` 69, `raise` 117, `event_targets` 183, `has_listeners` 249, `subscription` 279.
- `dotnet/mod.rs` (488): `DotNet` calls (ADR-014). Entry: `DotNetConfig` 33, `DotNetState` 58, `fit` 455.
- `dotnet/last_error.rs` (188): The exception `GetLastErrorObject` answers and its `GetType()` (GetLastErrorObject.al). Entry: `types` 168, `exception_text` 70.
- `dotnet/policy.rs` (269): What is refused before a `DotNet` request is sent (ADR-014 section 8). Entry: `Access` 6, `Policy` 14.
- `dotnet/registry.rs` (113): Runtime-provided .NET classes (ADR-014 section 1). Entry: `MemberKind` 19, `Pin` 28, `MemberFn` 45, `RegistryMember` 47.
- `errorcollect.rs` (140): ErrorBehavior::Collect scopes. Entry: `declares_collect` 44, `collect_error` 71, `close_error_scope` 86.
- `events.rs` (601): Event publishers and subscribers. Entry: `Registry` 37, `build` 116, `Subscriber` 14, `Publisher` 26.
- `faultsite.rs` (57): Where a refusal or a failure was raised, for `al-test-cli --explain`. Entry: `FaultFrame` 14, `trace_faults` 23, `fault_site` 30, `note_fault` 40.
- `filterpage.rs` (268): FilterPageBuilder. Entry: `filter_page_builder_op` 64.
- `flowfields.rs` (623): FlowFields (`CalcFormula`), FlowFilters, `CalcFields`, `CalcSums` and `SetAutoCalcFields`.
- `geturl.rs` (230): `GetUrl` for `ClientType::Web` with a company, an object, a record bookmark or filters (ADR-042). Entry: `Session::get_url`.
- `globaltriggers.rs` (225): The events of the platform codeunit `Global Triggers` (2000000002, in the platform `System.app`). Entry: `global_setup` 63, `global_enabled` 142, `global_trigger` 157.
- `hostvalues.rs` (259): Builtins that answer from the host (ADR-006). Entry: `datetime_of_unix_millis` 26, `Replaced` 142.
- `impact.rs` (95): The per-test record for the impact index (`Session::enable_impact_trace`, `take_impact_trace`, `Touches`): events raised (also with no subscriber), tables used and written, pages and reports used. Noted in `dispatch::publish`, `records::table_event`, `records::record_op`, the page events, `pages::page_instance`, `reports::report_schema`.
- `interfaces.rs` (106): Interface variables: dispatch through the implementing codeunit. Entry: `interface_test` 32, `interface_target` 54, `enum_implementation` 81.
- `lib.rs` (1070): Engine (immutable program) and Session (mutable state, grouped into the sub-states of `state.rs`), construction and the transaction entry points. The executor is the bytecode in `exec/`; `interp.rs` holds what both it and the rest of the VM use.
- `interp.rs`: Helpers the bytecode calls: `read`, `write` (places, with their index registers), `element`, `write_char`, `finish_call`, `call_site`, `run_native` (by-reference write-back, `fit_ref_number`), `vm_class_call`, `binary_op`, `unary_op`, `set_curr_field_no`, `clear_all`, `application_area`, constants.
- `exec/mod.rs`, `exec/compile.rs`, `exec/run.rs`, `exec/types.rs`: The executable form (ADR-034), the only executor. `compile` turns a routine of the IR into a `Chunk` of register instructions (`compile(.., true)` is the captured form that value capture runs, ADR-030; `compile_debug` is the debug form, one `Op::DbgHit` after each `Op::Pos`, ADR-048); `run_range` is the loop; `types.rs` is the interface the typed instructions get their static types from. Entry: `ExecProgram::chunk`, `captured_chunk`, `debug_chunk`, `compile`, `Session::run_chunk`.
- `debug.rs` (162): The debugger seam (ADR-048): `DebugHook` (called before each statement of the debug form, returns `Resume`), `DebugView` (position, top frame's locals live, outer frames rendered), `Session::attach_debugger`/`detach_debugger`; `Session::form` picks the plain, captured or debug chunk in `Session::call`. Entry: `DebugHook`, `DebugView`, `Session::debug_hit`.
- `ctx.rs` (289): `Session` as the context natives call back into: `FormatCtx` and `Ctx` (`record`, `page`, `session_op`, files, module lookups).
- `state.rs` (117): Sub-states of `Session`: `TxState`, `UiState`, `PageClients`, `EventState`, `CoverageState`, `ReportState` (the .NET host state is `dotnet::DotNetState`).
- `pages.rs` (432): The page test client (ADR-012): shared helpers, page triggers and events, the handle to a control. The rest is split by topic into `pages/`.
- `pages/pageops.rs` (538): The `TestPage.*` and `TestField.*` methods. Entry: `page_op`, `test_page_op`.
- `pages/open.rs` (891): Opening and closing a page, rows loaded on open. Entry: `page_open`, `open_page_as`, `page_close`, `close_with`.
- `pages/rows.rs` (976): The row window, navigation, `GoToKey`, `Find`, modal answers. Entry: `populate_window`, `page_move`, `page_go_to_key`.
- `pages/draft.rs` (304): New rows, drafts, saving a row, activating a control.
- `pages/controls.rs` (588): Writing a control and the `TestField` methods. Entry: `control_set_value`, `test_control_op`.
- `pages/actions.rs` (676): Control navigation, lookups, invoking actions, refresh after an action.
- `pages/format.rs` (435): Reading a control: value, text, type, caption, editable, enabled, visible.
- `pages/areas.rs` (104): Application areas on the test client.
- `pages/background.rs` (257): Page background tasks under the test client. Entry: `BgWorker` 11.
- `pages/filter.rs` (135): `TestPage.Filter` / `TestPart.Filter` (the `TestFilter` handle).
- `pages/parts.rs` (324): Page parts (ADR-012 follow-up).
- `pages/request.rs` (571): The request page of a report. Entry: `RequestClosed` 18, `action_id` 532.
- `pages/runobject.rs` (321): `RunObject` actions (ADR-012 follow-up).
- `partial.rs` (271): Partial records (SetLoadFields, LoadFields). Entry: `note_partial_fetch` 54, `load_fields_op` 122, `load_fields_row` 212.
- `platform/apps.rs` (67): The tables that list the apps of the tenant.
- `platform/date.rs` (361): The `Date` table (2000000007).
- `platform/fields.rs` (264): The Field and Key platform tables. Entry: `field_row` 116, `shell_entries` 65.
- `platform/host_tables.rs` (145): Tables whose rows are facts about the host.
- `platform/identity.rs` (67): The Session and Active Session tables.
- `platform/ids.rs` (45): Identifiers the runtime makes up for what a service tier would assign. Entry: `package_id` 11.
- `platform/integer.rs` (83): The `Integer` table (2000000026).
- `platform/links.rs` (225): `Record.AddLink`, `HasLinks`, `DeleteLink`, `DeleteLinks` and `CopyLinks`.
- `platform/mat.rs` (259): A computed table whose rows are listed in full once, then scanned like a stored one. Entry: `Entry` 11, `Mat` 29.
- `platform/mod.rs` (279): Row providers for the platform's system tables (ADR-013 decision 7, ADR-019). Entry: `Ident` 40, `Providers` 74, `pos` 181, `blank` 189.
- `platform/objects.rs` (480): The tables that list the compiled objects.
- `platform/pages.rs` (391): The page platform tables. Entry: `source_table` 36.
- `platform/permissions.rs` (572): The permission set tables.
- `platform/ranges.rs` (164): Filters on one integer-like field as a sorted set of closed intervals, for the generated tables (Integer, Date), whose r. Entry: `Iv` 8, `Set` 11, `intersect` 26, `of_expr` 60.
- `platform/seed.rs` (73): The rows a tenant holds before any AL runs, for the stored platform tables whose rows AL reads as facts about the sessio.
- `queries.rs` (881): Query objects: Open, Read, column values, SaveAs. Entry: `query_op` 107, `query_column` 350.
- `recmeta.rs` (734): Record builtins that read table metadata or guard on session state. Entry: `string_literal` 29, `mentions_table` 49, `is_blank` 60.
- `records.rs` (1565): Record.* on the store: Insert, Modify, Delete, Get, Find, filters, triggers, events. Entry: `record_op` 733, `table_event` 205, `run_trigger` 499, `delete_row` 297, `scan` 545, `unique_violation` 346.
- `refs.rs` (637): `RecordRef`, `FieldRef` and `KeyRef` on the VM. Entry: `record_ref_value` 42.
- `relations.rs` (179): TableRelation: lookups and Validate checks. Entry: `field_relation` 70, `validate_relation` 94.
- `rename.rs` (214): Rename propagation to related tables. Entry: `rename_references` 33, `propagate_rename` 86.
- `reports.rs` (1209): Report objects at run time: dataset walk, triggers, request page, `Report.SaveAs` and the dataset rows. Entry: `report_builtin` 80, `first_use` 314, `report_save_as` 936, `dataset_begin` 1109.
- `sessionops.rs` (422): Builtins that act on session state. Entry: `Sequence` 11.
- `testdata.rs` (420): Test data (ADR-020): `TestBacking` maps a provider's columns onto the compiled table's fields, rebuilds the values and builds the table once per thread. Entry: `TestBacking`, `convert`, `sql_name`.
- `transactions.rs` (446): Transactions as AL code sees them. Entry: `TxModel` 15, `CommitBehavior` 27, `codeunit_run` 193.
- `ui.rs` (347): Dialogs raised by the code under test (`Message`, `Confirm`, `StrMenu`) and the test handler functions that answer them. Entry: `UiKind` 18, `UiHandler` 66, `dispatch_ui` 158, `dispatch_ui_returning` 163.
- `views.rs` (510): Filter and view text: SetView, GetFilters, filter group text, AL property views (`property_view`). Entry: `parse_filter_text` 28, `filter_text` 50, `get_filters` 117, `property_view`.
- `xmlports.rs` (1304): XmlPort import and export (Format = Xml).

## Flows

Each chain is `caller -> callee` in call order; `[a, b]` means both happen in that order. Names are
functions or methods; the file is in the module list above.

### Corpus run: from main to a test outcome

1. `main` (al-test-cli/main.rs) starts a thread with a large stack and calls `run`.
2. `run`: `args::parse` -> `config::OracleFile::parse` (oracle/oracle.toml) -> per BC version `run_version`.
3. `run_version`: `pin::check` -> `oracle::parse_junit` (cases of real BC) -> `corpus::discover` ->
   `select::Selection::filter` / `find_explained` -> `runtime::load_app` per app -> `deps::load_with`
   (Microsoft apps from `.alpackages`, `--deps` or the artifact feed) -> `CorpusRuntime::run_suites_with`.
4. `run_suites_with`: `al_sema::lower::compile_apps_with` (compile pipeline below) -> `Engine::with_config`
   (`events::Registry::build`) -> `global_triggers_find_unavailable` -> `baseline::run` (install roots, once) ->
   for each test codeunit `run_codeunit_isolated` -> `run_codeunit_observed`. Test codeunits run on worker threads
   (`AL_TEST_THREADS`, `run_unit`); a baseline cannot cross threads, so each worker runs the install roots again
   on the engine that ran them first. `--explain` keeps one thread. `--timings` prints the compile parts and the
   slowest codeunits.
5. `run_codeunit_observed`: `Session::with_host` -> `restore_baseline` -> `run_install_codeunits` -> `OnRun` via
   `Session::call` -> per `[Test]` routine: `resolve_handlers`, `blocked_site` (refusal decided at lowering),
   `bind_handlers`, then `Session::call` -> `call_inner` -> `exec::compile` (once per routine) -> `Session::run_chunk`. The result becomes
   `Outcome::{Pass, Fail, NotRun}` (`Signal::Error` is a failure, `Signal::Unsupported` is a refusal).
6. Back in `run_version`: `report::build` (counts by area) -> `output::junit` / `markdown` -> `report::ratchet` and
   `decide` against `metrics/<major>/passing.txt` -> `format_baseline` with `--update-baseline`.
   Reading a run: `insight::top_refusals`, `insight::baseline_diff`, `explain::render`.

### `al-runner test`: from the command line to a result

1. `main` (alrunner-cli/main.rs) starts a thread with a large stack; `args::parse` returns `Command::Test`.
2. `run::test`: `apps::load` per APP (`al_deps::load::read_package_at_with`: folder, `*.Source.zip` or `.app`) ->
   `discover::test_codeunits` over each APP's files (which test codeunits exist, which the selection picks) ->
   `al_version::choose_target` -> `apps::combined_project` (the APPs' dependencies, plus the `Application` and `System`
   floors as declared Microsoft dependencies) -> `deps::load_project` (search list: `--dep`, `--package-cache`, each folder
   APP's `.alpackages`, then the artifact feed; `al_deps::load::scan_path` reads a directory, a package file or an app folder;
   a Microsoft package in `.alpackages` older than the run's BC version gives way to the feed, `artifact::load_with_artifacts`).
3. An APP with an `application` or `platform` floor gets the dependencies it implies (`Dependencies::implied_dependencies`),
   so it sees Base Application without declaring it.
4. `CorpusRuntime::run_suites_with` (the same compile and run as the corpus job; see "Corpus run" step 4), with
   `RunOptions::only` set to the selected codeunits. `CorpusRuntime::results` lists every test with its codeunit.
   Test data is on unless `--no-test-data`: `run::choose_test_data` finds the backup (`testdata::locate`: the build of the
   Microsoft apps on disk, in the artifact cache, downloaded by `al_deps::demo::ensure_demo_backup`, or another build of
   the version) and opens it (`al_bcdb_std::BackupProvider`); without a backup the run says why test data is off. The
   build of the tests (`apps::own_build`) is `BuildQuery::preferred`, so the apps, `System.app` and the data share it.
   The backup is passed as `RunOptions::test_data`; `run_suites_with` puts it on
   each `Engine`. `Session::with_host` gives the store a `TestBacking` (al-vm/testdata.rs): a table the store holds no
   entry for reads `provider.read` once for all threads (`TestData::read`), maps columns to fields, and builds the
   table once per thread. `run::test` prints `LoadStats::summary` before the test summary.
5. `run::test` filters to the selected tests, prints failures, `refusals::rank` / `render_top`, the summary line and the
   timing line, writes `output::junit`, and returns `output::exit_code`.

### Compile pipeline: parse -> workspace -> demand -> check -> lower

1. `lower::compile_apps_with` -> `compile_with_roots`.
2. `Workspace::load_apps` (al-sema/workspace.rs): workspace apps are parsed (`parse_file`, in parallel,
   `pages::expand_ranges` first for page expressions); dependency apps are only scanned (`scanned_file` ->
   `al_syntax::scan::scan`) and parsed later on demand (`Workspace::green`). Then `index_extensions_and_subscribers`
   and `resolve_references`.
3. `members::build`: fields (extensions merged), enum values, procedure signatures.
4. `check::check_with`: `check_object` for every workspace object (a test selection leaves unselected test codeunits
   out: `Roots::only_tests`), then `reach_baseline` (install roots) and `run_demand` (check/demand.rs):
   `activate` / `activate_table` / `activate_ops` / `activate_global` queue routines, `check_reached` checks them;
   files of newly reached dependency objects are parsed ahead on worker threads. Result: `CheckResult`
   (types and resolutions by node position, `Demand`). The demand loop runs in rounds of tasks (check a workspace
   object or a reached routine, activate a raised object); worker threads (`AL_CHECK_THREADS`) take chunks of a
   round, each with its own `Checker`, and report deltas that the calling thread merges (`demand.rs`: `Task`,
   `Pool`, `Delta`). Types and resolutions are a position map in 16 shards merged one thread per shard.
5. `lower::lower_with_positions`: `Workspace::restrict_to` (what demand did not reach stops resolving), then per object
   `Lowerer::routine` -> `stmt` / `expr` / `call` for reached routines and `stub` for the rest. Anything inexpressible
   becomes a refusal reason in `Lowered::unsupported`. Output: `al_ir::Program` (an `Arc`, shared with the engine) +
   call graph. `lower_object` is a function of the plain `Shared` data plus per-thread `Local`/`Seq` (syntax nodes
   and layouts belong to the thread that built them): worker threads (`AL_LOWER_THREADS`) and the calling thread take all active objects in turn,
   each lowered with positions numbered from 0 that `Routine::shift_positions` moves into place in object order.

### Compile cache: a run that finds the dependency apps already lowered (ADR-022)

1. `CorpusRuntime::try_run_suites_with` (testrun/runtime/corpus.rs), with `RunOptions::compile_cache`: the apps are put in the
   program order a cache needs, dependency apps first, Microsoft's before the others and every app after the apps it
   depends on (`compile_cache::dependency_order`; the caller's order is kept in `order` for `TestResult::app`,
   `SuiteRun::app` and `RunOptions::only`). No cache with `AL_EAGER_DEPENDENCIES` or without dependency apps.
2. `compile_with_cache` -> `compile_cache::open_or_build_with`: the dependency set is cached in layers (owner decision
   2026-10-15): `layer_ranges` (layer 1 = the Microsoft apps, then one layer per other app), `layer_keys` (each layer's key
   holds its own apps and the digest of the key below: executable hash, target, symbols, manifest and source hash) ->
   `read_entry` per layer (`al_cache::Entry::open` checks header, index, dictionary; a refused file is removed and
   reported). Each missing layer is built, lowest first: layer 1 = `lower::lower_for_cache` (the complete compile of the
   Microsoft apps), a higher layer = `lower::lower_layer` (compiles the layer's apps with the layers below taken from a
   `provided::Stack`, keeps its own objects and the lower objects it changes) -> `al_cache::build_layer` -> `write_entry`.
3. `lower::compile_loaded_cached` on the `Stack` of all layers (an object comes from the highest layer that holds it):
   `Workspace::load_apps` for all apps; `provided::check_identity`; `relower_set` (the
   dependency objects whose `DepFacts::embeds` the workspace extends or implements, with the objects that extend each
   other); the blocks of the others are read in parallel (`DepSource::block`); the checker skips them
   (`Roots::provided`); `lower_core` takes their pieces from the blocks and their facts (`Shared`: page reasons, parts,
   routine numbers, ...) from `DepFacts`, and lowers the rest as the eager compile does. An error gives the workspace
   back and the run compiles without the cache.
4. `Lowered::calls_of` / `calls_into` give the call graph: the direct calls stored in the block plus what a page,
   report or xmlport variable drives (`driven_edges`).

### Record.Insert(true): trigger, events, Global Triggers, store

1. `Session::eval` (al-vm/lib.rs) on `Expr::Builtin("Record.Insert#2")` -> `al_builtins::native::lookup` -> the entry
   in `native.rs` calls `Ctx::record("Insert", ..)` -> `Session::record` (lib.rs) -> `records::record_op`, arm `"Insert"`.
2. `record_op`, in order: `auto_increment_field` -> `table_event("onbeforeinsertevent")` (`listened`, then
   `dispatch::raise`) -> `assign_system_id` -> with RunTrigger: `run_ext_trigger("OnBeforeInsert")`,
   `run_trigger(OnInsert)` (`Session::call`), `run_ext_trigger("OnInsert")` -> `globaltriggers::global_trigger(Insert)`
   (`global_setup`, then `raise` on the platform codeunit `Global Triggers`, event `ondatabaseinsert`) ->
   `check_system_id_unique` (Insert(_, true) keeps the buffer's SystemId) -> `bump_row_version`, `stamp_insert` ->
   `take_auto_increment` -> `unique_violation` (duplicate key fails here, `fail_unique`) ->
   `store_for(rec).insert` (al-data `Store::insert`, `MemStore`) -> `run_ext_trigger("OnAfterInsert")` ->
   `table_event("onafterinsertevent")`.

### TestPage open

1. `Session::eval` on `Expr::Builtin("TestPage.OpenEdit#1")` -> native entry -> `Ctx::page` -> `pages::page_op`
   (wraps a page transaction: `in_page_transaction`) -> `page_op_inner` -> `test_page_op` -> `test_page_op_inner`
   (`"OpenEdit"`, `"OpenView"`, `"OpenNew"`) -> `page_open` -> `open_page` -> `open_page_as`.
2. `open_page_as`: `page_instance` / `construct_page` (instance of the page object) -> `page_trigger("oninit")` ->
   UI handler lookup (`ui.rs`, `resolve_handlers` in the runner) -> `check_page_events` ->
   `page_trigger("onopenpage")` and `page_event("onopenpageevent")` -> `load_first_row` -> `populate_current` /
   `populate_window` (control expressions, FlowFields). Controls are then read and written through `test_page_op_inner`.
3. Closing: `page_close` -> `close_with` (OnQueryClosePage, OnClosePage, saving values).

### Event raise

1. A call to a publisher routine: `Session::call` sees `engine.events.publishers` holds `(object, routine)` ->
   `dispatch::publish` (converts parameters, builds `EventCall`) -> `raise`.
   Table events build an `EventCall` themselves (`records::table_event`), `Global Triggers` events too
   (`globaltriggers::global_trigger`), page events via `pages::page_event`.
2. `raise`: `event_targets` (subscribers from `engine.events.subscribers[(object, event)]`, static or bound instances
   via `static_instance` / `bound_instances`; a Global Triggers subscriber that cannot run refuses here) -> per
   subscriber map parameters by name -> `Session::call(subscriber)` -> `var` values are written back into the arguments.
3. The registry is built once per program: `events::Registry::build` reads the `EventSubscriber` attributes of the IR.
   At compile time the same relation is `Workspace::subscribers_of`, which demand uses to reach subscribers.

### Guarded Codeunit.Run

1. `if Codeunit.Run(..) then` lowers to `Expr::Builtin("Codeunit.Run#1" | "#3")` with `ret_used = true`.
   `Session::eval` (lib.rs, the `Codeunit.Run#` branch) evaluates receiver and arguments, then
   `transactions::codeunit_run(id, recv, args, guarded = ret_used)`.
2. `codeunit_run`: unguarded -> `bg_refuse_transaction_call`; resolve the codeunit (`shared_instance` / `instance`);
   refuse a test codeunit started from a test (`nested_test_codeunit`); TableNo codeunits take the record as `Rec`;
   guarded and `in_write_transaction` -> `transaction_stopped()` (BC refuses a guarded run over an uncommitted write).
3. `Session::call(object, OnRun)` inside `tx_world += 1`. `Ok` + guarded -> `commit_transaction`, returns true.
   `Err(Signal::Error)` + guarded -> `add_top_frame` (call stack text), `rollback_transaction`, `last_error` set,
   returns false. Unguarded errors propagate. `Signal::Unsupported` always propagates (a refusal, not a failure).

## Reading a failure or a refusal

`al-test-cli --explain TEST` (explain.rs) shows the outcome, the reason and, with `Session::trace_faults`
(al-vm/faultsite.rs), the call stack the error or refusal left from, each frame with `file:line:col`
(`Program::positions`, docs/architecture.md "Source positions").
