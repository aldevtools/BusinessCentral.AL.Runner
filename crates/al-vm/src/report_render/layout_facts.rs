//! The facts of a Word layout file that the Html gate compares (`super::shape`): the part names of the package, its
//! relationships, the element names and namespace declarations of each XML part under `word/`, and the data bindings of the
//! content controls of its document part. Read from the bytes alone; a file this reader does not fully understand gives no
//! facts, so the save keeps Part A.

use std::collections::{BTreeMap, BTreeSet};

/// What [`read`] found in a layout file.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct LayoutFacts {
    /// Every part name of the package.
    pub(crate) parts: BTreeSet<String>,
    /// The relationships of every `.rels` part.
    pub(crate) relationships: Vec<Relationship>,
    /// The qualified element names (`prefix:local`) of each XML part under `word/` (`word/document.xml`,
    /// `word/settings.xml`, `word/styles.xml`, `word/theme/theme1.xml`, ...), by part name; `.rels` parts are read as
    /// [`Self::relationships`].
    pub(crate) elements: BTreeMap<String, BTreeSet<String>>,
    /// The namespace declarations (prefix, the empty text for a default namespace; URI) of those parts: the element names
    /// above are compared by prefix, so the prefixes' URIs are compared too.
    pub(crate) namespaces: BTreeSet<(String, String)>,
    /// The content controls of `word/document.xml` with a `w:dataBinding`, in document order.
    pub(crate) bindings: Vec<Binding>,
}

/// One `Relationship` of a `.rels` part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Relationship {
    /// Its `Type`.
    pub(crate) kind: String,
    /// It states a `TargetMode` (`External`, or `Internal` written out).
    pub(crate) external: bool,
}

/// One content control bound to the merger's data part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Binding {
    /// The `w:prefixMappings` text of its `w:dataBinding`.
    pub(crate) prefixes: String,
    /// The `w:xpath` text of its `w:dataBinding`.
    pub(crate) xpath: String,
    /// The control is a repeating section (`w15:repeatingSection`).
    pub(crate) repeating: bool,
    /// The xpaths of the bound repeating sections around it, outermost first.
    pub(crate) within: Vec<String>,
}

/// The facts of `layout`, or `None` when it is no zip archive, a part is compressed in a way the zip reader skips, or an XML
/// part under `word/` or a `.rels` part is not XML this reader understands.
pub(crate) fn read(layout: &[u8]) -> Option<LayoutFacts> {
    let entries = crate::report_layout_export::zip_entries(layout)?;
    if entries.len() != zip_entry_count(layout)? {
        return None;
    }
    let mut facts = LayoutFacts::default();
    for (name, data) in &entries {
        facts.parts.insert(name.clone());
        let xml = || std::str::from_utf8(data.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(data)).ok();
        match name.as_str() {
            "word/document.xml" => document(xml()?, &mut facts)?,
            rels if rels.ends_with(".rels") => facts.relationships.extend(relationships(xml()?)?),
            part if part.starts_with("word/") && part.ends_with(".xml") => {
                let mut names = BTreeSet::new();
                for t in tags(xml()?)? {
                    if !t.close {
                        names.insert(t.name.to_string());
                        facts.namespaces.extend(declarations(t.attrs)?);
                    }
                }
                facts.elements.insert(part.to_string(), names);
            }
            _ => {}
        }
    }
    Some(facts)
}

/// The entry count of the zip archive's end record (`zip_entries` skips an entry of an unknown method).
fn zip_entry_count(bytes: &[u8]) -> Option<usize> {
    let end = (0..bytes.len().saturating_sub(21))
        .rev()
        .find(|&i| bytes.get(i..i + 4) == Some(&[0x50, 0x4b, 0x05, 0x06][..]))?;
    let b = bytes.get(end + 10..end + 12)?;
    Some(usize::from(u16::from_le_bytes([b[0], b[1]])))
}

