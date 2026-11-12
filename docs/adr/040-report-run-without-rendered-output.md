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
valid layout (`reports/TestReportSaveAsPdfContent.al`, Windows arm). It is registered as one assumption id (section 5).

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
  (`reports/PRSaveAsFormats.al`, `al-language-onprem/session-system/PXSaveAsPdf.al`, BC 27.5).
- Html of a report with an RDLC layout: `false` and the pinned format text
  (`text/TestReportSaveAsDatasetContracts.al:SaveAs_Html_WithAnRdlcLayout_IsRefused`).
- Pdf of a `ProcessingOnly` report: the pinned `ReportProcessingOnly` answer or error
  (`reports/layouts-saveas/TestReportSaveAsColumnTypesContracts.al:372-388`).

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

## Amendment 2026-11-08: a placeholder PDF (Accepted, owner 2026-11-08)

### Decision

`Report.SaveAs(.., ReportFormat::Pdf, ..)`, `Report.SaveAsPdf` (by number and on a variable) and `TestRequestPage.SaveAsPdf`
in a request page handler succeed for a report whose default layout is RDLC or Word, and write a small valid placeholder
PDF: a `%PDF-1.4` header, a catalog, a page tree and one blank page (`al-vm/src/report_pdf_placeholder.rs`). This
applies to every such report, not only to the allow-list of section 3. There is no rendering parity and no layout work.

What runs is the Xml path (section 2): request parameters, data items, triggers, reportextension triggers, and errors
reaching the caller as there. Only the output bytes are a placeholder. The render-stage errors of section 6 stay and run
first. The Pdf of a processing-only report goes on to its pinned answer (false, `ReportProcessingOnly`).

### What it means

- The PDF content does not match BC. Only "it exists, it is a PDF, its length is > 0" is meant to match. Anything a
  test reads beyond that (an exact length, a page count, text in the PDF) is unpinned. A corpus case that asserts more
  than that stays refused or is listed as not passing; it is never counted as evidence for this path.
- The default layout type decides: `rdlc` or `word` (`ir::Report::default_layout_kind`: the `Type` of the
  `DefaultRenderingLayout`, else `DefaultLayout`, else `rdlc` when the only layout declared is RDLC). An Excel or
  Custom default, or a report whose layout cannot be told, keeps the refusal (or the custom-layout path).
- BC raises the layout events of `Reporting Triggers` around a render (SubstituteReport, SelectReportLayoutCode,
  HasCustomLayout, SelectedBuiltinLayoutType, ApplicationReportMergeStrategy, OnDocumentReady, and others). The
  placeholder raises none of them. A save is refused while a subscriber the test can see listens to one of them, or to
  the integration events Base Application's ReportManagement raises from them (OnAfterSubstituteReport and others).
  "Can see" is the rule of `Registry::unraised_subscriber` (decided without owner review 2026-10-19 for
  CaptionClassTranslate): the test names the subscriber's codeunit or a SingleInstance codeunit the subscriber holds.
  Base Application's ReportManagement subscribers do not log: they pick the layout from data. SelectedBuiltinLayoutType
  and SelectReportLayoutCode read table 9651 "Report Layout Selection", "Custom Report Layout" and the SingleInstance
  codeunit 9654 "Design-time Report Selection"; the platform reads Tenant Report Layout Selection. Such a selection can
  pick an Excel layout, and a Pdf of an Excel layout writes no output on BC (AMReportOutputTests.al:257). So a save is
  also refused while a Report Layout Selection row for the report and company selects anything but its built-in RDLC or
  Word layout, while "Design-time Report Selection" holds a selected layout, or while a Tenant Report Layout Selection row
  for the report names a layout that is not an RDLC or Word layout of the report. The other ReportManagement subscribers
  on the layout events (HasCustomLayout, SubstituteReport, OnIntermediateDocumentReady, OnDocumentReady) only forward to
  integration events, covered above; FetchReportLayoutByCode and GetCompanyMetadata only read data for the rendered bytes.
- One assumption id `report-pdf-placeholder`, marked each time the path writes; a test through it is a flagged pass.
  A probe on a Windows tier (RDLC and Word report to a stream and a file, a subscriber logging the layout events) is
  queued; when it lands it wins.
- For a report on the allow-list, a Pdf save with an RDLC or Word layout now writes the placeholder instead of
  marking the target as pending. Excel, Word and Html saves of a listed report keep the no-output path, and so does a
  SaveAsPdf to a file in the company build.

### Evidence

About 215 Microsoft test methods (BC 29) reach a PDF save, most through `Library - Post Inventory to G/L`.PostInvtCostToGL
or through request page handlers. None reads the PDF bytes; 2 need a non-empty file and 2 need the file to exist.

## Amendment 2026-11-11: placeholder Word, Excel and Html outputs, and a run without output (Accepted, owner 2026-11-11)

### Decision

The 2026-11-08 placeholder approach extends to the other formats and to `Report.Run`/`RunModal`. What runs is always the
Xml path (section 2); only the bytes are made up. The format x default layout grid follows what BC does
(reports/layouts-saveas/AMReportOutputTests.al, AMRdlcOutput.al):

| Format (SaveAs, SaveAsWord/Excel/Html, by number and on a variable) | RDLC layout | Word layout | Excel layout |
|---|---|---|---|
| Pdf | placeholder (2026-11-08) | placeholder (2026-11-08) | answers false, no error, no output (AMReportOutputTests.al:257) |
| Word | refused: fails in the Linux renderer, Windows not measured | placeholder: minimal valid .docx (:187, zip with `word/document.xml`) | refused: not measured |
| Excel | refused: as Word | answers false, no error, no output (:221) | placeholder: minimal valid .xlsx (:235, zip with one sheet) |
| Html | `ReportUnsupportedFormat` (pinned, built) | placeholder: minimal valid html (:205, markup) | refused: not measured |

`Report.Run`/`RunModal` (static and variable) and `Execute` of a report with an RDLC, Word or Excel default layout, in a
test session: the report runs as on the Xml path and writes no output; the call answers as BC does (Yes, PeVTests.al:740-763,
Windows values). BC raises SubstituteReport, the layout events, OnDocumentReady and OnDocumentDownload there; this path raises
none of them yet (a later change raises them in the pinned order).

### What stays refused

- Reading the content. The placeholder buffer and file carry the flag of section 4. Presence and size answer from the
  placeholder (F2, F5, S2: it exists, length > 0) and byte copies are allowed (CopyStream to another blob, a file, an e-mail
  attachment, a stored Blob; the flag goes along). Every path that parses the bytes raises `Unsupported`: S1 `Read`/`ReadText`
  into a Text, S3 hashes and Base64, S6 `XmlDocument.ReadFrom`, F9 Excel Buffer and `LibraryReportValidation`, Data Compression
  opening the zip. So an Excel cell check, a text search in the Html, an exact length or a page count stay refused, never wrong.
- Layout-event subscribers the test can see, and Report Layout Selection, Design-time Report Selection and Tenant Report Layout
  Selection rows that pick another layout: the same refusal as the 2026-11-08 amendment, for every format and for Run/RunModal.
- A Custom default layout, a layout that cannot be told, `TestRequestPage.Preview`, printing with `SystemPrinter`.

### What it unlocks

- Microsoft tests (ADR-040 table, BC 27.5): the Word/Excel/Html share of "file exists and is not empty" (36 + 12, split by
  format not counted) and of "output discarded" (about 220, mostly Pdf, already open). Excel cell tests (135) stay refused.
