//! The work ratchet: deterministic counts of what a corpus run did, against `metrics/<major>/work.json`.
//!
//! Wall time on a shared CI runner varies by half from one run to the next, and GitHub's runners give no instruction
//! counter, so a change that doubles what a run compiles can pass every time budget. The counts here (routines lowered,
//! objects loaded, dependency apps from source or from the compile cache, install baseline rows, tests run) are the same
//! on every run of the same inputs. The corpus run compares them per pass and in total with the file: a count more than
//! [`TOLERANCE_PERCENT`] above the file fails the run, one that far below prints a note asking to lower the file.
//! `--update-work` writes the file from a full run.

use std::collections::BTreeMap;

/// How far a count may rise above the file before the run fails, and fall below it before a note asks to lower it.
pub const TOLERANCE_PERCENT: u64 = 10;

/// The name of the counts of every pass of a version added up.
pub const TOTAL: &str = "total";

/// The counters of one pass, by name.
pub type Counts = BTreeMap<String, u64>;

/// The expected counts of one BC version: `metrics/<major>/work.json`.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WorkFile {
    /// The Microsoft build the counts were made with (the dependency apps' source). Information only: another build of
    /// the same minor changes the counts by far less than the tolerance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bc_build: Option<String>,
    /// By pass (`main`, `extra apps: ...`, and [`TOTAL`]).
    pub passes: BTreeMap<String, Counts>,
}

impl WorkFile {
    pub fn parse(text: &str) -> Result<WorkFile, String> {
        serde_json::from_str(text).map_err(|e| e.to_string())
    }

    /// The file's text: sorted keys, one counter per line, a final newline.
    pub fn to_text(&self) -> String {
        let mut s = serde_json::to_string_pretty(self).expect("work file serializes");
        s.push('\n');
        s
    }

    /// The file for the counts of `passes` (in run order), with their [`TOTAL`].
    pub fn of(bc_build: Option<String>, passes: &[(String, Counts)]) -> WorkFile {
        let mut all: BTreeMap<String, Counts> = passes.iter().cloned().collect();
        let mut total = Counts::new();
        for (_, c) in passes {
            for (k, v) in c {
                *total.entry(k.clone()).or_default() += v;
            }
        }
        all.insert(TOTAL.into(), total);
        WorkFile { bc_build, passes: all }
    }
}

/// Why this run's counts cannot be compared with the file (the file holds those of CI's normal run: the full corpus with
/// the dependency apps, no compile cache, the fixed seed); `None` when they can. From the flags only: see
/// [`not_full_corpus_env`] for the environment.
pub fn not_full_corpus(a: &crate::args::Args) -> Option<String> {
    if a.select.is_active() {
        return Some(format!("a selection ({})", a.select.describe()));
    }
    let flags = [
        (a.no_deps, "--no-deps"),
        (a.no_ratchet, "--no-ratchet"),
        (a.compile_cache, "--compile-cache"),
        (a.explain.is_some(), "--explain"),
        (a.seed.is_some_and(|s| s != al_host::seed::FIXED_RUN_SEED), "--seed"),
        (a.baseline.is_some(), "--baseline"),
        (!a.deps.is_empty(), "--deps"),
        (
            a.fetch_deps || a.build_compile_cache,
            "--fetch-deps or --build-compile-cache",
        ),
    ];
    flags.iter().find(|(on, _)| *on).map(|(_, f)| f.to_string())
}

/// The environment variables that change what a run compiles or installs (CI's other runs set them).
pub const WORK_ENV: [&str; 5] = [
    "AL_EAGER_DEPENDENCIES",
    "AL_FORCE_BASELINE",
    "AL_NO_BASELINE",
    "AL_CHECK_BASELINE_RULE",
    "ALRUNNER_EAGER_DECODE",
];

/// [`not_full_corpus`] for the environment: the first variable of [`WORK_ENV`] that is set.
pub fn not_full_corpus_env(var: impl Fn(&str) -> bool) -> Option<String> {
    WORK_ENV.iter().find(|v| var(v)).map(|v| format!("{v} set"))
}

