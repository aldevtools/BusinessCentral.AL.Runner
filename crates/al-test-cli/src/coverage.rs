//! The corpus coverage map: which builtin methods and overloads of the catalog some corpus test
//! calls (`--coverage-map`, `docs/corpus-coverage.md`).
//!
//! The checker resolves every call of the corpus's own files; an overload counts as pinned only when
//! the checker picked that exact overload (`Res::Builtin(id)`), and the map names one call site
//! (the first in file and line order, so the output does not depend on thread timing). The point is
//! to see, per type, what a probe topic still has to cover (owner decision 2026-10-15: close whole
//! topics, not one blocker at a time).

use crate::language_catalog::{LanguageCatalog, State, majors, runtime_receiver};
use al_builtins::catalog::Catalog;
use al_sema::Workspace;
use al_sema::check::{CheckResult, Res};
use al_sema::types::Ty;
use std::collections::BTreeMap;

/// One resolved builtin call in a file of the corpus.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Call {
    pub id: String,
    pub file: String,
    pub line: usize,
}

/// The builtin calls the checker resolved in the files of the workspace apps (not the dependency
/// apps). One entry per (overload, file, line). A call counts for the overload its argument types
/// select ([`selected_overload`]).
pub fn collect_calls(ws: &Workspace, checked: &CheckResult, catalog: &Catalog) -> Vec<Call> {
    let mut lines: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    let mut args: BTreeMap<usize, CallArgs> = BTreeMap::new();
    let mut out = std::collections::BTreeSet::new();
    for ((file, start, end), res) in checked.resolutions.iter() {
        let Res::Builtin(id) = res else { continue };
        let f = &ws.files[*file];
        if ws.kinds[f.app] != al_sema::AppKind::Workspace {
            continue;
        }
        // Offsets of the line starts, once per file.
        let starts = lines.entry(*file).or_insert_with(|| {
            let mut v = vec![0usize];
            v.extend(ws.text(*file).match_indices('\n').map(|(i, _)| i + 1));
            v
        });
        let line = starts.partition_point(|&s| s <= *start);
        let id = select_at(ws, checked, catalog, &mut args, (*file, *start, *end), id);
        out.insert(Call {
            id,
            file: f.path.clone(),
            line,
        });
    }
    out.into_iter().collect()
}

/// The overload the call at `at` (file, start, end) is to, by its argument types: `id` (the checker's
/// choice) unless [`selected_overload`] picks another. The argument ranges of a file are found once,
/// and only for a method with several overloads.
fn select_at(
    ws: &Workspace,
    checked: &CheckResult,
    catalog: &Catalog,
    args: &mut BTreeMap<usize, CallArgs>,
    (file, start, end): (usize, usize, usize),
    id: &str,
) -> String {
    let several = catalog
        .by_id(id)
        .is_some_and(|s| catalog.overloads(&s.receiver, &s.name).len() > 1);
    if !several {
        return id.to_string();
    }
    args.entry(file)
        .or_insert_with(|| call_args(ws, file))
        .get(&(start, end))
        .and_then(|a| {
            let tys: Vec<Ty> = a
                .iter()
                .map(|&(s, e)| checked.types.get(&(file, s, e)).cloned().unwrap_or(Ty::Unknown))
                .collect();
            selected_overload(catalog, id, &tys)
        })
        .unwrap_or_else(|| id.to_string())
}

/// The argument ranges of every call expression of a file, by the call's range.
type CallArgs = std::collections::HashMap<(usize, usize), Vec<(usize, usize)>>;

fn call_args(ws: &Workspace, file: usize) -> CallArgs {
    let range = |n: &al_syntax::SyntaxNode| (usize::from(n.text_range().start()), usize::from(n.text_range().end()));
    al_syntax::SyntaxNode::new_root(ws.green(file))
        .descendants()
        .filter(|n| n.kind() == al_syntax::SyntaxKind::CallExpr)
        .map(|n| {
            let args = n
                .children()
                .nth(1)
                .map(|a| a.children().map(|x| range(&x)).collect())
                .unwrap_or_default();
            (range(&n), args)
        })
        .collect()
}

