//! Reading `model.xml`: the tables, their columns in data-stream order, and their primary
//! keys. A real export's `model.xml` is 113 MB, so it is scanned as a stream and never held
//! as a tree: every table is a path of elements (`Element[SqlTable]` / `Relationship[Columns]`
//! / `Entry` / `Element[SqlSimpleColumn]` / ...) and the scanner follows that path with a
//! small stack.
//!
//! Entry points: `ModelScan`, `scan_streaming`, `split_name`.

use crate::bcp::BacpacColumn;
use crate::error::{Error, Result};
use quick_xml::Reader;
use quick_xml::events::Event;
use std::collections::HashMap;
use std::io::BufRead;

/// What `model.xml` says: tables as `(schema, name, columns)` in file order, and the
/// primary-key columns of each `schema.name`.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct ModelScan {
    pub tables: Vec<(String, String, Vec<BacpacColumn>)>,
    pub keys: HashMap<String, Vec<String>>,
}

/// Where the scanner is inside a captured `SqlTable` / `SqlPrimaryKeyConstraint` element.
/// Each variant is a position on the path to something wanted; everything else is `Other`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Ctx {
    Other,
    // SqlTable
    Table,
    ColumnsRel,
    ColumnsEntry,
    Col,
    SpecRel,
    SpecEntry,
    Spec,
    TypeRel,
    TypeEntry,
    // SqlPrimaryKeyConstraint
    Key,
    KeyColsRel,
    KeyColsEntry,
    KeySpec,
    KeyColRel,
    KeyColEntry,
    KeyDefRel,
    KeyDefEntry,
}

fn xml_err(e: impl std::fmt::Display) -> Error {
    crate::invalid!("model.xml: {e}")
}

fn attr(e: &quick_xml::events::BytesStart<'_>, key: &str) -> Result<Option<String>> {
    match e.try_get_attribute(key).map_err(xml_err)? {
        Some(a) => Ok(Some(a.unescape_value().map_err(xml_err)?.into_owned())),
        None => Ok(None),
    }
}

/// The column being read.
#[derive(Default)]
struct ColState {
    name: String,
    nullable_seen: bool,
    nullable: bool,
    spec_taken: bool,
    is_max: bool,
    length: i32,
    precision: u8,
    scale: u8,
    type_name: Option<String>,
}

