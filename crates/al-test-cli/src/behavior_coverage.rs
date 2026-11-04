//! The behavior coverage map: which triggers, properties, platform events, statement and
//! expression kinds, and runtime error sites some corpus test pins (`--behavior-coverage-map`,
//! `docs/corpus-behavior-coverage.md`; plan: `docs/reviews/2026-10-behavior-coverage-plan.md`).
//!
//! The catalog of each dimension is the union of (a) what the runtime knows and (b) what
//! Microsoft's own apps use (the dependency sources the corpus loads), so the gap list shows what
//! real code needs even where the runtime reads nothing yet. An item is *pinned* when it occurs in
//! a Workspace (corpus) file in a routine that static reach finds, or at object level for
//! properties. Reach is static: roots are triggers, `[Test]` procedures, event subscribers and
//! handlers of Workspace objects; edges are the procedure calls the checker resolved. Nothing runs.
//! "Reached" does not mean "observed": a branch a test never takes counts.

use al_sema::check::{CheckResult, Res};
use al_sema::{AppKind, Workspace};
use al_syntax::{SyntaxKind, SyntaxNode, ast};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Dim {
    Trigger,
    Property,
    Event,
    Syntax,
    Attribute,
}

impl Dim {
    pub const ALL: [Dim; 5] = [Dim::Trigger, Dim::Property, Dim::Event, Dim::Syntax, Dim::Attribute];
    pub fn title(self) -> &'static str {
        match self {
            Dim::Trigger => "Triggers per object kind",
            Dim::Property => "Object properties per object kind",
            Dim::Event => "Platform and system events",
            Dim::Syntax => "Statements and expressions",
            Dim::Attribute => "Method attributes",
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Dim::Trigger => "triggers",
            Dim::Property => "properties",
            Dim::Event => "events",
            Dim::Syntax => "syntax",
            Dim::Attribute => "attributes",
        }
    }
}

/// One place an item occurs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Occ {
    pub dim: Dim,
    pub group: String,
    pub name: String,
    /// In a Microsoft dependency app (not the corpus's own fixture).
    pub microsoft: bool,
    /// In a Workspace (corpus) file.
    pub corpus: bool,
    /// Corpus occurrence in a routine static reach finds (or at object level: always).
    pub reached: bool,
    pub file: String,
    pub line: usize,
}

/// What the runtime source mentions.
#[derive(Debug, Clone, Default)]
pub struct Known {
    /// Lower-case string literals of the runtime crates (non-test code).
    pub strings: HashSet<String>,
    /// Original-case literals that look like a trigger or event name (`OnX...`).
    pub on_names: BTreeSet<String>,
    /// Names after `SyntaxKind::` in the runtime crates.
    pub syntax_kinds: HashSet<String>,
}

/// The compound assignment operators and their token kinds.
const COMPOUND_ASSIGN: [(&str, &str); 4] = [
    ("+=", "PlusAssign"),
    ("-=", "MinusAssign"),
    ("*=", "StarAssign"),
    ("/=", "SlashAssign"),
];

impl Known {
    pub fn knows(&self, dim: Dim, name: &str) -> bool {
        let n = name.to_ascii_lowercase();
        match dim {
            Dim::Syntax if name.starts_with("AssignStmt ") => COMPOUND_ASSIGN
                .iter()
                .any(|(op, tok)| name.ends_with(op) && self.syntax_kinds.contains(*tok)),
            Dim::Syntax if name == "NameRef this" => self.strings.contains("this"),
            Dim::Syntax => {
                self.syntax_kinds.contains(name)
                    || n.strip_prefix("literal:")
                        .is_some_and(|k| self.syntax_kinds.contains(k))
                    || n.split(':').next_back().is_some_and(|op| self.strings.contains(op))
                    || n.rsplit(' ').next().is_some_and(|op| self.strings.contains(op))
            }
            _ => self.strings.contains(&n),
        }
    }
}

/// One item of a catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub dim: Dim,
    pub group: String,
    pub name: String,
    /// Occurrences in Microsoft's apps.
    pub ms_uses: usize,
    /// Occurrences in corpus files (reached or not).
    pub corpus_uses: usize,
    /// The first reached corpus occurrence.
    pub site: Option<(String, usize)>,
    pub runtime_reads: bool,
}

/// One `raise` site of the runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorSite {
    pub file: String,
    pub line: usize,
    /// The message with every `{..}` placeholder as `%`.
    pub template: String,
    pub code: String,
    /// A corpus string literal holds the message text.
    pub site: Option<(String, usize)>,
    /// Too little fixed text to match (all pieces shorter than 8 characters).
    pub generic: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BehaviorCoverage {
    pub corpus_commit: String,
    pub items: Vec<Item>,
    pub errors: Vec<ErrorSite>,
    /// The map against the AL language catalog (`docs/language-catalog.json`).
    pub language: Option<LanguageView>,
}

// ---------------------------------------------------------------------------------------------
// Rust source scanning (runtime knowledge and error sites)

/// A string literal of Rust source: where it starts and its text (escapes decoded).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lit {
    pub start: usize,
    pub text: String,
}

/// The string literals of Rust source outside comments. Raw strings are read; char literals skipped.
pub fn rust_strings(src: &str) -> Vec<Lit> {
    let b = src.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'/' if b.get(i + 1) == Some(&b'/') => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if b.get(i + 1) == Some(&b'*') => {
                i += 2;
                while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                    i += 1;
                }
                i += 2;
            }
            b'\'' => {
                // A char literal (`'"'`, `'\''`) or a lifetime. Skip a quoted char; leave lifetimes.
                if b.get(i + 1) == Some(&b'\\') {
                    i += 2;
                    while i < b.len() && b[i] != b'\'' {
                        i += 1;
                    }
                    i += 1;
                } else if b.get(i + 2) == Some(&b'\'') {
                    i += 3;
                } else {
                    i += 1;
                }
            }
            b'r' if matches!(b.get(i + 1), Some(b'"') | Some(b'#'))
                && (i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_')) =>
            {
                let mut j = i + 1;
                let mut hashes = 0;
                while b.get(j) == Some(&b'#') {
                    hashes += 1;
                    j += 1;
                }
                if b.get(j) != Some(&b'"') {
                    i += 1;
                    continue;
                }
                let start = i;
                j += 1;
                let body = j;
                let close = format!("\"{}", "#".repeat(hashes));
                let end = src[body..].find(&close).map_or(b.len(), |e| body + e);
                out.push(Lit {
                    start,
                    text: src[body..end].to_string(),
                });
                i = end + close.len();
            }
            b'"' => {
                let start = i;
                i += 1;
                let mut text = String::new();
                while i < b.len() && b[i] != b'"' {
                    if b[i] == b'\\' {
                        i += 1;
                        match b.get(i) {
                            Some(b'n') => text.push('\n'),
                            Some(b't') => text.push('\t'),
                            Some(b'r') => text.push('\r'),
                            Some(b'\n') => {
                                while b.get(i + 1).is_some_and(|c| c.is_ascii_whitespace()) {
                                    i += 1;
                                }
                            }
                            Some(b'u') => {
                                let close = src[i..].find('}').map_or(i, |e| i + e);
                                let hex: String = src[i..close].chars().filter(char::is_ascii_hexdigit).collect();
                                if let Some(c) = u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                                    text.push(c);
                                }
                                i = close;
                            }
                            Some(&c) => text.push(c as char),
                            None => {}
                        }
                        i += 1;
                    } else {
                        let ch = src[i..].chars().next().expect("in bounds");
                        text.push(ch);
                        i += ch.len_utf8();
                    }
                }
                i += 1;
                out.push(Lit { start, text });
            }
            _ => i += 1,
        }
    }
    out
}

/// A name that looks like a trigger or event: `On` and an upper-case letter, then letters and digits.
fn looks_like_on_name(s: &str) -> bool {
    s.len() >= 5
        && s.len() <= 48
        && s.starts_with("On")
        && s.as_bytes()[2].is_ascii_uppercase()
        && s.bytes().all(|c| c.is_ascii_alphanumeric())
}

/// The non-test Rust files under `dir`: no `tests` folder, no `tests.rs`, cut at `#[cfg(test)]`.
fn runtime_sources(dir: &Path) -> Vec<(String, String)> {
    fn walk(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        let mut entries: Vec<_> = rd.filter_map(Result::ok).map(|e| e.path()).collect();
        entries.sort();
        for p in entries {
            if p.is_dir() {
                if p.file_name().is_some_and(|n| n != "tests") {
                    walk(&p, out);
                }
            } else if p.extension().is_some_and(|e| e == "rs") && p.file_name().is_some_and(|n| n != "tests.rs") {
                out.push(p);
            }
        }
    }
    let mut files = Vec::new();
    walk(dir, &mut files);
    files
        .into_iter()
        .filter_map(|p| {
            let text = std::fs::read_to_string(&p).ok()?;
            let cut = text.find("#[cfg(test)]").unwrap_or(text.len());
            Some((p.to_string_lossy().into_owned(), text[..cut].to_string()))
        })
        .collect()
}