/// The overload of the method of `id` that the argument types `args` select, when it is not `id`.
///
/// The checker keeps the first overload that fits when several fit equally (`pick` in
/// `al-sema/src/check/calls.rs`), so a call `FRef.TestField(5)` resolves to `TestField(Value: Any)`
/// although `TestField(Value: Integer)` is the overload BC's compiler binds (it picks the most
/// specific one). Here a parameter whose type is the argument's own type scores 2, an `Any`,
/// `Joker` or `Variant` parameter 1, and any other parameter rules the overload out; the overload
/// with the highest score wins when it is the only one with that score and scores higher than `id`.
/// An argument of unknown type fits only the generic parameters, so a call the checker could not
/// type keeps its overload.
pub fn selected_overload(catalog: &Catalog, id: &str, args: &[Ty]) -> Option<String> {
    let resolved = catalog.by_id(id)?;
    let score = |sig: &al_builtins::signature::Signature| -> Option<usize> {
        let required = sig.params.iter().filter(|p| !p.optional).count();
        let variadic = sig.params.last().is_some_and(|p| p.variadic);
        if args.len() < required || (!variadic && args.len() > sig.params.len()) {
            return None;
        }
        let mut total = 0;
        for (i, a) in args.iter().enumerate() {
            let p = sig.params.get(i).or(sig.params.last())?;
            let pty = p.ty.to_ascii_lowercase();
            // `String` is the catalog's other spelling of Text (`HttpHeaders.GetValues(Key: String, ...)`).
            let pty = match pty.split(['[', ' ']).next().unwrap_or("") {
                "string" => "text",
                w => w,
            };
            let generic = matches!(pty, "any" | "joker" | "variant");
            total += match type_word(a) {
                Some(w) if w == pty => 2,
                _ if generic => 1,
                _ => return None,
            };
        }
        Some(total)
    };
    let own = score(resolved).unwrap_or(0);
    let mut best: Option<(&al_builtins::signature::Signature, usize)> = None;
    let mut tie = false;
    for sig in catalog.overloads(&resolved.receiver, &resolved.name) {
        if sig.instance != resolved.instance {
            continue;
        }
        let Some(sc) = score(sig) else { continue };
        match best {
            Some((_, b)) if sc < b => {}
            Some((_, b)) if sc == b => tie = true,
            _ => {
                best = Some((sig, sc));
                tie = false;
            }
        }
    }
    let (sig, sc) = best?;
    (!tie && sc > own && sig != resolved).then(|| catalog.id(sig))
}

/// The catalog's word for the type of an argument (`integer`, `text`, `record`, `array`), as the
/// first word of a parameter type is spelled in lower case.
fn type_word(t: &Ty) -> Option<String> {
    Some(
        match t {
            Ty::Integer => "integer",
            Ty::BigInteger => "biginteger",
            Ty::Decimal => "decimal",
            Ty::Boolean => "boolean",
            Ty::Char => "char",
            Ty::Byte => "byte",
            Ty::Text(_) => "text",
            Ty::Code(_) => "code",
            Ty::Date => "date",
            Ty::Time => "time",
            Ty::DateTime => "datetime",
            Ty::Duration => "duration",
            Ty::DateFormula => "dateformula",
            Ty::Guid => "guid",
            Ty::Option(_) => "option",
            Ty::Enum(_) => "enum",
            Ty::Record { .. } => "record",
            Ty::Label => "label",
            Ty::Variant => "variant",
            Ty::Array(..) => "array",
            Ty::Builtin(name) => return Some(name.to_ascii_lowercase()),
            _ => return None,
        }
        .to_string(),
    )
}

/// How often Microsoft's apps (the dependency apps the corpus loads, not the AL Language test
/// libraries) call each builtin overload, by the checker's resolutions and the argument types (as
/// [`collect_calls`] counts them).
pub fn microsoft_calls(ws: &Workspace, checked: &CheckResult, catalog: &Catalog) -> BTreeMap<String, usize> {
    let mut out = BTreeMap::new();
    let mut args: BTreeMap<usize, CallArgs> = BTreeMap::new();
    for ((file, start, end), res) in checked.resolutions.iter() {
        let Res::Builtin(id) = res else { continue };
        let app = ws.files[*file].app;
        if ws.kinds[app] == al_sema::AppKind::Dependency && !ws.apps[app].name.contains("AL Language") {
            let id = select_at(ws, checked, catalog, &mut args, (*file, *start, *end), id);
            *out.entry(id).or_insert(0) += 1;
        }
    }
    out
}