- Corpus: 12 cases refused with "renders output ... defaultrenderinglayout" (LtTkShowPrintStatus.al 9, CiReportRunLeads.al 2,
  W3ReportObjectIdPreviewLanguage.al 1), and the Word layout rows of AMReportOutputTests.al that assert only ok, saved and kind.
- Outside reporter (issue 6924): 49 tests stop at `Report.SaveAs(.., Html)` of e-mail body reports. Those that copy the Html
  into an attachment or check only that it saved can run; those that read the Html into the mail body text stay refused
  (S1). How many of the 49 read the text is not known; the reporter's `--explain` after the change will tell. Also the
  "renders output" refusals of Run/RunModal there (5 tests).

### Assumptions and probes

- Ids: `report-word-placeholder`, `report-excel-placeholder`, `report-html-placeholder`, `report-run-without-output`, each
  marked when its path writes or runs; a test through one is a flagged pass. Content beyond kind and length > 0 is never pinned.
- Probe to queue (Windows nightly, since the Linux tier does not render): RDLC layout to Word and Excel; Excel layout to Word
  and Html; Word layout to Excel again; `SaveAsWord`/`SaveAsExcel`/`SaveAsHtml(FileName)` then `File.Exists` and size;
  Run/RunModal/Execute of a Word and an Excel layout report with a logging subscriber. When it lands it wins.

### Implementation notes (2026-11-12)

- Code: `al-vm/src/report_placeholder/` (`grid.rs` the table above, `files.rs` the bytes, `run.rs` Run/RunModal/Execute) and the
  content mark `al-builtins/src/placeholder_output.rs`. Tests: `al-vm/tests/reporting/report_placeholder_outputs.rs`.
- Two cells the table marks "refused: not measured" are pinned by reports/CiReportRunLeads.al (BC 29.0 tier): an Excel layout
  to Word (:267-270) and to Html (:662-666) answer false with `ReportUnsupportedFormat`, as Pdf does (:657-661). They are built
  that way. RDLC to Word and Excel is pinned too: true, length > 0 (CiReportRunLeads.al:674-692 SaveAs_RdlcOnly_WordExcel, and
  AMRdlcOutput.al:336-373 on Windows), so the table's "refused" cells for it are corrected: an RDLC layout writes the Word and
  Excel placeholders.
- `Report.Run`/`RunModal`/`Execute` of a report with an Excel default layout do not run on BC: they raise "The current report
  operation is not supported for Excel Layout reports." (`ReportUnsupportedOperation`) before OnPreReport
  (CiReportRunLeads.al:236-260). That pinned error is built (for a report without OnInitReport, request page or reportextension;
  with one of them it stays refused). RDLC and Word layouts run without output as decided.
- The content mark applies to every placeholder, and stays when AL code opens a new OutStream on the Blob (the stream writes
  over the first bytes and the rest stays).
- The PDF placeholder is marked too (owner decision 2026-11-12). A read answers only while it stays inside the bytes the
  corpus pins at the start of BC's PDF, the signature `%PDF-` (`placeholder_output::PDF_PREFIX`): `ReadText(Text, 5)` of a
  Windows stream gives '%PDF-' (AMReportOutputTests.al:42-45, AMRdlcOutput.al:177-180, onprem AMRdlcSaveAs.al:199-202), and
  four Byte reads, each after EOS answered No, give 37,80,68,70 (streams/CjFix.al:128-147 for CjWordSections.al
  SectionBreaks_WordMergeDataItem_Pdf). So `EOS` before byte 5, `Read` into a Byte (or a Char on a single-byte encoding) of
  bytes 0-4, and `ReadText(Text, Length)` on a single-byte encoding that ends by byte 5 answer, and only while the buffer
  starts with those bytes (not a copy taken part way, not bytes AL wrote over them). Everything else is refused: a read past
  byte 5, a line read, a UTF-8 or UTF-16 text read (the tier may decode ahead), `InStream.Length`, the exact size (the size
  answers only against 0, as for the other placeholders), page counts, Base64 and hashes. The version after the signature is
  not BC's ('%PDF-1.3' in ReportEventsMoreCallForms.al:93, the placeholder's '1.4').
- Five corpus cases had passed on the readable placeholder's made-up bytes and are listed as not passing (owner OK
  2026-11-12): TestReportSaveAsPdfContent.al Report_SaveAs_Pdf_ReturnValueAgreesWithTheBytesInTheStream (Base64 of the whole
  blob, the exact length), ReportExtensionSaveAs.al ControlReportWithoutExtension_Xml_Pdf and BaseWithOwnLayout_Xml_Pdf
  (`Read(Head, 20)`, the exact length), and onprem AMRdlcSaveAs.al Static_SaveAsPdf (the page count) and
  Static_SaveAsHtml_IsRefused.
