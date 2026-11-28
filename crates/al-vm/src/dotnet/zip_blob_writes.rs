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
//! 1. ZipBlobUpdateFreshOutStream.al:49-223: the archive made by the 2-, 3- or 4-argument constructor (Update;
//!    leaveOpen false; Encoding.UTF8) over the OutStream of the `CreateOutStream` that is the newest edit of a loaded
//!    Blob with content (the stream's stamp, not the shared buffer, says which open made it) reads that content (BC
//!    hands .NET the Blob's own stream; the Blob is not cut). Its Dispose, the Blob unchanged since, makes the Blob the
//!    archive (108 -> 194 bytes). Temporary and stored records alike (:49 with CalcFields; :85 without it the Blob is
//!    not loaded and the bridge starts empty, as on main).
//! 2. ZipBlobUpdateBlobChanged.al:98, :152, a temporary record (`CreateOutStream` opened the stream on a Blob field of
//!    a record that is temporary, and no read or Insert had given the record that Blob when the archive was made):
//!    `ZipArchive(OutStream, Update, false, UTF8)` over an OutStream AL copied a zip into; its Dispose makes the Blob
//!    the archive also after AL changed the Blob, in two states only: Clear as AL's only change (:98); the Dispose of
//!    another such archive over the same OutStream (the same stream stamp) that read the same bytes, no AL change (:152).
//! 3. ZipBlobDotNetOverwrite.al:25-71, :96-123: the helper reports "overwrite" once per bridge, for a bridge that started empty whose one
//!    .NET object is `StreamWriter(OutStream)` and that holds just the handed-back bytes: that StreamWriter's write inside AL's
//!    `Write(Text)` or `Flush()`, from 0 right after AL's `Position := 0` to the stream's end (48 x B), or AL's `SetLength(10)`
//!    on the stream standing at its end (10 x A) (tools/dotnet-sidecar `Bridge.OverwriteMeasured`). The Blob then takes the
//!    whole content when it held just the bridge's bytes from its start and that bridge is the only one AL made over
//!    the Blob (the helper counts the writers of one bridge). The 10 x B write (:73-93) leaves the stream at 10, so a
//!    later AL write is not measured: it keeps the mark.
//!
//! Each served rewrite sets `BlobBuf::dotnet_rewritten`: the probes read the Blob after it, never write it (BC keeps the AL
//! and the .NET write positions apart, SgDotNetStreams.al:154-172), so the next write marks the Blob
//! (`al_builtins::stream::note_write`) and handing its stream to .NET again refuses, as on main after an overwrite.
//!
//! Every assumed rewrite needs the stamp of the stream handed to .NET, at the rewrite as when the bridge was made
//! ([`still_stamped`]). An OutStream copied (`:=`, a by-value parameter, a procedure's return value), before or after
//! .NET got it, loses it ([`unstamp_out_stream`]); one opened again, on its Blob or on another, no longer has it (the
//! stream's Blob and stamp are read together); one read from an array element never has it at .NET
//! ([`note_element_stream`]): not measured, so main's answer (part 1, whose archive the constructor seeded, refuses).
//!
//! Every other rewrite keeps main's answer: an Update archive's rewrite refuses, an overwrite marks the Blob
//! (`DOTNET_BRIDGE_OVERWRITTEN`). Not measured, so not served: another ZipArchiveMode over a fresh OutStream, the B
//! states on a stored table's record not declared temporary (read, inserted or neither), another encoding or leaveOpen
//! in part 2 (for either archive), two archives over the streams of two CreateOutStreams, an AL change other than the
//! measured ones (a Clear, then a rewrite through a new OutStream, with or without a later .NET append, :48 and :69:
//! those cells pin only the length and the entries, and corpus PR 1640's cell D6 measures that BC keeps the rewrite's
//! bytes in front of .NET's first write, which the whole-archive answer does not give; al-runner issue 9351), another
//! writer, start, end or length in part 3, a .NET append with no AL change before the Dispose. A change of the Blob
//! between a fresh archive's constructor and its Dispose refuses: the seed was decided at the constructor (main passed
//! those shapes with an archive that never read the Blob; al-runner issue 9318).
use super::{Bridge, Session};
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::stream::{BlobBuf, BlobRef};
use al_builtins::value::{DotNetKind, Value};
use al_host::dotnet::Rewrite;
use std::rc::Rc;
use std::sync::Arc;

/// The newest .NET write that reached a Blob through a bridge, for part 2's states (one per Blob), with the bridge's stream
/// stamp and constructor (`Bridge::opened_at`, `Bridge::update4_utf8`).
pub(super) struct NetWrite {
    blob: BlobRef,
    edits_before: u32,
    len_before: usize,
    edits_after: u32,
    replace: bool,
    opened_at: Option<u32>,
    update4_utf8: bool,
}

