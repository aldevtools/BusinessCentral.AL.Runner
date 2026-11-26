//! Maps SQL Server 8 KB pages inside a native full backup (`.bak`) to their file offsets
//! by parsing the backup's structure, not by trusting page self-identification (port of
//! `PageFile.cs`).
//!
//! How a full backup lays out page data (derived from the BC demo backups and validated
//! block-for-block against a fresh RESTORE of the same files, bcdb PROVENANCE.md
//! "Data-copy layout"):
//!
//!  * The first MSDA/MQDA region holds every GAM-allocated extent of the data file, in
//!    extent order: 8 consecutive blocks per extent. Which extents exist comes from the
//!    GAM page (1:2), which sits at a knowable position (block 2) because extent 0 is
//!    always allocated and leads the region. A GAM interval covers 63,904 extents; files
//!    larger than one interval continue with the next interval's GAM page.
//!  * The second MSDA/MQDA region (if present) re-dumps the extents that changed while the
//!    backup was running. RESTORE applies regions in order, so region 2 supersedes region 1.
//!  * Each region is padded to a 1 MB boundary with filler pseudo-pages (header bytes
//!    01 65, page id 0) that RESTORE discards. The reader verifies every block beyond the
//!    derived extent list is such a filler block.
//!  * The MSTL/MQTL transaction-log region is NOT replayed. Validated consequence on the
//!    demo backups: only allocation bookkeeping pages and a handful of system-table rows
//!    written during the backup differ from a real RESTORE; no BC table data page does.
//!
//! Page self-identification (m_pageId in the header) is deliberately not used to build the
//! map: deallocated pages keep their old headers, so a stale image elsewhere in the file
//! can carry the same page id as a live page. It is kept as a cross-check only.
//!
//! `IndexStore`, `open_indexed` (a kept page map, checked against the file's length and data
//! regions). Entry points: `Page`, `PageFile`, `CrossCheck`.

use crate::error::Result;
use crate::io::{ByteSource, le};
use crate::mtf::{DataRegion, MtfFile, PAGE_SIZE};
use std::borrow::Cow;
use std::collections::HashSet;
use std::sync::OnceLock;

/// Extents covered by one GAM page: 511,232 pages / 8.
pub const GAM_INTERVAL_EXTENTS: usize = 63_904;
pub const PAGES_PER_EXTENT: usize = 8;
pub const GAM_INTERVAL_PAGES: usize = GAM_INTERVAL_EXTENTS * PAGES_PER_EXTENT;
/// PFS interval: one PFS page per 8088 pages.
pub const PFS_INTERVAL: usize = 8088;

const PT_GAM: u8 = 8;
const PT_SGAM: u8 = 9;
const PT_DCM: u8 = 16;
const PT_FILE_HEADER: u8 = 15;
const PT_FILLER: u8 = 0x65;
const KNOWN_PAGE_TYPES: [u8; 16] = [1, 2, 3, 4, 8, 9, 10, 11, 13, 14, 15, 16, 17, 18, 19, 20];
const NONE: u64 = u64::MAX;

/// One 8 KB page: a borrow of the file where the source is memory, else an owned copy.
pub type Page<'a> = Cow<'a, [u8]>;

pub struct PageFile {
    src: Box<dyn ByteSource>,
    mtf: MtfFile,
    /// page id (file 1) -> file offset, `NONE` where unmapped
    map: Vec<u64>,
    mapped: usize,
    superseded: usize,
    gam_intervals: usize,
    file_pages: usize,
    pfs: Vec<OnceLock<Box<[u8]>>>,
}

fn is_filler(b: &[u8]) -> bool {
    b[0] == 1 && b[1] == PT_FILLER
}

fn expect_page(b: &[u8], page_id: i32, ty: u8, what: &str) -> Result<()> {
    let pid = le::i32(b, 32);
    let fid = le::u16(b, 36);
    if b[0] != 1 || b[1] != ty || pid != page_id || fid != 1 {
        return Err(crate::invalid!(
            "expected {what} (page 1:{page_id}, type {ty}) but found headerVersion={} type={} page {fid}:{pid} — backup layout differs from every observed file, refusing to guess",
            b[0],
            b[1]
        ));
    }
    Ok(())
}

