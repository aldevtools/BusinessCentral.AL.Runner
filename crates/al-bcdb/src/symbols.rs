//! The AL meaning layer (port of `Symbols.cs`): table and field metadata from the
//! `SymbolReference.json` inside Business Central `.app` packages. This is the schema
//! *input*: point the reader at the apps a database was built from and output uses AL
//! table/field names and AL types instead of SQL ones.
//!
//! Accepted inputs: the JSON itself, a NAVX `.app` package (zip content after the 40-byte
//! NAVX header), or the runtime wrapper `.app` Microsoft ships in BC artifacts (a zip whose
//! payload contains the inner NAVX `.app`). Tables and table extensions may be nested in
//! namespaces (AL namespaces, BC 24+); the loader walks them recursively.

use crate::error::{Error, Result};
use crate::zip::ZipIndex;
use serde::Deserialize;
use std::io::Read;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlField {
    pub id: i32,
    pub name: String,
    pub type_name: String,
    pub field_class: String,
}

#[derive(Debug, Clone)]
pub struct AlTable {
    pub id: i32,
    pub name: String,
    pub app_id: String,
    pub app_name: String,
    pub fields: Vec<AlField>,
}

/// A tableextension. `target_app_id` is the extended table's defining app when the
/// reference is qualified (`#<32-hex app id>#Name`, observed in the shipped Base
/// Application symbols), else `None`.
#[derive(Debug, Clone)]
pub struct AlTableExtension {
    pub id: i32,
    pub name: String,
    pub target_table: String,
    pub target_app_id: Option<String>,
    pub app_id: String,
    pub app_name: String,
    pub fields: Vec<AlField>,
}

#[derive(Default)]
pub struct SymbolStore {
    pub tables: Vec<AlTable>,
    pub table_extensions: Vec<AlTableExtension>,
}

// ---- the JSON shape --------------------------------------------------------------------

/// A JSON value kept when it is a string and skipped otherwise. A property's value can be
/// anything; reading it into a generic value tree first was a fifth of loading the Base
/// Application's 58 MB of symbols.
struct StrOrOther(Option<String>);

impl<'de> Deserialize<'de> for StrOrOther {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        use serde::de::{IgnoredAny, MapAccess, SeqAccess, Visitor};
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = StrOrOther;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("any JSON value")
            }
            fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E> {
                Ok(StrOrOther(Some(v.to_string())))
            }
            fn visit_string<E>(self, v: String) -> std::result::Result<Self::Value, E> {
                Ok(StrOrOther(Some(v)))
            }
            fn visit_bool<E>(self, _: bool) -> std::result::Result<Self::Value, E> {
                Ok(StrOrOther(None))
            }
            fn visit_i64<E>(self, _: i64) -> std::result::Result<Self::Value, E> {
                Ok(StrOrOther(None))
            }
            fn visit_u64<E>(self, _: u64) -> std::result::Result<Self::Value, E> {
                Ok(StrOrOther(None))
            }
            fn visit_f64<E>(self, _: f64) -> std::result::Result<Self::Value, E> {
                Ok(StrOrOther(None))
            }
            fn visit_unit<E>(self) -> std::result::Result<Self::Value, E> {
                Ok(StrOrOther(None))
            }
            fn visit_none<E>(self) -> std::result::Result<Self::Value, E> {
                Ok(StrOrOther(None))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> std::result::Result<Self::Value, A::Error> {
                while a.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
                Ok(StrOrOther(None))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> std::result::Result<Self::Value, A::Error> {
                while a.next_element::<IgnoredAny>()?.is_some() {}
                Ok(StrOrOther(None))
            }
        }
        d.deserialize_any(V)
    }
}

#[derive(Deserialize)]
struct RawProp {
    #[serde(rename = "Name")]
    name: Option<String>,
    #[serde(rename = "Value")]
    value: Option<StrOrOther>,
}

#[derive(Deserialize)]
struct RawSubtype {
    #[serde(rename = "Name")]
    name: Option<String>,
}

#[derive(Deserialize)]
struct RawTypeDef {
    #[serde(rename = "Name")]
    name: Option<String>,
    #[serde(rename = "Subtype")]
    subtype: Option<RawSubtype>,
}

