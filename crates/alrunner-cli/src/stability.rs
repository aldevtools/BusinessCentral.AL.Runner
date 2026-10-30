//! `al-runner test --stability FILE|microsoft` (ADR-058): the variants of the config, the report of the tests whose
//! outcome changes between them (console, `--output-json`, `--junit`), and the exit code. The config, the registry of
//! dimensions and the variants are al-testrun's (`al_testrun::stability`); the runs share one compile
//! (`CorpusRuntime::try_run_variants`).

use crate::args::TestArgs;
use al_testrun::runtime::TestResult;
use al_testrun::stability::{self, Change, Kind, Seen, Variant};
use sha2::Digest;

/// What a stability run runs: the variants of its one cell, and what makes the run repeatable.
pub struct Plan {
    /// `<bc major>.<minor>-<country>`: the cell of the variant ids.
    pub cell: String,
    pub variants: Vec<Variant>,
    pub master_seed: u64,
    /// `sha256:<hex>` of the config text.
    pub config_hash: String,
    /// The config as named on the command line.
    pub source: String,
}

impl Plan {
    pub fn perturbs(&self) -> Vec<stability::Perturb> {
        self.variants.iter().map(|v| v.perturb).collect()
    }
}

/// Refusals that need the run's environment (exit 2, before anything compiles).
fn check_environment() -> Result<(), String> {
    if al_testrun::host::real_clock() {
        return Err(
            "--stability cannot be used with ALRUNNER_REAL_CLOCK=1: the wall clock breaks the replay of a \
                    variant (ADR-058 section 4)"
                .into(),
        );
    }
    Ok(())
}

/// The master seed: the config's, else one from the wall clock when the config draws values (printed, so the run can
/// be repeated), else 0.
fn master_seed(cfg: &stability::Config) -> (u64, bool) {
    match cfg.master_seed {
        Some(s) => (s, false),
        None if cfg.draws() => {
            let wall = al_testrun::host::SystemClock::wall() as u64;
            (wall % 1_000_000_000, true)
        }
        None => (0, false),
    }
}

/// The plan of `--stability` without `--stability-replay`; `None` for a normal run or a replay.
pub fn plan(a: &TestArgs, cell: &str) -> Result<Option<Plan>, String> {
    let Some(st) = &a.stability else { return Ok(None) };
    if a.stability_replay.is_some() {
        return Ok(None);
    }
    check_environment()?;
    let (seed, picked) = master_seed(&st.config);
    let max = a.stability_max.unwrap_or(stability::DEFAULT_MAX_VARIANTS);
    let variants = stability::variants(&st.config, seed, max).map_err(|e| format!("--stability {}: {e}", st.source))?;
    if picked {
        eprintln!(
            "stability: master seed {seed} (picked from the clock; put \"masterSeed\": {seed} in the config to repeat this run)"
        );
    }
    Ok(Some(Plan {
        cell: cell.to_string(),
        variants,
        master_seed: seed,
        config_hash: format!("sha256:{:x}", sha2::Sha256::digest(st.text.as_bytes())),
        source: st.source.clone(),
    }))
}

/// The variant `--stability-replay` names, if any: an id of the config's variants, or printed values.
pub fn replay(a: &TestArgs, cell: &str) -> Result<Option<Variant>, String> {
    let Some(arg) = &a.stability_replay else {
        return Ok(None);
    };
    check_environment()?;
    let variants = match &a.stability {
        Some(st) => {
            let (seed, _) = master_seed(&st.config);
            if st.config.master_seed.is_none() && st.config.draws() {
                return Err(format!(
                    "--stability-replay {arg}: the config draws values but has no \"masterSeed\"; put the master seed the run printed in the config, or replay with the printed values"
                ));
            }
            let max = a.stability_max.unwrap_or(stability::DEFAULT_MAX_VARIANTS);
            Some(stability::variants(&st.config, seed, max).map_err(|e| format!("--stability {}: {e}", st.source))?)
        }
        None => None,
    };
    let v = stability::replay(arg, variants.as_deref()).map_err(|e| format!("--stability-replay {arg}: {e}"))?;
    let id = if variants.is_some() {
        v.id(cell)
    } else {
        "replay".into()
    };
    eprintln!("stability: replaying {id}  {}", v.describe());
    Ok(Some(v))
}

