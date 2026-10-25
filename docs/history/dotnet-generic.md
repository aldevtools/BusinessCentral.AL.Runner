# DotNet types: is there a generic way?

Repo: al-runtime-rs. Task: ADR-013 open question on `DotNet` variables.
State: research only. No code changed. Nothing pushed. This page ends in one
decision for the project owner (section 6).

Read `docs/dotnet-examples.md` first if you have not. It shows the five real
cases and the earlier proposal (Rust stand-ins per class). This page answers
the follow-up question: "do we have a more generic way?"

## Short answer

Yes. BC itself is generic here. A `DotNet` call in compiled AL is a late-bound
reflection call by name. BC finds the assembly on disk, loads it, and calls
`MethodInfo.Invoke`. A generic host can do the same for the standard library and
for community assemblies.

The generic host does not solve everything. Four Microsoft server classes
(`NavTenantSettingsHelper`, `NavUserAccountHelper`, `ALConfigSettings`,
`NavDesignerALFunctions`) read the running service tier's state. They cannot
work on their own. The runtime must provide their behaviour. The owner already
allows that for service internals.

So the answer is a hybrid: a generic host for arbitrary assemblies, plus a small
registry of runtime-provided types for the service internals.

All claims below come from files I read or decompiled. I did not run any
Microsoft or third-party binary. Where a number is my estimate, it says so.

## 1. How BC resolves DotNet types

### 1.1 The declaration

A `dotnet { }` block maps an AL alias to a .NET type. Example from System
Application (`Server Settings/src/dotnet.al`):

```al
dotnet
{
    assembly("Microsoft.Dynamics.Nav.Types")
    {
        Culture = 'neutral';
        PublicKeyToken = '31bf3856ad364e35';
        type("Microsoft.Dynamics.Nav.Types.ALConfigSettings"; "ALConfigSettings") { }
    }
}
```

- The assembly name is a name, not a path.
- The compiler binds each alias at compile time. It loads the assembly from
  probing paths and looks up the type. Failure gives `AL0185 DotNet '<type>' is
  missing`.
- A `Cloud`-target app cannot use `DotNet` at all (`AL0296`). Only `OnPrem` apps
  can. Source: AL Runner `docs/limitations.md`, section "DotNet interop".
  Our corpus app declares `"target": "Cloud"`
  (`corpus/tests/al-language/app.json:63`). So corpus code cannot use `DotNet`
  itself. It reaches `DotNet` only through Microsoft's apps. A community
  on-premises app can use it directly.
- At run time the emitted C# is late-bound. It builds
  `new NavDotNet(this, "<assembly>", "<type>", ...)` and calls members by name
  (`CreateDotNet`, `Invoke`, `InvokePropertyGet`). Source: header comment of
  AL Runner `AlRunner/Patches/DotNetInteropShims.cs`.

### 1.2 What the service tier does at run time

Decompiled from `Microsoft.Dynamics.Nav.Ncl.dll` and
`Microsoft.Dynamics.Nav.Types.dll`, BC 27.5.46862.53931. Class names are exact.

1. `NavDotNet.CreateDotNet` calls `CreateNavServerHandle`. That calls
   `NavAutomationHelper.CreateDotNetObject` (in `Nav.Types.dll`).
2. `NavAutomationHelper.GetAssembly` calls `Assembly.Load(fullName)`. It hooks
   `AppDomain.AssemblyResolve` first.
