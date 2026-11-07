//! Which prefix a namespace is written and named with: the declarations on an element, its ancestors and the writer's scope.
//! Issue 7208 (al-runner): xml/xml/XmlNsPrefixes.al Decl7208_* and Attribute_PrefixNumbering.

use super::*;

/// The prefix of the first declaration for `uri` that is in scope of `h`, the element's own declarations in the order they were
/// added first, then its ancestors' (`XElement.GetPrefixOfNamespace`). A default namespace declaration gives the empty prefix
/// (xml/TestXmlRuntimeContracts.al:XmlElement_NamespaceLookups). `None` when no element declares it.
pub(super) fn declared_prefix(h: &Handle, uri: &str) -> Option<String> {
    let mut cur = Some(h.clone());
    while let Some(e) = cur {
        for at in attrs_of(&e) {
            let hit = with(&at, |d| match d {
                XData::Attribute {
                    prefix,
                    local,
                    ns,
                    value,
                } if ns == XMLNS_URI && value == uri => Some(if prefix == "xmlns" {
                    local.clone()
                } else {
                    String::new()
                }),
                _ => None,
            });
            if hit.is_some() {
                return hit;
            }
        }
        cur = parent_of(&e);
    }
    None
}

/// The `Name` of an element or attribute that has a namespace but no prefix of its own (made by `Create`): the prefix a
/// declaration in scope gives it, else a colon alone (xml/xml/XmlNsPrefixes.al:Decl7208_* `*_name`, `*_k2`,
/// Attribute_PrefixNumbering `names_after_add`). An attribute that has no parent yet is named with `p0`
/// (Attribute_PrefixNumbering `detached`); one that is attached and finds a declaration is not pinned.
pub(super) fn unprefixed_name(h: &Handle, local: &str, ns: &str, is_attribute: bool) -> Result<String, Signal> {
    if is_attribute {
        let Some(parent) = parent_of(h) else {
            return Ok(format!("p0:{local}"));
        };
        return match declared_prefix(&parent, ns) {
            None => Ok(format!(":{local}")),
            Some(_) => unsupported("Name of an attached attribute whose namespace is declared (not pinned)"),
        };
    }
    // An element read with a prefix is named by the first declaration too, not by the prefix it was written with
    // (Decl7208_TwoPrefixes `pq_rtname`: `<q:name xmlns:p="urn:x" xmlns:q="urn:x" />` reads `p:name`).
    let own = with(h, |d| match d {
        XData::Element { prefix, .. } => prefix.clone(),
        _ => String::new(),
    });
    Ok(match declared_prefix(h, ns) {
        Some(p) if !p.is_empty() => format!("{p}:{local}"),
        None if !own.is_empty() => format!("{own}:{local}"),
        _ => format!(":{local}"),
    })
}

/// The prefix the writer uses for `uri` from its scope: the last declaration that is still visible and has a prefix
/// (xml/xml/XmlNsPrefixes.al:Decl7208_TwoPrefixes `pq_txt`, `qp_txt`).
pub(super) fn scope_prefix(scope: &[(String, String)], uri: &str) -> Option<String> {
    scope
        .iter()
        .rev()
        .find(|(p, u)| {
            !p.is_empty() && u == uri && scope.iter().rev().find(|(q, _)| q == p).map(|(_, v)| v.as_str()) == Some(uri)
        })
        .map(|(p, _)| p.clone())
}
