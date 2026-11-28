//! The base branch's oracle and not-passing lists, read with `git show REV:PATH` (ADR-065): `--list-new-cases`
//! adds lines only for cases the base oracle lacks, and `--list-changes` tells such lines apart from regressions.
//! `Base::carry` reads both sides of a corpus pin move for `rename_map::carry` (ADR-065 amendment 1).
//!
//! The base branch's oracle and lists, read with `git show REV:PATH` (`--list-new-cases`, `--list-
//! changes`).

use crate::config::{OracleFile, OracleFormat};
use crate::oracle::{self, CaseId};
use crate::rename_map;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The repository at one revision (a commit, branch or tree; git's empty tree is a base without an oracle).
pub struct Base {
    root: PathBuf,
    rev: String,
    /// `oracle/oracle.toml` at the revision; `None` when it has none.
    cfg: Option<OracleFile>,
}

fn git(root: &Path, args: &[&str]) -> Result<std::process::Output, String> {
    Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|e| format!("git: {e}"))
}

impl Base {
    pub fn open(root: &Path, rev: &str) -> Result<Base, String> {
        let ok = git(root, &["rev-parse", "--verify", "-q", &format!("{rev}^{{tree}}")])?;
        if !ok.status.success() {
            return Err(format!(
                "`{rev}` is not a revision of {} (fetch the base branch first)",
                root.display()
            ));
        }
        let mut b = Base {
            root: root.to_path_buf(),
            rev: rev.to_string(),
            cfg: None,
        };
        if let Some(text) = b.show("oracle/oracle.toml")? {
            b.cfg = Some(OracleFile::parse(&text).map_err(|e| format!("{rev}:oracle/oracle.toml: {e}"))?);
        }
        Ok(b)
    }

    pub fn rev(&self) -> &str {
        &self.rev
    }

    /// The file at the revision, `None` when the revision does not have it.
    pub fn show(&self, path: &str) -> Result<Option<String>, String> {
        let spec = format!("{}:{path}", self.rev);
        if !git(&self.root, &["cat-file", "-e", &spec])?.status.success() {
            return Ok(None);
        }
        let out = git(&self.root, &["show", &spec])?;
        if !out.status.success() {
            return Err(format!(
                "git show {spec}: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
        Ok(Some(String::from_utf8_lossy(&out.stdout).into_owned()))
    }

    /// The cases real BC passes in the base oracle of `major` (the main app with `suite` `None`). Empty when the
    /// base has no such version, suite or file: then every case is new.
    pub fn oracle_ids(&self, major: u16, suite: Option<&str>) -> Result<BTreeSet<CaseId>, String> {
        let Some(v) = self
            .cfg
            .as_ref()
            .and_then(|c| c.versions.iter().find(|v| v.major == major))
        else {
            return Ok(BTreeSet::new());
        };
        let (file, format) = match suite {
            None => (v.junit.as_str(), OracleFormat::Junit),
            Some(name) => match v.suites.iter().find(|s| s.name == name) {
                Some(s) => (s.oracle.as_str(), s.format),
                None => return Ok(BTreeSet::new()),
            },
        };
        let path = format!("oracle/{file}");
        let Some(text) = self.show(&path)? else {
            return Ok(BTreeSet::new());
        };
        let cases = parse_oracle(&text, format).map_err(|e| format!("{}:{path}: {e}", self.rev))?;
        Ok(crate::not_passing::bc_passed(&cases))
    }

    /// The base lists of BC major `major` carried to the head oracles of `head` (read from `oracle_dir`, the folder of
    /// the head `oracle.toml`) through the head corpus's rename map (`rename_map::carry`, ADR-065 amendment 1).
    /// `main_list`: the main app's list path when a run names its own (`--baseline`).
    pub fn carry(
        &self,
        root: &Path,
        head: &OracleFile,
        oracle_dir: &Path,
        major: u16,
        main_list: Option<&str>,
    ) -> Result<rename_map::Carry, String> {
        let mut base = Vec::new();
        if let Some((cfg, v)) = self
            .cfg
            .as_ref()
            .and_then(|c| c.versions.iter().find(|v| v.major == major).map(|v| (c, v)))
        {
            let lists = std::iter::once((None, cfg.corpus_app.as_str(), main_list.unwrap_or(&v.baseline))).chain(
                v.suites
                    .iter()
                    .map(|s| (Some(s.name.as_str()), s.app.as_str(), s.baseline.as_str())),
            );
            for (suite, app, list) in lists {
                base.push(rename_map::BaseList {
                    suite: suite.map(str::to_string),
                    app: app.to_string(),
                    oracle: self.oracle_ids(major, suite)?,
                    list: self.list(list)?,
                });
            }
        }
        let mut heads = Vec::new();
        if let Some(v) = head.versions.iter().find(|v| v.major == major) {
            let oracles = std::iter::once((None, head.corpus_app.as_str(), v.junit.as_str(), OracleFormat::Junit))
                .chain(
                    v.suites
                        .iter()
                        .map(|s| (Some(s.name.as_str()), s.app.as_str(), s.oracle.as_str(), s.format)),
                );
            for (suite, app, file, format) in oracles {
                let path = oracle_dir.join(file);
                // A suite whose oracle file is missing has no cases: the run reports the missing file itself.
                let cases = match std::fs::read_to_string(&path) {
                    Ok(text) => parse_oracle(&text, format).map_err(|e| format!("{}: {e}", path.display()))?,
                    Err(_) => Vec::new(),
                };
                heads.push(rename_map::HeadOracle {
                    suite: suite.map(str::to_string),
                    app: app.to_string(),
                    cases: cases.into_iter().map(|c| c.id).collect(),
                });
            }
        }
        let map = rename_map_at_head(root, head)?;
        rename_map::carry(&map, &base, &heads).map_err(|e| format!("BC {major}: {e}"))
    }
}

/// The rename map of the corpus checkout (`<corpus_repo>/tests/rename-map.tsv`); empty when the corpus has none.
pub fn rename_map_at_head(root: &Path, head: &OracleFile) -> Result<rename_map::RenameMap, String> {
    let path = root.join(&head.corpus_repo).join("tests/rename-map.tsv");
    match std::fs::read_to_string(&path) {
        Ok(text) => rename_map::RenameMap::parse(&text).map_err(|e| format!("{}: {e}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(rename_map::RenameMap::default()),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

/// An oracle file's cases, in its format.
fn parse_oracle(text: &str, format: OracleFormat) -> Result<Vec<oracle::OracleCase>, oracle::OracleError> {
    match format {
        OracleFormat::Junit => oracle::parse_junit(text),
        OracleFormat::Xunit => oracle::parse_xunit(text).map(|v| v.into_iter().map(|c| c.case).collect()),
    }
}

impl Base {
    /// The list file at the revision; empty when it has none.
    pub fn list(&self, path: &str) -> Result<BTreeSet<CaseId>, String> {
        Ok(self.list_opt(path)?.unwrap_or_default())
    }

    /// The list at `path` in the base revision, `None` when the base revision has no such file.
    pub fn list_opt(&self, path: &str) -> Result<Option<BTreeSet<CaseId>>, String> {
        self.show(path)?
            .map(|text| crate::not_passing::parse(&text).map_err(|e| format!("{}:{path}: {e}", self.rev)))
            .transpose()
    }
}