/// Reads `model.xml` as a stream, keeping only the tables, their simple columns and the
/// primary-key constraints. Equivalent to reading each such element as a tree and querying
/// it (the unit tests hold it to that), without building the tree: a production export has
/// about 196,000 columns, a dozen elements each.
pub(crate) fn scan_streaming<R: BufRead>(reader: &mut Reader<R>) -> Result<ModelScan> {
    let mut out = ModelScan::default();
    let mut buf = Vec::new();
    let mut stack: Vec<Ctx> = Vec::new();
    // the captured element
    let (mut schema, mut table) = (String::new(), String::new());
    let mut cols: Vec<BacpacColumn> = Vec::new();
    let mut col = ColState::default();
    let mut key_cols: Vec<String> = Vec::new();
    let mut key_target: Option<String> = None;
    let mut key_col_taken = false;
    loop {
        buf.clear();
        let ev = reader.read_event_into(&mut buf).map_err(xml_err)?;
        let (e, is_start) = match &ev {
            Event::Start(e) => (e, true),
            Event::Empty(e) => (e, false),
            Event::End(_) => {
                match stack.pop() {
                    Some(Ctx::Col) => {
                        let spec_missing = !col.spec_taken;
                        let type_name = col.type_name.take();
                        let q = || format!("[{schema}].[{table}].[{}]", col.name);
                        if spec_missing {
                            return Err(crate::invalid!("model.xml: column {} has no type specifier", q()));
                        }
                        let Some(type_name) = type_name else {
                            return Err(crate::invalid!("model.xml: column {} names no type", q()));
                        };
                        cols.push(BacpacColumn::from_model(
                            &col.name,
                            &type_name,
                            col.nullable,
                            col.is_max,
                            col.length,
                            col.precision,
                            col.scale,
                        ));
                    }
                    Some(Ctx::KeySpec) => key_col_taken = false,
                    // the captured element itself closed
                    Some(Ctx::Table) if stack.is_empty() => out.tables.push((
                        std::mem::take(&mut schema),
                        std::mem::take(&mut table),
                        std::mem::take(&mut cols),
                    )),
                    Some(Ctx::Key) if stack.is_empty() => {
                        if let (Some(t), false) = (key_target.take(), key_cols.is_empty()) {
                            let (s, n, _) = split_name(&t)?;
                            out.keys.insert(format!("{s}.{n}"), std::mem::take(&mut key_cols));
                        }
                        key_cols.clear();
                    }
                    _ => {}
                }
                continue;
            }
            Event::Eof => return Ok(out),
            _ => continue,
        };
        let name = e.local_name();
        let name = name.as_ref();
        let Some(&parent) = stack.last() else {
            // outside any captured element: only a table or key element starts one
            if name == b"Element" {
                match attr(e, "Type")?.as_deref() {
                    Some("SqlTable") => {
                        let (s, t, _) = split_name(&attr(e, "Name")?.unwrap_or_default())?;
                        (schema, table) = (s, t);
                        cols.clear();
                        if is_start {
                            stack.push(Ctx::Table);
                        } else {
                            out.tables
                                .push((std::mem::take(&mut schema), std::mem::take(&mut table), Vec::new()));
                        }
                    }
                    Some("SqlPrimaryKeyConstraint") if is_start => {
                        key_cols.clear();
                        key_target = None;
                        stack.push(Ctx::Key);
                    }
                    _ => {}
                }
            }
            continue;
        };
        let rel =
            |want: &str| -> Result<bool> { Ok(name == b"Relationship" && attr(e, "Name")?.as_deref() == Some(want)) };
        let next = match parent {
            Ctx::Table if rel("Columns")? => Ctx::ColumnsRel,
            Ctx::ColumnsRel if name == b"Entry" => Ctx::ColumnsEntry,
            Ctx::ColumnsEntry if name == b"Element" => {
                // Computed columns (SqlComputedColumn) are not stored and carry no data.
                if attr(e, "Type")?.as_deref() == Some("SqlSimpleColumn") {
                    let qualified = attr(e, "Name")?.unwrap_or_default();
                    let col_name = split_name(&qualified)?.2.ok_or_else(|| {
                        crate::invalid!("model.xml: column identifier '{qualified}' does not name a column of a table")
                    })?;
                    if !is_start {
                        // a self-closing column has no type specifier
                        return Err(crate::invalid!(
                            "model.xml: column [{schema}].[{table}].[{col_name}] has no type specifier"
                        ));
                    }
                    col = ColState {
                        name: col_name,
                        nullable: true,
                        ..Default::default()
                    };
                    Ctx::Col
                } else {
                    Ctx::Other
                }
            }
            Ctx::Col if name == b"Property" => {
                if !col.nullable_seen && attr(e, "Name")?.as_deref() == Some("IsNullable") {
                    col.nullable_seen = true;
                    col.nullable = attr(e, "Value")?.as_deref() != Some("False");
                }
                Ctx::Other
            }
            Ctx::Col if rel("TypeSpecifier")? => Ctx::SpecRel,
            Ctx::SpecRel if name == b"Entry" => Ctx::SpecEntry,
            Ctx::SpecEntry if name == b"Element" && !col.spec_taken => {
                col.spec_taken = true;
                Ctx::Spec
            }
            Ctx::Spec if name == b"Property" => {
                let value = attr(e, "Value")?;
                let num = |what: &str| -> Result<i64> {
                    value.as_deref().unwrap_or("0").parse::<i64>().map_err(|_| {
                        crate::invalid!(
                            "model.xml: column {}: {what} '{}' is not a number",
                            col.name,
                            value.as_deref().unwrap_or("")
                        )
                    })
                };
                match attr(e, "Name")?.as_deref() {
                    Some("IsMax") => col.is_max = value.as_deref() == Some("True"),
                    Some("Length") => col.length = num("Length")? as i32,
                    Some("Precision") => col.precision = num("Precision")? as u8,
                    Some("Scale") => col.scale = num("Scale")? as u8,
                    _ => {}
                }
                Ctx::Other
            }
            Ctx::Spec if rel("Type")? => Ctx::TypeRel,
            Ctx::TypeRel if name == b"Entry" => Ctx::TypeEntry,
            Ctx::TypeEntry if name == b"References" => {
                if col.type_name.is_none() {
                    col.type_name = attr(e, "Name")?.map(|n| n.trim_matches(|c| c == '[' || c == ']').to_string());
                }
                Ctx::Other
            }
            Ctx::Key if rel("ColumnSpecifications")? => Ctx::KeyColsRel,
            Ctx::Key if rel("DefiningTable")? => Ctx::KeyDefRel,
            Ctx::KeyColsRel if name == b"Entry" => Ctx::KeyColsEntry,
            Ctx::KeyColsEntry if name == b"Element" => {
                key_col_taken = false;
                Ctx::KeySpec
            }
            Ctx::KeySpec if rel("Column")? => Ctx::KeyColRel,
            Ctx::KeyColRel if name == b"Entry" => Ctx::KeyColEntry,
            Ctx::KeyColEntry if name == b"References" => {
                if !key_col_taken {
                    key_col_taken = true;
                    if let Some(n) = attr(e, "Name")? {
                        key_cols.push(split_name(&n)?.2.unwrap_or_default());
                    }
                }
                Ctx::Other
            }
            Ctx::KeyDefRel if name == b"Entry" => Ctx::KeyDefEntry,
            Ctx::KeyDefEntry if name == b"References" => {
                if key_target.is_none() {
                    key_target = attr(e, "Name")?;
                }
                Ctx::Other
            }
            _ => Ctx::Other,
        };
        if is_start {
            stack.push(next);
        }
    }
}

