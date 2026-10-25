# ADR-016: Release: AL Runner v3

- Status: Proposed
- Date: 2026-10-13

Repo: al-runtime-rs. This ADR sets how the runtime ships as **AL Runner v3**:
the user command line, the binary name, the build and release pipeline, the
.NET helper, versioning, WebAssembly, migration from v2, and the gates for a
preview and for v3.0. The feature list it must cover is in
`docs/al-runner-v2-parity.md`. Nothing here changes a crate seam. It adds a
front end and a release process. It needs the owner's review of the open
questions at the end.

## Context

### Owner's direction (2026-10-13)

- al-runtime-rs becomes **AL Runner version 3**. AL Runner v2 is the C# tool in
  `BusinessCentral.AL.Runner`.
- **Feature parity.** v3 supports everything v2 offers today. Commands, flags
  and calls may differ. A migration step and a migration guide are acceptable.
- **Release shape.** One self-contained executable per OS (Linux, macOS,
  Windows). Optionally the .NET helper (ADR-014, Proposed). Microsoft apps are
  never bundled (ADR-013).
- **WebAssembly** is of strong interest.

### Where v3 stands

- The only command line is `al-test-cli`. It runs the corpus against the oracle
  and checks the ratchet. It cannot run a user project. It loads two fixed apps
  (`crates/al-test-cli/src/main.rs`).
- Corpus result in `docs/status.md`: 2412 of 4836 pass, 0 fail, 2424 not run
  (49.9%). A binary in the main checkout reported 2885 (59.7%) on the same day,
  so the number moves quickly. Use the live number at gate time.
- Measured today: the corpus run takes 3.6 s wall clock cold from disk and
  peaks at 124 MB resident. The release binary is 5.2 MB. Compile alone is
  about 0.5 s (ADR-006 budget: 1 s).
- Dependencies (ADR-013, accepted on branch `deps`): the `al-deps` crate has
  version selection and a ZIP reader with CRC checks. No download, no cache, no
  Microsoft source loaded yet.
- `al-host` is an empty placeholder (`crates/al-host/src/lib.rs`).
- ADR-006 says CI builds the core crates for `wasm32-unknown-unknown`. No CI job
  does (`.github/workflows/ci.yml` has one job and no wasm step).
  `al-sema` uses `rayon` (`crates/al-sema/src/workspace.rs:5`), and
  `al-test-cli` starts the VM on a thread with a 512 MB stack
  (`crates/al-test-cli/src/main.rs:56`). Both matter for WASM.

### Where v2 stands

v2 is 2.12.0, a .NET global tool (`MSDyn365BC.AL.Runner`, command `al-runner`).
It needs the .NET SDK 9 or 10, downloads gigabytes of Microsoft artifacts, and
ships one engine variant per BC minor (27.0 to 28.5). A two-test project took
14 s warm and 54 to 77 s with cold caches in a run I made today. Its public
surface has 118 features (parity page). It is used by the VS Code extension
ALchemist (`--server`, `--dap stdio`) and by LethAL (`--server`, `--output-json`).

## Decision

### 1. Identity and scope

- v3 is the next major version of AL Runner. It is one product with one name.
- Feature parity is measured against the parity page. A row leaves that page
  only as `Done`, `N/A` with a reason, or `Deferred` with the owner's approval.
- One row cannot be matched at all: dependency apps that ship only compiled code
  (DP-10). v3 has no CLR. It refuses the tests that reach such an app and names
  the app. The migration guide says: keep v2 for those projects.
- No telemetry (v2 has none either).

### 2. The user command line

A new binary crate (`crates/alrunner-cli`, non-core, may use the OS) provides the
user command line. `al-test-cli` stays the corpus dev tool and keeps its
flags. Shared code moves into a library crate (`al-testrun`, extracted from
`al-test-cli/src/runtime.rs` and the report types) that both binaries use. This
is a refactor, not a seam change: the core crates and their traits do not move.

```
alrunner test   [OPTIONS] [PATH...]     run tests; PATH defaults to .
alrunner run    [PATH] [--code TEXT | --file FILE] [--codeunit ID]
alrunner serve  [--protocol 2]          JSON lines on stdin/stdout (v2 --server)
alrunner debug  [--stdio | --port N] PATH       Debug Adapter Protocol (v2 --dap)
alrunner debug dump-ir PATH OUT                 replaces --dump-csharp
alrunner deps   fetch | list | resolve-version PREFIX | pack | clean | path
alrunner package PATH OUT.app           replaces --emit-app
alrunner helper install | status        the optional .NET helper
alrunner doctor                         checks caches, network, helper, versions
alrunner migrate -- <v2 arguments>      prints the v3 command line
alrunner guide                          manual for automated callers (v2 --guide)
alrunner --version | --help
```

Rules:

1. **Keep v2 spellings wherever nothing is gained by renaming.** Where ADR-013
   already chose a name (`--deps`, `--no-download`, `--cache-dir`), that name is
   primary and the v2 spelling is an accepted alias. In most cases a v2 command
   works after replacing `al-runner` with `alrunner test`.
2. Exit codes stay: 0 pass, 1 test failed, 2 could not execute or bad invocation,
   3 could not compile, 6 `--filter` matched nothing. Codes 4 and 5 belong to the
   dev-only count and expectations flags and stay in `al-test-cli`. One new code,
   7, means "some tests were not run" (see open question 3).
3. Refused features fail clearly. A v2 flag that v3 defers (`--affected`,
   `--tdd`) exits 2 with the reason and the tracking row, never silently.
4. Output files, run header, summary, `Seed:` line and JSON field names copy
   v2 (`docs/al-runner-v2-parity.md`, OUT rows), so CI parsers keep working.

#### v2 to v3 command map