/// The corpus apps whose files count as the corpus's own: `al-language`, the OnPrem app and the
/// OnPrem DotNet app (items only a `Target = OnPrem` app can name are pinned there), plus the corpus apps they depend on.
pub const CORPUS_TEST_APPS: [&str; 3] = ["al-language", "al-language-onprem", "al-language-onprem-dotnet"];

/// The workspace both coverage maps check: the corpus apps `al-language` depends on (the fixture and the cross-app
/// fixtures, in dependency order) and every app of `CORPUS_TEST_APPS` as workspace apps, then the dependency apps of
/// the corpus's `.alpackages`.
pub fn corpus_workspace(root: &std::path::Path) -> Result<Workspace, String> {
    let tests = root.join("corpus/tests");
    let corpus_deps = crate::deps::corpus_source_dirs(&tests.join("al-language"))?;
    let mut apps = Vec::new();
    for dir in &corpus_deps {
        apps.push(crate::runtime::load_app(dir)?);
    }
    for name in CORPUS_TEST_APPS {
        apps.push(crate::runtime::load_app(&tests.join(name))?);
    }
    let mut kinds = vec![al_sema::AppKind::Workspace; apps.len()];
    let dependencies = crate::deps::load(&tests.join("al-language"), &corpus_deps, &[])?;
    kinds.extend(dependencies.apps.iter().map(|_| al_sema::AppKind::Dependency));
    apps.extend(dependencies.apps);
    Ok(Workspace::load_apps(
        apps,
        kinds,
        &al_version::corpus_symbols(al_version::MIN_BC_MAJOR),
    ))
}

/// The map of the pinned corpus under `root` (the repository root): the corpus's test apps
/// and the fixture, with the dependency apps of the corpus's `.alpackages` so that calls on Microsoft's
/// types resolve, checked with the preprocessor symbol of BC 27 and the whole catalog.
pub fn corpus_coverage(root: &std::path::Path, corpus_commit: &str) -> Result<Coverage, String> {
    let ws = corpus_workspace(root)?;
    let members = al_sema::members::build(&ws);
    let catalog = Catalog::load();
    let checked = al_sema::check::check(&ws, &members, &catalog);
    let mut calls = collect_calls(&ws, &checked, &catalog);
    // A must-not-compile case pins the calls its expected diagnostics name (`mnc::pinned_calls`).
    let mnc = root.join("corpus/tests/must-not-compile");
    if mnc.is_dir() {
        for case in crate::mnc::read_cases(&mnc)? {
            calls.extend(crate::mnc::pinned_calls(&case, &catalog));
        }
        calls.sort();
        calls.dedup();
    }
    let mut map = build(&catalog, &calls, corpus_commit);
    let lang = LanguageCatalog::load(root)?;
    map.language = Some(language_view(
        &lang,
        &catalog,
        &calls,
        &microsoft_calls(&ws, &checked, &catalog),
        |id| al_builtins::native::lookup(id).is_some(),
    ));
    Ok(map)
}

/// One method overload of the language catalog (what the AL language server offers).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LangRow {
    /// The type as the runtime catalog names it.
    pub owner: String,
    pub name: String,
    /// The parameter types, `(Text, [Integer])`.
    pub sig: String,
    pub bc: Vec<u32>,
    /// `OnPrem` when not available in the cloud.
    pub scope: String,
    /// The runtime catalog overload it matches.
    pub id: Option<String>,
    pub site: Option<(String, usize)>,
    pub state: State,
    pub ms_uses: usize,
}

/// The builtin map against the language catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageView {
    /// `27-29`.
    pub majors: String,
    pub rows: Vec<LangRow>,
    /// Runtime catalog overloads no catalog overload matches (documented only, or spelled differently).
    pub runtime_only: Vec<String>,
}

