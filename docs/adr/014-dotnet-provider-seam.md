# ADR-014: DotNet provider seam

- Status: Accepted (by the project owner, 2026-10-13)
- Date: 2026-10-13

## Context

Microsoft's apps declare variables of type `DotNet`. Each one maps an AL alias to
a .NET class. A `DotNet` call in compiled AL is a late-bound reflection call by
name (`NavDotNet`). The runtime is Rust and has no .NET.

ADR-013 decision 8 refuses every `DotNet` use until the owner decides. The
research is in `docs/dotnet-examples.md` (five real cases) and
`docs/dotnet-generic.md` (options, BC's resolution rules, what the server
classes need). Section 3 of the second page compares four ways to serve the
calls. This ADR records the owner's choice and specifies the seam.

The corpus app is `"target": "Cloud"` (`corpus/tests/al-language/app.json:63`),
so corpus code cannot declare `DotNet` itself (AL0296). Corpus tests reach
`DotNet` only through Microsoft's apps. A community on-premises app can use it
directly.

### Decided by the project owner (2026-10-13)

1. `DotNet` calls are served by a separate .NET helper process (the **sidecar**):
   C# on .NET 8, a small RPC protocol, reflection-based dispatch. It loads real
   assemblies: the .NET standard library, third-party and community DLLs, and
   also Microsoft's BC server DLLs (`Microsoft.Dynamics.Nav.*`) from the cached
   artifact, the way AL Runner does. Loading Microsoft's DLLs was explicitly
   approved.
2. BC server classes that cannot work outside a running service tier
   (`NavTenantSettingsHelper`, `NavUserAccountHelper`, `ALConfigSettings`,
   `NavDesignerALFunctions`, and others that read NST session or tenant state)
   are provided by the runtime itself as **service internals**, through a
   registry in front of the sidecar. Each member needs a corpus pin or a probe.

### Decided by the owner (2026-10-14)

3. The BC server classes that Microsoft's AL reaches through `DotNet` are provided in Rust through
   the registry of section 1: `ALConfigSettings`, `NavTenantSettingsHelper`, `NavUserAccountHelper`
   and `NavTestExecution` so far (`al-vm/src/dotnet/server_classes.rs`). This is not a fake of an
   AL object, so hard rule 7 does not apply.
4. Until a probe on real BC pins a value, it mirrors AL Runner v2, which runs BC's real classes on a
   faked session (`MetadataPatches.cs` builds that session). Each registry member records its
   source: `Pin::AlRunner("AL Runner v2 <file:line>")` now, `Pin::Probe` once a probe pins it,
   `Pin::Corpus` where a corpus test does. Where a probe and AL Runner differ, the probe wins.
5. The probe batch (corpus tests under `tests/al-language/session/`, measured on BC 27.5 and later) pinned the
   ALConfigSettings flags and limits, `IsSandbox` true, `IsProduction` false, `IsAzure` true on
   27 and 28, false from 29 (through `IsSaaSInfrastructure`, `Gate::NotSaaSInfrastructure`), `GetApplicationFamily` `W1`, the empty Power Platform
   environment id, and `IsInTestMode` (through `CanStartSession`). AL Runner's tenant is
   Production on an on-premises topology, so the oracle tenant, a sandbox on Azure topology,
   differs from it in `IsSandbox`, `IsProduction` and `IsAzure`. The shipped values follow the
   probe. The probe files reach the corpus submodule with its next bump; the pins cite them by
   path and head.
6. A registry member declares its AL result type in `al-builtins/src/dotnet_sigs.rs`. The
   compiler reads it, so `if Helper.IsAzure() then` and `Settings.ApiSubscriptionDelayTime() > 0`
   lower. A member of a registered type that has no row, and every member of a type the host
   serves, stays untyped and keeps the static refusal "a DotNet value used as an operand": BC types
   those from the assembly, which lowering never opens. A test checks that the registry and the
   table name the same members.

## Decision

### 1. Layers and call order

```
AL code                 DotNet variable, member call
  |
al-vm  DotNetRouter     1. registry of runtime-provided types   (al-vm, pure)
  |                     2. host provider                        (trait in al-host)
  |                     3. refuse, naming the type and member
al-host  DotNetHost     trait only; default implementation refuses everything
  |
al-test-cli  (OS)       SidecarHost: spawns the sidecar, speaks the protocol
  |  stdin/stdout
tools/dotnet-sidecar    C# on .NET 8: load, resolve, reflect, invoke
```

Rules:

- A type found in the registry is served by the registry for **all** its
  members. A member the registry does not implement is refused as
  `Type.Member`. It is never forwarded to the sidecar. Forwarding would run
  Microsoft's real code against no session and give a wrong answer or a
  `NullReferenceException` (measured, section 12).
- A type the registry does not know goes to the host provider.
- With no provider, or a provider that is unavailable, the call is refused.
- The registry may also hold stand-ins for hot standard-library classes later
  (`docs/dotnet-generic.md` section 3d). A stand-in is added only when it is
  measured faster or must work without .NET, and it must pass the same tests as
  the sidecar path. The sidecar path is the oracle for the stand-in.

### 2. The seam: capability traits in `al-host`

Core crates do no OS access (ADR-006). Spawning a process is OS access. So the
core sees only a trait. Only `al-test-cli` (and later front-ends) implement it
with a real process. This is a sketch of the shape. The exact signatures belong
to the first implementation and stay additive.

