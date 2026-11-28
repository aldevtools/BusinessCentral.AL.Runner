//! The files of the corpus citation check (ADR-065 amendment 2): the corpus sources at the checkout or at a revision,
//! the repository texts that cite them (`crates/`, `docs/`), and the corpus pull requests the assumption registry
//! names. `citation.rs` owns what a citation means. Entry points: `check` (`--check-citations`), `rewrite_all`
//! (`--rewrite-citations REV`, run by the corpus bump).

use crate::citation::{self, Corpus};
use crate::config::OracleFile;
use std::collections::BTreeSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The folders whose texts may cite the corpus.
const CITING: &[&str] = &["crates", "docs"];
/// The folder of `tests/` that holds no app: the must-not-compile cases (`mnc.rs`), each compiled alone, keyed by
/// their file stem, not by a codeunit.
const NOT_AN_APP: &str = "must-not-compile";

/// The corpus sources of the checkout `corpus` (`tests/<app>/**/*.al`).
pub fn corpus_at_checkout(corpus: &Path) -> Result<Corpus, String> {
    let tests = corpus.join("tests");
    let mut files = Vec::new();
    crate::corpus::walk(&tests, &mut files).map_err(|e| format!("{}: {e}", tests.display()))?;
    files.sort();
    let mut sources = Vec::with_capacity(files.len());
    for f in files {
        let rel = f.strip_prefix(&tests).unwrap_or(&f);
        let app = rel
            .components()
            .next()
            .map(|c| c.as_os_str().to_string_lossy().into_owned());
        if app.as_deref() == Some(NOT_AN_APP) {
            continue;
        }
        let text = std::fs::read_to_string(&f).map_err(|e| format!("{}: {e}", f.display()))?;
        sources.push((app.unwrap_or_default(), text));
    }
    Corpus::from_sources(sources)
}

