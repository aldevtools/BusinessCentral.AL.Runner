//! The order dependence part of a stability report (ADR-058 amendment 1, decisions 4 to 6): the test codeunits that
//! read SingleInstance state an earlier test codeunit of their BC session left (`CorpusRuntime::single_carried`),
//! split into "breaks with fresh SingleInstance state" (a flip under `isolation.singleInstance = fresh`, with its
//! reason) and "reads carried state, outcome unchanged" (`readsCarried`: "checked" when the run had a `fresh` variant,
//! else "not checked").

use al_testrun::runtime::CarriedCodeunit;
use al_testrun::stability::{Change, Kind, Seen};

/// The console part lists at most this many codeunits that read carried state with an unchanged outcome; the JSON
/// holds all (decision 6).
pub const CONSOLE_LIMIT: usize = 50;

/// The start of the message of a test the per-test time limit stopped (`al-testrun` `run.rs` `TIME_LIMIT_REASON`).
const TIME_LIMIT: &str = "the test used more than the time limit of ";

/// One codeunit that read carried state.
#[derive(Debug, Clone)]
pub struct Entry {
    pub c: CarriedCodeunit,
    /// Per read of `c`: the SingleInstance codeunit is in a dependency app (not one of the apps under test).
    pub dependency: Vec<bool>,
}

/// What the report knows of order dependence through SingleInstance state.
#[derive(Debug, Clone, Default)]
pub struct SiReport {
    pub entries: Vec<Entry>,
    /// The run had an `isolation.singleInstance = fresh` variant.
    pub checked: bool,
}

/// `Codeunit<id>.<test>` split.
fn split(test: &str) -> Option<(u32, &str)> {
    let (cu, t) = test.strip_prefix("Codeunit")?.split_once('.')?;
    Some((cu.parse().ok()?, t))
}

/// One line for a read: "reads Counter of SW Single 2 (app Stress Suite) that SW Test 50102 left".
pub fn read_line(r: &al_testrun::runtime::CarriedRead, dependency: bool) -> String {
    let app = if dependency {
        format!("app {} by {}, in a dependency app", r.app, r.publisher)
    } else {
        format!("app {}", r.app)
    };
    let writer = r
        .last_writer
        .as_ref()
        .map_or_else(|| "an earlier codeunit".to_string(), |(id, n)| format!("{n} ({id})"));
    format!("reads {} of {} ({app}) that {writer} left", r.global, r.single_name)
}