/// The extent bitmap of a GAM/SGAM/DCM/BCM page: two slots, slot 1 =
/// `[2 status bytes][u16 bitmap byte length][bitmap]`, LSB-first (bit n of byte k = extent
/// 8k+n relative to the page's interval).
fn alloc_bitmap(page: &[u8]) -> Result<Vec<u8>> {
    let slot_cnt = le::u16(page, 22);
    if slot_cnt != 2 {
        return Err(crate::invalid!("allocation page with {slot_cnt} slots (expected 2)"));
    }
    let s1 = le::u16(page, PAGE_SIZE - 4) as usize;
    if s1 < 96 || s1 + 4 > PAGE_SIZE {
        return Err(crate::invalid!("allocation page slot offset {s1} out of range"));
    }
    let len = le::u16(page, s1 + 2) as usize;
    if s1 + 4 + len > PAGE_SIZE {
        return Err(crate::invalid!("allocation bitmap of {len} bytes runs past the page"));
    }
    Ok(page[s1 + 4..s1 + 4 + len].to_vec())
}

fn bit_set(bmp: &[u8], i: usize) -> bool {
    bmp[i / 8] & (1 << (i % 8)) != 0
}

fn contains_pfs_page(extent_first_page: usize) -> bool {
    (0..PAGES_PER_EXTENT).any(|p| (extent_first_page + p) % PFS_INTERVAL == 0)
}

/// Result of [`PageFile::cross_check`].
#[derive(Debug)]
pub struct CrossCheck {
    pub agree: u64,
    pub stale_headers: u64,
    pub unidentified: u64,
    /// pages where "last image wins" would have picked a different image than the map
    pub last_wins_disagreements: Vec<i32>,
}

/// Where a caller keeps the page-map index of a backup between runs. The core does no OS access
/// (ADR-006); a command line implements this over a file.
pub trait IndexStore: Send + Sync {
    /// The bytes saved by an earlier [`save`](IndexStore::save), if any.
    fn load(&self) -> Option<Vec<u8>>;
    /// Keeps the bytes; a failure to keep them is the store's to ignore.
    fn save(&self, bytes: &[u8]);
}

const INDEX_MAGIC: &[u8; 8] = b"ALBXPMAP";
/// 2: the map ends with its checksum (al-runner issue 8479). A map of version 1 is rebuilt.
const INDEX_VERSION: u32 = 2;

/// A hash that notices truncation and flipped bytes; not a defence against an adversary (the
/// index lives in the user's own cache). The one checksum of a kept index: the page map's and the
/// catalog section's ([`crate::bak`]).
pub(crate) fn index_hash(b: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325 ^ b.len() as u64;
    let mut chunks = b.chunks_exact(8);
    for c in &mut chunks {
        h = (h ^ u64::from_le_bytes(c.try_into().unwrap()))
            .wrapping_mul(0x0100_0000_01b3)
            .rotate_left(29);
    }
    for &x in chunks.remainder() {
        h = (h ^ u64::from(x)).wrapping_mul(0x0100_0000_01b3);
    }
    h
}

impl PageFile {
    pub fn open(src: Box<dyn ByteSource>) -> Result<PageFile> {
        Self::open_with(src, None)
    }

    /// Opens the backup, taking the page map from `store` when it holds one made for this very
    /// file (same length, same data regions). Building the map verifies the lead block of every
    /// extent, one small read each, which on a cold 900 MB backup costs seconds; a saved map
    /// costs one read of about a megabyte. A map that does not fit the file, or is damaged, is
    /// ignored, rebuilt and saved again.
    pub fn open_indexed(src: Box<dyn ByteSource>, store: &dyn IndexStore) -> Result<PageFile> {
        Self::open_with(src, Some(store))
    }