#[derive(Deserialize)]
struct RawField {
    #[serde(rename = "Id")]
    id: Option<i32>,
    #[serde(rename = "Name")]
    name: Option<String>,
    #[serde(rename = "TypeDefinition")]
    type_def: Option<RawTypeDef>,
    #[serde(rename = "Properties")]
    properties: Option<Vec<RawProp>>,
}

#[derive(Deserialize)]
struct RawTable {
    #[serde(rename = "Id")]
    id: Option<i32>,
    #[serde(rename = "Name")]
    name: Option<String>,
    #[serde(rename = "Fields")]
    fields: Option<Vec<RawField>>,
}

#[derive(Deserialize)]
struct RawExt {
    #[serde(rename = "Id")]
    id: Option<i32>,
    #[serde(rename = "Name")]
    name: Option<String>,
    #[serde(rename = "TargetObject")]
    target: Option<String>,
    #[serde(rename = "Fields")]
    fields: Option<Vec<RawField>>,
}

#[derive(Deserialize)]
struct RawNs {
    #[serde(rename = "Tables")]
    tables: Option<Vec<RawTable>>,
    #[serde(rename = "TableExtensions")]
    table_extensions: Option<Vec<RawExt>>,
    #[serde(rename = "Namespaces")]
    namespaces: Option<Vec<RawNs>>,
}

/// The root has the same shape as a namespace plus the app's id and name. Not
/// `#[serde(flatten)]`: flatten reads the whole object into a generic tree first, which was
/// most of the time spent on the Base Application's 58 MB of symbols.
#[derive(Deserialize)]
struct RawRoot {
    #[serde(rename = "AppId")]
    app_id: Option<String>,
    #[serde(rename = "Name")]
    name: Option<String>,
    #[serde(rename = "Tables")]
    tables: Option<Vec<RawTable>>,
    #[serde(rename = "TableExtensions")]
    table_extensions: Option<Vec<RawExt>>,
    #[serde(rename = "Namespaces")]
    namespaces: Option<Vec<RawNs>>,
}

fn parse_fields(fields: Option<Vec<RawField>>, origin: &str) -> Result<Vec<AlField>> {
    let mut out = Vec::new();
    for f in fields.unwrap_or_default() {
        let mut type_name = "?".to_string();
        if let Some(td) = f.type_def {
            type_name = td.name.clone().unwrap_or_else(|| "?".into());
            if type_name == "Enum" {
                if let Some(n) = td.subtype.and_then(|s| s.name) {
                    type_name = format!("Enum \"{n}\"");
                }
            }
        }
        let mut field_class = "Normal".to_string();
        for p in f.properties.unwrap_or_default() {
            if p.name.as_deref() == Some("FieldClass") {
                if let Some(StrOrOther(Some(s))) = p.value {
                    field_class = s;
                }
            }
        }
        out.push(AlField {
            id: f.id.unwrap_or(0),
            name: f.name.ok_or_else(|| crate::invalid!("{origin}: a field has no Name"))?,
            type_name,
            field_class,
        });
    }
    Ok(out)
}

