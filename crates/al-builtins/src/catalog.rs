//! Generating the signature catalog from the docs scrape (ADR-008).

use crate::signature::{DocEntry, Signature};
use al_version::{Runtime, Target};
use std::collections::{BTreeMap, HashMap};

/// The checked-in catalog, compiled into the binary (no file access at runtime, ADR-006).
const SIGNATURES: &str = include_str!("../catalog/signatures.txt");
const OVERRIDES: &str = include_str!("../catalog/overrides.txt");
const DOCS_RUNTIMES: &str = include_str!("../catalog/docs-runtimes.txt");

/// Applies `overrides` to the generated `lines`. Every change must follow a
/// `#` evidence comment, and every removed line must exist.
pub fn apply_overrides(lines: &[&str], overrides: &str) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
    let mut has_evidence = false;
    for (n, raw) in overrides.lines().enumerate() {
        let line = raw.trim_end();
        if line.trim().is_empty() {
            has_evidence = false;
        } else if line.starts_with('#') {
            has_evidence = true;
        } else if let Some(rest) = line.strip_prefix("- ").or_else(|| line.strip_prefix("+ ")) {
            if !has_evidence {
                return Err(format!(
                    "overrides.txt:{}: change without a preceding `#` evidence comment",
                    n + 1
                ));
            }
            if line.starts_with('-') {
                let i = out
                    .iter()
                    .position(|l| l == rest)
                    .ok_or_else(|| format!("overrides.txt:{}: `{rest}` is not in the catalog", n + 1))?;
                out.remove(i);
            } else {
                out.push(rest.to_string());
            }
        } else {
            return Err(format!("overrides.txt:{}: expected `#`, `- ` or `+ `", n + 1));
        }
    }
    out.sort();
    Ok(out)
}

/// Receivers whose static methods can be called without a qualifier
/// (`Message(...)`, `Format(...)`, `CopyStr(...)`, `Commit()`, `BindSubscription(...)`).
/// `File` is here because Base Application's File Management codeunit calls `Download(...)`,
/// `Upload(...)` and `Exists(...)` unqualified (the static methods of the File type).
/// `SecretText` is here because corpus types/TestSecretText.al:73 calls
/// `SecretStrSubstNo(...)` unqualified and BC compiles it.
pub const GLOBAL_RECEIVERS: &[&str] = &["System", "Dialog", "Text", "Database", "Session", "SecretText", "File"];

#[derive(Clone)]
pub struct Catalog {
    /// Every overload of every version, in id order: untagged first, then by tag (ADR-015 3.3).
    pub signatures: Vec<Signature>,
    by_method: HashMap<(String, String), Vec<usize>>,
    globals: HashMap<String, Vec<usize>>,
    /// The app runtime this catalog serves; `None` shows every overload (the newest).
    runtime: Option<Runtime>,
    /// Per signature (same index): the runtime Microsoft's docs give for the overload ("Available or changed with
    /// runtime version X", `catalog/docs-runtimes.txt`), `None` for a line `overrides.txt` added (no docs page).
    docs: Vec<Option<Runtime>>,
    /// `Target::app_runtimes`: the declared runtime of an app under test below `runtime`, by app id in lower case.
    app_runtimes: HashMap<String, Runtime>,
}

impl Catalog {
    /// The whole catalog, as for the newest runtime: nothing is hidden.
    ///
    /// Parsed once per process (about 750 signatures); a caller gets a copy of it. Code that only reads the catalog
    /// takes [`Catalog::shared`] and copies nothing.
    pub fn load() -> Catalog {
        Catalog::shared().clone()
    }

