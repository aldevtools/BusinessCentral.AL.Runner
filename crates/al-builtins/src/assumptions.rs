//! The registry of behaviors the runtime implements on an unprobed assumption.
//!
//! Owner rule (CLAUDE.md hard rule 3, exception of 2026-10-15): behavior a Microsoft test needs may be
//! implemented before its corpus probe exists, but every test that passes through such behavior is a
//! FLAGGED pass: it relies on something no real-BC measurement has confirmed. Each such code path calls
//! [`assume`] with its entry of [`Assumption`] when it runs; the runner collects the entries a test hit and
//! reports the pass as flagged (console summary, JUnit property, JSON, `--explain`, `--top-assumptions`).
//!
//! One entry per assumption: its stable id, the behavior assumed, the Microsoft test(s) that need it and the
//! probe wanted. Each entry is a file of its own, `src/assumptions/<id>.rs` (`Variant => { id: ..., ... }`);
//! `build.rs` collects them in the order of their names into the `registry!` call below, so two pull requests
//! that each add an entry touch different files and merge without a conflict. `docs/probes-needed.md` holds a table generated from this registry (a test checks it; run
//! the test with `UPDATE_ASSUMPTIONS_DOC=1` to rewrite it). When a probe pins an assumption, delete its entry:
//! the compiler then names every place that still marks it.
//!
//! An entry for a user issue fixed before its corpus probe sets `awaiting`: the al-runner issue(s) and the corpus
//! file(s) that will pin it. A test requires it when the entry or its `docs/probes-needed.md` lines name an
//! al-runner issue; `al-test-cli --awaiting-assumptions` fails once every awaited file is in the pinned corpus.
//!
//! A test (`every_entry_is_marked_somewhere`) checks that each entry is used by at least one code path; an
//! unregistered marker does not compile.

use std::cell::Cell;

