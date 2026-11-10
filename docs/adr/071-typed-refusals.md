# ADR-071: Typed refusals with a stable key and a probe text

- Status: Accepted (owner, 2026-11-10)
- Date: 2026-11-10

## Context

Hard rules 3 and 4 say: refuse what no corpus test pins, and say so explicitly.
Today a refusal is free text. Nothing in the types forces it, groups it or links it to a probe.

Numbers from the architecture review (2026-11-10, commit `bf4592322`):

- `Signal::Unsupported(String)` carries the reason. Lowering uses `Result<T, String>`.
- About 2,500 run-time refusal sites. Each builds its own sentence with `format!`.
- 69 private copies of `fn unsupported` in `al-vm` and `al-builtins` (a fresh grep finds 80 with tests included).
- 83 hand-written `*_pinned` functions. Each is one list of pinned shapes.
- 52 commits in two weeks with the subject "Refuse ...". Someone noticed one more shape each time.
- 19 commits "... not with the debug format". A test (`readable_messages`) finds `{:?}` only after the fact.
- `docs/probes-needed.md` has 869 table rows of prose. 424 commits edited it. No row links to the refusal it documents.
- About 1,700 places in tests match a refusal by its wording. A reworded reason breaks them.
- `--top-refusals` folds quoted names and numbers with a regex over the text. It is a guess at the shape.

What already works: `enum Assumption` (one file per entry, compiler-checked, generated doc table) and `Gate` with `Evidence`.
Deleting an entry breaks every use at compile time. Refusals have no such thing.

## Decision

One typed refusal API, used everywhere a refusal is made.

1. **A refusal has fields, not a sentence.**
   - `key`: stable id of the refused shape, such as `page.part.update-propagation-both`. Dotted, lower case, never reworded.
   - `area`: derived from the key prefix (`page`, `record`, `dotnet`, ...). Used for reports.
   - `probe`: the corpus probe spec or issue the shape waits on. One sentence ("a ListPart with UpdatePropagation Both when CurrPage.Update runs") or an issue reference with its plain description. A refusal with no probe yet says so with an explicit `probe = Wanted`.
   - `reason`: the text for the user. Built from typed fields (see 3). Never a debug format.
2. **A compile-time registry, like the assumption registry.** One `enum RefusalKey` (or one entry per key), one small file per entry, checked by the compiler and by a test (no duplicate key, no entry without a probe field, no unused entry). The probe part of `docs/probes-needed.md` is generated from it, as the assumption table is.
3. **Arguments must implement `Describe`.** The `refuse!` macro wraps each argument in a type that has `Display` and no `Debug`. A debug format does not compile. The `readable_messages` scan can then go.
4. **One constructor for both sides.**
   - Static refusals (lowering reasons, `al-sema` diagnostics) build a `Refusal`.
   - `Signal::Unsupported(String)` becomes `Signal::Unsupported(Refusal)`.
   - Both use the same key space. The same shape refused statically and at run time has one key.
   - The 69 private `fn unsupported` copies become one function.
5. **Old sites keep compiling.** `From<String>` for `Refusal` gives the key `legacy`. Legacy sites render as today.
6. **The compile cache stores `Refusal`, not strings.** The cache format version is bumped once (ADR-022). Cached reasons from an older version are a cache miss.
7. **Tests assert the key.** A Rust test checks `key == "..."`, not the wording. Existing tests that match wording stay valid while their site is legacy and move to the key when the site migrates.

How reports use the key:

- `--top-refusals` groups by key. The folding regex stays only for legacy sites.
- JUnit and JSON refusal reasons carry the key as a separate field next to the text.
- An **unpinned-shape report** lists keys whose `probe` is `Wanted` or names an open issue, with a count of tests behind each. It replaces most hand edits of `docs/probes-needed.md`.
- A key whose probe has landed is removed together with the refusal, in the PR that implements the shape (hard rule 3).

Seam note (hard rule 5): the payload of `Signal::Unsupported` and of lowering reasons changes. This is a seam change, so this ADR is required. The change is mechanical and keeps behavior. Outcomes of tests do not change.

