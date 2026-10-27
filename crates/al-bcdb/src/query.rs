//! The BC meaning layer over a [`BcSource`] (port of the shared part of `Program.cs`):
//! company and app-id parsing of per-company table names, table resolution, `$ext`
//! companion merging, AL symbol names. Shared by every container and every front end.

use crate::error::{Error, Result};
use crate::source::{BcSource, SourceTable};
use crate::symbols::{AlField, AlTable, AlTableExtension, SymbolStore, ci_key, eq_ci, normalize_sql_name};
use crate::types::{SysColumn, type_name};
use crate::value::Value;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

/// A source table with its name taken apart: `<company>$<table>$<app id>` (and `$ext`).
#[derive(Debug, Clone)]
pub struct BcTable {
    pub table: SourceTable,
    pub company: Option<String>,
    pub table_name: String,
    pub app_id: Option<String>,
}

impl BcTable {
    pub fn sql_name(&self) -> &str {
        &self.table.name
    }
}

/// A GUID in any of the textual forms .NET's `Guid.TryParse` accepts that BC names use:
/// 8-4-4-4-12 or 32 hex digits, optionally in braces or parentheses.
pub fn is_guid(s: &str) -> bool {
    let inner = match (s.as_bytes().first(), s.as_bytes().last()) {
        (Some(b'{'), Some(b'}')) | (Some(b'('), Some(b')')) if s.len() >= 2 => &s[1..s.len() - 1],
        _ => s,
    };
    let hex = |p: &str| p.bytes().all(|b| b.is_ascii_hexdigit());
    if inner.len() == 32 {
        return hex(inner);
    }
    let parts: Vec<&str> = inner.split('-').collect();
    parts.len() == 5
        && [8, 4, 4, 4, 12]
            .iter()
            .zip(&parts)
            .all(|(n, p)| p.len() == *n && hex(p))
}

/// Takes a SQL object name apart. A name that matches no `<company>$<table>$<appid>` shape
/// (the platform's `$ndo$...` tables lead with `$`) keeps its raw SQL name: an empty derived
/// name would make the table undiscoverable from the listing.
pub fn parse_table_name(name: &str) -> (Option<String>, String, Option<String>) {
    let segs: Vec<&str> = name.split('$').collect();
    let is_ext = segs.last() == Some(&"ext");
    let core: &[&str] = if is_ext { &segs[..segs.len() - 1] } else { &segs };
    let (mut company, mut app_id, mut table_name) = (None, None, String::new());
    if core.len() >= 3 && is_guid(core[core.len() - 1]) {
        company = Some(core[..core.len() - 2].join("$"));
        table_name = core[core.len() - 2].to_string();
        app_id = Some(core[core.len() - 1].to_string());
    } else if core.len() == 2 && is_guid(core[1]) {
        table_name = core[0].to_string();
        app_id = Some(core[1].to_string());
    } else if core.len() == 2 {
        company = Some(core[0].to_string());
        table_name = core[1].to_string();
    } else if let Some(first) = core.first() {
        table_name = first.to_string();
    }
    if is_ext {
        table_name.push_str("$ext");
    }
    if table_name.is_empty() || table_name == "$ext" {
        return (None, name.to_string(), None);
    }
    (company, table_name, app_id)
}