/// The namespace declarations of a tag's attribute text: `xmlns:p="uri"` as (`p`, uri), `xmlns="uri"` as (``, uri); `None`
/// when one has no quoted value.
fn declarations(attrs: &str) -> Option<Vec<(String, String)>> {
    let mut out = Vec::new();
    let mut rest = attrs;
    while let Some(at) = rest.find("xmlns") {
        let before_ok = rest[..at].ends_with(|c: char| c.is_ascii_whitespace());
        let name_end = rest[at..]
            .find(|c: char| c == '=' || c.is_ascii_whitespace())
            .map_or(rest.len(), |e| at + e);
        let name = &rest[at..name_end];
        rest = &rest[name_end..];
        let prefix = match name.strip_prefix("xmlns") {
            Some("") => "",
            Some(p) => match p.strip_prefix(':') {
                Some(p) => p,
                None => continue,
            },
            None => continue,
        };
        if !before_ok {
            continue;
        }
        out.push((prefix.to_string(), attr(&format!(" {name}{rest}"), name)?));
    }
    Some(out)
}

/// The `Relationship` elements of a `.rels` part.
fn relationships(xml: &str) -> Option<Vec<Relationship>> {
    tags(xml)?
        .iter()
        .filter(|t| t.name == "Relationship" && !t.close)
        .map(|t| {
            Some(Relationship {
                kind: attr(t.attrs, "Type")?,
                external: attr(t.attrs, "TargetMode").is_some(),
            })
        })
        .collect()
}

/// One content control being read: whether it repeats and its binding.
#[derive(Default)]
struct Control {
    repeating: bool,
    binding: Option<(String, String)>,
}

fn document(xml: &str, facts: &mut LayoutFacts) -> Option<()> {
    let mut open: Vec<Control> = Vec::new();
    let mut names = BTreeSet::new();
    for t in tags(xml)? {
        if !t.close {
            names.insert(t.name.to_string());
            facts.namespaces.extend(declarations(t.attrs)?);
        }
        match (t.name, t.close) {
            ("w:sdt", false) if !t.self_close => open.push(Control::default()),
            ("w15:repeatingSection", false) => open.last_mut()?.repeating = true,
            ("w:dataBinding", false) => {
                let c = open.last_mut()?;
                if c.binding.is_some() {
                    return None;
                }
                c.binding = Some((attr(t.attrs, "w:prefixMappings")?, attr(t.attrs, "w:xpath")?));
            }
            ("w:sdt", true) => {
                let c = open.pop()?;
                if let Some((prefixes, xpath)) = c.binding {
                    let within = open
                        .iter()
                        .filter(|o| o.repeating)
                        .filter_map(|o| o.binding.as_ref().map(|b| b.1.clone()))
                        .collect();
                    facts.bindings.push(Binding {
                        prefixes,
                        xpath,
                        repeating: c.repeating,
                        within,
                    });
                }
            }
            _ => {}
        }
    }
    facts.elements.insert("word/document.xml".to_string(), names);
    open.is_empty().then_some(())
}

/// One tag of an XML text.
struct Tag<'a> {
    name: &'a str,
    close: bool,
    self_close: bool,
    attrs: &'a str,
}

/// The element tags of `xml` in order, without the declaration, processing instructions and comments; `None` for a
/// DOCTYPE, a CDATA section or a tag that does not end.
fn tags(xml: &str) -> Option<Vec<Tag<'_>>> {
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(i) = rest.find('<') {
        rest = &rest[i + 1..];
        if let Some(r) = rest.strip_prefix("!--") {
            rest = &r[r.find("-->")? + 3..];
            continue;
        }
        if rest.starts_with('!') {
            return None;
        }
        let end = rest.find('>')?;
        let body = &rest[..end];
        rest = &rest[end + 1..];
        if body.starts_with('?') {
            continue;
        }
        let (close, body) = match body.strip_prefix('/') {
            Some(b) => (true, b),
            None => (false, body),
        };
        let (self_close, body) = match body.strip_suffix('/') {
            Some(b) => (true, b),
            None => (false, body),
        };
        let name_end = body.find(|c: char| c.is_ascii_whitespace()).unwrap_or(body.len());
        let name = &body[..name_end];
        if name.is_empty() {
            return None;
        }
        out.push(Tag {
            name,
            close,
            self_close,
            attrs: &body[name_end..],
        });
    }
    Some(out)
}