/// The crates whose source says what the runtime reads.
const RUNTIME_CRATES: [&str; 6] = ["al-sema", "al-vm", "al-ir", "al-builtins", "al-data", "al-host"];
/// The crates whose `raise` sites are runtime errors.
const ERROR_CRATES: [&str; 4] = ["al-builtins", "al-vm", "al-data", "al-host"];

/// The markers around a list the front end only checks names against (an AL0124 list of allowed
/// properties, an AL0162 list of triggers): the names in it say nothing about what the runtime reads.
const VALIDATION_ONLY_BEGIN: &str = "// behavior-coverage: validation-only begin";
const VALIDATION_ONLY_END: &str = "// behavior-coverage: validation-only end";

/// `text` with every validation-only region blanked (a region without an end runs to the end of the file).
fn without_validation_only(text: &str) -> std::borrow::Cow<'_, str> {
    if !text.contains(VALIDATION_ONLY_BEGIN) {
        return std::borrow::Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(b) = rest.find(VALIDATION_ONLY_BEGIN) {
        out.push_str(&rest[..b]);
        let region = &rest[b..];
        let e = region
            .find(VALIDATION_ONLY_END)
            .map_or(region.len(), |e| e + VALIDATION_ONLY_END.len());
        out.extend(region[..e].chars().map(|c| if c == '\n' { '\n' } else { ' ' }));
        rest = &region[e..];
    }
    out.push_str(rest);
    std::borrow::Cow::Owned(out)
}

pub fn known_from_sources(sources: &[(String, String)]) -> Known {
    let mut k = Known::default();
    for (_, text) in sources {
        let text = &*without_validation_only(text);
        for lit in rust_strings(text) {
            if looks_like_on_name(&lit.text) {
                k.on_names.insert(lit.text.clone());
            }
            k.strings.insert(lit.text.to_ascii_lowercase());
        }
        // The lowering matches grammar kinds after `use SyntaxKind::*` (`IfStmt =>`): a whole-word
        // use of a grammar kind or compound-assignment token name counts too.
        for name in STATEMENT_KINDS
            .iter()
            .chain(&MORE_STATEMENT_KINDS)
            .chain(&EXPRESSION_KINDS)
            .map(|k| kind_name(*k))
            .chain(COMPOUND_ASSIGN.iter().map(|(_, t)| t.to_string()))
        {
            if text.match_indices(name.as_str()).any(|(i, _)| {
                let before = text[..i].chars().next_back();
                let after = text[i + name.len()..].chars().next();
                !before.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
                    && !after.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
            }) {
                k.syntax_kinds.insert(name);
            }
        }
        for (i, _) in text.match_indices("SyntaxKind::") {
            let name: String = text[i + 12..]
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                k.syntax_kinds.insert(name);
            }
        }
    }
    k
}

/// A message template of a `raise` site: placeholders `{..}` become `%`, `{{` and `}}` one brace.
fn template(lit: &str) -> String {
    let mut out = String::new();
    let mut it = lit.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '{' if it.peek() == Some(&'{') => {
                it.next();
                out.push('{');
            }
            '{' => {
                for d in it.by_ref() {
                    if d == '}' {
                        break;
                    }
                }
                out.push('%');
            }
            '}' if it.peek() == Some(&'}') => {
                it.next();
                out.push('}');
            }
            _ => out.push(c),
        }
    }
    out
}

fn line_of(text: &str, offset: usize) -> usize {
    text[..offset.min(text.len())].bytes().filter(|&c| c == b'\n').count() + 1
}

/// The `raise(message, code)` and `AlError { message, code }` sites of one file with a literal message.
pub fn error_sites(file: &str, text: &str) -> Vec<(usize, String, String)> {
    let lits = rust_strings(text);
    let mut out = Vec::new();
    let first_after = |p: usize, window: usize| -> Option<usize> {
        let k = lits.partition_point(|l| l.start < p);
        let l = lits.get(k)?;
        (l.start - p <= window && !text[p..l.start].contains(';')).then_some(k)
    };
    let mut seen = HashSet::new();
    let mut add = |k: usize, out: &mut Vec<(usize, String, String)>| {
        if !seen.insert(k) {
            return;
        }
        let code = lits
            .get(k + 1)
            .filter(|l| l.start - lits[k].start < 400 && l.text.len() <= 24 && !l.text.contains(' '))
            .map_or(String::new(), |l| l.text.clone());
        out.push((line_of(text, lits[k].start), template(&lits[k].text), code));
    };
    for (i, _) in text.match_indices("raise(") {
        let p = i + 6;
        // `raise(format!("..", ..), code)` or `raise("..", code)`.
        let mut start = p;
        if text[p..].starts_with("format!(") {
            start = p + 8;
        }
        if let Some(k) = first_after(start, 4) {
            add(k, &mut out);
        }
    }
    for (i, _) in text.match_indices("message:") {
        let p = i + 8;
        let near = text[..i].rfind("AlError").is_some_and(|a| i - a < 40);
        if !near {
            continue;
        }
        let mut start = p;
        let rest = text[p..].trim_start();
        let skipped = text[p..].len() - rest.len();
        start += skipped;
        for pre in ["format!(", "String::from(", "\""] {
            if text[start..].starts_with(pre) && pre != "\"" {
                start += pre.len();
                break;
            }
        }
        if let Some(k) = first_after(start, 4) {
            add(k, &mut out);
        }
    }
    let _ = file;
    out.sort();
    out
}

/// Whether every fixed piece of `template` occurs, in order, in `literal`.
fn template_in(template: &str, literal: &str) -> bool {
    let mut from = 0;
    for piece in template.split('%').filter(|p| !p.is_empty()) {
        match literal[from..].find(piece) {
            Some(at) => from += at + piece.len(),
            None => return false,
        }
    }
    true
}

fn is_generic(template: &str) -> bool {
    !template.split('%').any(|p| p.trim().chars().count() >= 8)
}

// ---------------------------------------------------------------------------------------------
// Syntax walk

const PRESENTATION_ONLY: [&str; 22] = [
    "caption",
    "captionml",
    "tooltip",
    "tooltipml",
    "applicationarea",
    "usagecategory",
    "description",
    "aboutTitle",
    "abouttext",
    "aboutTitleml",
    "abouttextml",
    "instructionaltext",
    "instructionaltextml",
    "image",
    "promotedcategory",
    "promoted",
    "promotedisbig",
    "promotedonly",
    "shortcutkey",
    "accessbypermission",
    "extensible",
    "dataclassification",
];

fn presentation_only(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    PRESENTATION_ONLY.iter().any(|p| p.eq_ignore_ascii_case(&n))
}

/// The expression and statement kinds the grammar has.
const STATEMENT_KINDS: [SyntaxKind; 14] = [
    SyntaxKind::Block,
    SyntaxKind::EmptyStmt,
    SyntaxKind::AssignStmt,
    SyntaxKind::ExprStmt,
    SyntaxKind::IfStmt,
    SyntaxKind::CaseStmt,
    SyntaxKind::WhileStmt,
    SyntaxKind::RepeatStmt,
    SyntaxKind::ForStmt,
    SyntaxKind::ForeachStmt,
    SyntaxKind::WithStmt,
    SyntaxKind::ExitStmt,
    SyntaxKind::BreakStmt,
    SyntaxKind::ContinueStmt,
];
const MORE_STATEMENT_KINDS: [SyntaxKind; 3] = [
    SyntaxKind::CaseBranch,
    SyntaxKind::CaseElse,
    SyntaxKind::AssertErrorStmt,
];
const EXPRESSION_KINDS: [SyntaxKind; 12] = [
    SyntaxKind::Literal,
    SyntaxKind::NameRef,
    SyntaxKind::BinExpr,
    SyntaxKind::PrefixExpr,
    SyntaxKind::MemberExpr,
    SyntaxKind::ScopeExpr,
    SyntaxKind::CallExpr,
    SyntaxKind::IndexExpr,
    SyntaxKind::ParenExpr,
    SyntaxKind::SetExpr,
    SyntaxKind::RangeExpr,
    SyntaxKind::TernaryExpr,
];

fn kind_name(k: SyntaxKind) -> String {
    format!("{k:?}")
}

fn is_stmt(k: SyntaxKind) -> bool {
    STATEMENT_KINDS.contains(&k) || MORE_STATEMENT_KINDS.contains(&k)
}

/// The first non-trivia token directly under `n`, lower case: the operator of a `BinExpr` or `PrefixExpr`.
fn operator(n: &SyntaxNode) -> Option<String> {
    n.children_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| !t.kind().is_trivia())
        .map(|t| t.text().to_ascii_lowercase())
}

