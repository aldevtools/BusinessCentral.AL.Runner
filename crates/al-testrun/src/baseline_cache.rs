//! The install baseline as an entry of the compile cache (ADR-022). Running the install code of the dependency
//! apps and `Company-Initialize` takes longer than reading what they leave, and it is the same for the same program
//! and the same world around it. The entry lives in the compile cache's folder (`<cache>/compile/`, file extension
//! `albr`), is tied to the same build of the runtime, is pruned with the other entries, and goes with
//! `al-runner cache clear` and is replaced by `--rebuild-compile-cache`. There is no second cache.
//!
//! The file is the [`BaselineRun`]: the data the install code left, the notes about roots that did not run to their
//! end, and the instant the run began. The file is written to a name of its own and renamed, so a reader never sees
//! half a file; a file that cannot be read is removed and the install code runs.

use crate::baseline::BaselineRun;
use al_builtins::value_codec::{Reader, put_i64, put_str, put_u8, put_u32};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// The first bytes of a file: what it is and which layout it has.
const MAGIC: &[u8; 8] = b"ALBRUN\0\x02";

/// The extension of the entry files (the layers have `alcc`, the source index `alci`).
pub const EXTENSION: &str = "albr";

/// What the key is made of, one part after the other; a part that is not given changes the key as well as one that
/// differs.
pub struct KeyBuilder(Sha256);

impl KeyBuilder {
    pub fn new() -> KeyBuilder {
        let mut h = Sha256::new();
        h.update(MAGIC);
        KeyBuilder(h)
    }
    pub fn text(&mut self, name: &str, value: &str) -> &mut Self {
        self.bytes(name, value.as_bytes())
    }
    pub fn bytes(&mut self, name: &str, value: &[u8]) -> &mut Self {
        self.0.update((name.len() as u64).to_le_bytes());
        self.0.update(name.as_bytes());
        self.0.update((value.len() as u64).to_le_bytes());
        self.0.update(value);
        self
    }
    pub fn finish(&self) -> String {
        self.0.clone().finalize().iter().map(|b| format!("{b:02x}")).collect()
    }
}

impl Default for KeyBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// The bytes of a run, or `None` when it cannot be kept: its baseline has no bytes (a row holds a value that is not
/// plain). `started_at` is the instant the run began (Unix milliseconds): the rows of the baseline
/// carry it, so a run that reads the file starts its own clock there (`SystemClock::begin_at`).
pub fn encode(run: &BaselineRun, started_at: i64) -> Option<Vec<u8>> {
    let mut out = MAGIC.to_vec();
    put_i64(&mut out, started_at);
    put_u32(&mut out, run.unseeded.len() as u32);
    for t in &run.unseeded {
        put_u32(&mut out, *t);
    }
    put_u32(&mut out, run.filled as u32);
    put_u32(&mut out, run.ran as u32);
    put_u32(&mut out, run.total as u32);
    put_u32(&mut out, run.notes.len() as u32);
    for n in &run.notes {
        put_str(&mut out, n);
    }
    put_u32(&mut out, run.restored.len() as u32);
    for c in &run.restored {
        put_str(&mut out, &c.table);
        put_u8(&mut out, op_code(c.op));
        put_u8(&mut out, c.same as u8);
        put_u32(&mut out, c.fields.len() as u32);
        for f in &c.fields {
            put_str(&mut out, f);
        }
    }
    put_u8(&mut out, 1);
    let baseline = run.baseline.to_bytes()?;
    out.extend_from_slice(&baseline);
    Some(out)
}