impl NetWrite {
    /// The write `bridge` is about to make (a rewrite when `replace`): its Blob's state before it.
    pub(super) fn before(bridge: &Bridge, replace: bool) -> Self {
        let buf = bridge.blob.borrow();
        NetWrite {
            blob: bridge.blob.clone(),
            edits_before: buf.content_edits,
            len_before: buf.bytes.len(),
            edits_after: buf.content_edits,
            replace,
            opened_at: bridge.opened_at,
            update4_utf8: bridge.update4_utf8,
        }
    }
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
    /// The Blob takes the whole new content (the pin).
    Replace,
    /// The Blob takes the whole new content under `zip-blob-writes`; a later write marks it (`BlobBuf::dotnet_rewritten`).
    ReplaceAssumed,
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

/// A copy of OutStream `v` was made: its stamp is dropped. The probes hand .NET a scalar OutStream variable that
/// `CreateOutStream` wrote, directly (ZipBlobUpdateFreshOutStream.al:146-170) or through a `var` parameter bound to a local
/// (ZipBlobUpdateBlobChanged.al:19-32, ZipBlobDotNetOverwrite.al:17-34), never a copy. The callers: `:=` of an OutStream
/// (`exec::run`), a by-value OutStream parameter and a procedure's return value, named or by `exit` (`Session::call_inner`). The runner's copy shares the stream cell, so the
/// original loses the stamp too: parts 1, 2 and 3 then give main's answer (`fresh_update_seed` before the bridge,
/// [`still_stamped`] in `rewrite_verdict` after it).
pub(crate) fn unstamp_out_stream(v: &Value) {
    if let Value::Library(al_builtins::library_kind::LibraryKind::OutStream, h) = v
        && let al_builtins::value::LibraryValue::OutStream(.., opened_at) = &mut *h.borrow_mut()
    {
        *opened_at = None;
    }
}

/// OutStream `v` was read from an array element (`Session::read_place`): its cell is noted for the session, so the stream
/// is never stamped when it reaches .NET ([`stamp_of`]), also when a later `CreateOutStream` writes the cell through a `var`
/// parameter bound to the element. No probe holds the stream in an array. Every array of OutStreams, copied whole or handed
/// by value, shares its elements' cells, and an element reaches .NET only by a read of it (or through a parameter bound by
/// that read).
pub(crate) fn note_element_stream(state: &mut super::DotNetState, v: &Value) {
    if let Value::Library(al_builtins::library_kind::LibraryKind::OutStream, h) = v {
        state.element_streams.note(h);
    }
}

type StreamCell = Rc<std::cell::RefCell<al_builtins::value::LibraryValue>>;

/// The OutStream cells [`note_element_stream`] noted, by address. Each entry holds a `Weak` of its cell, which keeps the
/// cell's allocation, so no other cell takes that address while the entry stays. Cells every holder dropped are pruned
/// when the map has doubled since the last prune, so a note and a lookup take constant time on average.
#[derive(Default)]
pub(super) struct ElementStreams {
    cells: std::collections::HashMap<usize, std::rc::Weak<std::cell::RefCell<al_builtins::value::LibraryValue>>>,
    prune_at: usize,
}

impl ElementStreams {
    fn note(&mut self, h: &StreamCell) {
        if self.cells.insert(Rc::as_ptr(h) as usize, Rc::downgrade(h)).is_none() && self.cells.len() >= self.prune_at {
            self.cells.retain(|_, w| w.strong_count() > 0);
            self.prune_at = 2 * self.cells.len().max(8);
        }
    }

    fn contains(&self, h: &StreamCell) -> bool {
        self.cells.contains_key(&(Rc::as_ptr(h) as usize))
    }

    /// No noted cell is still held (a map of gone cells answers as an empty one: `DotNetState::as_new`).
    pub(super) fn all_gone(&self) -> bool {
        self.cells.values().all(|w| w.strong_count() == 0)
    }
}

/// The bridge's stream still is the open it was when the bridge was made: the cell is on the bridge's Blob with the same
/// stamp, so no copy of it was made since, it was not opened again (a stamp counts the edits of one Blob only, so a
/// `CreateOutStream` on another Blob can give the same number) and it was not read from an array element. A stream every
/// holder dropped answers no: whether a copy was made is not known.
fn still_stamped(state: &super::DotNetState, bridge: &Bridge) -> bool {
    let Some(cell) = bridge.cell.upgrade() else {
        return false;
    };
    let opened_at = match &*cell.borrow() {
        al_builtins::value::LibraryValue::OutStream(blob, .., opened_at) if Arc::ptr_eq(blob, &bridge.blob) => {
            *opened_at
        }
        _ => None,
    };
    bridge.opened_at.is_some() && stamp_of(state, &cell, opened_at) == bridge.opened_at
}

/// The key of `blob` in `DotNetState::net_writes`: its address, unique while the entry holds the Blob.
pub(super) fn blob_key(blob: &BlobRef) -> usize {
    Arc::as_ptr(blob).cast::<()>() as usize
}

/// The stamp of the OutStream cell `h` handed to .NET: `opened_at`, or none for a cell read from an array element.
pub(super) fn stamp_of(state: &super::DotNetState, h: &StreamCell, opened_at: Option<u32>) -> Option<u32> {
    opened_at.filter(|_| !state.element_streams.contains(h))
}

/// Part 2's Blob: no read or Insert gave the record that Blob. Read when the bridge is made: a Clear in place gives the
/// Blob a fresh buffer, which resets these flags.
pub(super) fn of_no_row(buf: &BlobBuf) -> bool {
    !buf.db_row && !buf.temp_row && !buf.inserted && !buf.temp_inserted
}

impl Session {
    /// Notes a .NET append or rewrite ([`NetWrite::before`]) once it reached its Blob (part 2 reads the newest one).
    pub(super) fn note_net_write(&mut self, mut write: NetWrite) {
        write.edits_after = write.blob.borrow().content_edits;
        self.dotnet.net_writes.insert(blob_key(&write.blob), write);
    }

