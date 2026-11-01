//! The element tree of a document for XmlPort import (`xmlport.rs`).

use super::*;

/// A parsed element written `<x />`: no children, and not `<x></x>` (`open`).
pub(super) fn closed_in_start_tag(h: &Handle) -> bool {
    with(
        h,
        |d| matches!(d, XData::Element { open: false, children, .. } if children.is_empty()),
    )
}

/// The element tree of a document, for XmlPort import (`xmlport.rs`). `Ok(None)`: not
/// well-formed; `Err`: well-formed but uses namespaces or other features the reader does not
/// carry over.
pub(crate) fn read_tree(text: &str) -> Result<Result<crate::xmlport::Elem, Option<String>>, String> {
    use crate::xmlport::{Elem, Node};
    fn convert(h: &Handle) -> Result<Elem, String> {
        let (prefix, local, ns, attrs) = with(h, |d| match d {
            XData::Element {
                prefix,
                local,
                ns,
                attrs,
                ..
            } => (prefix.clone(), local.clone(), ns.clone(), attrs.clone()),
            _ => unreachable!("callers pass elements"),
        });
        // A prefixed element keeps its prefix in its name (`p:Name`); the port checks the prefix against its own
        // `Namespaces`. A default namespace is carried over only on the element that declares it, as its `xmlns` attribute
        // (the port names or skips that element: YXmlPortProps.al:UseLax_Import_AcceptsUnknownElements); an element that
        // inherits one is not.
        let declares_default = attrs.iter().any(|a| {
            with(
                a,
                |d| matches!(d, XData::Attribute { prefix, local, .. } if prefix.is_empty() && local == "xmlns"),
            )
        });
        if prefix.is_empty() != ns.is_empty() && !(prefix.is_empty() && declares_default) {
            return Err("namespaces in an XmlPort document".into());
        }
        let mut out = Elem::new(&if prefix.is_empty() {
            local.clone()
        } else {
            format!("{prefix}:{local}")
        });
        for a in attrs {
            let (p, l, n, v) = with(&a, |d| match d {
                XData::Attribute {
                    prefix,
                    local,
                    ns,
                    value,
                } => (prefix.clone(), local.clone(), ns.clone(), value.clone()),
                _ => unreachable!("attributes of an element"),
            });
            // `xmlns:p="uri"` is kept as an attribute of that name; any other prefixed attribute is not carried over.
            if p == "xmlns" {
                out.attrs.push((format!("xmlns:{l}"), v));
                continue;
            }
            if p.is_empty() && l == "xmlns" {
                out.attrs.push((l, v));
                continue;
            }
            if !p.is_empty() || !n.is_empty() {
                return Err("namespace declarations in an XmlPort document".into());
            }
            out.attrs.push((l, v));
        }
        for c in children_of(h) {
            enum K {
                Element,
                Text(String),
                Skip,
            }
            match with(&c, |d| match d {
                XData::Element { .. } => K::Element,
                XData::Text(t) | XData::CData(t) => K::Text(t.clone()),
                _ => K::Skip,
            }) {
                K::Element => out.children.push(Node::Elem(convert(&c)?)),
                K::Text(t) => out.children.push(Node::Text(t)),
                K::Skip => {}
            }
        }
        out.self_closed = closed_in_start_tag(h);
        Ok(out)
    }
    let doc = match parse_document_detail(text, false) {
        Ok(Ok(d)) => d,
        Ok(Err(f)) => return Ok(Err(f.map(|f| f.message))),
        Err(Signal::Unsupported(why)) => return Err(why),
        Err(other) => return Err(other.describe()),
    };
    let root = children_of(&doc)
        .into_iter()
        .find(|c| with(c, |d| matches!(d, XData::Element { .. })))
        .ok_or("a document without a root element")?;
    // The import of a port without namespaces fails on a root element in a namespace (al-vm xmlports.rs, ROOT_IN_NAMESPACE).
    let (local, ns) = with(&root, |d| match d {
        XData::Element { local, ns, .. } => (local.clone(), ns.clone()),
        _ => unreachable!("the root is an element"),
    });
    if !ns.is_empty() {
        return Err(format!("{}{local}", crate::xmlport::ROOT_IN_NAMESPACE));
    }
    convert(&root).map(Ok)
}

