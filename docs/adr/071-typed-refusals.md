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
