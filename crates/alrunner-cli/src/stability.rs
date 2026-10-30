//! `al-runner test --stability FILE|microsoft` (ADR-058): the variants of the config, the report of the tests whose
//! outcome changes between them (console, `--output-json`, `--junit`), and the exit code. The config, the registry of
//! dimensions and the variants are al-testrun's (`al_testrun::stability`); the runs share one compile
//! (`CorpusRuntime::try_run_variants`).

use crate::args::TestArgs;
use al_testrun::runtime::TestResult;
use al_testrun::stability::{self, Change, Kind, Seen, Variant};
use sha2::Digest;

/// What a stability run runs in this process: the cells (one, or one per value of the `environment` axis, which share
/// the compile, ADR-058 section 3), the variants of each, and what makes the run repeatable.
pub struct Plan {
    /// The cell names: `<bc major>.<minor>-<country>`, then `-companies<n>` (the n-th value of the `companies` axis)
    /// and `-<environment>` when the config names those axes.
    pub cells: Vec<String>,
    /// The variants of every cell, cell after cell; `Variant::index` counts within the cell (0: its base variant).
    pub variants: Vec<Variant>,
    /// The cell of each variant (an index into `cells`).
    pub cell_of: Vec<usize>,
    pub master_seed: u64,
    /// The run seed (ADR-061): part of every variant id.
    pub run_seed: i32,
    /// `sha256:<hex>` of the config text.
    pub config_hash: String,
    /// The config as named on the command line.
    pub source: String,
}

/// A cell name read back (`29.0-w1-companies2-onprem`): the parts a replay sets.
#[derive(Debug, Clone, PartialEq)]
pub struct CellName {
    pub version: String,
    pub country: String,
    /// The n-th value (from 1) of the `companies` axis.
    pub companies: Option<usize>,
    pub environment: Option<al_host::Environment>,
}

/// Reads a cell name; `None` when it is not one.
pub fn parse_cell(name: &str) -> Option<CellName> {
    let mut parts = name.split('-');
    let version = parts.next()?.to_string();
    let country = parts.next()?.to_string();
    if version.is_empty() || country.is_empty() {
        return None;
    }
    let mut c = CellName {
        version,
        country,
        companies: None,
        environment: None,
    };
    for p in parts {
        if let Some(n) = p.strip_prefix("companies") {
            c.companies = Some(n.parse().ok().filter(|n| *n > 0)?);
        } else {
            c.environment = Some(p.parse().ok()?);
        }
    }
    Some(c)
}