/// Primary collation weights of a string, approximating what .NET's culture-aware compare
/// (ICU root collation) gives for the identifiers BC uses: punctuation first in the CLDR
/// order, then digits, then letters with case and accents ignored. ASCII is exact; Latin-1
/// accented letters fold to their base letter; anything else sorts after the ASCII letters
/// by code point. A tie falls to accents, then case (lower before upper), then code points.
fn primary_weights(s: &str, out: &mut Vec<u32>) {
    const SYMBOLS: &str = " _-,;:!?.'\"()[]{}@*/\\&#%`^+<=>|~$";
    for c in s.chars() {
        let lower = c.to_lowercase().next().unwrap_or(c);
        if let Some(i) = SYMBOLS.find(c) {
            out.push(10 + i as u32);
        } else if lower.is_ascii_digit() {
            out.push(100 + lower as u32 - '0' as u32);
        } else if lower.is_ascii_lowercase() {
            out.push(200 + (lower as u32 - 'a' as u32) * 2);
        } else {
            match lower {
                'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => out.push(200),
                'æ' => out.extend([200, 200 + 4 * 2]),
                'ç' => out.push(200 + 2 * 2),
                'è' | 'é' | 'ê' | 'ë' => out.push(200 + 4 * 2),
                'ì' | 'í' | 'î' | 'ï' => out.push(200 + 8 * 2),
                'ñ' => out.push(200 + 13 * 2),
                'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' => out.push(200 + 14 * 2),
                'ù' | 'ú' | 'û' | 'ü' => out.push(200 + 20 * 2),
                'ý' | 'ÿ' => out.push(200 + 24 * 2),
                'ß' => out.extend([200 + 18 * 2, 200 + 18 * 2]),
                _ => out.push(1000 + lower as u32),
            }
        }
    }
}

/// A sort key whose ordering is [`collate_cmp`]: primary weights, then which letters are
/// accented or special, then which are upper case, then the text itself.
type CollationKey = (Vec<u32>, Vec<bool>, Vec<bool>, String);

fn collation_key(s: &str) -> CollationKey {
    let mut primary = Vec::with_capacity(s.len());
    primary_weights(s, &mut primary);
    (
        primary,
        s.chars().map(|c| !c.is_ascii()).collect(), // secondary: an accented or special letter sorts after its plain form
        s.chars().map(|c| c.is_uppercase()).collect(), // tertiary: lower case before upper case
        s.to_string(),
    )
}

/// Culture-aware string order, as bcdb's `OrderBy(t => t.Company)` sorts.
pub fn collate_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    collation_key(a).cmp(&collation_key(b))
}

/// Sort the way `tables` lists them: company (none first), then table name, culture-aware.
/// Keys are built once per table; comparing two names allocates nothing.
pub fn sort_tables(tables: &mut [BcTable]) {
    tables.sort_by_cached_key(|t| (t.company.as_deref().map(collation_key), collation_key(&t.table_name)));
}

pub fn bc_tables(src: &dyn BcSource) -> Result<Vec<BcTable>> {
    Ok(src
        .tables()?
        .iter()
        .map(|t| {
            let (company, table_name, app_id) = parse_table_name(&t.name);
            BcTable {
                table: t.clone(),
                company,
                table_name,
                app_id,
            }
        })
        .collect())
}

/// The companies: the distinct company segments of per-company table names, ordinal order.
pub fn companies(src: &dyn BcSource) -> Result<Vec<String>> {
    let mut v: Vec<String> = bc_tables(src)?
        .into_iter()
        .filter_map(|t| t.company)
        .filter(|c| !c.is_empty())
        .collect();
    v.sort();
    v.dedup();
    Ok(v)
}

pub fn strip_ext(table_name: &str) -> &str {
    table_name.strip_suffix("$ext").unwrap_or(table_name)
}

fn starts_with_ci(s: &str, prefix: &str) -> bool {
    s.to_uppercase().starts_with(&prefix.to_uppercase())
}

fn distinct<T: PartialEq + Clone>(it: impl Iterator<Item = T>) -> Vec<T> {
    let mut v: Vec<T> = Vec::new();
    for x in it {
        if !v.contains(&x) {
            v.push(x);
        }
    }
    v
}

/// What a caller names a table by, shared by `--table` and a serve request's keys.
#[derive(Debug, Clone, Default)]
pub struct TableSelector {
    pub table: Option<String>,
    pub company: Option<String>,
    /// an app-id prefix
    pub app: Option<String>,
}

