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
    /// The kind of server the company is built on, if not the run's own (`--environment`): the dependency installs,
    /// `Company-Initialize` and the codeunit run on it, the tests and the install code of the apps under test do not.
    pub environment: Option<al_host::Environment>,
    /// The names of the apps installed before the company is built (matched without case), if not every app: the
    /// install code of the other apps runs after the codeunit, and their event subscribers do not run before that.
    pub installed_first: Option<Vec<String>>,
}

/// The id of the DemoTool's `Create Demo Data from Config` (BCApps `src/Layers/W1/DemoTool/CreateDemoDatafromConfig.Codeunit.al`).
pub const DEMOTOOL_CODEUNIT: u32 = 101_899;
/// Its name.
pub const DEMOTOOL_CODEUNIT_NAME: &str = "Create Demo Data from Config";

/// The note of a baseline whose company data codeunit did not run to its end; it survives the kept baseline as text.
pub const BLOCKED_NOTE: &str = "the company data was not built: ";

/// The note of a `Message` the company data codeunit raised. Orchestrator decision (2026-10-22): BCApps' CI runs the
/// DemoTool's codeunit 101899 without a client (`Invoke-LegacyDemoDataTool`), and 101900 ends with a Message
/// (CreateDemonstrationData.Codeunit.al:133). While the company is built, and only then, a Message is accepted and its text
/// written to the run log; Confirm and StrMenu stay refused, and so does a Message of any other install code or a test.
/// Not language semantics, so no probe: what the runner shows is the runner's choice.
pub const MESSAGE_NOTE: &str = "the company data codeunit showed a message: ";

/// Why the company is blocked, from the notes of a baseline run.
pub fn blocked(notes: &[String]) -> Option<String> {
    notes
        .iter()
        .find_map(|n| n.strip_prefix(BLOCKED_NOTE))
        .map(|why| format!("{BLOCKED_NOTE}{why}"))
}

/// The files of one folder, read-only, plus what the code writes, in memory. AL paths are relative to the folder,
/// with `\` or `/` between the parts. A part that names no file exactly is matched without case, as the Windows
/// server BCApps' CI runs the DemoTool on does (the DemoTool names `CurrencyData.txt`, the folder holds
/// `CurrencyData.TXT`). Why that is an assumption: AL code can see it (File.Exists and File.Open of a name in other
/// case answer differently on a case-sensitive file system), no corpus test pins it (the corpus CI runs BC on Linux,
/// and the rule is the server's file system, not the language), and Microsoft's Legacy tests need it to get their
/// company. So the match without case marks `resource-file-name-case` (assumptions.rs) when it is used, and the
/// tests that run in the company are flagged passes. A path that is absolute or climbs out with `..` names no file
/// of the folder.
pub struct ResourceFiles {
    root: PathBuf,
    written: MemFileSystem,
    removed: BTreeSet<String>,
    /// The folders AL code created in the temporary folder (`key` form, no closing separator).
    dirs: BTreeSet<String>,
    /// Per written file (`key` form): when it was created (a counter; a rewrite keeps it, as NTFS keeps a file's creation time)
    /// and the path as AL code first spelled it.
    created: std::collections::BTreeMap<String, (u64, String)>,
    next_created: u64,
}

/// The temporary folder, in `key` form without the closing separator: what `TemporaryPath()` names (`\TEMP\`, al-vm
/// hostvalues.rs), and the folder of `File.CreateTempFile`'s names (`temp/`, al-builtins file.rs `TEMP_DIR`), which
/// FileManagement.ServerTempFileName returns and Config. XML Exchange puts its media folder next to.
const TEMP_ROOTS: [&str; 2] = ["\\temp", "temp"];

impl ResourceFiles {
    pub fn new(root: PathBuf) -> ResourceFiles {
        ResourceFiles {
            root,
            written: MemFileSystem::default(),
            removed: BTreeSet::new(),
            dirs: BTreeSet::new(),
            created: Default::default(),
            next_created: 0,
        }
    }