```rust
// al-host
pub trait DotNetHost {
    /// Cheap and cached. Err means every DotNet call in this run is refused.
    fn availability(&self) -> Result<(), DotNetUnavailable>;
    /// One DotNetSession per VM Session. Starts nothing until the first call.
    fn open_session(&self, cfg: &DotNetSessionConfig) -> Box<dyn DotNetSession>;
}

pub trait DotNetSession {
    fn construct(&mut self, ty: &DotNetType, args: Vec<DotNetArg>) -> Result<DotNetReply, DotNetFailure>;
    fn invoke(&mut self, target: DotNetTarget, member: &str, args: Vec<DotNetArg>) -> Result<DotNetReply, DotNetFailure>;
    fn get_property(&mut self, target: DotNetTarget, name: &str) -> Result<DotNetReply, DotNetFailure>;
    fn set_property(&mut self, target: DotNetTarget, name: &str, v: DotNetValue) -> Result<DotNetReply, DotNetFailure>;
    fn release(&mut self, handles: &[DotNetHandle]);
    fn close(self: Box<Self>); // ends the sidecar for this Session
}
```

- `DotNetType` is what the `dotnet { }` block declares: assembly name, culture,
  public key token, full CLR type name. Generic names keep the CLR spelling
  (``List`1[System.String]``).
- `DotNetTarget` is `Static(DotNetType)` or `Instance(DotNetHandle)`.
- `DotNetHandle` is a `u64`, unique per Session and never reused.
- `DotNetArg` is a `DotNetValue` plus a by-reference flag (AL `var`).
- `DotNetReply` holds the result value, the new values of by-reference
  arguments, and stream effects (section 4).
- `DotNetFailure` is either a refusal (with a reason) or a .NET exception
  (CLR type name, message, inner exceptions).
- `DotNetHost` is held by the Engine configuration, like the other host
  capabilities. The default (`NoDotNet`) reports
  `Unavailable("no .NET host configured")`. The deterministic test host in
  `al-host` uses it, so core tests and a future WASM build never need .NET.
- `al-builtins` gets one additive `Value` variant (ADR-010 allows this):
  `Value::DotNet(Rc<DotNetObject>)`. `DotNetObject` is either `Host(handle)` or
  `Internal(type, state)` for registry objects. `:=` shares the reference. This
  is the assumed BC rule and needs a probe (section 13).
- When the last `Rc` drops, the handle goes onto a release queue owned by the
  Session. The queue is flushed before the next call and when the Session
  closes. `Drop` never calls the host directly.

### 3. How a `DotNet` alias resolves

BC's rule (decompiled, `docs/dotnet-generic.md` section 1.2): the `dotnet { }`
block names an assembly and a type. At run time `NavAutomationHelper` calls
`Assembly.Load(fullName)`. If the framework does not supply it, the
`AssemblyResolve` handler tries, in order:

| BC step | Mirrored? | Our rule |
|---|---|---|
| Default load context (the .NET shared framework, so the standard library) | yes | The sidecar's own default context. The standard library comes from the installed .NET 8 runtime, not from the artifact. |
| 1. Add-ins folder (`<service>/Add-Ins`, recursive, `*.dll` then `*.exe`; also a temp copy) | partly | The `Add-ins` folders of the selected platform package, recursive. Not the temp copy. |
| 2. The service folder, top level | yes | The service-tier folder of the selected platform package (Newtonsoft, OpenXml, `Microsoft.Dynamics.Nav.*`). |
| 3. Registered add-in resolver | no | Replaced by `--dotnet-assembly-dir DIR` (repeatable), searched after step 2. For community DLLs on disk. |
| 4. Registered app add-in resolver | no | Not needed. |
| Fallback A: the Add-in table in the database (blob extracted to temp, retry) | later | Phase 3. The platform `Add-in` table is a platform table (ADR-013 decision 7) and needs a provider first. Until then this step is skipped and the miss is refused with that reason. |
| Fallback B: assemblies packaged inside the installed extension (`addin/` in the `.app`) | later | Phase 3. `al-deps` hands the DLL bytes of apps in the workspace to the sidecar (`load_bytes`, loaded from memory). Until then refused. |

- Assembly match is by simple name. The version rule follows AL Runner's guard
  in `AlRunner/DependencyLoader.cs` and is pinned by a test when implemented.
- The service-tier folder comes from the platform package that ADR-013 selects
  for the run, stored in the same cache outside the repo. Nothing from
  Microsoft is committed (hard rule 7).
- **Compile time.** Compilation never starts the sidecar. AL0296 (Cloud target
  uses `DotNet`) is checked from `app.json`, for apps we compile that are not
  Microsoft's. AL0185 (type missing) is **not** reproduced statically. BC would
  reject such code only when the assembly is missing. We accept the code and
  refuse at first use with the assembly name (ADR-009: never reject code BC
  accepts). Checking assembly metadata in pure Rust remains an option for later.
- **First use.** The first call on an alias resolves its type in the Session's
  sidecar and caches the result for the Session.

Where AL Runner does the same (checkout
`/home/stefan/Documents/Repos/community/BusinessCentral.AL.Runner`, commit
`351c2813d`). It reaches the same results by running BC's own `NavDotNet`
in-process.

| Topic | AL Runner file |
|---|---|
| Compile-time probing order for DotNet assemblies | `AlRunner/BcCompiler.cs` `GetOrCreateDotNetFactory` (line 600), `BuildDotNetProbingPaths` (654) |
| Add-ins and Test Assemblies folders of the artifact | `AlRunner/BcCompiler.cs` `EnumerateServiceTierAddinDirs` (847) |
| Runtime assembly resolution from the artifact, with a version guard | `AlRunner/DependencyLoader.cs`, `docs/assembly-resolution.md` |
| .NET reference packs needed because `netstandard.dll` is a facade | `AlRunner/BcCompiler.DotNetRefPacks.cs` |
| Absent add-in assembly becomes a named out-of-scope refusal | `AlRunner/Patches/NavDotNetPatches.cs` (line 12) |
| One member answered from the Access Control rows | `AlRunner/Patches/NavDotNetPatches.cs` `InvokeReflectedMember` (line 51) |
| Platform-refused types by name (`System.Drawing`), stand-in for `SecurityIdentifier` | `AlRunner/Patches/DotNetInteropShims.cs`, `docs/scope.md` section 3.16 |
| Fake `NavSession`, tenant and `TenantSettings` so `NavTenantSettingsHelper` works | `AlRunner/Patches/MetadataPatches.cs` (around line 780) |
| Config data missing in headless mode | `AlRunner/BcRuntime.cs` (line 2167) |
| Static .NET state is not isolated, only recorded | `AlRunner/Infrastructure/AlSessionStateTracker.cs` (line 169) |
| Scope statement for `DotNet` | `docs/scope.md` section 3.14, `docs/limitations.md` (line 447) |

AL Runner keeps a hand-built fake of the service tier session to make four
server classes work. Decision 2 of the owner replaces that fake with a registry
that reads our own configuration. This is the part we do differently on purpose.

### 4. Value marshalling

Follow BC's `NavDotNet` conversion table (`docs/dotnet-generic.md` section 1.2,
item 6). The wire carries a tagged value.

| AL type | Wire / .NET type | Direction | Note |
|---|---|---|---|
| Text, Code | `string` | both | UTF-16 semantics, as ADR-010 decision 3. |
| Integer, BigInteger, Byte, Char, Boolean | `int`, `long`, `byte`, `char`, `bool` | both | |
| Decimal | `decimal` | both | Sent as a string, so no float round trip. |
| Date, Time, DateTime | `DateTime` | both | Kind and time zone rule needs a probe. |
| Duration | `TimeSpan` | both | |
| Guid | `Guid` | both | |
| Option, Enum | .NET enum | AL to .NET | Sent as the CLR enum type plus name or ordinal. Results come back as the enum name and ordinal. |
| `DotNet` variable | handle | both | An object is never copied into Rust. |
| Uninitialised `DotNet` | `null` | both | |
| InStream, OutStream, Stream | `System.IO.Stream` | AL to .NET | Bridged, see below. |
| .NET result: primitive, string, decimal, `DateTime`, `Guid`, enum | by value | .NET to AL | |
| .NET result: anything else, **including arrays** | handle | .NET to AL | AL declares such results as `DotNet Array` or another alias. Members like `Length`, `GetValue`, `Array.Copy` are plain calls. |

- **Overload choice** follows BC: by name, argument count, and the runtime
  types of the arguments (`NavAutomationHelper.GetMethodInfo`). The sidecar
  implements the same preference order (exact, assignable, enum from integer,
  numeric conversion). A test pins each rule against a corpus case or a probe.
- **`var` arguments** are sent with a by-reference flag. The reply returns the
  new values by position.
- **Properties and fields** use `get_property` / `set_property`. Indexers are
  calls to `get_Item` / `set_Item`.
- **`foreach` over a `DotNet` value** runs as `GetEnumerator`, `MoveNext` and `Current` host calls (ADR-024, pinned by the DotNet probe app).
- **Streams.** The sidecar cannot see Rust memory. The first version copies:
  1. Passing an `InStream`: Rust sends the unread bytes. The sidecar wraps them
     in a read-only `MemoryStream`. After the call, Rust moves the AL read
     position by the number of bytes .NET consumed.
  2. Passing an `OutStream`: the sidecar creates a writable stream (the
     **bridge**). After the call, and after every later call that could touch
     it, Rust appends the newly written bytes to the AL blob.
  3. The bridge lives as long as the handle that holds it, or until Session end.
     This covers a `StreamWriter` wrapped around an `OutStream`, because the
     sidecar does no background work. Every write is caused by an AL call, and
     the bridge is drained after that call.
  4. Known limit: an AL read of the same blob between two .NET calls sees the
     bytes written so far, and a later AL write can interleave differently than
     in BC. Large streams are copied once per call. If a corpus case needs true
     sharing, a callback protocol (sidecar asks Rust to read or write) replaces
     the copy. That would be a protocol change recorded in a new ADR.

  Binary payloads (a PNG as `byte[]`) stay in the sidecar as handles. A bulk
  copy between a .NET `byte[]` and an AL stream is one request each way.

### 5. Object lifetime

- A `DotNet` object lives as long as an AL variable references it, and no
  longer than the Session.
- At Session end, `close()` ends the sidecar process. All handles die with it.
- `Dispose` follows BC: BC's `NavServerHandle` has a `suppressDisposeInvocation`
  flag. We copy its rule when a probe shows it (section 13). Until then no
  `Dispose` is called implicitly, and a test that depends on it is refused.
- `DotNet` objects are not part of the database. `asserterror` and rollback do
  not touch them. Snapshots (ADR-004) do not capture them. We do not try to
  restore .NET state, because BC does not.

### 6. Sidecar per Session, statics isolation

- One sidecar process per VM Session, started lazily at the first `DotNet`
  call that reaches the host. A run that never calls `DotNet` starts nothing
  and pays nothing.
- A Session is a test codeunit's session (`docs/architecture.md`). Test methods
  in one codeunit share the sidecar. That matches BC, where static state lives
  as long as the service tier.
- A new Session means a new process, so static .NET state never leaks between
  test codeunits. BC does leak it (example: the static
  `NavTenantSettingsHelper.testEnvironmentTypeIsSandbox`). Isolation is
  allowed under ADR-009 (determinism "free to be better"). The risk is a test
  that passes in BC only because an earlier test codeunit left static state. The
  oracle run order is not recorded. A mismatch of that kind is reported as a
  probe candidate, not hidden.
- Start-up is about 50 to 65 ms (section 12) with the .NET files in the page cache, 1 s or more
  on a cold CI runner. A warm standby is allowed as an
  optimisation behind the same trait (built, see "Decided without owner review"): keep one idle, already-started sidecar
  per worker, hand it to the next Session that needs one, start the next one in
  the background. A standby has loaded no test assemblies, so it is
  indistinguishable from a fresh process.
- Crash containment is a goal: a crash or hang in managed code ends only that
  Session's sidecar.

### 7. Errors

- A .NET exception raised by a call surfaces as an AL error in BC
  (`NavNCLDotNetInvokeException`, seen in AL Runner's comments in
  `MetadataPatches.cs` as `NavNCLDotNetInvokeException(IsSandbox)`). The exact
  text AL sees through `GetLastErrorText`, the error code, and what
  `GetLastErrorCallStack` shows are **not pinned anywhere**. Hard rule 3 forbids
  inventing them.
- Until a probe pins the format, an exception thrown by a `DotNet` call
  raises `Signal::Unsupported`. The reason carries the CLR exception type and
  message from the sidecar, so a probe author can read it. The test is
  reported as not run.
- This includes the common BC test pattern `asserterror` around a failing .NET
  call. It is not run until the text is pinned.
- Once pinned, the mapping lives in one function in `al-vm`, next to the
  registry, with the probe cited.
- Failures that are not .NET exceptions are refusals, never AL errors:
  sidecar missing, sidecar died, call timed out, protocol mismatch, assembly or
  type not found, member not found, policy refusal (section 8).

### 8. What is refused

Policy is data in the repo (a list file next to the registry), checked in Rust
before a request is sent. Every refusal names the type or member and the rule.

1. **Unregistered server classes.** For the server-core assemblies of the
   platform package (`Microsoft.Dynamics.Nav.Ncl`, `.Types`, `.NavUserAccount`,
   `.Common`, `.Core`, `.Apps`, `.Management`, `.Cloud` and their siblings in the
   service folder root), a type is sent to the sidecar **only if** it is on the
   `pure` list. Everything else is refused as
   `server class 'X' is neither registered nor classified pure`. A type goes on
   the `pure` list only when a corpus test or a probe shows it works with no
   session. Example from the spike: `Base64UrlHelper` is pure.
   `NavTenantSettingsHelper` is not.
2. **Registered types, unregistered members.** Refused as `Type.Member`.
3. **Add-in assemblies that reach outside** (Exchange, CRM, Power BI, Azure AD
   Graph, Key Vault, Azure SDK, SMTP/Mail, OAuth clients). Refused: they
   need the network, which belongs to the host network capability (ADR-006).
4. **Environment-touching namespaces and types**, refused by name:
   `System.IO.File`, `FileInfo`, `Directory`, `DirectoryInfo`, `FileStream`,
   `System.IO.Path` temp and current-directory members; `System.Net.*`,
   `System.Net.Http.*`, `System.Net.Mail.*`; `System.Diagnostics.Process`;
   `System.Environment`; `System.AppDomain`; `System.Reflection.Assembly` load
   members; `System.Runtime.InteropServices.*`; `Microsoft.Win32.*`;
   `System.DirectoryServices.*`; `System.Security.Principal.*`;
   `System.Drawing.*` (AL Runner refuses it for the same reason, `docs/scope.md`
   3.16); `System.Threading.Thread.Sleep`.
5. **Clock and randomness that bypass the host**, refused by member in the
   first version: `DateTime.Now`, `UtcNow`, `Today`, `DateTimeOffset.Now`,
   `UtcNow`, `Guid.NewGuid`, `new Random()` without a seed, `Environment.TickCount`,
   `Stopwatch` timestamps, `TimeZoneInfo.Local`. Later these may route to the
   host clock with a protocol call that sets it.
   Amended by ADR-024: the static `DateTime.Now`/`UtcNow`/`Today` and `DateTimeOffset.Now`/`UtcNow` now read the host clock.
6. **Transitive calls cannot be refused.** If an allowed method opens a file
   internally (`XmlDocument.Load(path)`), the policy cannot see it. The sidecar
   runs with a restricted environment to limit damage (section 9). Determinism
   for real .NET code is best effort, and the ADR says so.
7. **Gaps** reported with a reason, never emulated: `foreach` over `DotNet`
   (until pinned), `Add-in` table and app-packaged DLL resolution (until phase
   3), true shared streams.

### 9. Determinism limits

- The sidecar runs with the invariant of the oracle: culture `en-US`, time zone
  UTC, a cleared environment (only what .NET needs to start), and a working
  directory that is an empty temporary folder. ICU globalization is used as in
  BC (invariant-only globalization would change string comparison and casing
  results; the spike used it and did not test the difference).
- The .NET runtime must be major version 8, because the oracle is BC 27.5 on
  .NET 8 (`Microsoft.Dynamics.Nav.Server.runtimeconfig.json` targets
  `net8.0`). Rolling forward to 9 or 10 is off by default, since BCL behaviour
  can change between majors. `--dotnet-allow-roll-forward` switches it on and
  prints a warning. For later BC builds, the framework version follows the
  selected build's runtimeconfig.
- BCL behaviour that depends on the host OS (`System.Drawing`, file paths,
  certificates) can differ from Windows. Known cases are refused by name.
- The same sidecar and the same inputs give the same outputs, except where
  section 8 item 5 applies.

### 10. How the sidecar is found and started

- Source: `tools/dotnet-sidecar/` (C#, `net8.0`, no NuGet packages, no Microsoft
  files). It is the spike's `Program.cs` hardened (section 12).
- Build: `dotnet publish -c Release --disable-build-servers -p:UseSharedCompilation=false` (a
  VBCSCompiler server left running holds the caller's file descriptors). Needs the .NET 8 SDK once. Core crates
  have no `build.rs` for this (ADR-006 spirit: builds do not spawn tools).
- Lookup order in `al-test-cli`:
  1. `--dotnet-sidecar PATH` or `ALRT_DOTNET_SIDECAR`.
  2. `dotnet-sidecar/AlDotNetSidecar.dll` next to the `al-test-cli` binary.
  3. A cache folder `~/.cache/al-runtime-rs/dotnet-sidecar/<hash of source>/`.
     If missing and `dotnet` has an SDK, `al-test-cli` builds it there once and
     says so. If there is no SDK, the result is "sidecar not built" (section 11).
- `dotnet` itself is found through `DOTNET_ROOT`, then `PATH`, then the usual
  install folders.
- The first message is `hello`. The sidecar replies with its protocol version
  and runtime version. A mismatch is a refusal.
- Protocol: one request, one reply, strictly in order, over stdin and stdout.
  Stdout carries only protocol bytes; anything else the .NET code prints goes
  to stderr. The spike uses JSON lines because it is easy to read. The
  production framing is length-prefixed, with the same tagged values. The
  format is internal to the sidecar and `SidecarHost` and may change without an
  ADR, as long as the trait is unchanged.
- Each call has a deadline (default 60 s). On expiry the process is killed and
  the call is refused. A dead sidecar marks the Session's `DotNet` as dead with
  the exit code and the last stderr line. The next Session starts a new one.

### 11. A run without .NET

A `DotNet` call is refused, never a crash. The reason says which case:

| Case | Reason text (shape) |
|---|---|
| No host configured (default, WASM, core tests) | `DotNet 'T' needs a .NET host; none is configured` |
| `dotnet` not found | `DotNet 'T' needs the .NET 8 runtime; 'dotnet' not found (looked in DOTNET_ROOT, PATH, ...)` |
| Runtime major is not 8 | `... found 9.0.15; the sidecar needs Microsoft.NETCore.App 8.x` |
| Sidecar not built | `... sidecar not built; run 'dotnet publish' in tools/dotnet-sidecar` |
| Protocol mismatch, died, timed out | named in the reason |
| Platform artifact missing for a Microsoft assembly | `... assembly 'X' not in the selected platform package` |

- The check is **dynamic**, at the first call. Static refusal would hide tests
  that reach a `DotNet` procedure but never run the line (ADR-013 decision 8,
  and `docs/dotnet-examples.md` case 3).
- `Signal::Unsupported` makes the test "not run", which is not a failure.
- The ratchet needs care: `metrics/passing.txt` will hold cases that pass only
  with .NET. A local run without .NET must not report them as regressions, and
  `--update-baseline` must not drop them.
  - Local default: a baseline case refused with `dotnet-unavailable` counts as
    **skipped**, listed in the report, not as a regression. `--update-baseline`
    refuses to run in this state.
  - CI sets `--require-dotnet`. Then unavailability is a hard error (exit
    code to be chosen), so the ratchet means what it says.
  - This does not weaken an oracle (hard rule 2): no line leaves
    `passing.txt`, and CI still enforces every one.

### 12. Spike results

Throwaway code in `tools/dotnet-sidecar-spike/` (not a workspace member, not
wired into the runtime). C# sidecar plus a Rust client over stdio, JSON lines.
Machine: AMD Ryzen 5 5600X, Linux 7.2, .NET runtime 8.0.30, built with SDK
8.0.130. Release build of the client. The sidecar is unoptimised: it reflects
with no method cache. Two full runs (`cargo run --release --bin bench`), 10 000
calls each, so the figures are ranges.

| Measure | Run 1 | Run 2 |
|---|---:|---:|
| Start-up: spawn to first reply (median of 10) | 47.8 ms | 65.2 ms |
| First reflection call after start (JIT, median of 10) | 4.3 ms | 6.4 ms |
| Load `Types`, `Ncl`, `NavUserAccount` DLLs of BC 27.5 plus first call | 10.5 ms | 17.1 ms |
| Round trip, `ping` (protocol floor), median / p99 | 17.8 / 43.5 us | 27.6 / 101.7 us |
| `Convert.ToBase64String(9 bytes)`, median / p99 | 74.8 / 135.7 us | 86.9 / 186.4 us |
| `Convert.FromBase64String(12 chars)`, median / p99 | 68.7 / 110.5 us | 45.5 / 91.2 us |
| `MemoryStream.WriteByte` through a handle, median / p99 | 39.5 / 68.3 us | 37.7 / 72.3 us |
| `Convert.ToBase64String(1 MiB)`, median of 50 | 13.7 ms | 11.0 ms |

The spike returns `byte[]` results by value to keep the test short. The
decision in section 4 is that arrays are handles. The spike is not the spec.

What the checks showed (all eight pass, `cargo test --release` in the spike):

- `Convert.ToBase64String` and `FromBase64String` work, including the overload
  with a `Base64FormattingOptions` enum argument.
- `Encoding.UTF8` is a property read that returns a handle; `GetBytes` on that
  handle works.
- A `MemoryStream` created in the sidecar takes `Write`, reports `Length`, and
  returns `ToArray`. Releasing the handle empties the handle table.
- `Convert.FromBase64String("not base64!")` comes back as
  `System.FormatException` with .NET's own message. A missing type is
  `TypeLoadException` and a missing method is `MissingMethodException`. The
  sidecar stays alive after each.
- From the BC 27.5.46862.53931 service tier in the local cache,
  `Microsoft.Dynamics.Nav.Types.Base64UrlHelper.EncodeBase64UrlString` and
  `DecodeBase64UrlBytes` run with no session. The DLLs load from the cache
  folder through a `Resolving` handler.
- `NavTenantSettingsHelper.IsSandbox()` and `IsProduction()` called through the
  sidecar fail with `NullReferenceException`. This confirms decision 2: the
  class needs a session that does not exist outside a service tier.
- In the BC 29.0.54011.55816 cache, `NavTenantSettingsHelper` is not exported
  from `Microsoft.Dynamics.Nav.NavUserAccount.dll` at all. Server-class
  locations change between builds, which is another reason to keep them in a
  registry keyed by the AL alias and declared type, not in loaded DLLs.

Reading the numbers:

- A call costs about 40 to 90 us, of which about 20 us is the protocol floor.
  A System Application Base64 conversion is a few calls, so tens to hundreds of
  microseconds per procedure. At most 28 corpus tests reach a `DotNet` use
  (`docs/dotnet-examples.md`).
- A loop of 100 000 `DotNet` calls would cost about 4 to 9 s. That is the case
  where a stand-in or a batching call would pay. No such test is known.
- Start-up of 50 to 65 ms per Session that uses `DotNet` is cheap enough for
  the first version. A warm standby (section 6) hides most of it.
- Large payloads cost about 11 to 15 ms per MiB with base64 in JSON. A binary
  frame removes most of that. It matters only for large images and documents.
- p99 is within about 2x the median. The maximum is dominated by the first
  call (JIT) and garbage collection pauses.

### 13. Probes needed on real BC (to record in `docs/probes-needed.md` when built)

1. The AL error text, error code and call stack when a .NET method throws
   (`FormatException` from `Convert.FromBase64String`; an exception with an inner
   exception), visible through `GetLastErrorText`, `GetLastErrorCode`,
   `GetLastErrorCallStack`, with and without `asserterror`.
2. Whether `:=` between `DotNet` variables shares the reference.
3. When `Dispose` runs for `IDisposable` results (the `suppressDisposeInvocation`
   rule).
4. How `Date`, `Time` and `DateTime` cross to `DateTime` (kind, time zone,
   undefined values).
5. What `InStream`/`OutStream` arguments do when .NET keeps the stream after the
   call (the retained-stream case in section 4).
6. Overload choice cases where two candidates fit (`Convert.ToBase64String`
   with `Array` and an `Integer`).
7. Each registry member of the four server classes, as in
   `docs/dotnet-generic.md` section 4.

## Consequences

- One new C# component (`tools/dotnet-sidecar/`) to maintain. The runtime stops
  being pure Rust for `DotNet` tests. Everything else stays Rust.
- The standard library, Newtonsoft, OpenXml, community assemblies and the
  Microsoft add-ins become reachable with no per-class work. Server internals
  stay Rust and stay bounded (about 4 types, 60 to 80 members).
- `DotNet` tests need a .NET 8 runtime. No WASM support for `DotNet`.
- The registry and the sidecar can give different answers for one class. The
  rule in section 1 (the sidecar path is the oracle) controls that.
- Static .NET state is isolated per Session. That is cleaner than BC.
- Licensing: we download and run Microsoft's DLLs from the public artifact feed
  and commit none (hard rule 7). The owner approved this. The BC server license
  text was not reviewed by an agent.
- The ratchet gains an explicit "needs .NET" class and a CI flag.
- Errors from .NET are not-run until the probe pins the text. Some `asserterror`
  tests stay dark until then.
- Per-call cost is an order of magnitude above in-process calls (about 40 to
  90 us). If a measured test loop makes this too slow, the options are a
  registry stand-in for that class, or running the same protocol in-process.

## CI requirements

- A job step `actions/setup-dotnet` with `dotnet-version: 8.0.x` in
  `.github/workflows/ci.yml`, before the corpus step. It provides both the SDK
  to build the sidecar and the runtime to run it.
- `dotnet publish tools/dotnet-sidecar -c Release -o <dir> --disable-build-servers -p:UseSharedCompilation=false` once per job. No
  NuGet restore is needed beyond the SDK's own targeting packs.
- ICU (`libicu`) is present on `ubuntu-latest`.
- The BC platform package for the selected build is in the artifact cache
  (ADR-013). It is cached across runs and never committed.
- `al-test-cli --require-dotnet` in the corpus step.
- The sidecar's own tests run only where `dotnet` exists. Without it they skip
  with a visible message, as the spike's tests do.

## Conformance

- `al-host` test: the default `NoDotNet` host returns `Unavailable` and the
  test host never spawns a process.
- `al-vm` tests with a fake `DotNetHost` that records requests: registry first,
  then host, then refusal; a registered type never reaches the host; an
  unregistered member of a registered type is refused by name; the release queue
  flushes on the next call and at Session end; `:=` shares the handle.
- Marshalling tests: a table-driven test per row of section 4, using the fake
  host, plus the same table run against the real sidecar when `dotnet` exists.
- Policy tests: every entry of the refusal list produces a refusal naming the
  rule; a `pure` entry reaches the host; an unclassified server class is
  refused.
- Sidecar tests (need `dotnet`): the spike's checks, plus statics isolation (a
  static set in one Session is absent in the next), kill-on-timeout, protocol
  mismatch, restricted environment (culture, time zone, working directory).
- Runner tests: with `dotnet` unavailable, a baseline case that needs it is
  reported skipped, `--update-baseline` refuses, and `--require-dotnet` fails.
- Registry boundary: for every registry entry, the member list names its pin
  (corpus `file:line`) or its probe id. A member without either does not
  compile into the registry.
- `no_microsoft_source_in_repo` (ADR-013) keeps covering the sidecar folder.
- The corpus run (`metrics/passing.txt`) with .NET present is the integration
  test.

## Implementation notes (branch `dotnet-impl`)

The sketches above became code with these differences. None changes a seam; they are the
"exact signatures belong to the first implementation" the ADR allows.

- `DotNetUnavailable` carries the end of the sentence `DotNet 'T' ...` (for example
  `needs a .NET host; none is configured`). The VM puts the marker `dotnet-unavailable:` in
  front, and the runner uses it to tell "no .NET on this machine" from a real failure.
- `StreamEffects` names bridges: `bridges` (argument position to bridge id, for `OutStream`
  arguments of this call), `written` (bytes per bridge id, for every live bridge) and
  `consumed` (bytes .NET read, per `InStream` argument). The VM keeps the AL stream behind each
  id until the session ends.
- `DotNetValue::Enum` keeps the CLR type, so a .NET enum result goes back as an enum argument.
  AL `Option` and `Enum` values go to .NET as their ordinal.
- Wire format: 4-byte little-endian length plus UTF-8 JSON, tagged values. `Time` is refused
  (the mapping is not known), as are undefined Date and DateTime values and closing dates.
- The registry, the policy (`policy.txt`) and the release queue live in `al-vm/src/dotnet/`.
  The registry ships empty. `Value::DotNet` is `Rc<DotNetObject>`; its `Drop` queues the handle.
- `Engine::with_dotnet(program, blocked, DotNetConfig)` carries the host; `Engine::new` uses
  `NoDotNet`.
- `SidecarHost` (in `al-test-cli`) finds the sidecar by `--dotnet-sidecar` or
  `ALRT_DOTNET_SIDECAR`, then `dotnet-sidecar/` next to the binary, then a cache keyed by the
  source hash, built once when the SDK is present. The sidecar starts with a cleared
  environment, `TZ=UTC`, and an empty working folder. Starting a process gets at least 30 s
  whatever the call deadline.
- Lowering refuses, with a reason, a `DotNet` value used as an operand, passed to a builtin,
  indexed, or iterated, because BC types such results from the assembly and lowering never
  opens it. Results are coerced exactly where they are stored.
- Not built: Fallback A and B of the resolution table (phase 3), the
  compile-time AL0296 check, `foreach` over `DotNet`, and a Rust-side method cache. (The warm
  standby was built later, see below.)

## Decided without owner review (2026-10-14, branch `sidecar-start`)

The warm standby of section 6 is built, for a run that has the Microsoft apps. No seam changes:
`DotNetHost` and `DotNetSession` are as in section 2; the standby is a method of `SidecarHost`
(`prewarm`, `shutdown`), which only the command lines call.

Why. The perf budget `a_full_corpus_run_with_the_microsoft_apps_stays_within_its_budget` failed once
on CI: two Media test codeunits took 7.4 s each, against a runtime of 0.02 s. Measured on CI, those
two codeunits spend 1.1 to 1.3 s of their 1.2 to 1.4 s starting the sidecar (a start is 0.055 s on the
dev box, 0.19 s with the .NET files dropped from the page cache). The runtime is 0.15 s for all
35 requests. The 7.4 s run was the same start on a slower runner. Two sessions start a process
each (one per session, section 6), at the moment the tests begin, on a runner whose four cores are
all busy.

What changed.

1. `al-test-cli` and `alrunner` call `SidecarHost::prewarm(2)` before the compile when the run
   loads dependency apps (not with `--no-deps`). Background threads find `dotnet` and start two
   sidecar processes, which answer `hello` and wait. They overlap the compile (2 to 4 s on CI).
2. A session that makes its first `DotNet` call takes one standby whole, or waits for one that is
   still starting, or starts its own if there is none. A standby is used by one session only, so
   static .NET state still does not leak between sessions (section 6). The host starts a
   replacement for each one taken (so a run with more than two sessions keeps being served), and
   `shutdown()` ends the unused ones at the end of the run (the sidecar also ends on its own when
   the parent exits, because its stdin closes).
3. A run without dependency apps, or with no `DotNet` call, still starts no process of its own
   accord; a run with the Microsoft apps starts two that may go unused (about 0.1 s of CPU,
   spread over the compile). This changes the sentence of section 6, "a run that never calls
   `DotNet` starts nothing", for runs with the Microsoft apps.
4. Reporting. `SidecarHost::stats()` counts processes started, the time each start took, the time
   sessions waited for theirs (zero for a standby that was ready), the time to find `dotnet`, and
   the requests. `--timings` prints a `dotnet:` line, `slowest test codeunits:` shows
   `(sidecar start X s)` for the part of a codeunit spent waiting for a process, and the markdown
   report ("How long it took") and `--json` carry the same numbers.
5. The perf budget measures the runtime: the per-codeunit limit of 2 s is applied to a
   codeunit's time minus its sidecar start. The start has its own limit (10 s for one process; a
   start is about 0.05 to 1.3 s). The limit was not raised.

## Decided without owner review (2026-10-21, branch `lane-perf-small-app`): standbys on first use with the compile cache

Item 1 of the decision of 2026-10-14 starts two standbys before the compile so that their start overlaps it. With
a compile cache hit the compile takes about 0.5 to 1 s, and many runs of a small partner app never call `DotNet`:
the two standbys then cost about 0.2 to 0.3 s of CPU and 0.9 G instructions of 7.4 G (a small partner app, one
test codeunit, extends Base Application), for nothing. Now:

1. `alrunner` with the compile cache on calls `SidecarHost::prewarm_on_first_use(2)` instead of `prewarm(2)`.
   Nothing starts before a session needs a process. The first session that needs one starts its own, and the two
   standbys start beside it for the sessions that follow (the pool then works as before).
2. `alrunner --no-compile-cache` and `al-test-cli` keep `prewarm(2)`: their compile takes seconds, so the starts
   overlap it.
3. Measured. The small partner app: 6.45 G instructions against 7.39 G, CPU 2.2 to 2.36 s against 2.35 to 2.56 s
   (5 alternating pairs, load 22 to 23 on 12 cores). Tests-ERM 27.5 with test data (cache on): 9 333 tests, 0
   outcome differences; the sessions waited 0.10 s in total for their processes, against 0.01 s (one start that
   no longer overlaps anything), out of 131 to 152 s of tests; 121 processes either way.
4. No outcome can change: a session gets a process of its own either way, and a standby cannot be told from a
   fresh process (section 6).

## Decided without owner review (2026-10-15, sidecar-find)

- A binary outside the repository finds its helper. `alrunner` carries the sidecar sources
  (`tools/dotnet-sidecar`, six files) built in (`al-testrun/src/dotnet.rs`, `EMBEDDED`). Lookup order is now:
  the flag or `ALRT_DOTNET_SIDECAR`; `dotnet-sidecar/` next to the binary; the repository's sources when the
  binary sits inside one; else the built-in sources. The last two use one cache,
  `~/.cache/al-runtime-rs/dotnet-sidecar/<hash of the sources>`, so a copied binary reuses a helper the repository
  build made, and otherwise builds it once from the built-in sources when a .NET 8 SDK is present. A test keeps
  the built-in list equal to the folder.
- Cache folder of the helper (issue 5747, tooling): `--cache-dir`, `ALRUNNER_CACHE` and the other cache variables of
  the app cache put it in `<cache root>/dotnet-sidecar`. Without them Linux and macOS keep the folder above; a
  machine with neither `XDG_CACHE_HOME` nor `HOME` (Windows) uses the default app cache root,
  `%USERPROFILE%\.cache\al-runtime\dotnet-sidecar`. With no folder at all, "sidecar not built" names the variable
  to set (`al-testrun/src/dotnet.rs`, `sidecar_cache_dir`).
- A run whose tests were stopped by an unusable helper (reason `dotnet-unavailable`) prints
  `WARNING: DotNet unavailable: N test(s) not run ...` before the results and again after the summary line, ends
  with `Result: DOTNET UNAVAILABLE (exit 8)`, and exits 8 (before 7; `--allow-not-run` still makes it 0). Why: on
  2026-10-15 a binary copied out of the repository lost 20 Tests-ERM and 30 NP Retail passes and looked like a
  regression.


## Amendment 2026-10-15 (owner decision, branch `environment-flag`): `--environment saas|onprem`

**Decision (owner).** The runtime emulates one server, and Environment Information answers what the probe
tier answered: a SaaS-like sandbox (`IsSaaS` Yes, `IsSandbox` Yes, `IsProduction` No, name `Sandbox`). Microsoft's
own test suites run on an on-prem server. Codeunit 134614 "Test App Permissions" (Tests-SINGLESERVER) deletes
every user in `InitializeData` and only afterwards calls `SetTestabilitySoftwareAsAService(true)`; on a SaaS
server the User Permissions subscriber `CheckSuperPermissionsBeforeDeleteUser` raises "There should be at least
one enabled 'SUPER' user." at the first delete (13 failures). So the server kind is a run setting:

1. `alrunner test --environment saas|onprem` and `al-test-cli --environment saas|onprem`. **Default `saas`**, as
   probed. The corpus runs, the ratchet and every baseline stay `saas`.
2. `onprem` makes the `NavTenantSettingsHelper` registry type answer `IsSandbox` false and `IsProduction`
   true (`IsProduction` is `!IsSandbox`); `ALConfigSettings.IsSaaS` (membership entitlement) is false in both.
   System Application's own AL then derives `IsSaaS` false, `IsSandbox` false and `IsOnPrem` true, and honors
   `SetTestabilitySandbox` / `SetTestabilitySoftwareAsAService` as its code says (nothing is faked on top).
   `GetEnvironmentName` stays the helper's empty name, so AL answers `Production`.
3. The host carries it: `al_host::Environment` on `Identity::environment()` (default `Saas`;
   `Host::in_environment`); the registry reads `s.host.identity.environment()`.
4. **The on-prem values are an unprobed assumption**, required by Microsoft's test (hard rule 3 exception,
   owner 2026-10-15; codeunit 134614). The OnPrem-suite probe (tests/al-language-onprem, the "BC OnPrem 29.0" leg)
   is requested; when it lands it wins. `docs/probes-needed.md` has the row; the code comments cite the test.
   The assumption registry of rule 3 is not in the code yet; the id gets registered when it is.
5. In scope with it (same exception): **deleting a User deletes its Access Control rows.** The corpus pins only the
   User Property cascade (TestUserDeleteCascadeOrder.al:26-84); the same test 134614 needs the roles of the users it
   deleted to be gone. It was refused before ("the platform deletes them too, which is not pinned"); it now runs
   (`usertable.rs`, `user_after_delete`), and is a second unprobed assumption.

**Measured** (BC 27.5.46862.48827, `alrunner test Tests-SINGLESERVER.Source.zip`, test data on):

| | `--environment saas` | `--environment onprem` |
|---|---:|---:|
| Codeunit 134614 (15 tests) | 0 passed, 13 failed, 2 not run | 0 passed, 0 failed, 15 not run |
| Whole bucket (922 tests) | 74 passed, 18 failed, 830 not run | 103 passed, 4 failed, 815 not run |

Codeunit 134614 does not pass yet on either setting. On `onprem` it now gets past `User.DeleteAll()` and stops
at the next gap: `DotNet new Microsoft.Dynamics.Nav.AzureADGraphClient.MockGraphQuery` (a test-library class
that is neither registered nor classified pure). The four failures of the on-prem run
(three `Document Approval - Comments` tests "The part with ID = ... was not found on the page", one ERM RS Package
Dimensions test "The Dimension does not exist") were refused before at `GraphQuery.GetCurrentUser`; they are
separate findings.