## Migration plan

Small PRs, one area at a time. Each PR is behavior-preserving.

1. **PR 1: the API.** `Refusal`, `refuse!`, `Describe`, the registry, `From<String>` as `legacy`, one shared `unsupported` helper, cache version bump, `--top-refusals` grouping by key with the regex as fallback.
2. **PR 2: the ratchet.** A test that counts free-text sites (a `Signal::Unsupported(` construction or an `unsupported(` call with no key, plus the private copies of `fn unsupported`). The count goes in `metrics/refusal-free-text.toml` per area. It only goes down. Same pattern as `metrics/fn-lengths.toml`. Never raised without the owner.
3. **PRs 3 and on: one area each**, in this order: the areas with the most refusals in the latest top-refusals run first (record, page, dotnet, builtins by folder). Each PR:
   - gives every refusal site in the area a key and a probe field,
   - moves the matching `docs/probes-needed.md` rows next to the key,
   - moves tests of that area from wording to key,
   - lowers the area's ratchet count to 0.
4. **Last:** the ratchet counts legacy sites as 0 everywhere, `From<String>` is deleted, and a new refusal can only be made with a key.

An agent that touches a refusal site in an area not yet migrated may migrate that site. It must lower the count in the same PR.

## Out of scope

- **Refusing by shape key automatically** (default-deny from an allow-list generated from the corpus run, as the review proposes in section 3.3). It changes outcomes and pass counts. It needs its own ADR, after the unpinned-shape report gives numbers on one Microsoft suite and NP Retail.
- Changing any refusal text, key meaning or test outcome in the API PRs. Error texts still come only from the corpus (hard rule 3).
- The `*_pinned` functions and the unpinned-error-text mark (U+FDD1). They are the next step after this one, and the key gives them a place to attach.
- Assumptions and gates. They stay as they are.

## Consequences

- Hard rule 4 gets one visible place: every refusal has a key, an area and a probe.
- The debug-format bug class cannot compile.
- Rankings and tests no longer depend on wording.
- The probe backlog is generated from code, so it cannot drift from the refusals.
- Cost: one touch of about 2,500 sites over many PRs, one cache version bump, and a short period where `legacy` and keyed sites exist together.
- Risk: the folded keys of `--top-refusals` change once. Ranks are compared across the change by area, not by key.

## Conformance

- A registry test: unique keys, every entry has `probe`, no unused entry, key prefix matches an area.
- A compile-fail test (or `trybuild`-style check): a `Debug`-only argument in `refuse!` does not compile.
- `refusal_free_text_ratchet` (PR 2): counts per area, only goes down.
- A cache test: a stored refusal round-trips with its key.

## Implementation notes (step 1, PR 1a: the type and the seam)

What was built, and where it differs from the text above. None of this changes a decision.

- **Key lists (owner decision, 2026-11-10).** Decision 2 allows "one `enum RefusalKey` (or one entry per key)". The
  owner chose **one list per crate that refuses**, one file per key in `crates/<crate>/src/refusals/<key>.rs`, collected
  by that crate's `build.rs` into one `refusal_keys!` call (`keys::NAME` constants and `ALL`). One test in `al-test-cli`
  (`tests/suite/refusal_keys.rs`) checks all lists together (unique keys, lower case and dotted, known area, no `legacy`
  entry, one file per key, every key used as `keys::NAME`) and that the generated table in `docs/probes-needed.md` is
  current (`al-test-cli --refusals-doc` rewrites it). A crate that starts a list adds it to
  `al-test-cli/src/refusal_keys.rs` `LISTS`. Reason: a new key in the bottom crate rebuilds every crate; a key in
  `al-vm` rebuilds five. A key that lowering and the VM both use sits in `al-builtins`. Step 1 has one list
  (`al-builtins`) with one key.