| v2 | v3 |
|---|---|
| `al-runner DIR...` | `alrunner test DIR...` |
| `--test P`, `--filter P` | `--filter P` (`--test` accepted) |
| `--exclude-test P` | same |
| `--isolation M`, `--test-isolation M` | `--isolation M`; `method` means `test` |
| `--test-timeout S` | same |
| `--seed N` | same |
| `--jobs N`, `-j N` | same (threads, not processes) |
| `--bc-version V` | same |
| `--artifact-path DIR` | removed. Use `--bc-version` and `--deps` |
| `--country CC` | same |
| `--package-cache P` | `--deps P` (old name accepted) |
| `--auto-provision` | default; accepted, no effect |
| `--no-auto-provision` | `--no-download` (old name accepted) |
| `provision [DIR]` | `alrunner deps fetch [DIR]` |
| `provision --platform-apps`, `--test-apps`, `--force` | `deps fetch --platform-apps`, `--test-apps`, `--force` |
| `provision --service-tier` | removed (no engine DLLs) |
| `provision --resolve-version P` | `deps resolve-version P` |
| `provision --test-data` | `deps fetch --test-data` |
| `--test-data[=PATH]`, `--test-data-company`, `--test-data-normalize-company` | same on `test` |
| `--cache DIR` | `--cache-dir DIR` (old name accepted) |
| `--no-cache` | same: do not read or write the dependency cache |
| `--print-cache-key` | removed |
| `--watch` | same on `test` |
| `--affected`, `--include-failing`, `--strict-environment` | deferred; exit 2 with a message |
| `--server` | `alrunner serve` |
| `--dap [PORT]`, `--dap stdio` | `alrunner debug --port N`, `alrunner debug --stdio` |
| `--tdd` | deferred until its ADR (parity row SEL-10) |
| `--per-suite`, `--bundled` | `--bundled` accepted, no effect; `--per-suite` refused |
| `--define S`, `--preprocessor-symbols A,B` | same |
| `--output-junit P` | same (`--junit` accepted) |
| `--output-json` | same (`--json` accepted) |
| `--coverage`, `--coverage-out P` | same |
| `--show-pass`, `--failures-only`, `--quiet`, `--verbose` | same |
| `--strict` | accepted, no effect |
| `--no-strict-exit` | same |
| `--dump-csharp DIR` | `alrunner debug dump-ir PATH OUT` |
| `--precompile A --out B` | removed (no DLLs) |
| `--emit-app DIR OUT` | `alrunner package DIR OUT` |
| `--resume-aborts`, `--merge-counts`, `--merge-results` | removed (not needed) |
| `--out`, `--classify`, `--expectations*`, `--count-*` | dev-only; stay in `al-test-cli` |
| `--guide` | `alrunner guide` |
| `-v`, `-V`, `version` | `alrunner --version` (and `alrunner version`) |

Environment variables: prefix `ALRUNNER_`. The shared subset keeps the v2 name
as a fallback read (`AL_RUNNER_SEED`, `AL_RUNNER_TEST_TIMEOUT_SEC`,
`AL_RUNNER_VERBOSE`, `AL_RUNNER_SHOW_PASS`, `AL_RUNNER_FAILURES_ONLY`,
`AL_RUNNER_ARTIFACTS_ROOT`, `AL_RUNNER_CACHE_ROOT`) so existing CI files keep
working. v3 keeps its data under a `v3` subfolder of any root it reads from a v2
variable, so the two versions never write into each other's caches.

This replaces the draft names in the other ADRs (`AL_RUNTIME_CACHE` in ADR-013,
`ALRT_DOTNET_SIDECAR` in ADR-014). Those two ADRs should adopt `ALRUNNER_*`
when they are next edited.

#### Binary name

| Option | What follows |
|---|---|
| `alrunner` (recommended) | Different from v2's `al-runner`, so both can be on `PATH`. Short. A launcher named `al-runner` can still accept v2 syntax (section 6). |
| `al-runner` | Scripts keep working with no edit. But a v2 tool and a v3 binary fight over one name, and a v2 flag that v3 changes fails in confusing ways. |
| `al-test` or `alr` | Rejected. `al-test` reads like the dev tool `al-test-cli`. `alr` is also the command of the Alire package manager. |

### 3. Distribution

#### Build targets

| Target | Tier | Notes |
|---|---|---|
| `x86_64-unknown-linux-musl` | 1 | Static. The one Linux build for x64. Runs in Alpine, distroless and any glibc |
| `aarch64-unknown-linux-musl` | 1 | Static |
| `aarch64-apple-darwin` | 1 | Signed and notarized |
| `x86_64-apple-darwin` | 2 | Cross-built on the arm64 runner. Intel Macs are fewer each year |
| `x86_64-pc-windows-msvc` | 1 | Authenticode-signed |
| `aarch64-pc-windows-msvc` | 2 | Cross-built on x64. Signed from the x64 runner |
| `wasm32-wasip1`, `wasm32-unknown-unknown` | separate | Section 5 |

A glibc build (`x86_64-unknown-linux-gnu`) is added only if the musl build is
measurably slower. musl's allocator is slow, so the build links `mimalloc`.
Measure the corpus run on both before the preview and decide. The binary needs
no .NET, no SDK and no C compiler (v2 needed all three in some cases).

#### Release pipeline: plain GitHub Actions matrix (recommended)

Triggered by a tag `v3.*`. Stages:

1. **plan**: check the tag matches the crate version; resolve the commit.
2. **test**: `./check.sh`, then the corpus ratchet, then the perf budgets, on
   Linux. Same as `ci.yml`.
3. **build**: matrix over the targets above on native runners
   (`ubuntu-latest`, `ubuntu-24.04-arm`, `macos-14`, `windows-latest`), with
   `--locked` and stripped, LTO release profile.
4. **smoke test** each built binary: `--version`, then `alrunner test` on a
   small fixture project that lives in the repo (not Microsoft code).