    /// The same catalog, shared by the whole process.
    pub fn shared() -> &'static Catalog {
        static CATALOG: std::sync::OnceLock<Catalog> = std::sync::OnceLock::new();
        CATALOG.get_or_init(|| Catalog::build(None))
    }

    /// The catalog an app of the target's runtime sees: `overloads` and `global`
    /// leave out what is tagged newer; [`Catalog::hidden`] returns it. Ids and
    /// `by_id` do not depend on the target (ADR-015 3.3).
    pub fn for_target(target: &Target) -> Catalog {
        let mut c = Catalog::build(Some(target.runtime));
        c.app_runtimes = target
            .app_runtimes
            .iter()
            .filter(|(_, r)| *r < target.runtime)
            .map(|(id, r)| (id.to_lowercase(), *r))
            .collect();
        c
    }

    fn build(runtime: Option<Runtime>) -> Catalog {
        let base: Vec<&str> = SIGNATURES
            .lines()
            .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
            .collect();
        let lines = apply_overrides(&base, OVERRIDES).unwrap_or_else(|e| panic!("{e}"));
        let mut signatures: Vec<Signature> = lines
            .iter()
            .map(|l| Signature::from_line(l).unwrap_or_else(|e| panic!("bad catalog line `{l}`: {e}")))
            .collect();
        number_order(&mut signatures);
        // The docs runtime of a generated line; a line only `overrides.txt` has gets none.
        let documented: std::collections::HashSet<&str> = base.iter().copied().collect();
        let listed: HashMap<&str, Option<Runtime>> = DOCS_RUNTIMES
            .lines()
            .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
            .filter_map(|l| l.split_once('\t'))
            .map(|(line, r)| (line, Runtime::parse(r).ok()))
            .collect();
        let one = Runtime { major: 1, minor: 0 };
        let docs = signatures
            .iter()
            .map(|s| {
                let line = s.to_line();
                if !documented.contains(line.as_str()) {
                    return None;
                }
                listed.get(line.as_str()).copied().unwrap_or(Some(one))
            })
            .collect();
        let mut by_method: HashMap<(String, String), Vec<usize>> = HashMap::new();
        let mut globals: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, s) in signatures.iter().enumerate() {
            by_method
                .entry((s.receiver.to_lowercase(), s.name.to_lowercase()))
                .or_default()
                .push(i);
            if !s.instance && GLOBAL_RECEIVERS.iter().any(|g| g.eq_ignore_ascii_case(&s.receiver)) {
                globals.entry(s.name.to_lowercase()).or_default().push(i);
            }
        }
        Catalog {
            signatures,
            by_method,
            globals,
            runtime,
            docs,
            app_runtimes: HashMap::new(),
        }
    }

    /// The runtime Microsoft's docs give for the overload with id `id` ("Available or changed with runtime
    /// version X"; `corpus/scripts/al-surface.json`, field `runtime_version`). "Or changed": the overload may be
    /// older than X, so the value clears a call at runtime X and later and decides nothing below it. `None`: the
    /// docs do not list the overload (a line of `overrides.txt`).
    pub fn docs_runtime(&self, id: &str) -> Option<Runtime> {
        let (qual, n) = id.rsplit_once('#')?;
        let (recv, name) = qual.rsplit_once('.')?;
        let n: usize = n.parse().ok()?;
        let i = *self
            .by_method
            .get(&(recv.to_lowercase(), name.to_lowercase()))?
            .get(n.checked_sub(1)?)?;
        self.docs[i]
    }

    /// The runtime an app under test declares when it is below this catalog's runtime (`Target::app_runtimes`).
    pub fn app_runtime(&self, app_id: &str) -> Option<Runtime> {
        if self.app_runtimes.is_empty() {
            return None;
        }
        self.app_runtimes.get(&app_id.to_lowercase()).copied()
    }

    /// The runtime this catalog serves, or `None` when it shows everything.
    pub fn runtime(&self) -> Option<Runtime> {
        self.runtime
    }

    /// The newest runtime any overload is tagged with, else the runtime floor:
    /// how far the docs behind this catalog reach (ADR-015 3.3).
    pub fn newest_documented_runtime(&self) -> Runtime {
        self.signatures
            .iter()
            .filter_map(|s| s.since)
            .max()
            .unwrap_or_else(Runtime::floor)
    }

    fn visible(&self, s: &Signature) -> bool {
        match (s.since, self.runtime) {
            (Some(since), Some(rt)) => since <= rt,
            _ => true,
        }
    }

    /// Overloads of `receiver.method` (case-insensitive), static and instance,
    /// that exist at this catalog's runtime.
    pub fn overloads(&self, receiver: &str, method: &str) -> Vec<&Signature> {
        self.by_method
            .get(&(receiver.to_lowercase(), method.to_lowercase()))
            .map(|v| {
                v.iter()
                    .map(|&i| &self.signatures[i])
                    .filter(|s| self.visible(s))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Overloads of `receiver.method` that exist only in a newer runtime.
    pub fn hidden(&self, receiver: &str, method: &str) -> Vec<&Signature> {
        self.by_method
            .get(&(receiver.to_lowercase(), method.to_lowercase()))
            .map(|v| {
                v.iter()
                    .map(|&i| &self.signatures[i])
                    .filter(|s| !self.visible(s))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Whether any type of the catalog, in any version, has a method or function called `name`
    /// (case-insensitive). A name for which this is false is a name no AL type can supply.
    pub fn knows_name(&self, name: &str) -> bool {
        self.signatures.iter().any(|s| s.name.eq_ignore_ascii_case(name))
    }

    /// Overloads callable unqualified as `name(...)` at this catalog's runtime.
    pub fn global(&self, name: &str) -> Vec<&Signature> {
        self.globals
            .get(&name.to_lowercase())
            .map(|v| {
                v.iter()
                    .map(|&i| &self.signatures[i])
                    .filter(|s| self.visible(s))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Unqualified overloads that exist only in a newer runtime.
    pub fn hidden_global(&self, name: &str) -> Vec<&Signature> {
        self.globals
            .get(&name.to_lowercase())
            .map(|v| {
                v.iter()
                    .map(|&i| &self.signatures[i])
                    .filter(|s| !self.visible(s))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The signature with overload id `id` (`Record.Insert#2`).
    pub fn by_id(&self, id: &str) -> Option<&Signature> {
        let (qual, n) = id.rsplit_once('#')?;
        let (recv, name) = qual.rsplit_once('.')?;
        let n: usize = n.parse().ok()?;
        self.by_method
            .get(&(recv.to_lowercase(), name.to_lowercase()))?
            .get(n.checked_sub(1)?)
            .map(|&i| &self.signatures[i])
    }

    /// Stable overload id: `Receiver.Method#n`, n counting from 1 in catalog order.
    pub fn id(&self, sig: &Signature) -> String {
        let all = &self.by_method[&(sig.receiver.to_lowercase(), sig.name.to_lowercase())];
        let n = all
            .iter()
            .position(|&i| &self.signatures[i] == sig)
            .expect("signature from this catalog")
            + 1;
        format!("{}.{}#{n}", sig.receiver, sig.name)
    }
}

/// Tags up to this runtime keep their place in the sorted line order. Overload ids existed
/// before tags did (`Page.Run#4`, `Report.Run#3`), and registered implementations are keyed by
/// them, so the sixteen overloads tagged 17.0 when tags were introduced keep their numbers.
const NUMBERED_IN_PLACE_UP_TO: Runtime = Runtime { major: 17, minor: 0 };

/// Puts `signatures` (sorted by line) in id order: an overload tagged newer than
/// [`NUMBERED_IN_PLACE_UP_TO`] comes after every other one, so a later runtime that adds an
/// overload never renumbers an existing one (ADR-015 3.3 point 6). The sort is stable, so
/// inside one tag the order stays the sorted line order.
fn number_order(signatures: &mut [Signature]) {
    signatures.sort_by_key(|s| s.since.filter(|r| *r > NUMBERED_IN_PLACE_UP_TO));
}

/// `catalog/docs-runtimes.txt`: `<catalog line>\t<runtime>` for every overload of `al-surface.json` whose docs
/// runtime is above 1.0 (`?` when the docs give none), sorted. A line of two entries gets the higher runtime.
pub fn generate_docs_runtimes(surface_json: &str) -> Result<Vec<String>, String> {
    let surface: BTreeMap<String, BTreeMap<String, Vec<DocEntry>>> =
        serde_json::from_str(surface_json).map_err(|e| e.to_string())?;
    let mut by_line: BTreeMap<String, Option<Runtime>> = BTreeMap::new();
    for methods in surface.values() {
        for overloads in methods.values() {
            for o in overloads {
                let line = Signature::from_doc(o)?.to_line();
                let r = o
                    .runtime_version
                    .as_deref()
                    .and_then(|v| Runtime::parse(v.trim().trim_end_matches('.')).ok());
                let e = by_line.entry(line).or_insert(r);
                *e = match (*e, r) {
                    (Some(a), Some(b)) => Some(a.max(b)),
                    _ => None,
                };
            }
        }
    }
    let one = Runtime { major: 1, minor: 0 };
    Ok(by_line
        .into_iter()
        .filter(|(_, r)| r.is_none_or(|r| r > one))
        .map(|(l, r)| format!("{l}\t{}", r.map_or_else(|| "?".to_string(), |r| r.to_string())))
        .collect())
}

/// All overloads in `al-surface.json`, one catalog line each, sorted. Entries
/// whose syntax line does not parse are returned as errors, never dropped.
pub fn generate(surface_json: &str) -> Result<Vec<String>, String> {
    let surface: BTreeMap<String, BTreeMap<String, Vec<DocEntry>>> =
        serde_json::from_str(surface_json).map_err(|e| e.to_string())?;
    let mut lines = Vec::new();
    let mut errors = Vec::new();
    for (ty, methods) in &surface {
        for (key, overloads) in methods {
            for o in overloads {
                match Signature::from_doc(o) {
                    Ok(s) => lines.push(s.to_line()),
                    Err(e) => errors.push(format!("{ty}.{key}: {e}")),
                }
            }
        }
    }
    if !errors.is_empty() {
        return Err(errors.join("\n"));
    }
    lines.sort();
    lines.dedup();
    Ok(lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_catalog_is_parsed_once_and_every_copy_is_the_one_a_fresh_parse_gives() {
        let fresh = Catalog::build(None);
        assert!(std::ptr::eq(Catalog::shared(), Catalog::shared()));
        assert_eq!(Catalog::load().signatures, fresh.signatures);
        assert_eq!(Catalog::shared().signatures, fresh.signatures);
        let id = Catalog::shared().id(&fresh.signatures[10]);
        assert_eq!(Catalog::load().id(&fresh.signatures[10]), id);
    }

    #[test]
    fn overrides_remove_and_add_lines() {
        let out = apply_overrides(&["A.x inst ()", "A.y inst ()"], "# why\n- A.x inst ()\n+ A.z inst ()\n").unwrap();
        assert_eq!(out, vec!["A.y inst ()", "A.z inst ()"]);
    }

    #[test]
    fn override_without_evidence_is_rejected() {
        assert!(apply_overrides(&["A.x inst ()"], "- A.x inst ()\n").is_err());
        assert!(
            apply_overrides(&["A.x inst ()"], "# why\n- A.x inst ()\n\n+ A.z inst ()\n").is_err(),
            "blank line ends a block"
        );
    }

    #[test]
    fn removing_a_line_that_does_not_exist_is_rejected() {
        assert!(apply_overrides(&["A.x inst ()"], "# why\n- A.nope inst ()\n").is_err());
    }

    #[test]
    fn checked_in_overrides_apply_and_parse() {
        let base: Vec<&str> = SIGNATURES.lines().filter(|l| !l.starts_with('#')).collect();
        for l in apply_overrides(&base, OVERRIDES).unwrap() {
            Signature::from_line(&l).unwrap();
        }
        let c = Catalog::load();
        assert!(
            c.overloads("Codeunit", "Run")
                .iter()
                .any(|s| s.instance && s.params[0].optional)
        );
    }

    #[test]
    fn every_catalog_line_round_trips() {
        for l in SIGNATURES.lines().filter(|l| !l.starts_with('#')) {
            assert_eq!(Signature::from_line(l).unwrap().to_line(), l);
        }
    }

    #[test]
    fn lookup_by_receiver_and_method_is_case_insensitive() {
        let c = Catalog::load();
        assert_eq!(c.overloads("record", "INSERT").len(), 3);
        assert!(c.overloads("Record", "NoSuchMethod").is_empty());
    }

    #[test]
    fn globals_are_static_methods_of_the_global_receivers() {
        let c = Catalog::load();
        assert!(!c.global("Message").is_empty());
        assert!(!c.global("CopyStr").is_empty());
        assert!(!c.global("Commit").is_empty());
        assert!(c.global("Insert").is_empty(), "instance methods are not global");
        assert!(c.global("Open").is_empty(), "Dialog.Open is an instance method");
    }

    #[test]
    fn by_id_is_the_inverse_of_id() {
        let c = Catalog::load();
        for s in c.overloads("Record", "Insert") {
            assert_eq!(c.by_id(&c.id(s)), Some(s));
        }
        assert!(c.by_id("Record.Insert#9").is_none());
    }

    fn sig(line: &str) -> Signature {
        Signature::from_line(line).unwrap()
    }

    #[test]
    fn a_runtime_tag_round_trips() {
        for l in [
            "A.x inst (P: Text) -> Boolean? @17.0",
            "A.y static () @18.0",
            "A.z inst () -> Text @17.1",
        ] {
            let s = sig(l);
            assert_eq!(s.to_line(), l);
            assert!(s.since.is_some());
        }
        assert_eq!(
            sig("A.y static () @18.0").since,
            Some(al_version::Runtime { major: 18, minor: 0 })
        );
        assert_eq!(sig("A.y static ()").since, None);
        assert!(Signature::from_line("A.y static () @x").is_err());
        assert!(Signature::from_line("A.y static () @17.0 junk").is_err());
    }

    const SURFACE: &str = r#"{"Record":{
        "Old":[{"syntax":"Record.Old()","runtime_version":"1.0."}],
        "Floor":[{"syntax":"Record.Floor()","runtime_version":"16.0."}],
        "New":[{"syntax":"Record.New()","runtime_version":"17.0."}],
        "Newer":[{"syntax":"Record.Newer()","runtime_version":"18.0."}],
        "Untagged":[{"syntax":"Record.Untagged()"}]}}"#;

    #[test]
    fn generate_tags_only_overloads_above_the_floor() {
        let lines = generate(SURFACE).unwrap();
        assert_eq!(
            lines,
            vec![
                "Record.Floor static ()",
                "Record.New static () @17.0",
                "Record.Newer static () @18.0",
                "Record.Old static ()",
                "Record.Untagged static ()",
            ]
        );
    }

    #[test]
    fn docs_runtimes_list_the_overloads_above_1_0() {
        assert_eq!(
            generate_docs_runtimes(SURFACE).unwrap(),
            vec![
                "Record.Floor static ()\t16.0",
                "Record.New static () @17.0\t17.0",
                "Record.Newer static () @18.0\t18.0",
                "Record.Untagged static ()\t?",
            ]
        );
    }

    #[test]
    fn the_docs_runtime_of_an_overload_comes_from_the_scrape_and_an_override_has_none() {
        let c = Catalog::load();
        let rt = |id: &str| c.docs_runtime(id);
        let of = |recv: &str, name: &str, params: usize| {
            let s = c
                .overloads(recv, name)
                .into_iter()
                .find(|s| s.params.len() == params)
                .unwrap();
            c.id(s)
        };
        // NavApp.GetResourceAsText(ResourceName: Text [, Encoding: TextEncoding]): "14.0." in the scrape.
        assert_eq!(
            rt(&of("NavApp", "GetResourceAsText", 2)),
            Some(Runtime { major: 14, minor: 0 })
        );
        assert_eq!(rt(&of("Dialog", "Message", 2)), Some(Runtime { major: 1, minor: 0 }));
        // TestPage.Filter is a line of overrides.txt only.
        assert_eq!(rt(&of("TestPage", "Filter", 0)), None);
    }

    #[test]
    fn a_target_gives_the_declared_runtime_of_an_app_below_its_own() {
        let mut t = target("27.5");
        t.app_runtimes = vec![
            ("App-A".into(), Runtime { major: 13, minor: 0 }),
            ("b".into(), Runtime { major: 16, minor: 0 }),
        ];
        let c = Catalog::for_target(&t);
        assert_eq!(c.app_runtime("app-a"), Some(Runtime { major: 13, minor: 0 }));
        assert_eq!(c.app_runtime("b"), None, "at the target's own runtime");
        assert_eq!(c.app_runtime("c"), None);
    }

    fn target(bc: &str) -> al_version::Target {
        al_version::Target::for_bc(al_version::BcVersion::parse(bc).unwrap())
    }

    #[test]
    fn a_target_sees_only_overloads_up_to_its_runtime() {
        let c27 = Catalog::for_target(&target("27.5"));
        assert!(c27.overloads("Record", "FullyQualifiedName").is_empty());
        let hidden = c27.hidden("Record", "FullyQualifiedName");
        assert_eq!(hidden.len(), 1);
        assert_eq!(hidden[0].since, Some(al_version::Runtime { major: 17, minor: 0 }));
        assert!(!c27.overloads("Record", "Insert").is_empty());
        assert!(c27.hidden("Record", "Insert").is_empty());

        let c28 = Catalog::for_target(&target("28"));
        assert_eq!(c28.overloads("Record", "FullyQualifiedName").len(), 1);
        assert!(c28.hidden("Record", "FullyQualifiedName").is_empty());
        // `load()` is the newest: everything is visible.
        assert_eq!(Catalog::load().overloads("Record", "FullyQualifiedName").len(), 1);
    }

    #[test]
    fn hidden_overloads_do_not_leak_into_globals_and_ids_still_resolve() {
        let c27 = Catalog::for_target(&target("27"));
        // An id resolves whatever the target (the IR holds ids, ADR-008).
        let all = Catalog::load();
        let tagged = all.overloads("Record", "FullyQualifiedName")[0];
        assert_eq!(c27.by_id(&all.id(tagged)), Some(tagged));
        assert_eq!(c27.id(tagged), all.id(tagged));
    }

    #[test]
    fn an_overload_tagged_after_the_numbered_in_place_runtime_is_numbered_last() {
        let mut v: Vec<Signature> = [
            "A.m static (V: Text) @18.0",
            "A.m static (W: Text) @17.0",
            "A.m static (X: Text)",
            "A.m static (Y: Text) @19.0",
            "A.m static (Z: Text)",
        ]
        .iter()
        .map(|l| sig(l))
        .collect();
        number_order(&mut v);
        let order: Vec<String> = v.iter().map(|s| s.params[0].name.clone()).collect();
        // 17.0 and untagged keep their sorted places; 18.0 and 19.0 follow, oldest first.
        assert_eq!(order, ["W", "X", "Z", "V", "Y"]);
    }

    #[test]
    fn the_overloads_tagged_17_keep_the_ids_they_had_before_tags() {
        let c = Catalog::load();
        let by_name = c
            .overloads("Codeunit", "Run")
            .into_iter()
            .find(|s| s.since.is_some())
            .unwrap();
        assert_eq!(c.id(by_name), "Codeunit.Run#2");
        let by_number = c
            .overloads("Codeunit", "Run")
            .into_iter()
            .find(|s| s.since.is_none() && !s.instance)
            .unwrap();
        assert_eq!(c.id(by_number), "Codeunit.Run#3");
    }

    #[test]
    fn overload_ids_are_stable_and_distinct() {
        let c = Catalog::load();
        let ids: Vec<String> = c.overloads("Record", "Insert").iter().map(|s| c.id(s)).collect();
        assert_eq!(ids, vec!["Record.Insert#1", "Record.Insert#2", "Record.Insert#3"]);
    }
}