- The size of a marked placeholder (`Blob.Length`, `File.Len`, and through them System Application's `Temp Blob.Length()`) is the
  value `Value::PlaceholderSize`: it answers a comparison with 0 (greater) and refuses everything else (a number, `Format`,
  arithmetic, another comparison, a record field, and any builtin or .NET member given it: a record filter, `Get`, `Validate`,
  a List or Dictionary element, key or index, `placeholder_output::size_guard`; only `Variant.Is*` asks its type). This is the correction the 2026-11-12 amendment (A4) names. It is a value and
  not only the syntactic form `Blob.Length() > 0`, so that `Temp Blob.Length() > 0`, which returns the Blob's length through
  two procedures, answers too (AMReportOutputTests.al:146, CiReportRunLeads.al:72).
- A run that would print (SystemPrinter `true`, or a report that declares `UseSystemPrinter = true`) is refused, as "What stays
  refused" says. An Excel layout run checks the layout event subscribers and the layout selections before it raises its pinned
  error, as the RDLC and Word runs do.
- The false answers raise SubstituteReport (static form, as the RDLC to Html answer) and not OnDocumentReady (AMReportOutputTests.al:221,
  :257); a subscriber the test can see to another layout event refuses them.

## Amendment 2026-11-12: opaque rendered content in Text values (Accepted, owner 2026-11-12)

Status: Accepted by the owner on 2026-11-12, with these answers to the open questions: Part A as written; A7 (run Microsoft's layout
selection code, a temporary selection of a Word layout no longer refused) yes; the one exact `Regex.Replace` entry yes; Part B is
built only after the Windows probe is merged; `Contains` only (no `StrPos`/`IndexOf`); `Message` with a marked text stays refused.
Part A is built (al-builtins `rendered_text.rs`; A7 in al-vm `report_placeholder/selection.rs`, for a user-defined Word layout selected
by name without an app id); Part B waits for the probe.
Part A is the opaque value. Part B ("known static text of the chosen layout") is part of the amendment by owner decision
of 2026-11-12; Part A is written so that it works without Part B.

### The problem

The 2026-11-11 amendment writes a placeholder Html file and refuses every read of it into a Text (S1). An outside
reporter counted his suite against that rule (al-runner issue 6924, comments of 2026-11-12 06:49, 07:03 and 07:08 UTC):

- 229 tests save a Word layout e-mail body report as Html. None of them calls `SaveAs` itself. The path is Microsoft's:
  `Report Selections`.`SaveReportAsHTML` -> `Report.SaveAs(.., ReportFormat::Html, OutStream)` into a Temp Blob ->
  `Email Item`.`SetBody` (CopyStream) -> `Mail Management`.`SendViaEmailModule` -> `Email Item`.`GetBodyText()`
  (`InStream.Read(Text)`) -> `Email Message`.`Create(.., Body, true, ..)`.
- A subscriber of the reporter then runs `BodyText.AddText(Message.GetBody())`, appends an Html signature and calls
  `Message.Create(.., Format(BodyText), true)`. Tests with the dialog read `EmailEditor.BodyField.Value()` in a ModalPageHandler.
- 184 tests assert nothing about the body. 4 assert `AreNotEqual('', Body)`. 41 assert `Body.Contains(<language marker>)`;
  the marker is static text of the Word layout.

Under S1 all 229 stop at `GetBodyText()`. The goal: the 184 and the 4 run, the 41 are reported as not run. No test may
fail or pass because of text the runtime made up.

### Decision in short

1. A Text can hold **rendered content**: text that BC produced with a renderer and whose characters this runtime does not
   know. It is known to be not empty. Nothing else is known about it.
2. Rendered content is marked inside the text with one reserved character, `RENDERED_TEXT_MARK` (U+FDD4), the fourth
   mark of the existing kind (`CALL_STACK_MARK`, `CARRIED_TEXT_MARK`, `SQL_COUNT_TEXT_MARK`; `Txt::has_marker`). No new
   `Value` variant and no new field on `Value` or `Txt`.
3. A marked text may be moved: assigned, passed, returned, stored in an unbounded Text, a Variant, a List or Dictionary
   value, a TextBuilder or BigText, joined with known text, written to a UTF-8 stream and read back whole. Each of these
   gives a marked value again.
4. A marked text may be asked one question: is it equal to `''`. The answer is no.
5. Every other use refuses with `Signal::Unsupported`. The rule is an allow-list: a use that is not listed in the table of A3
   refuses. A new allowed use is added only as an exact match of a pinned or reported shape.
6. The mark starts at exactly one place: a whole text read of a Html placeholder buffer (section A2).
7. Part B: `Contains(X)` answers true when `X` is fixed text of the Word layout the save used, under a narrow rule. It
   never answers false.

This changes no seam. `Txt`, `Value`, `Ctx` and the builtin catalog keep their shapes; the change adds a mark, a guard at
the builtin call and rows in one table.

## Part A: the opaque value

### A1. Where the mark lives, and why there

**Decision: in the text itself, as U+FDD4, owned by a new module `al-builtins/src/rendered_text.rs`.** The module owns the
mark, the refusal text, the allow-list of section A3 and the helpers the other owners call. The blob side stays with
`al-builtins/src/placeholder_output.rs` (2026-11-11 amendment).

Reasons:

- The mechanism exists and has one owner per rule. `native::text_of` already refuses a marked text for every builtin that
  takes a Text argument through it. `ops::compare` already answers a marked text against `''` and refuses the rest.
  `Txt::push_txt` and `+` keep the characters. One fix is needed first: `Txt::push_str` (`text.rs`) updates the cached
  marker answer only for `CALL_STACK_MARK`; it must do so for every mark. The stream code already writes and reads a mark
  (`CALL_STACK_MARK_BYTES`), the row store already refuses a marked key, the filter code already refuses a marked filter value.
- The mark survives paths that a side flag would lose without notice: a `String` round trip inside a native, a `format!`,
  the wire to the .NET sidecar, a stored row.
- Cost for ordinary text: none on the value. `Txt::has_marker` scans a text once and caches the answer; the scan looks for
  four characters where it looks for three today. The append test that counts scans (`marker_scans`) stays as it is.

Alternatives considered and not chosen:

| Alternative | Why not |
|---|---|
| A new `Value::RenderedText` variant | Every `match` on `Value::Text` would need a decision, about 1,000 places. A place that forgets the variant fails closed, which is good, but joined text ("signature + body") needs a second representation for the known parts, and TextBuilder, BigText, List and JSON hold `Txt`, not `Value`. |
| A flag on `Txt` beside the string | Lost by every `into_string`, `to_string` and `format!`. Each loss is a silent wrong result. |
| Keep the placeholder Html as real text and flag the test | The test would see made-up characters: `Contains('NLB')` answers false and the test fails wrongly. |

Shape of a marked text: known characters and marks in the order BC has them. `Signature + Body` is the signature's
characters followed by the mark. The mark is followed by one character that numbers the render (read only directly after
the mark). The number is unique in the process, never per session: a SingleInstance global or a stored row can carry a
marked text from one test session into the next, and a number that started again at 0 would then name another render.
The number points to an entry of a process-wide list: the report, the format and the layout. An entry is never reused.

The mark has two forms (review finding 3):

- **whole** (U+F0000 + n): the complete rendered content of render n stands at this place, unchanged.
- **changed** (U+100000 + n): content derived from render n by an allowed step that can change characters (only P12 today).

Part A treats both forms alike. Part B answers only for the whole form. Every row of A3 must say which form its result
has; a row that does not embed the whole content unchanged gives the changed form or is not allowed. Marks from two
renders can stand in one text.

### A2. Where a marked text starts

Only here:

| Source | Result |
|---|---|
| `InStream.Read(Text)` without a length, at position 0 of a buffer that holds the **Html** placeholder (mark of `placeholder_output`, format Html) | A text that is the mark alone. The placeholder's own markup never reaches AL. |
| `BigText.Read(InStream)` on such a buffer | The BigText holds the mark alone. |
| The same two reads on a buffer that a marked text was written to (A4) | The text that was written, marks included. |

Every other read of such a buffer stays refused as S1 says: `Read` with a length, `Read` when its return value is used
(the number of characters is not known), `ReadText` (it reads one line, and the line breaks of the rendering are not known),
`EOS`, `Position`, `Length`, a read that starts after position 0. Word, Excel and Pdf placeholders never give a marked
text: they are binary, and a text read of them stays refused.

Why "not empty" is safe to assume: BC's Html of a Word layout has output and starts with markup
(reports/layouts-saveas/AMReportOutputTests.al:205-217, `has_output` Yes, `kind` markup). The Html placeholder is written
only for a Word layout.

That case is a report with data that runs to its end. Nothing pins the output of a report that ends with
`CurrReport.Quit` or whose data items read no row. So a read of A2 refuses when the report run that wrote the placeholder
ended by `CurrReport.Quit` or produced no dataset row, until probe group P (B5) measures both.

### A3. What is allowed

Every row gives a marked result or moves the value unchanged, unless the row says otherwise. "BC" says why the result is
right on BC whatever the rendering holds.

| # | Use | Result | BC |
|---|---|---|---|
| P1 | Assignment to a `Text` without a length, a `Variant`, a `var` or by-value parameter, a return value, an event argument | the same text | A copy of a text is the text. |
| P2 | `A + B`, `+=`, `TextBuilder.Append`, `AppendLine`, `BigText.AddText` (at the end) | the parts in order, marks kept | Joining does not look at the characters. |
| P3 | `TextBuilder.ToText()`, `Format(TextBuilder)`, `Format(Text)` and `Format(Variant holding it)` with no length and no format number, `BigText.GetSubText(T, 1)` without a length and with its return value unused (the return value is a length) | the same text | Each is the identity on a Text (the corpus pins this for known text; the citing line goes into the code comment). |
| P4 | `StrSubstNo(Known, .., T, ..)` with the marked text as a value, never as the template, and only where the template uses that value as `%n`. A `#n` field pads or cuts to a width and shows `*` on overflow, which depends on the length: refused. | the filled template with the mark | The value is put in through `Format` (format-evaluate/P86StrSubstNo.al Sub_Percent), as for a SQL counter text (al-runner issue 7129). |
| P5 | `T = ''`, `T <> ''`, and `''` on the left | false, true | Rendered content is not empty (A2); known text joined to it cannot make it empty. Already the rule for a call stack text (`ops::compare`). |
| P6 | `T = Known` or `T <> Known` (these two operators only) where the known characters before the first mark of `T` already differ from `Known` | the exact answer, always "not equal" | The known characters are BC's, and text equality on BC is ordinal and case-sensitive (the `Operators_TextComparison_IsCultureOrderAndCaseSensitiveEquality` case). Existing rule (`ops::known_prefix_order`), used for equality only. |
| P7 | `OutStream.Write(T)`, `WriteText(T)`, `BigText.Write(OutStream)` on a UTF-8 stream, with the return value unused (it is a count that depends on the length) | bytes of the known parts and the mark; the buffer is marked (A4) | Writing does not look at the characters. |
| P8 | `List.Add`/`Set`/`Insert`/`Get`, `Dictionary.Add`/`Set`/`Get` with the text as a **value** | stored and returned unchanged | As for a SQL counter value (`sqlcount::guard`). |
| P9 | `Variant.IsText`, `IsCode` and the other `Is*` | the pinned answer for a Text | The type is known. |
| P10 | `Session.LogMessage` with the text as message or dimension value | nothing AL can read | Telemetry is not readable by the test (SqlCounters.al:516 P6924_Counters_FormattedInTelemetryDimension_NoError). |
| P11 | A page field whose source is a Text variable or an unbounded expression, read by `TestField.Value()` when the control shows a Text unchanged (no `OptionCaption`, no format property, not `ExtendedDatatype = RichContent` until measured) | the same text | The control shows the text as it is. |
| P12 | `DotNet` `Regex.Replace(T, Pattern, Replacement)`, static, only with the exact pattern and replacement of `Email Message Impl.`.`ReplaceRgbaColorsWithRgb` (System Application) | the sidecar's result on the known parts; every mark in the result gets the **changed** form | `Email Message`.`Create` calls it on every body. The replacement text is never empty, so a not-empty text stays not empty. See A5. |

`Email Message`.`Create`, `GetBody`, `Email Item`.`SetBody`, `GetBodyText` and the editor page are AL compiled from
Microsoft's source. They need no rule of their own: they use P1, P2, P5, P7, P12, A2 and the Blob rules of A4.

Two gaps against the goal. The owner decided both on 2026-11-12:

- `Format(BigText)` (no format string) is added to P3 by owner decision: on a BigText that holds a mark it returns the
  text with the mark in the **changed** form (A1), so Part A passes it on and Part B no longer answers for it. The reporter's
  subscriber uses `BodyText.AddText(..)` and `Format(BodyText)`. Probe cell queued: `Format` of a BigText that holds a long
  text (does BC return it whole?); when it lands it wins.
- BC 27 to 29 compile `DoSaveReportAsHTMLInTempBlob` with its `#if not CLEAN27` block: after the save it runs
  `FileManagement.BLOBExportToServerFile`, raises `OnReplaceHTMLText` with the file path and runs
  `BLOBImportFromServerFile`. These are byte copies to and from a file. They run only as far as the file rules of the
  2026-11-11 amendment allow (F4, F8 as byte copies with the mark); any step those rules do not cover refuses.

### A3b. What refuses

Everything that is not in the table above. The reason is
`a use of the rendered content of report <name> (<format>, layout <layout>): <what>; the runtime has no layout renderer, so
the text is not known`. The most likely uses, so that nobody adds them by mistake:

| Use | Why it cannot be answered |
|---|---|
| `=`, `<>` against a text that is not `''` (outside P6), `case`, `in` | depends on the characters |
| `<`, `>`, `<=`, `>=`, a range in `in [a..z]` or in a `case` label, always, also against `''` and also when the known characters before the mark differ | BC orders text by culture, not by code point (`Operators_TextComparison_IsCultureOrderAndCaseSensitiveEquality`): `('Body ' + Html) < 'body'` is false on BC and an ordinal compare says true. See A5 for the code path that answers this wrongly today. |
| `Contains`, `StrPos`, `IndexOf`, `LastIndexOf`, `StartsWith`, `EndsWith`, `IndexOfAny` | depends on the characters. Also on a known part: kept refused in Part A, one rule for all |
| `StrLen`, `Text.Length`, `MaxStrLen` of the value's content, `TextBuilder.Length`, `BigText.Length` | the length is not known |
| `CopyStr`, `Substring`, `DelStr`, `Remove`, `PadStr`, `InsStr`, `Insert`, `T[i]` read or write, `foreach` over the text, `Split`, `SelectStr` | a part of unknown text, and a part can lose the mark |
| `UpperCase`, `LowerCase`, `ToUpper`, `ToLower`, `Trim*`, `DelChr`, `ConvertStr`, `Replace`, `IncStr` | Could be allowed as "unknown stays unknown", but `Trim`, `DelChr` and `Replace` can give `''`, which breaks P5. All refused until a reported test needs one; then it is added as its own row with its own proof that the result is not empty. |
| Assignment or parameter passing into `Text[N]`, `Code[N]`, a Text or Code **field** of a record, a Label | BC raises when the text is longer than N. The length is not known, so neither "raises" nor "fits" is known. `Code` also changes case. |
| `Evaluate`, an implicit conversion to a number, date, Guid, Boolean, Option | parses the characters |
| Hashes (`Cryptography Management`, `GenerateHash`), `Base64 Convert`, `Compress`, encryption, `SecretText` wrapping | computes from the characters |
| A Dictionary **key**, a List searched by value (`Contains`, `IndexOf`, `Remove(Value)`), a sort | compares the characters |
| `SetRange`, `SetFilter`, `Get`, `Validate` with the value; a `TestField.SetValue`, `TestFilter.SetFilter` | compares or stores into bounded fields |
| `Error`, `Message`, `Confirm`, `StrMenu`, `Notification`, `ErrorInfo` texts, `Dialog.Update`, `CurrPage.Caption` | the text reaches the test (`GetLastErrorText`, a handler's argument) or the user. Refused in step 1; a handler that only receives the text could be opened later as its own row. |
| `JsonObject.Add`, `JsonValue.SetValue`, `JsonObject.ReadFrom(T)`, `XmlText.Create`, `XmlDocument.ReadFrom(T)`, `XmlElement.InnerText` | Writing would be safe only if every later read of the document were guarded; reading parses. Refused in step 1. |
| `HttpContent.WriteFrom(T)`, `HttpClient` bodies and headers | the mocked or real receiver sees bytes the runtime made up |
| `File.Write`, `WriteText` to a file opened in text mode, `Download*`, `Hyperlink` | the bytes leave the session, or AL reads them back in pieces |
| A stream write in an encoding other than UTF-8 | The known parts would be encoded with loss, and no mark form is defined there. Refused in step 1. |
| Any `DotNet` call with a marked argument, outside P12 | the sidecar would compute on the mark as if it were text |
| `InStream.ReadText`, `Read` with a length, `EOS`, a second read | see A2 |

### A4. Blobs, Temp Blob and stored rows

- A buffer that a marked text is written to gets the placeholder mark of the 2026-11-11 amendment with a new kind "text
  with rendered content". All its rules apply unchanged: `HasValue` true, `Length` > 0 only as a sign, `CreateInStream`
  allowed, `CopyStream` copies bytes and mark, a record assignment and a stored Blob keep the mark with the row (S4), hashes,
  Base64, Xml and zip readers refuse.
- `Email Message`.`Body` is a Blob field. `Modify` stores bytes and mark; `CalcFields` returns both. The mark of a stored
  Blob is already stored with the row (S4 of section 4).
- A whole text read gives the written text back (A2). The known parts follow the pinned stream rules for known text
  (terminator and encoding: streams/ProbeJsonAndInStreamBytes.al); the mark's bytes are its UTF-8 form plus the render number.
- The byte length is never answered as a number (review finding 2). `Blob.Length`, `TempBlob.Length`, `File.Len` and
  `FileManagement.GetFileSize` of a buffer or file with a placeholder mark answer only in the sign-only form the runtime
  already has for a call stack text (`stream::blob_length_sign`: the call compared directly with 0 by `> 0`, `<> 0` or
  `= 0`). Every other use of the length refuses. `TempBlob.Length() > 1000` is true on BC and would be false on a
  93-byte placeholder.
- This holds for the buffers of this amendment and it is a correction to the 2026-11-11 build for all placeholder kinds:
  on branch `adr040-outputs`, `blob_length_of` and `File.Len` return the placeholder's byte count today. That is fixed
  there before this amendment is built on it.

### A5. How the allow-list is enforced

One rule, one owner each. A marked text meets the runtime at five kinds of places:

| Place | Owner | Default |
|---|---|---|
| A builtin call (function, method, operator builtin) | New guard `rendered_text::guard(id, receiver, args)`, called where `sqlcount::guard` is called (`al-vm/src/exec/run.rs`, both call sites). It runs while a process-wide flag is set. The first read of A2 sets the flag and nothing clears it for the rest of the process, because a marked text can outlive the session that read it (a SingleInstance global, a stored row, the strict order mode). It looks at the receiver and each argument, into a Variant, a TextBuilder and a BigText. A marked value at a position that table A3 does not list for this builtin id refuses. | refuse |
| A native that reads a Text argument | `native::text_of` (exists): `None` for a marked text, so the native refuses. Natives of table A3 take the text through one named helper, `rendered_text::carried`. | refuse |
| Comparison | `ops::compare` for `=` and `<>`: P5 and P6 answer, the rest refuses with the rendered refusal text. `ops::compare_order` for `<`, `>`, `<=`, `>=` and ranges: refuses a marked operand before anything else. Today it does not: `text_of` gives `None` for a marked text and `compare_order` then falls to `compare` and `known_prefix_order`, an ordinal answer where BC uses culture order. The same wrong answer exists today for `CALL_STACK_MARK`. Fixing `compare_order` for every mark is a separate fix with its own test and comes before this amendment is built. | refuse |
| Conversion to a declared type | `ops::convert`: a marked text converts only to an unbounded Text and to a Variant. Today a marked text passes into any `Text[N]` unchecked (`ops.rs:68`, `:240`); for U+FDD4 that arm refuses. | refuse |
| The .NET sidecar | The marshalling of a `DotNet` call's arguments: a marked text refuses unless (type, member, the known arguments) is an entry of `rendered_text::DOTNET_ALLOWED`. One entry: P12. | refuse |

Cost: a run that never reads a Html placeholder into a Text pays one Boolean test per builtin call, the same shape as
`self.sql.active`. After the first such read every builtin call of the process looks at its arguments. No cost on text
operations.

### A6. What happens at the edges

- **An assert in the test.** `Library Assert` is AL. `AreNotEqual('', Body)` runs P1, P3 and P5 and passes, as on BC.
  `AreEqual`, `IsTrue(Body.Contains(..))`, `ExpectedMessage` on the body reach a refused comparison or text function, so the
  test is reported as not run. An assert that fails on another value and puts the body into its message reaches `Error`
  with a marked text: refused, not a failure.
- **An output parameter or return value back in the test.** Allowed (P1). The test can hold it, pass it on and ask P5.
- **Error or Message text.** Refused (table). A test never fails with a made-up text in its message.
- **JSON, XML, HttpContent, a file.** Refused in step 1 (table). Each can be opened later with its own guard and proof.
- **The runner's own output.** A refusal reason, a failure text, `--explain`, the debugger's variables and the profiler
  print a marked text through `Describe`, which prints `<rendered content of report X>` for the mark, never the mark
  character and never placeholder markup.
- **State that outlives the test.** A SingleInstance global or a stored Blob keeps the mark, because the mark is in the
  text and in the row.

### A7. Needed first: the layout that `SaveReportAsHTML` selects

Without this, none of the 229 tests reaches the placeholder. `Report Selections`.`SaveReportAsHTMLImpl` always calls
`Report Layout Selection`.`SetTempLayoutSelectedName(LayoutName, AppID)` (or `SetTempLayoutSelected(Code)`) before the
save. That fills the SingleInstance codeunit "Design-time Report Selection". The 2026-11-08 amendment refuses a save while
that codeunit holds a selection. The reporter's layout is a user-defined Word layout (a `Tenant Report Layout` row
imported from a .docx), named by a GUID.

Decision: a save is not refused for a selection alone. The runtime resolves the selected layout and uses its **type** in
the format x layout grid of the 2026-11-11 amendment:

- The selection is read by raising the platform's layout selection events (`SelectReportLayoutCode`,
  `SelectedBuiltinLayoutType`) to Base Application's `ReportManagement` subscribers only, so Microsoft's selection code
  runs as AL (hard rule 7). Subscribers of other apps keep the "a subscriber the test can see" refusal.
