# ADR-067: An InStream handed to .NET carries its read position

- Status: Proposed
- Date: 2026-11-05

## Context

ADR-014 section 4 (streams, item 1) sends .NET the unread bytes of an AL `InStream`. The sidecar wraps them in a read-only
`MemoryStream` that starts at 0, and after the call Rust moves the AL position by the bytes .NET consumed.

BC hands .NET the Blob's own stream, which seeks (`SgDotNetStreams.al:106-126`). .NET code can seek back before the AL
position and read bytes AL or earlier .NET code already read. Microsoft's Excel Buffer does this: `SelectSheetsNameStream`
and then `OpenBookStream` on one `InStream` call `WorkbookReader.Open` twice on the same stream (Base Application
`ExcelBuffer.Table.al`, `GetSheetsNameListFromStream` and `OpenBookStream`; the AL code does not reset the stream). The second
open reads the zip again from its start. Real BC 27.5, 28.5 and 29.0 pass this (al-runner issue 7134; corpus pull request 1152,
`streams/excel-buffer-roundtrip/TestExcelStream.Codeunit.al`, `ExcelBuffer_SelectSheetThenRead_TempBlob_SameStream_PreservesCell`
and `_StoredBlob_SameStream_PreservesCell`). This runtime sends the second call only the bytes after the position the first call
left, and `WorkbookReader.Open` raises "The Microsoft Excel workbook cannot be opened". The test is refused.

Fixes that keep the current protocol were checked and rejected:

1. Send all bytes from 0 when the position is past the start. A `ZipArchive` seeks to absolute offsets, so it would work. A .NET
   reader that reads on from the current position (a `StreamReader` after AL read a header line) would read the bytes again,
   silently. That breaks hard rule 4.
2. Send the stream as a seeded `OutStream` bridge (`outs` with `v`, ADR-014 decision of 2026-10-30). That bridge seeks but
   always stands at the end of the bytes, so it matches BC only when the AL position is at the end. After `WorkbookReader.Open`
   the position is inside the stream. A bridge also reports no position back, and bytes written to it would be appended to the
   Blob, which an `InStream` must not do.
3. Reset the AL stream in Rust before the call. AL can see the position (`EOS`, the next `Read`), so that changes observable
   behavior.

Each needs the start position on the wire, which ADR-014 does not carry.

## Decision

1. A stream at its start is sent as today (`{"t":"ins","v":...}`).
2. A stream whose position is past its start is sent whole, with its position: `{"t":"ins","v":<all bytes>,"p":<position>}`
   (Rust: a new `DotNetValue::InStreamAt { bytes, at }`).
3. The sidecar's read-only `MemoryStream` starts at `p`. A .NET reader that reads on sees what it sees today. Code that seeks
   back sees the bytes before the position, as in BC.
4. The `consumed` count the sidecar reports stays the stream's `Position`, now counted from the start of all the bytes. Rust
   sets the AL position to it (today it adds the count to the old position).
5. The protocol version goes from 4 to 5. Host and sidecar change together, and a mismatch is refused at start as today.
6. Still copy-in and copy-out: no callback protocol, so ADR-014 section 4 item 4 stands.
7. Where the AL position stands after each call is not pinned. The path marks the assumption `instream-dotnet-seek-back` until a
   probe pins `EOS` after each Excel Buffer call (`streams/excel-buffer-roundtrip/R7134ExcelStreamPosition.al` on the corpus
   branch `al-runtime-rs-staging-r7132`, not run on BC yet).

## Consequences

- The two same-stream Excel Buffer cases of corpus pull request 1152 can pass (flagged until the position probe lands).
- A stream past its start costs one copy of all its bytes instead of the rest. This is the same copy cost class as today.
- An older sidecar does not work with a newer host, as with every protocol bump.
- A reference implementation with a red-then-green test (`crates/al-vm/tests/dotnet.rs`,
  `an_instream_dotnet_already_read_is_sent_whole_with_its_position_and_the_position_follows_dotnet`) is on the branch
  `rt-wp9-7134`. It is not merged and waits for this ADR.

## Conformance

- `crates/al-testrun/src/dotnet.rs` `encode` test: the `ins` tag with `p`.
- `crates/al-vm/tests/dotnet.rs`: a second .NET call on a stream an earlier call read sends all bytes with the position, and the
  AL position follows what .NET reports.
- Corpus: `TestExcelStream.Codeunit.al` same-stream cases, once pinned.
