# DotNet in Microsoft's AL: examples and the decision they need

Repo: al-runtime-rs. Task: ADR-013 (dependency apps). State: the owner has
answered every open question except this one, and asked for examples first.

## The question

Microsoft's System Application and Base Application declare variables of type
`DotNet`. Each one points at a .NET class, not at AL code. The runtime has no
.NET. So it must either refuse the procedure, or run Rust code in place of the
.NET class. The owner's rule says AL objects are taken as AL and never shimmed.
A .NET class is not an AL object, so the rule does not settle this. This page
shows what is at stake, with real cases.

The decision to make is at the end ("Decision needed").

## What a `DotNet` variable looks like

In `Environment Information Impl.` (System Application):

```al
var
    NavTenantSettingsHelper: DotNet NavTenantSettingsHelper;

procedure IsProduction(): Boolean
begin
    exit(NavTenantSettingsHelper.IsProduction())
end;
```

The name `NavTenantSettingsHelper` is an alias. A `dotnet { }` block in the same
app maps it to the real class, here
`Microsoft.Dynamics.Nav.NavUserAccount.NavTenantSettingsHelper` in the assembly
`Microsoft.Dynamics.Nav.Ncl`. The class has no AL source. It lives in
Microsoft's server code.

## How I counted

The numbers are approximate. They come from a text-based call graph, not from
our compiler.

1. Source: System Application, Base Application and Business Foundation from the
   BC 27.5.46862.53931 artifacts already in the local AL Runner cache. The
   oracle ran on a 27.5 build, but its exact build number is not recorded, so
   this may differ by a few builds. Nothing was copied into the repository.
2. Parse every procedure and trigger. Map each `DotNet` alias to its .NET class.
3. A procedure "uses DotNet" when it has a `DotNet` variable (local, parameter or
   object level) and the name appears in its body.
4. Follow calls from each corpus `[Test]` procedure: calls on `Codeunit`,
   `Record`, `Page` and `Report` variables, calls inside the same object,
   `Codeunit.Run`, and table triggers after `Insert(true)` and its siblings.
   A second run also follows event publishers to their subscribers.
5. A test "reaches" DotNet when some path ends at a procedure that uses it.

The script is `tools/dotnet_survey.py`. Run it on a machine that has the source
extracted. `--tests` lists every test with file and line.

"Reaches" is static. A test can reach a procedure and still never run the line
that uses DotNet (case 3 below shows how).

Known gaps. It under-counts calls through interfaces, `RecordRef`, `Variant`,
`Validate`, and page and report triggers. It over-counts calls matched by
procedure name only, and it assumes every subscriber of an event fires.

## Result

The corpus has 4838 test procedures.

| Run | Tests that reach a DotNet use |
|---|---:|
| Calls only | 20 |
| Calls plus event subscribers | 28 |

None of these tests passes today. Real BC passes all of them (the oracle junit
has no failure for them).

Across the three source trees, 2297 procedures use DotNet and 488 different .NET
types appear. 166 are Microsoft server classes (`Microsoft.Dynamics.*`), 236 are
`System.*` or `Newtonsoft.*`, and 86 are others (OpenXml and so on). The corpus
reaches 10 of them (calls only) or 19 (with events). So the corpus decides
very little. Real projects that depend on these apps will reach many more.

### Ranked by tests that reach the type

| .NET type | Calls only | With events | Kind |
|---|---:|---:|---|
| `System.Convert`, `System.Text.Encoding`, `System.IO.MemoryStream`, `System.Array` | 9 | 9 | Standard library |
| `NavTenantSettingsHelper` | 5 | 7 | Server internal |
| `NavUserAccountHelper` | 4 | 6 | Server internal |
| `ALConfigSettings` | 4 | 6 | Server internal |
| `System.Base64FormattingOptions` | 2 | 2 | Standard library |
| `NavDesignerALFunctions`, `NavDesignerALCopyResponse` | 1 | 1 | Server internal |
| `System.Xml.XmlDocument`, `XmlNode`, `XmlReaderSettings`, `XmlTextReader`, `System.IO.StringReader`, `UTF8Encoding` | 0 | 6 | Standard library |
| `NavTestExecution` | 0 | 2 | Server internal |
| `TenantInfo`, `GraphQuery` (Azure AD) | 0 | 2 | Server internal |

The tests overlap, so the rows do not add up to 20 or 28.

## Five concrete cases

Each case: the dependency procedure, the .NET type, what it does, and the
corpus tests that reach it.

### 1. Base64 Convert: `FromBase64` and `ToBase64`

- Object: codeunit `Base64 Convert Impl.` (System Application, `Base64ConvertImpl.Codeunit.al`).
- DotNet types: `Convert`, `Encoding`, `MemoryStream`, `Array`, `Base64FormattingOptions`.
- Use: decode or encode Base64 text. `FromBase64` calls `Convert.FromBase64String`
  and writes the bytes to an `OutStream`. `ToBase64` does the reverse.
- Reached by 8 tests through `FromBase64` and 2 through `ToBase64`. Example:
  `streams/media/TestMediaPngImport.al:70`, which imports a PNG given as Base64 text.
  The test's helper `ImportBase64` (line 258) calls `Base64Convert.FromBase64`.
  Others: `streams/TestPageFieldFormatsMediaFilled_Tests.al:22`,
  `reports/TestReportSaveAsPdfContent.al:137`.
- Why it matters: here the Base64 step is only a vehicle. The test is about
  Media fields. Refusing hides those results too.

### 2. Environment Information: `IsSandbox`, `IsProduction`