impl SymbolStore {
    /// Load symbols from `(origin, bytes)` pairs: each a `.json` file or an `.app` package.
    pub fn load<'a>(inputs: impl IntoIterator<Item = (&'a str, Vec<u8>)>) -> Result<SymbolStore> {
        let mut store = SymbolStore::default();
        for (origin, bytes) in inputs {
            store.load_one(origin, bytes)?;
        }
        Ok(store)
    }

    fn load_one(&mut self, origin: &str, bytes: Vec<u8>) -> Result<()> {
        let json = if origin.to_ascii_lowercase().ends_with(".json") {
            bytes
        } else {
            extract_symbol_reference(bytes, origin, 0)?
        };
        let json: &[u8] = if json.starts_with(&[0xEF, 0xBB, 0xBF]) {
            &json[3..]
        } else {
            &json
        };
        let root: RawRoot = serde_json::from_slice(json)
            .map_err(|e| crate::invalid!("{origin}: SymbolReference.json is not valid: {e}"))?;
        let app_id = root.app_id.unwrap_or_default();
        let app_name = root.name.unwrap_or_default();
        if app_id.is_empty() {
            return Err(crate::invalid!("{origin}: SymbolReference.json has no AppId"));
        }
        let ns = RawNs {
            tables: root.tables,
            table_extensions: root.table_extensions,
            namespaces: root.namespaces,
        };
        self.walk(ns, &app_id, &app_name, origin)
    }

    fn walk(&mut self, ns: RawNs, app_id: &str, app_name: &str, origin: &str) -> Result<()> {
        for t in ns.tables.unwrap_or_default() {
            self.tables.push(AlTable {
                id: t.id.unwrap_or(0),
                name: t.name.ok_or_else(|| crate::invalid!("{origin}: a table has no Name"))?,
                app_id: app_id.to_string(),
                app_name: app_name.to_string(),
                fields: parse_fields(t.fields, origin)?,
            });
        }
        for e in ns.table_extensions.unwrap_or_default() {
            let mut target = e
                .target
                .ok_or_else(|| crate::invalid!("{origin}: a tableextension has no TargetObject"))?;
            let mut target_app_id = None;
            // qualified form "#<32-hex app id>#Name" (observed in shipped Base Application symbols)
            let tb = target.as_bytes();
            if tb.len() > 34 && tb[0] == b'#' && tb[33] == b'#' {
                target_app_id =
                    Some(guid_from_32_hex(&target[1..33]).ok_or_else(|| {
                        crate::invalid!("{origin}: target app id '{}' is not a GUID", &target[1..33])
                    })?);
                target = target[34..].to_string();
            }
            self.table_extensions.push(AlTableExtension {
                id: e.id.unwrap_or(0),
                name: e
                    .name
                    .ok_or_else(|| crate::invalid!("{origin}: a tableextension has no Name"))?,
                target_table: target,
                target_app_id,
                app_id: app_id.to_string(),
                app_name: app_name.to_string(),
                fields: parse_fields(e.fields, origin)?,
            });
        }
        for sub in ns.namespaces.unwrap_or_default() {
            self.walk(sub, app_id, app_name, origin)?;
        }
        Ok(())
    }

    /// The field a tableextension adds to a base table, addressed the way its SQL
    /// companion-table column is: the extending app id (from the column's
    /// `<Field>$<app id>` suffix), the base table's AL name and defining app, and the
    /// SQL-normalized field name.
    pub fn find_extension_field(
        &self,
        ext_app_id: &str,
        target_table_normalized: &str,
        target_app_id: Option<&str>,
        field_sql_name: &str,
    ) -> Option<(&AlTableExtension, &AlField)> {
        for ext in &self.table_extensions {
            if !ext.app_id.eq_ignore_ascii_case(ext_app_id) {
                continue;
            }
            if !name_compat(&ext.target_table, target_table_normalized) {
                continue;
            }
            if let (Some(a), Some(b)) = (&ext.target_app_id, target_app_id) {
                if !a.eq_ignore_ascii_case(b) {
                    continue;
                }
            }
            if let Some(f) = ext
                .fields
                .iter()
                .find(|f| f.field_class == "Normal" && eq_ci(&normalize_sql_name(&f.name), field_sql_name))
            {
                return Some((ext, f));
            }
        }
        None
    }

    /// All symbol tables whose AL name matches (several apps can define same-named tables).
    pub fn find(&self, al_name: &str) -> Vec<&AlTable> {
        self.tables.iter().filter(|t| eq_ci(&t.name, al_name)).collect()
    }

    /// The AL table whose name and app id match a SQL object's parsed name parts.
    pub fn find_for_sql_table(&self, normalized_table_name: &str, app_id: Option<&str>) -> Option<&AlTable> {
        self.tables.iter().find(|t| {
            name_compat(&t.name, normalized_table_name) && app_id.is_none_or(|a| t.app_id.eq_ignore_ascii_case(a))
        })
    }
}

/// Case-insensitive equality (the ordinal-ignore-case comparison bcdb uses).
pub fn eq_ci(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b) || (!a.is_ascii() || !b.is_ascii()) && a.to_uppercase() == b.to_uppercase()
}

/// A key two names share exactly when [`eq_ci`] holds for them, for a lookup by name: the name upper-cased. Two ASCII
/// names that differ only in ASCII case upper-case alike, and for any other pair `eq_ci` compares the upper-cased forms.
pub fn ci_key(name: &str) -> String {
    if name.is_ascii() {
        name.to_ascii_uppercase()
    } else {
        name.to_uppercase()
    }
}