/// The run [`encode`] wrote and the instant it began, or `None` for anything else.
pub fn decode(bytes: &[u8]) -> Option<(BaselineRun, i64)> {
    let mut r = Reader::new(bytes.strip_prefix(&MAGIC[..])?);
    let started_at = r.i64()?;
    let mut unseeded = std::collections::BTreeSet::new();
    for _ in 0..r.u32()? {
        unseeded.insert(r.u32()?);
    }
    let filled = r.u32()? as usize;
    let ran = r.u32()? as usize;
    let total = r.u32()? as usize;
    let mut notes = Vec::new();
    for _ in 0..r.u32()? {
        notes.push(r.str()?);
    }
    let mut restored = Vec::new();
    for _ in 0..r.u32()? {
        let table = r.str()?;
        let op = op_name(r.u8()?)?;
        let same = r.u8()? != 0;
        let mut fields = Vec::new();
        for _ in 0..r.u32()? {
            fields.push(r.str()?);
        }
        restored.push(al_data::memory::RestoredChange {
            table,
            op,
            same,
            fields,
        });
    }
    if r.u8()? != 1 {
        return None;
    }
    let baseline = al_vm::Baseline::from_bytes(r.rest())?;
    Some((
        BaselineRun {
            baseline,
            unseeded,
            filled,
            notes,
            ran,
            total,
            restored,
            trace: None,
        },
        started_at,
    ))
}

const OPS: [&str; 4] = ["Modify", "Delete", "Rename", "DeleteAll"];

fn op_code(op: &str) -> u8 {
    OPS.iter().position(|o| *o == op).unwrap_or(0) as u8
}

fn op_name(code: u8) -> Option<&'static str> {
    OPS.get(code as usize).copied()
}

fn file(dir: &Path, key: &str) -> PathBuf {
    // The build tag first: a cleanup tells the builds apart by the name (ADR-022, "Cleanup of old builds").
    let tag = al_cache::build_tag(&crate::compile_cache::runtime_build());
    dir.join(format!("{tag}-{key}.{EXTENSION}"))
}

/// The run kept under `key` and the instant it began. A file that cannot be read is removed. A hit marks the file
/// as used now, so that the cleanup of the compile cache keeps what runs still use.
pub fn read(dir: &Path, key: &str) -> Option<(BaselineRun, i64)> {
    read_with(dir, key, decode)
}

/// What `decode` makes of the file kept under `key`. A file it cannot read is removed; a hit marks the file as used.
pub(crate) fn read_with<T>(dir: &Path, key: &str, decode: impl FnOnce(&[u8]) -> Option<T>) -> Option<T> {
    let path = file(dir, key);
    let bytes = std::fs::read(&path).ok()?;
    match decode(&bytes) {
        Some(run) => {
            crate::compile_cache::touch(&path);
            Some(run)
        }
        None => {
            let _ = std::fs::remove_file(&path);
            None
        }
    }
}

/// Keeps `run` under `key`, with the instant it began. Best effort: a folder that cannot be written costs the next
/// run its install code, nothing else. The cleanup is the compile cache's (`compile_cache::prune`).
pub fn write(dir: &Path, key: &str, run: &BaselineRun, started_at: i64) {
    if let Some(bytes) = encode(run, started_at) {
        write_bytes(dir, key, &bytes);
    }
}

