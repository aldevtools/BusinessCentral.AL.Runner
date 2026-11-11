//! A package whose source files are not read yet (ADR-022, "pure tests"). Reading a package for a
//! run inflates every `.al` entry: Base Application alone is 5 000 files, 150 MB of text, and a run
//! whose tests reach none of it still paid 0.1 s of CPU. Here only the central directory and the
//! manifest are read. The files are read one at a time, by position, when something asks.
//!
//! The hash of the package is made from the zip directory (name, size and CRC-32 of every entry), so
//! it too needs no inflating. The CRC-32 of an entry is checked when the entry is read.

use crate::package::{Manifest, Vars, is_al, is_app, manifest_of_source_zip, parse_manifest, percent_decode_once};
use crate::zip::{END_SEARCH, Entry, find_end_record, local_header_len, parse_central_directory};
use al_sema::{LazySource, SourceLoader};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::path::Path;
use std::sync::Arc;

/// A package that is read lazily: its manifest, the path of each `.al` file in the order eager
/// reading gives them, and the way to read one.
pub struct LazyPackage {
    pub manifest: Manifest,
    pub names: Vec<String>,
    /// No `.al` file and symbols that list no object ([`crate::empty`]): the package's source is complete.
    pub no_objects: bool,
    pub source: LazySource,
}

#[derive(Debug)]
struct ZipLoader {
    file: File,
    /// Where the archive starts in the file (the NAVX header in front of a package).
    base: u64,
    /// The `.al` entries, in the order of `LazyPackage::names`.
    entries: Vec<Entry>,
}

fn read_exact_at(f: &File, off: u64, buf: &mut [u8]) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::FileExt::read_exact_at(f, buf, off)
    }
    #[cfg(windows)]
    {
        let (mut done, mut off) = (0, off);
        while done < buf.len() {
            let n = std::os::windows::fs::FileExt::seek_read(f, &mut buf[done..], off)?;
            if n == 0 {
                return Err(std::io::ErrorKind::UnexpectedEof.into());
            }
            done += n;
            off += n as u64;
        }
        Ok(())
    }
}

/// The inflated bytes of `e`, checked against its recorded size and CRC-32.
fn entry_bytes(f: &File, base: u64, e: &Entry) -> Result<Vec<u8>, String> {
    let at = base + e.local_offset() as u64;
    let mut head = [0u8; 30];
    read_exact_at(f, at, &mut head).map_err(|err| format!("entry `{}`: {err}", e.name))?;
    let skip = local_header_len(&head, &e.name).map_err(|err| err.to_string())?;
    let mut raw = vec![0u8; e.compressed_size()];
    read_exact_at(f, at + skip as u64, &mut raw).map_err(|err| format!("entry `{}`: {err}", e.name))?;
    e.decode(&raw).map_err(|err| err.to_string())
}

impl SourceLoader for ZipLoader {
    fn text(&self, i: usize) -> String {
        let e = &self.entries[i];
        let bytes = entry_bytes(&self.file, self.base, e)
            .unwrap_or_else(|why| panic!("a package file that read at load cannot be read now: {why}"));
        // The decoded bytes become the text without a copy when they are valid UTF-8.
        let mut text = match String::from_utf8(bytes) {
            Ok(text) => text,
            Err(e) => String::from_utf8_lossy(e.as_bytes()).into_owned(),
        };
        if text.starts_with('\u{feff}') {
            text.drain(..'\u{feff}'.len_utf8());
        }
        text
    }
}

/// The open file, where the archive starts in it and the central directory of the zip at `path`.
fn open_directory(path: &Path) -> Option<(File, u64, Vec<Entry>)> {
    let file = File::open(path).ok()?;
    let total = file.metadata().ok()?.len();
    let tail_len = total.min(END_SEARCH as u64);
    let mut tail = vec![0u8; tail_len as usize];
    read_exact_at(&file, total - tail_len, &mut tail).ok()?;
    let end = find_end_record(&tail).ok()?;
    let cd_start = (total - tail_len) + end.at as u64 - end.cd_size as u64;
    let mut cd = vec![0u8; end.cd_size];
    read_exact_at(&file, cd_start, &mut cd).ok()?;
    let entries = parse_central_directory(&cd, end.count).ok()?;
    let base = cd_start.checked_sub(end.cd_offset as u64)?;
    Some((file, base, entries))
}

