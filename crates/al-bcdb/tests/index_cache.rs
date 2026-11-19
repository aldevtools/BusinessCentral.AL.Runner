//! The page-map index of a `.bak` can be kept by the caller: a second open reads the saved map
//! instead of verifying every extent of the file (cold, that is the difference between
//! seconds and milliseconds on a 900 MB backup).

mod common;
use al_bcdb::bak::BakSource;
use al_bcdb::catalog::SqlSequence;
use al_bcdb::io::ByteSource;
use al_bcdb::pagefile::{IndexStore, PageFile};
use al_bcdb::source::read_all;
use al_bcdb::types::SysColumn;
use al_bcdb::value::Value;
use common::*;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Default)]
struct Memory {
    saved: Mutex<Option<Vec<u8>>>,
    saves: AtomicUsize,
}

impl IndexStore for Memory {
    fn load(&self) -> Option<Vec<u8>> {
        self.saved.lock().unwrap().clone()
    }
    fn save(&self, bytes: &[u8]) {
        self.saves.fetch_add(1, Ordering::SeqCst);
        *self.saved.lock().unwrap() = Some(bytes.to_vec());
    }
}

/// A byte source that counts its reads.
struct Counting {
    bytes: Vec<u8>,
    reads: std::sync::Arc<AtomicUsize>,
}

impl ByteSource for Counting {
    fn len(&self) -> u64 {
        self.bytes.len() as u64
    }
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> al_bcdb::Result<()> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        self.bytes.read_at(offset, buf)
    }
}

fn counting() -> (Box<dyn ByteSource>, std::sync::Arc<AtomicUsize>) {
    let reads = std::sync::Arc::new(AtomicUsize::new(0));
    (
        Box::new(Counting {
            bytes: fixture_bytes("typeprobe.bak"),
            reads: reads.clone(),
        }),
        reads,
    )
}

#[test]
fn the_second_open_reads_the_saved_map_and_finds_the_same_pages() {
    let store = Memory::default();
    let (src, first_reads) = counting();
    let a = PageFile::open_indexed(src, &store).unwrap();
    assert_eq!(store.saves.load(Ordering::SeqCst), 1, "the first open saves the map");
    let (src, second_reads) = counting();
    let b = PageFile::open_indexed(src, &store).unwrap();
    assert_eq!(store.saves.load(Ordering::SeqCst), 1, "the second open saves nothing");
    // The second open reads the container's framing and nothing per extent.
    let (src, framing_reads) = counting();
    al_bcdb::mtf::MtfFile::parse(&*src).unwrap();
    assert_eq!(
        second_reads.load(Ordering::SeqCst),
        framing_reads.load(Ordering::SeqCst),
        "second open: {} reads, framing alone: {}, first open: {}",
        second_reads.load(Ordering::SeqCst),
        framing_reads.load(Ordering::SeqCst),
        first_reads.load(Ordering::SeqCst)
    );
    assert_eq!(a.page_count(), b.page_count());
    assert_eq!(a.superseded_page_count(), b.superseded_page_count());
    assert_eq!(a.file_pages(), b.file_pages());
    for pid in 0..a.file_pages() as i32 {
        let (x, y) = (a.try_page(1, pid).unwrap(), b.try_page(1, pid).unwrap());
        assert_eq!(x.as_deref(), y.as_deref(), "page {pid}");
    }
}

#[test]
fn a_saved_map_of_another_file_is_ignored_and_replaced() {
    let store = Memory::default();
    let mut bytes = fixture_bytes("typeprobe.bak");
    bytes.extend_from_slice(&[0u8; 8192]); // same backup, a different length
    PageFile::open_indexed(Box::new(bytes), &store).unwrap();
    let (src, _) = counting();
    PageFile::open_indexed(src, &store).unwrap();
    assert_eq!(
        store.saves.load(Ordering::SeqCst),
        2,
        "the stale map was rebuilt and saved again"
    );
    // garbage is ignored the same way
    *store.saved.lock().unwrap() = Some(vec![1, 2, 3]);
    let (src, _) = counting();
    PageFile::open_indexed(src, &store).unwrap();
    assert_eq!(store.saves.load(Ordering::SeqCst), 3);
}

