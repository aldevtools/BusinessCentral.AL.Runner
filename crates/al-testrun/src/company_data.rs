//! A company built by AL code in the install baseline (`al-runner test --company-data demotool`): after the dependency
//! installs and `Company-Initialize`, the `OnRun` of one dependency codeunit runs and writes the company's data. For
//! Microsoft's Legacy test lane that is the DemoTool's `Create Demo Data from Config` (codeunit 101899), which BCApps'
//! CI runs through page 101900 "Demonstration Data Tool" in a new company after `Company-Initialize`
//! (`build/scripts/ImportTestDataInBcContainer.ps1`, `Invoke-LegacyDemoDataTool`). Nothing is faked: the rows are what
//! the codeunit writes, and a codeunit that does not run to its end blocks the company (every test is not run, with the
//! reason), since a part of the company is not the company Microsoft's tests run against.
//!
//! The codeunit reads its resource files (`DemoDataConfig.xml`, pictures, text files) through the `File` type, by paths
//! relative to the server's user folder, where BCApps' script copies them. [`ResourceFiles`] serves them from one
//! folder, read-only; what the codeunit writes (its log file) stays in memory.

use al_host::{FileSystem, MemFileSystem};
use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

/// The codeunit that builds the company, and the files it reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompanyData {
    /// The id of the dependency codeunit whose `OnRun` builds the company.
    pub codeunit: u32,
    /// Its name, as a check that the id names the codeunit meant (a dependency codeunit of another name with that id
    /// is not run, and the company is blocked).
    pub name: String,
    /// The folder whose files the codeunit reads through `File`, if any.
    pub resources: Option<PathBuf>,
    /// What the company depends on beyond the program, for the key of the kept baseline (the source of the codeunit's
    /// app and its resources, as the caller describes them).
    pub identity: String,
}

/// The id of the DemoTool's `Create Demo Data from Config` (BCApps `src/Layers/W1/DemoTool/CreateDemoDatafromConfig.Codeunit.al`).
pub const DEMOTOOL_CODEUNIT: u32 = 101_899;
/// Its name.
pub const DEMOTOOL_CODEUNIT_NAME: &str = "Create Demo Data from Config";

/// The note of a baseline whose company data codeunit did not run to its end; it survives the kept baseline as text.
pub const BLOCKED_NOTE: &str = "the company data was not built: ";

/// Why the company is blocked, from the notes of a baseline run.
pub fn blocked(notes: &[String]) -> Option<String> {
    notes
        .iter()
        .find_map(|n| n.strip_prefix(BLOCKED_NOTE))
        .map(|why| format!("{BLOCKED_NOTE}{why}"))
}

/// The files of one folder, read-only, plus what the code writes, in memory. AL paths are relative to the folder,
/// with `\` or `/` between the parts, and are matched without case, as the Windows server BCApps' CI runs the
/// DemoTool on does (the DemoTool names `CurrencyData.txt`, the folder holds `CurrencyData.TXT`). A path that is
/// absolute or climbs out with `..` names no file of the folder.
pub struct ResourceFiles {
    root: PathBuf,
    written: MemFileSystem,
    removed: BTreeSet<String>,
}

impl ResourceFiles {
    pub fn new(root: PathBuf) -> ResourceFiles {
        ResourceFiles {
            root,
            written: MemFileSystem::default(),
            removed: BTreeSet::new(),
        }
    }

    fn key(path: &str) -> String {
        path.replace('/', "\\").to_lowercase()
    }

    /// The file of the folder that `path` names, matched part by part without case.
    fn resolve(&self, path: &str) -> Option<PathBuf> {
        if self.removed.contains(&Self::key(path)) {
            return None;
        }
        let rel = path.replace('\\', "/");
        let mut out = self.root.clone();
        let mut any = false;
        for c in Path::new(&rel).components() {
            match c {
                Component::Normal(p) => {
                    let want = p.to_string_lossy().to_lowercase();
                    let exact = out.join(p);
                    out = if exact.exists() {
                        exact
                    } else {
                        std::fs::read_dir(&out)
                            .ok()?
                            .filter_map(|e| e.ok())
                            .find(|e| e.file_name().to_string_lossy().to_lowercase() == want)?
                            .path()
                    };
                    any = true;
                }
                Component::CurDir => {}
                _ => return None,
            }
        }
        (any && out.is_file()).then_some(out)
    }
}

impl FileSystem for ResourceFiles {
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        if let Some(b) = self.written.read(&Self::key(path)) {
            return Some(b);
        }
        std::fs::read(self.resolve(path)?).ok()
    }
    fn write(&mut self, path: &str, bytes: &[u8]) -> Result<(), String> {
        self.removed.remove(&Self::key(path));
        self.written.write(&Self::key(path), bytes)
    }
    fn exists(&self, path: &str) -> bool {
        self.written.exists(&Self::key(path)) || self.resolve(path).is_some()
    }
    fn remove(&mut self, path: &str) -> bool {
        let was = self.exists(path);
        self.written.remove(&Self::key(path));
        if was {
            self.removed.insert(Self::key(path));
        }
        was
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_files_are_read_without_case_and_writes_stay_in_memory() {
        let dir = std::env::temp_dir().join(format!("alr-resfiles-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("Images").join("Item")).unwrap();
        std::fs::write(dir.join("CurrencyData.TXT"), b"EUR").unwrap();
        std::fs::write(dir.join("Images").join("Item").join("1000.jpg"), b"jpg").unwrap();
        let mut fs = ResourceFiles::new(dir.clone());
        assert_eq!(fs.read("CurrencyData.txt").as_deref(), Some(&b"EUR"[..]));
        assert_eq!(fs.read("images\\item\\1000.JPG").as_deref(), Some(&b"jpg"[..]));
        assert!(fs.exists("Images/Item/1000.jpg"));
        assert!(!fs.exists("Images"), "a folder is no file");
        assert!(!fs.exists("..\\CurrencyData.TXT") && !fs.exists("/etc/passwd"));
        fs.write("CreateDemoData.log", b"log").unwrap();
        assert_eq!(fs.read("createdemodata.log").as_deref(), Some(&b"log"[..]));
        assert!(
            !dir.join("CreateDemoData.log").exists(),
            "nothing is written to the folder"
        );
        assert!(fs.remove("CurrencyData.txt"));
        assert!(!fs.exists("CurrencyData.txt"));
        assert!(
            dir.join("CurrencyData.TXT").exists(),
            "a removed resource stays on disk"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_blocked_reason_is_read_back_from_the_notes() {
        let notes = vec![
            "Company-Initialize.OnRun did not run to its end: x".to_string(),
            format!("{BLOCKED_NOTE}Create Demo Data from Config.OnRun stopped: y"),
        ];
        assert_eq!(
            blocked(&notes).as_deref(),
            Some("the company data was not built: Create Demo Data from Config.OnRun stopped: y")
        );
        assert!(blocked(&notes[..1]).is_none());
    }
}