/// The name of a pass in the file: `main`, or the extra apps its environment installs.
pub fn pass_name(main: bool, extra_apps: &[String]) -> String {
    if main {
        "main".into()
    } else {
        format!("extra apps: {}", extra_apps.join(", "))
    }
}

/// The counts of a run of one pass.
pub fn counts_of(w: &al_testrun::runtime::RunWork) -> Counts {
    let l = &w.lower;
    [
        ("compiles", w.compiles),
        ("objects_loaded", l.objects),
        ("objects_lowered", l.objects_lowered),
        ("routines_lowered", l.routines_lowered),
        ("dependency_objects_lowered", l.dependency_objects_lowered),
        ("dependency_routines_lowered", l.dependency_routines_lowered),
        ("objects_from_cache", l.objects_from_cache),
        ("dependency_apps", w.dependency_apps),
        (
            "dependency_apps_from_source",
            w.dependency_apps - w.dependency_apps_from_cache,
        ),
        ("dependency_apps_from_cache", w.dependency_apps_from_cache),
        ("cache_layers_hit", w.cache_layers_hit),
        ("cache_layers_built", w.cache_layers_built),
        ("baseline_rows", w.baseline_rows.unwrap_or(0)),
        ("tests_run", w.tests_run),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v as u64))
    .collect()
}

/// One difference between the file and a run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Finding {
    /// A count rose more than the tolerance above the file (a pass the file does not have counts from 0): fails.
    Above {
        pass: String,
        counter: String,
        old: u64,
        new: u64,
    },
    /// A count fell more than the tolerance below the file: a note.
    Below {
        pass: String,
        counter: String,
        old: u64,
        new: u64,
    },
    /// A pass of the file did not run: a note.
    Gone { pass: String },
}

impl Finding {
    pub fn fails(&self) -> bool {
        matches!(self, Finding::Above { .. })
    }

    pub fn line(&self, version: &str, file: &str) -> String {
        let change = |old: u64, new: u64| match old {
            0 => "new".to_string(),
            _ => format!("{:+.0}%", (new as f64 - old as f64) * 100.0 / old as f64),
        };
        match self {
            Finding::Above {
                pass,
                counter,
                old,
                new,
            } => format!(
                "work ratchet: BC {version}, pass `{pass}`: {counter} {old} -> {new} ({}), more than {TOLERANCE_PERCENT}% above {file}",
                change(*old, *new)
            ),
            Finding::Below {
                pass,
                counter,
                old,
                new,
            } => format!(
                "work ratchet: BC {version}, pass `{pass}`: {counter} {old} -> {new} ({}), more than {TOLERANCE_PERCENT}% below {file}; lower the file (--update-work)",
                change(*old, *new)
            ),
            Finding::Gone { pass } => {
                format!(
                    "work ratchet: BC {version}, pass `{pass}` of {file} did not run; lower the file (--update-work)"
                )
            }
        }
    }
}