    fn open_with(src: Box<dyn ByteSource>, store: Option<&dyn IndexStore>) -> Result<PageFile> {
        let mtf = MtfFile::parse(&*src)?;
        let mut pf = PageFile {
            src,
            mtf,
            map: Vec::new(),
            mapped: 0,
            superseded: 0,
            gam_intervals: 0,
            file_pages: 0,
            pfs: Vec::new(),
        };
        let restored = store
            .and_then(|s| s.load())
            .is_some_and(|bytes| pf.restore_index(&bytes));
        if !restored {
            pf.build_map()?;
            if let Some(s) = store {
                s.save(&pf.index_bytes());
            }
        }
        let n = pf.file_pages / PFS_INTERVAL + 2;
        pf.pfs = (0..n).map(|_| OnceLock::new()).collect();
        Ok(pf)
    }

    /// The page map as bytes: a header naming the file it was made for (length and data
    /// regions), the counts, one little-endian offset per page id, then the [`index_hash`] of all
    /// the bytes before it.
    fn index_bytes(&self) -> Vec<u8> {
        let mut b = Vec::with_capacity(104 + self.map.len() * 8);
        b.extend_from_slice(INDEX_MAGIC);
        b.extend_from_slice(&INDEX_VERSION.to_le_bytes());
        b.extend_from_slice(&self.src.len().to_le_bytes());
        b.extend_from_slice(&(self.mtf.mqda_regions.len() as u32).to_le_bytes());
        for r in &self.mtf.mqda_regions {
            b.extend_from_slice(&r.data_offset.to_le_bytes());
            b.extend_from_slice(&r.block_count.to_le_bytes());
        }
        for v in [
            self.mapped,
            self.superseded,
            self.gam_intervals,
            self.file_pages,
            self.map.len(),
        ] {
            b.extend_from_slice(&(v as u64).to_le_bytes());
        }
        for &o in &self.map {
            b.extend_from_slice(&o.to_le_bytes());
        }
        let hash = index_hash(&b);
        b.extend_from_slice(&hash.to_le_bytes());
        b
    }

    /// Takes the page map from `bytes` when they are the bytes saved (their checksum holds) and
    /// were made for this file; false otherwise (and the page file is left as it was). An offset
    /// is not checked against the page it names, so a map whose checksum fails is never read: a
    /// changed offset that still lies inside the file reads another page (al-runner issue 8479).
    fn restore_index(&mut self, bytes: &[u8]) -> bool {
        let Some((bytes, hash)) = bytes.split_last_chunk::<8>() else {
            return false;
        };
        if index_hash(bytes) != u64::from_le_bytes(*hash) {
            return false;
        }
        let at = std::cell::Cell::new(0usize);
        let take = |n: usize| -> Option<&[u8]> {
            let from = at.get();
            let s = bytes.get(from..from.checked_add(n)?)?;
            at.set(from + n);
            Some(s)
        };
        let ok = (|| -> Option<(Vec<u64>, [usize; 4])> {
            if take(8)? != INDEX_MAGIC || u32::from_le_bytes(take(4)?.try_into().ok()?) != INDEX_VERSION {
                return None;
            }
            if u64::from_le_bytes(take(8)?.try_into().ok()?) != self.src.len() {
                return None;
            }
            let regions = u32::from_le_bytes(take(4)?.try_into().ok()?) as usize;
            if regions != self.mtf.mqda_regions.len() {
                return None;
            }
            for r in &self.mtf.mqda_regions {
                let off = u64::from_le_bytes(take(8)?.try_into().ok()?);
                let blocks = u64::from_le_bytes(take(8)?.try_into().ok()?);
                if off != r.data_offset || blocks != r.block_count {
                    return None;
                }
            }
            let mut counts = [0usize; 5];
            for c in &mut counts {
                *c = usize::try_from(u64::from_le_bytes(take(8)?.try_into().ok()?)).ok()?;
            }
            let n = counts[4];
            if bytes.len().checked_sub(at.get())? != n.checked_mul(8)? {
                return None;
            }
            let len = self.src.len();
            let mut map = Vec::with_capacity(n);
            let mut mapped = 0usize;
            for chunk in take(n * 8)?.chunks_exact(8) {
                let o = u64::from_le_bytes(chunk.try_into().ok()?);
                if o != NONE {
                    if o.checked_add(PAGE_SIZE as u64)? > len {
                        return None;
                    }
                    mapped += 1;
                }
                map.push(o);
            }
            if mapped != counts[0] {
                return None;
            }
            Some((map, [counts[0], counts[1], counts[2], counts[3]]))
        })();
        match ok {
            Some((map, [mapped, superseded, gam_intervals, file_pages])) => {
                self.map = map;
                self.mapped = mapped;
                self.superseded = superseded;
                self.gam_intervals = gam_intervals;
                self.file_pages = file_pages;
                true
            }
            None => false,
        }
    }