pub fn resolve_table(src: &dyn BcSource, sel: &TableSelector) -> Result<BcTable> {
    let want = sel
        .table
        .as_deref()
        .ok_or_else(|| Error::Argument("--table is required".into()))?;
    let norm = normalize_sql_name(want);
    let mut matches: Vec<BcTable> = bc_tables(src)?
        .into_iter()
        .filter(|t| eq_ci(t.sql_name(), want) || eq_ci(&t.table_name, &norm))
        .collect();
    if let Some(comp) = &sel.company {
        matches.retain(|t| t.company.as_deref().is_some_and(|c| starts_with_ci(c, comp)));
    }
    // Two installed apps may define the same table name in the same company (legal via AL
    // namespaces; Microsoft's own demo database ships Dimension Set Entry twice): the app id
    // is the only distinguishing part, selectable by prefix.
    if let Some(app) = &sel.app {
        matches.retain(|t| t.app_id.as_deref().is_some_and(|a| starts_with_ci(a, app)));
    }
    if matches.is_empty() {
        return Err(Error::Argument(format!("no table matches '{want}'")));
    }
    if distinct(matches.iter().map(|m| m.company.clone())).len() > 1 {
        let mut with_rows = Vec::new();
        for m in &matches {
            if src.row_count(&m.table)? > 0 {
                with_rows.push(m.clone());
            }
        }
        if distinct(with_rows.iter().map(|m| m.company.clone())).len() == 1 {
            matches = with_rows;
        } else {
            let list = distinct(matches.iter().map(|m| m.company.clone()))
                .iter()
                .map(|c| c.clone().unwrap_or_default())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(Error::Argument(format!(
                "table '{want}' exists in multiple companies ({list}) — use --company"
            )));
        }
    }
    if matches.len() > 1 {
        let hint = if distinct(matches.iter().map(|m| m.app_id.clone())).len() > 1 {
            " — use --app <app-id-prefix> to select the defining app"
        } else {
            ""
        };
        let names = matches
            .iter()
            .map(|m| m.sql_name().to_string())
            .collect::<Vec<_>>()
            .join(" | ");
        return Err(Error::Argument(format!("ambiguous table '{want}': {names}{hint}")));
    }
    Ok(matches.remove(0))
}

/// Companion-table column name `<Field>$<extending app id>` split, or `None`.
pub fn split_ext_column(name: &str) -> Option<(&str, &str)> {
    let i = name.rfind('$')?;
    (i > 0 && is_guid(&name[i + 1..])).then(|| (&name[..i], &name[i + 1..]))
}

/// An extension field stored as a column: with the AL field its extending app's
/// tableextension symbols define, when symbols resolve it.
#[derive(Debug, Clone)]
pub struct ExtColumn<'s> {
    pub column: SysColumn,
    pub ext_app_id: String,
    pub ext: Option<&'s AlTableExtension>,
    pub field: Option<&'s AlField>,
}

/// The `$ext` companion of a base table (or `None`), and its extension-field columns.
pub fn extension_columns<'s>(
    src: &dyn BcSource,
    sym: Option<&'s SymbolStore>,
    t: &BcTable,
) -> Result<(Option<BcTable>, Vec<ExtColumn<'s>>)> {
    extension_columns_in(src, sym, &bc_tables(src)?, t)
}

/// [`extension_columns`] over a table list the caller already parsed (parsing every table name
/// of a backup costs milliseconds; a caller that reads many tables parses once).
pub fn extension_columns_in<'s>(
    src: &dyn BcSource,
    sym: Option<&'s SymbolStore>,
    tables: &[BcTable],
    t: &BcTable,
) -> Result<(Option<BcTable>, Vec<ExtColumn<'s>>)> {
    let want = format!("{}$ext", t.table_name);
    let companion = tables
        .iter()
        .find(|x| x.company == t.company && x.app_id == t.app_id && eq_ci(&x.table_name, &want))
        .cloned();
    let mut out = Vec::new();
    let Some(c) = companion else { return Ok((None, out)) };
    for col in src.columns(&c.table)? {
        let Some((base, app)) = split_ext_column(&col.name) else {
            continue;
        }; // base-key mirror / timestamp
        let hit = sym.and_then(|s| s.find_extension_field(app, strip_ext(&t.table_name), t.app_id.as_deref(), base));
        out.push(ExtColumn {
            ext_app_id: app.to_string(),
            ext: hit.map(|h| h.0),
            field: hit.map(|h| h.1),
            column: col,
        });
    }
    Ok((Some(c), out))
}