/// `Codeunit<id>.<test>`: how the report names a test.
fn name(r: &TestResult) -> String {
    format!("Codeunit{}.{}", r.codeunit, r.test)
}

/// What the report compares of each selected result of one variant.
pub fn seen(results: &[&TestResult]) -> Vec<Seen> {
    results
        .iter()
        .map(|r| Seen {
            test: name(r),
            kind: Kind::of(&r.outcome),
            message: match &r.outcome {
                al_testrun::Outcome::Pass => String::new(),
                al_testrun::Outcome::Fail(m) | al_testrun::Outcome::NotRun(m) => m.clone(),
            },
            assumption: !r.assumptions.is_empty(),
        })
        .collect()
}

/// The report of a stability run: printed lines, the changes, and whether a flip makes the run fail.
pub struct Outcome {
    pub lines: Vec<String>,
    pub changes: Vec<Change>,
    pub flips: usize,
}

/// Compares every variant with the base variant and renders the console report (ADR-058 section 5).
pub fn compare(plan: &Plan, per_variant: &[Vec<Seen>]) -> Outcome {
    let changes = stability::changes(per_variant);
    let flips = changes.iter().filter(|c| c.is_flip()).count();
    let mut lines = vec![
        String::new(),
        format!("Stability ({}, {} variants):", plan.source, plan.variants.len()),
    ];
    for v in &plan.variants {
        lines.push(format!("  {}  {}", v.id(&plan.cell), v.describe()));
    }
    let tests = per_variant.first().map_or(0, Vec::len);
    let changed: std::collections::BTreeSet<&str> = changes.iter().map(|c| c.test.as_str()).collect();
    let flipped: std::collections::BTreeSet<&str> = changes
        .iter()
        .filter(|c| c.is_flip())
        .map(|c| c.test.as_str())
        .collect();
    let mut by_test: std::collections::BTreeMap<&str, Vec<&Change>> = Default::default();
    for c in &changes {
        by_test.entry(c.test.as_str()).or_default().push(c);
    }
    if !flipped.is_empty() {
        lines.push(String::new());
        lines.push("Tests whose outcome changed:".into());
        for (t, cs) in by_test.iter().filter(|(_, cs)| cs.iter().any(|c| c.is_flip())) {
            lines.push(format!("  {t}  (base: {})", cs[0].base.text()));
            for c in cs.iter().filter(|c| c.is_flip()) {
                let v = &plan.variants[c.variant];
                let mut line = format!(
                    "    {} -> {} under {}  {}",
                    c.base.text(),
                    c.outcome.text(),
                    v.id(&plan.cell),
                    v.describe()
                );
                if c.assumption {
                    line.push_str("  (relies on an unprobed assumption)");
                }
                if !c.message.is_empty() {
                    line.push_str(&format!("\n      {}", first_line(&c.message)));
                }
                lines.push(line);
            }
        }
    }
    let not_run: Vec<&Change> = changes.iter().filter(|c| !c.is_flip()).collect();
    if !not_run.is_empty() {
        lines.push(String::new());
        lines.push("Not run in some variant (not a finding by itself):".into());
        for c in not_run {
            let v = &plan.variants[c.variant];
            lines.push(format!(
                "  {}  {} -> {} under {}  {}\n      {}",
                c.test,
                c.base.text(),
                c.outcome.text(),
                v.id(&plan.cell),
                v.describe(),
                first_line(&c.message)
            ));
        }
    }
    // The variants with the most flips.
    let mut per: Vec<(usize, usize)> = (1..plan.variants.len())
        .map(|i| (i, changes.iter().filter(|c| c.variant == i && c.is_flip()).count()))
        .filter(|(_, n)| *n > 0)
        .collect();
    per.sort_by_key(|&(i, n)| (std::cmp::Reverse(n), i));
    if !per.is_empty() {
        lines.push(String::new());
        lines.push("Variants with the most flips:".into());
        for (i, n) in per.iter().take(5) {
            let v = &plan.variants[*i];
            lines.push(format!("  {n:>4}  {}  {}", v.id(&plan.cell), v.describe()));
        }
    }
    lines.push(String::new());
    lines.push(format!(
        "Stability: {tests} tests, {} stable, {} flipped, {} changed to or from not run; master seed {}, config {}",
        tests - changed.len(),
        flipped.len(),
        changed.len() - flipped.len(),
        plan.master_seed,
        plan.config_hash
    ));
    Outcome { lines, changes, flips }
}

