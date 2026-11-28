//! The gate of a .NET rewrite of the bytes a bridge had handed back to its Blob (`StreamEffects::replaced`), and the seed of
//! a ZipArchive in Update mode over a fresh `CreateOutStream`. One owner: `Session::apply_stream_effects` asks
//! [`Session::rewrite_verdict`], `Session::value_to_wire` asks [`fresh_update_seed`].
//!
//! Pinned: the Dispose of `ZipArchive(OutStream, Update, false, Encoding)` over an OutStream AL copied a zip into, with the
//! Blob unchanged since, makes the Blob the archive's whole content (tests/al-language/streams/blob-tempblob/
//! ZipBlobFieldUpdate.al:5-8; al-runner issue 8360).
//!
//! Assumed (`zip-blob-writes`, corpus PR 1633, tests/al-language-onprem-dotnet/dotnet/zip-blob-writes/, the same values on
//! every BC version the corpus runs), each fact by equality:
//! 1. ZipBlobUpdateFreshOutStream.al:49-223: the archive made by the 2-, 3- or 4-argument constructor (Update; leaveOpen false;
//!    Encoding.UTF8) over the OutStream of the `CreateOutStream` that is the newest edit of a loaded Blob with content
//!    (the stream's stamp, not the shared buffer, says which open made it) reads that content (BC hands .NET the Blob's own stream; the Blob is not cut). Its Dispose, the Blob unchanged since, makes
//!    the Blob the archive (108 -> 194 bytes). Temporary and stored records alike (:49 with CalcFields; :85 without it the
//!    Blob is not loaded and the bridge starts empty, as on main).
//! 2. ZipBlobUpdateBlobChanged.al:48-152, a temporary record (`CreateOutStream` opened the stream on a Blob field of a record
//!    that is temporary, and no read or Insert had given the record that Blob when the archive was made): `ZipArchive(OutStream, Update, false, UTF8)` over an
//!    OutStream AL copied a zip into; its Dispose makes the Blob the archive also after AL changed the Blob, in the measured
//!    states only, AL's first change a Clear: Clear, one `CreateOutStream` and one two-argument `CopyStream` through that new
//!    stream of as many bytes as the archive read (:48); that rewrite and bytes .NET then appended through another stream (:69); Clear only (:98); the Dispose of another
//!    archive that read the same bytes, no AL change (:152).
//! 3. ZipBlobDotNetOverwrite.al:25-71, :96-123: the helper reports "overwrite" once per bridge, for a bridge that started empty whose one
//!    .NET object is `StreamWriter(OutStream)` and that holds just the handed-back bytes: that StreamWriter's write inside AL's
//!    `Write(Text)` or `Flush()`, from 0 right after AL's `Position := 0` to the stream's end (48 x B), or AL's `SetLength(10)`
//!    on the stream standing at its end (10 x A) (tools/dotnet-sidecar `Bridge.OverwriteMeasured`). The Blob then takes the
//!    whole content when it held just the bridge's bytes from its start. The 10 x B write (:73-93) leaves the stream at 10, so a
//!    later AL write is not measured: it keeps the mark.
//!
//! Every other rewrite keeps main's answer: an Update archive's rewrite refuses, an overwrite marks the Blob
//! (`DOTNET_BRIDGE_OVERWRITTEN`). Not measured, so not served: another ZipArchiveMode over a fresh OutStream, the B states on a
//! stored table's record not declared temporary (read, inserted or neither), another encoding or leaveOpen in part 2, an AL
//! change other than the measured ones, another writer, start, end or length in part 3, a .NET append with no AL change before
//! the Dispose. A change of the Blob between a fresh archive's constructor and its Dispose refuses: the seed was decided at the
//! constructor (main passed those shapes with an archive that never read the Blob; al-runner issue 9318).
use super::{Bridge, Session};
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::stream::{BlobBuf, BlobRef};
use al_builtins::value::{DotNetKind, Value};
use al_host::dotnet::Rewrite;
use std::sync::Arc;

