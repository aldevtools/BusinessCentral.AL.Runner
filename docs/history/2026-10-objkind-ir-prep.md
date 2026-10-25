# objkind-ir: preparation note (ADR-032 step 1)

Status: prep only. No source file was edited. Counts come from a text scan, not the compiler.
Treat every count as an upper bound; the compiler finds the exact set during the rewrite.

## Goal

`al_ir::Object::kind` is a `String` (`crates/al-ir/src/lib.rs:160`). Move `ObjKind` into `al-ir`
and make the field typed. Invented kinds (`"uncompiled report"`) become a flag.

## Where the strings come from

- **One producer of the invented kind:** `crates/al-sema/src/lower/shared.rs:240`,
  `format!("uncompiled {}", o.kind.label())`. Every other producer copies `ObjKind::label()`.
- **One strip helper per consumer:** `base_kind` in `al-vm/src/platform/objects.rs:21` and a closure
  in `platform/profiles.rs:136`. Both vanish once `uncompiled` is a flag.
- **Source enum today:** `al-sema/src/workspace.rs` (`label`, `from_keyword`, `from_type_name`,
  `extended`); a second text mapping in `al-sema/src/alcodes.rs:354`.

## ObjKind in al-ir

16 variants. `Display` and `FromStr` must give today's strings exactly.