/// Extension fields stored as columns of the base table itself (`<Field>$<extending app
/// id>`), the BC 29 layout: there is no `$ext` companion.
pub fn base_extension_columns<'s>(
    src: &dyn BcSource,
    sym: Option<&'s SymbolStore>,
    t: &BcTable,
) -> Result<Vec<ExtColumn<'s>>> {
    let mut out = Vec::new();
    for col in src.columns(&t.table)? {
        let Some((base, app)) = split_ext_column(&col.name) else {
            continue;
        };
        let hit = sym.and_then(|s| s.find_extension_field(app, strip_ext(&t.table_name), t.app_id.as_deref(), base));
        out.push(ExtColumn {
            ext_app_id: app.to_string(),
            ext: hit.map(|h| h.0),
            field: hit.map(|h| h.1),
            column: col,
        });
    }
    Ok(out)
}

/// `(max)` / `(n)` / `(p,s)` after a SQL type name.
pub fn length_suffix(c: &SysColumn) -> String {
    match c.xtype {
        231 | 239 => {
            if c.max_length < 0 {
                "(max)".into()
            } else {
                format!("({})", c.max_length / 2)
            }
        }
        167 | 175 | 165 | 173 => {
            if c.max_length < 0 {
                "(max)".into()
            } else {
                format!("({})", c.max_length)
            }
        }
        106 | 108 => format!("({},{})", c.precision, c.scale),
        _ => String::new(),
    }
}

pub fn sql_type_text(c: &SysColumn) -> String {
    format!("{}{}", type_name(c.xtype), length_suffix(c))
}

// ---- describe ---------------------------------------------------------------------------

pub struct DescField {
    pub field: AlField,
    pub sql_column: Option<String>,
    pub sql_type: Option<String>,
}

pub struct DescExt {
    pub column: String,
    pub sql_type: String,
    pub ext_app_id: String,
    pub ext_name: Option<String>,
    pub ext_app_guid: Option<String>,
    pub ext_app_name: Option<String>,
    pub field: Option<AlField>,
}

pub struct Describe {
    pub al: AlTable,
    pub sql_object: String,
    pub fields: Vec<DescField>,
    pub ext_fields: Vec<DescExt>,
    pub companion: Option<String>,
    /// `$`-prefixed and `timestamp` columns: (name, sql type text)
    pub system_columns: Vec<(String, String)>,
}