    pub fn mtf(&self) -> &MtfFile {
        &self.mtf
    }
    pub fn page_count(&self) -> usize {
        self.mapped
    }
    /// Pages whose region-1 image was replaced by the changed-extent re-read.
    pub fn superseded_page_count(&self) -> usize {
        self.superseded
    }
    pub fn gam_interval_count(&self) -> usize {
        self.gam_intervals
    }
    /// The file's length in pages: the header's Size or the end of the copied data,
    /// whichever is larger.
    pub fn file_pages(&self) -> usize {
        self.file_pages
    }

    fn read_block(&self, r: &DataRegion, block: u64) -> Result<Page<'_>> {
        if block >= r.block_count {
            return Err(crate::invalid!(
                "block {block} beyond region ({} blocks)",
                r.block_count
            ));
        }
        self.src.slice(r.data_offset + block * PAGE_SIZE as u64, PAGE_SIZE)
    }

    fn set(&mut self, pid: usize, off: u64) {
        if pid >= self.map.len() {
            self.map.resize(pid + 1, NONE);
        }
        if self.map[pid] == NONE {
            self.mapped += 1;
        }
        self.map[pid] = off;
    }

    fn is_mapped(&self, pid: usize) -> bool {
        self.map.get(pid).is_some_and(|&o| o != NONE)
    }

    fn build_map(&mut self) -> Result<()> {
        if self.mtf.mqda_regions.len() > 2 {
            return Err(crate::unsupported!(
                "{} MQDA data regions — only the full-copy + changed-extent-re-read shape (1 or 2 regions) has been derived and validated",
                self.mtf.mqda_regions.len()
            ));
        }
        let r0 = self.mtf.mqda_regions[0].clone();

        // Bootstrap: extent 0 always leads region 1, so block 0 is the file header page.
        let hdr_page = self.read_block(&r0, 0)?.into_owned();
        expect_page(&hdr_page, 0, PT_FILE_HEADER, "file header page")?;
        // The file header data is a FixedVar record at page offset 96 whose field count
        // varies with the SQL Server version that wrote the file (observed: 56 and 60
        // fields). "Size" (in pages) is variable-length column 4.
        let var_cols = crate::records::fixed_var_var_columns(&hdr_page, 96)?;
        if var_cols.len() < 5 || var_cols[4].1 < 4 {
            return Err(crate::invalid!(
                "file header record has {} variable columns — Size field not where every observed layout puts it, refusing to guess",
                var_cols.len()
            ));
        }
        let header_size_pages = le::i32(&hdr_page, var_cols[4].0);
        if header_size_pages <= 0 || header_size_pages % PAGES_PER_EXTENT as i32 != 0 {
            return Err(crate::invalid!(
                "file header Size field reads {header_size_pages} pages — not a positive multiple of 8, refusing to guess"
            ));
        }
        let header_size_pages = header_size_pages as usize;
        // The header's Size is a LOWER bound on the file, not the end of the copied data
        // (bcdb PROVENANCE.md "File size is a lower bound"). FilePages is derived below
        // from the copy itself.
        self.file_pages = header_size_pages;

        // --- Region 0: the full copy. Per GAM interval, in extent order, the stream holds
        // every extent that is GAM-allocated, contains a PFS page, is the interval's first
        // extent, or is SGAM-marked mixed-with-free-pages.
        let mut cursor: u64 = 0;
        let mut derived_pages = 0usize;
        let mut exhausted = false;
        let intervals = header_size_pages
            .max(r0.block_count as usize)
            .div_ceil(GAM_INTERVAL_PAGES)
            .max(1);
        for k in 0..intervals {
            let lead_block = cursor;
            // Interval 0: GAM at page 2 / SGAM at 3. Interval k>0: GAM at the interval's
            // first page, SGAM next.
            let gam_pid = if k == 0 { 2 } else { k * GAM_INTERVAL_PAGES };
            let gam_page = self.read_block(&r0, lead_block + if k == 0 { 2 } else { 0 })?;
            expect_page(&gam_page, gam_pid as i32, PT_GAM, &format!("GAM page of interval {k}"))?;
            let sgam_page = self.read_block(&r0, lead_block + if k == 0 { 3 } else { 1 })?;
            expect_page(
                &sgam_page,
                gam_pid as i32 + 1,
                PT_SGAM,
                &format!("SGAM page of interval {k}"),
            )?;
            let dcm_page = self.read_block(&r0, lead_block + 6)?;
            expect_page(
                &dcm_page,
                (k * GAM_INTERVAL_PAGES + 6) as i32,
                PT_DCM,
                &format!("DCM page of interval {k}"),
            )?;
            let gam = alloc_bitmap(&gam_page)?;
            let sgam = alloc_bitmap(&sgam_page)?;
            drop((gam_page, sgam_page, dcm_page));
            let mut placements: Vec<(usize, u64)> = Vec::new();
            for e in 0..GAM_INTERVAL_EXTENTS {
                let first_page = k * GAM_INTERVAL_PAGES + e * PAGES_PER_EXTENT;
                let in_stream = e == 0 // interval lead extent
                    || !(e < gam.len() * 8 && bit_set(&gam, e)) // GAM bit clear = allocated
                    || (e < sgam.len() * 8 && bit_set(&sgam, e)) // mixed extent with free pages
                    || contains_pfs_page(first_page); // PFS pages are always copied
                if !in_stream {
                    continue;
                }
                if cursor + PAGES_PER_EXTENT as u64 > r0.block_count {
                    exhausted = true;
                    break;
                }
                // End of the copied data: the first proposed extent whose lead block is
                // filler IS that end (page type 0x65 is not a SQL page type).
                // Only the 40 header bytes are needed, so only they are read.
                let lead = self.src.slice(r0.data_offset + cursor * PAGE_SIZE as u64, 40)?;
                if is_filler(&lead) {
                    exhausted = true;
                    break;
                }
                // Self-check the structural map rather than trusting GAM order alone: the
                // lead block of every mapped extent must carry that extent's own first page.
                let lead_pid = le::i32(&lead, 32);
                let lead_fid = le::u16(&lead, 36);
                if lead[0] != 1 || lead_fid != 1 || lead_pid != first_page as i32 {
                    return Err(crate::invalid!(
                        "block {cursor} of the {} region should lead extent {} (page 1:{first_page}) per the GAM, but carries page {lead_fid}:{lead_pid} — backup layout differs from the derived model, refusing to guess",
                        r0.dblk,
                        first_page / PAGES_PER_EXTENT
                    ));
                }
                drop(lead);
                placements.push((first_page, cursor));
                cursor += PAGES_PER_EXTENT as u64;
                derived_pages = derived_pages.max(first_page + PAGES_PER_EXTENT);
            }
            for (first_page, c) in placements {
                for p in 0..PAGES_PER_EXTENT {
                    self.set(first_page + p, r0.data_offset + (c + p as u64) * PAGE_SIZE as u64);
                }
            }
            if exhausted {
                break;
            }
        }
        self.gam_intervals = intervals;
        // The file is at least as long as the last page the copy actually carries; the
        // header may legitimately claim more (free space at the end of the file).
        self.file_pages = header_size_pages.max(derived_pages);
        self.verify_filler_tail(&r0, cursor)?;

        // --- Region 2 (optional): the extents that changed while the backup ran, written as
        // 8-block extent frames in ascending extent order. WHICH extents is not recorded in
        // any single on-disk structure, so the frame extent is chosen by consensus of
        // frame-aligned page headers, constrained by (a) strictly ascending frame order and
        // (b) membership in the interval's FINAL DCM image (read from the interval's
        // lead-extent frame in this region) or being a lead extent itself. Exactly one
        // candidate may satisfy both. Validated byte-for-byte against fresh RESTOREs of
        // five backups (bcdb PROVENANCE.md "Re-read region").
        if self.mtf.mqda_regions.len() == 2 {
            let r1 = self.mtf.mqda_regions[1].clone();
            let mut c1: u64 = 0;
            let mut prev_extent: i64 = -1;
            let mut current_interval: i64 = -1;
            let mut dcm_final: Option<Vec<u8>> = None;
            while c1 + PAGES_PER_EXTENT as u64 <= r1.block_count {
                // whole 64 KB extent frame in one positional read
                let frame = self
                    .src
                    .slice(r1.data_offset + c1 * PAGE_SIZE as u64, PAGES_PER_EXTENT * PAGE_SIZE)?;
                let pg = |p: usize| &frame[p * PAGE_SIZE..(p + 1) * PAGE_SIZE];
                let mut candidates: HashSet<i64> = HashSet::new();
                for p in 0..PAGES_PER_EXTENT {
                    let page = pg(p);
                    if page[0] != 1 || !KNOWN_PAGE_TYPES.contains(&page[1]) {
                        continue;
                    }
                    let pid = le::i32(page, 32);
                    let fid = le::u16(page, 36);
                    if fid != 1 || pid < 0 || pid as usize >= self.file_pages {
                        continue;
                    }
                    if pid as usize % PAGES_PER_EXTENT != p {
                        continue; // not frame-aligned: no vote
                    }
                    candidates.insert(pid as i64 / PAGES_PER_EXTENT as i64);
                }
                if candidates.is_empty() {
                    if (0..PAGES_PER_EXTENT).all(|p| is_filler(pg(p))) {
                        break; // trailer padding
                    }
                    return Err(crate::invalid!(
                        "re-read frame at block {c1} has no frame-aligned page identity and is not padding — cannot place, refusing to guess"
                    ));
                }
                let mut valid: Vec<i64> = Vec::new();
                for &e in &candidates {
                    if e <= prev_extent {
                        continue;
                    }
                    let iv = e / GAM_INTERVAL_EXTENTS as i64;
                    let is_lead = (iv == 0 && e <= 1) || e == iv * GAM_INTERVAL_EXTENTS as i64;
                    let in_dcm = iv == current_interval
                        && dcm_final.as_ref().is_some_and(|d| {
                            let rel = (e - iv * GAM_INTERVAL_EXTENTS as i64) as usize;
                            rel < d.len() * 8 && bit_set(d, rel)
                        });
                    if is_lead || in_dcm {
                        valid.push(e);
                    }
                }
                if valid.len() != 1 {
                    let mut cs: Vec<i64> = candidates.iter().copied().collect();
                    cs.sort_unstable();
                    return Err(crate::invalid!(
                        "re-read frame at block {c1}: {} placement candidates ({}) survive the ascending and changed-map constraints — refusing to guess",
                        valid.len(),
                        cs.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")
                    ));
                }
                let extent = valid[0];
                let interval = extent / GAM_INTERVAL_EXTENTS as i64;
                if interval != current_interval {
                    // First frame of an interval is its lead extent; its slot 6 carries the
                    // interval's final DCM image, needed to admit that interval's other frames.
                    if extent != interval * GAM_INTERVAL_EXTENTS as i64 {
                        return Err(crate::invalid!(
                            "re-read section of interval {interval} does not start with its lead extent — refusing to guess"
                        ));
                    }
                    let dcm_page = pg(6);
                    expect_page(
                        dcm_page,
                        (interval * GAM_INTERVAL_PAGES as i64 + 6) as i32,
                        PT_DCM,
                        &format!("final DCM page of interval {interval}"),
                    )?;
                    dcm_final = Some(alloc_bitmap(dcm_page)?);
                    current_interval = interval;
                }
                drop(frame);
                for p in 0..PAGES_PER_EXTENT {
                    let pid = extent as usize * PAGES_PER_EXTENT + p;
                    if self.is_mapped(pid) {
                        self.superseded += 1;
                    }
                    self.set(pid, r1.data_offset + (c1 + p as u64) * PAGE_SIZE as u64);
                }
                prev_extent = extent;
                c1 += PAGES_PER_EXTENT as u64;
            }
            self.verify_filler_tail(&r1, c1)?;
        }
        Ok(())
    }

    fn verify_filler_tail(&self, r: &DataRegion, from: u64) -> Result<()> {
        // The tail is contiguous: scan it in large chunks, one filler check per block.
        let mut b = from;
        while b < r.block_count {
            let blocks = (256u64).min(r.block_count - b) as usize;
            let chunk = self
                .src
                .slice(r.data_offset + b * PAGE_SIZE as u64, blocks * PAGE_SIZE)?;
            for i in 0..blocks {
                if !is_filler(&chunk[i * PAGE_SIZE..]) {
                    return Err(crate::invalid!(
                        "block {} of {} region is neither mapped by the derived extent list nor padding filler — backup layout differs from the derived model, refusing to guess",
                        b + i as u64,
                        r.dblk
                    ));
                }
            }
            b += blocks as u64;
        }
        Ok(())
    }

    /// Whether a page is individually allocated per the PFS byte map. IAM/GAM bits track
    /// whole extents; pages inside an extent can be individually deallocated and then keep
    /// a stale image in the backup, so readers must filter on this bit. PFS layout: page
    /// 1:1 covers pages 0..8087, then one PFS page every 8088 pages; single record at slot
    /// 0, byte array at record+4, one byte per page, bit 0x40 = allocated (bcdb
    /// PROVENANCE.md "PFS pages").
    pub fn is_page_allocated(&self, page_id: i32) -> Result<bool> {
        if page_id < 0 {
            return Err(crate::invalid!("page id {page_id} is negative"));
        }
        let pid = page_id as usize;
        let base_idx = pid / PFS_INTERVAL;
        let base = base_idx * PFS_INTERVAL;
        let load = || -> Result<Box<[u8]>> {
            let pfs_pid = if base == 0 { 1 } else { base };
            let pfs = self.page(1, pfs_pid as i32)?;
            if pfs[1] != 11 {
                return Err(crate::invalid!("page 1:{pfs_pid} is not a PFS page (type {})", pfs[1]));
            }
            let s0 = le::u16(&pfs, PAGE_SIZE - 2) as usize;
            if s0 + 4 + PFS_INTERVAL > PAGE_SIZE {
                return Err(crate::invalid!("PFS page 1:{pfs_pid} slot offset {s0} out of range"));
            }
            Ok(pfs[s0 + 4..s0 + 4 + PFS_INTERVAL].to_vec().into_boxed_slice())
        };
        match self.pfs.get(base_idx) {
            Some(cell) => {
                if let Some(d) = cell.get() {
                    return Ok(d[pid - base] & 0x40 != 0);
                }
                let d = load()?;
                let v = d[pid - base] & 0x40 != 0;
                let _ = cell.set(d);
                Ok(v)
            }
            None => Ok(load()?[pid - base] & 0x40 != 0),
        }
    }

    /// The page, or `None` when the backup carries no such page.
    pub fn try_page(&self, file_id: i32, page_id: i32) -> Result<Option<Page<'_>>> {
        if file_id != 1 {
            return Err(crate::unsupported!(
                "page reference to file {file_id}: only single-data-file databases are supported (BC databases use one data file)"
            ));
        }
        if page_id < 0 {
            return Ok(None);
        }
        match self.map.get(page_id as usize) {
            Some(&off) if off != NONE => Ok(Some(self.src.slice(off, PAGE_SIZE)?)),
            _ => Ok(None),
        }
    }

    pub fn page(&self, file_id: i32, page_id: i32) -> Result<Page<'_>> {
        self.try_page(file_id, page_id)?
            .ok_or_else(|| crate::invalid!("page ({file_id}:{page_id}) is not an allocated page of this backup"))
    }

    /// Cross-check the structural map against page self-identification, the empirical
    /// method the map replaced. Scans every block of every data region.
    pub fn cross_check(&self) -> Result<CrossCheck> {
        let (mut agree, mut stale, mut unident) = (0u64, 0u64, 0u64);
        let mut last_wins: std::collections::HashMap<i32, u64> = std::collections::HashMap::new();
        for r in &self.mtf.mqda_regions {
            let mut b = 0u64;
            while b < r.block_count {
                let blocks = 256u64.min(r.block_count - b) as usize;
                let chunk = self
                    .src
                    .slice(r.data_offset + b * PAGE_SIZE as u64, blocks * PAGE_SIZE)?;
                for i in 0..blocks {
                    let blk = &chunk[i * PAGE_SIZE..(i + 1) * PAGE_SIZE];
                    if blk[0] == 1 && KNOWN_PAGE_TYPES.contains(&blk[1]) {
                        let pid = le::i32(blk, 32);
                        let fid = le::u16(blk, 36);
                        if fid == 1 && pid >= 0 {
                            let off = r.data_offset + (b + i as u64) * PAGE_SIZE as u64;
                            last_wins.insert(pid, off);
                            if self.map.get(pid as usize).is_some_and(|&m| m == off) {
                                agree += 1;
                            } else {
                                stale += 1;
                            }
                            continue;
                        }
                    }
                    unident += 1;
                }
                b += blocks as u64;
            }
        }
        let mut disagreements: Vec<i32> = last_wins
            .iter()
            .filter(|(pid, off)| self.map.get(**pid as usize).is_some_and(|&m| m != NONE && m != **off))
            .map(|(pid, _)| *pid)
            .collect();
        disagreements.sort_unstable();
        Ok(CrossCheck {
            agree,
            stale_headers: stale,
            unidentified: unident,
            last_wins_disagreements: disagreements,
        })
    }

    /// Compare every mapped page byte-for-byte against a restored copy of the same backup
    /// (the validation oracle): (identical, header-only differences, body-differing page ids).
    pub fn compare_against(&self, mdf: &dyn ByteSource) -> Result<(u64, u64, Vec<i32>)> {
        let (mut exact, mut hdr) = (0u64, 0u64);
        let mut body = Vec::new();
        let mut b = vec![0u8; PAGE_SIZE];
        for (pid, &off) in self.map.iter().enumerate() {
            if off == NONE {
                continue;
            }
            let a = self.src.slice(off, PAGE_SIZE)?;
            mdf.read_at(pid as u64 * PAGE_SIZE as u64, &mut b)?;
            if a[..] == b[..] {
                exact += 1;
            } else if a[96..] == b[96..] {
                hdr += 1;
            } else {
                body.push(pid as i32);
            }
        }
        Ok((exact, hdr, body))
    }
}