- Object: codeunit `Environment Information Impl.` (System Application).
- DotNet type: `NavTenantSettingsHelper`, a Microsoft server class.
- Use: ask the server whether the tenant is a sandbox or production.
- Reached by 3 tests through `IsSandbox` and 1 through `IsProduction`. Example:
  `session-system/TestEnvironmentInformationAgreement.al:26` asserts that
  `IsProduction()` is the opposite of `IsSandbox()`.
- Why it matters: the answer is the same data as the "Environment type" row of
  the identity table in ADR-013 (Production by default). If a shim exists, it
  reads that host setting. It adds no new behaviour to invent.

### 3. User Permissions: `IsSuper` reaches `HasUserPermissionSetAssigned`

- Object: codeunit `User Permissions Impl.` (System Application).
- DotNet type: `NavUserAccountHelper`.
- Use: `IsPermissionSetAssigned` asks the server whether a user holds a
  permission set. The AL code first checks the Access Control table itself and
  asks the server only when the direct check finds nothing.
- Reached by 3 tests. Example:
  `permissions/access-control/TestAccessControlBacksIsSuper.al:37`. That test requires that
  the permission answer and the Access Control table agree.
- Why it matters: the .NET call runs only after two AL checks fail. If the
  Access Control table holds the row, the AL code answers first. If the user does
  not exist, `User.Get` fails and the code exits first. Tests that stay on those
  two paths never reach the .NET call, so a refusal at the call would not
  show. The outcome depends on the path, not on the type.

### 4. Server Setting: `InitializeConfigSettings`

- Object: codeunit `Server Setting Impl.` (System Application).
- DotNet type: `ALConfigSettings`. `ALConfigSettings.Instance()` returns the
  server's configuration object, and methods like `ApiServicesEnabled()` read it.
- Use: read server configuration flags.
- Reached by 4 tests, mostly through the Environment Information path
  (`IsSaaS` reads it) and through a table event subscriber in another app
  (`record/TestTableEventAsyncSubscriberError.al:26`).
- Why it matters: the values are not pinned anywhere in the corpus. A shim would
  need real BC to say what each flag returns. Rule 3 in `CLAUDE.md` applies: a
  probe first.

### 5. Configuration/Personalization: `CopyProfile`

- Object: codeunit `Conf./Personalization Mgt.` (Base Application).
- DotNet types: `NavDesignerALFunctions` and `NavDesignerALCopyResponse`.
- Use: `NavDesignerALFunctions.CopyProfile` copies a profile. The AL code then
  reads the new row back from the `All Profile` table. So the .NET call must
  also write that row. It is a service operation on metadata, like the metadata
  tables the owner said we must create ourselves.
- Reached by 1 test: `profiles/TestCopyProfile.al:62` expects the error "could
  not copy" when the target profile id already exists.
- Why it matters: this one cannot be a pure function. A shim needs a write path
  into a platform table provider.

## Alternatives

| Option | What it is | What follows |
|---|---|---|
| Refuse | The first call on a `DotNet` variable raises `Signal::Unsupported` naming the .NET type (hard rule 4: refuse mid-run what cannot be seen statically). | No Rust code for .NET. At most 20 to 28 corpus tests stay not run. Tests that never run a DotNet line can still pass. |
| Shim in Rust | A Rust module stands in for a .NET class. It is registered by type name. The AL declaration stays real and runs. | Each shim needs a pin or a probe (hard rule 3). Standard-library classes are public and documented. Server classes are internal and need probes on real BC. |
| Run real .NET | Host a .NET runtime and load Microsoft's assemblies, as AL Runner does. | Exact behaviour. But the runtime stops being Rust-native, needs Microsoft binaries at run time, and conflicts with ADR-006. Not recommended. |

The shim option has two sizes:

- Standard-library classes only: `Convert`, `Encoding`, `MemoryStream`, `Array`,
  `Base64FormattingOptions`, and later `Xml*`, `Uri`, `Regex`. Their behaviour is
  public. Corpus tests such as the PNG import pin the observable result.
- Server classes as well: `NavTenantSettingsHelper`, `NavUserAccountHelper`,
  `ALConfigSettings`, `NavDesignerALFunctions`. These are service internals. The
  owner's second sentence ("anything that is a service internal structure ... we
  have to create ourselves") fits them. Some need probes.

## Decision needed

May the runtime contain Rust code that stands in for a .NET class declared in
Microsoft's AL, and if so for which classes?

1. **No. Refuse every `DotNet` use.** Nothing to write. At most 20 to 28 corpus
   tests stay dark, among them the Media and Base64 tests that use Base64 only
   as a vehicle.
2. **Yes, for standard-library classes only.** Write shims for `Convert`,
   `Encoding`, `MemoryStream`, `Array` and similar, when a test reaches them.
   Refuse server classes. About 9 tests (calls only) come into reach.
3. **Yes, for both, when a test reaches them.** As option 2, and also shims for
   server classes, each one backed by the identity configuration or by a probe on
   real BC. Registry only; an unregistered class is refused with its name.

My recommendation is option 3, built in order of need, with nothing in the first
increment. Reasons. The owner's rule separates AL objects (must run as AL) from
service internals (we create). A .NET class is neither AL nor stored data, and
the server classes are the second kind. Standard-library shims alone would leave
the Environment and Permissions tests, which the corpus does pin, dark. The risk
of option 3 is behaviour we guess wrong, so every shim must name its pin or its
probe, and an unpinned one stays refused.

Until the owner decides, ADR-013 treats every `DotNet` use as refused. That is
option 1, and it is the safe default under hard rule 4.