/// Every catalog overload with its runtime overload, pinned site, state and Microsoft use.
/// `implemented`: the runtime registers an implementation for the overload id.
pub fn language_view(
    lang: &LanguageCatalog,
    catalog: &Catalog,
    calls: &[Call],
    ms_calls: &BTreeMap<String, usize>,
    implemented: impl Fn(&str) -> bool,
) -> LanguageView {
    let mut first: BTreeMap<&str, (&str, usize)> = BTreeMap::new();
    for c in calls {
        let site = (c.file.as_str(), c.line);
        let e = first.entry(c.id.as_str()).or_insert(site);
        if site < *e {
            *e = site;
        }
    }
    let m = crate::language_catalog::match_methods(lang, catalog);
    let mut rows: Vec<LangRow> = lang
        .area("methods")
        .zip(m.matched)
        .map(|(e, hit)| {
            let id = hit.map(|(id, _)| id);
            let state = match &id {
                Some(id) if implemented(id) => State::Implemented,
                Some(_) => State::Refused,
                None => State::Unknown,
            };
            LangRow {
                owner: runtime_receiver(&e.owner).to_string(),
                name: e.name.clone(),
                sig: e.sig.clone(),
                bc: e.bc.clone(),
                scope: e.scope.clone(),
                site: id
                    .as_deref()
                    .and_then(|i| first.get(i))
                    .map(|(f, l)| (f.to_string(), *l)),
                ms_uses: id.as_deref().and_then(|i| ms_calls.get(i)).copied().unwrap_or(0),
                id,
                state,
            }
        })
        .collect();
    rows.sort_by(|a, b| {
        (a.owner.as_str(), a.name.to_ascii_lowercase(), a.sig.as_str()).cmp(&(
            b.owner.as_str(),
            b.name.to_ascii_lowercase(),
            b.sig.as_str(),
        ))
    });
    LanguageView {
        majors: lang.majors(),
        rows,
        runtime_only: m.runtime_only,
    }
}

/// Counts of one group of rows: all, pinned, implemented, refused, unknown, Microsoft-used and unpinned.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    pub all: usize,
    pub pinned: usize,
    pub implemented: usize,
    pub refused: usize,
    pub unknown: usize,
    pub ms_unpinned: usize,
}

impl Counts {
    pub fn of<'a>(rows: impl IntoIterator<Item = &'a LangRow>) -> Counts {
        let mut c = Counts::default();
        for r in rows {
            c.add(r.site.is_some(), r.state, r.ms_uses > 0);
        }
        c
    }

    pub fn add(&mut self, pinned: bool, state: State, ms_used: bool) {
        self.all += 1;
        self.pinned += usize::from(pinned);
        match state {
            State::Implemented => self.implemented += 1,
            State::Refused => self.refused += 1,
            State::Unknown => self.unknown += 1,
        }
        self.ms_unpinned += usize::from(ms_used && !pinned);
    }
}

/// `12.3%` of `all`; `-` for an empty group.
pub fn pct(n: usize, all: usize) -> String {
    if all == 0 {
        "-".into()
    } else {
        format!("{:.1}%", 100.0 * n as f64 / all as f64)
    }
}