/// The differences between `expected` (the file) and `actual` (the run) beyond the tolerance, in file order.
pub fn compare(expected: &WorkFile, actual: &WorkFile) -> Vec<Finding> {
    let empty = Counts::new();
    let mut out = Vec::new();
    for (pass, counts) in &actual.passes {
        let old_counts = expected.passes.get(pass).unwrap_or(&empty);
        for (counter, &new) in counts {
            let old = old_counts.get(counter).copied().unwrap_or(0);
            let (pass, counter) = (pass.clone(), counter.clone());
            if new * 100 > old * (100 + TOLERANCE_PERCENT) {
                out.push(Finding::Above {
                    pass,
                    counter,
                    old,
                    new,
                });
            } else if new * 100 < old * (100 - TOLERANCE_PERCENT) {
                out.push(Finding::Below {
                    pass,
                    counter,
                    old,
                    new,
                });
            }
        }
    }
    for pass in expected.passes.keys() {
        if !actual.passes.contains_key(pass) {
            out.push(Finding::Gone { pass: pass.clone() });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(passes: &[(&str, &[(&str, u64)])]) -> WorkFile {
        WorkFile {
            bc_build: None,
            passes: passes
                .iter()
                .map(|(p, c)| (p.to_string(), c.iter().map(|(k, v)| (k.to_string(), *v)).collect()))
                .collect(),
        }
    }

    #[test]
    fn counts_within_ten_percent_either_way_pass_quietly() {
        let old = file(&[("main", &[("routines_lowered", 1000), ("tests_run", 100)])]);
        let new = file(&[("main", &[("routines_lowered", 1100), ("tests_run", 90)])]);
        assert_eq!(compare(&old, &new), vec![]);
    }

    #[test]
    fn a_count_more_than_ten_percent_above_the_file_fails() {
        let old = file(&[("main", &[("routines_lowered", 1000)])]);
        let new = file(&[("main", &[("routines_lowered", 1101)])]);
        let f = compare(&old, &new);
        assert_eq!(
            f,
            vec![Finding::Above {
                pass: "main".into(),
                counter: "routines_lowered".into(),
                old: 1000,
                new: 1101
            }]
        );
        assert!(f[0].fails());
        assert_eq!(
            f[0].line("29", "metrics/29/work.json"),
            "work ratchet: BC 29, pass `main`: routines_lowered 1000 -> 1101 (+10%), more than 10% above metrics/29/work.json"
        );
    }

    #[test]
    fn a_count_more_than_ten_percent_below_the_file_is_a_note() {
        let old = file(&[("main", &[("routines_lowered", 1000)])]);
        let new = file(&[("main", &[("routines_lowered", 899)])]);
        let f = compare(&old, &new);
        assert_eq!(
            f,
            vec![Finding::Below {
                pass: "main".into(),
                counter: "routines_lowered".into(),
                old: 1000,
                new: 899
            }]
        );
        assert!(!f[0].fails());
        assert!(f[0].line("29", "w.json").ends_with("lower the file (--update-work)"));
    }

    #[test]
    fn a_pass_or_counter_the_file_does_not_have_counts_from_zero_and_fails() {
        let old = file(&[("main", &[("tests_run", 10)])]);
        let new = file(&[
            ("main", &[("tests_run", 10), ("compiles", 1)]),
            ("extra apps: X", &[("routines_lowered", 5), ("tests_run", 0)]),
        ]);
        let f = compare(&old, &new);
        assert_eq!(f.len(), 2, "{f:?}");
        assert!(f.iter().all(Finding::fails));
        assert!(
            f[0].line("27", "w.json")
                .contains("pass `extra apps: X`: routines_lowered 0 -> 5 (new)")
        );
        assert!(f[1].line("27", "w.json").contains("pass `main`: compiles 0 -> 1 (new)"));
    }

    #[test]
    fn a_pass_of_the_file_that_did_not_run_is_a_note() {
        let old = file(&[("main", &[("tests_run", 10)]), ("extra apps: X", &[("tests_run", 3)])]);
        let new = file(&[("main", &[("tests_run", 10)])]);
        assert_eq!(
            compare(&old, &new),
            vec![Finding::Gone {
                pass: "extra apps: X".into()
            }]
        );
    }

    #[test]
    fn the_file_adds_a_total_over_the_passes_and_reads_back_what_it_writes() {
        let a: Counts = [("tests_run".to_string(), 10)].into();
        let b: Counts = [("tests_run".to_string(), 4), ("compiles".to_string(), 1)].into();
        let w = WorkFile::of(
            Some("29.1.1.0".into()),
            &[
                (pass_name(true, &[]), a),
                (pass_name(false, &["T".into(), "U".into()]), b),
            ],
        );
        assert_eq!(w.passes[TOTAL]["tests_run"], 14);
        assert_eq!(w.passes[TOTAL]["compiles"], 1);
        assert!(w.passes.contains_key("extra apps: T, U"));
        let text = w.to_text();
        assert!(text.ends_with("}\n"));
        assert_eq!(WorkFile::parse(&text).unwrap(), w);
    }
}