    /// The verdict on the rewrite of `bridge`, whose Blob held `before` bytes at the bridge's last reply.
    pub(super) fn rewrite_verdict(&self, bridge: &Bridge, before: usize, kind: Option<Rewrite>) -> Verdict {
        let buf = bridge.blob.borrow();
        let unchanged = buf.content_edits == bridge.edits && buf.bytes.len() == before && buf.render_pending.is_none();
        let stamped = still_stamped(&self.dotnet, bridge);
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
            Some(Rewrite::ZipUpdate4 | Rewrite::ZipUpdate)
                if bridge.fresh && stamped && unchanged && buf.write_pos == 0 =>
            {
                assume(Assumption::ZipBlobWrites);
                Verdict::ReplaceAssumed
            }
            // Part 2.
            Some(Rewrite::ZipUpdate4)
                if !bridge.fresh && stamped && self.changed_state_measured(bridge, before, &buf) =>
            {
                assume(Assumption::ZipBlobWrites);
                Verdict::ReplaceAssumed
            }
            // Part 3: the bridge is the one AL handed its Blob to (the sidecar counts the writers of one bridge only).
            Some(Rewrite::Overwrite)
                if bridge.origin_empty && stamped && unchanged && self.bridges_of(&bridge.blob) == 1 =>
            {
                assume(Assumption::ZipBlobWrites);
                Verdict::ReplaceAssumed
            }
            Some(Rewrite::Overwrite) => Verdict::Mark,
            _ => Verdict::Refuse,
        }
    }

    /// How many bridges AL made over `blob` in this session.
    fn bridges_of(&self, blob: &BlobRef) -> usize {
        self.dotnet
            .bridges
            .values()
            .filter(|b| Arc::ptr_eq(&b.blob, blob))
            .count()
    }

    /// Part 2: the archive `ZipArchive(OutStream, Update, false, Encoding.UTF8)`, a Blob of a temporary record (the
    /// stream opened on it, no read or Insert gave it the Blob when the bridge was made), cleared in place, no mark, in one
    /// measured state.
    fn changed_state_measured(&self, bridge: &Bridge, before: usize, buf: &BlobBuf) -> bool {
        if !(bridge.update4_utf8
            && bridge.temp_record
            && bridge.of_no_row
            && buf.render_pending.is_none()
            && al_builtins::native::blob_clears_in_place(self, &bridge.blob))
        {
            return false;
        }
        let newest = self.dotnet.net_writes.get(&blob_key(&bridge.blob));
        let net_newest = newest.filter(|w| w.edits_after == buf.content_edits);
        // :98 Clear only: the first AL change since the bridge was made is a Clear of the Blob, and the newest change is
        // AL's (a Clear in place sets `cleared_at` to the edit count it leaves: al-builtins `native::values`).
        let cleared = buf.line_breaks.cleared_at();
        match net_newest {
            None => cleared == bridge.edits.wrapping_add(1) && buf.content_edits == cleared && buf.bytes.is_empty(),
            // :152 another `ZipArchive(OutStream, Update, false, Encoding.UTF8)` over the same OutStream (one CreateOutStream
            // made both streams: the stamps are equal) that read the same bytes, its Dispose first, no AL change before it.
            Some(w) if w.replace => {
                w.update4_utf8
                    && w.opened_at.is_some_and(|stamp| bridge.opened_at == Some(stamp))
                    && w.len_before == before
                    && w.edits_before == bridge.edits
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use al_builtins::value::LibraryValue;

    fn cell() -> StreamCell {
        Rc::new(std::cell::RefCell::new(LibraryValue::OutStream(
            Arc::new(al_builtins::stream::BlobCell::new(BlobBuf::default())),
            al_builtins::stream::Encoding::Utf8,
            None,
            Some(1),
        )))
    }

    #[test]
    fn a_noted_cell_stays_noted_while_held_and_dropped_cells_are_pruned() {
        let mut noted = ElementStreams::default();
        let kept = cell();
        noted.note(&kept);
        for _ in 0..100 {
            noted.note(&cell());
        }
        assert!(noted.contains(&kept));
        assert!(!noted.contains(&cell()));
        assert!(
            noted.cells.len() < 100,
            "dropped cells are pruned: {}",
            noted.cells.len()
        );
        assert!(!noted.all_gone());
        drop(kept);
        assert!(noted.all_gone());
    }
}
