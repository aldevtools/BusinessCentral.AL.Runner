//! The base branch's oracle and not-passing lists, read with `git show REV:PATH` (ADR-065): `--list-new-cases`
//! adds lines only for cases the base oracle lacks, and `--list-changes` tells such lines apart from regressions.

use crate::config::{OracleFile, OracleFormat};
use crate::oracle::{self, CaseId};
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
        let cases = match format {
            OracleFormat::Junit => oracle::parse_junit(&text),
            OracleFormat::Xunit => oracle::parse_xunit(&text).map(|v| v.into_iter().map(|c| c.case).collect()),
        }
        .map_err(|e| format!("{}:{path}: {e}", self.rev))?;
        Ok(crate::not_passing::bc_passed(&cases))
    }

    /// The list file at the revision; empty when it has none.
    pub fn list(&self, path: &str) -> Result<BTreeSet<CaseId>, String> {
        match self.show(path)? {
            Some(text) => crate::not_passing::parse(&text).map_err(|e| format!("{}:{path}: {e}", self.rev)),
            None => Ok(BTreeSet::new()),
        }
    }
}
