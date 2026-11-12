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
  arithmetic, another comparison, a record field). This is the correction the 2026-11-12 amendment (A4) names. It is a value and
  not only the syntactic form `Blob.Length() > 0`, so that `Temp Blob.Length() > 0`, which returns the Blob's length through
  two procedures, answers too (AMReportOutputTests.al:146, CiReportRunLeads.al:72).
- A run that would print (SystemPrinter `true`, or a report that declares `UseSystemPrinter = true`) is refused, as "What stays
  refused" says. An Excel layout run checks the layout event subscribers and the layout selections before it raises its pinned
  error, as the RDLC and Word runs do.
- The false answers raise SubstituteReport (static form, as the RDLC to Html answer) and not OnDocumentReady (AMReportOutputTests.al:221,
  :257); a subscriber the test can see to another layout event refuses them.