/// The item of a body node: the kind, refined by operator or literal type. `None`: not a statement or expression.
fn syntax_item(n: &SyntaxNode) -> Option<(String, String)> {
    let k = n.kind();
    if k == SyntaxKind::AssignStmt {
        // `x += 1` is a compound assignment, an item of its own.
        return Some((
            "statement".into(),
            match operator(n) {
                Some(op) if op != ":=" => format!("AssignStmt {op}"),
                _ => kind_name(k),
            },
        ));
    }
    if is_stmt(k) {
        return Some(("statement".into(), kind_name(k)));
    }
    if k == SyntaxKind::NameRef && n.text().to_string().trim().eq_ignore_ascii_case("this") {
        return Some(("expression".into(), "NameRef this".into()));
    }
    if !EXPRESSION_KINDS.contains(&k) {
        return None;
    }
    let name = match k {
        SyntaxKind::BinExpr => match operator(n) {
            Some(op) => format!("BinExpr {op}"),
            None => "BinExpr".into(),
        },
        SyntaxKind::PrefixExpr => match operator(n) {
            Some(op) => format!("PrefixExpr {op}"),
            None => "PrefixExpr".into(),
        },
        SyntaxKind::Literal => match n
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| !t.kind().is_trivia())
        {
            Some(t) if t.kind() == SyntaxKind::Ident => format!("Literal:{}", t.text().to_ascii_lowercase()),
            Some(t) => format!("Literal:{:?}", t.kind()),
            None => "Literal".into(),
        },
        _ => kind_name(k),
    };
    Some(("expression".into(), name))
}

/// A routine of a Workspace object, for reach.
#[derive(Debug, Clone)]
struct RoutineInfo {
    start: usize,
    end: usize,
    root: bool,
}

/// Reach over the Workspace routines: per file the routines by offset, the reached flags.
struct Reach {
    /// Per file index: routines sorted by start.
    by_file: HashMap<usize, Vec<usize>>,
    routines: Vec<RoutineInfo>,
    reached: Vec<bool>,
}

impl Reach {
    fn containing(&self, file: usize, offset: usize) -> Option<usize> {
        let list = self.by_file.get(&file)?;
        let k = list.partition_point(|&r| self.routines[r].start <= offset);
        let r = *list.get(k.checked_sub(1)?)?;
        (offset < self.routines[r].end).then_some(r)
    }
    fn reached_at(&self, file: usize, offset: usize) -> bool {
        self.containing(file, offset).is_some_and(|r| self.reached[r])
    }
}

fn is_root(r: &ast::Routine) -> bool {
    r.is_trigger()
        || r.attributes().any(|a| {
            let n = a.name().to_ascii_lowercase();
            n == "test" || n == "eventsubscriber" || n.ends_with("handler")
        })
}

fn build_reach(ws: &Workspace, checked: &CheckResult) -> Reach {
    let mut routines = Vec::new();
    let mut by_file: HashMap<usize, Vec<usize>> = HashMap::new();
    let mut ids: HashMap<(usize, usize), usize> = HashMap::new();
    for (o, obj) in ws.objects.iter().enumerate() {
        if ws.kinds[obj.app] != AppKind::Workspace {
            continue;
        }
        let node = ws.object_ast(o);
        for r in ast::routines_in(node.syntax()) {
            let range = r.syntax().text_range();
            let (start, end) = (usize::from(range.start()), usize::from(range.end()));
            let id = routines.len();
            routines.push(RoutineInfo {
                start,
                end,
                root: is_root(&r),
            });
            ids.insert((o, start), id);
            by_file.entry(obj.file).or_default().push(id);
        }
    }
    for list in by_file.values_mut() {
        list.sort_by_key(|&r| routines[r].start);
    }
    let mut reach = Reach {
        by_file,
        reached: vec![false; routines.len()],
        routines,
    };
    let mut edges: Vec<Vec<usize>> = vec![Vec::new(); reach.routines.len()];
    for (key, res) in checked.resolutions.iter() {
        let (file, start, _) = &key;
        let Res::Proc { object, offset } = res else { continue };
        let Some(&to) = ids.get(&(*object, *offset)) else {
            continue;
        };
        if let Some(from) = reach.containing(*file, *start) {
            edges[from].push(to);
        }
    }
    let mut queue: VecDeque<usize> = (0..reach.routines.len()).filter(|&r| reach.routines[r].root).collect();
    for &r in &queue {
        reach.reached[r] = true;
    }
    while let Some(r) = queue.pop_front() {
        for &t in &edges[r] {
            if !reach.reached[t] {
                reach.reached[t] = true;
                queue.push_back(t);
            }
        }
    }
    reach
}

/// The apps whose use counts as "Microsoft's own code": dependency apps that are not the corpus's
/// own fixture or test apps.
fn is_microsoft(ws: &Workspace, app: usize) -> bool {
    ws.kinds[app] == AppKind::Dependency && !ws.apps[app].name.contains("AL Language")
}

fn publisher_kind(arg: &str) -> String {
    arg.rsplit("::").next().unwrap_or(arg).trim().to_ascii_lowercase()
}

/// Splits `text` at top-level commas (outside quotes and brackets).
fn split_args(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let (mut depth, mut quote, mut cur) = (0i32, None::<char>, String::new());
    for c in text.chars() {
        match (quote, c) {
            (Some(q), c) if c == q => {
                quote = None;
                cur.push(c);
            }
            (Some(_), c) => cur.push(c),
            (None, '\'' | '"') => {
                quote = Some(c);
                cur.push(c);
            }
            (None, '(' | '[') => {
                depth += 1;
                cur.push(c);
            }
            (None, ')' | ']') => {
                depth -= 1;
                cur.push(c);
            }
            (None, ',') if depth == 0 => out.push(std::mem::take(&mut cur)),
            (None, c) => cur.push(c),
        }
    }
    out.push(cur);
    out.into_iter().map(|s| s.trim().to_string()).collect()
}

/// Every occurrence of a catalog item in the files of the workspace.
pub fn collect(ws: &Workspace, checked: &CheckResult) -> Vec<Occ> {
    let reach = build_reach(ws, checked);
    // Procedure names declared outside the platform app: an event with such a name is a business
    // event of an AL object, not a platform event.
    let mut business: HashSet<String> = HashSet::new();
    let mut events: Vec<(Occ, String)> = Vec::new();
    let mut out = Vec::new();
    for (fi, file) in ws.files.iter().enumerate() {
        let corpus = ws.kinds[file.app] == AppKind::Workspace;
        let microsoft = is_microsoft(ws, file.app);
        if !corpus && !microsoft {
            continue;
        }
        let platform_app = ws.apps[file.app].name == "System";
        let text = ws.text(fi);
        let mut starts = vec![0usize];
        starts.extend(text.match_indices('\n').map(|(i, _)| i + 1));
        let line = |off: usize| starts.partition_point(|&s| s <= off);
        let root = SyntaxNode::new_root(ws.green(fi));
        for obj in root.children().filter_map(ast::Object::cast) {
            let kind = obj.kind();
            let occ = |dim, group: String, name: String, off: usize, reached: bool| Occ {
                dim,
                group,
                name,
                microsoft,
                corpus,
                reached,
                file: file.path.clone(),
                line: line(off),
            };
            // Object-level properties (and those of sections).
            for p in obj.syntax().descendants().filter(|n| n.kind() == SyntaxKind::Property) {
                // Not the properties of local routines' attributes: properties only live in the body tree.
                let section = p
                    .ancestors()
                    .skip(1)
                    .find_map(ast::Section::cast)
                    .map(|s| s.name().to_ascii_lowercase());
                let prop = ast::Property(p.clone());
                let name = prop.name();
                if name.is_empty() || presentation_only(&name) {
                    continue;
                }
                let group = match section {
                    Some(s) => format!("{kind}/{s}"),
                    None => kind.clone(),
                };
                out.push(occ(
                    Dim::Property,
                    group,
                    name,
                    usize::from(p.text_range().start()),
                    true,
                ));
            }
            for r in ast::routines_in(obj.syntax()) {
                let off = usize::from(r.syntax().text_range().start());
                let reached = corpus && reach.reached_at(fi, off);
                let name = r.name();
                if r.is_trigger() {
                    let section = r
                        .syntax()
                        .ancestors()
                        .skip(1)
                        .find_map(ast::Section::cast)
                        .map(|s| s.name().to_ascii_lowercase());
                    let group = match section {
                        Some(s) => format!("{kind}/{s}"),
                        None => kind.clone(),
                    };
                    out.push(occ(Dim::Trigger, group, name.clone(), off, reached));
                } else if !platform_app {
                    business.insert(name.to_ascii_lowercase());
                }
                for a in r.attributes() {
                    out.push(occ(Dim::Attribute, "procedure".into(), a.name(), off, reached));
                    let an = a.name().to_ascii_lowercase();
                    if an == "eventsubscriber" {
                        let args = a.args().map(|n| n.text().to_string()).unwrap_or_default();
                        let args = split_args(args.trim().trim_start_matches('(').trim_end_matches(')'));
                        if args.len() >= 3 {
                            let ev = args[2].trim_matches('\'').trim_matches('"').to_string();
                            events.push((
                                occ(Dim::Event, publisher_kind(&args[0]), ev.clone(), off, reached),
                                ev.to_ascii_lowercase(),
                            ));
                        }
                    } else if platform_app
                        && matches!(an.as_str(), "integrationevent" | "businessevent" | "internalevent")
                    {
                        let kind_group = kind.clone();
                        let mut o = occ(Dim::Event, kind_group, name.clone(), off, false);
                        o.microsoft = false; // a declaration, not a use
                        o.corpus = false;
                        events.push((o, "\u{0}declared".into()));
                    }
                }
                if let Some(body) = r.body() {
                    for n in body.descendants() {
                        if let Some((group, name)) = syntax_item(&n) {
                            out.push(occ(
                                Dim::Syntax,
                                group,
                                name,
                                usize::from(n.text_range().start()),
                                reached,
                            ));
                        }
                    }
                }
            }
        }
    }
    // Platform events: subscribed names no AL object declares as a procedure, and the platform
    // app's own publishers.
    for (o, key) in events {
        if key == "\u{0}declared" || !business.contains(&key) {
            out.push(o);
        }
    }
    out
}

