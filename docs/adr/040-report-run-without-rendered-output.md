# ADR-040: A report run without rendered output

- Status: Accepted (owner, 2026-10-16, with the allow-list of reports)
- Date: 2026-10-16
- Related: ADR-009 (observable equivalence), ADR-013 (external objects), ADR-006 (core purity); CLAUDE.md hard rules 3, 4 and 6
- Research: `~/.cache/al-runtime/orchestration/report-rendering-options.md` (counts below come from it)

## Context

`Report.SaveAsPdf`, `SaveAsExcel`, `SaveAsWord`, `SaveAsHtml`, `TestRequestPage.SaveAsPdf` and `SaveAsExcel`, and
`Report.SaveAs` to a rendered format are refused today (`only the Xml dataset is produced`). There is no layout
renderer, and building one is not planned: RDLC needs Microsoft's ReportViewer, Word to PDF needs Aspose. The
owner decided on 2026-10-16 that renderers are not now.

### Measured (BC 27.5, 33 Microsoft buckets, 40,300 tests, 20,912 not run)

544 not-run tests are refused first at a rendering call. What they do with the output:

| Class | Tests | Needs rendered bytes? |
|---|---|---|
| The output is discarded; the report is run for its triggers (posting, check printing) | about 220 | no |
| "Renders without error" (Report Layout, Report Layout Test, Daily Report Tests, Job Queue `HasValue`) | about 80 | yes |
| File exists and is not empty (`CheckFileNotEmpty`) | 36 (+12 with a dataset check) | yes |
| Excel cell values at fixed addresses (`VerifyCellValueByRef('AJ', 49, ..)`) | 135 | yes, Microsoft's exact grid |
| Dataset check plus a render call | 26 | for the render step |
| `SaveAsHtml` of a custom layout | 12 | no, AL renders (separate change) |

The first class is the one case where nothing about the rendered output is observed.
`Library - Costing.PostInvtCostToGL` and `Library - Assembly.PostInvtCostToGL` call
`PostInventoryCosttoGL.SaveAsPdf(path)` because the report posts to G/L in its data item triggers; the test then
reads G/L entries. `Check.SaveAsPdf(ServerTempFileName)` in the bank tests is the same shape. About 190 of the
220 go through `Post Inventory Cost to G/L`, about 10 through `Check`; the rest is not yet attributed.

## Decision

### 1. What the change does

A rendering save of a report whose caller is on the **allow-list** (section 3) runs the report exactly as the Xml
path does, writes **no output**, marks the target as "render pending", and raises `Signal::Unsupported` the moment
AL observes that target. Nothing is faked: no placeholder bytes, no empty-but-valid file.

Calls covered:

- `Report.SaveAsPdf(Id, FileName)`, `SaveAsExcel`, `SaveAsWord`, `SaveAsHtml`, and the same on a report variable
  (`R.SaveAsPdf(FileName)`), with a file name.
- `Report.SaveAs(Id, Parameters, Format, OutStream[, RecordRef])` and `R.SaveAs(.., OutStream)` with Format
  Pdf, Word, Excel or Html, to a stream.
- `TestRequestPage.SaveAsPdf(FileName)` and `SaveAsExcel(FileName)` in a request page handler.

Not covered, still refused as today: `TestRequestPage.Preview`, `Report.Run`/`RunModal` with `SystemPrinter`,
`[ReportHandler]`, `Report.SaveAsXml(FileName)` (a dataset, separate task), a Pdf of a `ProcessingOnly` report
(pinned answer stays).

### 2. What runs

Same as the Xml path (`al-vm/src/reports.rs` `report_save_as`, `dataset.rs`): request parameters applied, data
items iterated with views and links, `OnInitReport`, `OnPreReport`, data item triggers, reportextension triggers,
`OnPostReport`, errors reaching the caller as on the Xml path. The dataset itself is built (to run triggers and
`CalcFields`) and then dropped. Triggers that only exist in a render (`OnPreRendering`, page-number and
`CurrReport.Preview` use) keep their existing refusals.