5. **sign**:
   - Windows: Azure Artifact Signing with OIDC and the `release` environment, the
     same flow v2's `sign-and-pack` job uses (`publish.yml`). That action runs
     only on x64 Windows runners.
   - macOS: Developer ID signature, then notarization with `notarytool`. A bare
     binary cannot carry a stapled ticket, so Gatekeeper checks online on first
     run. Needs an Apple Developer Program account. Not verified here; confirm
     the flow on a test tag first.
   - Linux: none. Publish SHA-256 sums and a build provenance attestation
     (`actions/attest-build-provenance`) for every archive.
6. **package**: `.tar.xz` (Unix) and `.zip` (Windows) with the binary, licence and
   short README; one `SHA256SUMS`; install scripts `install.sh` and
   `install.ps1`.
7. **helper**: build the .NET helper (below) as a separate asset.
8. **release**: create a draft GitHub release. A person publishes it.
9. **channels**: after publish, jobs update Homebrew, winget, npm and the
   container image. Each is idempotent, so a failed channel can be rerun.

Why not `cargo-dist`: it would write most of stages 3, 6 and 9 for us. A web
search today shows cargo-dist 0.32.0 (May 2026) with shell, PowerShell, MSI and
Homebrew installers (secondary sources; I did not open its docs). I could not
confirm npm packaging or how it fits Azure Artifact Signing and Apple
notarization. We need both signing flows, the helper asset and the WASM
artifacts, so the custom steps would outweigh what it saves. v2 already runs a
plain pipeline with the same signing. If the owner prefers it, `cargo-dist` can
replace stages 3, 6 and 9 and the rest stays.

#### Install channels

| Channel | When | Notes |
|---|---|---|
| GitHub release archives + `install.sh` / `install.ps1` | preview | The base for everything else |
| Homebrew tap | preview | `brew install <tap>/alrunner`. Optional dependency note for the helper |
| winget | 3.0 | Manifest from the signed zip, via a release action |
| npm wrapper `@alrunner/cli` | 3.0 | Per-platform packages as optional dependencies (the pattern esbuild and biome use). `npx alrunner test` in CI without a download script |
| GitHub Action `setup-alrunner` | 3.0 | Installs the binary, restores the cache, sets `ALRUNNER_CACHE`. The CI convenience for AL-Go and plain workflows |
| Container image on ghcr.io | 3.0 | `FROM scratch` plus the musl binary |
| `dotnet tool` package | open question 5 | Lets `dotnet tool update -g MSDyn365BC.AL.Runner` move v2 users to v3. Depends on .NET's support for tools that carry a native executable. Not verified here |
| `cargo install`, `cargo binstall`, mise (`ubi:`), Scoop | free | Work from the release assets. Not promoted |

#### The .NET helper (ADR-014 calls it the sidecar)

- Separate release asset: `alrunner-dotnet-helper-<version>.zip`, a
  framework-dependent .NET 8 build. The managed part is one file set for all
  operating systems.
- On Linux the helper also needs the Win32 shim (`libwin32_stubs.so` for x64 and
  arm64), because Microsoft's server DLLs make Win32 calls. v2 ships it for the
  same reason. So the Linux helper asset is per architecture.
- The helper holds no Microsoft code. It loads Microsoft's DLLs from the cached
  artifact at run time (ADR-013, ADR-014).
- Lookup follows ADR-014 section 10: the flag or `ALRUNNER_DOTNET_SIDECAR`, then
  `dotnet-sidecar/AlDotNetSidecar.dll` next to the binary, then the cache
  folder. The release adds one step to the cache case: `alrunner helper install`
  downloads the prebuilt helper that matches the binary's version, checks the
  SHA-256 and writes it to the cache. Users then need no .NET SDK. Without the
  helper, a `DotNet` use is refused with the message ADR-014 section 11 defines,
  plus the command that fixes it.
- Helper and core share a protocol version. The `hello` message (ADR-014) rejects
  a mismatch. The helper is released in lockstep with the core: same version
  number, same tag.
- The machine needs the .NET 8 runtime for the helper only. Everything else runs
  without .NET.

### 4. Versioning

- **SemVer for `alrunner`**: `3.0.0-preview.N` while in preview, then `3.0.0`.
  - Major: a break in the command line, exit codes, JSON/JUnit shape, or the
    server protocol.
  - Minor: new features, new supported BC versions, new language surface.
  - Patch: fixes. A change that makes a result match BC is a patch.
- **Public contract** (what a major version protects): flags, exit codes, JSON
  and JUnit field names, server protocol number, helper protocol number. Passing
  or not passing a given AL test is not part of it: a "not run" test can become a
  pass in any release.
- **BC versions are data, not version numbers.** ADR-015 (Proposed) gives one
  codebase and one binary for BC 27 and every later major. A run builds one
  `Target` (BC version plus app runtime) from `--bc-version` or the `app.json`
  floor. Oracles and baselines are kept per major. The binary reports what it
  knows: `alrunner 3.1.0 (BC 27 and later; verified against 27.5, 28.5, 29.0)`.
  Release notes list the verified versions. A BC version with no oracle runs
  with a warning (ADR-015 section 3.8). Adding a verified BC major needs a new
  version number only if behaviour or the command line changes. v3 does not run
  for BC 26 and older, so the oldest supported floor is a fixed rule, not a
  moving range as in v2's `bc-versions.txt`.
- **Cache format** has its own integer in a `format` file. A mismatch rebuilds
  the cache. It never fails.
- Every JSON and JUnit report records the selected BC build (ADR-013), so a result
  can be tied to a build.

### 5. WebAssembly

#### Hosts

Core crates are already forbidden from OS access (ADR-006). Every host supplies
the same capabilities through `al-host` traits. The first three exist only on
paper because `al-host` is empty; ADR-013 defines the dependency capability and
the identity values.

