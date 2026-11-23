//! Option parsing, strict the way bcdb's is: an option a command does not accept fails the
//! command (a mistyped `--compayn` silently reading every company is a wrong answer
//! reported as success), a value-taking option without a value is refused, and a stray
//! positional argument is refused instead of dropped.
//!
//! Entry points: `command_keys`, `parse`.

use std::collections::HashMap;

const READ_OPTS: &[&str] = &[
    "table",
    "company",
    "app",
    "top",
    "select",
    "sha256",
    "merge-extensions",
    "symbols",
    "format",
];

pub fn command_keys(cmd: &str) -> Option<Vec<&'static str>> {
    Some(match cmd {
        "tables" => vec!["symbols"],
        "companies" | "check" => vec![],
        "describe" => vec!["table", "company", "app", "symbols"],
        "validate" => vec!["against"],
        "read" => READ_OPTS.to_vec(),
        "verify" => READ_OPTS.iter().copied().chain(["fixture"]).collect(),
        "serve" => vec!["symbols"],
        "export" => vec!["company", "out", "format", "jobs", "symbols", "all-tables"],
        "restore" => vec![
            "to",
            "replace",
            "no-replace",
            "dry-run",
            "batch-size",
            "table",
            "exclude-table",
            "rename-company",
            "create",
            "no-create",
            "allow-column-loss",
            "no-allow-column-loss",
            "include-system",
            "include-identity",
            "strict",
        ],
        _ => return None,
    })
}

const GLOBAL: &[&str] = &["prefetch"];
const VALUELESS: &[&str] = &[
    "prefetch",
    "merge-extensions",
    "replace",
    "no-replace",
    "dry-run",
    "include-system",
    "include-identity",
    "create",
    "no-create",
    "allow-column-loss",
    "no-allow-column-loss",
    "strict",
    "all-tables",
];

pub fn parse(args: &[String], cmd: &str) -> Result<HashMap<String, String>, String> {
    let accepted = command_keys(cmd).ok_or_else(|| format!("unknown command '{cmd}'"))?;
    let mut d = HashMap::new();
    let mut key: Option<String> = None;
    for a in args {
        if let Some(k) = a.strip_prefix("--") {
            if let Some(pending) = &key {
                return Err(format!("--{pending} needs a value, but '{a}' is the next option"));
            }
            if !accepted.contains(&k) && !GLOBAL.contains(&k) {
                let list: Vec<String> = accepted.iter().chain(GLOBAL).map(|k| format!("--{k}")).collect();
                return Err(format!(
                    "unknown option '{a}' for '{cmd}' — accepted: {}",
                    list.join(", ")
                ));
            }
            d.insert(k.to_string(), "true".to_string());
            key = if VALUELESS.contains(&k) {
                None
            } else {
                Some(k.to_string())
            };
        } else if let Some(k) = key.take() {
            d.insert(k, a.clone());
        } else {
            return Err(format!(
                "unexpected argument '{a}' for '{cmd}' — options are named, e.g. --table \"{a}\""
            ));
        }
    }
    if let Some(k) = key {
        return Err(format!("--{k} needs a value"));
    }
    if let Some(f) = d.get("format") {
        if f != "tsv" && f != "json" {
            return Err(format!("--format expects tsv or json, got '{f}'"));
        }
    }
    Ok(d)
}
