//! The page-map index of a `.bak` can be kept by the caller: a second open reads the saved map
//! instead of verifying every extent of the file (cold, that is the difference between
//! seconds and milliseconds on a 900 MB backup).

mod common;
use al_bcdb::bak::BakSource;
use al_bcdb::io::ByteSource;
use al_bcdb::pagefile::{IndexStore, PageFile};
use al_bcdb::source::read_all;
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
    use al_bcdb::BcSource;
    for (x, y) in a.tables().unwrap().iter().zip(b.tables().unwrap()) {
        assert_eq!(x.name, y.name);
        let cols = a.columns(x).unwrap();
        assert_eq!(
            read_all(&a, x, &cols).unwrap(),
            read_all(&b, y, &cols).unwrap(),
            "{}",
            x.name
        );
    }
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
