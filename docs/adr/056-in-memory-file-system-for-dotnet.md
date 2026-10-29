# ADR-056: .NET file access runs against the session's in-memory file system

- Status: Accepted (owner, 2026-10-29: file I/O is simulated in memory per run)
- Date: 2026-10-29

## Context

AL code reaches files in two ways. AL's own `File` type already runs on the
host file system capability (ADR-006, `al-host::FileSystem`); its default,
`MemFileSystem`, keeps every file in memory, and `--files-dir DIR` puts them on
a real directory. The second way is .NET: `System.IO.File`, `Directory`,
`FileInfo`, `DirectoryInfo`, `FileStream`, the temp members of `System.IO.Path`,
and members of other classes that open a file by its path
(`XmlDocument.Save(path)`, `new StreamReader(path)`). ADR-014 section 8 item 4
refused all of these by policy. Only the company build (ADR-013, owner rule
2026-10-22) had its temporary folder served by its own file system
(`al-testrun/src/company_data.rs`).

Microsoft's test libraries mix both ways. Library - Report Dataset saves a
report's dataset with `Report.SaveAsXml` into `FileManagement.ServerTempFileName`
(an AL `File` name), then checks it with `DotNet System.IO.File.Exists` before
reading it with AL's `File`. Every report test that uses the library stopped at
that `File.Exists`: 21 tests of NP Retail, and many of Microsoft's own.

The owner, 2026-10-29: "I'm not sure if there is any reason to not simulate
their file writes in memory, because tests should run the same way over and over
again. So if I execute the same test twice and it really writes something onto
disk that doesn't get cleaned up, then the second test will have the file
existing already and behaves differently."

The `XmlDocument.Save(path)` case also showed a defect: outside the company
build the path went to the .NET helper, which wrote into its own working folder
on the real disk (corpus `RD_ServerTempFileName_XmlSave_Compress` was refused
with a `DirectoryNotFoundException` from that folder).

## Decision

### 1. One store: the session's file system

.NET file calls use the same file system as AL's `File` type, so a file AL
wrote is a file .NET sees and the other way round. When that file system is in
memory (`FileSystem::in_memory`, true for `MemFileSystem`), .NET's static
`System.IO.File` and `System.IO.Directory` calls of section 3 are answered from
it in `al-vm` (router step 1d, `dotnet/mod.rs`), before the policy, and never
reach the .NET helper.