- **`Refusal`** (`al-builtins/src/refusal.rs`) is the size of a `String`: free text is stored as its `String` (no
  allocation more than before), a keyed refusal as a `Box` of key and text. `Signal` does not grow (a test pins `Signal`
  at 40 bytes and `Result<Value, Signal>` at 48, "not larger than"). The plan's first layout, one `Box` for every
  refusal, cost `CallLoop` 0.4 % after ADR-072 step A merged; this layout is the plan's fallback and costs nothing
  measurable. `key()`, `area()` (the key up to the
  first dot), `reason()`, `into_reason()`, `map_reason()` (keeps the key behind a site prefix). There is no
  `From<Refusal> for String`: it would overlap the blanket `From<S: Into<String>>`; owned text is `into_reason()`.
- **Decision 5, "old sites keep compiling"**, is not true for a construction that passes a `String` straight to the
  variant. Those 349 sites became `Signal::unsupported(..)` (`#[cold]`, `#[inline(never)]`), one character each, so no
  line moved and no cap changed. `Signal::Unsupported(format!(..))` no longer compiles; patterns are unchanged.
- **Legacy reads as before.** `Display` is the text; `Debug` of a legacy refusal is the `Debug` of its `String` (tests
  compare the `Debug` of `Signal`); `Deref<Target = str>` and `==` with `str` and `String` keep the about 290 readers
  that match wording. `Deref` and the comparisons are shims and go with `From<String>` in the last step.
- **Serialization.** postcard (the compile cache): the pair (key, text). JSON: a legacy refusal is its plain text and a
  keyed one `{"key", "reason"}`, so `compile_dump` of `main` and of the branch are identical.
- **`refuse!` and decision 3.** `format!` reads a named placeholder (`{x:?}`) from the scope, past any wrapper. `refuse!`
  therefore takes positional placeholders only, checked at compile time (`positional_only` in a `const` assertion);
  each argument is wrapped in `Shown`, which has `Display` through `Describe` and no `Debug`. Three `compile_fail` doc
  tests show it (a `{:?}`, a `Debug`-only argument, a named placeholder). The `readable_messages` scan stays until the
  last step.
- **Decision 4, one helper.** `al_builtins::native::unsupported` takes free text or a keyed refusal. The 67 private
  copies of `fn unsupported` without a text prefix are deleted. Kept: the two that add a prefix (`Tenant Profile
  Setting: `, `Tenant Report Layout Cfg: `) and two private helpers named `refuse` (`al-vm` `package_data`,
  `datatransfer`), which now take `impl Into<Refusal>`. PR 2's count includes these four.
- **Static refusals (decision 4) and the cache (decision 6), storage only.** `Lowered::unsupported`,
  `DepFacts::unsupported` and the assembler's pieces hold `Refusal`; the lowerer keeps `Result<T, String>` and its
  `Vec<String>` reasons, which become legacy refusals where they leave it. The other reason strings in the IR and the
  facts (`Expr::Refused`, the page, report and xmlport reasons) change with their area; each such change bumps the
  cache `FORMAT` again (19 to 20 here). `Engine::blocked`, read on every call, stays a `String` table until the first
  static key, which needs a CallLoop measurement.
- **Not in PR 1a.** `Outcome::NotRun` carrying the `Refusal` and `--top-refusals` grouping by key need the key in the
  outcome, which the seam note did not name: they are PR 1b. The key as its own field in JUnit and JSON, and the
  unpinned-shape report, are later pull requests (they change the JUnit bytes).
- **Areas** (`AREAS`): the corpus folder names in key form plus `dotnet`, `builtin`, `compile` and `runner`. A
  `Probe::ByDesign` for refusals that wait on nothing (time limit, memory limit) is added when the first such site gets
  a key.
- **ADR-072.** Its step 5 moves `Signal` and `Describe` to `al-value`; `refusal.rs` moves with them.
- **Proof site.** `xml.sibling.attribute-or-declaration` (`al-builtins/src/xml/edit.rs`, `position_in_parent`), text
  unchanged.
- **Speed.** `perf_vm` instructions per round, `main` against this branch: no bench more than 0.2 % higher (table in the
  pull request).