The call answers `true` for the Boolean form. This is an assumption: Windows BC answers true for a report with a
valid layout (`handlers/TestReportSaveAsPdfContent.al`, Windows arm). It is registered as one assumption id (section 5).

### 3. When it applies: an allow-list, not a rule

A render call without observation does **not** prove the test is indifferent to the render: the layout smoke
tests also observe nothing in AL. Passing them would be a vacuous pass. So the change applies only to reports
listed in an `ignored_output` registry in the runtime code (same place and checks as the assumption registry:
ids checked by the compiler and a test). Each entry names the report and cites the Microsoft test that needs
it and shows the output is discarded.

First entries (derived from the measured callers):

| Report | Why the output is not the point | Cited tests |
|---|---|---|
| `Post Inventory Cost to G/L` | Posts to G/L in its triggers; `LibraryCosting` and `LibraryAssembly` discard the file | SCM Kitting D3 Part 1/2, SCM WIP Costing Production I/II, SCM Costing Batch (via the two libraries) |
| `Check` | Prints and posts check ledger entries; the file name is a GUID or temp name nobody reads | ERM Cash Manager, Match Bank Rec. Scenarios |

A report not on the list keeps the current refusal. The list grows by evidence only: a test that discards the
output, cited in the code comment and in `docs/probes-needed.md` (rule 3 exception of 2026-10-15). Layout smoke
tests, `CheckFileNotEmpty` tests and Excel cell tests are never on the list.

### 4. The observation paths that raise `Signal::Unsupported`

A target is a **file name** (the set of pending paths in the session) or a **blob buffer** (a flag on `BlobBuf`,
an additive field). The flag follows the buffer on assignment and copy of the buffer (a `Blob` field copied by
record assignment, `CopyStream`, `TempBlob.CreateInStream` on the same buffer). It is cleared only when AL
re-creates the buffer (`CreateOutStream` that truncates, `Clear`, `Blob.CreateOutStream` after `Clear`) or
overwrites the path with a write it makes itself (`File.Create`, `Copy` onto it).

Every path below raises `Unsupported` with the reason `rendered output of report <name> is not produced`. This
is the complete list; a path not on it must be shown not to read the target before the ADR is accepted.

File name target (virtual file system, `al-builtins/src/file.rs`):

| # | Path | Why it is an observation |
|---|---|---|
| F1 | `File.Open`, `File.TextMode`, `File.CreateInStream`, any read from the file | reads the bytes |
| F2 | `File.Exists` | BC answers true after a render |
| F3 | `File.Erase`, `FileManagement.DeleteServerFile` | BC raises when the file is missing |
| F4 | `File.Copy`, `File.Rename`, `FileManagement.MoveFile`, `CopyServerFile`, `MoveAndRenameClientFile` | moves bytes |
| F5 | `File.Name`, `File.Len`, `FileManagement.GetFileSize`, `GetFileName` of a path that is pending, `ServerFileExists`, `ClientFileExists`, `GetDirectoryName`-style listing of the folder (`Directory.GetFiles`, `File` virtual table) | metadata or presence |
| F6 | `File.Download`, `DownloadFromStream`, `DownloadHandler`, `FileManagement.DownloadHandler`, `BLOBExport` | sends bytes to a client |
| F7 | `FileManagement.ServerTempFileName`/`ServerCreateTempSubDirectory`/`DeleteServerDirectory` on the folder containing a pending file, `ServerDirectoryExists` after the folder is removed | presence and removal |
| F8 | `FileManagement.GetFileContents`, `BLOBImport`, `BLOBImportFromServerFile`, `IsAllowedPath`-with-existence | reads |
| F9 | `Excel Buffer` `OpenBook`, `OpenBookStream`, `ReadSheet`, `LibraryReportValidation.OpenFile`/`OpenExcelFile`/`CheckIfValueExists`/`VerifyCellValue*` | opens the file |
| F10 | `CheckFileNotEmpty`, `LibraryUtility.CheckFileNotEmpty`, `FileMgt` size checks | reach F1 or F5 |
| F11 | `HttpContent.WriteFrom`/`Email` attach from a pending path (`Email.AddAttachment`, `Document Service` upload, `Report Inbox` insert from the file) | reads bytes |