| Variant | String |
|---|---|
| Table | `table` |
| TableExtension | `tableextension` |
| Page | `page` |
| PageExtension | `pageextension` |
| Codeunit | `codeunit` |
| Report | `report` |
| ReportExtension | `reportextension` |
| Query | `query` |
| XmlPort | `xmlport` |
| Enum | `enum` |
| EnumExtension | `enumextension` |
| Interface | `interface` |
| PermissionSet | `permissionset` |
| PermissionSetExtension | `permissionsetextension` |
| Profile | `profile` |
| Other | `object` |

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ObjKind { Table, TableExtension, /* ... in the table's order */ Other }
impl ObjKind { pub const fn label(self) -> &'static str { /* table above */ } }
impl std::fmt::Display for ObjKind { /* f.write_str(self.label()) */ }
impl std::str::FromStr for ObjKind { type Err = UnknownKind; /* exact inverse of label */ }
```

- Keep `from_keyword`, `from_type_name`, `extended` in `al-sema` as functions or an extension
  trait. They depend on AL syntax, not on the IR.
- `al-sema` re-exports `al_ir::ObjKind`, so its 46 sites compile unchanged.
- Add `Object::uncompiled: bool` (`#[serde(default)]` is not needed; the cache format is bumped).
- Consumers that need the old text call `kind.label()`; with `uncompiled` they build
  `format!("uncompiled {}", ..)` only in the compile_dump mapper (see Proof).
- Round-trip test: for every variant, `label` then `FromStr` returns the variant; unknown text errors.

## Cache format bump

- `al_cache::FORMAT` in `crates/al-cache/src/key.rs:7`, now 6. Bump to 7.
- Reason: `Object` is stored with postcard. `String` to enum (variant index) plus a new bool changes
  the byte layout. Old entries must be a clean miss.
- The compiler change alone already changes `Key::runtime_build`; the bump is for the layout rule in the
  doc comment. Variant order is part of the format: append only after this step.
- Check: the cache differential (`cache_vs_uncached`) shows 0 differences, and a test loads a
  format-6 file and sees a miss, not an error.

## Site inventory (upper bounds, strings that match an object-kind label next to `kind`/match arms)

| Crate | Part | Sites | Notes |
|---|---|---|---|
| al-vm | src | 98 | main work; per-file below |
| al-sema | src | 46 | mostly `ObjKind` already; the one `format!("uncompiled ..")`, `alcodes.rs` |
| al-testrun | src | 11 | `reach.rs`, `runtime.rs`, `observe.rs` |
| al-test-cli | tests | 2 | compare kind text |
| alrunner-cli | src | 2 | `serve.rs` keywords; most are AL keywords, not IR kinds |
| al-bcdb | tests | 2 | probably unrelated (BC database names) |
| al-deps | tests | 1 | |
| al-ir | src | 1 | the field |
| al-bcdb-cli, al-builtins, al-syntax | src | 3 | probably unrelated, confirm |
| **Total** | | **166** | |

The earlier "about 240" counted every line with a kind word, including AL keyword tables and
`.expect("table")`. ADR-032 counted 104 string compares. The compiler will say; expect 100 to 170.

### al-vm, by file (line numbers on origin/main today)

- **Compare `o.kind == ".."`:** `lib.rs:2454`; `dispatch.rs:157`; `pages.rs:47,466,484,491,3843,3925,3967`;
  `pages/background.rs:81,88`; `platform/mod.rs:203`; `platform/links.rs:77`; `platform/permissions.rs:163,533`;
  `platform/metadata.rs:139,152`; `reports.rs:374,447,455,516,538`; `transactions.rs:230,239`;
  `xmlports.rs:138`; `refs.rs:254`.
- **Tuple patterns `(Some(id), "table")`:** `platform/{coverage,fields,metadata,mod,objects,pages,seed}.rs`,
  about 20 sites; these use `o.kind.as_str()` or `base_kind(o)`.
- **`base_kind(o) == ..`:** `platform/{objects,permissions,reports,profiles}.rs`, about 12 sites.
- **Match on kind text:** `lib.rs:1585-1586`; `events.rs:185-190,220,236,246` (also keys a map by
  `(&str, id)`); `callstack.rs:81-86` and `dotnet/mod.rs:340` (display names, add `ObjKind::pascal()` or keep local match);
  `platform/objects.rs:32-67,278-300` (option ordinals; becomes a `match ObjKind`);
  `platform/permissions.rs:256-262` (this maps permission-set words, keep text but parse to `ObjKind`);
  `records.rs:530`; `reports.rs:552`; `dispatch.rs:90-94`.
- **Argument kind text:** `.id_of("table", v)` in `platform/{objects,pages}.rs` (4 sites): change the parameter to `ObjKind`.
- **Not sites:** `.expect("table")` (about 40 lines in flowfields, queries, records, relations, rename).

## Rewrite order (each step compiles and passes on its own)

1. **al-ir:** add `ObjKind`, `uncompiled`, round-trip test. Keep `kind: String` for now, nothing else changes.
2. **al-sema:** replace its enum with the re-export; fix `alcodes.rs`; lowering fills both fields.
   Still write the string field too. Add a temporary `Object::kind_text()`.
3. **al-vm, one file at a time:** compare via `kind_text()` first only if a file is large; otherwise
   go straight to the enum. Order: `platform/objects.rs` (drops `base_kind`), `platform/*`, `pages*`, `reports`,
   `events`, `lib.rs`, rest.
4. **al-testrun, CLIs, tests:** same rewrite.
5. **Flip the field:** `Object::kind: ObjKind`, delete `kind_text()`, bump `al_cache::FORMAT` to 7.
6. **Docs:** `docs/codemap.md`, ADR-032 status.

Steps 1-4 can be one commit each; step 5 is the only one that breaks the cache.

## Proof

1. `compile_dump` before and after is byte-identical once the typed fields are mapped back to the old
   text (`uncompiled` plus kind becomes `uncompiled <label>`). The mapper lives only in the example.
2. `tools/compare_junit.py` over `--all-versions --no-ratchet --junit` shows identical outcomes on every major.
3. `tools/demand_vs_eager.sh run`, `tools/cache_vs_uncached.sh`, `tools/baseline_rule_check.sh run`: 0 differences.
4. `./check.sh` green; `perf_vm` and `perf_budget` instruction counts not worse than +1 %.

## Risks

- A typo-tolerant text match (`o.kind.starts_with("uncompiled")`) hides in a helper. Grep `uncompiled`
  and `strip_prefix` after the rewrite; both should return nothing.
- `Other` ("object") must stay reachable from `FromStr` or cached programs holding it fail to load.
- Lane C owns `al-sema` and lane D `al-data`: run this only after both merged.