#[test]
fn a_table_reads_the_same_rows_through_a_saved_map() {
    let store = Memory::default();
    let (src, _) = counting();
    let a = BakSource::open_indexed(src, &store).unwrap();
    assert_eq!(store.saves.load(Ordering::SeqCst), 1);
    let (src, reads) = counting();
    let b = BakSource::open_indexed(src, &store).unwrap();
    assert_eq!(store.saves.load(Ordering::SeqCst), 1, "nothing to save the second time");
    let (framing, framing_reads) = counting();
    al_bcdb::mtf::MtfFile::parse(&*framing).unwrap();
    assert_eq!(
        reads.load(Ordering::SeqCst),
        framing_reads.load(Ordering::SeqCst),
        "with the map and the catalog kept, an open reads the container's framing and no page"
    );
    assert_eq!(dump(&a), dump(&b));
}

/// What a source gives a run: every table (name, columns, rows or the reader's error), the sequences and the last
/// row version. The equality oracle of a kept index: an open through it must give what a fresh open gives.
#[derive(Debug, PartialEq)]
struct Dump {
    tables: Vec<(String, Vec<SysColumn>, Result<Vec<Vec<Value>>, String>)>,
    sequences: Result<Vec<SqlSequence>, String>,
    last_row_version: Result<Option<u64>, String>,
}

fn dump(s: &BakSource) -> Dump {
    use al_bcdb::BcSource;
    let tables = s
        .tables()
        .unwrap()
        .iter()
        .map(|t| {
            let cols = s.columns(t).unwrap();
            let rows = read_all(s, t, &cols).map_err(|e| e.to_string());
            (t.name.clone(), cols, rows)
        })
        .collect();
    Dump {
        tables,
        sequences: s.sequences().map_err(|e| e.to_string()),
        last_row_version: s.last_row_version().map_err(|e| e.to_string()),
    }
}

#[test]
fn a_kept_index_reads_what_a_fresh_open_reads() {
    let fresh = dump(&BakSource::open(Box::new(fixture_bytes("typeprobe.bak"))).unwrap());
    assert!(fresh.tables.len() > 3);
    let store = Memory::default();
    let (src, _) = counting();
    let built = BakSource::open_indexed(src, &store).unwrap();
    assert_eq!(store.saves.load(Ordering::SeqCst), 1, "the first open builds the index");
    assert_eq!(dump(&built), fresh, "built");
    let (src, _) = counting();
    let read = BakSource::open_indexed(src, &store).unwrap();
    assert_eq!(store.saves.load(Ordering::SeqCst), 1, "the second open reads it");
    assert_eq!(dump(&read), fresh, "read");
}

/// The offsets of a saved page map: the header is the magic, the version, the file length, the region count and one
/// pair per region, then five counts, the last of them the number of offsets that follow.
fn map_offsets(map: &[u8]) -> std::ops::Range<usize> {
    let regions = u32::from_le_bytes(map[20..24].try_into().unwrap()) as usize;
    let start = 24 + 16 * regions + 40;
    let n = u64::from_le_bytes(map[start - 8..start].try_into().unwrap()) as usize;
    start..start + 8 * n
}

#[test]
fn a_kept_map_with_two_page_offsets_swapped_is_rebuilt() {
    let store = Memory::default();
    let (src, _) = counting();
    let fresh = PageFile::open_indexed(src, &store).unwrap();
    let good = store.saved.lock().unwrap().clone().unwrap();
    // The first two mapped offsets, swapped: each still lies inside the file and the mapped count is unchanged.
    let offsets = map_offsets(&good);
    let mapped: Vec<usize> = (offsets.start..offsets.end)
        .step_by(8)
        .filter(|&at| good[at..at + 8] != [0xff; 8])
        .take(2)
        .collect();
    let mut bad = good.clone();
    let (x, y) = (mapped[0], mapped[1]);
    bad[x..x + 8].copy_from_slice(&good[y..y + 8]);
    bad[y..y + 8].copy_from_slice(&good[x..x + 8]);
    *store.saved.lock().unwrap() = Some(bad);
    let (src, _) = counting();
    let again = PageFile::open_indexed(src, &store).unwrap();
    assert_eq!(
        store.saves.load(Ordering::SeqCst),
        2,
        "the damaged map is rebuilt and saved again"
    );
    assert_eq!(store.saved.lock().unwrap().clone().unwrap(), good);
    for pid in 0..fresh.file_pages() as i32 {
        let (a, b) = (fresh.try_page(1, pid).unwrap(), again.try_page(1, pid).unwrap());
        assert_eq!(a.as_deref(), b.as_deref(), "page {pid}");
    }
}