/// The newest .NET write that reached a Blob through a bridge, for part 2's states (one per Blob).
pub(super) struct NetWrite {
    blob: BlobRef,
    edits_before: u32,
    len_before: usize,
    edits_after: u32,
    replace: bool,
}

/// Part 2's record: `CreateOutStream` opened the stream on a Blob field of a temporary record (declared temporary or of a
/// `TableType = Temporary` table: `RecordState::temporary`). The VM keeps that open on the stream (al-vm `temp_blob_share.rs`).
pub(super) fn opened_on_temp_record(open: Option<&al_builtins::stream::OutOpen>) -> bool {
    open.and_then(|o| o.record.upgrade())
        .is_some_and(|r| r.borrow().temporary)
}

/// What `apply_stream_effects` does with one rewrite.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Verdict {
    /// The Blob takes the whole new content.
    Replace,
    /// The Blob keeps its bytes and is marked `DOTNET_BRIDGE_OVERWRITTEN` (main's answer to an overwrite).
    Mark,
    /// Refused (main's answer to every other rewrite).
    Refuse,
}

/// Part 1's call: the constructor of System.IO.Compression.ZipArchive with ZipArchiveMode Update as its second argument and
/// two, three (leaveOpen false) or four (leaveOpen false, Encoding.UTF8) arguments. The OutStream is argument 0.
pub(super) fn fresh_update_ctor(clr_type: &str, construct: bool, values: &[Value]) -> bool {
    let enum_is = |v: &Value, ty: &str, member: &str| {
        matches!(v, Value::DotNet(o) if matches!(&o.kind, DotNetKind::Enum { clr_type, name, .. }
            if clr_type == ty && name == member))
    };
    let host_is = |v: &Value, ty: &str| matches!(v, Value::DotNet(o) if matches!(&o.kind, DotNetKind::Host { clr_type, .. } if clr_type == ty));
    let update = |v: &Value| enum_is(v, "System.IO.Compression.ZipArchiveMode", "Update");
    construct
        && clr_type == "System.IO.Compression.ZipArchive"
        && match values {
            [_, mode] => update(mode),
            [_, mode, Value::Boolean(false)] => update(mode),
            [_, mode, Value::Boolean(false), enc] => update(mode) && host_is(enc, UTF8_ENCODING),
            _ => false,
        }
}

/// The type `Encoding.UTF8` gives on the .NET runtime of the helper.
const UTF8_ENCODING: &str = "System.Text.UTF8Encoding+UTF8EncodingSealed";

/// Part 1's stream and Blob: the `CreateOutStream` that made the handed stream (its stamp `opened_at`) is the Blob's newest
/// edit (no other `CreateOutStream`, write, Import or Clear since; every OutStream of a Blob shares its buffer, so the
/// stamp, not the buffer, says which stream that open made). The Blob is loaded, with content, the position 0, no mark, no
/// .NET stream view.
pub(super) fn fresh_update_seed(buf: &BlobBuf, opened_at: Option<u32>) -> bool {
    buf.loaded
        && !buf.first_write_truncates
        && buf.render_pending.is_none()
        && buf.dotnet_view == 0
        && !buf.bytes.is_empty()
        && opened_at == Some(buf.content_edits)
        && buf.write_pos == 0
}

/// Part 2's Blob: no read or Insert gave the record that Blob. Read when the bridge is made: a Clear in place gives the Blob a
/// fresh buffer, which resets these flags.
pub(super) fn of_no_row(buf: &BlobBuf) -> bool {
    !buf.db_row && !buf.temp_row && !buf.inserted && !buf.temp_inserted
}

impl Session {
    /// Notes a .NET append or rewrite that reached `blob` (part 2 reads the newest one).
    pub(super) fn note_net_write(&mut self, blob: &BlobRef, edits_before: u32, len_before: usize, replace: bool) {
        let edits_after = blob.borrow().content_edits;
        let writes = &mut self.dotnet.net_writes;
        writes.retain(|w| !Arc::ptr_eq(&w.blob, blob));
        writes.push(NetWrite {
            blob: blob.clone(),
            edits_before,
            len_before,
            edits_after,
            replace,
        });
    }