    /// `key` of a folder: without the closing separator.
    fn dir_key(path: &str) -> String {
        Self::key(path).trim_end_matches('\\').to_string()
    }

    /// Whether `path` is inside the temporary folder (or is it).
    fn in_temp(path: &str) -> bool {
        Self::temp_root(&Self::dir_key(path)).is_some()
    }

    /// The temporary folder that `k` (a `dir_key`) is in, or is.
    fn temp_root(k: &str) -> Option<&'static str> {
        TEMP_ROOTS
            .into_iter()
            .find(|r| k == *r || k.starts_with(&format!("{r}\\")))
    }

    fn key(path: &str) -> String {
        Self::from_user_folder(path).replace('/', "\\").to_lowercase()
    }

    /// `path` relative to the user folder. BCApps copies the resources into the session's user folder, the folder above
    /// `TemporaryPath()` (`\TEMP\` here, al-vm hostvalues.rs), after the page opens (build/scripts/
    /// ImportTestDataInBcContainer.ps1, `Invoke-LegacyDemoDataTool`), and the DemoTool reads them as
    /// `TemporaryPath() + '..\WalkMeTour'` or `TemporaryPath() + '\..\' + 'PostingExchangeDefinitions'`. So a path that
    /// climbs out of the temporary folder names the user folder. Only the lookup changes: the text AL code holds is left
    /// as it is (File.IsPathTemporary refuses such a path, al-builtins file.rs).
    fn from_user_folder(path: &str) -> std::borrow::Cow<'_, str> {
        let mut parts = path.split(['\\', '/']).filter(|p| !p.is_empty());
        let rooted = path.starts_with(['\\', '/']);
        match (parts.next(), parts.next()) {
            (Some(t), Some("..")) if rooted && t.eq_ignore_ascii_case("TEMP") => {
                parts.collect::<Vec<_>>().join("\\").into()
            }
            _ => path.into(),
        }
    }

    /// The file of the folder that `path` names, matched part by part without case.
    fn resolve(&self, path: &str) -> Option<PathBuf> {
        if self.removed.contains(&Self::key(path)) {
            return None;
        }
        self.resolve_entry(path).filter(|p| p.is_file())
    }

    /// The file or folder of the folder that `path` names, matched part by part without case.
    fn resolve_entry(&self, path: &str) -> Option<PathBuf> {
        let rel = Self::from_user_folder(path).replace('\\', "/");
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
                        let found = std::fs::read_dir(&out)
                            .ok()?
                            .filter_map(|e| e.ok())
                            .find(|e| e.file_name().to_string_lossy().to_lowercase() == want)?
                            .path();
                        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::ResourceFileNameCase);
                        found
                    };
                    any = true;
                }
                Component::CurDir => {}
                _ => return None,
            }
        }
        any.then_some(out)
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
        let k = Self::key(path);
        self.removed.remove(&k);
        if !self.created.contains_key(&k) {
            self.next_created += 1;
            self.created.insert(k.clone(), (self.next_created, path.to_string()));
        }
        self.written.write(&k, bytes)
    }
    fn exists(&self, path: &str) -> bool {
        self.written.exists(&Self::key(path)) || self.resolve(path).is_some()
    }
    /// Only a folder reached by climbing out of the temporary folder (`TemporaryPath() + '..\WalkMeTour'`, the form the
    /// DemoTool uses) and only while AL code has neither written nor removed a file in it: then it holds exactly what
    /// was staged. The paths are `dir` joined with each name by `/`: Windows BC joins with `\`, but the DemoTool cuts
    /// the names out again with System.IO.Path (File Management.GetFileName, GetFileNameWithoutExtension, GetExtension),
    /// which the runtime's .NET runs on Linux, where only `/` separates. Windows .NET takes `/` as well, so the names AL
    /// code gets are the ones Windows BC gives it; the joined text is only seen inside the company build.
    fn staged_folder_files(&self, dir: &str) -> Option<Vec<String>> {
        if matches!(Self::from_user_folder(dir), std::borrow::Cow::Borrowed(_)) {
            return None;
        }
        let folder = self.resolve_entry(dir).filter(|p| p.is_dir())?;
        let prefix = format!("{}\\", Self::key(dir).trim_end_matches('\\'));
        if self
            .written
            .paths()
            .iter()
            .chain(&self.removed)
            .any(|k| k.starts_with(&prefix))
        {
            return None;
        }
        let mut names: Vec<String> = std::fs::read_dir(folder)
            .ok()?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort_by_key(|n| n.to_uppercase());
        let dir = dir.trim_end_matches(['\\', '/']);
        Some(names.into_iter().map(|n| format!("{dir}/{n}")).collect())
    }
    /// Only a folder AL code created in the temporary folder: the files written directly in it, in name order without case
    /// (NTFS lists by name), each as the folder joined with `/` to the file name as first written (the same join as
    /// `staged_folder_files`, for the runtime's .NET System.IO.Path). Windows BC 29.0 lists the files MediaSet.ExportFile wrote
    /// there (windows-check-results 2026-10-23 RJ item B; corpus tests/al-language-onprem/media/MediaFiles.al:772,
    /// RJ_MediaSet_ExportFile_ConfigPackageShape, cu 61286); in that run creation order and name order agreed, so a folder
    /// where they differ is an `Err` (the caller refuses it). The caller marks `company-build-folder-listing-name-order`.
    fn temp_folder_files(&self, dir: &str) -> Option<Result<Vec<String>, String>> {
        let k = Self::dir_key(dir);
        if !self.dirs.contains(&k) {
            return None;
        }
        let prefix = format!("{k}\\");
        let mut files: Vec<(&u64, String)> = self
            .created
            .iter()
            .filter(|(key, _)| key.strip_prefix(&prefix).is_some_and(|rest| !rest.contains('\\')))
            .map(|(_, (n, path))| (n, path.rsplit(['\\', '/']).next().unwrap_or_default().to_string()))
            .collect();
        files.sort_by_key(|(_, name)| name.to_uppercase());
        if !files.is_sorted_by_key(|(n, _)| **n) {
            return Some(Err(
                "the files' creation order and name order differ (which one Windows BC lists by is not pinned)".into(),
            ));
        }
        let dir = dir.trim_end_matches(['\\', '/']);
        Some(Ok(files.into_iter().map(|(_, name)| format!("{dir}/{name}")).collect()))
    }
    /// The temporary folder and what is in it, a folder staged in the user folder, and the empty text (Directory.Exists('')
    /// is No: QxDotNetDemoShapes.al:281). Any other path (another drive, a relative one) is not served.
    fn serves_path(&self, path: &str) -> bool {
        path.is_empty() || Self::in_temp(path) || matches!(Self::from_user_folder(path), std::borrow::Cow::Owned(_))
    }
    /// The temporary folder exists, a folder AL code created in it until it is removed, and a staged folder of the user
    /// folder (QxDotNetDemoShapes.al:271-291: missing No, created Yes, with a closing separator Yes, a file No, '' No,
    /// TemporaryPath Yes, deleted No).
    fn dir_exists(&self, path: &str) -> bool {
        if path.is_empty() {
            return false;
        }
        let k = Self::dir_key(path);
        if TEMP_ROOTS.contains(&k.as_str()) || self.dirs.contains(&k) {
            return true;
        }
        matches!(Self::from_user_folder(path), std::borrow::Cow::Owned(_))
            && self.resolve_entry(path).is_some_and(|p| p.is_dir())
    }
    /// A folder of the temporary folder, with the folders above it (FileManagement.ServerCreateDirectory of a nested folder:
    /// QxDotNetDemoShapes.al:304-307). A file of that name is refused (the .NET exception is not pinned).
    fn create_dir(&mut self, path: &str) -> Result<(), String> {
        if !Self::in_temp(path) {
            return Err("a folder outside the temporary folder".into());
        }
        let k = Self::dir_key(path);
        if self.exists(path) {
            return Err("a file of that name exists (the .NET exception is not pinned)".into());
        }
        let mut at = k.as_str();
        let root = Self::temp_root(&k).unwrap_or_default();
        while at.len() > root.len() {
            self.dirs.insert(at.to_string());
            at = &at[..at.rfind('\\').unwrap_or(0)];
        }
        Ok(())
    }
    /// Removes a folder AL code created, recursively with its folders and files (QxDotNetDemoShapes.al:317-325), or
    /// an empty one without (:326-327). A missing folder, or one that holds something without `recursive`, raises a .NET
    /// exception no test pins: refused.
    fn remove_dir(&mut self, path: &str, recursive: bool) -> Result<(), String> {
        let k = Self::dir_key(path);
        if !self.dirs.contains(&k) {
            return Err("the folder does not exist (the .NET exception is not pinned)".into());
        }
        let prefix = format!("{k}\\");
        let files: Vec<String> = self
            .written
            .paths()
            .into_iter()
            .filter(|p| p.starts_with(&prefix))
            .collect();
        let below: Vec<String> = self.dirs.iter().filter(|d| d.starts_with(&prefix)).cloned().collect();
        if !recursive && !(files.is_empty() && below.is_empty()) {
            return Err("the folder is not empty (the .NET exception is not pinned)".into());
        }
        for f in files {
            self.written.remove(&f);
            self.created.remove(&f);
        }
        for d in below {
            self.dirs.remove(&d);
        }
        self.dirs.remove(&k);
        Ok(())
    }
    fn remove(&mut self, path: &str) -> bool {
        let was = self.exists(path);
        self.written.remove(&Self::key(path));
        self.created.remove(&Self::key(path));
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

    // `resource-file-name-case` (assumptions.rs): only a name that matches without case, not exactly, marks it.
    #[test]
    fn a_resource_file_found_only_without_case_marks_its_assumption() {
        use al_builtins::assumptions::{Assumption, take_hits};
        let dir = std::env::temp_dir().join(format!("alr-rescase-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("CurrencyData.TXT"), b"EUR").unwrap();
        let fs = ResourceFiles::new(dir.clone());
        take_hits();
        assert!(fs.exists("CurrencyData.TXT"));
        assert!(take_hits().is_empty(), "an exact name takes no assumption");
        assert!(fs.read("CurrencyData.txt").is_some());
        assert_eq!(take_hits(), vec![Assumption::ResourceFileNameCase]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    // BCApps copies the resources into the session's user folder, the folder above TemporaryPath()
    // (ImportTestDataInBcContainer.ps1, Invoke-LegacyDemoDataTool), and the DemoTool reads them as
    // `TemporaryPath() + '..\WalkMeTour'` and `TemporaryPath() + '\..\' + 'PostingExchangeDefinitions'`.
    #[test]
    fn a_path_that_climbs_out_of_the_temporary_folder_names_a_resource() {
        let dir = std::env::temp_dir().join(format!("alr-restemp-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("WalkMeTour")).unwrap();
        std::fs::write(dir.join("WalkMeTour").join("a.txt"), b"a").unwrap();
        let fs = ResourceFiles::new(dir.clone());
        assert_eq!(fs.read("\\TEMP\\..\\WalkMeTour\\a.txt").as_deref(), Some(&b"a"[..]));
        assert!(
            fs.exists("\\TEMP\\\\..\\WalkMeTour\\a.txt"),
            "the doubled separator of `TemporaryPath() + '\\..\\'`"
        );
        assert!(
            !fs.exists("\\TEMP\\..\\..\\WalkMeTour\\a.txt"),
            "no climbing above the user folder"
        );
        assert!(!fs.exists("\\Other\\..\\WalkMeTour\\a.txt"));
        assert!(
            !fs.exists("\\TEMP\\WalkMeTour\\a.txt"),
            "the temporary folder holds no resource"
        );
        assert!(!fs.exists("WalkMeTour\\..\\WalkMeTour\\a.txt"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    // Create Getting Started Data lists `TemporaryPath() + '..\WalkMeTour'` (File Management.GetServerDirectoryFilesList,
    // DotNet Directory.GetFiles). Windows BC lists the files, without case, in name order (windows-check-results
    // 2026-10-22 QX item 14); subfolders are not files.
    #[test]
    fn a_staged_folder_lists_its_files_in_name_order_and_no_other_folder_is_listed() {
        let dir = std::env::temp_dir().join(format!("alr-reslist-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("WalkMeTour").join("SetupDevice")).unwrap();
        for n in ["c.png", "B.png", "a.png"] {
            std::fs::write(dir.join("WalkMeTour").join(n), b"x").unwrap();
        }
        let mut fs = ResourceFiles::new(dir.clone());
        assert_eq!(
            fs.staged_folder_files("\\TEMP\\..\\WalkMeTour"),
            Some(vec![
                "\\TEMP\\..\\WalkMeTour/a.png".to_string(),
                "\\TEMP\\..\\WalkMeTour/B.png".to_string(),
                "\\TEMP\\..\\WalkMeTour/c.png".to_string()
            ])
        );
        assert_eq!(
            fs.staged_folder_files("\\TEMP\\..\\walkmetour\\").map(|f| f[0].clone()),
            Some("\\TEMP\\..\\walkmetour/a.png".to_string())
        );
        for other in [
            "WalkMeTour",
            "\\TEMP\\..\\Missing",
            "\\TEMP\\..\\WalkMeTour\\a.png",
            "\\TEMP\\WalkMeTour",
            "/etc",
        ] {
            assert_eq!(fs.staged_folder_files(other), None, "{other}");
        }
        fs.write("\\TEMP\\..\\WalkMeTour\\d.png", b"d").unwrap();
        assert_eq!(
            fs.staged_folder_files("\\TEMP\\..\\WalkMeTour"),
            None,
            "a folder AL code changed is no longer only what was staged"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    // The folders of the temporary folder, as static System.IO.Directory sees them on real BC (tests/al-language-onprem-dotnet/
    // lt-qx-dotnet/QxDotNetDemoShapes.al:265-292 RD_DirectoryExists_Static, :294-346 RD_FileManagement_ServerFolderOperations).
    #[test]
    fn folders_of_the_temporary_folder_are_created_and_removed_in_memory() {
        let dir = std::env::temp_dir().join(format!("alr-resdirs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("WalkMeTour")).unwrap();
        let mut fs = ResourceFiles::new(dir.clone());
        let path = "\\TEMP\\ltrd1";
        let media = format!("{path}/Media");
        assert!(fs.serves_path(path) && fs.serves_path("") && fs.serves_path("\\TEMP\\..\\WalkMeTour"));
        assert!(!fs.serves_path("C:\\Windows") && !fs.serves_path("relative.txt"));
        assert!(!fs.dir_exists(path), "missing");
        assert!(fs.dir_exists("\\TEMP\\"), "TemporaryPath");
        assert!(!fs.dir_exists(""), "empty text");
        assert!(fs.dir_exists("\\TEMP\\..\\WalkMeTour"), "a staged folder");
        fs.create_dir(&media).unwrap();
        assert!(fs.dir_exists(&media) && fs.dir_exists(path), "create nested");
        assert!(fs.dir_exists(&format!("{path}/")), "trailing slash");
        fs.create_dir(&media).unwrap();
        fs.write(&format!("{media}/a.txt"), b"x").unwrap();
        assert!(!fs.dir_exists(&format!("{media}/a.txt")), "a file path");
        assert!(
            fs.remove_dir(path, false).is_err(),
            "a folder that is not empty, not recursive: not pinned"
        );
        fs.remove_dir(&media, true).unwrap();
        assert!(
            !fs.dir_exists(&media) && !fs.exists(&format!("{media}/a.txt")),
            "recursive removes files"
        );
        fs.remove_dir(path, false).unwrap();
        assert!(!fs.dir_exists(path), "remove empty non-recursive");
        assert!(fs.remove_dir(path, true).is_err(), "a missing folder: not pinned");
        assert!(
            fs.create_dir("\\TEMP\\..\\Other").is_err(),
            "only inside the temporary folder"
        );
        // The folder of File.CreateTempFile's names (`temp/1.tmp`, al-builtins file.rs TEMP_DIR), as FileManagement.
        // ServerTempFileName and Config. XML Exchange's media folder use it.
        assert!(fs.serves_path("temp/Media") && fs.dir_exists("temp") && !fs.dir_exists("temp/Media"));
        fs.create_dir("temp/Media").unwrap();
        assert!(fs.dir_exists("temp/Media/"));
        fs.remove_dir("temp/Media", true).unwrap();
        assert!(!fs.dir_exists("temp/Media") && fs.dir_exists("temp"));
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

    // Config. XML Exchange.UpdateConfigPackageMediaSet (ConfigXMLExchange.Codeunit.al:1505-1522) lists the Media folder the
    // DemoTool's MediaSet.ExportFile wrote into (File Management.GetServerDirectoryFilesList, DotNet Directory.GetFiles). Windows
    // BC 29.0 lists the files written there, the full path with no extension (windows-check-results 2026-10-23 RJ item B, cu 61286
    // RJ_MediaSet_ExportFile_ConfigPackageShape); its run could not tell creation order from name order, so only a folder where the
    // two agree is listed (`company-build-folder-listing-name-order`).
    #[test]
    fn a_folder_created_in_the_temporary_folder_lists_its_files_only_while_creation_and_name_order_agree() {
        let dir = std::env::temp_dir().join(format!("alr-restmplist-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut fs = ResourceFiles::new(dir.clone());
        let media = "\\TEMP\\pkg/Media";
        fs.create_dir(media).unwrap();
        assert_eq!(fs.temp_folder_files(media), Some(Ok(vec![])), "an empty folder");
        fs.write("\\TEMP\\pkg/Media\\{A1}-1", b"a").unwrap();
        fs.write("\\TEMP\\pkg/Media\\{B2}-1", b"b").unwrap();
        fs.write("\\TEMP\\pkg/Media\\{a1}-1", b"again").unwrap();
        fs.write("\\TEMP\\pkg/Media\\Sub\\x", b"x").unwrap();
        fs.write("\\TEMP\\pkg/other", b"o").unwrap();
        assert_eq!(
            fs.temp_folder_files("\\TEMP\\pkg/Media\\"),
            Some(Ok(vec![
                "\\TEMP\\pkg/Media/{A1}-1".to_string(),
                "\\TEMP\\pkg/Media/{B2}-1".to_string()
            ])),
            "the files in name order, as written first, not those of a folder below"
        );
        fs.write("\\TEMP\\pkg/Media\\{0}-1", b"z").unwrap();
        assert!(
            matches!(fs.temp_folder_files(media), Some(Err(_))),
            "created last, first by name: the order is not pinned"
        );
        assert!(fs.remove("\\TEMP\\pkg/Media\\{0}-1"));
        assert_eq!(fs.temp_folder_files(media).map(|r| r.map(|f| f.len())), Some(Ok(2)));
        for other in [
            "\\TEMP\\",
            "\\TEMP\\missing",
            "temp",
            "\\TEMP\\..\\WalkMeTour",
            "/etc",
            "",
        ] {
            assert_eq!(fs.temp_folder_files(other), None, "{other}");
        }
        fs.remove_dir("\\TEMP\\pkg", true).unwrap();
        assert_eq!(fs.temp_folder_files(media), None, "a removed folder");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