fn first_line(s: &str) -> &str {
    s.lines().next().unwrap_or("")
}

/// The exit code of a stability run: 1 when a test flipped (unless `--stability-allow-flips`), else the base
/// variant's.
pub fn exit_code(base: u8, flips: usize, allow: bool) -> u8 {
    if flips > 0 && !allow { 1 } else { base }
}

/// The `stability` object of `--output-json` (ADR-058 section 5).
pub fn json(plan: &Plan, out: &Outcome, per_variant: &[Vec<Seen>]) -> serde_json::Value {
    let variants: Vec<serde_json::Value> = plan
        .variants
        .iter()
        .map(|v| {
            let values: serde_json::Map<String, serde_json::Value> = v
                .values
                .iter()
                .map(|(k, val)| (k.clone(), val.clone().into()))
                .collect();
            serde_json::json!({ "id": v.id(&plan.cell), "cell": plan.cell, "values": values })
        })
        .collect();
    let flips: Vec<serde_json::Value> = out
        .changes
        .iter()
        .map(|c| {
            let v = &plan.variants[c.variant];
            // oneAtATime variants are minimal already; a variant of several dimensions is not reduced in this version.
            let minimal = if v.values.len() == 1 {
                let (k, val) = &v.values[0];
                serde_json::json!({ k: val })
            } else {
                serde_json::Value::Null
            };
            serde_json::json!({
                "test": c.test,
                "base": c.base.text(),
                "variant": v.id(&plan.cell),
                "outcome": c.outcome.text(),
                "message": c.message,
                "minimal": minimal,
                "dataAxis": false,
                "finding": c.is_flip(),
                "reliesOnAssumption": c.assumption,
            })
        })
        .collect();
    let tests = per_variant.first().map_or(0, Vec::len);
    let changed: std::collections::BTreeSet<&str> = out.changes.iter().map(|c| c.test.as_str()).collect();
    let flipped: std::collections::BTreeSet<&str> = out
        .changes
        .iter()
        .filter(|c| c.is_flip())
        .map(|c| c.test.as_str())
        .collect();
    serde_json::json!({
        "format": 1,
        "config": plan.config_hash,
        "masterSeed": plan.master_seed,
        "runtimeBuild": crate::version::line(),
        "variants": variants,
        "flips": flips,
        "summary": {
            "variants": plan.variants.len(),
            "tests": tests,
            "stable": tests - changed.len(),
            "flipped": flipped.len(),
            "notRunChanges": changed.len() - flipped.len(),
        }
    })
}

fn xml(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            '\'' => o.push_str("&apos;"),
            c if (c as u32) < 0x20 && c != '\n' && c != '\t' && c != '\r' => {}
            c => o.push(c),
        }
    }
    o
}