/// The AL schema of one table: field ids, AL names and types, and the SQL columns they map to.
pub fn describe(src: &dyn BcSource, sym: &SymbolStore, sel: &TableSelector) -> Result<Describe> {
    let t = resolve_table(src, sel)?;
    let al = sym
        .find_for_sql_table(strip_ext(&t.table_name), t.app_id.as_deref())
        .ok_or_else(|| {
            Error::Argument(format!(
                "table '{}' (app {}) is not defined in the provided symbols — pass the app that defines it",
                t.table_name,
                t.app_id.as_deref().unwrap_or("-")
            ))
        })?;
    let cols = src.columns(&t.table)?;
    let mut fields = Vec::new();
    for f in &al.fields {
        if f.field_class != "Normal" {
            fields.push(DescField {
                field: f.clone(),
                sql_column: None,
                sql_type: None,
            });
            continue;
        }
        let norm = normalize_sql_name(&f.name);
        let sql = cols.iter().find(|c| eq_ci(&c.name, &norm));
        fields.push(DescField {
            field: f.clone(),
            sql_column: sql.map(|c| c.name.clone()),
            sql_type: sql.map(sql_type_text),
        });
    }
    let (companion, comp_cols) = extension_columns(src, Some(sym), &t)?;
    let mut ext_fields = Vec::new();
    for e in base_extension_columns(src, Some(sym), &t)?.into_iter().chain(comp_cols) {
        ext_fields.push(DescExt {
            sql_type: sql_type_text(&e.column),
            column: e.column.name.clone(),
            ext_app_id: e.ext_app_id.clone(),
            ext_name: e.ext.map(|x| x.name.clone()),
            ext_app_guid: e.ext.map(|x| x.app_id.clone()),
            ext_app_name: e.ext.map(|x| x.app_name.clone()),
            field: e.field.cloned(),
        });
    }
    let system_columns = cols
        .iter()
        .filter(|c| c.name.starts_with('$') || c.name == "timestamp")
        .map(|c| (c.name.clone(), sql_type_text(c)))
        .collect();
    Ok(Describe {
        al: al.clone(),
        sql_object: t.sql_name().to_string(),
        fields,
        ext_fields,
        companion: companion.map(|c| c.sql_name().to_string()),
        system_columns,
    })
}

// ---- reading -------------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct ReadRequest {
    pub sel: TableSelector,
    /// the raw `top` text, checked where both the command line and serve meet
    pub top: Option<String>,
    pub select: Option<String>,
    pub sha256: Option<String>,
    pub merge_extensions: bool,
}

struct Avail {
    col: SysColumn,
    from_ext: bool,
    header: String,
}

/// Resolves one select / sha256 name to a column, by SQL column name or AL field name.
/// The name is matched as written first and only then trimmed: BC turns an AL field name
/// carrying a leading or trailing space into a SQL column with that space (observed:
/// "Reten_ Pol_ Filtering "), and trimming first would make such a column unaddressable,
/// while the trim itself is what lets `--select "A, B"` work.
fn resolve_column<'a>(name: &str, pool: &'a [Avail], what: &str) -> Result<&'a Avail> {
    let find = |token: &str| -> Vec<&'a Avail> {
        let n = normalize_sql_name(token);
        pool.iter()
            .filter(|a| eq_ci(&a.col.name, &n) || eq_ci(&normalize_sql_name(&a.header), &n))
            .collect()
    };
    let mut hits = find(name);
    if hits.is_empty() && name != name.trim() {
        hits = find(name.trim());
    }
    match hits.len() {
        0 => Err(Error::Argument(format!(
            "{what}column '{name}' not found; available: {}",
            pool.iter().map(|a| a.col.name.as_str()).collect::<Vec<_>>().join(", ")
        ))),
        1 => Ok(hits[0]),
        _ => Err(Error::Argument(format!(
            "{what}column '{name}' is ambiguous: {} — use the full SQL column name",
            hits.iter().map(|h| h.col.name.as_str()).collect::<Vec<_>>().join(" | ")
        ))),
    }
}

/// The BC major whose databases keep extension fields in the base table's own columns.
const BASE_TABLE_EXTENSION_MAJOR: &str = "29";

/// A read, resolved: the headers are known before any row is.
pub struct ReadPlan {
    pub headers: Vec<String>,
    /// Set when `--merge-extensions` was asked for and had nothing to merge.
    pub note: Option<String>,
    top: usize,
    base: SourceTable,
    base_wanted: Vec<SysColumn>,
    companion: Option<CompanionPlan>,
    selected: Vec<Selected>,
}

struct CompanionPlan {
    table: SourceTable,
    wanted: Vec<SysColumn>,
    /// join key positions in `wanted` / in `base_wanted`
    key_in_comp: Vec<usize>,
    key_in_base: Vec<usize>,
    needs_rows: bool,
}

struct Selected {
    name: String,
    from_ext: bool,
    /// position in the base row or the companion row
    pos: usize,
    sha: bool,
}