/// The items: occurrences grouped, plus the names the runtime knows that no code uses.
pub fn classify(occs: &[Occ], known: &Known) -> Vec<Item> {
    let mut map: BTreeMap<(Dim, String, String), Item> = BTreeMap::new();
    // Keep the first spelling seen for a case-insensitive name.
    let mut spelling: HashMap<(Dim, String, String), String> = HashMap::new();
    for o in occs {
        let lower = o.name.to_ascii_lowercase();
        let key = (o.dim, o.group.clone(), lower);
        spelling.entry(key.clone()).or_insert_with(|| o.name.clone());
        let item = map.entry(key).or_insert_with(|| Item {
            dim: o.dim,
            group: o.group.clone(),
            name: o.name.clone(),
            ms_uses: 0,
            corpus_uses: 0,
            site: None,
            runtime_reads: known.knows(o.dim, &o.name),
        });
        if o.microsoft {
            item.ms_uses += 1;
        }
        if o.corpus {
            item.corpus_uses += 1;
            if o.reached {
                let site = (o.file.clone(), o.line);
                if item.site.as_ref().is_none_or(|s| site < *s) {
                    item.site = Some(site);
                }
            }
        }
    }
    // (a)-only: names the runtime source mentions that no scanned code uses.
    let used_triggers: HashSet<String> = map
        .keys()
        .filter(|(d, _, _)| matches!(d, Dim::Trigger | Dim::Event))
        .map(|(_, _, n)| n.clone())
        .collect();
    for name in &known.on_names {
        if used_triggers.contains(&name.to_ascii_lowercase()) {
            continue;
        }
        let dim = if name.ends_with("Event") {
            Dim::Event
        } else {
            Dim::Trigger
        };
        map.entry((dim, "(any)".into(), name.to_ascii_lowercase()))
            .or_insert_with(|| Item {
                dim,
                group: "(any)".into(),
                name: name.clone(),
                ms_uses: 0,
                corpus_uses: 0,
                site: None,
                runtime_reads: true,
            });
    }
    // Every grammar kind is in the catalog, even when no scanned code uses it.
    for k in STATEMENT_KINDS.iter().chain(&MORE_STATEMENT_KINDS) {
        map.entry((Dim::Syntax, "statement".into(), kind_name(*k).to_ascii_lowercase()))
            .or_insert_with(|| Item {
                dim: Dim::Syntax,
                group: "statement".into(),
                name: kind_name(*k),
                ms_uses: 0,
                corpus_uses: 0,
                site: None,
                runtime_reads: known.knows(Dim::Syntax, &kind_name(*k)),
            });
    }
    for k in &EXPRESSION_KINDS {
        map.entry((Dim::Syntax, "expression".into(), kind_name(*k).to_ascii_lowercase()))
            .or_insert_with(|| Item {
                dim: Dim::Syntax,
                group: "expression".into(),
                name: kind_name(*k),
                ms_uses: 0,
                corpus_uses: 0,
                site: None,
                runtime_reads: known.knows(Dim::Syntax, &kind_name(*k)),
            });
    }
    map.into_values().collect()
}

/// The error sites of the runtime against the string literals of the corpus.
pub fn match_errors(
    sites: Vec<(String, usize, String, String)>,
    corpus_literals: &[(String, usize, String)],
) -> Vec<ErrorSite> {
    sites
        .into_iter()
        .map(|(file, line, template, code)| {
            let generic = is_generic(&template);
            let site = if generic {
                None
            } else {
                corpus_literals
                    .iter()
                    .filter(|(_, _, lit)| template_in(&template, lit))
                    .map(|(f, l, _)| (f.clone(), *l))
                    .min()
            };
            ErrorSite {
                file,
                line,
                template,
                code,
                site,
                generic,
            }
        })
        .collect()
}

/// The string literals of the Workspace files, decoded.
pub fn corpus_literals(ws: &Workspace) -> Vec<(String, usize, String)> {
    let mut out = Vec::new();
    for (fi, file) in ws.files.iter().enumerate() {
        if ws.kinds[file.app] != AppKind::Workspace {
            continue;
        }
        let text = ws.text(fi);
        let root = SyntaxNode::new_root(ws.green(fi));
        let mut starts = vec![0usize];
        starts.extend(text.match_indices('\n').map(|(i, _)| i + 1));
        for t in root.descendants_with_tokens().filter_map(|e| e.into_token()) {
            if t.kind() == SyntaxKind::StringLit {
                let raw = t.text();
                let inner = raw.trim_matches('\'').replace("''", "'");
                let off = usize::from(t.text_range().start());
                out.push((file.path.clone(), starts.partition_point(|&s| s <= off), inner));
            }
        }
    }
    out
}

/// The map of the pinned corpus under `root` (the repository root).
pub fn corpus_behavior_coverage(root: &Path, corpus_commit: &str) -> Result<BehaviorCoverage, String> {
    let ws = crate::coverage::corpus_workspace(root)?;
    let members = al_sema::members::build(&ws);
    let catalog = al_builtins::catalog::Catalog::load();
    let checked = al_sema::check::check(&ws, &members, &catalog);

    let mut runtime = Vec::new();
    for c in RUNTIME_CRATES {
        runtime.extend(runtime_sources(&root.join("crates").join(c).join("src")));
    }
    let known = known_from_sources(&runtime);
    let items = classify(&collect(&ws, &checked), &known);

    let mut sites = Vec::new();
    for c in ERROR_CRATES {
        for (path, text) in runtime_sources(&root.join("crates").join(c).join("src")) {
            let rel = path
                .strip_prefix(&root.to_string_lossy().into_owned())
                .unwrap_or(&path)
                .trim_start_matches('/')
                .to_string();
            for (line, tpl, code) in error_sites(&rel, &text) {
                sites.push((rel.clone(), line, tpl, code));
            }
        }
    }
    sites.sort();
    sites.dedup();
    let errors = match_errors(sites, &corpus_literals(&ws));
    let lang = crate::language_catalog::LanguageCatalog::load(root)?;
    let language = Some(language_view(&lang, &items, &known));
    Ok(BehaviorCoverage {
        corpus_commit: corpus_commit.to_string(),
        items,
        errors,
        language,
    })
}

// ---------------------------------------------------------------------------------------------
// The language catalog view

use crate::language_catalog::{
    LanguageCatalog, State, event_owner, majors, property_owners, syntax_items, trigger_owners,
};

/// The areas of the language catalog this map measures, with their dimension.
pub const LANGUAGE_AREAS: [(&str, Dim); 5] = [
    ("triggers", Dim::Trigger),
    ("properties", Dim::Property),
    ("events", Dim::Event),
    ("syntax", Dim::Syntax),
    ("attributes", Dim::Attribute),
];