/// `--junit` of a stability run: one `testsuite` per variant, named by its id, with its values as properties; a test
/// that changed outcome against the base variant gets a `failure` in that variant (ADR-058 section 5).
pub fn junit(plan: &Plan, results: &[Vec<&TestResult>], out: &Outcome) -> String {
    let mut x = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<testsuites>\n");
    for (vi, (v, rs)) in plan.variants.iter().zip(results).enumerate() {
        let failures = rs
            .iter()
            .filter(|r| {
                matches!(r.outcome, al_testrun::Outcome::Fail(_))
                    || out
                        .changes
                        .iter()
                        .any(|c| c.variant == vi && c.is_flip() && c.test == name(r))
            })
            .count();
        let skipped = rs
            .iter()
            .filter(|r| matches!(r.outcome, al_testrun::Outcome::NotRun(_)))
            .count();
        x.push_str(&format!(
            "  <testsuite name=\"{}\" tests=\"{}\" failures=\"{failures}\" skipped=\"{skipped}\">\n    <properties>\n",
            xml(&v.id(&plan.cell)),
            rs.len()
        ));
        for (k, val) in &v.values {
            x.push_str(&format!(
                "      <property name=\"{}\" value=\"{}\"/>\n",
                xml(k),
                xml(val)
            ));
        }
        x.push_str("    </properties>\n");
        for r in rs {
            x.push_str(&format!(
                "    <testcase classname=\"{}\" name=\"{}\" time=\"{:.3}\"",
                xml(&r.codeunit_name),
                xml(&r.test),
                r.time.as_secs_f64()
            ));
            let flip = out
                .changes
                .iter()
                .find(|c| c.variant == vi && c.is_flip() && c.test == name(r));
            match (&r.outcome, flip) {
                (o, Some(c)) => {
                    let original = match o {
                        al_testrun::Outcome::Fail(m) => format!(": {m}"),
                        _ => String::new(),
                    };
                    let msg = format!(
                        "outcome changed from {} under {}: {}{original}",
                        c.base.text(),
                        v.id(&plan.cell),
                        v.describe()
                    );
                    x.push_str(&format!(
                        ">\n      <failure message=\"{}\"/>\n    </testcase>\n",
                        xml(&msg)
                    ));
                }
                (al_testrun::Outcome::Fail(m), None) => {
                    x.push_str(&format!(
                        ">\n      <failure message=\"{}\"/>\n    </testcase>\n",
                        xml(m)
                    ));
                }
                (al_testrun::Outcome::NotRun(m), None) => {
                    x.push_str(&format!(
                        ">\n      <skipped message=\"{}\"/>\n    </testcase>\n",
                        xml(m)
                    ));
                }
                (al_testrun::Outcome::Pass, None) => x.push_str("/>\n"),
            }
        }
        x.push_str("  </testsuite>\n");
    }
    x.push_str("</testsuites>\n");
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan_of(text: &str) -> Plan {
        let cfg = stability::parse_config(text).unwrap();
        Plan {
            cell: "29.0-w1".into(),
            variants: stability::variants(&cfg, 7, 1000).unwrap(),
            master_seed: 7,
            config_hash: "sha256:x".into(),
            source: "test".into(),
        }
    }

    fn seen(t: &str, k: Kind) -> Seen {
        Seen {
            test: t.into(),
            kind: k,
            message: if k == Kind::Fail { "boom".into() } else { String::new() },
            assumption: false,
        }
    }

    #[test]
    fn a_flip_is_reported_with_its_variant_and_fails_the_run() {
        let plan = plan_of(r#"{"format":1,"dimensions":{"order.methods":["reverse"]}}"#);
        let runs = vec![
            vec![seen("Codeunit1.A", Kind::Pass), seen("Codeunit1.B", Kind::Pass)],
            vec![seen("Codeunit1.A", Kind::Pass), seen("Codeunit1.B", Kind::Fail)],
        ];
        let out = compare(&plan, &runs);
        assert_eq!(out.flips, 1);
        let text = out.lines.join("\n");
        assert!(text.contains("29.0-w1/1  order.methods=reverse"), "{text}");
        assert!(text.contains("pass -> fail under 29.0-w1/1"), "{text}");
        assert!(text.contains("2 tests, 1 stable, 1 flipped"), "{text}");
        assert_eq!(exit_code(0, out.flips, false), 1);
        assert_eq!(exit_code(0, out.flips, true), 0);
        let j = json(&plan, &out, &runs);
        assert_eq!(j["summary"]["flipped"], 1);
        assert_eq!(j["flips"][0]["minimal"]["order.methods"], "reverse");
        assert_eq!(j["variants"][1]["id"], "29.0-w1/1");
    }

    #[test]
    fn a_change_to_not_run_is_listed_apart_and_does_not_fail_the_run() {
        let plan = plan_of(r#"{"format":1,"dimensions":{"isolation.alone":[true]}}"#);
        let runs = vec![
            vec![seen("Codeunit1.A", Kind::Pass)],
            vec![seen("Codeunit1.A", Kind::NotRun)],
        ];
        let out = compare(&plan, &runs);
        assert_eq!(out.flips, 0);
        assert!(out.lines.join("\n").contains("Not run in some variant"));
        assert_eq!(exit_code(0, out.flips, false), 0);
    }
}