/// An open zip: the file, where the archive starts in it and its entries by name.
type OpenZip = (File, u64, std::collections::HashMap<String, Entry>);

/// The files of a dependency app's package or folder, read when asked for (`al_ir::LayoutSource`). The package's central
/// directory is read on the first ask, so a run that never exports a layout pays nothing.
pub struct PackageFiles {
    path: std::path::PathBuf,
    zip: std::sync::OnceLock<Option<OpenZip>>,
}

impl PackageFiles {
    pub fn new(path: std::path::PathBuf) -> PackageFiles {
        PackageFiles {
            path,
            zip: std::sync::OnceLock::new(),
        }
    }
}

impl al_ir::LayoutSource for PackageFiles {
    fn file(&self, path: &str) -> Option<Vec<u8>> {
        if self.path.is_dir() {
            return std::fs::read(self.path.join(path)).ok();
        }
        let (file, base, entries) = self
            .zip
            .get_or_init(|| {
                let (file, base, entries) = open_directory(&self.path)?;
                Some((file, base, entries.into_iter().map(|e| (e.name.clone(), e)).collect()))
            })
            .as_ref()?;
        entry_bytes(file, *base, entries.get(path)?).ok()
    }
}

/// How many inflated bytes of a translation file [`PackageTranslations`] reads for its `<file target-language>`.
const XLIFF_HEAD: usize = 16 << 10;

/// Where one translation file of a [`PackageTranslations`] lives.
enum XliffAt {
    Entry(Entry),
    File(std::path::PathBuf),
}

/// The translation files of a dependency app's package or folder (`al_ir::TranslationSource`), ADR-053 step 2. The first
/// ask reads the zip directory and the start of each file for its language; a file's units are read on the first ask for
/// that file. A wrapper package, or a file that is not a zip, gives no files (its translated reads stay refused).
pub struct PackageTranslations {
    path: std::path::PathBuf,
    open: std::sync::OnceLock<OpenTranslations>,
}

struct OpenTranslations {
    file: Option<(File, u64)>,
    at: Vec<XliffAt>,
    heads: Vec<al_ir::TranslationFile>,
    read: Vec<std::sync::OnceLock<al_ir::TranslationFile>>,
}

impl PackageTranslations {
    pub fn new(path: std::path::PathBuf) -> PackageTranslations {
        PackageTranslations {
            path,
            open: std::sync::OnceLock::new(),
        }
    }

    fn open(&self) -> &OpenTranslations {
        self.open.get_or_init(|| {
            let (file, mut found) = if self.path.is_dir() {
                let found = crate::load::translation_file_paths(&self.path)
                    .into_iter()
                    .map(|(name, p)| (name, XliffAt::File(p)))
                    .collect();
                (None, found)
            } else {
                match open_directory(&self.path) {
                    Some((file, base, entries)) => {
                        let app = entries.iter().any(|e| e.name == "NavxManifest.xml");
                        let found = entries
                            .into_iter()
                            .filter_map(|e| {
                                let name = if app {
                                    percent_decode_once(&e.name)
                                } else {
                                    e.name.clone()
                                };
                                crate::package::is_translation_file(&name).then_some((name, XliffAt::Entry(e)))
                            })
                            .collect();
                        (Some((file, base)), found)
                    }
                    None => (None, Vec::new()),
                }
            };
            found.sort_by(|a: &(String, XliffAt), b| a.0.cmp(&b.0));
            let mut heads = Vec::with_capacity(found.len());
            let mut at = Vec::with_capacity(found.len());
            for (name, place) in found {
                let start = xliff_start(file.as_ref(), &place);
                let mut head = al_ir::TranslationFile {
                    name,
                    ..Default::default()
                };
                match start.and_then(|b| crate::xliff::target_language(&b)) {
                    Ok(l) => head.target_language = l,
                    Err(e) => head.error = Some(e),
                }
                heads.push(head);
                at.push(place);
            }
            let read = heads.iter().map(|_| std::sync::OnceLock::new()).collect();
            OpenTranslations { file, at, heads, read }
        })
    }
}

