//! The run seed of `al-runner` (ADR-061): `--seed N`, else `ALRUNNER_SEED`, else `AL_RUNNER_SEED` (v2's name), else
//! a fresh seed this command line picks from the OS (the core crates never pick one, ADR-006). It is printed at the
//! start and in the summary, and written to `--output-json`, `--junit` and `serve` summaries.

use crate::args::TestArgs;
use al_testrun::seed::{RunSeed, SeedSource};

/// The environment variables of the run seed, in the order they are read.
pub const ENV_VARS: [&str; 2] = ["ALRUNNER_SEED", "AL_RUNNER_SEED"];

/// A fresh run seed from 0 to 2147483647, from the OS's randomness (`RandomState` keys, no new dependency).
pub fn fresh() -> i32 {
    use std::hash::{BuildHasher, Hasher};
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u128(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos()),
    );
    (h.finish() % (u64::from(al_host::seed::MAX_RUN_SEED as u32) + 1)) as i32
}

/// The bad-invocation text of a run seed that is not an integer from 0 to 2147483647 (exit 2).
pub fn bad_seed(what: &str, text: &str) -> String {
    format!("{what} needs a whole number from 0 to 2147483647, got `{text}`")
}

/// The run seed by precedence: `cli`, then the environment, then fresh. A bad environment value is an error (exit 2).
pub fn resolve(cli: Option<i32>, env: &dyn Fn(&str) -> Option<String>) -> Result<(i32, SeedSource), String> {
    if let Some(n) = cli {
        return Ok((n, SeedSource::Option));
    }
    for var in ENV_VARS {
        if let Some(text) = env(var).filter(|t| !t.trim().is_empty()) {
            return al_host::seed::parse(&text)
                .map(|n| (n, SeedSource::Env))
                .ok_or_else(|| bad_seed(var, &text));
        }
    }
    Ok((fresh(), SeedSource::Fresh))
}

/// Fixes the run seed of a command line once, before anything runs, so every part of the run (both sides of an
/// impact run, every matrix cell, every stability variant) uses the same one. A `--stability-replay` id with a seed
/// (`<cell>/<n>@<seed>`) sets it, unless `--seed` names another (exit 2).
pub fn ensure(a: &mut TestArgs) -> Result<(), String> {
    if a.run_seed.is_some() {
        return Ok(());
    }
    let from_id = a
        .stability_replay
        .as_deref()
        .and_then(|r| al_testrun::stability::split_replay_seed(r).1);
    let cli = match (a.seed, from_id) {
        (Some(s), Some(id)) if s != id => {
            return Err(format!(
                "--seed {s} and the seed {id} of the --stability-replay id differ; give one of them"
            ));
        }
        (s, id) => s.or(id),
    };
    let (seed, source) = resolve(cli, &|v| std::env::var(v).ok())?;
    a.run_seed = Some(RunSeed {
        seed,
        source,
        // Owner decision 2: `Library - Random` gets the derived seed only when asked, or in a stability run.
        library: a.seed_library || a.stability.is_some() || a.stability_replay.is_some(),
    });
    Ok(())
}

/// The run seed of `a`: the one [`ensure`] fixed, else a fresh one (a caller that did not go through `ensure`).
pub fn of(a: &TestArgs) -> RunSeed {
    a.run_seed.unwrap_or_else(|| RunSeed {
        library: a.seed_library,
        ..RunSeed::new(fresh(), SeedSource::Fresh)
    })
}

/// The line before the first test, and the first line of the summary's seed part.
pub fn line(s: RunSeed) -> String {
    format!("Seed: {}", s.seed)
}

/// The command that replays the first failure: `al-runner test <same paths> --seed N --test <name>`.
pub fn replay_line(s: RunSeed, apps: &[std::path::PathBuf], test: &str) -> String {
    let quote = |t: &str| {
        if t.is_empty() || t.contains(|c: char| c.is_whitespace() || c == '"' || c == '\'') {
            format!("\"{}\"", t.replace('"', "\\\""))
        } else {
            t.to_string()
        }
    };
    let paths: Vec<String> = apps.iter().map(|p| quote(&p.display().to_string())).collect();
    format!(
        "Replay: al-runner test {} --seed {} --test {}",
        paths.join(" "),
        s.seed,
        quote(test)
    )
}

/// `seed` and `seedSource` at the top of an `--output-json` document.
pub fn json(doc: String, s: RunSeed) -> String {
    match serde_json::from_str::<serde_json::Value>(&doc) {
        Ok(mut d) if d.is_object() => {
            d["seed"] = s.seed.into();
            d["seedSource"] = s.source.text().into();
            serde_json::to_string_pretty(&d).expect("a JSON value serializes")
        }
        _ => doc,
    }
}

/// The license of the run (ADR-060) as the `license` object of an `--output-json` document.
pub fn json_license(doc: String, license: &al_testrun::license::LicenseUse) -> String {
    match serde_json::from_str::<serde_json::Value>(&doc) {
        Ok(mut d) if d.is_object() => {
            d["license"] = license.json();
            serde_json::to_string_pretty(&d).expect("a JSON value serializes")
        }
        _ => doc,
    }
}