impl SiReport {
    /// From the base variant's list; `own_apps`: the ids of the apps under test.
    pub fn new<'a>(
        carried: impl Iterator<Item = &'a CarriedCodeunit>,
        own_apps: &[String],
        plan: &crate::stability::Plan,
    ) -> SiReport {
        let mut entries: Vec<Entry> = carried
            .map(|c| {
                // The reads of the team's own apps first, then those of dependency apps.
                let mut reads: Vec<(bool, al_testrun::runtime::CarriedRead)> = c
                    .reads
                    .iter()
                    .map(|r| (!own_apps.iter().any(|a| a.eq_ignore_ascii_case(&r.app_id)), r.clone()))
                    .collect();
                reads.sort_by_key(|(dep, _)| *dep);
                let (dependency, reads) = reads.into_iter().unzip();
                Entry {
                    dependency,
                    c: CarriedCodeunit { reads, ..c.clone() },
                }
            })
            .collect();
        // Grouped by the app of the SingleInstance codeunit, the team's own apps first (decision 6).
        entries.sort_by_key(|e| {
            (
                e.dependency.first().copied().unwrap_or(false),
                e.c.reads.first().map(|r| r.app.clone()).unwrap_or_default(),
                e.c.codeunit,
            )
        });
        SiReport {
            entries,
            checked: plan.variants.iter().any(|v| v.perturb.single_fresh),
        }
    }

    fn entry_of(&self, test: &str) -> Option<&Entry> {
        let (id, _) = split(test)?;
        self.entries.iter().find(|e| e.c.codeunit == id)
    }

    /// Whether flip `c` happened under `isolation.singleInstance = fresh`.
    fn is_si_flip(plan: &crate::stability::Plan, c: &Change) -> bool {
        c.is_flip() && plan.variants[c.variant].perturb.single_fresh
    }

    /// The flips under the dimension, per codeunit.
    fn breaks<'a>(&self, plan: &crate::stability::Plan, changes: &'a [Change]) -> Vec<(&Entry, Vec<&'a Change>)> {
        self.entries
            .iter()
            .filter_map(|e| {
                let mine: Vec<&Change> = changes
                    .iter()
                    .filter(|c| Self::is_si_flip(plan, c) && split(&c.test).is_some_and(|(id, _)| id == e.c.codeunit))
                    .collect();
                (!mine.is_empty()).then_some((e, mine))
            })
            .collect()
    }

    /// `flips[].reason` of a flip under the dimension (decision 6).
    pub fn reason(&self, plan: &crate::stability::Plan, c: &Change) -> Option<serde_json::Value> {
        if !Self::is_si_flip(plan, c) {
            return None;
        }
        let e = self.entry_of(&c.test)?;
        let kind = if c.base == Kind::Pass {
            "passesOnlyAfterEarlierCodeunits"
        } else {
            "failsOnlyAfterEarlierCodeunits"
        };
        Some(serde_json::json!({ "kind": kind, "isolation": e.c.isolation, "reads": reads_json(e) }))
    }

    /// The one-line reason a JUnit failure message adds.
    pub fn reason_line(&self, plan: &crate::stability::Plan, c: &Change) -> Option<String> {
        if !Self::is_si_flip(plan, c) {
            return None;
        }
        let e = self.entry_of(&c.test)?;
        Some(reads_text(e))
    }

    /// The JUnit property `singleInstanceCarried` of a test of the base variant: the reads of its codeunit when it is in
    /// `readsCarried`.
    pub fn carried_property(&self, plan: &crate::stability::Plan, changes: &[Change], codeunit: u32) -> Option<String> {
        let broken = self.breaks(plan, changes);
        let e = self.entries.iter().find(|e| e.c.codeunit == codeunit)?;
        (!broken.iter().any(|(b, _)| b.c.codeunit == codeunit)).then(|| reads_text(e))
    }

    /// The `readsCarried` list (decision 5): every entry without a flip under the dimension.
    pub fn reads_carried(
        &self,
        plan: &crate::stability::Plan,
        changes: &[Change],
        base: &[Seen],
    ) -> Vec<serde_json::Value> {
        let broken = self.breaks(plan, changes);
        self.entries
            .iter()
            .filter(|e| !broken.iter().any(|(b, _)| b.c.codeunit == e.c.codeunit))
            .map(|e| {
                let (outcome, note) = self.unchanged(e, base);
                serde_json::json!({
                    "codeunit": { "id": e.c.codeunit, "name": e.c.name },
                    "isolation": e.c.isolation,
                    "inChain": e.c.in_chain,
                    "outcome": outcome,
                    "checked": self.checked && e.c.fresh.is_some(),
                    "note": note,
                    "reads": reads_json(e),
                })
            })
            .collect()
    }

    /// The base outcome of entry `e` ("pass", "fail" or "not run") and what its fresh run showed besides: "fails either
    /// way, with another message" or "not compared (time limit)" (amendment 1, edge cases).
    fn unchanged(&self, e: &Entry, base: &[Seen]) -> (&'static str, Option<String>) {
        let prefix = format!("Codeunit{}.", e.c.codeunit);
        let mine: Vec<&Seen> = base.iter().filter(|s| s.test.starts_with(&prefix)).collect();
        let outcome = if mine.iter().any(|s| s.kind == Kind::Fail) {
            "fail"
        } else if mine.iter().all(|s| s.kind == Kind::Pass) {
            "pass"
        } else {
            "not run"
        };
        let mut note = None;
        for (test, o) in e.c.fresh.iter().flatten() {
            let Some(s) = mine.iter().find(|s| s.test[prefix.len()..].eq_ignore_ascii_case(test)) else {
                continue;
            };
            let fresh_msg = match o {
                al_testrun::Outcome::Fail(m) | al_testrun::Outcome::NotRun(m) => m.as_str(),
                al_testrun::Outcome::Pass => "",
            };
            if s.message.starts_with(TIME_LIMIT) || fresh_msg.starts_with(TIME_LIMIT) {
                return (outcome, Some("not compared (time limit)".into()));
            }
            if s.kind == Kind::Fail && matches!(o, al_testrun::Outcome::Fail(_)) && s.message != fresh_msg {
                note = Some(format!(
                    "fails either way, with another message: {} (carried) against {} (fresh)",
                    first_line(&s.message),
                    first_line(fresh_msg)
                ));
            }
        }
        (outcome, note)
    }

    /// The console section "Order dependence (SingleInstance state)" (decision 6), empty when no codeunit read carried
    /// state.
    pub fn lines(&self, plan: &crate::stability::Plan, changes: &[Change], base: &[Seen]) -> Vec<String> {
        if self.entries.is_empty() {
            return Vec::new();
        }
        let mut lines = vec![String::new(), "Order dependence (SingleInstance state):".to_string()];
        let broken = self.breaks(plan, changes);
        if !broken.is_empty() {
            lines.push("  Breaks with fresh SingleInstance state:".into());
            for (e, flips) in &broken {
                lines.push(format!(
                    "    test codeunit {} {} (TestIsolation {})",
                    e.c.codeunit, e.c.name, e.c.isolation
                ));
                for (r, dep) in e.c.reads.iter().zip(&e.dependency) {
                    lines.push(format!("      {}", read_line(r, *dep)));
                }
                for c in flips {
                    let t = split(&c.test).map_or(c.test.as_str(), |(_, t)| t);
                    let label = if c.base == Kind::Pass {
                        "passes only after earlier codeunits"
                    } else {
                        "fails only after earlier codeunits"
                    };
                    let base_msg = base
                        .iter()
                        .find(|s| s.test == c.test)
                        .map_or("", |s| s.message.as_str());
                    let (carried, fresh) = if c.base == Kind::Pass {
                        ("pass".to_string(), format!("fail: {}", first_line(&c.message)))
                    } else {
                        (format!("fail: {}", first_line(base_msg)), "pass".to_string())
                    };
                    lines.push(format!("      {t}: {label}; carried {carried}; fresh {fresh}"));
                }
            }
        }
        let rest = self.reads_carried(plan, changes, base);
        if !rest.is_empty() {
            let how = if self.checked {
                "checked"
            } else {
                "not checked: no fresh variant"
            };
            lines.push(format!("  Reads carried state, outcome unchanged ({how}):"));
            for v in rest.iter().take(CONSOLE_LIMIT) {
                let e = self
                    .entries
                    .iter()
                    .find(|e| v["codeunit"]["id"] == e.c.codeunit)
                    .expect("an entry");
                let note = v["note"].as_str().map(|n| format!("; {n}")).unwrap_or_default();
                lines.push(format!(
                    "    test codeunit {} {} ({}): {}{note}",
                    e.c.codeunit,
                    e.c.name,
                    v["outcome"].as_str().unwrap_or(""),
                    reads_text(e)
                ));
            }
            if rest.len() > CONSOLE_LIMIT {
                lines.push(format!(
                    "    and {} more (--output-json lists all)",
                    rest.len() - CONSOLE_LIMIT
                ));
            }
        }
        lines
    }

    /// The tests of the entries, for `orderDependent` (kept for one release, filled from the new list).
    pub fn tests(&self, base: &[Seen]) -> Vec<String> {
        base.iter()
            .filter(|s| split(&s.test).is_some_and(|(id, _)| self.entries.iter().any(|e| e.c.codeunit == id)))
            .map(|s| s.test.clone())
            .collect()
    }
}

fn reads_text(e: &Entry) -> String {
    e.c.reads
        .iter()
        .zip(&e.dependency)
        .map(|(r, d)| read_line(r, *d))
        .collect::<Vec<_>>()
        .join("; ")
}

fn reads_json(e: &Entry) -> Vec<serde_json::Value> {
    e.c.reads
        .iter()
        .zip(&e.dependency)
        .map(|(r, dep)| {
            serde_json::json!({
                "singleInstance": { "id": r.single_id, "name": r.single_name, "app": r.app,
                                    "publisher": r.publisher, "dependencyApp": dep },
                "global": r.global,
                "lastWriter": r.last_writer.as_ref().map(|(id, n)| serde_json::json!({ "id": id, "name": n })),
            })
        })
        .collect()
}

fn first_line(s: &str) -> &str {
    s.lines().next().unwrap_or("")
}