/// Object kinds whose lowering refuses every trigger or property it does not handle (the
/// catch-all reason in the named file). An item of such an owner that the runtime source does
/// not read is refused, not ignored. The test `refusing_owners_still_refuse` keeps the list honest.
pub const REFUSING_OWNERS: [(&str, &str, &str, &str); 29] = [
    (
        "properties",
        "Report",
        "reports.rs",
        "report property {other} is not supported yet",
    ),
    (
        "properties",
        "ReportDataItem",
        "reports.rs",
        "property {pn} of dataitem {name} is not supported yet",
    ),
    (
        "properties",
        "RequestPage",
        "reports.rs",
        "request page property {name} is not supported yet",
    ),
    (
        "properties",
        "RequestPageExtension",
        "reports.rs",
        "request page property {} is not supported yet",
    ),
    (
        "properties",
        "ReportExtension",
        "reports.rs",
        "reportextension property {name} is not supported yet",
    ),
    (
        "properties",
        "ReportExtensionModify",
        "reports.rs",
        "property {} of modify({target}) is not supported yet",
    ),
    (
        "properties",
        "Query",
        "queries.rs",
        "query property {} is not supported yet",
    ),
    (
        "properties",
        "QueryDataItem",
        "queries.rs",
        "dataitem {name}: property {other} is not supported yet",
    ),
    (
        "properties",
        "QueryColumn",
        "queries.rs",
        "column {name}: property {other} is not supported yet",
    ),
    (
        "properties",
        "XmlPort",
        "xmlports.rs",
        "xmlport property {other} is not supported yet",
    ),
    (
        "properties",
        "XmlPortTableElement",
        "xmlports.rs",
        "property {pn} of {section_name} {name} is not supported yet",
    ),
    (
        "properties",
        "XmlPortFieldElement",
        "xmlports.rs",
        "property {pn} of {section_name} {name} is not supported yet",
    ),
    (
        "properties",
        "XmlPortTextElement",
        "xmlports.rs",
        "property {pn} of {section_name} {name} is not supported yet",
    ),
    (
        "properties",
        "XmlPortFieldAttribute",
        "xmlports.rs",
        "property {pn} of {section_name} {name} is not supported yet",
    ),
    (
        "properties",
        "XmlPortTextAttribute",
        "xmlports.rs",
        "property {pn} of {section_name} {name} is not supported yet",
    ),
    (
        "triggers",
        "Report",
        "reports.rs",
        "report trigger {n} is not supported yet",
    ),
    (
        "triggers",
        "ReportDataItem",
        "reports.rs",
        "trigger {tn} of dataitem {name} is not supported yet",
    ),
    (
        "triggers",
        "RequestPage",
        "reports.rs",
        "request page trigger {name} is not supported yet",
    ),
    (
        "triggers",
        "RequestPageExtension",
        "reports.rs",
        "request page trigger {} is not supported yet",
    ),
    (
        "triggers",
        "ReportExtension",
        "reports.rs",
        "reportextension trigger {n} is not supported yet",
    ),
    (
        "triggers",
        "ReportExtensionDataSetModify",
        "reports.rs",
        "trigger {} in modify({}) is not supported yet",
    ),
    (
        "triggers",
        "PageExtension",
        "lower/shared.rs",
        "page extension `{ename}`: trigger {} is not supported yet",
    ),
    (
        "triggers",
        "ControlExtension",
        "lower/shared.rs",
        "trigger {} in modify({}) is not supported yet",
    ),
    (
        "triggers",
        "ActionExtension",
        "lower/shared.rs",
        "trigger {} in modify({}) is not supported yet",
    ),
    (
        "triggers",
        "XmlPortTableElement",
        "xmlports.rs",
        "trigger {tn} of {section_name} {name} is not supported yet",
    ),
    (
        "triggers",
        "XmlPortFieldElement",
        "xmlports.rs",
        "trigger {tn} of {section_name} {name} is not supported yet",
    ),
    (
        "triggers",
        "XmlPortTextElement",
        "xmlports.rs",
        "trigger {tn} of {section_name} {name} is not supported yet",
    ),
    (
        "triggers",
        "XmlPortFieldAttribute",
        "xmlports.rs",
        "trigger {tn} of {section_name} {name} is not supported yet",
    ),
    (
        "triggers",
        "XmlPortTextAttribute",
        "xmlports.rs",
        "trigger {tn} of {section_name} {name} is not supported yet",
    ),
];

/// One item of the language catalog in a measured area.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LangItem {
    pub area: String,
    pub owner: String,
    pub name: String,
    pub bc: Vec<u32>,
    pub scope: String,
    pub site: Option<(String, usize)>,
    pub state: State,
    pub ms_uses: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageView {
    pub majors: String,
    /// Measured items (presentation-only properties and keywords without an item left out).
    pub items: Vec<LangItem>,
    /// Presentation-only properties of the catalog, left out (as in the dimension above).
    pub presentation: usize,
    /// Statement keywords without a syntax item of their own (`else`).
    pub no_item: Vec<String>,
    /// Catalog areas this map does not measure yet, with their size.
    pub not_measured: Vec<(String, usize)>,
}

/// Every trigger, trigger event, property, statement keyword and attribute of the language
/// catalog, with the first pinned corpus site, the runtime state and the Microsoft use, taken from
/// the items of the map whose group maps to the item's owner.
pub fn language_view(lang: &LanguageCatalog, items: &[Item], known: &Known) -> LanguageView {
    // (dim, owner, lower name) -> (first site, MS uses)
    type Acc = (Option<(String, usize)>, usize);
    let mut acc: HashMap<(Dim, String, String), Acc> = HashMap::new();
    let mut add = |dim: Dim, owner: &str, name: &str, it: &Item| {
        let e = acc
            .entry((dim, owner.to_string(), name.to_ascii_lowercase()))
            .or_insert((None, 0));
        if let Some(site) = &it.site
            && e.0.as_ref().is_none_or(|s| site < s)
        {
            e.0 = Some(site.clone());
        }
        e.1 += it.ms_uses;
    };
    for it in items {
        match it.dim {
            Dim::Trigger => {
                for o in trigger_owners(&it.group) {
                    add(Dim::Trigger, o, &it.name, it);
                }
            }
            Dim::Property => {
                for o in property_owners(&it.group) {
                    add(Dim::Property, o, &it.name, it);
                }
            }
            Dim::Event => {
                if let Some(o) = event_owner(&it.group) {
                    add(Dim::Event, o, &it.name, it);
                }
            }
            Dim::Syntax => add(Dim::Syntax, "", &it.name, it),
            Dim::Attribute => add(Dim::Attribute, "", &it.name, it),
        }
    }
    let refusing = |area: &str, owner: &str| REFUSING_OWNERS.iter().any(|(a, o, _, _)| *a == area && *o == owner);
    let mut out = Vec::new();
    let mut presentation = 0;
    let mut no_item = Vec::new();
    for (area, dim) in LANGUAGE_AREAS {
        for e in lang.area(area) {
            if dim == Dim::Property && presentation_only(&e.name) {
                presentation += 1;
                continue;
            }
            let (site, ms_uses, reads) = if dim == Dim::Syntax {
                let Some(targets) = syntax_items(&e.name) else {
                    no_item.push(e.name.clone());
                    continue;
                };
                let mut site: Option<(String, usize)> = None;
                let mut ms = 0;
                for t in targets {
                    if let Some((s, m)) = acc.get(&(Dim::Syntax, String::new(), t.to_ascii_lowercase())) {
                        ms += m;
                        if let Some(x) = s
                            && site.as_ref().is_none_or(|y| x < y)
                        {
                            site = Some(x.clone());
                        }
                    }
                }
                (site, ms, targets.iter().any(|t| known.knows(Dim::Syntax, t)))
            } else {
                let (site, ms) = acc
                    .get(&(dim, e.owner.clone(), e.name.to_ascii_lowercase()))
                    .cloned()
                    .unwrap_or((None, 0));
                (site, ms, known.knows(dim, &e.name))
            };
            let state = if reads {
                State::Implemented
            } else if refusing(area, &e.owner) {
                State::Refused
            } else {
                State::Unknown
            };
            out.push(LangItem {
                area: area.to_string(),
                owner: e.owner.clone(),
                name: e.name.clone(),
                bc: e.bc.clone(),
                scope: e.scope.clone(),
                site,
                state,
                ms_uses,
            });
        }
    }
    let measured: HashSet<&str> = LANGUAGE_AREAS.iter().map(|(a, _)| *a).collect();
    let mut not_measured: BTreeMap<String, usize> = BTreeMap::new();
    for e in &lang.items {
        if !measured.contains(e.area.as_str()) && e.area != "methods" {
            *not_measured.entry(e.area.clone()).or_default() += 1;
        }
    }
    LanguageView {
        majors: lang.majors(),
        items: out,
        presentation,
        no_item,
        not_measured: not_measured.into_iter().collect(),
    }
}