/// The corpus sources at revision `rev` of the corpus repository `corpus`, read with git.
pub fn corpus_at_rev(corpus: &Path, rev: &str) -> Result<Corpus, String> {
    let ls = crate::base_oracle::git(corpus, &["ls-tree", "-r", "-z", "--name-only", rev, "--", "tests"])?;
    if !ls.status.success() {
        return Err(format!(
            "`{rev}` is not a revision of {}: {}",
            corpus.display(),
            String::from_utf8_lossy(&ls.stderr)
        ));
    }
    let paths: Vec<String> = String::from_utf8_lossy(&ls.stdout)
        .split('\0')
        .filter(|p| p.ends_with(".al") && !p.starts_with(&format!("tests/{NOT_AN_APP}/")))
        .map(str::to_string)
        .collect();
    let mut child = Command::new("git")
        .arg("-C")
        .arg(corpus)
        .args(["cat-file", "--batch"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| format!("git cat-file: {e}"))?;
    let mut stdin = child.stdin.take().ok_or("git cat-file: no stdin")?;
    let request: String = paths.iter().map(|p| format!("{rev}:{p}\n")).collect();
    let writer = std::thread::spawn(move || stdin.write_all(request.as_bytes()));
    let out = child.wait_with_output().map_err(|e| format!("git cat-file: {e}"))?;
    writer
        .join()
        .map_err(|_| "git cat-file: writer panicked")?
        .map_err(|e| format!("git cat-file: {e}"))?;
    // Each object: `<sha> blob <size>\n<bytes>\n`.
    let (mut sources, mut at, b) = (Vec::with_capacity(paths.len()), 0, &out.stdout);
    for p in &paths {
        let nl = b[at..]
            .iter()
            .position(|&c| c == b'\n')
            .ok_or("git cat-file: short output")?
            + at;
        let head = String::from_utf8_lossy(&b[at..nl]);
        let size: usize = head
            .rsplit(' ')
            .next()
            .and_then(|s| s.parse().ok())
            .ok_or(format!("git cat-file: {head}"))?;
        let text = String::from_utf8_lossy(&b[nl + 1..nl + 1 + size]).into_owned();
        at = nl + 1 + size + 1;
        let app = p
            .strip_prefix("tests/")
            .and_then(|r| r.split('/').next())
            .unwrap_or_default();
        sources.push((app.to_string(), text));
    }
    Corpus::from_sources(sources)
}

/// The corpus pull requests the assumption registry names: a pending citation must cite one of them.
pub fn registry_prs() -> BTreeSet<u32> {
    let mut prs = BTreeSet::new();
    for a in al_builtins::assumptions::Assumption::ALL {
        let i = a.info();
        for text in [i.assumed, i.tests, i.probe] {
            prs.extend(corpus_prs(text));
        }
    }
    prs
}

/// The corpus pull requests `text` names: `corpus PR <n>`, `corpus PR #<n>` or `corpus pull request <n>`, any case,
/// with `corpus` right before. A bare `PR <n>` is not read: the registry also names runner pull requests (review r1 of
/// PR 9432, M1).
fn corpus_prs(text: &str) -> Vec<u32> {
    let lower = text.to_ascii_lowercase();
    let mut out = Vec::new();
    for form in ["corpus pr ", "corpus pull request "] {
        for (k, _) in lower.match_indices(form) {
            let rest = lower[k + form.len()..].trim_start_matches('#');
            let n = rest.bytes().take_while(u8::is_ascii_digit).count();
            if let Ok(pr) = rest[..n].parse() {
                out.push(pr);
            }
        }
    }
    out
}

/// Every citation in `crates/` and `docs/` of `root` that does not resolve at the checkout `corpus`, as
/// `path:line: token: reason`, and the number of citations read.
pub fn check(root: &Path, corpus: &Path) -> Result<(Vec<String>, usize), String> {
    let pinned = corpus_at_checkout(corpus)?;
    let prs = registry_prs();
    let (mut errors, mut read) = (Vec::new(), 0);
    for (path, text) in citing_texts(root)? {
        for f in citation::find(&text) {
            read += 1;
            let line = text[..f.at.start].matches('\n').count() + 1;
            let token = &text[f.at.clone()];
            let r = f.citation.and_then(|c| {
                citation::resolve(&c, &pinned, &prs)?;
                // A token is written as it prints (review r1 of PR 9432, L2), so a search for it finds it.
                match citation::canonical(&c, &pinned)? {
                    Some(k) if k.to_string() != token => Err(format!("write it as {k}")),
                    _ => Ok(()),
                }
            });
            if let Err(e) = r {
                errors.push(format!("{}:{line}: {token}: {e}", path.display()));
            }
        }
    }
    Ok((errors, read))
}

/// Rewrites the citations of `crates/` and `docs/` from the corpus at revision `old_rev` to the checkout `corpus`
/// (`citation::rewrite`, the rename map of the checkout). Returns the files it wrote and the notes.
pub fn rewrite_all(root: &Path, cfg: &OracleFile, old_rev: &str) -> Result<(Vec<PathBuf>, Vec<String>), String> {
    let corpus = root.join(&cfg.corpus_repo);
    let old = corpus_at_rev(&corpus, old_rev)?;
    let new = corpus_at_checkout(&corpus)?;
    let map = crate::base_oracle::rename_map_at_head(root, cfg)?;
    let (mut written, mut notes) = (Vec::new(), Vec::new());
    for (path, text) in citing_texts(root)? {
        let (out, n) = citation::rewrite(&text, &old, &new, &map);
        notes.extend(n.into_iter().map(|n| format!("{}: {n}", path.display())));
        if out != text {
            std::fs::write(root.join(&path), out).map_err(|e| format!("{}: {e}", path.display()))?;
            written.push(path);
        }
    }
    Ok((written, notes))
}

/// The UTF-8 files under `crates/` and `docs/` of `root` that hold `corpus` (paths relative to `root`), build output
/// and hidden folders skipped.
fn citing_texts(root: &Path) -> Result<Vec<(PathBuf, String)>, String> {
    let mut files = Vec::new();
    for dir in CITING {
        walk(&root.join(dir), &mut files)?;
    }
    let mut out = Vec::new();
    for f in files {
        // A file that is not UTF-8 (an app package, an image) holds no citation.
        if let Ok(text) = std::fs::read_to_string(&f)
            && text.contains(citation::PREFIX)
        {
            out.push((f.strip_prefix(root).unwrap_or(&f).to_path_buf(), text));
        }
    }
    Ok(out)
}

/// Every file under `dir`, sorted; `target` and hidden folders skipped.
fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .collect();
    entries.sort();
    for p in entries {
        let name = p
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if p.is_dir() {
            if name != "target" && !name.starts_with('.') {
                walk(&p, out)?;
            }
        } else {
            out.push(p);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_pull_request_named_as_a_corpus_one_is_read() {
        assert_eq!(corpus_prs("runner PR 7870; see PR 8541"), Vec::<u32>::new());
        assert_eq!(
            corpus_prs("Corpus PR 1594 (x), corpus PR #1600, Corpus pull request 1633"),
            vec![1594, 1600, 1633]
        );
    }
}