/// The value of attribute `name` in a tag's attribute text, quoted with `"` or `'`, with the five predefined entities
/// replaced; `None` when the tag has no such attribute.
fn attr(attrs: &str, name: &str) -> Option<String> {
    let mut rest = attrs;
    loop {
        let at = rest.find(name)?;
        let before_ok = rest[..at].ends_with(|c: char| c.is_ascii_whitespace());
        let after = rest[at + name.len()..].trim_start();
        rest = &rest[at + name.len()..];
        let Some(after) = after.strip_prefix('=').map(str::trim_start) else {
            continue;
        };
        if !before_ok {
            continue;
        }
        let quote = after.chars().next()?;
        if quote != '"' && quote != '\'' {
            return None;
        }
        let value = &after[1..];
        let value = &value[..value.find(quote)?];
        return Some(
            value
                .replace("&lt;", "<")
                .replace("&gt;", ">")
                .replace("&quot;", "\"")
                .replace("&apos;", "'")
                .replace("&amp;", "&"),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bindings_name_the_repeating_sections_around_them() {
        let mut f = LayoutFacts::default();
        let xml = "<?xml version=\"1.0\"?><w:document><!-- c --><w:body><w:sdt><w:sdtPr>\
            <w:dataBinding w:prefixMappings=\"xmlns:ns0='urn:x/'\" w:xpath=\"/ns0:P[1]/ns0:Row\" /><w15:repeatingSection />\
            </w:sdtPr><w:sdtContent><w:sdt><w:sdtPr><w15:repeatingSectionItem /></w:sdtPr><w:sdtContent><w:sdt><w:sdtPr>\
            <w:dataBinding w:prefixMappings=\"xmlns:ns0='urn:x/'\" w:xpath=\"/ns0:P[1]/ns0:Row[1]/ns0:T[1]\" /></w:sdtPr>\
            <w:sdtContent><w:r><w:t>T</w:t></w:r></w:sdtContent></w:sdt></w:sdtContent></w:sdt></w:sdtContent></w:sdt>\
            </w:body></w:document>";
        document(xml, &mut f).unwrap();
        assert_eq!(
            f.bindings,
            vec![
                Binding {
                    prefixes: "xmlns:ns0='urn:x/'".into(),
                    xpath: "/ns0:P[1]/ns0:Row[1]/ns0:T[1]".into(),
                    repeating: false,
                    within: vec!["/ns0:P[1]/ns0:Row".into()],
                },
                Binding {
                    prefixes: "xmlns:ns0='urn:x/'".into(),
                    xpath: "/ns0:P[1]/ns0:Row".into(),
                    repeating: true,
                    within: vec![],
                },
            ]
        );
        let names = &f.elements["word/document.xml"];
        assert!(names.contains("w15:repeatingSectionItem") && names.contains("w:t"));
    }

    #[test]
    fn xml_this_reader_does_not_understand_gives_no_facts() {
        let mut f = LayoutFacts::default();
        // A control that does not close, two bindings on one control, a CDATA section.
        assert_eq!(document("<w:body><w:sdt><w:sdtPr>", &mut f), None);
        assert_eq!(
            document(
                "<w:sdt><w:dataBinding w:prefixMappings='a' w:xpath='b'/><w:dataBinding w:prefixMappings='a' \
                 w:xpath='c'/></w:sdt>",
                &mut f
            ),
            None
        );
        assert!(tags("<a><![CDATA[x]]></a>").is_none());
        assert_eq!(read(b"not a zip"), None);
    }

    #[test]
    fn namespace_declarations_are_read_with_their_prefix() {
        assert_eq!(
            declarations(" xmlns:w=\"urn:w\" w:val=\"xmlns\" xmlns='urn:d' a:xmlns=\"no\"").unwrap(),
            vec![
                ("w".to_string(), "urn:w".to_string()),
                (String::new(), "urn:d".to_string())
            ]
        );
        assert_eq!(declarations(" xmlns:w=urn"), None);
    }

    #[test]
    fn a_relationship_names_its_type_and_whether_it_states_a_target_mode() {
        let rels = "<?xml version=\"1.0\"?><Relationships xmlns=\"x\"><Relationship Type=\"t/settings\" Target=\"/word/settings.xml\" \
                    Id=\"R1\" /><Relationship Id=\"R2\" Type=\"t/image\" TargetMode=\"External\" Target=\"http://a/b.png\" />\
                    </Relationships>";
        assert_eq!(
            relationships(rels).unwrap(),
            vec![
                Relationship {
                    kind: "t/settings".into(),
                    external: false
                },
                Relationship {
                    kind: "t/image".into(),
                    external: true
                },
            ]
        );
    }
}