/// The element tree of a document for an import into a port whose elements are all in namespace `want`
/// (`UseDefaultNamespace = true`, al-builtins xmlport.rs `parse_in_namespace`). Pinned shapes only
/// (xmlport/lt-bd-xmlport/BdXmlPort.al:499): every element in `want`, by default or by a prefix; the root elsewhere; the
/// root's first child in no namespace. Anything else in another namespace, and any other namespace declaration, is refused.
pub(crate) fn read_tree_in(
    text: &str,
    want: &str,
) -> Result<Result<crate::xmlport::InNamespace, Option<String>>, String> {
    use crate::xmlport::{Elem, InNamespace, Node};
    fn element(h: &Handle) -> (String, String) {
        with(h, |d| match d {
            XData::Element { local, ns, .. } => (local.clone(), ns.clone()),
            _ => unreachable!("callers pass elements"),
        })
    }
    fn elements(h: &Handle) -> Vec<Handle> {
        children_of(h)
            .into_iter()
            .filter(|c| with(c, |d| matches!(d, XData::Element { .. })))
            .collect()
    }
    fn convert(h: &Handle, want: &str) -> Result<Elem, String> {
        let (local, ns) = element(h);
        if ns != want {
            return Err("an XmlPort document with an element outside the port's namespace (not pinned)".into());
        }
        let attrs = with(h, |d| match d {
            XData::Element { attrs, .. } => attrs.clone(),
            _ => unreachable!("an element"),
        });
        let mut out = Elem::new(&local);
        for a in attrs {
            let (p, l, n, v) = with(&a, |d| match d {
                XData::Attribute {
                    prefix,
                    local,
                    ns,
                    value,
                } => (prefix.clone(), local.clone(), ns.clone(), value.clone()),
                _ => unreachable!("attributes of an element"),
            });
            let declaration = p == "xmlns" || (p.is_empty() && l == "xmlns");
            if declaration && v == want {
                continue;
            }
            if declaration || !p.is_empty() || !n.is_empty() {
                return Err(
                    "an XmlPort document with another namespace declaration or a prefixed attribute (not pinned)"
                        .into(),
                );
            }
            out.attrs.push((l, v));
        }
        for c in children_of(h) {
            match with(&c, |d| match d {
                XData::Element { .. } => None,
                XData::Text(t) | XData::CData(t) => Some(Some(t.clone())),
                _ => Some(None),
            }) {
                None => out.children.push(Node::Elem(convert(&c, want)?)),
                Some(Some(t)) => out.children.push(Node::Text(t)),
                Some(None) => {}
            }
        }
        out.self_closed = closed_in_start_tag(h);
        Ok(out)
    }
    let doc = match parse_document_detail(text, false) {
        Ok(Ok(d)) => d,
        Ok(Err(f)) => return Ok(Err(f.map(|f| f.message))),
        Err(Signal::Unsupported(why)) => return Err(why),
        Err(other) => return Err(other.describe()),
    };
    let root = elements(&doc)
        .into_iter()
        .next()
        .ok_or("a document without a root element")?;
    let (local, ns) = element(&root);
    if ns != want {
        return Ok(Ok(InNamespace::RootElsewhere(local)));
    }
    let prefixed = with(&root, |d| match d {
        XData::Element { attrs, .. } => attrs
            .iter()
            .filter_map(|a| {
                with(a, |d| match d {
                    XData::Attribute { prefix, local, ns, .. }
                        if !prefix.is_empty() && prefix != "xmlns" && ns == want =>
                    {
                        Some(local.clone())
                    }
                    _ => None,
                })
            })
            .next(),
        _ => None,
    });
    if let Some(attr) = prefixed {
        return Ok(Ok(InNamespace::RootAttributeInNamespace(attr)));
    }
    if let Some(first) = elements(&root).first() {
        let (child, child_ns) = element(first);
        if child_ns.is_empty() {
            return Ok(Ok(InNamespace::FirstChildInNone { root: local, child }));
        }
    }
    convert(&root, want).map(|e| Ok(InNamespace::Tree(e)))
}