Stream or blob target:

| # | Path | Why it is an observation |
|---|---|---|
| S1 | `InStream.Read`, `ReadText`, `EOS`, `Position`, `ResetPosition`, `Length`, `CopyStream` as source | reads |
| S2 | `Blob.HasValue`, `Blob.Length`, `TempBlob.Length`, `TempBlob.HasValue`, `Blob.CreateInStream`, `TempBlob.CreateInStream` | presence or size, then bytes |
| S3 | `Base64 Convert.ToBase64`, `ToBase64Url`, `Codeunit "Temp Blob"` helpers (`ToText`, `ToBase64String`, `FromRecord`), `Cryptography Management.GenerateHash` over the stream | reads |
| S4 | A record assignment, `Insert`, `Modify`, `TransferFields`, `CalcFields` that copies a pending Blob into a stored row; `Report Inbox."Report Output"`, `Job Queue Log`, `Document Attachment`, `Media`/`MediaSet` import | spreads the pending buffer. Allowed only if the stored buffer is flagged too and every later read is covered (the flag is stored with the row) |
| S5 | `File.CreateOutStream` + `CopyStream` from a pending stream, `HttpClient` body, `Download*` | reads |
| S6 | `XmlDocument.ReadFrom`, `JsonObject.ReadFrom`, `Evaluate` over the stream text | reads |
| S7 | `Report Inbox` / `Job Queue Entry` run through the Job Queue to PDF, Word, Excel (`Report Output Type`) | stores the stream |
| S8 | `Variant`/`Format`/`Message` of the stream object are not observations (the object, not its content) | listed so nobody adds them |

Not an observation, still runs: assigning the path to a variable, `Format` of the path, `StrSubstNo` with the
path, passing it to another procedure, `FileManagement.GetFileNameWithoutExtension`/`GetExtension` (pure string
functions), `ClearLastError`.

### 5. Flagged passes (rule 3)

The behaviour is not pinned by a probe. It rests on Microsoft's tests passing on real BC (the exception of
2026-10-15). So:

- One assumption id `report-render-discarded` in the runtime's assumption registry, marked each time the code runs.
- A test that runs through it is reported "passed, relies on an unprobed assumption".
- A row in `docs/probes-needed.md`, and a probe queued on the corpus: a report on the allow-list saved to a temp file and a stream, `File.Exists` and `TempBlob.Length` afterwards on a Windows tier, plus the answer value and the triggers' order. When the probe lands it wins.

### 6. Render-stage errors that stay

These errors come from the platform's render stage and are already pinned. They stay as implemented and run
**before** the no-output path:

- Pdf, Html, Word, Excel of a report with `DefaultLayout` and no layout file: `false` and
  `Report <id> does not have a valid layout. Valid layout types are: Word, RDLC, Excel, and Custom.`
  (`report/PRSaveAsFormats.al`, `al-language-onprem/session/PXSaveAsPdf.al`, BC 27.5).
- Html of a report with an RDLC layout: `false` and the pinned format text
  (`handlers/TestReportSaveAsDatasetContracts.al:SaveAs_Html_WithAnRdlcLayout_IsRefused`).
- Pdf of a `ProcessingOnly` report: the pinned `ReportProcessingOnly` answer or error
  (`handlers/TestReportSaveAsColumnTypesContracts.al:372-388`).

Errors a real render would raise from a broken layout cannot be seen without a renderer. That is the reason the
change is an allow-list (section 3): for a listed report the test's claim does not include "the layout renders".

### 7. What stays refused