fn name_compat(al_name: &str, sql_name: &str) -> bool {
    eq_ci(&normalize_sql_name(al_name), sql_name)
}

/// BC's AL-name to SQL-identifier normalization: characters invalid in SQL identifiers are
/// replaced with '_' (observed across all demo-database object/column names).
pub fn normalize_sql_name(al_name: &str) -> String {
    al_name
        .chars()
        .map(|c| {
            if matches!(c, '.' | '"' | '\\' | '/' | '\'' | '%' | '[' | ']') {
                '_'
            } else {
                c
            }
        })
        .collect()
}

/// `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx` from 32 hex digits, lower case (.NET `Guid.ToString()`).
fn guid_from_32_hex(s: &str) -> Option<String> {
    if s.len() != 32 || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let l = s.to_ascii_lowercase();
    Some(format!(
        "{}-{}-{}-{}-{}",
        &l[..8],
        &l[8..12],
        &l[12..16],
        &l[16..20],
        &l[20..]
    ))
}

/// Unwrap .app packaging until SymbolReference.json is found.
fn extract_symbol_reference(mut bytes: Vec<u8>, origin: &str, depth: usize) -> Result<Vec<u8>> {
    if depth > 3 {
        return Err(crate::invalid!(
            "{origin}: nested deeper than any observed .app packaging"
        ));
    }
    // NAVX header: magic "NAVX", u32 header length at +4; the zip follows.
    if bytes.len() > 8 && &bytes[..4] == b"NAVX" {
        let hdr_len = i32::from_le_bytes(bytes[4..8].try_into().unwrap());
        if hdr_len <= 0 || hdr_len as usize >= bytes.len() {
            return Err(crate::invalid!("{origin}: NAVX header length {hdr_len} out of range"));
        }
        bytes.drain(..hdr_len as usize);
    }
    if bytes.len() < 4 || &bytes[..2] != b"PK" {
        return Err(crate::invalid!(
            "{origin}: not a NAVX/zip .app package and not a .json file"
        ));
    }
    let zip = ZipIndex::parse(&bytes)?;
    let base = |n: &str| n.rsplit('/').next().unwrap_or(n).to_string();
    if let Some(sr) = zip
        .entries
        .iter()
        .find(|e| base(&e.name).eq_ignore_ascii_case("SymbolReference.json"))
    {
        let mut out = Vec::with_capacity(sr.size as usize);
        zip.open(&bytes, sr)?.read_to_end(&mut out).map_err(Error::from)?;
        return Ok(out);
    }
    // runtime wrapper: exactly one inner .app entry
    let inner: Vec<_> = zip
        .entries
        .iter()
        .filter(|e| base(&e.name).to_ascii_lowercase().ends_with(".app"))
        .collect();
    if inner.len() == 1 {
        let mut out = Vec::with_capacity(inner[0].size as usize);
        zip.open(&bytes, inner[0])?.read_to_end(&mut out).map_err(Error::from)?;
        let name = base(&inner[0].name);
        return extract_symbol_reference(out, &format!("{origin} -> {name}"), depth + 1);
    }
    Err(crate::invalid!(
        "{origin}: no SymbolReference.json and no single inner .app in the package"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A lookup by [`ci_key`] finds exactly the names [`eq_ci`] calls equal (the backup provider's table index and the
    /// read planner's column sets rely on it).
    #[test]
    fn ci_key_agrees_with_eq_ci() {
        let names = [
            "G_L Entry",
            "g_l entry",
            "G_L ENTRY",
            "G_L Entry$ext",
            "Straße",
            "STRASSE",
            "strasse",
            "Ärger",
            "ärger",
            "ÄRGER",
            "ı",
            "I",
            "i",
            "İ",
            "ſ",
            "s",
            "S",
            "ﬁ",
            "FI",
            "Customer",
            "Customer ",
            "",
        ];
        for a in names {
            for b in names {
                assert_eq!(eq_ci(a, b), ci_key(a) == ci_key(b), "{a:?} and {b:?}");
            }
        }
    }
}