/// `<property name="seed" value="N" />` on every `testsuite` of a JUnit file, as in v2.
pub fn junit(xml: &str, s: RunSeed) -> String {
    with_properties(xml, &format!("<property name=\"seed\" value=\"{}\" />", s.seed))
}

/// The license of the run (ADR-060) as `license.<name>` properties on every `testsuite` of a JUnit file.
pub fn junit_license(xml: &str, license: &al_testrun::license::LicenseUse) -> String {
    let esc = |v: &str| {
        v.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
    };
    let props: String = license
        .properties()
        .iter()
        .map(|(k, v)| format!("<property name=\"license.{k}\" value=\"{}\" />", esc(v)))
        .collect();
    with_properties(xml, &props)
}

/// `prop` (one or more `<property>` elements) added to the properties of every `testsuite` of a JUnit file.
fn with_properties(xml: &str, prop: &str) -> String {
    let mut out = String::with_capacity(xml.len() + 64);
    let mut rest = xml;
    while let Some(at) = rest.find("<testsuite ") {
        let Some(end) = rest[at..].find('>').map(|e| at + e) else {
            break;
        };
        let self_closing = rest[..end].ends_with('/');
        if self_closing {
            out.push_str(rest[..end - 1].trim_end());
            out.push_str(&format!("><properties>{prop}</properties></testsuite>"));
            rest = &rest[end + 1..];
            continue;
        }
        out.push_str(&rest[..=end]);
        rest = &rest[end + 1..];
        let lead = rest.len() - rest.trim_start().len();
        if rest[lead..].starts_with("<properties>") {
            let open = lead + "<properties>".len();
            out.push_str(&rest[..open]);
            out.push_str(prop);
            rest = &rest[open..];
        } else {
            out.push_str(&format!("\n    <properties>{prop}</properties>"));
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |k| pairs.iter().find(|(n, _)| *n == k).map(|(_, v)| v.to_string())
    }

    #[test]
    fn the_seed_comes_from_the_option_then_the_environment_then_fresh() {
        let both = [("ALRUNNER_SEED", "5"), ("AL_RUNNER_SEED", "6")];
        assert_eq!(resolve(Some(4), &env(&both)), Ok((4, SeedSource::Option)));
        assert_eq!(resolve(None, &env(&both)), Ok((5, SeedSource::Env)));
        assert_eq!(
            resolve(None, &env(&[("AL_RUNNER_SEED", " 6 ")])),
            Ok((6, SeedSource::Env))
        );
        let (n, src) = resolve(None, &env(&[])).unwrap();
        assert_eq!(src, SeedSource::Fresh);
        assert!(n >= 0);
        assert!(
            resolve(None, &env(&[("ALRUNNER_SEED", "x")]))
                .unwrap_err()
                .contains("ALRUNNER_SEED")
        );
        assert!(resolve(None, &env(&[("ALRUNNER_SEED", "-3")])).is_err());
    }

    #[test]
    fn fresh_seeds_differ() {
        let seeds: std::collections::BTreeSet<i32> = (0..8).map(|_| fresh()).collect();
        assert!(seeds.len() > 1, "{seeds:?}");
    }

    #[test]
    fn every_testsuite_gets_the_seed_property() {
        let s = RunSeed::new(42, SeedSource::Option);
        let x = "<testsuites>\n  <testsuite name=\"A\" tests=\"1\">\n    <properties><property name=\"codeunit\" value=\"5\" /></properties>\n  </testsuite>\n  <testsuite name=\"B\" tests=\"0\">\n  </testsuite>\n  <testsuite name=\"C\" tests=\"0\" />\n</testsuites>\n";
        let y = junit(x, s);
        assert_eq!(y.matches("<property name=\"seed\" value=\"42\" />").count(), 3, "{y}");
        assert!(
            y.contains("<properties><property name=\"seed\" value=\"42\" /><property name=\"codeunit\""),
            "{y}"
        );
        assert!(y.contains("<testsuite name=\"C\" tests=\"0\"><properties>"), "{y}");
    }

    #[test]
    fn the_json_document_gets_seed_and_source() {
        let d = json("{\"total\":1}".into(), RunSeed::new(7, SeedSource::Env));
        let v: serde_json::Value = serde_json::from_str(&d).unwrap();
        assert_eq!(v["seed"], 7);
        assert_eq!(v["seedSource"], "env");
    }

    #[test]
    fn the_replay_line_names_the_paths_seed_and_test() {
        let s = RunSeed::new(4271833, SeedSource::Fresh);
        assert_eq!(
            replay_line(s, &["./app".into(), "my tests".into()], "Codeunit5.Posts"),
            "Replay: al-runner test ./app \"my tests\" --seed 4271833 --test Codeunit5.Posts"
        );
    }
}