- Layout smoke tests (Report Layout, Report Layout Test, Daily Report Tests, Test Report SaveAs): the claim is the render.
- `CheckFileNotEmpty` tests and any test that opens, sizes or copies the output.
- Excel cell tests (`LibraryReportValidation`).
- `Test Report SaveAs` (Job Queue to Report Inbox): the claim is bytes of the right type.
- Any report not on the allow-list.

## How the guard is proven complete

1. **One test per path** in section 4 (F1 to F11, S1 to S7), as Rust tests over the VM in `al-vm` using a small
   AL fixture workspace: save a listed report to a file or stream, then run the path, and assert
   `Signal::Unsupported` with the reason text. A path missing a test fails review.
2. **Three controls that must not raise**: a test per "not an observation" item (path as text, `Format`, string helpers).
3. **A catalog audit test**: every builtin in `al-builtins/src/catalog.rs` whose signature takes `InStream`,
   `OutStream`, `Blob`, `File`, `Text` path of a file builtin or a `TempBlob` is listed in a table in the test.
   A new builtin of that kind that is not in the table fails the test. This is what keeps the list complete as
   builtins are added.
4. **Flag propagation tests**: record copy, `CopyStream`, `Insert`/`Get` round trip of a pending Blob, TempBlob
   to Blob field, each raise on a later read.
5. **Dual check against Microsoft's buckets**: run the ranked buckets before and after. No test that passed
   before may change outcome. Every newly passing test appears in the flagged-pass list; any test that newly
   runs and fails is reviewed by hand, because it may have reached an observation the list missed.
6. **Static inventory** (done before acceptance, recorded in the ADR when Accepted): for each listed report, read
   every cited test's call graph to the end and list the observation calls found. None may be an unlisted path.

## Consequences

- About 190 (Post Inventory Cost to G/L) plus about 10 (Check) tests can reach their real assertions; whether they
  pass depends on other gaps. The measured number replaces these estimates after the first run. The 220 in the
  research is an upper bound for the class, not for the allow-list.
- Layout smoke (about 80), `CheckFileNotEmpty` (36), Excel cells (135) stay refused with the render reason. The
  refusal text should say "needs a layout renderer" so `--top-refusals` keeps ranking them.
- A rendered-output feature added later (sidecar ReportViewer, Aspose, Custom layout dispatch) replaces the
  no-output path for the reports it handles. The allow-list then shrinks to reports whose output is truly unread.
- Cost: a flag on `BlobBuf` and a pending-path set in `Session`. Both additive (no seam change). The check on
  every blob read is one branch on a field already loaded.
- Risk: one missed observation path is a silent fake. The catalog audit test (3) is the control; it must be in
  place before any report is listed.
- Custom-layout dispatch (`OnCustomDocumentMergerEx`, Pageworks) is a separate change and a separate ADR if it
  needs a seam. It does not depend on this one.

## Owner decision 2026-10-22: SaveAsPdf in the company build

While Microsoft's DemoTool builds the company (`Session::set_company_build`, set by al-testrun baseline.rs around the company
data codeunit only), `SaveAsPdf` to a file of any report takes the no-output path: the report runs (triggers and data), no file
is written, the path is marked like a listed report's, and `report-render-discarded` is marked. The DemoTool saves three PDFs
under TemporaryPath that nothing reads (MakeCorrections.Codeunit.al:62, PostRevaluation.Codeunit.al:82,
RunPostInventoryCosttoGL.Codeunit.al:17). Other formats, a stream target, and every SaveAsPdf outside the company build keep
the refusal. Test: `report_no_output_in_the_company_build_save_as_pdf_to_a_file_runs_the_report_and_writes_no_file`.

## Conformance

- `al-vm` tests named `report_no_output_*`: one per path F1 to F11 and S1 to S7, the controls, flag propagation.
- A catalog audit test in `al-builtins` (stream, blob and file builtins versus the table).
- The assumption registry test (ids checked by the compiler) for `report-render-discarded`.
- Bucket comparison: `--baseline-diff` before and after; flagged passes listed.