/// The first [`XLIFF_HEAD`] inflated bytes (or fewer) of a translation file.
fn xliff_start(file: Option<&(File, u64)>, at: &XliffAt) -> Result<Vec<u8>, String> {
    use std::io::Read;
    match (at, file) {
        (XliffAt::File(p), _) => {
            let mut out = Vec::new();
            File::open(p)
                .and_then(|f| f.take(XLIFF_HEAD as u64).read_to_end(&mut out))
                .map_err(|e| e.to_string())?;
            Ok(out)
        }
        (XliffAt::Entry(e), Some((f, base))) => {
            let at = base + e.local_offset() as u64;
            let mut head = [0u8; 30];
            read_exact_at(f, at, &mut head).map_err(|err| err.to_string())?;
            let skip = local_header_len(&head, &e.name).map_err(|err| err.to_string())?;
            let mut raw = vec![0u8; e.compressed_size().min(XLIFF_HEAD)];
            read_exact_at(f, at + skip as u64, &mut raw).map_err(|err| err.to_string())?;
            let mut out = Vec::new();
            let mut decoder = e.decoder().map_err(|err| err.to_string())?;
            // The stored bytes are cut, so the inflater stops where they end: what it gave so far is the start.
            let _ = decoder.push::<crate::zip::ZipError>(&raw, &mut |piece| {
                out.extend_from_slice(piece);
                Ok(())
            });
            out.truncate(XLIFF_HEAD);
            Ok(out)
        }
        (XliffAt::Entry(_), None) => Err("the package cannot be read".into()),
    }
}

impl al_ir::TranslationSource for PackageTranslations {
    fn files(&self) -> &[al_ir::TranslationFile] {
        &self.open().heads
    }

    fn file(&self, index: usize) -> &al_ir::TranslationFile {
        let open = self.open();
        open.read[index].get_or_init(|| {
            let head = &open.heads[index];
            let bytes = match (&open.at[index], &open.file) {
                (XliffAt::File(p), _) => std::fs::read(p).map_err(|e| e.to_string()),
                (XliffAt::Entry(e), Some((f, base))) => entry_bytes(f, *base, e),
                (XliffAt::Entry(_), None) => Err("the package cannot be read".into()),
            };
            crate::xliff::translation_file(head.name.clone(), bytes.and_then(|b| crate::xliff::parse(&b)))
        })
    }
}