- The answer (layout name and app id, or a custom layout code) must name a layout the runtime can find: a rendering layout
  the report or a reportextension declares, or a `Tenant Report Layout` row for the report. Its type decides the grid
  column. Word gives the Html placeholder.
- Refused: a name that matches nothing, a `Custom Report Layout` code (table 9650, the old custom layouts), a type that
  cannot be told, an answer that changes between the two events.

This is not pinned. No corpus case saves with a temporary layout selection, and the events' order inside `SaveAs` is pinned
only for the default layout (AMReportOutputTests.al:210). It is built under the second exception of hard rule 3 (the
reporter's tests pass on BC and need the save to succeed with the selected Word layout), with its own assumption id and probe.
That exception is its only basis. Probe cells S5 (a name that matches nothing) and S6 (the events and their order inside
`SaveAs`) of B5 are required cells of the probe: until they are in the oracle, the unmatched name stays refused and the
two events are raised only to `ReportManagement`, in the one order the default-layout case pins.

### A8. Ways the mark could be lost, and what prevents each

A lost mark is a silent wrong result: AL would read made-up or missing characters as real text.

| # | How it could be lost | What prevents it |
|---|---|---|
| L1 | A native reads a Text argument with `as_str()` and never asks `has_marker` | The guard of A5 runs before the native and does not depend on how the native reads. A test walks the whole builtin catalog: every builtin with a Text, Code, Variant, Joker, TextBuilder or BigText receiver or parameter is called with a marked value and must refuse, unless its id and position are a row of A3. A new builtin without a row refuses and the test names it. |
| L2 | A text part without the mark (`CopyStr`, `T[i]`, `Split`) | All refused (table). The catalog test covers them. The VM's index and `foreach` paths (`interp.rs:516`, `:1306`) already test `has_marker`; a unit test per path. |
| L3 | Truncation into `Text[N]` or a field | Conversion refuses (A5). Test: assign to `Text[10]`, a record field, a `Code[20]` parameter. |
| L4 | A stream write in an encoding that drops the mark, then a read | Non-UTF-8 writes refuse. Test per `TextEncoding`. |
| L5 | A byte path: a marked buffer read as bytes, hashed, sized exactly | The buffer carries the placeholder mark; the 2026-11-11 tests (`report_no_output_*`, S1 to S7) run for the new kind too. |
| L6 | The sidecar returns a text or number computed from the mark | `DOTNET_ALLOWED` is the only way in. Test: every DotNet call in the test fixture with a marked argument refuses; P12 keeps the mark. A sidecar result for P12 that has lost a mark refuses (the count of marks in and out must match). |
| L7 | A second source: someone marks another reader as "whole read" | A2 is one function (`rendered_text::read_whole`). Test: every stream and blob reader of the catalog on a Html placeholder buffer refuses except the two of A2. |
| L8 | The placeholder markup itself reaches AL | A2 gives the mark alone. Test: after a read, the text holds no character of the placeholder file. |
| L9 | AL code or data holds U+FDD4 by itself | It refuses where it would have run. That is a wrong refusal, not a wrong result. Same as the three existing marks. |
| L10 | A container compares or prints its elements (List.Contains, Dictionary key, `Format(List)`) | Element comparison goes through `ops::compare`; keys refuse in the guard. Catalog test rows for containers that hold a marked value. |
| L11 | The runner prints the mark or the placeholder in a report | `Describe` test for the mark; a test that a JUnit file and `--explain` of a refused test hold neither U+FDD4 nor the placeholder markup. |
| L13 | An ordered comparison answers by code point | `compare_order` refuses a marked operand (A5). Test: `<`, `>`, `<=`, `>=`, `in` range and `case` range, with the mark at the start and after a differing known prefix, for each of the four marks. |
| L14 | A length leaves as a number (Blob, Temp Blob, file, return value of `Write`, `WriteText`, `GetSubText`, `Read`) | Sign-only length (A4); the rows P3 and P7 require an unused return value. Test per call, with the value used and unused. |
| L15 | A marked text from an earlier session is read with another session's render list, or with the guard off | Process-wide render numbers and flag (A1, A5). Test: strict order, a SingleInstance codeunit keeps the body of render A, the next test codeunit renders B and asks about the kept body. |
| L16 | An allowed step changes the content and Part B still answers from the layout | The whole and changed forms (A1). Test: after P12 `Contains` refuses; a `#n` field refuses. The catalog test fails when a row of A3 does not declare its form. |
| L12 | A later change adds an allowed use by widening a rule | Table A3 is data in one file with one row per builtin id and position, each with its citation. A test fails when a row has no citation or when the table and this ADR section differ in count. |

Added after review round 1 (2026-11-12): the guard of A5 covers builtin calls only. These paths are not builtin calls and
each has its own owner and test (`al-vm/tests/reporting/rendered_text_leaks.rs`):

| # | Loss | What prevents it |
|---|---|---|
| L17 | `T[i] := c` writes over the mark or a character of unknown position | `interp.rs` `write_char` refuses rendered content at every position. For the three other marks it keeps a position inside the known characters before the first mark and refuses the rest. |
| L18 | A `Text[N]` variable takes the text through the write-back of a `var` argument (`InStream.Read`, `Dictionary.Get`, `List.Get`, `BigText.GetSubText`, a procedure's `var Text` parameter) or as a `foreach` variable | `interp.rs` `rendered_into_place`: the conversion of an assignment (A5). |
| L19 | A write that is no append changes a buffer a marked text was written to, or a marked text is written over bytes that were there | `stream::write_bytes` and `rendered_text::mark_written`: the buffer gets the kind "text with rendered content that was written over", and no read answers for it. BC's rendering is longer than its mark, so which bytes BC keeps is not known. |
| L20 | A report dataset column or an XmlPort text variable writes the mark as bytes into a buffer without a mark | `dataset::escape` and `xp_pass_variable` refuse. |
| L21 | A page control's text reaches `AssertEquals`, an `As*` method, a lookup or a request page's parameters | `control_text` refuses; only `TestField.Value()` reads it (P11). |

`BigText.GetSubText` of P3 is the Text target overload with the Integer position 1 only; the BigText target is not in table A3
and refuses.

### A9. Assumptions and flagged passes

- `report-html-rendered-text`: marked when a read of A2 gives a marked text. Text: the Html of a Word layout is not empty,
  and a body that only moves through Text values and UTF-8 streams never changes what the test can see. Source: the
  reporter's tests (al-runner issue 6924). A test through it is a flagged pass.
- `email-body-rgba-replace-keeps-content`: marked when P12 runs. Text: `Regex.Replace` with that pattern and replacement
  maps a not-empty text to a not-empty text.
- `report-layout-temp-selection`: marked on every save A7 resolves (the selected layout is a Tenant Report Layout row, never the
  report's default).
- All three get a row in `docs/probes-needed.md` and a file under `al-builtins/src/assumptions/`.
- Probes to queue on the corpus (Windows tier, it renders): (1) a Word layout report saved as Html to a Temp Blob,
  `InStream.Read(Text)`, then `T <> ''`, `Format(T) = T`, `TB.Append(T); TB.ToText() = T`, a UTF-8 stream round trip
  `= T`, a Blob field stored and read back `= T`. (2) `Email Message`.`Create(.., T, true)` then `GetBody() <> ''`.
  (3) A user-defined Word layout selected with `SetTempLayoutSelectedName`, saved as Html: answer, events and their order.
  (4) `TestField.Value()` of a `MultiLine` control and of a `RichContent` control over a long Text variable.
  When a probe lands it wins.

### A10. Evidence

| Claim | Pinned by | State |
|---|---|---|
| Html of a Word layout has output and is markup | reports/layouts-saveas/AMReportOutputTests.al:205-217 | pinned |
| That same case reads 5 characters and runs `Contains` on the Html (`has_request_value`, `has_constant_column` both No) | AMReportOutputTests.al:43, :165-166, :215-216 | pinned on BC; stays not passing here (reads content) |
| Html of an RDLC layout is refused by BC | text/TestReportSaveAsDatasetContracts.al `SaveAs_Html_WithAnRdlcLayout_IsRefused` | pinned, built |
| A marked text is not `''`, and round-trips through a stream | errors/last-error/WdCallStackToBlob.al:96-102 (call stack mark) | pinned for the mechanism, not for Html |
| A marked text may go to telemetry unread | session-system/session/SqlCounters.al:516 | pinned for the mechanism |
| `Read(Text)` without a length reads to the terminator or the end | streams/ProbeJsonAndInStreamBytes.al `InStreamRead_Text_*` | pinned for known text |
| `StrSubstNo` fills a value through `Format` | format-evaluate/P86StrSubstNo.al | pinned for known text |
| The e-mail path (`Email Item`, `Email Message`, the editor page) with a rendered body | no corpus case | **not measured**; reporter's 188 tests pass on BC |
| A save with a temporary layout selection | no corpus case | **not measured** (A7) |
| `TestField.Value()` of the editor's body controls | no corpus case; `RichContent` appears only in must-not-compile | **not measured** |
| `Regex.Replace` keeps a not-empty text not empty | reasoning on the pattern only | **not measured** |
| Which of the 188 tests meet a refused use after the body is read | the reporter's `--explain` after the build | **not known** |

### A11. What it unlocks, and what stays

- Reporter's suite: up to 188 of 229 (184 + 4) with Part A. With Part A alone the 41 `Contains` tests are reported as not
  run; with Part B those whose marker is a certain static text of the layout the save used can run. "Up to": a test that meets
  a refused use on the way (for example a connector that reads the body into JSON) stays not run, and `--explain` names
  the use.
- Microsoft's tests: not counted. A count of tests that stop at "read of the content of a placeholder Html" after the
  2026-11-11 build gives the number.
- Corpus: no case moves. AMReportOutputTests.al `WordLayout_SaveAsHtml` reads characters and stays listed.

### A12. Conformance

- `al-vm` tests `rendered_text_*`: one per row of A3 (allowed: the result holds the mark and the known parts in order),
  one per row of the refusal table, one per line L1 to L12.
- The catalog test of L1 in `al-builtins`, and the table and ADR count test of L12.
- The assumption registry test for the three ids; `--assumptions-doc` regenerated.
- A reduced AL fixture of the reporter's path (Temp Blob, `Email Item`, `Email Message`, signature, editor handler) run
  against Microsoft's apps: passes flagged, and its `Contains` twin is not run.
- `--baseline-diff` on BC 27, 28 and 29: no regression, no new failure.

## Part B: known static text of the chosen layout

Owner decision 2026-11-12: this part belongs to the amendment. His reasoning: a test of an extension should check what
the extension controls, which is the dataset, the layout file and which layout is used. A check that the fixed text of
the chosen layout appears in the output is a valid case of "which layout is used". Any other check of the output belongs
in a dataset test. So one question about the content is answered, and only with "yes". Part A does not depend on Part B.

### B1. Why

A test can prove from end to end that the layout its code picked is the layout the save used: each layout holds a
different fixed text, and the test searches the saved Html for it. With Part A alone such a test is never run. The
reporter's suite has 41 of them (comment of 2026-11-12 07:08 UTC), and his team keeps them for exactly this reason.

### B2. Decision

The session entry of a render (A1) also records the **layout** that A7 resolved and the **certain static texts** of that
layout. One more question is then answered:

- `T.Contains(X)` answers **true** when all of this holds: `T` holds a mark whose render has a certain static text that
  contains `X` (ordinal, case-sensitive); that mark has the **whole** form (A1); `X` is a known text of 1 to 250
  characters, each an ASCII letter or digit.
- A mark in the **changed** form never answers. So a body that went through `Email Message`.`Create` (P12, the
  `Regex.Replace`) is not answered: the replacement can rewrite text next to or inside a static text (`rgba(1,2,3,1)` as
  layout text, or split over runs), and a rule on `X` alone cannot exclude that. A body read before that call (from
  `Email Item`, from the Temp Blob) is answered. Answering after P12 needs a proof for that transform and probe cells
  (group P, B5). Owner decision 2026-11-12: not answered after P12 until that probe exists.
- A static text that contains `rgba(` is dropped from the list, so the list stays right if that decision is taken later.
- **False is never answered.** When the rule does not give true, the call refuses as in Part A.
- Everything else refuses as in Part A. `StrPos`, `IndexOf` and their `> 0` forms stay refused: they return a position,
  and the position is not known. Answering "some position above 0" would need a second kind of unknown value (question 5).

A **certain static text** is the text of one `w:t` element of `word/document.xml` of a Word layout for which all of this holds:

- It is in a paragraph (`w:p`) that is a direct child of `w:body`, or of a table cell that is itself in no content control.
- No ancestor is a content control (`w:sdt`), so it is in no repeater, no data-bound control and no conditional region.
- The paragraph holds no field (`w:fldSimple`, `w:fldChar`, `w:instrText`), no drawing, picture or text box, no
  `mc:AlternateContent`.
- Neither the run nor the paragraph is hidden (`w:vanish`, `w:webHidden`), the paragraph names no style (`w:pStyle`) and
  the run names no character style (`w:rStyle`), because a style can hide or re-case text. The run has no `w:caps` and no
  `w:smallCaps` (the Html may hold the text in upper case). The run is not a deletion or a move (`w:del`, `w:moveFrom`).
- Document-wide formatting: when `word/styles.xml` cannot be read, or its `w:docDefaults` or its default paragraph style
  set `w:vanish`, `w:caps` or `w:smallCaps`, the document gives no static texts at all. The same for a document with
  tracked changes.
- Headers, footers, footnotes and comments are other parts of the file and are not read.
- One `w:t` only. Text that Word split over two runs is not joined, because the Html may put markup between the runs.

A layout with no such text, a layout that is not a .docx, a zip or XML that does not parse cleanly, or an encrypted file
gives an empty list. Then every question refuses as in Part A. The rule names no app, report or text: it reads whatever
Word layout A7 resolved.

The rows above are the starting rule. The probe of B5 decides each row; a row the probe does not confirm on BC is removed
before the build, and a row the probe shows to be too strict may be widened only to what the probe measured.

### B3. What it needs

- Reading the layout bytes: the `Tenant Report Layout` row's layout Blob, or the layout file of a declared rendering layout
  (already loaded for layout export, `al-vm/src/report_layout_export.rs`).
- A zip reader for one entry and an XML reader. `flate2` (MIT or Apache-2.0) and `quick-xml` (MIT) are already
  dependencies of the workspace. The zip central directory is read by code of our own or by the reader the Data
  Compression support already has. No new crate with another license class. The parser is a new module
  `al-vm/src/report_placeholder/static_text.rs`; it does no OS access.
- The parse runs once per render and only when a layout was resolved. No cost for other tests.

### B4. Risks

| Risk | Effect | Answer |
|---|---|---|
| BC's Word to Html conversion drops, splits, re-encodes or re-cases a static text | a wrong **pass** | Built only on what the probe of B5 shows on BC. Letters and digits only, one `w:t` only. |
| The layout BC used is not the layout the runtime resolved | a wrong pass on a test whose claim is exactly that | A7 runs Microsoft's selection code as AL. Any doubt in A7 refuses the save, so no static text exists. Probe cell group S of B5 measures it. |
| A static text sits in a region Word shows only sometimes (styles, section properties, compatibility settings) | a wrong pass | The list of B2 excludes what is known, and a paragraph that names a style is excluded. The probe lists more. |
| The body was changed after the read (a subscriber replaces text, a `#n` field pads or cuts it) | a wrong pass | Only a mark in the whole form answers (A1). Every allowed row declares its form; P12 gives the changed form, `#n` is refused. A step added later gives the changed form unless it proves it embeds the content unchanged. |
| A wrong answer here is a wrong pass, not a refusal | trust in a green run | Only "true" is answered, each answer marks an assumption (flagged pass), and the build waits for the probe (question 4). |

### B5. The Windows probe, precisely

The Linux tier of the corpus does not render Word layouts, so the probe runs on the Windows tier. It is one new folder in
the corpus, written so that a probe agent can start from this text.

Files (folder `tests/al-language/reports/layouts-html-text/`, object ids and prefix from the corpus's free range):

| File | Content |
|---|---|
| `HtFixtures.al` | Table `HT Row` (Integer key, `Txt` Text[100]). Report `HT Static Rpt`: one data item on `HT Row`, columns `Txt` and a constant column `Const` with the value `HTCONST1`; `DefaultRenderingLayout` = `HtMain`; rendering layouts `HtMain` (`HtMain.docx`) and `HtOther` (`HtOther.docx`), both Word. A second report `HT Empty Rpt` with the same layout shapes whose data item has a filter that gives 0 rows. |
| `HtMain.docx`, `HtOther.docx` | Word layouts made by hand in Word from the report's exported XML part. `HtMain.docx` holds the cells of the table below. `HtOther.docx` holds only one body paragraph `HTOTHERLAYOUT1`. |
| `HtSave.al` | Codeunit `HT Save`: `Save(ReportId; LayoutName; var Body: Text)` sets the layout when a name is given, saves with `Report.SaveAs(ReportId, '', ReportFormat::Html, OutStream)` into a Temp Blob on a UTF-8 stream and reads it with `InStream.Read(Body)`. Also `SaveBytesLog`: length > 0 and the first 15 characters. |
| `HtStaticTextTests.al` | Test codeunit. One test per cell. Each test observes `Format(Body.Contains(<marker>))` through `Probe Observer` and expects the value BC gives. The expected values are filled in from the first CI run of the probe branch (ADR-007), not guessed. |

Cells in `HtMain.docx`. Every marker is unique, ASCII letters and digits, and appears nowhere else:

| Cell | Where the marker is in the .docx | Question |
|---|---|---|
| C1 | plain paragraph, direct child of the body, one run: `HTBODY1` | Is plain body text in the Html, unchanged? |
| C2 | table cell, no content control: `HTCELL1` | Same for a table cell. |
| C3 | page header: `HTHEADER1`; page footer: `HTFOOTER1` | Does the Html hold header and footer text? |
| C4 | plain-text content control bound to column `Const`, placeholder text `HTPLACEHOLDER1` | Is the dataset value `HTCONST1` in the Html, and is the control's placeholder text gone? |
| C5 | static text `HTINREPEAT1` inside the repeater over the data item, beside the bound `Txt` control | With 2 rows: present. With 0 rows (`HT Empty Rpt`): present or gone? |
| C6 | static text `HTAFTERREPEAT1` in a body paragraph after the repeater | Present with 0 rows too? |
| C7 | run with hidden text (`w:vanish`): `HTHIDDEN1`; paragraph in a style whose font is hidden: `HTHIDDENSTYLE1` | Does hidden text reach the Html? |
| C8 | text inside a plain (unbound) content control: `HTPLAINSDT1` | Is static text in an unbound control kept? |
| C9 | one word with a format change in the middle (two runs): `HTSPL` bold + `IT1` regular | Does `Contains('HTSPLIT1')` hold, or does markup sit between the runs? |
| C10 | `HTAMP1 & HTAMP2`, `HTLT1 < HTLT2`, `HTGT1 > HTGT2` in one paragraph each | How are `&`, `<` and `>` written: `Contains('HTAMP1 & HTAMP2')` and `Contains('HTAMP1 &amp; HTAMP2')`, the same for `&lt;` and `&gt;`. |
| C11 | `HTUML1 äöü é € HTUML2` | Are non-ASCII letters written as they are, as numeric entities or as named entities? One `Contains` per form. |
| C12 | `HTSP1 HTSP2` with one space, with two spaces, with a non-breaking space, with a tab | Is a space kept as a space? One `Contains` per form. |
| C13 | `HTFIELD1` beside a merge field or a `DATE` field in one paragraph | Is static text beside a field kept? |
| C14 | text box: `HTTEXTBOX1`; footnote: `HTFOOTNOTE1` | Do these parts reach the Html? |
| C15 | `HTCase1` mixed case | Is the case kept (`Contains('HTCase1')`, `Contains('HTCASE1')`)? |
| C16 | a paragraph with a tracked insertion `HTINS1` and a tracked deletion `HTDEL1` | Which of the two is in the Html? |
| C17 | a run with all caps (`w:caps`): `HTCaps1`; a run with small caps: `HTSmall1` | Is the text in the Html as typed or in upper case? `Contains` of both forms. |
| C18 | a run in a character style that hides text: `HTRSTYLEHID1`; a run in a character style with all caps: `HTRStyleCaps1` | Does a character style change what the Html holds? |
| C19 | a second layout `HtDefaults.docx` whose document defaults set all caps, with one body paragraph `HTDefCaps1` | Do document defaults change the text? |
| C20 | layout text `HTRGB1 rgba(1,2,3,1) HTRGB2` in one run, and `rgba(1,2,3,` + `1)` split by a format change | What the Html holds before `Email Message`.`Create` and what `GetBody()` holds after it (`Contains('rgba(1,2,3,1)')`, `Contains('rgb(1, 2, 3)')`). |

Cell group S, which layout is used (two tests per row: `Contains('HTBODY1')` and `Contains('HTOTHERLAYOUT1')`):

| Cell | Setup |
|---|---|
| S1 | default layout, no selection |
| S2 | `Report Layout Selection`.`SetTempLayoutSelectedName('HtOther', AppId)` before the save, `ClearTempLayoutSelected` after |
| S3 | a `Tenant Report Layout` row for the report made from `HtOther.docx` with a GUID as name (the way `UserDefinedWordExcelLayoutRows.al` makes its rows), selected by name with an empty app id |
| S4 | a `Tenant Report Layout Selection` row that names `HtOther` |
| S5 | S2 with a name that matches no layout: answer of `SaveAs`, error text and code, output length |
| S6 | S2 and a logging subscriber to `SubstituteReport`, `SelectReportLayoutCode`, `SelectedBuiltinLayoutType`, `OnDocumentReady`: the events and their order inside `SaveAs` (the value for A7) |

Group P, the Part A claims on real text (A9 probes 1 and 2): after the read in S1, `Body <> ''`, `Format(Body) = Body`,
TextBuilder and UTF-8 stream round trips equal `Body`, a Blob field stored and read back equals `Body`, and
`Email Message`.`Create(.., Body, true)` then `GetBody() <> ''` and `GetBody().Contains('HTBODY1')`. Also in group P:

- P-quit: a report whose `OnPreReport` calls `CurrReport.Quit`, saved as Html: the answer of `SaveAs`, `TempBlob.Length() > 0`, `Body <> ''`.
- P-empty: `HT Empty Rpt` (0 rows): the same three observations, and `Contains('HTBODY1')`.
- P-order: `('Body ' + Body) < 'body'` and `Body < 'a'`, to keep the culture order of joined text on record.
- P-length: `TempBlob.Length()` of the Html compared with 0 and logged as "over 1000: yes/no".
- P-pad: `StrSubstNo('#1########', Body)` equals `'**********'`.

Rules for the probe agent: one observation per `Contains`, no assert that depends on another cell; the .docx files are
committed binary fixtures like `Y17LayR1.docx`; the tests are Windows-tier only; nothing of Microsoft's is copied.

- Assumption id `report-html-layout-static-text`, marked each time `Contains` answers true from a static text. A test
  through it is a flagged pass until the probe is in the corpus and the oracle; then the id is removed and the cells are
  cited in the code.

### B6. Conformance

- Parser tests on small .docx fixtures made for the tests (one per rule of B2).
- `rendered_text_static_*` tests: true only inside the rule, refusal outside, never false.
- The reduced fixture of A12 with two layouts and two markers: the right marker passes flagged, the other marker's
  `Contains` is not run.
- After the corpus bump: the probe's cells that the rule answers pass; every other cell stays listed as not passing
  (it asks for false or for text outside the rule), which is the expected state.

## Open questions for the owner (amendment 2026-11-12; answered 2026-11-12, see Status)

1. Accept Part A as written: a rendered Html body may be read whole into a Text, moved and compared with `''`, and every
   other use refuses?
2. Accept A7: a save with a temporary layout selection is no longer refused when Microsoft's selection code, run as AL,
   names a Word layout the runtime can find? Without it, none of the 229 tests reaches the placeholder.
3. Accept P12: one exact `Regex.Replace` call from `Email Message`.`Create` may run on a marked text? Without it, every
   `Email Message`.`Create` with a rendered body refuses.
4. Part B is decided (owner, 2026-11-12). Still open: build it only after the Windows probe of B5 is merged, or before
   it as a flagged pass? A wrong answer in Part B is a wrong pass, so this text proposes to wait for the probe.
5. Part B: answer `StrPos(T, X) > 0` and `IndexOf(X) > 0` too? That needs a second unknown value, "a position above 0".
   This text proposes `Contains` only for now.
6. `Error` and `Message` with a marked text refuse in step 1. Open `Message` to a MessageHandler that only receives the
   text when a reported test needs it, or keep it refused?
