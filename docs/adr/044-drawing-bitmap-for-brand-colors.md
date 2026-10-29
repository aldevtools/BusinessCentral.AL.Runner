# ADR-044: `System.Drawing.Bitmap` for Microsoft's brand colors

- Status: Accepted (owner, 2026-10-18, option 3). Linux and macOS only since 2026-10-29: on Windows the registry claims no System.Drawing type and every System.Drawing call goes to .NET (ADR-027 row 14)
- Date: 2026-10-18
- Related: ADR-027 (scope, row 14), ADR-014 (policy, section 8), ADR-024, ADR-039, ADR-040 (same pattern), ADR-009

## Context

Both Contoso demo-data lanes of Microsoft's test recipe stop in their first step. `Foundation Module`.CreateSetupData
calls `Create Company Information`.CreateSetupCompany (CreateCompanyInformation.Codeunit.al:76), which calls Base App
`O365 Brand Color`.CreateDefaultBrandColors. For each of 12 colors that calls `MakePicture` (O365BrandColor.Table.al:113-137).
The refusal is `DotNet new System.Drawing.Bitmap` (policy.txt:61, ADR-027 row 14: "v3 will not substitute an imaging
library, bytes would differ from GDI+").

### What `MakePicture` uses

`new Bitmap(100, 100)`; `Graphics.FromImage`; `ColorTranslator.FromHtml("#RRGGBB")`; `new SolidBrush(Color)`;
`Graphics.FillEllipse(brush, 0, 0, 100, 100)`; `Graphics.Dispose`; `Bitmap.Save(InStream, ImageFormat.Png)`;
`Bitmap.Dispose`. The result goes to `"Sample Picture"` (field 15, type Media) through `ImportStream(InStr, '')`.
The brand colors page shows it as a brick. Nothing else in Microsoft's source reads it.

### What Microsoft's tests observe

Only `HasValue`: O365BrandColorTests.Codeunit.al:74 and :158 (`Assert.IsTrue(O365BrandColor."Sample Picture".HasValue, ...)`).
No test reads the bytes, the size or the media id. The Contoso data tool only needs `MakePicture` not to fail.

### Can the .NET helper (sidecar) run it on Linux?

No. Measured on this machine (.NET 8, `System.Drawing.Common` 8.0.0, MIT): `new Bitmap(100, 100)` throws
`TypeInitializationException: The type initializer for 'Gdip' threw an exception`. Since .NET 7 the package supports
Windows only; the Linux switch of .NET 6 is gone. `ColorTranslator.FromHtml` does work (managed code). The only way back
is libgdiplus (Mono's native GDI+ clone) installed on the host, plus a runtime that still honors it. That is a system
package outside our control (not in the Arch repositories checked here), and I could not verify its license offline;
it must be checked against the allow-list before anyone relies on it. It also brings back an install the project is
removing (ADR-039). The corpus CI runs BC in a Linux container, so a probe of Bitmap there would hit the same error:
a probe needs the Windows nightly (Linux suspect rule).

Permissive imaging libraries exist (SkiaSharp, MIT, native part BSD-3; Rust `png` and `tiny-skia`, MIT/Apache and BSD-3).
SixLabors ImageSharp is not on the allow-list (split license). None of them draws the same bytes as GDI+.

## Options

1. **Native stand-in limited to the eight members above.** Draw the ellipse, encode a PNG. About 300 lines in Rust, the
   members registered with pins. Bytes differ from GDI+, so anything that exports the picture and compares bytes
   would be wrong. Needs a probe of what AL can see (below) before it is pinned. Reverses the ADR-027 row 14 wording.
2. **Sidecar loading.** Not possible on Linux (see above). With a shim type backed by SkiaSharp it is option 1 with a
   .NET dependency and a second PNG encoder. Not recommended.
3. **Run `MakePicture`, refuse only what is observable (ADR-040 pattern).** The Bitmap members above are accepted and draw
   nothing. `Save` marks the stream as a "picture placeholder"; `ImportStream` gives the media a value so `HasValue`
   is true. Any later read of the picture content (`ExportStream`, `Length`, `CopyStream`, a download) raises
   `Signal::Unsupported`. Everything outside the eight members stays refused. Smallest change and it cannot show a
   wrong byte. It needs a placeholder flag on a media blob (an additive field, but it touches the Media storage
   path) and the same probe.
4. **Do nothing.** Both lanes stay refused with this reason, and everything behind it stays unreached.

## Recommendation

Option 3. It unblocks the lanes, keeps hard rule 4 (nothing observable is guessed) and follows ADR-040. Option 1 only
if the owner wants pictures that can be exported.

## Probes wanted (corpus, Windows nightly; the Linux container cannot run Bitmap)

1. After `Bitmap(100, 100)`, `FillEllipse`, `Save(InStream of a Temp Blob, ImageFormat.Png)` and `Media.ImportStream`:
   is `HasValue` true, and what are `ExportStream` length and its first 8 bytes (PNG signature `89 50 4E 47 0D 0A 1A 0A`)?
   (Decides whether `Save` into an InStream writes into the blob at all; the Microsoft tests say it does.)
2. The width and height in the PNG header (expect 100 by 100), and the pixel at the center versus a corner.
3. `Save` with another `ImageFormat` (Jpeg, Bmp): refused in any option until probed.

## Decision

Option 3, accepted by the owner on 2026-10-18.

### Owner decision (2026-10-18): option 3

Accept the eight `System.Drawing` members `MakePicture` uses. They draw nothing. The picture the stream and the
Media field hold has a value. Any read of the picture content is refused.

What the implementation does (`crates/al-vm/src/dotnet/drawing.rs`, tests in `crates/al-vm/tests/host/bitmap.rs`):

- Served by the registry (ADR-014 section 1), so a run needs no .NET host: `new Bitmap(Integer, Integer)` (both positive),
  `Graphics.FromImage(Bitmap)`, `ColorTranslator.FromHtml('#RRGGBB')`, `new SolidBrush(Color)`,
  `Graphics.FillEllipse(SolidBrush, Integer x4)`, `Graphics.Dispose()`, `Bitmap.Save(InStream, ImageFormat.Png)`,
  `Bitmap.Dispose()`, and the static `ImageFormat.Png`.
- Every other member of these types is refused ("a member the runtime does not implement"). So is every other argument
  kind: a name or short form for `FromHtml`, a size of zero, `Save` to an OutStream or with another format.
  `policy.txt:61` (`deny-type System.Drawing.`) stays, so every other `System.Drawing` type is still refused.
- `Save` sets the ADR-040 mark (`BlobBuf::render_pending`) on the buffer its InStream reads. Stream reads, `Length`,
  `HasValue` of that Blob and every other ADR-040 observation path are refused with
  `picture drawn by System.Drawing.Bitmap is not produced`.
- `Media.ImportStream` of such a stream does not read it. It stores a media object with no bytes and the same mark. The
  field then has a value (`HasValue` true). `Media.ExportStream` and the `Tenant Media` row of that media (content, size,
  MIME type) are refused with the same text. `MediaId` is only a Guid and is not refused.
- Assumption `bitmap-picture-not-drawn` (hard rule 3 exception of 2026-10-15), marked on `new Bitmap` and on `Save`. Its
  evidence is Microsoft's O365BrandColorTests.Codeunit.al:74 and :158. The probes of the section above are its row in
  `docs/probes-needed.md`. When they land, the probe wins.
- ADR-027 row 14 ("v3 will not substitute an imaging library") still holds: no imaging library is added and no pixel is
  drawn.

## Consequences

Option 3: 12 more Contoso steps run; the Foundation data exists for the 13 tests that depend on it. Any test that reads the
picture bytes is refused with a reason naming the media field.

## Conformance

`crates/al-vm/tests/host/bitmap.rs`: MakePicture runs and the Media has a value (also after Insert and Get), a stream read, `Length` and `ExportStream` are refused, members and argument kinds outside the eight are refused. `the_declared_result_types_match_the_registry` keeps `dotnet_sigs.rs` in step with the registry.