| Host | How it runs | Microsoft app bytes come from | Not available |
|---|---|---|---|
| VS Code desktop extension | Spawn the native binary. WASM is not needed here | The extension downloads with Node, which has no CORS limit, into extension storage | none |
| VS Code for the web (vscode.dev, github.dev) | WASM in a web worker | Files the user put in the workspace, read with `workspace.fs`. The browser cannot fetch the CDN | downloads, `DotNet`, `--jobs`, `--test-data`, TCP debugger |
| Node (npm package) | WASM through `WebAssembly`, files through `fs` | The host script downloads (Node `fetch`, range requests) and hands bytes over | `DotNet` unless the host supplies a provider |
| wasmtime or any WASI host | `wasm32-wasip1` module with folders mapped in | The embedding program, or a mapped cache folder | network (WASI p1), threads, `DotNet`. Good for running untrusted AL tests in a sandbox |
| Browser playground | WASM in a worker | A folder or zip the user picks, or a pack file the user made with `alrunner deps pack` | everything in the web row |

Capabilities each host must provide through `al-host`:

1. **Dependency source** (ADR-013): list packages, read an entry and its CRC.
2. **Clock**: `Today`, `CurrentDateTime`, and a monotonic clock for test timeouts.
3. **Identity**: user, company, session values (defaults from ADR-013).
4. **Randomness**: a seed source for `Random`, `CreateGuid`.
5. **Output**: a sink for messages, logs and reports. No `stdout` in core.
6. **Cancel flag** checked by the VM between statements.
7. **Network and `DotNet` provider**: optional. Missing means the use is refused.

#### The browser CORS limit

Checked on 2026-10-13 with `curl` by the orchestrator: Microsoft's artifact CDN
(`bcartifacts-exdbf9fwegejdqak.b02.azurefd.net`) sends no CORS headers and
answers the preflight request with 403. A browser page cannot fetch from it.
It does support HTTP range requests (one country artifact is about 963 MB).
Options:

| Option | What follows |
|---|---|
| User supplies the files (folder pick, zip, or a `deps pack` file) | Works everywhere. We redistribute nothing. The user needs one native download first. **Recommended for the playground** |
| A trusted host downloads and passes bytes in (VS Code desktop, Node, native CLI) | No CORS limit outside the browser. **Recommended for the extension and Node** |
| Ask Microsoft to enable CORS on the CDN | Cheap to ask. We cannot plan on it |
| A browser extension with host permissions | Works. Heavy to build and to get approved. Later |
| A proxy that serves Microsoft's files | **Rejected for us to run.** It redistributes Microsoft's files, which ADR-013's answer ("download into a local cache, commit nothing") does not cover. A user may run their own pass-through, at their own risk. We do not ship or host one |

`alrunner deps pack PATH -o FILE` is new. It writes one file holding only the
entries a project needs (about 40 MB for the corpus, ADR-013). The user makes it
from their own download. Nobody else receives it from us.

#### Size, speed, limits

All WASM numbers below are estimates. The wasm target is not installed on this
machine and no wasm build exists. Measure before relying on them.

- **Size.** Native binary 5.2 MB. A WASM build is likely a similar size before
  compression. Check this with the first build.
- **Memory.** Native corpus run peaks at 124 MB. wasm32 allows up to 4 GiB, and
  browsers and Node often cap lower. Base Application (113 MB of AL, ADR-013) in
  memory is not measured. It is the main risk.
- **Speed.** Expect WASM to run slower than native, and single-threaded.
  `al-sema` uses `rayon` for parsing. Put it behind a default feature `parallel`
  that WASM builds turn off. Browser threads need cross-origin isolation headers
  and are not planned.
- **Stack.** The CLI runs the VM on a 512 MB stack. WASM stacks are fixed at link
  time and the engine limit is often about 1 MB. Deep AL recursion would fail.
  Add a configurable call-depth limit that raises BC's error, and test it at the
  WASM stack size.
- **Time.** `std::time::Instant::now` panics on `wasm32-unknown-unknown`. It is
  used only in one test today (`al-syntax/src/lexer.rs`). The VM must read time
  from the host.
- **Conformance.** Add the CI job ADR-006 already promises: build the core crates
  for `wasm32-unknown-unknown`, and run the corpus ratchet under Node.
- **Refused in WASM, by name:** `DotNet` (no helper process; ADR-014's seam lets a
  host supply one), real HTTP (the VM is synchronous, ADR-002, and browser
  `fetch` is not), file access, `--jobs`, `--test-data`, `--watch`, the TCP
  debugger. A stdio or message-based debugger can work in a worker later.

Deliverable order: CI wasm build first (cheap, protects ADR-006), then a Node
package and a WASI module as beta after the preview, then the playground. None
blocks v3.0.

### 6. Migration and coexistence

- **Side by side.** `alrunner` and `al-runner` are different commands and can be
  installed together. v3 uses different cache folders (a `v3` subfolder). v3 does
  not read v2's caches except as a read-only extra dependency folder: v2's
  `platform-apps/*.app` files contain the AL source (ADR-013 survey), so v3 can
  use them and save a download. The VS Code AL cache (`~/.bcartifacts.cache`) is
  also read-only.
- **Migration step.** `alrunner migrate -- <v2 arguments>` prints the equivalent v3
  command and says what changed. It never runs anything.
- **Compat launcher (optional, open question 2).** A binary named `al-runner`
  accepts v2 syntax, runs the mapped v3 command and prints one note to stderr.
  Installers offer it as an opt-in file. It helps scripts that are hard to edit.
- **Phases:**
  1. *Now to preview*: v2 is the supported tool. v3 is in this repository.
  2. *Preview* (`3.0.0-preview.N`): both are published. The README of each states
     which to use. v2 gets all fixes.
  3. *v3.0*: v3 becomes the default in docs. v2 gets security fixes and new-BC
     support for a stated window (open question 6), then is archived on a `v2`
     branch with its last NuGet version left in place.