fn position_ci(cols: &[SysColumn], name: &str) -> Option<usize> {
    cols.iter().position(|c| eq_ci(&c.name, name))
}

/// The columns with the first of each name kept (names compared as [`eq_ci`] does).
fn distinct_by_name(cols: impl Iterator<Item = SysColumn>) -> Vec<SysColumn> {
    let mut out: Vec<SysColumn> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for c in cols {
        if seen.insert(ci_key(&c.name)) {
            out.push(c);
        }
    }
    out
}

/// [`position_ci`] for many names over one list: each name's first position.
fn positions_ci(cols: &[SysColumn]) -> HashMap<String, usize> {
    let mut out = HashMap::with_capacity(cols.len());
    for (i, c) in cols.iter().enumerate() {
        out.entry(ci_key(&c.name)).or_insert(i);
    }
    out
}

pub fn plan_read(src: &dyn BcSource, sym: Option<&SymbolStore>, req: &ReadRequest) -> Result<ReadPlan> {
    let t = resolve_table(src, &req.sel)?;
    plan_read_table(src, sym, &t, req)
}

/// [`plan_read`] for a table already in hand (`req.sel` is not consulted): a caller that
/// walks every table does not need to find each one by name again.
pub fn plan_read_table(
    src: &dyn BcSource,
    sym: Option<&SymbolStore>,
    t: &BcTable,
    req: &ReadRequest,
) -> Result<ReadPlan> {
    plan_read_table_in(src, sym, &bc_tables(src)?, t, req)
}