3. The resolve handler (`ResolveEventHandler`) tries, in order:
   1. The **Add-ins folder**: `<service folder>/Add-Ins`, searched recursively
      for `*.dll`, then `*.exe`. Also a temp folder
      `%TEMP%/Microsoft Dynamics NAV/Add-Ins/<platform version>`.
      Picks the best match by name and version.
   2. The **service folder itself** (the NST executable's directory), top level
      only.
   3. A registered add-in resolver (`navAddInResolver`).
   4. A registered **app add-in resolver** (`navAppsServerAddInAssemblyResolver`).
4. If creation still fails, `CreateNavServerHandle` falls back twice:
   1. The **Add-in table** in the database (`AddInProvider.ExtractAddInResourceFromAddInTable`).
      It extracts the blob to the temp Add-ins folder and retries.
   2. The **app's own assemblies**: `TenantNavAppsAddInRetriever` asks
      `NavAppAddInAssemblyManager` for assemblies packaged inside the installed
      extension. AL Runner docs call this the `addin/` folder of the `.app`
      (`docs/dependency-metadata-from-bc.md`, line 85).
5. Calls go through `NavDotNet.Invoke<T>`. It picks the `MethodInfo` by name and
   by the runtime types of the arguments (`NavAutomationHelper.GetMethodInfo`,
   with a `ParameterModifier` for `var` arguments). It then calls
   `methodInfo.Invoke(instance, args)`. Exceptions come back as
   `NavNCLDotNetInvokeException`.
6. Value conversion happens in `NavDotNet`. AL `Text` becomes `string`,
   `Decimal` becomes `decimal`, `Date` becomes `DateTime`, `InStream`,
   `OutStream` and `Stream` become `System.IO.Stream`, and so on
   (`NavDotNet.cs` around lines 289-345 and 655-720 in the decompile).

So BC's rule is: **name in, reflection, runtime-type overload pick.** No code is
generated per class.

### 1.3 What the platform package holds

The BC artifact has two parts: `w1` (apps) and `platform` (service tier, web
client, AL tooling).

Local caches I checked:

| Cache | Content |
|---|---|
| `~/.local/share/al-runner/artifacts/<build>/` | AL Runner's flat copy of the service-tier DLLs. For 27.5.46862.53931: 503 entries. Also `platform-apps/` (Base Application, System Application, Business Foundation, Application, `System.app`) and `test-apps/`. About 40 builds from 25.0 to 29.0 are cached. |
| `~/.bcartifacts.cache/sandbox/27.5.46862.48827/platform/ServiceTier/PFiles64/Microsoft Dynamics NAV/270/Service/` | The real service folder, with `Add-ins/` (44 sub-folders) and 363 top-level entries. Also `Test Assemblies/`, `WebClient/`, `ModernDev/`. |

Facts from the service folder:

- `Microsoft.Dynamics.Nav.Server.runtimeconfig.json` targets `net8.0`
  (`Microsoft.NETCore.App 8.0.0` and `Microsoft.AspNetCore.App 8.0.0`). BC 27 is
  ordinary .NET 8, not .NET Framework.
- The .NET base class library (BCL) is **not** in the artifact. It comes from
  the installed shared runtime. The declarations say `assembly("netstandard")`.
  On .NET 8 that is a forwarding facade onto the shared framework.
- The platform package also contains the AL tooling
  (`ModernDev/.../al development environment/System.app`).

### 1.4 Which DLLs supply the types Microsoft's AL uses

I parsed every `dotnet { }` block in System Application and Base Application
(27.5.46862.53931). Result: 587 declared aliases in 55 assemblies. I then counted
`: DotNet <Alias>` variable declarations across System Application, Base
Application and Business Foundation. The count is textual and approximate.
490 aliases are used. 2939 variable declarations in total.

| Assembly (file) | Where it lives | Declared types | Variable declarations |
|---|---|---:|---:|
| `netstandard` (BCL: `System.*`, `System.Xml.*`, `System.IO.*`, `System.Text.*`, `System.Net.*`, `System.Security.Cryptography.*` ...) | shared .NET runtime, not in artifact | 225 | 1781 (61 %) |
| `Newtonsoft.Json` (`JObject`, `JArray`, `JProperty` ...) | `Service/` root | 10 | 216 |
| `DocumentFormat.OpenXml` | `Service/` root | 63 | 113 |
| `Microsoft.Dynamics.Nav.Ncl` (incl. `NavDesignerALFunctions`, `NavTestExecution`) | `Service/` root | 50 | 104 |
| `Microsoft.Dynamics.Nav.LicensingService.Model` | `Service/` root | 7 | 97 |
| `Microsoft.Dynamics.Nav.CrmCustomizationHelper` | `Add-ins/crmcustomization` | 1 | 66 |
| `Microsoft.Dynamics.Nav.ClientExtensions` | `Add-ins/ClientExtensions` | 18 | 60 |
| `Microsoft.Dynamics.Nav.OpenXml` | `Service/` root | 8 | 53 |
| `Microsoft.Dynamics.Nav.NavUserAccount` (`NavTenantSettingsHelper`, `NavUserAccountHelper`) | `Service/` root | 5 | 46 |
| `System.Drawing.Common` | `Service/` root | 15 | 43 |
| `Microsoft.Dynamics.Nav.EwsWrapper`, `Microsoft.Exchange.WebServices.NETStandard` | `Add-ins/NavEwsWrapper`, `Service/` root | 14 | 58 |
| `Microsoft.Dynamics.Nav.PowerBIEmbedded` | `Add-ins/PowerBIEmbedded` | 6 | 41 |
| `System.Security.Cryptography.Xml` | `Service/` root | 23 | 31 |
| `Microsoft.Dynamics.Nav.Types` (`ALConfigSettings`) | `Service/` root | 6 | 10 |
| About 40 smaller assemblies (SMTP, MimeKit, MailKit, BarCodeFont, GraphClient, OAuth, DocumentReport, Timer ...) | `Add-ins/*` or `Service/` root | about 130 | remainder |

Top aliases by declarations: `XmlNode` 293, `XmlDocument` 144, `JObject` 144,
`Array` 132, `UserInfo` 80, `CrmHelper` 66, `Convert` 48, `XmlNodeList` 46,
`String` 44, `Encoding` 43, `MemoryStream` 38, `XmlNamespaceManager` 36, `Uri`
32, `X509Certificate2` 32, `Regex` 32.

What this says:

- About 61 % of uses are the BCL. About 7 % are Newtonsoft. About one third
  are Microsoft server and add-in assemblies.
- The BCL part is a long tail (490 aliases used), not four classes. Four
  stand-ins cover little. `XmlDocument`, `Regex`, `X509Certificate2`,
  `CultureInfo` and `JObject` each hide a large behaviour surface.
- Some assemblies are third-party with their own licenses: Aspose (commercial),
  IDAutomation (font encoders), MimeKit/MailKit, Newtonsoft.

I could not measure what community apps use. The local repos only contain
Microsoft's `dotnet.al` files. The data above is Microsoft's use only.

## 2. How AL Runner handles DotNet today

AL Runner (C#, checkout `/home/stefan/Documents/Repos/community/BusinessCentral.AL.Runner`,
commit 351c2813d, 2026-10-06) does not reimplement `DotNet`. It runs BC's own
compiler and BC's own `NavDotNet`. Paths are relative to the checkout.

- **Resolution at compile time.** `AlRunner/BcCompiler.cs`
  `GetOrCreateDotNetFactory` (line 600) builds BC's `AssemblyLocator` with the
  probing paths from `BuildDotNetProbingPaths` (line 652), in this order:
  1. staged shim directory (an assembly whose tier copy does not bind),
  2. .NET reference packs (`NETCore.App.Ref`, `NETStandard.Library.Ref`), because
     the shared framework's `netstandard.dll` is a forwarding facade that BC's
     compile-time metadata reader does not follow,
  3. the artifact's service-tier directory,
  4. `Add-ins/*` of the service tier and `Test Assemblies/*` from the highest
     version in `~/.bcartifacts.cache/sandbox` (`EnumerateServiceTierAddinDirs`,
     line 842),
  5. the shared runtime directory, last.
- **No probing path for app-supplied or user-supplied DLLs.** I found no code
  that reads `assemblyProbingPaths`, an app's `addin/` folder, or a
  command-line option for extra assembly folders. A community on-premises app
  with its own DLL would not bind. This is my reading of the search results,
  not a tested fact.
- **Resolution at run time.** `docs/assembly-resolution.md` and
  `AlRunner/DependencyLoader.cs`: default-context `Resolving` handlers serve
  assemblies by simple name from the artifact directory, with a version guard.
- **Reference packs.** `AlRunner/BcCompiler.DotNetRefPacks.cs`: a runtime-only
  .NET install without `packs/` makes every alias fail `AL0185`. The runner
  prints a clear warning.
- **Scope statement.** `docs/scope.md` section 3.14: the `DotNet` surface is in
  scope and runs on BC's `NavDotNet` runtime, on the same terms as BC.
  `docs/limitations.md` ("DotNet interop", line 447).
- **Platform refusals.** `docs/scope.md` section 3.16 and
  `AlRunner/Patches/DotNetInteropShims.cs`: `System.Drawing` throws
  `PlatformNotSupportedException` on Linux, so it is refused by name.
  `SecurityIdentifier` has a small runner-provided stand-in because the
  framework type refuses on Linux.
- **Absent assemblies.** `AlRunner/Patches/NavDotNetPatches.cs`: a Cecil patch
  turns a missing add-in assembly (silent `NullReferenceException`) into
  `RunnerOutOfScopeException` that names the assembly.
  `docs/limitations.md` line 507: `MockAzureKeyVaultSecretProvider` ships in no
  artifact and is dropped.
- **The server classes work only because of a faked session.**
  `AlRunner/Patches/MetadataPatches.cs` around line 780 builds a skeleton
  `NavSession`, system tenant and `TenantSettings` by `GetUninitializedObject`
  and field-poking, so that `NavTenantSettingsHelper.IsSandbox()` does not hit
  a null `Session.Tenant`. `NavDotNetPatches.InvokeReflectedMember` (line 51)
  replaces one member (`NavUserAccountHelper.IsUserSuperInAllCompanies`) with
  an answer from the Access Control rows. `AlRunner/BcRuntime.cs` sets the
  user, company, session and tenant values (ADR-013, "What AL Runner does", item 3).
- **State between tests.** `AlRunner/Infrastructure/AlSessionStateTracker.cs`
  line 169: static .NET state "cannot be told apart per field", so any DotNet
  call counts as both a read and a write of one hidden kind. AL Runner does not
  isolate it. It only records it for affected-test selection.

Takeaway: AL Runner is generic because it is C# on top of BC's own runtime.
It gets the generic part free. It pays by keeping a hand-built fake of the
service tier's session. That fake is the same work we face for the four server
classes.

## 3. Generic options

Common facts for options a, b and d:

- Reflection and overload choice must follow `NavAutomationHelper` (section 1.2)
  to match BC. Values to marshal: text, integers, decimal, boolean, date/time,
  Guid, byte arrays, enums as integers, `InStream`/`OutStream`/`Stream`,
  `var` arguments, `foreach` over enumerables, generics, nested `DotNet`
  results, null and exceptions.
- Streams are the hard part. AL streams live in Rust. .NET wants a
  `System.IO.Stream`. Either copy data across at the call, or give .NET a proxy
  stream that calls back into Rust.
- Compile-time checks (`AL0185`, `AL0296`) need only "assembly exists, type
  exists, target is OnPrem". That can be done in pure Rust by reading .NET
  metadata from DLL bytes. A host capability hands the bytes to `al-sema`.
  A Rust ECMA-335 reader crate would be needed. I did not verify a specific
  crate. This part is needed in every option except per-class stand-ins.
- Either option needs managed code of our own (a small C# helper that does
  `Assembly.Load`, `GetType`, overload pick and `Invoke`). Native code cannot
  call `System.Reflection` directly through hostfxr. It can only call managed
  static methods that the host has exported. So option a is not "pure Rust"
  either.
- .NET itself must be installed on the machine that runs tests. BC 27 needs
  .NET 8. This machine has 8.0.30, 9.0, 10.0 and the 10.0.111 SDK.

### 3a. Host the .NET runtime in-process from Rust

- **How.** Load `hostfxr` (via `nethost`), start the runtime, get function
  pointers to a managed helper, load assemblies, reflect, marshal. The
  `netcorehost` crate does this (MIT; wraps nethost and hostfxr; function
  pointers via `AssemblyDelegateLoader`). I read its README only. I did not
  verify release status or platform list. Its README does not say how
  unload or multiple runtimes work.
- **Feasibility.** High on Linux and Windows. Not possible in WASM.
- **Effort (my estimate).** A first slice (start runtime, create, call, strings,
  numbers, bytes) is about one to two weeks. Full marshalling (streams,
  `var` args, enumerators, generics, enums, exceptions) is several more weeks.
- **Risks.**
  - Microsoft's hosting docs say one .NET runtime per process, and it cannot be
    unloaded. I know this from the hosting documentation. I did not re-check it
    today. If true, per-test isolation must use `AssemblyLoadContext` or a
    cleanup protocol (section 5).
  - A crash or hang in managed code takes the whole test process down.
  - `unsafe` FFI and GC handle bookkeeping in Rust.
  - Parallel tests (ADR-009 allows them) share one runtime and its statics.
- **Licensing.** .NET runtime and `netcorehost`: MIT. Microsoft's `Microsoft.Dynamics.*`
  DLLs are proprietary and are not ours to commit (ADR-013 hard rule 7). We
  would download them from Microsoft's feed, as AL Runner does. Third-party DLLs
  in the service tier carry their own terms (Aspose is commercial). I did not
  review the BC server license terms for running these DLLs outside a licensed
  BC install. The owner should check that once (section 6).
- **ADR-006.** Fits only as a host capability. `dlopen`, process-wide runtime
  and file access are OS access, so the code lives in a crate behind an
  `al-host` trait, not in a core crate. Core crates stay WASM-clean. The
  runtime without this capability still builds for WASM and refuses `DotNet`.

### 3b. Out-of-process .NET sidecar

- **How.** A small C# program (the "sidecar") runs as a child process. Rust
  talks to it over stdin/stdout with a small length-prefixed protocol. Calls:
  load assembly, resolve type, create, invoke (static/instance, with `var`
  slots), get/set property, enumerate, release handle, and a stream callback
  for `InStream`/`OutStream`. Values are tagged. Objects stay in the sidecar
  and Rust holds integer handles.
- **Feasibility.** High. Same reflection code as option a. Works wherever a
  process can start. Not in a browser (a remote sidecar over a socket could
  serve a browser, but that is out of scope).
- **Effort (my estimate).** About the same as option a plus the protocol: one to
  two weeks for a first slice. The protocol is the part that gets reviewed once.
- **Risks.**
  - Call cost: one round trip per call. I have not measured it. Section 6
    makes the measurement the first step.
  - A C# project in a Rust repo. The sidecar needs the .NET SDK to build, or a
    prebuilt binary that we do not ship.
  - Stream callbacks make chatty protocols for large data.
- **Isolation gain.** A sidecar per session gives real isolation of statics and
  crash containment. A pool allows parallel tests.
- **Licensing.** Same as option a. No FFI to review.
- **ADR-006.** Best fit. The VM talks to a trait. The process spawn lives in an
  `al-host` implementation or in `al-test-cli`. The protocol can later run
  in-process (option a) without changing the VM.

### 3c. Per-class Rust stand-ins (the earlier proposal)

- **How.** A registry: .NET type name to Rust implementation. Unregistered
  types are refused with their name (hard rule 4).
- **Feasibility.** Always possible, WASM-compatible, fastest at run time,
  deterministic by construction.
- **Effort (my estimate).** Days for `Convert`, `Encoding`, `MemoryStream`,
  `Array`, `Base64FormattingOptions`. Weeks for `XmlDocument` family
  (DOM semantics, namespaces, XPath). Open-ended for `Regex` (.NET semantics
  differ from the Rust `regex` crate), `CultureInfo` formatting, `X509Certificate2`,
  `JObject` (Newtonsoft, 144 uses) and `System.Data.DataTable`.
- **Risks.**
  - Each class needs corpus pins or probes on real BC (hard rule 3). Edge
    cases of a re-implemented BCL are where mismatches hide.
  - It cannot cover community assemblies at all.
  - The long tail in section 1.4 is the problem, not the first five classes.
- **Licensing.** Clean. Behaviour comes from public documentation and probes.
- **ADR-006.** Fits trivially, because it is pure code.

### 3d. Hybrid

- **How.** One provider interface in `al-host`:
  1. a registry of **runtime-provided types** is asked first (service
     internals, plus hot BCL classes later),
  2. otherwise the **generic host** (option b, later also a),
  3. otherwise refuse with the type name.
- **Feasibility.** Highest. Each layer can ship alone.
- **Risks.** Two implementations of one class can drift. Rule: a stand-in is
  added only when it is measured faster, or it must work in WASM, and it must
  pass the same tests as the generic path. The generic path is the oracle for
  the stand-in.
- **Licensing and ADR-006.** As the layers above.

## 4. Do the BC server classes work outside a service tier?

I decompiled the four classes with `ilspycmd` (reads IL, does not execute it)
from `Microsoft.Dynamics.Nav.NavUserAccount.dll`, `Nav.Types.dll` and
`Nav.Ncl.dll` (27.5.46862.53931). All four are `static` classes.

| Class | Needs | Works standalone? |
|---|---|---|
| `ALConfigSettings` | `ServerUserSettings.Instance` (a singleton that reads the service's `CustomSettings.config`). Each member returns one setting. | Closest to standalone. It needs `Nav.Types.dll` and a config source. AL Runner notes the config data file is "not present in headless mode" (`BcRuntime.cs:2167`). So it needs our own settings values. |
| `NavTenantSettingsHelper` | `NavCurrentThread.Session` (a thread-bound `NavSession` from `Ncl.dll`), then `Session.Tenant.TenantSettings`. `GetApplicationFamily` reads `NavGlobal.AppDatabase.SqlDatabaseProperties`. `EnableM365Collaboration` writes to the tenant database. `GetRingName` and similar read server settings JSON. | No. It throws a null reference without a session. AL Runner makes it work by building a fake session with field-poking. |
| `NavUserAccountHelper` | `Session`, `UserManagement`, `PermissionManagement`, `ALDatabase.UserName`, `Session.Authenticator`, `Session.User`. The Windows-group members call `System.DirectoryServices.AccountManagement`, which is Windows-only. `CreateUserFromAAdGroupObjectId` needs Entra ID and "inserts the user record in a separate session" (System App comment, `SecurityGroupImpl`). | No. Even with a faked session, the Windows and Entra members cannot work off Windows or off a real tenant. |
| `NavDesignerALFunctions` | `NavCurrentThread.Session.CurrentDesignerExtension`, `NavGlobal.NCLMetadata`, `NavEnvironment.Instance.ResourceExposurePolicyProvider`, `NavDesignerExportManager`, `NavDesignerRecompilationManager`. | No. It edits designer extension state and metadata. It cannot run without the whole metadata and app system. |

Use in Microsoft's AL (System Application): `NavTenantSettingsHelper` has 8
distinct members called (`TryGetStringTenantSetting`, `IsProduction`,
`IsSandbox`, `GetEnvironmentName`, `GetApplicationFamily`,
`EnableM365Collaboration`, `GetLinkedPowerPlatformEnvironmentId`,
`GetEnvironmentApplicationSetting`). `ALConfigSettings` has 12.
`NavDesignerALFunctions` has 3 in System Application
(`IsDownloadSourceCodeAllowedForCurrentUser`,
`GenerateDesignerPackageZipStreamByVersion`, `SanitizeDesignerFileName`) and more
in Base Application (`CopyProfile`). `NavUserAccountHelper` is the widest: users,
permissions, groups, tokens, web service keys, authentication state.

What this means:

1. **The generic host cannot be the answer for these four.** To run the real
   classes, the host would have to load `Ncl.dll` and `Types.dll` and the
   dependency closure of the service tier (AL Runner provisions about 55 of
   its DLLs, ADR-013), then fake `NavSession` by reflection as AL Runner does.
   That reproduces the part of AL Runner that ADR-013 and ADR-009 let us avoid.
   It also ties our results to BC's internal object layout, which changes
   between builds.
2. **The runtime provides their behaviour instead.** The owner already allows
   "service internal" structures. Each member becomes a runtime-provided
   function that reads our identity, environment, session and settings
   configuration (ADR-013, decided items). Examples:
   - `IsProduction`/`IsSandbox`/`GetEnvironmentName`: the same environment-type
     setting that `Environment Information` needs. AL Runner's default is
     Production.
   - `ALConfigSettings`: the settings values. The shipped `CustomSettings.config`
     lists defaults (for example `ApiServicesEnabled` true,
     `TestAutomationEnabled` true, `UsePermissionSetsFromExtensions` true,
     `EnableEntraGroupsOnPrem` false, `EnableMembershipEntitlement` empty).
     Those are the service's defaults, not proof of what the oracle tenant
     returned. Hard rule 3 still applies: pin each value from the corpus or a
     probe, or keep it refused.
   - `NavUserAccountHelper.IsPermissionSetAssigned`/`GetEffectivePermissionForObject`:
     answered from the Access Control and permission tables in our store.
   - Windows-group, Entra ID and token members: refuse by name. They have no
     meaning without a real directory.
   - `NavDesignerALFunctions`: refuse by name, except where a corpus test pins
     behaviour (the `CopyProfile` case in `docs/dotnet-examples.md`).
3. **The count is small and closed.** Four types, about 60 to 80 members,
   all in Microsoft's server code. The long tail of section 1.4 does not
   include them. So "service internals in the runtime, everything else
   generic" is a bounded split.

I did not execute any of these classes. The statements above come from reading
decompiled IL and from AL Runner's own comments.

## 5. Determinism and test isolation (ADR-009)

What BC does, which we must match where AL can see it:

- `DotNet` objects are not part of the database. Rollback and commit do not touch
  them. `asserterror` rolls back database changes. The .NET object, and any
  state in it, stays as it was. A runtime that tried to roll back .NET state
  would differ from BC.
- Static .NET state lives as long as the service process. Example:
  `NavTenantSettingsHelper.testEnvironmentTypeIsSandbox` is a static field that a
  test sets and no one resets. It leaks across sessions in BC itself. A fresh
  host process is cleaner than BC. That is allowed ("determinism ... free to be
  better", ADR-009), as long as no passing BC outcome changes.

How the pieces meet our design:

- **Per-test snapshot and restore (ADR-004).** The store snapshot is O(1)
  persistent-map root swap. It cannot capture .NET objects. At the test
  boundary we start a fresh Session. All handles that Session holds are
  dropped. That is the one reliable reset point.
- **Handles.** AL variables of type `DotNet` hold a handle (an integer), not the
  object. A Session owns its handles. When the Session ends, the host releases
  them. `Dispose` runs as in BC (`NavServerHandle` has a
  `suppressDisposeInvocation` flag; copy that rule).
- **Statics.** Per option:
  - Sidecar per Session: statics reset with the process. Strongest isolation,
    highest start-up cost. A warm pool amortizes the cost.
  - One sidecar per run, one `AssemblyLoadContext` per Session: statics of
    loaded assemblies reset when the context unloads. BCL statics
    (`CultureInfo.CurrentCulture`, `Environment`, `Console`) are shared, because
    the BCL is loaded once.
  - In-process runtime (option a): one runtime per process. Only the
    `AssemblyLoadContext` approach is available.
  - Stand-ins: no hidden state by construction.
- **Environmental calls that .NET code makes itself** break ADR-006 purity and
  determinism: `DateTime.Now`, `Guid.NewGuid`, `Random`, file system, network,
  environment variables, time zone, culture. A generic host runs the real code,
  so these calls bypass our clock, identity and network capabilities. Ways out:
  - run the sidecar with a fixed culture, a fixed time zone and a restricted
    environment,
  - refuse known problem namespaces by name (`System.IO.File`,
    `System.Net.*`), as AL Runner refuses `System.Drawing`,
  - or intercept time through `TimeProvider` where the .NET code allows it.
  The honest statement is: determinism for real .NET code is best effort.
  `System.Drawing` on Linux (AL Runner `docs/scope.md` 3.16) shows that the
  host OS can also change results.
- **Parallel tests.** One sidecar or one `AssemblyLoadContext` per worker avoids
  cross-test statics. One shared in-process runtime does not.
- **The runtime-provided types** (section 4) read Session configuration, so they
  reset with the Session and stay deterministic. This is a point in favour of
  the hybrid.

## 6. Recommendation

### The decision

Question for the project owner: **How should `DotNet` calls be served, given
that the four server classes cannot run on their own and the standard library
is a long tail?**

In every option below, the four server classes (`NavTenantSettingsHelper`,
`NavUserAccountHelper`, `ALConfigSettings`, `NavDesignerALFunctions`) are
provided by the runtime as service internals, each member backed by a pin or
a probe. The options differ in what serves everything else.

1. **Generic host via a sidecar process, with a runtime-provided registry on
   top (my recommendation).**
   - What it does: a small C# sidecar loads real assemblies and reflects, as BC
     does. It serves the BCL, Newtonsoft, OpenXml, Microsoft add-ins and
     community assemblies. Core crates stay pure. Stand-ins for hot BCL classes
     are added later only if measured.
   - What follows: one new C# component to maintain. .NET 8 or newer must be
     installed where `DotNet` tests run. Anything without .NET is refused.
     Nothing is available in WASM.
   - Why this over in-process: both need a C# helper, so the in-process option
     gains no purity. The sidecar adds crash isolation and per-Session
     statics isolation. The protocol can later move in-process without
     changing the VM.
   - First small step: write the ADR "DotNet provider seam" as Proposed (a
     trait in `al-host`, a `Value` handle variant, a registry before the host)
     and ask for acceptance (hard rule 5). Then a throwaway spike outside the
     core crates: a sidecar that answers `Convert.ToBase64String`,
     `Convert.FromBase64String`, `Encoding.UTF8.GetBytes`, `new MemoryStream`
     and `Array.Copy` over stdin/stdout, driven from an `#[ignore]`d test in
     `al-test-cli`. Measure the per-call cost and the start-up cost. Decide
     from those numbers whether a warm pool is needed.

2. **Generic host in-process (hostfxr, `netcorehost`), with the same registry.**
   - What it does: same reflection helper, loaded into the test process.
   - What follows: no per-call process hop. A managed crash kills the test run.
     One runtime per process, no unload. Isolation relies on
     `AssemblyLoadContext` only. `unsafe` FFI in the host crate.
   - First small step: a spike that starts the runtime with `netcorehost`,
     calls one managed static method that returns a string, and measures
     start-up inside `cargo test`.

3. **Stand-ins only, no generic host (the earlier proposal).**
   - What it does: Rust classes for chosen BCL types, plus the four server
     classes. Everything else stays refused.
   - What follows: pure, fast, WASM-capable. Coverage is whatever we write.
     Community assemblies never work. The BCL tail (XML, Regex, certificates,
     JSON) grows without bound.
   - First small step: the `Convert`/`Encoding`/`MemoryStream`/`Array`/
     `Base64FormattingOptions` set behind the refuse-by-default registry, with
     the reaching corpus tests (`media/TestMediaPngImport.al:70`) as the pin.

Why I lean to option 1, in the order I reasoned:

1. The corpus decides little (10 to 19 types). Real projects reach hundreds
   (490 aliases are used in Microsoft's own three apps).
2. The BCL stand-in surface is large and each class needs pins or probes.
3. BC itself is late-bound reflection, so a reflection host matches BC's
   mechanism and its overload rules by construction.
4. Community on-premises assemblies can only be served by loading them.
5. The service internals cannot be served by any generic host, so they need
   the registry in all options.

What would change my mind: a measured per-call cost that makes common loops too
slow (then add stand-ins for the hot classes, or go in-process), or a decision
that tests must run without .NET installed (then option 3), or a licensing
answer that bars running Microsoft's DLLs outside a licensed BC server (then the
generic host serves only the BCL and community assemblies; Microsoft add-ins
would be refused).

### Open items for the owner, not blocking the first step

- Licensing: may we download and run `Microsoft.Dynamics.*` and the third-party
  service-tier DLLs from the public artifact feed for tests? AL Runner does so
  today. I did not review the license text.
- Cloud target: our corpus app is `Cloud`. A `DotNet` test of our own needs an
  `OnPrem` target app. Microsoft's apps use `DotNet` already, so ADR-013 work
  can start without it. AL Runner hit the same limit (`AlRunner#4014`).
- Determinism policy: which environmental .NET namespaces are refused by name.

## Sources

- `docs/dotnet-examples.md`, `docs/adr/006-core-purity.md`,
  `docs/adr/009-observable-equivalence.md`, `docs/adr/013-external-objects.md`,
  `docs/adr/004-snapshot-store.md`, `CLAUDE.md`.
- AL Runner checkout (commit 351c2813d): the files named in section 2.
- Decompiled with `ilspycmd` 9.1 (no execution) from BC 27.5.46862.53931:
  `Microsoft.Dynamics.Nav.NavUserAccount.dll`, `Microsoft.Dynamics.Nav.Types.dll`,
  `Microsoft.Dynamics.Nav.Ncl.dll`.
- Artifact listings: `~/.local/share/al-runner/artifacts/27.5.46862.53931/`,
  `~/.bcartifacts.cache/sandbox/27.5.46862.48827/platform/ServiceTier/`.
- Declaration and usage counts: my own text scan of System Application, Base
  Application and Business Foundation 27.5.46862.53931. Approximate. Scratch
  files were kept outside the repository.
- `netcorehost` README: https://github.com/OpenByteDev/netcorehost