#[test]
fn a_flipped_byte_anywhere_in_a_kept_index_is_a_miss() {
    let store = Memory::default();
    let (src, _) = counting();
    BakSource::open_indexed(src, &store).unwrap();
    let good = store.saved.lock().unwrap().clone().unwrap();
    let mut taken = Vec::new();
    for at in (0..good.len()).step_by(37) {
        let mut bad = good.clone();
        bad[at] ^= 0x20;
        *store.saved.lock().unwrap() = Some(bad);
        let (src, _) = counting();
        BakSource::open_indexed(src, &store).unwrap();
        if store.saved.lock().unwrap().as_deref() != Some(&good[..]) {
            taken.push(at);
        }
    }
    assert!(
        taken.is_empty(),
        "{} of {} flipped positions were taken as a kept index, the first at {:?}",
        taken.len(),
        good.len().div_ceil(37),
        &taken[..taken.len().min(5)]
    );
}

#[test]
fn a_version_1_map_is_rebuilt() {
    let store = Memory::default();
    let (src, _) = counting();
    PageFile::open_indexed(src, &store).unwrap();
    let good = store.saved.lock().unwrap().clone().unwrap();
    assert_eq!(
        u32::from_le_bytes(good[8..12].try_into().unwrap()),
        2,
        "a map is saved as version 2"
    );
    // The layout of version 1: the same header and offsets, without the trailing checksum.
    let mut old = good[..good.len() - 8].to_vec();
    old[8..12].copy_from_slice(&1u32.to_le_bytes());
    *store.saved.lock().unwrap() = Some(old);
    let (src, _) = counting();
    PageFile::open_indexed(src, &store).unwrap();
    assert_eq!(store.saves.load(Ordering::SeqCst), 2, "a version 1 map is rebuilt");
    assert_eq!(store.saved.lock().unwrap().clone().unwrap(), good);
}

#[test]
fn a_damaged_catalog_section_is_rebuilt_without_losing_the_map() {
    let store = Memory::default();
    let (src, _) = counting();
    BakSource::open_indexed(src, &store).unwrap();
    let good = store.saved.lock().unwrap().clone().unwrap();
    for cut in [good.len() - 1, good.len() - 5000, 20, 0] {
        *store.saved.lock().unwrap() = Some(good[..cut].to_vec());
        let (src, _) = counting();
        let b = BakSource::open_indexed(src, &store).unwrap();
        use al_bcdb::BcSource;
        assert!(b.tables().unwrap().len() > 3, "cut at {cut}");
        assert_eq!(
            store.saved.lock().unwrap().clone().unwrap(),
            good,
            "cut at {cut}: saved again whole"
        );
    }
    // a flipped byte in the catalog section
    let mut bad = good.clone();
    let n = bad.len();
    bad[n - 10] ^= 0xff;
    *store.saved.lock().unwrap() = Some(bad);
    let (src, _) = counting();
    BakSource::open_indexed(src, &store).unwrap();
    assert_eq!(store.saved.lock().unwrap().clone().unwrap(), good);
}

/// The SQL sequences are kept with the catalog: with the index kept, reading them reads no page (every test session of a
/// run with test data asks for them; walking the system tables for them was about 30 M instructions per run), and they are
/// the ones a walk of the file finds.
#[test]
fn the_sequences_are_kept_with_the_catalog() {
    use al_bcdb::BcSource;
    let store = Memory::default();
    let (src, _) = counting();
    BakSource::open_indexed(src, &store).unwrap();
    let (src, reads) = counting();
    let b = BakSource::open_indexed(src, &store).unwrap();
    let after_open = reads.load(Ordering::SeqCst);
    let kept = b.sequences().unwrap();
    assert_eq!(
        reads.load(Ordering::SeqCst),
        after_open,
        "the kept sequences are read without a page"
    );
    let walked = BakSource::open(Box::new(fixture_bytes("typeprobe.bak")))
        .unwrap()
        .sequences()
        .unwrap();
    assert_eq!(kept, walked);
}