    /// The verdict on the rewrite of `bridge`, whose Blob held `before` bytes at the bridge's last reply.
    pub(super) fn rewrite_verdict(&self, bridge: &Bridge, before: usize, kind: Option<Rewrite>) -> Verdict {
        let buf = bridge.blob.borrow();
        let unchanged = buf.content_edits == bridge.edits && buf.bytes.len() == before && buf.render_pending.is_none();
        match kind {
            // The pin (8360): the Blob still holds the bridge's bytes, the AL position at their end.
            Some(Rewrite::ZipUpdate4)
                if !bridge.fresh
                    && unchanged
                    && buf.write_pos == before
                    && al_builtins::native::blob_clears_in_place(self, &bridge.blob) =>
            {
                Verdict::Replace
            }
            // Part 1.
            Some(Rewrite::ZipUpdate4 | Rewrite::ZipUpdate) if bridge.fresh && unchanged && buf.write_pos == 0 => {
                assume(Assumption::ZipBlobWrites);
                Verdict::Replace
            }
            // Part 2.
            Some(Rewrite::ZipUpdate4) if !bridge.fresh && self.changed_state_measured(bridge, before, &buf) => {
                assume(Assumption::ZipBlobWrites);
                Verdict::Replace
            }
            // Part 3.
            Some(Rewrite::Overwrite) if bridge.origin_empty && unchanged => {
                assume(Assumption::ZipBlobWrites);
                Verdict::Replace
            }
            Some(Rewrite::Overwrite) => Verdict::Mark,
            _ => Verdict::Refuse,
        }
    }

    /// Part 2: the archive `ZipArchive(OutStream, Update, false, Encoding.UTF8)`, a Blob of a temporary record (the stream
    /// opened on it, no read or Insert gave it the Blob when the bridge was made), cleared in place, no mark, in one measured
    /// state.
    fn changed_state_measured(&self, bridge: &Bridge, before: usize, buf: &BlobBuf) -> bool {
        if !(bridge.update4_utf8
            && bridge.temp_record
            && bridge.of_no_row
            && buf.render_pending.is_none()
            && al_builtins::native::blob_clears_in_place(self, &bridge.blob))
        {
            return false;
        }
        let newest = self
            .dotnet
            .net_writes
            .iter()
            .find(|w| Arc::ptr_eq(&w.blob, &bridge.blob));
        let net_newest = newest.filter(|w| w.edits_after == buf.content_edits);
        // The first AL change since the bridge was made is a Clear of the Blob (a Clear in place sets `cleared_at` to the edit
        // count it leaves: al-builtins `native::values`).
        let lb = &buf.line_breaks;
        let cleared = lb.cleared_at();
        let cleared_first = cleared == bridge.edits.wrapping_add(1);
        // After that Clear, one `CreateOutStream`, then one two-argument `CopyStream` through the stream that open made
        // (ZipBlobUpdateBlobChanged.al:43-45, `NewOut`), which left `edits` edits.
        let rewritten = |edits: u32| {
            let opened = cleared.wrapping_add(1);
            lb.out_opens() == 1
                && lb.out_open_edits() == opened
                && edits == cleared.wrapping_add(2)
                && lb.copied_through(edits) == Some(opened)
        };
        match net_newest {
            // :98 Clear only; :48 Clear and a rewrite of as many bytes. The newest change is AL's.
            None => {
                cleared_first
                    && ((buf.content_edits == cleared && buf.bytes.is_empty())
                        || (rewritten(buf.content_edits) && buf.bytes.len() == before))
            }
            // :69 that rewrite, then a .NET append through another stream.
            Some(w) if !w.replace => {
                cleared_first && rewritten(w.edits_before) && w.len_before == before && buf.bytes.len() > before
            }
            // :152 another archive that read the same bytes, its Dispose first, no AL change before it.
            Some(w) => w.len_before == before && w.edits_before == bridge.edits,
        }
    }
}