/// [`plan_read_table`] over a table list the caller already parsed.
pub fn plan_read_table_in(
    src: &dyn BcSource,
    sym: Option<&SymbolStore>,
    tables: &[BcTable],
    t: &BcTable,
    req: &ReadRequest,
) -> Result<ReadPlan> {
    let t = t.clone();
    let al_table = sym.and_then(|s| s.find_for_sql_table(strip_ext(&t.table_name), t.app_id.as_deref()));
    if sym.is_some() && al_table.is_none() {
        return Err(Error::Argument(format!(
            "table '{}' (app {}) is not defined in the provided symbols — pass the app that defines it",
            t.table_name,
            t.app_id.as_deref().unwrap_or("-")
        )));
    }
    let cols = src.columns(&t.table)?;

    // --merge-extensions: one AL record = base row + $ext companion row, joined on the
    // companion's own key. Not the base table's clustered key: BC keys a companion on its
    // base table's AL primary key, while the base table is clustered on whichever key
    // carries Clustered = 1 (bcdb PROVENANCE.md "Extension companion join key"). A base row
    // without a companion row reads its extension fields as NULL.
    let mut companion: Option<BcTable> = None;
    let mut ext_cols: Vec<ExtColumn> = Vec::new();
    let mut key_cols: Vec<SysColumn> = Vec::new(); // the join key as base-table columns
    let mut comp_key: Vec<SysColumn> = Vec::new(); // the same key as companion columns, same order
    if req.merge_extensions {
        let (c, e) = extension_columns_in(src, sym, tables, &t)?;
        companion = c;
        ext_cols = e;
        if let Some(c) = &companion {
            let comp_cols = src.columns(&c.table)?;
            let key = src.row_key_columns(&c.table)?;
            if key.is_empty() {
                return Err(crate::invalid!(
                    "--merge-extensions: companion {} has no key to join it to {} on — refusing to guess",
                    c.sql_name(),
                    t.sql_name()
                ));
            }
            for n in key {
                comp_key.push(
                    comp_cols.iter().find(|c| eq_ci(&c.name, &n)).cloned().ok_or_else(|| {
                        crate::invalid!("key column {n} of {} is not among its columns", c.sql_name())
                    })?,
                );
                key_cols.push(cols.iter().find(|c| eq_ci(&c.name, &n)).cloned().ok_or_else(|| {
                    crate::invalid!(
                        "companion {} is keyed on {n}, which base table {} does not have — cannot join, refusing to guess",
                        c.sql_name(),
                        t.sql_name()
                    )
                })?);
            }
        }
    }

    // A selectable column: from the base table or the companion, with its AL header.
    // Base-table "<Field>$<app id>" columns (BC 29) get their AL name from the extending
    // app's tableextension symbols, like companion columns do.
    let base_ext: HashMap<String, Option<String>> = base_extension_columns(src, sym, &t)?
        .into_iter()
        .map(|e| (e.column.name.to_uppercase(), e.field.map(|f| f.name.clone())))
        .collect();
    let mut all: Vec<Avail> = Vec::new();
    for c in &cols {
        let header = al_table
            .and_then(|al| {
                al.fields
                    .iter()
                    .find(|f| f.field_class == "Normal" && eq_ci(&normalize_sql_name(&f.name), &c.name))
                    .map(|f| f.name.clone())
            })
            .or_else(|| base_ext.get(&c.name.to_uppercase()).cloned().flatten())
            .unwrap_or_else(|| c.name.clone());
        all.push(Avail {
            col: c.clone(),
            from_ext: false,
            header,
        });
    }
    for e in &ext_cols {
        all.push(Avail {
            col: e.column.clone(),
            from_ext: true,
            header: e.field.map(|f| f.name.clone()).unwrap_or_else(|| e.column.name.clone()),
        });
    }

    let selected_av: Vec<&Avail> = match &req.select {
        Some(sel) => sel
            .split(',')
            .map(|n| resolve_column(n, &all, ""))
            .collect::<Result<_>>()?,
        None => all.iter().collect(),
    };
    let headers: Vec<String> = selected_av.iter().map(|s| s.header.clone()).collect();

    // Checked here rather than at the front ends because a serve request carries "top" too.
    let top = match &req.top {
        None => usize::MAX,
        Some(ts) => match ts.trim().trim_start_matches('+').parse::<i64>() {
            Ok(n) if n >= 0 && !ts.trim().starts_with("++") => n as usize,
            _ => {
                return Err(Error::Argument(format!(
                    "top expects a whole number of rows, got '{ts}'"
                )));
            }
        },
    };

    // --sha256 "A,B": replace those columns' binary values by "sha256:<hex>". A name that
    // matches no selected column is refused: silently doing nothing would return the raw
    // value where a hash was asked for.
    let mut sha_cols: HashSet<String> = HashSet::new();
    if let Some(sh) = &req.sha256 {
        let pool: Vec<Avail> = selected_av
            .iter()
            .map(|a| Avail {
                col: a.col.clone(),
                from_ext: a.from_ext,
                header: a.header.clone(),
            })
            .collect();
        for n in sh.split(',') {
            sha_cols.insert(resolve_column(n, &pool, "--sha256 ")?.col.name.clone());
        }
    }

    // Only the columns the answer needs are decoded: the selected base columns plus, when
    // joining a companion, the join key. Selecting one column of a table with blobs must
    // not pay for the blobs.
    let base_wanted = distinct_by_name(
        selected_av
            .iter()
            .filter(|s| !s.from_ext)
            .map(|s| s.col.clone())
            .chain(if companion.is_some() { key_cols.clone() } else { vec![] }),
    );
    let comp_plan = companion.as_ref().map(|c| {
        let comp_wanted = distinct_by_name(
            comp_key
                .iter()
                .cloned()
                .chain(selected_av.iter().filter(|s| s.from_ext).map(|s| s.col.clone())),
        );
        CompanionPlan {
            table: c.table.clone(),
            key_in_comp: comp_key
                .iter()
                .map(|k| position_ci(&comp_wanted, &k.name).unwrap())
                .collect(),
            key_in_base: key_cols
                .iter()
                .map(|k| position_ci(&base_wanted, &k.name).unwrap())
                .collect(),
            needs_rows: selected_av.iter().any(|s| s.from_ext),
            wanted: comp_wanted,
        }
    });
    let mut selected = Vec::new();
    let base_pos = positions_ci(&base_wanted);
    let comp_pos = comp_plan.as_ref().map(|c| positions_ci(&c.wanted));
    for s in &selected_av {
        let pool = if s.from_ext {
            comp_pos.as_ref().expect("ext column without companion")
        } else {
            &base_pos
        };
        selected.push(Selected {
            name: s.col.name.clone(),
            from_ext: s.from_ext,
            pos: *pool
                .get(&ci_key(&s.col.name))
                .expect("selected column was added to the wanted set"),
            sha: sha_cols.contains(&s.col.name),
        });
    }

    // --merge-extensions on a table with no companion did nothing; say so, so "nothing to
    // merge" is distinguishable from "merge not applied".
    let note = (req.merge_extensions && companion.is_none()).then(|| {
        if !base_ext.is_empty() {
            // The wording is bcdb's, byte for byte (the parity tests compare it). It names the
            // layout by the BC major that introduced it; that is text in a message, not a
            // version test, so it is passed in rather than spelled in the literal.
            format!("--merge-extensions: {} has no $ext companion; its extension fields are columns of the base table (BC {} layout) and are returned without merging", t.sql_name(), BASE_TABLE_EXTENSION_MAJOR)
        } else {
            format!("--merge-extensions: {} has no $ext companion and no extension columns; nothing to merge", t.sql_name())
        }
    });
    Ok(ReadPlan {
        headers,
        note,
        top,
        base: t.table.clone(),
        base_wanted,
        companion: comp_plan,
        selected,
    })
}