/// Field accessors for the 96-byte page header (offsets observed and confirmed via DBCC
/// PAGE, bcdb PROVENANCE.md "Page header offsets").
pub mod header {
    use crate::io::le;
    pub fn page_type(p: &[u8]) -> u8 {
        p[1]
    }
    /// 0x80 = compression info (CI) present
    pub fn type_flag_bits(p: &[u8]) -> u8 {
        p[2]
    }
    pub fn slot_count(p: &[u8]) -> usize {
        le::u16(p, 22) as usize
    }
    /// The slot count, refused when the slot array could not fit the page.
    pub fn checked_slot_count(p: &[u8]) -> crate::error::Result<usize> {
        let n = slot_count(p);
        if n * 2 > super::PAGE_SIZE - 96 {
            return Err(crate::invalid!("page claims {n} slots — page corrupt?"));
        }
        Ok(n)
    }
    pub fn next_page(p: &[u8]) -> (i32, u16) {
        (le::i32(p, 16), le::u16(p, 20))
    }
    pub fn ghost_records(p: &[u8]) -> u16 {
        le::u16(p, 58)
    }
    pub fn pminlen(p: &[u8]) -> usize {
        le::u16(p, 14) as usize
    }
    /// Offset of slot `s` from the slot array at the end of the page.
    pub fn slot_offset(p: &[u8], s: usize) -> usize {
        le::u16(p, super::PAGE_SIZE - 2 * (s + 1)) as usize
    }
}