/// Keeps `bytes` under `key`, as [`write`] does.
pub(crate) fn write_bytes(dir: &Path, key: &str, bytes: &[u8]) {
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    let target = file(dir, key);
    // A name of its own per writer: two runs that write at once do not mix their bytes.
    let temp = dir.join(format!(".{key}.tmp-{}", std::process::id()));
    if std::fs::write(&temp, bytes).is_err() || std::fs::rename(&temp, &target).is_err() {
        let _ = std::fs::remove_file(&temp);
        return;
    }
    crate::compile_cache::prune(dir, &[target]);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run() -> BaselineRun {
        let engine = al_vm::Engine::new(al_ir::Program::default(), Vec::new());
        let mut session = al_vm::Session::new(engine);
        BaselineRun {
            baseline: session.capture_baseline(),
            unseeded: [7u32, 9].into_iter().collect(),
            filled: 3,
            notes: vec!["Company-Initialize.OnRun did not run to its end: why".into()],
            ran: 17,
            total: 18,
            restored: Vec::new(),
            trace: None,
        }
    }

    fn same(a: &BaselineRun, b: &BaselineRun) -> bool {
        (&a.unseeded, a.filled, &a.notes, a.ran, a.total, a.baseline.to_bytes())
            == (&b.unseeded, b.filled, &b.notes, b.ran, b.total, b.baseline.to_bytes())
    }

    #[test]
    fn a_run_survives_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let r = run();
        write(dir.path(), "k1", &r, 1_792_000_000_123);
        let (back, at) = read(dir.path(), "k1").expect("written and read");
        assert_eq!(at, 1_792_000_000_123, "the instant the run began comes back");
        assert!(same(&r, &back));
        assert!(back.restored.is_empty());
    }

    #[test]
    fn a_key_with_no_file_reads_nothing() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read(dir.path(), "none").is_none());
        write(dir.path(), "k1", &run(), 0);
        assert!(read(dir.path(), "other").is_none());
    }

    #[test]
    fn a_file_that_cannot_be_read_is_removed() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "k1", &run(), 0);
        let path = file(dir.path(), "k1");
        let mut bytes = std::fs::read(&path).unwrap();
        bytes.truncate(bytes.len() - 3);
        std::fs::write(&path, bytes).unwrap();
        assert!(read(dir.path(), "k1").is_none());
        assert!(!path.exists(), "the damaged file is gone");
        std::fs::write(&path, b"nonsense").unwrap();
        assert!(read(dir.path(), "k1").is_none());
        assert!(!path.exists());
    }

    #[test]
    fn the_changes_the_install_code_made_to_restored_rows_survive_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let mut r = run();
        r.restored.push(al_data::memory::RestoredChange {
            table: "T".into(),
            op: "Rename",
            same: false,
            fields: vec!["A".into(), "B".into()],
        });
        r.restored.push(al_data::memory::RestoredChange {
            table: "U".into(),
            op: "DeleteAll",
            same: false,
            fields: Vec::new(),
        });
        write(dir.path(), "k1", &r, 0);
        let (back, _) = read(dir.path(), "k1").expect("kept");
        assert_eq!(back.restored, r.restored);
    }

    #[test]
    fn the_entry_sits_in_the_compile_cache_folder_and_goes_with_its_cleanup() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "old", &run(), 0);
        write(dir.path(), "new", &run(), 0);
        assert!(
            file(dir.path(), "old").exists(),
            "next to the layer files, not in a folder of its own"
        );
        // An entry nothing used for more than the cleanup limit goes when another is written; a used one stays.
        let long_ago = std::time::SystemTime::now()
            - std::time::Duration::from_secs((crate::compile_cache::KEEP_DAYS + 1) * 24 * 3600);
        std::fs::File::options()
            .write(true)
            .open(file(dir.path(), "old"))
            .unwrap()
            .set_modified(long_ago)
            .unwrap();
        write(dir.path(), "newer", &run(), 0);
        assert!(!file(dir.path(), "old").exists(), "unused for too long");
        assert!(file(dir.path(), "new").exists());
        assert!(file(dir.path(), "newer").exists());
        // A read marks the file as used.
        std::fs::File::options()
            .write(true)
            .open(file(dir.path(), "new"))
            .unwrap()
            .set_modified(long_ago)
            .unwrap();
        assert!(read(dir.path(), "new").is_some());
        write(dir.path(), "newest", &run(), 0);
        assert!(file(dir.path(), "new").exists(), "a read renewed it");
    }

    #[test]
    fn the_key_depends_on_every_part_and_on_its_name() {
        let k = |f: &dyn Fn(&mut KeyBuilder)| {
            let mut b = KeyBuilder::new();
            f(&mut b);
            b.finish()
        };
        let a = k(&|b| {
            b.text("build", "x").text("day", "1");
        });
        assert_eq!(
            a,
            k(&|b| {
                b.text("build", "x").text("day", "1");
            })
        );
        assert_ne!(
            a,
            k(&|b| {
                b.text("build", "y").text("day", "1");
            })
        );
        assert_ne!(
            a,
            k(&|b| {
                b.text("build", "x").text("day", "2");
            })
        );
        // The same bytes under another name, and the same text cut elsewhere, are other keys.
        assert_ne!(
            a,
            k(&|b| {
                b.text("build", "x").text("night", "1");
            })
        );
        assert_ne!(
            k(&|b| {
                b.text("a", "bc").text("d", "e");
            }),
            k(&|b| {
                b.text("a", "b").text("cd", "e");
            })
        );
        assert_eq!(a.len(), 64);
    }
}