macro_rules! registry {
    ($( $(#[$m:meta])* $variant:ident => {
        id: $id:expr,
        assumed: $assumed:expr,
        tests: $tests:expr,
        probe: $probe:expr,
        $(note: $note:expr,)?
        $(awaiting: $awaiting:expr,)?
    } ),* $(,)?) => {
        /// A behavior implemented on an assumption no probe has confirmed.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
        #[repr(u8)]
        pub enum Assumption { $( $(#[$m])* $variant ),* }

        impl Assumption {
            /// Every entry, in registry order.
            pub const ALL: &'static [Assumption] = &[ $( Assumption::$variant ),* ];

            /// The entry's text.
            pub fn info(self) -> Info {
                match self {
                    $( Assumption::$variant => Info {
                        id: $id,
                        assumed: $assumed,
                        tests: $tests,
                        probe: $probe,
                        note: registry!(@note $($note)?),
                        awaiting: registry!(@awaiting $($awaiting)?),
                    } ),*
                }
            }
        }
    };
    (@note) => { None };
    (@note $n:expr) => { Some($n) };
    (@awaiting) => { None };
    (@awaiting $w:expr) => { Some($w) };
}

/// What the registry says about one assumption.
#[derive(Debug, Clone, Copy)]
pub struct Info {
    /// The stable id: lower case words with hyphens.
    pub id: &'static str,
    /// The behavior assumed, in one line.
    pub assumed: &'static str,
    /// The Microsoft test(s) that need it.
    pub tests: &'static str,
    /// The probe wanted.
    pub probe: &'static str,
    /// An open point about the entry (a probe that contradicts it, say).
    pub note: Option<&'static str>,
    /// The corpus pin that will replace the assumption, when one is on its way. Required for an entry that
    /// names an al-runner issue (a runtime fix for a user issue that landed before its probe); `al-test-cli
    /// --awaiting-assumptions` fails once every awaited file is in the pinned corpus.
    pub awaiting: Option<Awaiting>,
}

/// The corpus pin an assumption waits for.
#[derive(Debug, Clone, Copy)]
pub struct Awaiting {
    /// The al-runner issue number(s) the assumption serves; empty for a writer's probe that serves no issue.
    pub issues: &'static [u32],
    /// The corpus test file(s) that will pin it, relative to the corpus root (`tests/al-language/...al`); for a test
    /// that will be added to a file already in the corpus, `<file>::<test procedure>`.
    pub files: &'static [&'static str],
    /// The day the entry started waiting, `YYYY-MM-DD` (`--awaiting-assumptions --summary` lists the oldest first).
    pub since: &'static str,
}

/// One sentence on what `w` waits for, for the generated table and `--list-assumptions`.
pub fn awaiting_text(w: &Awaiting) -> String {
    let issues: Vec<String> = w.issues.iter().map(u32::to_string).collect();
    let files: Vec<String> = w.files.iter().map(|f| format!("`{f}`")).collect();
    if issues.is_empty() {
        return format!("Awaiting since {}: corpus {}.", w.since, files.join(", "));
    }
    format!(
        "Awaiting since {}: al-runner issue{} {}, corpus {}.",
        w.since,
        if w.issues.len() == 1 { "" } else { "s" },
        issues.join(", "),
        files.join(", ")
    )
}

include!(concat!(env!("OUT_DIR"), "/assumption_registry.rs"));

/// The words of a [`HitBits`]: room for 256 assumptions.
const HIT_WORDS: usize = 4;

/// The assumption marks as a bit set: bit `a as u32` for assumption `a`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HitBits(pub [u64; HIT_WORDS]);

impl HitBits {
    /// No marks.
    pub const EMPTY: HitBits = HitBits([0; HIT_WORDS]);

    /// The bit set with the mark of `a` added.
    #[inline]
    pub fn with(self, a: Assumption) -> HitBits {
        let mut w = self.0;
        let i = a as usize;
        w[i / 64] |= 1u64 << (i % 64);
        HitBits(w)
    }

    /// Whether the mark of `a` is set.
    #[inline]
    pub fn has(self, a: Assumption) -> bool {
        let i = a as usize;
        self.0[i / 64] & (1u64 << (i % 64)) != 0
    }
}

impl std::ops::BitOr for HitBits {
    type Output = HitBits;
    fn bitor(self, o: HitBits) -> HitBits {
        let mut w = self.0;
        for (a, b) in w.iter_mut().zip(o.0) {
            *a |= b;
        }
        HitBits(w)
    }
}

thread_local! {
    static HITS: Cell<HitBits> = const { Cell::new(HitBits::EMPTY) };
}

/// Notes that the code path of `a` ran. Cheap: one thread-local read and write. The runner collects the
/// marks of each test with [`take_hits`].
#[inline]
pub fn assume(a: Assumption) {
    HITS.with(|h| h.set(h.get().with(a)));
}

/// [`assume`] that answers true, for a match guard that decides to take the assumed path.
#[inline]
pub fn hit(a: Assumption) -> bool {
    assume(a);
    true
}

/// Forgets the marks of this thread (a test is about to start).
pub fn clear_hits() {
    HITS.with(|h| h.set(HitBits::EMPTY));
}

/// The marks of this thread as a bit set (bit `a as u32` for assumption `a`); clears them. With [`add_hit_bits`], a
/// caller that replays a recorded run of code puts back the marks that run made.
pub fn take_hit_bits() -> HitBits {
    HITS.with(|h| h.replace(HitBits::EMPTY))
}

/// Adds the marks of the bit set `bits` (from [`take_hit_bits`]) to this thread's marks.
pub fn add_hit_bits(bits: HitBits) {
    HITS.with(|h| h.set(h.get() | bits));
}

/// The marks of this thread since the last [`clear_hits`] or `take_hits`, in registry order; clears them.
pub fn take_hits() -> Vec<Assumption> {
    let bits = HITS.with(|h| h.replace(HitBits::EMPTY));
    Assumption::ALL.iter().copied().filter(|a| bits.has(*a)).collect()
}

/// The entry with this id.
pub fn by_id(id: &str) -> Option<Assumption> {
    Assumption::ALL.iter().copied().find(|a| a.info().id == id)
}

/// The registry as the table `docs/probes-needed.md` holds between its markers.
pub fn markdown_table() -> String {
    let mut s = String::from("| Id | Assumed | Microsoft tests that need it | Probe wanted |\n|---|---|---|---|\n");
    for a in Assumption::ALL {
        let i = a.info();
        let cell = |t: &str| t.replace('|', "\\|");
        let note = i.note.map(|n| format!(" **{}**", cell(n))).unwrap_or_default();
        let awaiting = i
            .awaiting
            .map(|w| format!(" {}", cell(&awaiting_text(&w))))
            .unwrap_or_default();
        s.push_str(&format!(
            "| `{}` | {}{} | {} | {}{} |\n",
            i.id,
            cell(i.assumed),
            note,
            cell(i.tests),
            cell(i.probe),
            awaiting
        ));
    }
    s
}

/// The line before the generated table in `docs/probes-needed.md`.
pub const PROBES_TABLE_BEGIN: &str = "<!-- assumptions:begin (generated from crates/al-builtins/src/assumptions.rs; UPDATE_ASSUMPTIONS_DOC=1 cargo test -p al-builtins assumptions rewrites it) -->";
/// The line after it.
pub const PROBES_TABLE_END: &str = "<!-- assumptions:end -->";

/// `docs/probes-needed.md` (`text`) with the table between its two markers replaced by
/// [`markdown_table`]; the rest of the file is kept. `al-test-cli --assumptions-doc` writes it, and the
/// regen bot on main runs that after every green push (`.github/workflows/ci.yml`, job `regen-bot`).
pub fn probes_needed_doc(text: &str) -> Result<String, String> {
    let (Some(a), Some(b)) = (text.find("<!-- assumptions:begin"), text.find(PROBES_TABLE_END)) else {
        return Err("docs/probes-needed.md has no assumptions block: add the two marker lines".into());
    };
    if b < a {
        return Err("docs/probes-needed.md: the end marker comes before the begin marker".into());
    }
    Ok(format!(
        "{}{PROBES_TABLE_BEGIN}\n{}{PROBES_TABLE_END}{}",
        &text[..a],
        markdown_table(),
        &text[b + PROBES_TABLE_END.len()..]
    ))
}

/// The registry for `--list-assumptions`; `counts` gives, per id, (flagged passes, other tests that hit it)
/// of the last run.
pub fn listing(counts: &dyn Fn(&str) -> Option<(usize, usize)>) -> String {
    let mut s = String::new();
    for a in Assumption::ALL {
        let i = a.info();
        let hits = match counts(i.id) {
            Some((flagged, other)) => format!("{flagged} flagged passes, {other} other tests"),
            None => "not hit in the last run".to_string(),
        };
        s.push_str(&format!(
            "{}  ({hits})\n    assumed: {}\n    needed by: {}\n    probe wanted: {}\n",
            i.id, i.assumed, i.tests, i.probe
        ));
        if let Some(n) = i.note {
            s.push_str(&format!("    NOTE: {n}\n"));
        }
        if let Some(w) = &i.awaiting {
            s.push_str(&format!("    {}\n", awaiting_text(w)));
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probes_needed_doc_replaces_only_the_table_between_the_markers() {
        let text = format!("# Probes\n\nintro\n{PROBES_TABLE_BEGIN}\n| old |\n{PROBES_TABLE_END}\n\nrest\n");
        let new = probes_needed_doc(&text).unwrap();
        assert_eq!(
            new,
            format!(
                "# Probes\n\nintro\n{PROBES_TABLE_BEGIN}\n{}{PROBES_TABLE_END}\n\nrest\n",
                markdown_table()
            )
        );
        // Current text comes back unchanged.
        assert_eq!(probes_needed_doc(&new).unwrap(), new);
        assert!(probes_needed_doc("no markers here").is_err());
    }

    #[test]
    fn marks_are_collected_per_thread_and_cleared() {
        clear_hits();
        assume(Assumption::ReadUncommittedReadOpensNoWriteTransaction);
        assume(Assumption::ReadUncommittedReadOpensNoWriteTransaction);
        assume(Assumption::CodeCoverageSnapshotWhileLogging);
        // In registry order, the order of the ids.
        assert_eq!(
            take_hits(),
            vec![
                Assumption::CodeCoverageSnapshotWhileLogging,
                Assumption::ReadUncommittedReadOpensNoWriteTransaction
            ]
        );
        assert!(take_hits().is_empty());
        assume(Assumption::ReadUncommittedReadOpensNoWriteTransaction);
        std::thread::spawn(|| assert!(take_hits().is_empty())).join().unwrap();
        clear_hits();
    }

    #[test]
    fn ids_are_unique_stable_words() {
        let mut ids: Vec<&str> = Assumption::ALL.iter().map(|a| a.info().id).collect();
        for id in &ids {
            assert!(
                !id.is_empty() && id.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
                "{id}"
            );
        }
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), Assumption::ALL.len());
        assert!(Assumption::ALL.len() <= 64 * HIT_WORDS, "the marks are a HitBits");
        for a in Assumption::ALL {
            assert_eq!(by_id(a.info().id), Some(*a));
        }
    }

    /// Each entry's file is named after its id, so two entries with one id are one file (a conflict git shows),
    /// and the registry order is the order of the ids.
    #[test]
    fn each_entry_is_the_file_named_after_its_id() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/assumptions");
        let mut names: Vec<String> = std::fs::read_dir(&dir)
            .expect("src/assumptions")
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        let ids: Vec<String> = Assumption::ALL.iter().map(|a| format!("{}.rs", a.info().id)).collect();
        assert_eq!(names, ids, "src/assumptions holds exactly one file `<id>.rs` per entry");
    }

    /// Every `*.rs` file under `crates/*/src`.
    fn sources() -> Vec<(std::path::PathBuf, String)> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let mut out = Vec::new();
        let mut stack = vec![root];
        while let Some(d) = stack.pop() {
            let Ok(rd) = std::fs::read_dir(&d) else { continue };
            for e in rd.flatten() {
                let p = e.path();
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if p.is_dir() {
                    if matches!(name, "target" | "tests" | ".git" | "corpus") {
                        continue;
                    }
                    stack.push(p);
                } else if name.ends_with(".rs") && p.to_string_lossy().contains("/src/") {
                    if let Ok(t) = std::fs::read_to_string(&p) {
                        out.push((p, t));
                    }
                }
            }
        }
        out
    }

    #[test]
    fn every_entry_is_marked_somewhere() {
        let files = sources();
        let mut unused = Vec::new();
        for a in Assumption::ALL {
            let needle = format!("Assumption::{a:?}");
            let used = files
                .iter()
                .filter(|(p, _)| {
                    !p.ends_with("assumptions.rs") && !p.parent().is_some_and(|d| d.ends_with("assumptions"))
                })
                .any(|(_, t)| t.contains(&needle));
            if !used {
                unused.push(a.info().id);
            }
        }
        assert!(
            unused.is_empty(),
            "registered but never marked by a code path (delete the entry or add the marker): {unused:?}"
        );
    }

    /// The al-runner issue numbers a text names: `al-runner issue 5806`, `al-runner issues 5606, 5641`,
    /// `al-runner 5808`, `al-runner #5808`.
    fn al_runner_issues(text: &str) -> Vec<u32> {
        let mut out = Vec::new();
        for (at, _) in text.match_indices("al-runner") {
            let rest = text[at + "al-runner".len()..].trim_start();
            let rest = rest
                .strip_prefix("issues")
                .or_else(|| rest.strip_prefix("issue"))
                .unwrap_or(rest)
                .trim_start();
            // A list of numbers: `5606, 5641`, `#5606 and #5641`.
            let mut s = rest;
            loop {
                let t = s.trim_start_matches(['#', ' ']);
                let digits: String = t.chars().take_while(char::is_ascii_digit).collect();
                let Ok(n) = digits.parse() else { break };
                out.push(n);
                s = &t[digits.len()..];
                s = s
                    .strip_prefix(", ")
                    .or_else(|| s.strip_prefix(" and "))
                    .or_else(|| s.strip_prefix(" or "))
                    .unwrap_or("");
            }
        }
        out
    }

    #[test]
    fn al_runner_issue_references_are_read() {
        assert_eq!(al_runner_issues("as BC does (al-runner issue 5806)"), vec![5806]);
        assert_eq!(
            al_runner_issues("al-runner issues 5606, 5641 and 5700"),
            vec![5606, 5641, 5700]
        );
        assert_eq!(al_runner_issues("Repeater (al-runner 5808)"), vec![5808]);
        assert_eq!(al_runner_issues("al-runner #5808"), vec![5808]);
        assert!(al_runner_issues("AL Runner v2 does it, al-runner-rs too").is_empty());
    }

    /// The problems of one entry's `awaiting`, given the al-runner issues its texts and docs name.
    fn awaiting_problems(id: &str, awaiting: Option<&Awaiting>, named: &[u32]) -> Vec<String> {
        let mut out = Vec::new();
        let Some(w) = awaiting else {
            if !named.is_empty() {
                out.push(format!(
                    "{id}: names al-runner issue(s) {named:?} but has no `awaiting` (a runtime fix for a user issue must say which corpus pin replaces it)"
                ));
            }
            return out;
        };
        // An entry may await a corpus pin that serves no al-runner issue: a writer's probe of an unprobed assumption (corpus PR 737 for
        // the HTTP entries, 2026-10-25). Its issues list is then empty.
        for n in named {
            if !w.issues.contains(n) {
                out.push(format!(
                    "{id}: names al-runner issue {n}, which `awaiting` does not list"
                ));
            }
        }
        if w.files.is_empty() {
            out.push(format!("{id}: `awaiting` names no corpus file"));
        }
        for f in w.files {
            // `file::Test`: one test of a file already in the corpus.
            let f = f.split_once("::").map_or(*f, |(f, _)| f);
            if !f.starts_with("tests/") || !f.ends_with(".al") || f.contains('\\') || f.contains("..") {
                out.push(format!(
                    "{id}: awaited corpus file `{f}` must be a path relative to the corpus root, `tests/<app>/.../<file>.al`"
                ));
            }
        }
        let b = w.since.as_bytes();
        let date = b.len() == 10
            && b[4] == b'-'
            && b[7] == b'-'
            && b.iter()
                .enumerate()
                .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit());
        if !date {
            out.push(format!("{id}: `awaiting.since` `{}` is not a date YYYY-MM-DD", w.since));
        }
        out
    }

    const SAMPLE: Awaiting = Awaiting {
        issues: &[5806],
        files: &["tests/al-language/bgtasks/lt-x/X.al"],
        since: "2026-10-24",
    };

    #[test]
    fn an_entry_that_names_an_al_runner_issue_must_await_its_pin() {
        assert_eq!(awaiting_problems("a", None, &[]), Vec::<String>::new());
        assert_eq!(awaiting_problems("a", None, &[5806]).len(), 1);
        assert!(awaiting_problems("a", Some(&SAMPLE), &[5806]).is_empty());
        assert_eq!(awaiting_problems("a", Some(&SAMPLE), &[5806, 5807]).len(), 1);
        let bad = Awaiting {
            issues: &[],
            files: &["corpus/tests/x.al", "bgtasks/X.al"],
            since: "24.10.2026",
        };
        assert_eq!(awaiting_problems("a", Some(&bad), &[]).len(), 3);
    }

    /// The lines of `docs/probes-needed.md` outside the generated table that name the entry `id` (in backticks).
    fn doc_lines_naming<'a>(doc: &'a str, id: &str) -> Vec<&'a str> {
        let needle = format!("`{id}`");
        let (a, b) = (doc.find("<!-- assumptions:begin"), doc.find(PROBES_TABLE_END));
        doc.lines()
            .filter(|l| l.contains(&needle))
            .filter(|l| {
                let at = l.as_ptr() as usize - doc.as_ptr() as usize;
                !matches!((a, b), (Some(a), Some(b)) if at >= a && at <= b)
            })
            .collect()
    }

    #[test]
    fn every_entry_that_names_an_al_runner_issue_awaits_its_pin() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/probes-needed.md");
        let doc = std::fs::read_to_string(&path).expect("docs/probes-needed.md");
        let mut problems = Vec::new();
        for a in Assumption::ALL {
            let i = a.info();
            let mut named = Vec::new();
            for t in [i.assumed, i.tests, i.probe, i.note.unwrap_or("")] {
                named.extend(al_runner_issues(t));
            }
            for l in doc_lines_naming(&doc, i.id) {
                named.extend(al_runner_issues(l));
            }
            named.sort();
            named.dedup();
            problems.extend(awaiting_problems(i.id, i.awaiting.as_ref(), &named));
        }
        assert!(problems.is_empty(), "{}", problems.join("\n"));
    }

    #[test]
    fn the_table_and_listing_name_what_an_entry_awaits() {
        let s = awaiting_text(&SAMPLE);
        assert_eq!(
            s,
            "Awaiting since 2026-10-24: al-runner issue 5806, corpus `tests/al-language/bgtasks/lt-x/X.al`."
        );
    }

    /// The table is generated. A pull request does not need to regenerate it: the regen bot on main does
    /// that after the merge (`.github/workflows/ci.yml`, job `regen-bot`). So a stale table is reported, not
    /// failed; `UPDATE_ASSUMPTIONS_DOC=1` rewrites it locally.
    #[test]
    fn the_table_in_probes_needed_is_generated_from_the_registry() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/probes-needed.md");
        let text = std::fs::read_to_string(&path).expect("docs/probes-needed.md");
        let want = probes_needed_doc(&text).unwrap_or_else(|e| panic!("{e}"));
        if want != text {
            if std::env::var_os("UPDATE_ASSUMPTIONS_DOC").is_some() {
                std::fs::write(&path, want).expect("write docs/probes-needed.md");
            } else {
                eprintln!(
                    "note: the assumptions table of docs/probes-needed.md is out of date; the regen bot updates it on main after the merge (locally: UPDATE_ASSUMPTIONS_DOC=1 cargo test -p al-builtins assumptions)"
                );
            }
        }
    }
}