`MemFileSystem` gains folders: the folders AL code created, plus the run's two
temporary folders, which always exist (`\TEMP\`, what `TemporaryPath()`
returns, and `temp/`, where `File.CreateTempFile` and so
`FileManagement.ServerTempFileName` name their files). A folder path reads `/`
as `\` and drops closing separators (`Directory.Exists(Path + '/')` is Yes,
QxDotNetDemoShapes.al:288). File paths match exactly, as AL's `File` already
does.

### 2. Lifetime: one store per session, that is per test codeunit

The runner makes one `Session`, with a fresh host, per test codeunit
(`al-testrun/src/runtime/run.rs`, `run_codeunit_core`). The store lives as long
as the session:

- **Between the tests of one codeunit** files are kept. AL globals and
  single-instance codeunits are kept there too, and the database is kept under
  `TestIsolation = Codeunit`. On real BC files are not part of the database
  transaction, so no rollback undoes them; the runtime does not undo them
  either, under `Codeunit`, `Function` or `Disabled` isolation.
- **Between test codeunits** every codeunit starts with an empty store. This is
  where the runtime deliberately differs from real BC, whose disk keeps what an
  earlier codeunit or an earlier run wrote. It is the owner's rule: the same test
  gives the same result on every run, whatever ran before it, and codeunits can
  run in parallel. It holds under `Disabled` isolation as well, which keeps the
  database across codeunits.
- The install baseline runs in its own session; files it writes do not reach
  the tests (the baseline carries the database only).
- Temporary file names count per session, so a codeunit gets the same names on
  every run.

### 3. What is served

All for the in-memory store, outside the company build (which keeps its own
rules):

| Call | Answer | Pinned by |
|---|---|---|
| `File.Exists(path)` | Yes for a file the run wrote and did not delete; No for every other path | No after a delete: QxDotNetDemoShapes.al:328, :334. Yes: assumption `dotnet-file-exists-sees-run-files` |
| `File.Delete(path)` of an existing file | deletes it | :326-328 (through File Management.DeleteServerFile) |
| `Directory.Exists(path)` | Yes for a temporary folder and a folder AL created; No for a missing folder, a file, the empty text | :279-302 |
| `Directory.CreateDirectory(path)` | creates it and the folders above it; returns a value whose use is refused | :286-287, :313-314 |
| `Directory.Delete(path[, recursive])` | removes a folder AL created; recursively its files and folders | :293-294, :317-327 |
| `XmlDocument.Save(path)` | .NET saves into an AL stream; the bytes become the file | :370-377 (as in the company build) |
| `XmlDocument.Load(path)` | .NET loads from a stream over the file's bytes | .NET reads the same bytes from a file stream |
| `WorkbookWriter.Create(path)` (OpenXml) | the stream form of `Create` writes into an AL stream; the file exists from `Create`, a look at it is refused until the writer's `Close` or `Dispose`, which make the stream's bytes the file | Excel Buffer.CreateBook/CloseBook, then SaveToStream reads the file (added 2026-10-29, NP Retail lane) |
| `WorkbookReader.Open(path)` (OpenXml) | the stream form of `Open` reads a stream over the file's bytes | Excel Buffer.OpenBook |

A path that was never written does not exist (owner decision). That includes
paths outside the temporary folders: `File.Exists('C:\Windows\win.ini')` is No.

### 4. What is refused, and never sent to .NET

- A path that names a stored file or folder only when ASCII case or `/` against
  `\` are ignored (Windows would find it; whether BC does is not pinned).
- `File.Delete` of a missing file, `Directory.Delete` of a missing folder, of a
  temporary folder, or of a full folder without `recursive`,
  `Directory.CreateDirectory` over a file: .NET raises exceptions whose AL text no
  test pins.
- `XmlDocument.Save` and `WorkbookWriter.Create` into a folder that does not
  exist in the store, or a path with no folder; `XmlDocument.Load` and
  `WorkbookReader.Open` of a file that does not exist. `WorkbookWriter.Open(path)`
  (it reads and writes the file).
- A file that holds report output that was never produced (ADR-040) refuses
  `File.Exists`, `File.Delete` and `XmlDocument.Load`, as AL's `File` does.
- Every other member of `System.IO.File` and `System.IO.Directory`
  (`ReadAllText`, `WriteAllText`, `ReadAllBytes`, `Copy`, `Move`, `GetFiles`,
  `Open`, `Create`, and the rest), named in the refusal with "ADR-056".
- `FileInfo`, `DirectoryInfo` (except the company build's staged folders),
  `FileStream`, and `Path.GetTempPath`, `GetTempFileName`, `GetFullPath`,
  `GetRandomFileName`: still refused by the policy (ADR-014 section 8).
  `Path` members that only take text apart (`GetFileName`, `GetExtension`,
  `Combine`) read no disk and go to .NET as before.
- Outside the company build, a .NET member that opens a file by a path given as
  its first argument: the path constructors of `StreamReader`, `StreamWriter`,
  `XmlTextReader`, `XmlTextWriter`; `XmlReader.Create`, `XmlWriter.Create`;
  `XDocument` and `XElement` `Load` and `Save`; `XslCompiledTransform.Load` and
  `Transform` (`opens_file_by_path` in `dotnet/mod.rs`). Members outside this list
  that open a file internally are ADR-014 section 8 item 6.
- With `--files-dir`, the file system is a real directory and not in memory:
  every .NET file call above is refused, `XmlDocument.Save` and `Load` of a path
  included. Nothing reaches the disk through .NET.

### 5. Interplay with AL's own file and stream types

- AL `File` (`Create`, `Open`, `Exists`, `Erase`, `CreateTempFile`) and .NET read
  and write the same store (section 1).
- A report's `SaveAsXml(params, file)` and the other report `SaveAs*` calls with a
  file name write the store through AL's file capability, so .NET sees the file.
- `TempBlob`, `InStream`, `OutStream`, `UploadIntoStream` and
  `DownloadFromStream` hold bytes in memory and touch no file; this ADR does not
  change them. `UploadIntoStream` and `Download` still need a client and stay
  refused.

## Consequences

- The 21 NP Retail report tests that stopped at `File.Exists` run on; so do the
  report tests of Microsoft's apps that read their dataset through Library -
  Report Dataset. A pass through `File.Exists` Yes is a flagged pass until the
  probe in section "Open points" lands.
- `XmlDocument.Save(path)` no longer writes into the .NET helper's folder.
- A test that depends on a file left by an earlier codeunit or an earlier run
  fails or differs here while it passes on a reused BC server. That is the
  owner's intent; such a test is order dependent on real BC too.

## Conformance

- `crates/al-host/src/lib.rs`
  `the_memory_file_system_keeps_folders_and_removes_them_with_their_files`.
- `crates/al-vm/tests/dotnet.rs`, section "ADR-056": the corpus pins of
  QxDotNetDemoShapes.al on the in-memory store, the assumption marker, the
  refusals (near matches, unserved members, path constructors, `--files-dir`), and
  `XmlDocument.Save` and `Load` through a stream.
- Corpus onprem-dotnet `lt-qx-dotnet/QxDotNetDemoShapes.al` (`RD_*`) runs on the
  in-memory store.

## Open points for the owner

1. **Store per test codeunit.** The store is emptied between codeunits even under
   `TestIsolation = Disabled`, where real BC (and the runtime's database) keeps
   state across codeunits. Keep, or carry the store across codeunits under
   `Disabled` only?
2. **No rollback inside a codeunit.** Files written by one test are seen by the
   next test of the same codeunit, also under `Function` isolation. This follows
   BC (files are not transactional) and AL's `File` type as it was. Not pinned by
   a probe.
3. **Unwritten paths outside the temporary folders.** `File.Exists` and
   `Directory.Exists` answer No for any path the run did not write, also
   `C:\`, `/etc` or the server's own folders that exist on real BC. Owner
   decision, but a test that checks a real server folder now gets No instead of a
   refusal.
4. **More members.** `ReadAllText`, `WriteAllText`, `ReadAllBytes`,
   `WriteAllBytes`, `Copy`, `Move`, `GetFiles` of a folder AL created, `FileInfo`,
   and `FileStream` are refused. Each needs a probe for its encoding, its
   overwrite rule and its exceptions before it is served.
5. **`--files-dir`.** .NET file calls are refused when the user gives a files
   directory. Alternative: serve them from that directory too.
6. **Probe wanted** for the assumption `dotnet-file-exists-sees-run-files`: AL
   `File` writes a file under `TemporaryPath()`, then `DotNet File.Exists` of the
   same path; the same after `Report.SaveAsXml` into
   `FileManagement.ServerTempFileName('xml')`.