- **Guide outline** (to write at the preview gate, in `docs/migration-v2-to-v3.md`):
  1. Who should move now, who should wait (compiled-only dependencies).
  2. Install v3; uninstall or keep v2.
  3. The command map (section 2 table), with five worked CI examples (GitHub
     Actions, Azure DevOps, GitLab, AL-Go custom step, local watch loop).
  4. What changes in output: failure text has no .NET exception prefix, JUnit suite
     names, "not run" tests and exit code 7.
  5. Dependencies and caches: where files go, how to go offline, how to reuse v2's
     downloads.
  6. Environment variables: old name, new name.
  7. Editors and tools: ALchemist, LethAL; what to update.
  8. What v3 does not do (DP-10, the deferred list) and where to report a gap.
  9. How to compare v2 and v3 results on your project (`--output-junit` from both).
  10. Rollback.

### 7. Readiness gates

#### Gate P: a v3 preview (`3.0.0-preview.1`)

All of:

1. `alrunner test PATH...` runs a real project (app + test app) from a folder.
2. Dependencies work end to end (ADR-013): selection, download, cache, System
   Application and Base Application compiled on demand, `--deps`, `--no-download`.
3. Reports: console, exit codes 0/1/2/3/6/7, `--output-junit`, `--output-json`,
   `--filter`, `--exclude-test`, `--seed`, `--isolation` (all three modes).
4. **0 wrong answers.** Corpus fail count is 0 and every not-run test carries a
   reason. This is the existing rule.
5. **No regression against v2 on the corpus**: every corpus test that v2 passes
   either passes in v3 or is listed with a reason (by design out of scope, or a
   named blocker). The list is generated, not hand-written.
6. Real-project runs: at least 3 independent projects (for example one of
   Stefan's own, one ISV-style app with its own test app, one from Microsoft's
   public BCApps) run to the end. Each is run in v2 and v3. Every difference in
   test outcome is explained.
7. Signed and notarized binaries for Linux x64/arm64, macOS arm64, Windows x64;
   `install.sh`, `install.ps1`, Homebrew.
8. Docs: README, getting started, migration guide draft, `alrunner guide`.
9. Budgets hold: corpus cold compile at most 1 s; a hello-world project with
   warm cache starts in at most 1 s (new budget, set at the first measurement).

#### Gate R: v3.0

All of gate P, plus:

1. **Parity page closed**: every row is `Done`, `N/A` with a reason, or `Deferred`
   with the owner's written approval. Count at the time of writing: 3 Done and 99
   open (headline of the parity page).
2. Corpus: the v2-regression list is empty or every entry has an owner-approved
   reason. The oracle version matches the newest BC minor v3 claims to support
   (ADR-015).
3. Real-project runs: at least 10 projects, including a subset of Microsoft's
   BaseApp test buckets that v2 runs today. No unexplained difference.
4. `serve` speaks protocol-v2; ALchemist and LethAL run against it unchanged
   (their owners confirm).
5. `--coverage` produces Cobertura; `debug` passes the same DAP script that v2's
   tests use.
6. The .NET helper ships and the `DotNet` cases chosen in ADR-014 pass.
7. winget, npm, Action and container image published.
8. At least four weeks of preview with no open bug that gives a wrong answer.
9. The WASM CI job is green. Shipping WASM artifacts is not a v3.0 gate.

## Owner direction (2026-10-13): server mode and watch mode are deferred

AL Runner v2 has `--server` and `--watch` to hide its startup cost. The owner
asked to defer features like these while startup is not a problem for v3.

- Today the whole corpus compiles and runs in about 0.7 s cold, so start-up is
  not a concern for workspace-only projects.
- Standing rule (owner, 2026-10-13): every v2 feature that exists only to hide
  start-up or run-time cost is deferred under the same rule. As of today that
  covers:
  - `--watch` and its dashboard (PERF-4, PERF-5);
  - `--affected` (PERF-6);
  - server-mode warm state (ED-5);
  - `--jobs` worker parallelism (SEL-8);
  - persistent dependency caches (PERF-2; ADR-013 already makes them
    optional);
  - their environment variables (`AL_RUNNER_WATCH_*`, `AL_RUNNER_JOBS_*`).

  The parity table counts these as "deferred", not as missing. Features that
  are an interface (a protocol, an output format, a flag that changes results)
  are not covered by this rule.
- Revisit when either of these happens:
  1. ADR-013 dependency loading is measured, and a warm start with Base
     Application is clearly slower than a test run;
  2. an editor integration is wanted. ALchemist and LethAL drive v2 through
     `--server` (protocol-v2) and `--dap stdio` (ED-1, ED-10, ED-11). For them
     the server is the interface, not a speed feature. Supporting them needs
     the protocol even when start-up is fast.

  The interface points of both tools, what v3 lacks and a stepwise plan are in
  `docs/integrations/alchemist-lethal.md`. Gate R item 4 depends on it.

## Owner decision (2026-10-14): separate now, decide the cutover later

- The runtime ships under a new binary name, and the work stays in the
  al-runtime-rs repository.
- The CLI is designed cleanly. It does not mirror v2's flags.
- The goal is unchanged: full feature parity with AL Runner v2.
- At the preview gate the owner decides, with parity numbers and real-project
  runs in hand, between two paths:
  - stay a separate product;
  - cut over the BusinessCentral.AL.Runner repository. v2 then moves to a
    branch, and a thin launcher that accepts v2 syntax is added.

This answers open questions 1 (binary name: a new one, still to be picked),
2 (no compatibility launcher for now) and 4 (the repository is al-runtime-rs
for now).

## Decided without owner review (2026-10-14): the first `alrunner test`

Built on branch `alrunner-cli` as the owner asked: a minimal user command line with Microsoft's BaseApp
test buckets as the first real target. Each item is easy to change.

- **Binary name `alrunner`**, crate `alrunner-cli` (open question 1: the recommended name; the owner's
  2026-10-14 decision already said "a new one, still to be picked").
- **Shared code is the crate `al-testrun`**, extracted from `al-test-cli`: the test runner (`runtime.rs`),
  install baseline, dependency loading, .NET sidecar host, `--explain` text, target choice, and now test-codeunit
  discovery and refusal ranking. `al-test-cli` re-exports the modules, so its tests and flags are unchanged. This
  is the refactor section 2 describes. No seam changed.