impl ReadPlan {
    /// Stream the rows. `sink` returns `Ok(false)` to stop.
    pub fn run(&self, src: &dyn BcSource, sink: &mut dyn FnMut(&[Value]) -> Result<bool>) -> Result<()> {
        if self.top == 0 {
            return Ok(());
        }
        // With a companion: read it fully first, keyed by the decoded join-key values.
        let mut ext_rows: Option<HashMap<String, Vec<Value>>> = None;
        if let Some(c) = &self.companion {
            if c.needs_rows {
                let mut m = HashMap::new();
                src.read_rows(&c.table, &c.wanted, &mut |row| {
                    m.insert(join_key(c.key_in_comp.iter().map(|&i| &row[i])), row.to_vec());
                    Ok(true)
                })?;
                ext_rows = Some(m);
            }
        }
        let mut emitted = 0usize;
        let mut out: Vec<Value> = vec![Value::Null; self.selected.len()];
        src.read_rows(&self.base, &self.base_wanted, &mut |row| {
            let ext_row = match (&ext_rows, &self.companion) {
                (Some(m), Some(c)) => m.get(&join_key(c.key_in_base.iter().map(|&i| &row[i]))),
                _ => None,
            };
            for (o, s) in out.iter_mut().zip(&self.selected) {
                let v = if !s.from_ext {
                    &row[s.pos]
                } else {
                    match ext_row {
                        Some(r) => &r[s.pos],
                        None => &Value::Null,
                    }
                };
                *o = if s.sha {
                    match v {
                        Value::Null => Value::Null,
                        Value::Binary(b) => Value::Text(format!("sha256:{}", hex_upper(&Sha256::digest(b)))),
                        _ => {
                            return Err(Error::Argument(format!(
                                "--sha256 column '{}' did not decode to binary data",
                                s.name
                            )));
                        }
                    }
                } else {
                    v.clone()
                };
            }
            emitted += 1;
            Ok(sink(&out)? && emitted < self.top)
        })
    }
}

fn join_key<'a>(vals: impl Iterator<Item = &'a Value>) -> String {
    vals.map(|v| v.to_tsv()).collect::<Vec<_>>().join("\u{1}")
}

fn hex_upper(b: &[u8]) -> String {
    use std::fmt::Write;
    b.iter().fold(String::with_capacity(b.len() * 2), |mut s, x| {
        let _ = write!(s, "{x:02X}");
        s
    })
}