fn language_markdown(v: &LanguageView, s: &mut String) {
    use crate::coverage::{Counts, pct};
    s.push_str(&format!(
        "## Language catalog (AL language server, BC {})\n\n",
        v.majors
    ));
    s.push_str("The catalog of each area is what the AL language server offers in probe files (`docs/language-catalog.json`, generated by `tools/language-catalog/generate.sh`): triggers per owner, trigger events, properties per owner (the probe contexts: `PageField`, `ReportDataItem`, ...), statement keywords and attributes. **Pinned**: an item of the dimension tables below whose group maps to the owner (`crates/al-test-cli/src/language_catalog.rs`) is pinned. **Implemented**: a string literal of the runtime crates names it (the runtime reads it; a name read for one owner counts for all). **Refused**: not read, and the owner's lowering refuses what it does not handle (`REFUSING_OWNERS`). **Unknown**: not read and not refused: the runtime ignores it or rejects it as unknown. Method overloads are in `corpus-coverage.md`.\n\n");
    let mut by_area: BTreeMap<&str, Vec<&LangItem>> = BTreeMap::new();
    for i in &v.items {
        by_area.entry(i.area.as_str()).or_default().push(i);
    }
    let counts = |items: &[&LangItem]| {
        let mut c = Counts::default();
        for i in items {
            c.add(i.site.is_some(), i.state, i.ms_uses > 0);
        }
        c
    };
    let row = |label: &str, c: Counts| {
        format!(
            "| {label} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            c.all,
            c.pinned,
            pct(c.pinned, c.all),
            c.implemented,
            pct(c.implemented, c.all),
            c.refused,
            c.unknown,
            c.ms_unpinned
        )
    };
    const HEAD: &str = "| Area | Items | Pinned | Pinned % | Implemented | Implemented % | Refused | Unknown | MS-used unpinned |\n|---|---:|---:|---:|---:|---:|---:|---:|---:|\n";
    s.push_str(HEAD);
    for (area, items) in &by_area {
        s.push_str(&row(area, counts(items)));
    }
    s.push_str(&format!(
        "\nLeft out: {} presentation-only properties (Caption, ToolTip, ApplicationArea, ...), {} statement keywords without an item of their own ({}). Not measured yet: {}.\n",
        v.presentation,
        v.no_item.len(),
        v.no_item.join(", "),
        v.not_measured.iter().map(|(a, n)| format!("{a} ({n})")).collect::<Vec<_>>().join(", ")
    ));
    for (area, items) in &by_area {
        let mut by_owner: BTreeMap<&str, Vec<&LangItem>> = BTreeMap::new();
        for i in items {
            by_owner.entry(i.owner.as_str()).or_default().push(i);
        }
        s.push_str(&format!("\n### {area} per owner\n\n"));
        s.push_str(&HEAD.replace("| Area |", "| Owner |"));
        for (owner, list) in &by_owner {
            s.push_str(&row(if owner.is_empty() { "(code)" } else { owner }, counts(list)));
        }
        s.push_str(&format!("\n#### {area}: unpinned items by owner\n\n"));
        for (owner, list) in &by_owner {
            let mut gap: Vec<&&LangItem> = list.iter().filter(|i| i.site.is_none()).collect();
            if gap.is_empty() {
                continue;
            }
            gap.sort_by_key(|i| i.name.to_ascii_lowercase());
            s.push_str(&format!(
                "- **{}** ({} of {}): {}\n",
                if owner.is_empty() { "(code)" } else { owner },
                gap.len(),
                list.len(),
                gap.iter()
                    .map(|i| {
                        let mut t = format!("`{}` {}", i.name, i.state.label());
                        if i.ms_uses > 0 {
                            t.push_str(&format!(", MS {}", i.ms_uses));
                        }
                        if majors(&i.bc) != v.majors {
                            t.push_str(&format!(", BC {}", majors(&i.bc)));
                        }
                        if !i.scope.is_empty() {
                            t.push_str(&format!(", {}", i.scope));
                        }
                        t
                    })
                    .collect::<Vec<_>>()
                    .join("; ")
            ));
        }
    }
    s.push('\n');
}

// ---------------------------------------------------------------------------------------------
// Output

impl BehaviorCoverage {
    pub fn of(&self, dim: Dim) -> Vec<&Item> {
        self.items.iter().filter(|i| i.dim == dim).collect()
    }
}

fn gap_order(a: &&Item, b: &&Item) -> std::cmp::Ordering {
    b.ms_uses
        .cmp(&a.ms_uses)
        .then(b.runtime_reads.cmp(&a.runtime_reads))
        .then(a.group.cmp(&b.group))
        .then(a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase()))
}

/// Rows of a gap table per dimension in the Markdown.
const GAP_ROWS: usize = 250;

pub fn to_markdown(c: &BehaviorCoverage) -> String {
    let mut s = String::new();
    s.push_str("# Corpus coverage of language behavior\n\n");
    s.push_str("Generated by `al-test-cli --behavior-coverage-map` (not committed, ADR-065; the test `corpus_behavior_coverage_builds` checks that it can be built). Do not edit. Plan: `docs/reviews/2026-10-behavior-coverage-plan.md`. The builtin overloads are in `corpus-coverage.md`.\n\n");
    s.push_str(&format!("Corpus commit `{}`.\n\n", c.corpus_commit));
    if let Some(v) = &c.language {
        language_markdown(v, &mut s);
        s.push_str("## Items the runtime or Microsoft's apps mention\n\n");
    }
    s.push_str("An item is **pinned** when it occurs in a corpus file in a routine that static reach finds (roots: triggers, `[Test]` procedures, event subscribers and handlers of corpus objects; edges: resolved procedure calls). Properties count when declared on a corpus object. Reached is not observed. The catalog of each dimension is the union of what the runtime source mentions and what Microsoft's apps (System, System Application, Base Application, Business Foundation, Application) use. **MS uses** counts the occurrences in those apps. **Runtime reads** says a string literal (or `SyntaxKind` name) of the runtime crates matches the item; `no` means the runtime does not read it, so even a passing probe needs new code. Gap lists are sorted by MS uses.\n\n");
    s.push_str("| Dimension | Catalog | Pinned | Used by Microsoft | Microsoft-used and unpinned | of those, runtime does not read |\n|---|---:|---:|---:|---:|---:|\n");
    for d in Dim::ALL {
        let items = c.of(d);
        let pinned = items.iter().filter(|i| i.site.is_some()).count();
        let ms = items.iter().filter(|i| i.ms_uses > 0).count();
        let gap = items.iter().filter(|i| i.ms_uses > 0 && i.site.is_none()).count();
        let unknown = items
            .iter()
            .filter(|i| i.ms_uses > 0 && i.site.is_none() && !i.runtime_reads)
            .count();
        s.push_str(&format!(
            "| {} | {} | {pinned} | {ms} | {gap} | {unknown} |\n",
            d.title(),
            items.len()
        ));
    }
    let pinned_err = c.errors.iter().filter(|e| e.site.is_some()).count();
    let generic = c.errors.iter().filter(|e| e.generic).count();
    s.push_str(&format!(
        "| Runtime error sites (text pinned) | {} | {pinned_err} | - | {} | - |\n\n",
        c.errors.len(),
        c.errors.len() - pinned_err
    ));
    s.push_str("Presentation-only properties (Caption, ToolTip, ApplicationArea, ... ) are left out of the property catalog.\n");
    for d in Dim::ALL {
        let mut gaps: Vec<&Item> = c.of(d).into_iter().filter(|i| i.site.is_none()).collect();
        gaps.sort_by(gap_order);
        s.push_str(&format!("\n## {}: unpinned items ({})\n\n", d.title(), gaps.len()));
        s.push_str("| Group | Name | MS uses | Runtime reads | Corpus uses (not reached) |\n|---|---|---:|---|---:|\n");
        for i in gaps.iter().take(GAP_ROWS) {
            s.push_str(&format!(
                "| {} | `{}` | {} | {} | {} |\n",
                i.group,
                i.name,
                i.ms_uses,
                if i.runtime_reads { "yes" } else { "no" },
                i.corpus_uses
            ));
        }
        if gaps.len() > GAP_ROWS {
            s.push_str(&format!(
                "\n{} more in `corpus-behavior-coverage.json`.\n",
                gaps.len() - GAP_ROWS
            ));
        }
    }
    s.push_str(&format!(
        "\n## Runtime error sites without a corpus test ({} of {}; {generic} have too little fixed text to match)\n\n",
        c.errors.len() - pinned_err,
        c.errors.len()
    ));
    s.push_str("A site is pinned when a corpus string literal holds the message text. Codes are not matched. Sites with a computed message are not listed.\n\n");
    for e in c.errors.iter().filter(|e| e.site.is_none()) {
        s.push_str(&format!(
            "- {}:{} `{}`{}{}\n",
            e.file,
            e.line,
            e.template.replace('\n', " "),
            if e.code.is_empty() {
                String::new()
            } else {
                format!(" code `{}`", e.code)
            },
            if e.generic { " (generic)" } else { "" }
        ));
    }
    s
}