- **Exit codes**: 0 all selected tests passed, 1 a test failed, 2 bad invocation or the run could not start, 6 the
  selection matched no test, 7 some tests were not run. `--allow-not-run` makes 7 into 0. When several apply the
  order is 2, 6, 1, 7 (a failure outranks a refusal). Code 3 (could not compile) is not used: the compiler refuses
  per routine, never for a whole run. This takes the first option of open question 3.
- **Flags** (a clean set, not v2's): `--dep PATH` (a package file, an app folder or a folder of packages),
  `--package-cache DIR` (a folder of `.app` files; v2's spelling), `--bc-version`, `--no-download`, `--cache-dir`,
  `--codeunit ID`, `--filter TEXT` (`--test` accepted), `--junit FILE`, `--top-refusals [N]`, `--allow-not-run`,
  `--verbose`, `--quiet`, `--timings`, `--files-dir`, the `--dotnet-*` flags of `al-test-cli`. `--test-data` is refused
  with exit 2 until ADR-020 is accepted, and the flags deferred in this ADR (`--watch`, `--affected`, `--tdd`) are
  refused by name. Rule 1 (keep v2 spellings) applies to `--package-cache`, `--filter`/`--test` and `--junit`.
- **Isolation** is `TestIsolation = Codeunit`, which is the default of Microsoft's test runner
  (`ALTestRunner.psm1`, `$TestIsolation = "Codeunit"`) and what every corpus oracle is recorded under. There is no
  `--isolation` flag yet (parity row SEL-4/SEL-5).
- **Dependencies** come from the closure of the APPs' `app.json`: `--dep`, then `--package-cache`, then each folder
  APP's `.alpackages`, then Microsoft's artifact feed (ADR-013 Layer A). A source archive that holds Microsoft's
  `$(app_...)` variables is read with the build named by a record beside it, else by a four-number build among the
  folders it lies in (the artifact cache layout), else by an exact `--bc-version`. A prefix gives no build and
  the archive is refused with a message saying so.
- **`application` and `platform` floors are dependencies.** Microsoft's buckets declare neither Base Application
  nor `Application`; AL gives an app with an `application` floor a dependency on the `Application` aggregate, which
  propagates Base Application, System Application and Business Foundation. `alrunner` declares both floors as
  Microsoft dependencies (so the feed can supply them) and adds the implied ids to the visibility of each app
  (`Request::application_implies_dependencies`, off for the corpus job).
- **Not-run tests are the measurement.** With no test data (ADR-020 is Proposed) Microsoft's tests fail on missing
  setup data, so the summary line says failures are not trusted. The ranking of refusal reasons is the output to
  act on.
- **Measured on Microsoft's buckets**: `docs/alrunner-buckets.md` has the recipe and the numbers for all 32 buckets.
- **A run never ends in a panic of the lowering.** A binary expression the parser left with one operand is refused
  in its routine (a page field source with a hyphen in its name reached the synthetic source routine unquoted, which
  panicked and ended the whole Tests-ERM run). The cause, in `al-sema/src/pages.rs` (`Section::args` drops the quotes
  of a lone quoted name), is not fixed here.

## Consequences

- A new crate set (`alrunner-cli`, `al-testrun`, `al-host-std`, `al-deps`) appears.
  `al-test-cli` keeps the corpus job and loses only code that moves.
- ADR-013 and ADR-014 should adopt the `ALRUNNER_*` variable names.
- ADR-015 and this ADR agree on how the verified versions are reported (section 4). ADR-015 3.9 says non-corpus projects get preprocessor symbols from `app.json` only, which this ADR follows.
- The parity page becomes a living document. Each merged feature updates its row.
- Several parity rows need their own ADR before code: server protocol, debugger,
  platform table providers, test data, `--tdd`, `--affected`.
- Cost of signing: an Apple Developer Program membership, and the existing Azure
  Artifact Signing setup reused.
- v3 refuses DP-10 apps. Some v2 users cannot move.

## Conformance

- Release workflow smoke test: every built binary runs `alrunner --version` and a
  fixture project.
- A test compares `alrunner --help` flags with the command map in this ADR.
- A test runs `alrunner migrate` over each v2 flag in the map.
- CI job: core crates build for `wasm32-unknown-unknown` (ADR-006).
- The parity page row counts are checked by a small script in CI so the page cannot
  drift from the tables.
- Gate checks are scripts, not people: the v2-regression list, the real-project
  differential and the budget tests run from one command.

## Open questions for the owner

Each is one decision. My recommendation is first.

1. **What is the binary called?**
   - `alrunner` (recommended): both versions can be on `PATH`.
   - `al-runner`: v2 scripts keep working, but v2 and v3 fight over the name.
   - Something else of your choice.
2. **Do we ship a launcher named `al-runner` that accepts v2 syntax?**
   - Opt-in file in the installers (recommended): easy migration, small code.
   - Only `alrunner migrate`: less code, scripts must be edited.
   - Neither.
3. **What exit code do tests that were not run (refused) give?**
   - 7, non-zero by default; `--allow-not-run` makes it 0 (recommended): CI cannot
     go green while tests were skipped.
   - 0 by default, a warning, and `--strict-not-run` for 7: friendlier, but a
     project can look green while most tests did not run.
   - Count them as failures (exit 1): matches v2 for out-of-scope surfaces, hides
     the difference between wrong and missing.
4. **Where does v3 live when it is released?**
   - Move v3 into the `BusinessCentral.AL.Runner` repository as `main`, v2 on a
     `v2` branch (recommended): one project, one issue list, stars and links keep
     working.
   - New repository `alrunner`; v2 stays where it is: clean history, but two issue
     lists.
   - Keep `al-runtime-rs` and rename it later.
5. **Do v2 users get v3 through `dotnet tool update`?**
   - Publish v3 under the same NuGet id if .NET supports a native executable in a
     tool package (needs a check; recommended if it works): no action for users.
   - Keep NuGet frozen at v2 and point to the new installers: simple, but v2 users
     are not nudged.
6. **How long does v2 get fixes after v3.0?**
   - 6 months for security and new-BC-version fixes (recommended).
   - 12 months.
   - Until a stated v3.x release.
7. **Which v2 features may be deferred past v3.0?** Candidates: `--affected`
   (PERF-6), `--tdd` (SEL-10), `--test-data` (DP-20), `package` (IN-10), a user
   "known gap" file (DEV-1).
   - Defer `--affected` only (recommended): at v3 speed a full run is cheap.
   - Defer `--affected` and `--tdd`.
   - Defer none: strict parity at 3.0.
8. **A dependency app that ships only compiled code (DP-10): what do we do?**
   - Refuse the tests that reach it and name the app (recommended).
   - Also compile against its symbols and refuse at the first call: more tests run
     up to the call, but this is partial behaviour that hard rule 4 discourages.
   - Treat as "keep v2" and say so in the migration guide only.
9. **How much WASM ships with the first stable release?**
   - Only the CI build, then a Node package and WASI module as beta later
     (recommended).
   - Node, WASI and a browser playground (no Microsoft apps) with 3.0.
   - Nothing until after 3.0.
10. **Which extra targets do we build?**
    - Windows arm64 and macOS Intel as tier 2 (recommended).
    - Only the four tier 1 targets.
    - Add a glibc Linux build too.
11. **Does the compat env-var fallback (`AL_RUNNER_*` read by v3) stay?**
    - Yes for the 7 shared names, with a `v3` data subfolder (recommended).
    - No: only `ALRUNNER_*`; CI files must be edited.

## Owner decision (2026-10-15): alpha

The owner asked for a first public build before gate P. The gates in section 7 stay as written.

- **License**: MIT, like AL Runner v2. Copyright holder: Stefan Maron. `LICENSE` is at the repo root and the
  workspace sets `license = "MIT"`. *Changed 2026-10-16, see "License change" below.*
- **First release `3.0.0-alpha.1`**, below the preview gate on purpose: no v2 parity, refused tests are
  expected, results may change. The crates share one version (`[workspace.package]`).
  `alrunner --version` prints the version and the short commit, for example `alrunner 3.0.0-alpha.1 (d73d2cacb)`.
- **Unsigned binaries for three targets**: `x86_64-unknown-linux-musl`, `x86_64-pc-windows-msvc`,
  `aarch64-apple-darwin`. Archives are `.tar.xz` (Linux, macOS) and `.zip` (Windows) with the binary, `LICENSE` and
  a short README. One `SHA256SUMS`. `install.sh` and `install.ps1`. A build provenance attestation
  (`actions/attest-build-provenance`) for every archive. The Gatekeeper and SmartScreen warnings and their
  workarounds are in the README ("Try the alpha"). Signing (section 3 stage 5) waits for the preview.
  The aarch64 Linux, Intel macOS and Windows arm64 targets of section 3 are not built for the alpha.
- **Linux allocator**: the musl build links `mimalloc` (the `mimalloc` feature of `alrunner-cli`), as section 3
  recommends. The musl against glibc measurement of section 3 is **not done**: the author's machine has no musl
  target. Decide with the first numbers from a dry run, and add a gnu build only if musl is clearly slower.
- **The .NET helper**: unchanged. The binary embeds the helper sources and builds them into the cache on first
  `DotNet` use, which needs a .NET 8 SDK. There is no helper asset in the alpha.
- **Two kinds of build, one pipeline.** Both use the reusable workflow `build-packages.yml` (build on native
  runners, smoke test, package, attest, checksums). Smoke test: `release/smoke.sh` runs `--version`, then
  `alrunner test` on the fixture in `release/smoke/` (an app and a test app, no Microsoft code, no download).
  - **`edge`**: one rolling pre-release with the tag `edge`. It holds the newest build of the `release` branch.
    When CI is green on a push to `release`, `edge.yml` rebuilds the three targets, moves the tag to that commit,
    replaces the assets and rewrites the body (commit, date, compare link). It is a pre-release and never
    "latest". A newer push cancels an older build. No new tag or release per push.
  - **Numbered alphas** (`3.0.0-alpha.N`): a tag `v3.*` on a commit of `release`. `release-tag.yml` (named `release.yml`
    before 2026-10-20) refuses a tag that is not the crate version or whose commit is not on `release`, runs the CI
    checks (`ci.yml`, reused), builds, and creates a **draft** pre-release. A person publishes it. A manual run is
    a dry run and creates no release. Changed by the owner on 2026-10-20: see below.
- **Branches**: development stays on `main`. `release` is fast-forwarded from `main` when the owner wants a new
  edge build (`git push origin main:release`). Nobody commits on it, except a hotfix that cannot come from
  `main`: it is committed on `release`, released, and merged back into `main`. A fast-forward that fails
  afterwards means the merge-back is missing. Steps: `docs/releasing.md`. Open for the owner: branch protection
  for `release` (no force push) and protected tags `v3.*` and `edge`. Repository settings are not changed here.
- **Docs and feedback**: README "Try the alpha" and the issue template `.github/ISSUE_TEMPLATE/alpha-feedback.md`.

### Owner decision (2026-10-20): numbered alphas are normal releases

"Build it, start with 3.0.0-alpha.2 as normal releases, update the readme with the tool recommendation."
`release-tag.yml` now publishes each numbered alpha as a normal GitHub release (not a draft, not a
pre-release), marked "latest". Reason: mise's github backend skips pre-releases, so `mise up` found nothing.
The workflow also checks that `alrunner --version` prints the tag's version. `edge` stays a pre-release.
The README recommends mise (`mise use -g github:StefanMaron/al-runtime-rs`).

Also 2026-10-20: edge becomes a nightly build. `nightly-edge.yml` runs at 02:17 UTC, fast-forwards `release`
to the newest main commit with green CI on a push (never a force push), and calls `edge.yml` with that commit.
It pushes with GITHUB_TOKEN, so CI does not run twice on the same commit. A person's push to `release` works
as before. Named alphas stay manual.

### Owner decision (2026-10-21): the binary is `al-runner`

- The binary `alrunner` becomes `al-runner`: the Cargo bin name, the `--version` line
  (`al-runner 3.x (commit)`), the release archives (`al-runner-<label>-<target>`), the installers and
  the help text. Earlier text in this ADR and in other ADRs keeps the old name as written.
- Kept: the crate name `alrunner-cli`, the `ALRUNNER_*` environment variables (owner undecided), the
  cache folders, the Windows install folder `%LOCALAPPDATA%\alrunner\bin` (so a `PATH` entry made for an
  earlier alpha keeps working), and file names such as `028-alrunner-serve.md`.
- The installers still install a release from before the rename under its old name, and point out a
  leftover `alrunner` in the install folder.
- **The v2 `PATH` clash** (decided without owner review, 2026-10-21; tooling only, so ADR-009 allows it).
  AL Runner v2's dotnet tool is also called `al-runner`. v3 can only notice the clash when it runs:
  - Each command except `serve`, `--help` and `--version` looks for other files named `al-runner` or
    `al-runner.exe` on `PATH` and in `~/.dotnet/tools`. It skips itself (canonical path) and links to a
    program of another name (a version manager's shim). If it finds one, it prints one line to stderr,
    at most once a day (stamp file `path-note.stamp` in the cache folder). Never on stdout.
  - `al-runner --version` prints its line, then `also on PATH: <path>: <first line of its --version>` for
    each other copy, waiting at most 2 seconds for each.
  - `ALRUNNER_NO_PATH_NOTE=1` turns both off (the `--version` list sets it for the copies it runs).
  - `install.sh` and `install.ps1` print the same advice when v2 is in `~/.dotnet/tools`, and say which
    `PATH` order to use when v2's folder comes first.

## Rejected options

- **Keep `al-test-cli` as the user tool.** It is the oracle harness; its flags and
  exit codes serve the ratchet. Mixing the two would tie user compatibility to
  corpus tooling.
- **Run v2 inside v3 for hard cases.** Defeats the point of a self-contained
  executable and drags in .NET, the SDK and gigabytes of DLLs.
- **A hosted proxy for Microsoft's artifacts.** Redistribution (section 5).
- **Bundle Microsoft apps in the release.** ADR-013 forbids it.

## Owner decision (2026-10-15): v3 replaces v2; server mode and debugging are in scope

The deferral of server mode and watch mode (2026-10-13) is lifted: v3 is to replace v2 in
every way. For the tools that call v2 (ALchemist, LethAL; see docs/integrations/alchemist-lethal.md):

- **As close to v2 as possible, but no dead weight.** v3 keeps its own clean command line
  (no `al-runner` launcher). Where a v2 interface shape is a poor decision, v3 changes it and
  documents the change.
- **Tools adapt when they move to v3.** A migration document for tool authors,
  `docs/manual/tool-protocol.md`, lists every difference (command, flags,
  `--version` text, JSON fields, protocol messages, exit codes) with the exact change a tool
  needs. It is written so an AI-assisted maintainer can apply it directly, and it grows with
  each step below.
- **Order:** (1) v2-shaped `--output-json` and the flags LethAL uses on `alrunner test`;
  (2) Cobertura coverage; (3) `alrunner serve` with runTests and shutdown; (4) execute and
  cancel; (5) value capture and iteration tracking; (6) DAP debugging, with its own ADR.
  All six are in v3.0.

## License change (owner decision 2026-10-16)

From this change on, al-runtime-rs is licensed under the Business Source License 1.1 (`LICENSE`, SPDX
`BUSL-1.1`). Licensor: Stefan Maron. Additional Use Grant: production use is free, including commercial
work (developing, testing and building Business Central extensions for oneself or for customers, locally
and in build pipelines); selling, sublicensing or offering the work, a modified version, or a product or
service in which it is a substantial part, to third parties for a fee is not granted. Change Date: four
years after each version is first published. Change License: Apache License 2.0. Versions published
before the change (up to and including 3.0.0-alpha.1 and the edge builds before the change) remain under
MIT. Outside contributions need a contributor agreement before they are accepted, so the licensing stays
with one holder.

## Amendment (Proposed, 2026-10-21): repository move and binary name

Status of this section: Proposed. It records owner decisions of 2026-10-21. It
answers the "stay separate or cut over" choice of the 2026-10-14 decision above.

- **Cut over.** v3 is now `main` of `aldevtools/BusinessCentral.AL.Runner`.
  v2 is frozen on the branch `v2` of the same repository.
- **Organization.** The corpus is `aldevtools/BusinessCentral.AL.Language.Tests`.
  The BC container is `aldevtools/MsDyn365Bc.On.Linux`.
- **Working repository.** `StefanMaron/al-runtime-rs` stays the working
  repository until the running lanes finish. Its `main` is copied to the new
  repository. After that it is archived.
- **References.** The corpus submodule URL, the `repository` field in
  `Cargo.toml`, the installers' default repository (`ALRUNNER_REPO`), the
  mise name (`github:aldevtools/BusinessCentral.AL.Runner`) and the tools that
  call `gh` point to the new names. GitHub Actions `uses:` lines do not follow
  a transfer redirect, so every `uses:` that names a moved repository must name
  the new one.
- **Old numbers.** Pull request and issue numbers in history text written
  before the move (below about 5570) refer to the archived
  `StefanMaron/al-runtime-rs`. New text names the repository next to the number.
- **Binary name.** The binary becomes `al-runner` (a separate change). The
  environment variables keep the `ALRUNNER_` prefix for now; the owner has not
  decided about them.