/// Splits a model.xml identifier `[a].[b]` or `[a].[b].[c]` into its bracketed parts.
pub(crate) fn split_name(bracketed: &str) -> Result<(String, String, Option<String>)> {
    let b = bracketed.as_bytes();
    let mut parts: Vec<String> = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] != b'[' {
            return Err(crate::invalid!("model.xml: malformed identifier '{bracketed}'"));
        }
        // find the closing ']' that is not doubled
        let mut j = i + 1;
        loop {
            match bracketed[j..].find(']') {
                None => return Err(crate::invalid!("model.xml: malformed identifier '{bracketed}'")),
                Some(k) => {
                    let end = j + k;
                    if end + 1 < b.len() && b[end + 1] == b']' {
                        j = end + 2;
                        continue;
                    }
                    parts.push(bracketed[i + 1..end].replace("]]", "]"));
                    i = end + 1;
                    break;
                }
            }
        }
        if i < b.len() && b[i] == b'.' {
            i += 1;
        }
    }
    let n = parts.len();
    if n != 2 && n != 3 {
        return Err(crate::invalid!("model.xml: identifier '{bracketed}' has {n} parts"));
    }
    let mut it = parts.into_iter();
    let (p1, p2, p3) = (it.next().unwrap(), it.next().unwrap(), it.next());
    Ok((p1, p2, p3))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::ByteSource;
    use crate::zip::ZipIndex;

    // ---- the reference: the whole subtree of a table as a tree, queried by path ----------

    struct Node {
        name: String,
        attrs: Vec<(String, String)>,
        children: Vec<Node>,
    }

    impl Node {
        fn attr(&self, k: &str) -> Option<&str> {
            self.attrs.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str())
        }
        /// `Relationship[@Name=relationship]/Entry/<leaf>` children: the one shape every
        /// lookup in model.xml has.
        fn under<'a>(&'a self, relationship: &'a str, leaf: &'a str) -> impl Iterator<Item = &'a Node> + 'a {
            self.children
                .iter()
                .filter(move |r| r.name == "Relationship" && r.attr("Name") == Some(relationship))
                .flat_map(|r| r.children.iter().filter(|e| e.name == "Entry"))
                .flat_map(move |e| e.children.iter().filter(move |c| c.name == leaf))
        }
        fn first_under<'a>(&'a self, relationship: &'a str, leaf: &'a str) -> Option<&'a Node> {
            self.under(relationship, leaf).next()
        }
        fn prop(&self, name: &str) -> Option<&str> {
            self.children
                .iter()
                .filter(|c| c.name == "Property")
                .find(|p| p.attr("Name") == Some(name))
                .and_then(|p| p.attr("Value"))
        }
    }

    fn node(e: &quick_xml::events::BytesStart<'_>) -> Node {
        let name = String::from_utf8_lossy(e.local_name().as_ref()).into_owned();
        let attrs = e
            .attributes()
            .map(|a| {
                let a = a.unwrap();
                (
                    String::from_utf8_lossy(a.key.local_name().as_ref()).into_owned(),
                    a.unescape_value().unwrap().into_owned(),
                )
            })
            .collect();
        Node {
            name,
            attrs,
            children: Vec::new(),
        }
    }

    fn subtree<R: BufRead>(reader: &mut Reader<R>, start: Node) -> Node {
        let mut stack = vec![start];
        let mut buf = Vec::new();
        loop {
            buf.clear();
            match reader.read_event_into(&mut buf).unwrap() {
                Event::Start(e) => stack.push(node(&e)),
                Event::Empty(e) => stack.last_mut().unwrap().children.push(node(&e)),
                Event::End(_) => {
                    let done = stack.pop().unwrap();
                    match stack.last_mut() {
                        Some(p) => p.children.push(done),
                        None => return done,
                    }
                }
                Event::Eof => panic!("ends inside an element"),
                _ => {}
            }
        }
    }

    fn dom_table(el: &Node) -> (String, String, Vec<BacpacColumn>) {
        let (schema, table, _) = split_name(el.attr("Name").unwrap()).unwrap();
        let mut cols = Vec::new();
        for col in el.under("Columns", "Element") {
            if col.attr("Type") != Some("SqlSimpleColumn") {
                continue;
            }
            let name = split_name(col.attr("Name").unwrap()).unwrap().2.unwrap();
            let nullable = col.prop("IsNullable") != Some("False");
            let spec = col.first_under("TypeSpecifier", "Element").unwrap();
            let ty = spec
                .first_under("Type", "References")
                .and_then(|r| r.attr("Name"))
                .unwrap()
                .trim_matches(|c| c == '[' || c == ']')
                .to_string();
            let (mut is_max, mut length, mut precision, mut scale) = (false, 0i32, 0u8, 0u8);
            for p in spec.children.iter().filter(|c| c.name == "Property") {
                let v = p.attr("Value");
                match p.attr("Name") {
                    Some("IsMax") => is_max = v == Some("True"),
                    Some("Length") => length = v.unwrap_or("0").parse().unwrap(),
                    Some("Precision") => precision = v.unwrap_or("0").parse().unwrap(),
                    Some("Scale") => scale = v.unwrap_or("0").parse().unwrap(),
                    _ => {}
                }
            }
            cols.push(BacpacColumn::from_model(
                &name, &ty, nullable, is_max, length, precision, scale,
            ));
        }
        (schema, table, cols)
    }

    fn dom_key(el: &Node) -> Option<(String, Vec<String>)> {
        let target = el.first_under("DefiningTable", "References")?.attr("Name")?;
        let (schema, table, _) = split_name(target).unwrap();
        let cols: Vec<String> = el
            .under("ColumnSpecifications", "Element")
            .filter_map(|s| s.first_under("Column", "References")?.attr("Name"))
            .map(|n| split_name(n).unwrap().2.unwrap())
            .collect();
        (!cols.is_empty()).then(|| (format!("{schema}.{table}"), cols))
    }

    fn scan_dom<R: BufRead>(reader: &mut Reader<R>) -> ModelScan {
        let mut out = ModelScan::default();
        let mut buf = Vec::new();
        loop {
            buf.clear();
            match reader.read_event_into(&mut buf).unwrap() {
                Event::Start(e) if e.local_name().as_ref() == b"Element" => {
                    let n = node(&e);
                    match n.attr("Type") {
                        Some("SqlTable") => out.tables.push(dom_table(&subtree(reader, n))),
                        Some("SqlPrimaryKeyConstraint") => {
                            if let Some((k, c)) = dom_key(&subtree(reader, n)) {
                                out.keys.insert(k, c);
                            }
                        }
                        _ => {}
                    }
                }
                Event::Eof => return out,
                _ => {}
            }
        }
    }

    fn fixture_model() -> Vec<u8> {
        let bytes: Vec<u8> = include_bytes!("../tests/fixtures/typeprobe.bacpac").to_vec();
        ZipIndex::parse(&bytes)
            .unwrap()
            .read(&bytes as &dyn ByteSource, "model.xml")
            .unwrap()
    }

    #[test]
    fn the_streaming_scan_finds_what_the_tree_scan_finds() {
        let xml = fixture_model();
        let want = scan_dom(&mut Reader::from_reader(&xml[..]));
        assert!(want.tables.len() > 20 && !want.keys.is_empty());
        let got = scan_streaming(&mut Reader::from_reader(&xml[..])).unwrap();
        assert_eq!(got.tables.len(), want.tables.len());
        for (g, w) in got.tables.iter().zip(&want.tables) {
            assert_eq!(g, w);
        }
        assert_eq!(got.keys, want.keys);
    }

    #[test]
    fn computed_columns_and_odd_shapes_are_handled_like_the_tree_scan() {
        // A computed column is not stored; an element outside any table is ignored; a
        // self-closing property counts; a key without columns is dropped.
        let xml = br#"<?xml version="1.0"?><DataSchemaModel xmlns="http://schemas.microsoft.com/sqlserver/dac/Serialization/2012/02"><Model>
<Element Type="SqlSchema" Name="[dbo]"/>
<Element Type="SqlTable" Name="[dbo].[T$1]">
 <Property Name="IsAnsiNullsOn" Value="True"/>
 <Relationship Name="Columns"><Entry>
  <Element Type="SqlSimpleColumn" Name="[dbo].[T$1].[a]"><Property Name="IsNullable" Value="False"/>
   <Relationship Name="TypeSpecifier"><Entry><Element Type="SqlTypeSpecifier"><Property Name="Length" Value="30"/><Relationship Name="Type"><Entry><References ExternalSource="BuiltIns" Name="[nvarchar]"/></Entry></Relationship></Element></Entry></Relationship>
  </Element></Entry>
  <Entry><Element Type="SqlComputedColumn" Name="[dbo].[T$1].[c]"><Property Name="IsNullable" Value="False"/></Element></Entry>
  <Entry><Element Type="SqlSimpleColumn" Name="[dbo].[T$1].[d&amp;e]">
   <Relationship Name="TypeSpecifier"><Entry><Element Type="SqlTypeSpecifier"><Property Name="Precision" Value="38"/><Property Name="Scale" Value="20"/><Relationship Name="Type"><Entry><References Name="[decimal]"/></Entry></Relationship></Element></Entry></Relationship>
  </Element></Entry>
 </Relationship>
</Element>
<Element Type="SqlPrimaryKeyConstraint" Name="[dbo].[pk_1]"><Relationship Name="DefiningTable"><Entry><References Name="[dbo].[T$1]"/></Entry></Relationship>
 <Relationship Name="ColumnSpecifications"><Entry><Element Type="SqlIndexedColumnSpecification"><Relationship Name="Column"><Entry><References Name="[dbo].[T$1].[a]"/></Entry></Relationship></Element></Entry></Relationship></Element>
<Element Type="SqlPrimaryKeyConstraint" Name="[dbo].[pk_2]"><Relationship Name="DefiningTable"><Entry><References Name="[dbo].[T$1]"/></Entry></Relationship></Element>
</Model></DataSchemaModel>"#;
        let want = scan_dom(&mut Reader::from_reader(&xml[..]));
        let got = scan_streaming(&mut Reader::from_reader(&xml[..])).unwrap();
        assert_eq!(got, want);
        assert_eq!(got.tables[0].2.len(), 2);
        assert_eq!(got.tables[0].2[1].name, "d&e");
        assert_eq!(got.tables[0].2[1].precision, 38);
        assert_eq!(got.keys["dbo.T$1"], vec!["a".to_string()]);
    }

    #[test]
    fn a_column_without_a_type_specifier_is_refused_by_name() {
        let xml = br#"<Model><Element Type="SqlTable" Name="[dbo].[T]"><Relationship Name="Columns"><Entry>
<Element Type="SqlSimpleColumn" Name="[dbo].[T].[a]"/></Entry></Relationship></Element></Model>"#;
        let e = scan_streaming(&mut Reader::from_reader(&xml[..])).unwrap_err();
        assert!(
            e.message().contains("[dbo].[T].[a]") && e.message().contains("no type specifier"),
            "{e}"
        );
    }

    #[test]
    fn identifiers_with_brackets_and_dots_split_into_parts() {
        assert_eq!(
            split_name("[dbo].[a]]b.c]").unwrap(),
            ("dbo".into(), "a]b.c".into(), None)
        );
        assert_eq!(
            split_name("[s].[t].[c d]").unwrap(),
            ("s".into(), "t".into(), Some("c d".into()))
        );
        assert!(split_name("[a]").is_err());
        assert!(split_name("a.b").is_err());
    }
}