/// Reads the manifest and the central directory of the package at `path`. `None`: the package
/// cannot be read lazily (a wrapper around an inner `.app`, an unreadable or unusual file); the
/// caller reads it as usual and gets the error, if there is one, from that.
pub fn read_lazy(path: &Path, vars: &Vars) -> Option<LazyPackage> {
    let (file, base, entries) = open_directory(path)?;
    let find = |name: &str| entries.iter().find(|e| e.name == name);
    let (manifest, source_zip) = if let Some(m) = find("NavxManifest.xml") {
        // A wrapper holds the real package inside.
        if !entries.iter().any(|e| is_al(&e.name)) && entries.iter().any(|e| is_app(&e.name)) {
            return None;
        }
        let xml = entry_bytes(&file, base, m).ok()?;
        (parse_manifest(&String::from_utf8_lossy(&xml)).ok()?, false)
    } else {
        let json = entry_bytes(&file, base, find("app.json")?).ok()?;
        (
            manifest_of_source_zip(&String::from_utf8_lossy(&json), vars).ok()?,
            true,
        )
    };
    let mut al: Vec<(String, &Entry)> = entries
        .iter()
        .filter(|e| is_al(&e.name))
        .map(|e| {
            let name = if source_zip {
                e.name.clone()
            } else {
                percent_decode_once(&e.name)
            };
            (name, e)
        })
        .collect();
    al.sort_by(|a, b| a.0.cmp(&b.0));
    let mut all: Vec<&Entry> = entries.iter().collect();
    all.sort_by(|a, b| a.name.cmp(&b.name));
    let mut h = Sha256::new();
    h.update(b"al-runtime zip directory");
    h.update((all.len() as u64).to_le_bytes());
    for e in all {
        h.update((e.name.len() as u64).to_le_bytes());
        h.update(e.name.as_bytes());
        h.update((e.size as u64).to_le_bytes());
        h.update(e.crc.to_le_bytes());
    }
    let identity: [u8; 32] = h.finalize().into();
    let no_objects = al.is_empty()
        && find("SymbolReference.json")
            .filter(|e| e.size <= crate::empty::MAX_SYMBOLS_LEN)
            .and_then(|e| entry_bytes(&file, base, e).ok())
            .is_some_and(|bytes| crate::empty::holds_no_objects(&bytes));
    let names: Vec<String> = al.iter().map(|(n, _)| n.clone()).collect();
    let entries: Vec<Entry> = al.into_iter().map(|(_, e)| e.clone()).collect();
    Some(LazyPackage {
        manifest,
        names,
        no_objects,
        source: LazySource {
            loader: Arc::new(ZipLoader { file, base, entries }),
            identity,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zip::testutil::{Builder, navx};

    const MANIFEST: &str =
        r#"<Package><App Id="aa" Name="App" Publisher="P" Version="1.0.0.0" /><Dependencies /></Package>"#;

    fn write(dir: &Path, name: &str, files: &[(&str, &str)]) -> std::path::PathBuf {
        let mut b = Builder::new().add("NavxManifest.xml", MANIFEST.as_bytes(), true);
        for (n, t) in files {
            b = b.add(n, t.as_bytes(), true);
        }
        let path = dir.join(name);
        std::fs::write(&path, navx(&b.finish())).unwrap();
        path
    }

    #[test]
    fn a_lazy_package_names_and_reads_the_files_an_eager_read_gives() {
        let dir = tempfile::tempdir().unwrap();
        let files = [
            ("src/B.al", "\u{feff}codeunit 2 B { }"),
            ("src/A%2520b.al", "codeunit 1 A { }"),
            ("readme.md", "not al"),
        ];
        let path = write(dir.path(), "x.app", &files);
        let lazy = read_lazy(&path, &Vars::default()).expect("lazy");
        let (manifest, sources) = crate::load::read_package_at(&path).unwrap();
        assert_eq!(lazy.manifest, manifest);
        assert_eq!(lazy.names, sources.iter().map(|s| s.0.clone()).collect::<Vec<_>>());
        for (i, (_, text)) in sources.iter().enumerate() {
            assert_eq!(&lazy.source.loader.text(i), text);
        }
    }

    #[test]
    fn the_identity_follows_the_content_without_reading_it() {
        let dir = tempfile::tempdir().unwrap();
        let a = write(dir.path(), "a.app", &[("A.al", "codeunit 1 A { }")]);
        let same = write(dir.path(), "b.app", &[("A.al", "codeunit 1 A { }")]);
        let changed = write(dir.path(), "c.app", &[("A.al", "codeunit 1 Z { }")]);
        let renamed = write(dir.path(), "d.app", &[("Z.al", "codeunit 1 A { }")]);
        let id = |p: &Path| read_lazy(p, &Vars::default()).unwrap().source.identity;
        assert_eq!(id(&a), id(&same));
        assert_ne!(id(&a), id(&changed));
        assert_ne!(id(&a), id(&renamed));
    }

    #[test]
    fn a_wrapper_or_a_file_that_is_not_a_package_is_left_to_the_eager_read() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("junk.app"), b"not a zip").unwrap();
        assert!(read_lazy(&dir.path().join("junk.app"), &Vars::default()).is_none());
        let inner = write(dir.path(), "inner.app", &[]);
        let wrapper = navx(
            &Builder::new()
                .add("NavxManifest.xml", MANIFEST.as_bytes(), true)
                .add("inner.app", &std::fs::read(inner).unwrap(), true)
                .finish(),
        );
        std::fs::write(dir.path().join("w.app"), wrapper).unwrap();
        assert!(read_lazy(&dir.path().join("w.app"), &Vars::default()).is_none());
    }

    const XLF: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<xliff version="1.2"><file datatype="xml" source-language="en-US" target-language="LANG" original="App"><body><group id="body">
<trans-unit id="Codeunit 1 - NamedType 2"><source>Hello</source><target state="translated">TEXT</target></trans-unit>
</group></body></file></xliff>"#;

    #[test]
    fn package_translations_name_the_files_first_and_read_one_when_asked() {
        use al_ir::TranslationSource;
        let dir = tempfile::tempdir().unwrap();
        let nl = XLF.replace("LANG", "nl-BE").replace("TEXT", "Hallo");
        let fr = XLF.replace("LANG", "fr-BE").replace("TEXT", "Bonjour");
        let path = write(
            dir.path(),
            "t.app",
            &[
                ("Translations/App%2520X.nl-BE.xlf", nl.as_str()),
                ("Translations/App%2520X.fr-BE.xlf", fr.as_str()),
                ("Translations/App%2520X.g.xlf", "not read"),
                ("Translations/Broken.de-DE.xlf", "<xliff><nothing/></xliff>"),
            ],
        );
        let t = PackageTranslations::new(path);
        let heads: Vec<(&str, &str, bool)> = t
            .files()
            .iter()
            .map(|f| (f.name.as_str(), f.target_language.as_str(), f.error.is_some()))
            .collect();
        assert_eq!(
            heads,
            [
                ("Translations/App%20X.fr-BE.xlf", "fr-BE", false),
                ("Translations/App%20X.nl-BE.xlf", "nl-BE", false),
                ("Translations/Broken.de-DE.xlf", "", true),
            ]
        );
        assert!(t.files().iter().all(|f| f.units.is_empty()));
        assert_eq!(t.file(1).target("Codeunit 1 - NamedType 2"), Some("Hallo"));
        assert_eq!(
            t.file(0).unit("Codeunit 1 - NamedType 2").unwrap().1,
            &al_ir::UnitState::Translated
        );
        assert!(t.file(2).error.is_some());
        // A folder of a dependency app gives the same.
        let folder = dir.path().join("app");
        std::fs::create_dir_all(folder.join("Translations")).unwrap();
        std::fs::write(folder.join("Translations/App.nl-NL.xlf"), XLF.replace("LANG", "nl-NL")).unwrap();
        let t = PackageTranslations::new(folder);
        assert_eq!(t.files()[0].target_language, "nl-NL");
        assert_eq!(t.file(0).target("Codeunit 1 - NamedType 2"), Some("TEXT"));
        // A file that is not a package gives no files.
        std::fs::write(dir.path().join("junk.app"), b"not a zip").unwrap();
        assert!(PackageTranslations::new(dir.path().join("junk.app")).files().is_empty());
    }

    #[test]
    fn package_files_serve_a_file_of_a_zip_or_a_folder_on_demand() {
        use al_ir::LayoutSource;
        let dir = tempfile::tempdir().unwrap();
        let zip = dir.path().join("src.zip");
        let b = Builder::new()
            .add("app.json", b"{}", true)
            .add("Sales/Invoice.rdlc", b"RDLC-BYTES\x00\xff", true);
        std::fs::write(&zip, b.finish()).unwrap();
        let files = PackageFiles::new(zip);
        assert_eq!(files.file("Sales/Invoice.rdlc").unwrap(), b"RDLC-BYTES\x00\xff");
        assert_eq!(files.file("Sales/Other.rdlc"), None);
        let folder = dir.path().join("app");
        std::fs::create_dir_all(folder.join("Sales")).unwrap();
        std::fs::write(folder.join("Sales/Invoice.docx"), b"DOCX").unwrap();
        let files = PackageFiles::new(folder);
        assert_eq!(files.file("Sales/Invoice.docx").unwrap(), b"DOCX");
        assert_eq!(files.file("Sales/Missing.docx"), None);
    }
}