fn language_markdown(v: &LanguageView, s: &mut String) {
    let total = Counts::of(&v.rows);
    s.push_str(&format!(
        "## Language catalog (AL language server, BC {})\n\n",
        v.majors
    ));
    s.push_str("The catalog is every method overload the AL language server offers (`docs/language-catalog.json`, generated by `tools/language-catalog/generate.sh` from the completion and signature help of the language server of each BC major). **Pinned**: a corpus call is to the runtime overload the catalog overload matches (the overload its argument types select, or one a must-not-compile case names; see the runtime catalog section). **Implemented**: the runtime registers an implementation for that overload. **Refused**: the runtime catalog has the overload but no implementation, so lowering refuses a call to it. **Unknown**: the runtime catalog has no such overload, so the checker cannot resolve a call to it. **MS uses**: calls in Microsoft's apps that the corpus loads.\n\n");
    s.push_str(&format!(
        "**Total: {} overloads; pinned {} ({}); implemented {} ({}); refused {}; unknown {}; used by Microsoft and unpinned {}.**\n\n",
        total.all,
        total.pinned,
        pct(total.pinned, total.all),
        total.implemented,
        pct(total.implemented, total.all),
        total.refused,
        total.unknown,
        total.ms_unpinned
    ));
    s.push_str("| Type | Overloads | Pinned | Pinned % | Implemented | Implemented % | Refused | Unknown | MS-used unpinned |\n|---|---:|---:|---:|---:|---:|---:|---:|---:|\n");
    let mut by_type: BTreeMap<&str, Vec<&LangRow>> = BTreeMap::new();
    for r in &v.rows {
        by_type.entry(r.owner.as_str()).or_default().push(r);
    }
    for (t, rows) in &by_type {
        let c = Counts::of(rows.iter().copied());
        s.push_str(&format!(
            "| {t} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            c.all,
            c.pinned,
            pct(c.pinned, c.all),
            c.implemented,
            pct(c.implemented, c.all),
            c.refused,
            c.unknown,
            c.ms_unpinned
        ));
    }
    s.push_str("\n### Unpinned overloads by type\n\nSorted by type, then name. Columns: state, MS uses, the BC majors that have it when not all, `OnPrem` when not available in the cloud.\n");
    for (t, rows) in &by_type {
        let gap: Vec<&&LangRow> = rows.iter().filter(|r| r.site.is_none()).collect();
        if gap.is_empty() {
            continue;
        }
        s.push_str(&format!("\n#### {t} ({} unpinned of {})\n\n", gap.len(), rows.len()));
        for r in gap {
            let mut extra = String::new();
            if majors(&r.bc) != v.majors {
                extra.push_str(&format!(", BC {}", majors(&r.bc)));
            }
            if !r.scope.is_empty() {
                extra.push_str(&format!(", {}", r.scope));
            }
            s.push_str(&format!(
                "- `{t}.{}{}`: {}, MS uses {}{extra}{}\n",
                r.name,
                r.sig,
                r.state.label(),
                r.ms_uses,
                r.id.as_ref().map(|i| format!(" (`{i}`)")).unwrap_or_default()
            ));
        }
    }
    if !v.runtime_only.is_empty() {
        s.push_str("\n### In the runtime catalog, not matched to a catalog overload\n\n");
        for id in &v.runtime_only {
            s.push_str(&format!("- `{id}`\n"));
        }
    }
    s.push('\n');
}

/// One overload of the catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Overload {
    pub id: String,
    /// The catalog line, as `signatures.txt` has it.
    pub signature: String,
    /// One call in the corpus that the checker resolved to this overload.
    pub site: Option<(String, usize)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeCoverage {
    pub name: String,
    pub overloads: Vec<Overload>,
}

impl TypeCoverage {
    pub fn pinned(&self) -> usize {
        self.overloads.iter().filter(|o| o.site.is_some()).count()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Coverage {
    pub corpus_commit: String,
    pub types: Vec<TypeCoverage>,
    /// The map against the AL language catalog (`docs/language-catalog.json`).
    pub language: Option<LanguageView>,
}

impl Coverage {
    pub fn total(&self) -> (usize, usize) {
        (
            self.types.iter().map(TypeCoverage::pinned).sum(),
            self.types.iter().map(|t| t.overloads.len()).sum(),
        )
    }
}

/// The map of `catalog` against `calls`. Types and overloads are in catalog order of name and id.
pub fn build(catalog: &Catalog, calls: &[Call], corpus_commit: &str) -> Coverage {
    let mut first: BTreeMap<&str, (&str, usize)> = BTreeMap::new();
    for c in calls {
        let site = (c.file.as_str(), c.line);
        let e = first.entry(c.id.as_str()).or_insert(site);
        if site < *e {
            *e = site;
        }
    }
    let mut by_type: BTreeMap<String, Vec<Overload>> = BTreeMap::new();
    for sig in &catalog.signatures {
        let id = catalog.id(sig);
        let site = first.get(id.as_str()).map(|(f, l)| (f.to_string(), *l));
        by_type.entry(sig.receiver.clone()).or_default().push(Overload {
            signature: sig.to_line(),
            id,
            site,
        });
    }
    Coverage {
        corpus_commit: corpus_commit.to_string(),
        types: by_type
            .into_iter()
            .map(|(name, overloads)| TypeCoverage { name, overloads })
            .collect(),
        language: None,
    }
}

/// The map as Markdown: a summary table, then the overloads no corpus test calls, per type.
pub fn to_markdown(c: &Coverage) -> String {
    let (pinned, all) = c.total();
    let mut s = String::new();
    s.push_str("# Corpus coverage of the builtin catalog\n\n");
    s.push_str("Generated by `al-test-cli --coverage-map` (the test `corpus_coverage_map_is_current` checks it). Do not edit.\n\n");
    s.push_str(&format!(
        "Corpus commit `{}`. An overload is pinned when at least one call of the corpus's own files is to exactly that overload: the overload the argument types select (the most specific one, as BC's compiler binds it; `coverage::selected_overload`), or a call a must-not-compile case names in an expected diagnostic (`mnc::pinned_calls`).\n\n",
        c.corpus_commit
    ));
    if let Some(v) = &c.language {
        language_markdown(v, &mut s);
        s.push_str("## Runtime catalog (ADR-008 signatures)\n\nThe overloads of the runtime's own catalog (`crates/al-builtins/catalog/signatures.txt`).\n\n");
    }
    s.push_str(&format!("**Total: pinned {pinned} of {all} overloads.**\n\n"));
    s.push_str("| Type | Pinned | Overloads | Unpinned |\n|---|---:|---:|---:|\n");
    for t in &c.types {
        s.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            t.name,
            t.pinned(),
            t.overloads.len(),
            t.overloads.len() - t.pinned()
        ));
    }
    s.push_str("\n## Unpinned overloads\n");
    for t in c.types.iter().filter(|t| t.pinned() < t.overloads.len()) {
        s.push_str(&format!(
            "\n### {} (pinned {} of {})\n\n",
            t.name,
            t.pinned(),
            t.overloads.len()
        ));
        for o in t.overloads.iter().filter(|o| o.site.is_none()) {
            s.push_str(&format!("- `{}` `{}`\n", o.id, o.signature));
        }
    }
    s.push_str("\n## Pinned overloads\n");
    for t in c.types.iter().filter(|t| t.pinned() > 0) {
        s.push_str(&format!("\n### {}\n\n", t.name));
        for o in t.overloads.iter().filter(|o| o.site.is_some()) {
            let (f, l) = o.site.as_ref().expect("filtered");
            s.push_str(&format!("- `{}` `{}`: {f}:{l}\n", o.id, o.signature));
        }
    }
    s
}