impl Plan {
    /// The plan of `cfg` in the cell `base` (`<major>.<minor>-<country>`); `companies`: the n-th value (from 1) of the
    /// `companies` axis this process runs.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        cfg: &stability::Config,
        base: &str,
        companies: Option<usize>,
        master_seed: u64,
        run_seed: i32,
        max: usize,
        config_hash: String,
        source: String,
    ) -> Result<Plan, String> {
        let variants = stability::variants(cfg, master_seed, max).map_err(|e| format!("--stability {source}: {e}"))?;
        let mut name = base.to_string();
        if let Some(n) = companies {
            name.push_str(&format!("-companies{n}"));
        }
        let envs: Vec<Option<al_host::Environment>> = if cfg.matrix.environments.is_empty() {
            vec![None]
        } else {
            cfg.matrix.environments.iter().copied().map(Some).collect()
        };
        let mut plan = Plan {
            cells: Vec::new(),
            variants: Vec::new(),
            cell_of: Vec::new(),
            master_seed,
            run_seed,
            config_hash,
            source,
        };
        for env in envs {
            plan.cells.push(match env {
                Some(e) => format!("{name}-{}", stability::environment_text(e)),
                None => name.clone(),
            });
            for v in &variants {
                let mut v = v.clone();
                v.perturb.environment = env;
                plan.cell_of.push(plan.cells.len() - 1);
                plan.variants.push(v);
            }
        }
        Ok(plan)
    }

    /// `<cell>/<n>@<run seed>` of the i-th variant (ADR-061 section 5): `--stability-replay` with it sets the run seed
    /// too.
    pub fn id_at(&self, i: usize) -> String {
        format!(
            "{}@{}",
            self.variants[i].id(&self.cells[self.cell_of[i]]),
            self.run_seed
        )
    }

    pub fn perturbs(&self) -> Vec<stability::Perturb> {
        self.variants.iter().map(|v| v.perturb.clone()).collect()
    }

    /// The variants of each cell: (cell index, range of variant indexes).
    fn ranges(&self) -> Vec<(usize, std::ops::Range<usize>)> {
        let mut out: Vec<(usize, std::ops::Range<usize>)> = Vec::new();
        for (i, &c) in self.cell_of.iter().enumerate() {
            match out.last_mut() {
                Some((last, r)) if *last == c => r.end = i + 1,
                _ => out.push((c, i..i + 1)),
            }
        }
        out
    }

    /// The variant an id names (`<cell>/<n>[@seed]` or `<n>`). With one cell a cell part that names another cell of
    /// the same version and country is accepted as before; with several cells it must name one of them.
    pub fn replay_in(&self, arg: &str) -> Result<Variant, String> {
        let id = stability::split_replay_seed(arg).0;
        let (cell, number) = match id.rsplit_once('/') {
            Some((c, n)) => (Some(c), n),
            None => (None, id),
        };
        let n: usize = number
            .parse()
            .map_err(|_| format!("\"{id}\" is not a variant id (<cell>/<n>)"))?;
        let ci = match cell {
            Some(c) if self.cells.len() > 1 => self.cells.iter().position(|x| x == c).ok_or_else(|| {
                format!(
                    "the cell {c} is not a cell of this run; its cells are {}",
                    self.cells.join(", ")
                )
            })?,
            _ => 0,
        };
        let r = self
            .ranges()
            .into_iter()
            .find(|(c, _)| *c == ci)
            .map(|(_, r)| r)
            .unwrap_or(0..0);
        self.variants[r.clone()]
            .get(n)
            .cloned()
            .ok_or_else(|| format!("the config has {} variants per cell; there is no variant {n}", r.len()))
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

/// The master seed: the config's, else the run seed (ADR-061 section 5: a stability run prints one seed, not two).
fn master_seed(cfg: &stability::Config, run_seed: i32) -> u64 {
    cfg.master_seed.unwrap_or(u64::from(run_seed as u32))
}

/// The n-th value (from 1) of the config's `companies` axis that `--companies` of this run names (the parent of a
/// matrix passes it to each cell); `None` when the config has no such axis.
fn companies_index(a: &TestArgs, cfg: &stability::Config) -> Result<Option<usize>, String> {
    if cfg.matrix.companies.is_empty() {
        return Ok(None);
    }
    for (i, v) in cfg.matrix.companies.iter().enumerate() {
        let parsed = v.as_deref().map(al_host::Companies::parse).transpose()?;
        if parsed == a.companies {
            return Ok(Some(i + 1));
        }
    }
    Err("--companies of this run is no value of the config's companies axis".into())
}

/// The plan of the config `a` names, in the cell `base` (`<major>.<minor>-<country>`).
fn plan_of(a: &TestArgs, st: &crate::args::StabilityArg, base: &str) -> Result<Plan, String> {
    let run_seed = crate::seed::of(a).seed;
    Plan::new(
        &st.config,
        base,
        companies_index(a, &st.config)?,
        master_seed(&st.config, run_seed),
        run_seed,
        a.stability_max.unwrap_or(stability::DEFAULT_MAX_VARIANTS),
        format!("sha256:{:x}", sha2::Sha256::digest(st.text.as_bytes())),
        st.source.clone(),
    )
}

/// The plan of `--stability` without `--stability-replay`; `None` for a normal run or a replay.
pub fn plan(a: &TestArgs, cell: &str) -> Result<Option<Plan>, String> {
    let Some(st) = &a.stability else { return Ok(None) };
    if a.stability_replay.is_some() {
        return Ok(None);
    }
    check_environment()?;
    let plan = plan_of(a, st, cell)?;
    // `order.codeunits` changes outcomes only where codeunits share the database (ADR-058 section 2).
    if st.config.dimensions.iter().any(|(n, _)| n == "order.codeunits")
        && a.isolation != Some(al_testrun::isolation::TestIsolation::Disabled)
    {
        eprintln!(
            "stability: order.codeunits cannot change an outcome in this run: each test codeunit starts from the \
             database after install; use --isolation disabled to run the codeunits on one database"
        );
    }
    Ok(Some(plan))
}

/// The variant `--stability-replay` names, if any: an id of the config's variants, or printed values.
pub fn replay(a: &TestArgs, cell: &str) -> Result<Option<Variant>, String> {
    let Some(arg) = &a.stability_replay else {
        return Ok(None);
    };
    check_environment()?;
    let (v, id) = match &a.stability {
        Some(st) => {
            let run_seed = crate::seed::of(a);
            if st.config.master_seed.is_none()
                && st.config.draws()
                && run_seed.source == al_testrun::seed::SeedSource::Fresh
            {
                return Err(format!(
                    "--stability-replay {arg}: the config draws values from the run seed; replay with the variant id the run printed (`<cell>/<n>@<seed>`), --seed, or the printed values"
                ));
            }
            let plan = plan_of(a, st, cell)?;
            let id = stability::split_replay_seed(arg).0;
            let is_id = id.rsplit('/').next().is_some_and(|n| n.parse::<usize>().is_ok());
            if is_id {
                let v = plan
                    .replay_in(arg)
                    .map_err(|e| format!("--stability-replay {arg}: {e}"))?;
                let i = plan.variants.iter().position(|x| *x == v).unwrap_or(0);
                (v, plan.id_at(i))
            } else {
                let v = stability::replay(arg, None).map_err(|e| format!("--stability-replay {arg}: {e}"))?;
                (v, "replay".into())
            }
        }
        None => (
            stability::replay(arg, None).map_err(|e| format!("--stability-replay {arg}: {e}"))?,
            "replay".into(),
        ),
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

/// The changes of every cell: each variant compared with its own cell's base variant (ADR-058 section 3: a cell
/// runs the whole perturbation set). `Change::variant` is an index into `plan.variants`.
fn changes_of(plan: &Plan, per_variant: &[Vec<Seen>]) -> Vec<Change> {
    let mut out = Vec::new();
    for (_, r) in plan.ranges() {
        let slice = &per_variant[r.start.min(per_variant.len())..r.end.min(per_variant.len())];
        for mut c in stability::changes(slice) {
            c.variant += r.start;
            out.push(c);
        }
    }
    out
}

/// Tests, stable, flipped, changed to or from not run: of the variants in `range`.
fn counts(
    changes: &[Change],
    per_variant: &[Vec<Seen>],
    range: std::ops::Range<usize>,
) -> (usize, usize, usize, usize) {
    let tests = per_variant.get(range.start).map_or(0, Vec::len);
    let mine = || changes.iter().filter(|c| range.contains(&c.variant));
    let changed: std::collections::BTreeSet<&str> = mine().map(|c| c.test.as_str()).collect();
    let flipped: std::collections::BTreeSet<&str> = mine().filter(|c| c.is_flip()).map(|c| c.test.as_str()).collect();
    (
        tests,
        tests.saturating_sub(changed.len()),
        flipped.len(),
        changed.len() - flipped.len(),
    )
}

/// Compares every variant with its cell's base variant and renders the console report (ADR-058 section 5).
pub fn compare(plan: &Plan, per_variant: &[Vec<Seen>]) -> Outcome {
    let changes = changes_of(plan, per_variant);
    let flips = changes.iter().filter(|c| c.is_flip()).count();
    let per_cell = plan.variants.len() / plan.cells.len().max(1);
    let mut lines = vec![
        String::new(),
        if plan.cells.len() > 1 {
            format!(
                "Stability ({}, {} cells of {per_cell} variants):",
                plan.source,
                plan.cells.len()
            )
        } else {
            format!("Stability ({}, {} variants):", plan.source, plan.variants.len())
        },
    ];
    for i in 0..plan.variants.len() {
        lines.push(format!("  {}  {}", plan.id_at(i), plan.variants[i].describe()));
    }
    let mut by_test: std::collections::BTreeMap<&str, Vec<&Change>> = Default::default();
    for c in &changes {
        by_test.entry(c.test.as_str()).or_default().push(c);
    }
    if flips > 0 {
        lines.push(String::new());
        lines.push("Tests whose outcome changed:".into());
        for (t, cs) in by_test.iter().filter(|(_, cs)| cs.iter().any(|c| c.is_flip())) {
            // The base outcome of each cell the test flipped in.
            let mut cells_done: Vec<usize> = Vec::new();
            for c in cs.iter().filter(|c| c.is_flip()) {
                let cell = plan.cell_of[c.variant];
                if !cells_done.contains(&cell) {
                    cells_done.push(cell);
                    if plan.cells.len() > 1 {
                        lines.push(format!("  {t}  (base in {}: {})", plan.cells[cell], c.base.text()));
                    } else {
                        lines.push(format!("  {t}  (base: {})", c.base.text()));
                    }
                }
                let v = &plan.variants[c.variant];
                let mut line = format!(
                    "    {} -> {} under {}  {}",
                    c.base.text(),
                    c.outcome.text(),
                    plan.id_at(c.variant),
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
                plan.id_at(c.variant),
                v.describe(),
                first_line(&c.message)
            ));
        }
    }
    // The variants with the most flips.
    let mut per: Vec<(usize, usize)> = (0..plan.variants.len())
        .filter(|&i| plan.variants[i].index > 0)
        .map(|i| (i, changes.iter().filter(|c| c.variant == i && c.is_flip()).count()))
        .filter(|(_, n)| *n > 0)
        .collect();
    per.sort_by_key(|&(i, n)| (std::cmp::Reverse(n), i));
    if !per.is_empty() {
        lines.push(String::new());
        lines.push("Variants with the most flips:".into());
        for (i, n) in per.iter().take(5) {
            lines.push(format!(
                "  {n:>4}  {}  {}",
                plan.id_at(*i),
                plan.variants[*i].describe()
            ));
        }
    }
    lines.push(String::new());
    if plan.cells.len() > 1 {
        for (ci, r) in plan.ranges() {
            let (tests, stable, flipped, not_run) = counts(&changes, per_variant, r);
            lines.push(format!(
                "Stability {}: {tests} tests, {stable} stable, {flipped} flipped, {not_run} changed to or from not run",
                plan.cells[ci]
            ));
        }
    }
    let (tests, stable, flipped, not_run) = counts(&changes, per_variant, 0..plan.variants.len().min(per_cell.max(1)));
    let (tests, stable, flipped, not_run) = if plan.cells.len() > 1 {
        // Over all cells: a test counts as flipped when it flipped in any cell.
        let changed: std::collections::BTreeSet<&str> = changes.iter().map(|c| c.test.as_str()).collect();
        let fl: std::collections::BTreeSet<&str> = changes
            .iter()
            .filter(|c| c.is_flip())
            .map(|c| c.test.as_str())
            .collect();
        (
            tests,
            tests.saturating_sub(changed.len()),
            fl.len(),
            changed.len() - fl.len(),
        )
    } else {
        (tests, stable, flipped, not_run)
    };
    lines.push(format!(
        "Stability: {tests} tests, {stable} stable, {flipped} flipped, {not_run} changed to or from not run; master seed {}, config {}",
        plan.master_seed, plan.config_hash
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

/// The `stability` object of `--output-json` (ADR-058 section 5), with the cells of this process and a summary of
/// each.
pub fn json(plan: &Plan, out: &Outcome, per_variant: &[Vec<Seen>]) -> serde_json::Value {
    let variants: Vec<serde_json::Value> = (0..plan.variants.len())
        .map(|i| {
            let v = &plan.variants[i];
            let values: serde_json::Map<String, serde_json::Value> = v
                .values
                .iter()
                .map(|(k, val)| (k.clone(), val.clone().into()))
                .collect();
            serde_json::json!({ "id": plan.id_at(i), "cell": plan.cells[plan.cell_of[i]], "values": values })
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
                "cell": plan.cells[plan.cell_of[c.variant]],
                "base": c.base.text(),
                "variant": plan.id_at(c.variant),
                "outcome": c.outcome.text(),
                "message": c.message,
                "minimal": minimal,
                "dataAxis": false,
                "finding": c.is_flip(),
                "reliesOnAssumption": c.assumption,
            })
        })
        .collect();
    let summary = |r: std::ops::Range<usize>| {
        let (tests, stable, flipped, not_run) = counts(&out.changes, per_variant, r.clone());
        serde_json::json!({
            "variants": r.len(),
            "tests": tests,
            "stable": stable,
            "flipped": flipped,
            "notRunChanges": not_run,
        })
    };
    let cells: Vec<serde_json::Value> = plan
        .ranges()
        .into_iter()
        .map(|(ci, r)| serde_json::json!({ "cell": plan.cells[ci], "summary": summary(r) }))
        .collect();
    let changed: std::collections::BTreeSet<&str> = out.changes.iter().map(|c| c.test.as_str()).collect();
    let flipped: std::collections::BTreeSet<&str> = out
        .changes
        .iter()
        .filter(|c| c.is_flip())
        .map(|c| c.test.as_str())
        .collect();
    let tests = per_variant.first().map_or(0, Vec::len);
    serde_json::json!({
        "format": 1,
        "config": plan.config_hash,
        "masterSeed": plan.master_seed,
        "runtimeBuild": crate::version::line(),
        "variants": variants,
        "cells": cells,
        "flips": flips,
        "summary": {
            "variants": plan.variants.len(),
            "tests": tests,
            "stable": tests.saturating_sub(changed.len()),
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
        let id = plan.id_at(vi);
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
            xml(&id),
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
                        id,
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
        Plan::new(&cfg, "29.0-w1", None, 7, 7, 1000, "sha256:x".into(), "test".into()).unwrap()
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
        // ADR-061 section 5: the id carries the run seed.
        assert!(text.contains("29.0-w1/1@7  order.methods=reverse"), "{text}");
        assert!(text.contains("pass -> fail under 29.0-w1/1@7"), "{text}");
        assert!(text.contains("2 tests, 1 stable, 1 flipped"), "{text}");
        assert_eq!(exit_code(0, out.flips, false), 1);
        assert_eq!(exit_code(0, out.flips, true), 0);
        let j = json(&plan, &out, &runs);
        assert_eq!(j["summary"]["flipped"], 1);
        assert_eq!(j["flips"][0]["minimal"]["order.methods"], "reverse");
        assert_eq!(j["variants"][1]["id"], "29.0-w1/1@7");
    }

    /// ADR-058 section 3: cells that differ only in `environment` run in one process; each cell has its own base
    /// variant, ids name the cell, and a test whose outcome differs between the cells' base variants is no flip.
    #[test]
    fn environment_cells_share_the_process_and_compare_within_each_cell() {
        let plan = plan_of(
            r#"{"format":1,"matrix":{"environment":["saas","onprem"]},"dimensions":{"order.methods":["reverse"]}}"#,
        );
        assert_eq!(plan.cells, vec!["29.0-w1-saas", "29.0-w1-onprem"]);
        assert_eq!(plan.variants.len(), 4);
        let ids: Vec<String> = (0..4).map(|i| plan.id_at(i)).collect();
        assert_eq!(
            ids,
            [
                "29.0-w1-saas/0@7",
                "29.0-w1-saas/1@7",
                "29.0-w1-onprem/0@7",
                "29.0-w1-onprem/1@7"
            ]
        );
        assert_eq!(plan.variants[2].perturb.environment, Some(al_host::Environment::OnPrem));
        assert!(plan.variants[2].perturb.method_order == stability::MethodOrder::Declared);
        let runs = vec![
            vec![seen("Codeunit1.A", Kind::Pass), seen("Codeunit1.B", Kind::Pass)],
            vec![seen("Codeunit1.A", Kind::Pass), seen("Codeunit1.B", Kind::Pass)],
            // On-prem: A fails in the cell's base variant already (a difference between cells, not a flip).
            vec![seen("Codeunit1.A", Kind::Fail), seen("Codeunit1.B", Kind::Pass)],
            vec![seen("Codeunit1.A", Kind::Fail), seen("Codeunit1.B", Kind::Fail)],
        ];
        let out = compare(&plan, &runs);
        assert_eq!(out.flips, 1);
        let text = out.lines.join("\n");
        assert!(text.contains("pass -> fail under 29.0-w1-onprem/1@7"), "{text}");
        assert!(text.contains("29.0-w1-saas: 2 tests, 2 stable, 0 flipped"), "{text}");
        assert!(text.contains("29.0-w1-onprem: 2 tests, 1 stable, 1 flipped"), "{text}");
        let j = json(&plan, &out, &runs);
        assert_eq!(j["flips"][0]["cell"], "29.0-w1-onprem");
        assert_eq!(j["flips"][0]["variant"], "29.0-w1-onprem/1@7");
        assert_eq!(j["variants"][2]["cell"], "29.0-w1-onprem");
        assert_eq!(j["cells"][1]["summary"]["flipped"], 1);
        assert_eq!(j["summary"]["flipped"], 1);
        // A replay names its cell.
        let v = plan.replay_in("29.0-w1-onprem/1@7").unwrap();
        assert_eq!(v.perturb.environment, Some(al_host::Environment::OnPrem));
        assert!(
            plan.replay_in("29.0-w1-elsewhere/1")
                .unwrap_err()
                .contains("29.0-w1-onprem")
        );
    }

    /// The cell names of the matrix axes read back into their values.
    #[test]
    fn cell_names_read_back() {
        let c = parse_cell("29.0-w1-companies2-onprem").unwrap();
        assert_eq!(c.version, "29.0");
        assert_eq!(c.country, "w1");
        assert_eq!(c.companies, Some(2));
        assert_eq!(c.environment, Some(al_host::Environment::OnPrem));
        let c = parse_cell("28.5-us").unwrap();
        assert_eq!((c.companies, c.environment), (None, None));
        assert!(parse_cell("29.0").is_none());
        assert!(parse_cell("29.0-w1-nonsense").is_none());
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