pub fn to_json(c: &BehaviorCoverage) -> String {
    let dims: Vec<serde_json::Value> = Dim::ALL
        .iter()
        .map(|&d| {
            let mut items = c.of(d);
            items.sort_by(gap_order);
            serde_json::json!({
                "dimension": d.key(),
                "catalog": items.len(),
                "pinned": items.iter().filter(|i| i.site.is_some()).count(),
                "items": items.iter().map(|i| serde_json::json!({
                    "group": i.group,
                    "name": i.name,
                    "ms_uses": i.ms_uses,
                    "corpus_uses": i.corpus_uses,
                    "runtime_reads": i.runtime_reads,
                    "pinned": i.site.is_some(),
                    "site": i.site.as_ref().map(|(f, l)| format!("{f}:{l}")),
                })).collect::<Vec<_>>(),
            })
        })
        .collect();
    let language = c.language.as_ref().map(|v| {
        serde_json::json!({
            "bc": v.majors,
            "presentation_left_out": v.presentation,
            "keywords_without_item": v.no_item,
            "not_measured": v.not_measured.iter().map(|(a, n)| serde_json::json!({"area": a, "items": n})).collect::<Vec<_>>(),
            "items": v.items.iter().map(|i| serde_json::json!({
                "area": i.area,
                "owner": i.owner,
                "name": i.name,
                "bc": i.bc,
                "scope": i.scope,
                "pinned": i.site.is_some(),
                "site": i.site.as_ref().map(|(f, l)| format!("{f}:{l}")),
                "state": i.state.label(),
                "ms_uses": i.ms_uses,
            })).collect::<Vec<_>>(),
        })
    });
    let mut text = serde_json::to_string_pretty(&serde_json::json!({
        "corpus_commit": c.corpus_commit,
        "language": language,
        "dimensions": dims,
        "errors": {
            "sites": c.errors.len(),
            "pinned": c.errors.iter().filter(|e| e.site.is_some()).count(),
            "items": c.errors.iter().map(|e| serde_json::json!({
                "file": e.file,
                "line": e.line,
                "template": e.template,
                "code": e.code,
                "generic": e.generic,
                "pinned": e.site.is_some(),
                "site": e.site.as_ref().map(|(f, l)| format!("{f}:{l}")),
            })).collect::<Vec<_>>(),
        },
    }))
    .expect("json");
    text.push('\n');
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use al_sema::workspace::AppSource;

    fn app(id: &str, name: &str, deps: &[&str], file: &str, text: &str) -> AppSource {
        AppSource {
            id: id.into(),
            name: name.into(),
            dependencies: deps.iter().map(|d| d.to_string()).collect(),
            files: vec![(file.into(), text.into())],
        }
    }

    /// A Microsoft-like dependency, then a corpus app.
    const MS: &str = "table 100 T
{
    fields { field(1; A; Integer) { trigger OnValidate() begin if 1 < 2 then exit; end; } }
    trigger OnInsert() begin if 1 < 2 then exit; end;
    trigger OnDelete() begin end;
    procedure BusinessEventPublisher() begin end;
}
codeunit 101 S
{
    [EventSubscriber(ObjectType::Table, Database::T, 'OnBeforeInsertEvent', '', false, false)]
    local procedure A() begin end;
    [EventSubscriber(ObjectType::Table, Database::T, 'OnBeforeInsertEvent', '', false, false)]
    local procedure B() begin end;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::S, 'OnMyBusiness', '', false, false)]
    local procedure C() begin end;
    procedure OnMyBusiness() begin end;
}
";
    const CORPUS: &str = "table 50100 U
{
    DelayedInsert = true;
    fields { field(1; A; Integer) { } }
    trigger OnInsert() begin if 1 < 2 then exit; end;
}
codeunit 50101 Tests
{
    Subtype = Test;
    [Test]
    procedure Reaches() begin Helper(); end;
    local procedure Helper() begin while false do ; end;
    local procedure Unreached() begin repeat until true; end;
}
";

    fn collected() -> (Vec<Occ>, Vec<Item>) {
        let ws = Workspace::load_apps(
            vec![
                app("d", "Base Application", &[], "ms.al", MS),
                app("w", "corpus", &["d"], "own.al", CORPUS),
            ],
            vec![AppKind::Dependency, AppKind::Workspace],
            &Default::default(),
        );
        let members = al_sema::members::build(&ws);
        let checked = al_sema::check::check(&ws, &members, &al_builtins::catalog::Catalog::load());
        let occs = collect(&ws, &checked);
        let items = classify(&occs, &Known::default());
        (occs, items)
    }

    fn item<'a>(items: &'a [Item], dim: Dim, name: &str) -> &'a Item {
        items
            .iter()
            .find(|i| i.dim == dim && i.name.eq_ignore_ascii_case(name))
            .unwrap_or_else(|| panic!("no item {name}"))
    }

    #[test]
    fn a_trigger_is_pinned_by_a_corpus_trigger_and_counted_in_microsoft_code() {
        let (_, items) = collected();
        let insert = item(&items, Dim::Trigger, "OnInsert");
        assert_eq!(insert.ms_uses, 1);
        assert_eq!(insert.site, Some(("own.al".into(), 5)));
        let delete = item(&items, Dim::Trigger, "OnDelete");
        assert_eq!((delete.ms_uses, delete.site.clone()), (1, None));
        let validate = items.iter().find(|i| i.name == "OnValidate").unwrap();
        assert_eq!(validate.group, "table/field");
    }

    #[test]
    fn a_statement_is_pinned_only_in_a_reached_routine() {
        let (_, items) = collected();
        // Helper is called by the test; Unreached is not called by anything.
        assert_eq!(item(&items, Dim::Syntax, "WhileStmt").site, Some(("own.al".into(), 12)));
        assert_eq!(item(&items, Dim::Syntax, "RepeatStmt").site, None);
        assert_eq!(item(&items, Dim::Syntax, "RepeatStmt").corpus_uses, 1);
        // Operators are told apart.
        assert_eq!(item(&items, Dim::Syntax, "BinExpr <").ms_uses, 2);
        // A kind nothing uses is still in the catalog.
        assert_eq!(item(&items, Dim::Syntax, "ForeachStmt").ms_uses, 0);
    }

    #[test]
    fn properties_count_at_object_level_and_presentation_properties_are_left_out() {
        let (_, items) = collected();
        assert_eq!(
            item(&items, Dim::Property, "DelayedInsert").site,
            Some(("own.al".into(), 3))
        );
        assert!(items.iter().all(|i| !i.name.eq_ignore_ascii_case("Caption")));
    }

    #[test]
    fn platform_events_exclude_events_an_al_object_declares() {
        let (_, items) = collected();
        let before = item(&items, Dim::Event, "OnBeforeInsertEvent");
        assert_eq!((before.ms_uses, before.group.as_str()), (2, "table"));
        assert!(
            items.iter().all(|i| i.name != "OnMyBusiness"),
            "a business event is not a platform event"
        );
    }

    #[test]
    fn an_item_only_the_runtime_knows_or_only_microsoft_uses_is_told_apart() {
        let mut known = Known::default();
        known.strings.insert("oninsert".into());
        known.on_names.insert("OnRename".into());
        known.on_names.insert("OnInsert".into());
        let (occs, _) = collected();
        let items = classify(&occs, &known);
        assert!(item(&items, Dim::Trigger, "OnInsert").runtime_reads);
        assert!(!item(&items, Dim::Trigger, "OnDelete").runtime_reads);
        let rename = item(&items, Dim::Trigger, "OnRename");
        assert_eq!(
            (rename.ms_uses, rename.runtime_reads, rename.group.as_str()),
            (0, true, "(any)")
        );
        // OnInsert is used, so it is not added a second time as a runtime-only name.
        assert_eq!(items.iter().filter(|i| i.name == "OnInsert").count(), 1);
    }

    #[test]
    fn rust_strings_skips_comments_and_char_literals_and_reads_escapes() {
        let src = "let a = \"x\\\"y\"; // \"no\"\nlet c = '\"'; let d = r#\"raw \"q\"\"#; /* \"no\" */ let e = \"a\\\n   b\";";
        let got: Vec<String> = rust_strings(src).into_iter().map(|l| l.text).collect();
        assert_eq!(got, vec!["x\"y", "raw \"q\"", "ab"]);
    }

    #[test]
    fn error_sites_read_raise_and_alerror_with_their_code() {
        let src = "fn f() { raise(\"The record does not exist.\", \"Dialog\"); raise(format!(\"Value {} too big\", x), \"Overflow\");\n let e = AlError { message: format!(\"Bad {:?}\", y), code: \"Bad\".into(), call_stack: String::new() }; raise(msg, \"X\"); }";
        let sites = error_sites("f.rs", src);
        let got: Vec<(String, String)> = sites.into_iter().map(|(_, t, c)| (t, c)).collect();
        assert!(
            got.contains(&("The record does not exist.".into(), "Dialog".into())),
            "{got:?}"
        );
        assert!(got.contains(&("Value % too big".into(), "Overflow".into())), "{got:?}");
        assert!(got.iter().any(|(t, _)| t == "Bad %"), "{got:?}");
        assert_eq!(got.len(), 3, "a computed message is not a site: {got:?}");
    }

    #[test]
    fn an_error_text_is_pinned_when_the_fixed_pieces_occur_in_order_in_one_corpus_literal() {
        let lits = vec![
            (
                "a.al".to_string(),
                7usize,
                "Expected: The Customer does not exist. Got x".to_string(),
            ),
            ("a.al".to_string(), 3, "Other".to_string()),
        ];
        let sites = vec![
            (
                "r.rs".to_string(),
                1,
                "The % does not exist.".to_string(),
                String::new(),
            ),
            (
                "r.rs".to_string(),
                2,
                "Never said anywhere here.".to_string(),
                String::new(),
            ),
            ("r.rs".to_string(), 3, "%: %".to_string(), String::new()),
        ];
        let got = match_errors(sites, &lits);
        assert_eq!(got[0].site, Some(("a.al".into(), 7)));
        assert_eq!(got[1].site, None);
        assert!(got[2].generic && got[2].site.is_none());
    }

    #[test]
    fn the_output_names_the_dimensions_and_sorts_gaps_by_microsoft_use() {
        let (occs, _) = collected();
        let mut c = BehaviorCoverage {
            corpus_commit: "abc".into(),
            items: classify(&occs, &Known::default()),
            errors: vec![],
            language: None,
        };
        c.errors.push(ErrorSite {
            file: "r.rs".into(),
            line: 4,
            template: "Nope".into(),
            code: "C".into(),
            site: None,
            generic: true,
        });
        let md = to_markdown(&c);
        assert!(md.contains("Corpus commit `abc`"));
        assert!(md.contains("## Triggers per object kind: unpinned items"));
        assert!(md.contains("r.rs:4 `Nope` code `C` (generic)"));
        let json: serde_json::Value = serde_json::from_str(&to_json(&c)).unwrap();
        assert_eq!(
            json["dimensions"].as_array().unwrap().len(),
            5,
            "triggers, properties, events, syntax, attributes"
        );
        // Gaps: the highest Microsoft use first.
        let trig = &json["dimensions"][0]["items"];
        let uses: Vec<u64> = trig
            .as_array()
            .unwrap()
            .iter()
            .map(|i| i["ms_uses"].as_u64().unwrap())
            .collect();
        let mut sorted = uses.clone();
        sorted.sort_by(|a, b| b.cmp(a));
        assert_eq!(uses, sorted);
    }

    #[test]
    fn a_grammar_kind_the_lowering_matches_by_its_bare_name_is_read() {
        let k = known_from_sources(&[(
            "lower.rs".into(),
            "use SyntaxKind::*;\nmatch k { IfStmt => 1, PlusAssign => 2, IfStmtX => 3 }\n".into(),
        )]);
        assert!(k.knows(Dim::Syntax, "IfStmt"));
        assert!(k.knows(Dim::Syntax, "AssignStmt +="));
        assert!(!k.knows(Dim::Syntax, "AssignStmt -="));
        assert!(!k.knows(Dim::Syntax, "WhileStmt"));
    }

    #[test]
    fn names_in_a_validation_only_list_are_not_read() {
        // A list the front end only checks names against (AL0124 allowed properties) says nothing about
        // what the runtime does with the property.
        let k = known_from_sources(&[(
            "alcodes.rs".into(),
            "fn a() { x(\"Subtype\"); }\n\
             // behavior-coverage: validation-only begin\n\
             const ALLOWED: &[&str] = &[\"TestIsolation\", \"OnBeforeTestRun\"];\n\
             // behavior-coverage: validation-only end\n\
             fn b() { x(\"Permissions\"); }\n"
                .into(),
        )]);
        assert!(k.knows(Dim::Property, "Subtype"));
        assert!(k.knows(Dim::Property, "Permissions"));
        assert!(!k.knows(Dim::Property, "TestIsolation"));
        assert!(!k.on_names.contains("OnBeforeTestRun"));
    }

    #[test]
    fn refusing_owners_still_refuse() {
        let sema = Path::new(env!("CARGO_MANIFEST_DIR")).join("../al-sema/src");
        for (area, owner, file, text) in REFUSING_OWNERS {
            let src = std::fs::read_to_string(sema.join(file)).unwrap();
            assert!(
                src.contains(text),
                "{area} {owner}: `{text}` is no longer in al-sema/src/{file}"
            );
        }
    }

    #[test]
    fn compound_assignment_this_and_attributes_are_their_own_items() {
        let ws = Workspace::load(
            vec![app(
                "w",
                "w",
                &[],
                "w.al",
                "codeunit 60000 W\n{\n    [Test]\n    procedure T()\n    var\n        i: Integer;\n    begin\n        i := 1;\n        i += 2;\n        this.T2();\n    end;\n\n    procedure T2()\n    begin\n    end;\n}\n",
            )],
            &Default::default(),
        );
        let members = al_sema::members::build(&ws);
        let checked = al_sema::check::check(&ws, &members, &al_builtins::catalog::Catalog::load());
        let occs = collect(&ws, &checked);
        let attr: Vec<&Occ> = occs.iter().filter(|o| o.dim == Dim::Attribute).collect();
        assert_eq!(attr.len(), 1, "the [Test] attribute");
        assert!(attr[0].name == "Test" && attr[0].reached && attr[0].group == "procedure");
        let names: Vec<String> = occs
            .into_iter()
            .filter(|o| o.dim == Dim::Syntax)
            .map(|o| o.name)
            .collect();
        assert!(names.iter().any(|n| n == "AssignStmt"), "{names:?}");
        assert!(names.iter().any(|n| n == "AssignStmt +="), "{names:?}");
        assert!(names.iter().any(|n| n == "NameRef this"), "{names:?}");
    }

    #[test]
    fn the_language_view_maps_groups_to_catalog_owners_with_state_and_microsoft_use() {
        let lang = LanguageCatalog::parse(
            r#"{"sources":[{"bc":27,"al_extension":"a"},{"bc":29,"al_extension":"b"}],"items":[
{"area":"triggers","owner":"Field","name":"OnValidate","bc":[27,29]},
{"area":"triggers","owner":"ReportDataItem","name":"OnPreDataItemX","bc":[29]},
{"area":"triggers","owner":"Table","name":"OnNothing","bc":[27,29]},
{"area":"properties","owner":"Field","name":"Caption","bc":[27,29]},
{"area":"properties","owner":"PageField","name":"Editable","bc":[27,29]},
{"area":"events","owner":"Table","name":"OnAfterInsertEvent","bc":[27,29]},
{"area":"syntax","owner":"keyword","name":"if","bc":[27,29]},
{"area":"syntax","owner":"keyword","name":"else","bc":[27,29]},
{"area":"datatypes","name":"Integer","bc":[27,29]}
]}"#,
        )
        .unwrap();
        let item = |dim, group: &str, name: &str, site: Option<usize>, ms| Item {
            dim,
            group: group.into(),
            name: name.into(),
            ms_uses: ms,
            corpus_uses: usize::from(site.is_some()),
            site: site.map(|l| ("a.al".to_string(), l)),
            runtime_reads: false,
        };
        let items = vec![
            item(Dim::Trigger, "table/field", "OnValidate", Some(9), 3),
            item(Dim::Trigger, "tableextension/field", "OnValidate", Some(4), 2),
            item(Dim::Property, "pageextension/field", "editable", None, 5),
            item(Dim::Event, "table", "OnAfterInsertEvent", Some(1), 0),
            item(Dim::Syntax, "statement", "IfStmt", Some(7), 1),
        ];
        let known = Known {
            strings: ["onvalidate".to_string()].into_iter().collect(),
            ..Default::default()
        };
        let v = language_view(&lang, &items, &known);
        let get = |n: &str| v.items.iter().find(|i| i.name == n).unwrap();
        assert_eq!(
            get("OnValidate").site,
            Some(("a.al".into(), 4)),
            "first site over both groups"
        );
        assert_eq!(get("OnValidate").ms_uses, 5);
        assert_eq!(get("OnValidate").state, State::Implemented);
        assert_eq!(
            get("OnPreDataItemX").state,
            State::Refused,
            "report data items refuse unknown triggers"
        );
        assert_eq!(get("OnNothing").state, State::Unknown);
        assert_eq!(get("Editable").ms_uses, 5);
        assert!(get("Editable").site.is_none());
        assert!(get("OnAfterInsertEvent").site.is_some());
        assert_eq!(get("if").site, Some(("a.al".into(), 7)));
        assert_eq!(v.presentation, 1, "Caption is left out");
        assert_eq!(v.no_item, vec!["else".to_string()]);
        assert_eq!(v.not_measured, vec![("datatypes".to_string(), 1)]);
        let mut c = BehaviorCoverage {
            corpus_commit: "abc".into(),
            items,
            errors: vec![],
            language: Some(v),
        };
        let md = to_markdown(&c);
        assert!(
            md.contains("## Language catalog (AL language server, BC 27, 29)"),
            "{md}"
        );
        assert!(
            md.contains("| triggers | 3 | 1 | 33.3% | 1 | 33.3% | 1 | 1 | 0 |"),
            "{md}"
        );
        assert!(
            md.contains("- **ReportDataItem** (1 of 1): `OnPreDataItemX` refused, BC 29"),
            "{md}"
        );
        c.language = None;
        assert!(!to_markdown(&c).contains("Language catalog"));
    }
}