/// The map as JSON, for tools.
pub fn to_json(c: &Coverage) -> String {
    let (pinned, all) = c.total();
    let types: Vec<serde_json::Value> = c
        .types
        .iter()
        .map(|t| {
            serde_json::json!({
                "type": t.name,
                "pinned": t.pinned(),
                "overloads": t.overloads.len(),
                "methods": t.overloads.iter().map(|o| serde_json::json!({
                    "id": o.id,
                    "signature": o.signature,
                    "pinned": o.site.is_some(),
                    "site": o.site.as_ref().map(|(f, l)| format!("{f}:{l}")),
                })).collect::<Vec<_>>(),
            })
        })
        .collect();
    let language = c.language.as_ref().map(|v| {
        let t = Counts::of(&v.rows);
        serde_json::json!({
            "bc": v.majors,
            "overloads": t.all,
            "pinned": t.pinned,
            "implemented": t.implemented,
            "refused": t.refused,
            "unknown": t.unknown,
            "methods": v.rows.iter().map(|r| serde_json::json!({
                "type": r.owner,
                "name": r.name,
                "sig": r.sig,
                "bc": r.bc,
                "scope": r.scope,
                "id": r.id,
                "pinned": r.site.is_some(),
                "site": r.site.as_ref().map(|(f, l)| format!("{f}:{l}")),
                "state": r.state.label(),
                "ms_uses": r.ms_uses,
            })).collect::<Vec<_>>(),
            "runtime_only": v.runtime_only,
        })
    });
    let mut text = serde_json::to_string_pretty(&serde_json::json!({
        "corpus_commit": c.corpus_commit,
        "pinned": pinned,
        "overloads": all,
        "types": types,
        "language": language,
    }))
    .expect("json");
    text.push('\n');
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use al_sema::workspace::AppSource;

    /// A fixture "corpus" of one file: calls on a Text, a Label-free global and a List.
    const FIXTURE: &str = "codeunit 50100 C
{
    procedure P()
    var
        T: Text;
        L: List of [Text];
    begin
        if T.Contains('x') then
            T.Split(',');
        L.Add('a');
        Message('hi');
        T.Contains('y');
    end;
}
";

    fn map() -> Coverage {
        let ws = Workspace::load(
            vec![AppSource {
                id: "a".into(),
                name: "a".into(),
                dependencies: vec![],
                files: vec![("fixture.al".into(), FIXTURE.into())],
            }],
            &Default::default(),
        );
        let members = al_sema::members::build(&ws);
        let catalog = Catalog::load();
        let checked = al_sema::check::check(&ws, &members, &catalog);
        let calls = collect_calls(&ws, &checked, &catalog);
        build(&catalog, &calls, "abc123")
    }

    fn find<'a>(c: &'a Coverage, ty: &str, name: &str) -> Vec<&'a Overload> {
        c.types
            .iter()
            .filter(|t| t.name == ty)
            .flat_map(|t| &t.overloads)
            .filter(|o| o.signature.starts_with(&format!("{ty}.{name} ")))
            .collect()
    }

    #[test]
    fn an_overload_is_pinned_when_a_call_resolves_to_it_and_names_the_first_site() {
        let c = map();
        let contains = find(&c, "Text", "Contains");
        assert_eq!(contains.len(), 1);
        // Lines 8 and 12 call it; the first one is named.
        assert_eq!(contains[0].site, Some(("fixture.al".to_string(), 8)));
        let split = find(&c, "Text", "Split");
        assert!(split.len() >= 2, "Split has several overloads");
        assert_eq!(
            split.iter().filter(|o| o.site.is_some()).count(),
            1,
            "only the called one"
        );
        assert_eq!(find(&c, "List", "Add")[0].site, Some(("fixture.al".to_string(), 10)));
        assert!(find(&c, "Dialog", "Message").iter().any(|o| o.site.is_some()));
        assert!(find(&c, "Text", "StartsWith")[0].site.is_none());
    }

    #[test]
    fn the_totals_count_pinned_against_all_overloads_and_the_output_names_both() {
        let c = map();
        let (pinned, all) = c.total();
        assert!(pinned >= 4 && pinned < all, "{pinned} of {all}");
        let md = to_markdown(&c);
        assert!(md.contains("Corpus commit `abc123`"));
        assert!(md.contains(&format!("**Total: pinned {pinned} of {all} overloads.**")));
        assert!(md.contains("| Text | "), "{}", &md[..400]);
        assert!(md.contains("### Text (pinned"), "unpinned section per type");
        assert!(md.contains("fixture.al:8"), "a pinned overload names its call");
        let json: serde_json::Value = serde_json::from_str(&to_json(&c)).unwrap();
        assert_eq!(json["pinned"], pinned);
        assert_eq!(json["overloads"], all);
        assert_eq!(json["corpus_commit"], "abc123");
    }

    #[test]
    fn the_language_view_names_pinned_implemented_refused_and_unknown_per_catalog_overload() {
        let lang = LanguageCatalog::parse(
            r#"{"sources":[{"bc":27,"al_extension":"a"},{"bc":28,"al_extension":"b"}],"items":[
{"area":"methods","owner":"Text","name":"Contains","sig":"(Text)","bc":[27,28]},
{"area":"methods","owner":"Text","name":"StartsWith","sig":"(Text)","bc":[28]},
{"area":"methods","owner":"Text","name":"NoSuchMethod","sig":"()","scope":"OnPrem","bc":[27,28]}
]}"#,
        )
        .unwrap();
        let c = map();
        let catalog = Catalog::load();
        let ws_calls = vec![Call {
            id: c
                .types
                .iter()
                .flat_map(|t| &t.overloads)
                .find(|o| o.signature.starts_with("Text.Contains "))
                .unwrap()
                .id
                .clone(),
            file: "fixture.al".into(),
            line: 8,
        }];
        let ms = BTreeMap::from([(ws_calls[0].id.clone(), 7usize)]);
        let v = language_view(&lang, &catalog, &ws_calls, &ms, |id| id.starts_with("Text.Contains"));
        assert_eq!(v.majors, "27-28");
        let row = |n: &str| v.rows.iter().find(|r| r.name == n).unwrap();
        assert_eq!(row("Contains").site, Some(("fixture.al".into(), 8)));
        assert_eq!(row("Contains").state, State::Implemented);
        assert_eq!(row("Contains").ms_uses, 7);
        assert_eq!(
            row("StartsWith").state,
            State::Refused,
            "in the runtime catalog, no implementation"
        );
        assert_eq!(row("NoSuchMethod").state, State::Unknown);
        let mut with = c.clone();
        with.language = Some(v);
        let md = to_markdown(&with);
        assert!(
            md.contains("**Total: 3 overloads; pinned 1 (33.3%); implemented 1 (33.3%); refused 1; unknown 1"),
            "{md}"
        );
        assert!(md.contains("| Text | 3 | 1 | 33.3% | 1 | 33.3% | 1 | 1 | 0 |"), "{md}");
        assert!(
            md.contains("- `Text.StartsWith(Text)`: refused, MS uses 0, BC 28"),
            "{md}"
        );
        assert!(
            md.contains("- `Text.NoSuchMethod()`: unknown, MS uses 0, OnPrem"),
            "{md}"
        );
        let json: serde_json::Value = serde_json::from_str(&to_json(&with)).unwrap();
        assert_eq!(json["language"]["unknown"], 1);
    }

    /// A call counts for the overload its argument types select, as BC's compiler picks the most
    /// specific one: `FRef.TestField(5)` is `TestField(Integer)`, not `TestField(Any)`, although
    /// the checker's tie-break keeps the first overload that fits.
    #[test]
    fn a_call_counts_for_the_overload_its_argument_types_select() {
        let src = "codeunit 60001 T
{
    procedure P()
    var
        RecRef: RecordRef;
        FRef: FieldRef;
        OStr: OutStream;
        I: Integer;
        V: Variant;
        S: SecretText;
        A: array[3] of Integer;
        H: HttpHeaders;
        Vals: array[2] of Text;
    begin
        FRef.TestField(I);
        OStr.Write(true);
        OStr.Write(V);
        Clear(S);
        Clear(A);
        Clear(I);
        H.GetValues('k', Vals);
    end;
}
";
        let ws = Workspace::load(
            vec![AppSource {
                id: "a".into(),
                name: "a".into(),
                dependencies: vec![],
                files: vec![("typed.al".into(), src.into())],
            }],
            &Default::default(),
        );
        let members = al_sema::members::build(&ws);
        let catalog = Catalog::load();
        let checked = al_sema::check::check(&ws, &members, &catalog);
        let calls = collect_calls(&ws, &checked, &catalog);
        let sig_at = |line: usize| -> Vec<String> {
            calls
                .iter()
                .filter(|c| c.line == line)
                .map(|c| catalog.by_id(&c.id).unwrap().to_line())
                .collect()
        };
        assert_eq!(sig_at(15), ["FieldRef.TestField inst (Value: Integer)"]);
        assert_eq!(
            sig_at(16)[0].split(" -> ").next().unwrap(),
            "OutStream.Write inst (Value: Boolean, ?Length: Integer)"
        );
        assert_eq!(
            sig_at(17)[0].split(" -> ").next().unwrap(),
            "OutStream.Write inst (Value: Variant, ?Length: Integer)"
        );
        assert_eq!(sig_at(18), ["System.Clear static (var Variable: SecretText)"]);
        assert_eq!(sig_at(19), ["System.Clear static (var Variable: Array of [Any])"]);
        // No overload takes an Integer: the Any one stays.
        assert_eq!(sig_at(20), ["System.Clear static (var Variable: Any)"]);
        // `String` and `Text` are one type: the two overloads tie, the checker's choice stays.
        assert_eq!(
            sig_at(21)[0].split(" -> ").next().unwrap(),
            "HttpHeaders.GetValues inst (Key: String, Values: Array of [Text])"
        );
    }

    #[test]
    fn calls_in_a_dependency_app_do_not_count() {
        let ws = Workspace::load_apps(
            vec![
                AppSource {
                    id: "d".into(),
                    name: "d".into(),
                    dependencies: vec![],
                    files: vec![("dep.al".into(), FIXTURE.into())],
                },
                AppSource {
                    id: "w".into(),
                    name: "w".into(),
                    dependencies: vec!["d".into()],
                    files: vec![("own.al".into(), "codeunit 60000 W { procedure Q() begin end; }".into())],
                },
            ],
            vec![al_sema::AppKind::Dependency, al_sema::AppKind::Workspace],
            &Default::default(),
        );
        let members = al_sema::members::build(&ws);
        let checked = al_sema::check::check(&ws, &members, &Catalog::load());
        assert!(
            collect_calls(&ws, &checked, &Catalog::load())
                .iter()
                .all(|c| c.file != "dep.al")
        );
    }
}
