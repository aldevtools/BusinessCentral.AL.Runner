//! Name and namespace natives of nodes.

use super::*;

// ----- names

/// Whether the receiver is a variable of type `ty` that holds nothing: never assigned, or cleared.
pub(super) fn unassigned(r: &Option<&mut Value>, ty: &str) -> bool {
    matches!(r.as_deref(), Some(Value::Library(name, h)) if name.as_str() == ty && matches!(&*h.borrow(), LibraryValue::Empty))
}

pub(super) fn element_name(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> NResult {
    // An unassigned XmlElement raises NotInitialized (xml/XmlElement.al:820 Default_Element_Uninitialized `name`); a cleared
    // one is an unassigned one (Clear marks that assumption; corpus branch al-runtime-rs-staging-ul b753d3fb7a,
    // xml/lt-ul-clear-ref/UlClearXmlRef.al:90-102 ClearXmlElement_NeverAssigned).
    if unassigned(&r, "XmlElement") {
        return Err(uninitialized("XmlElement"));
    }
    // So does an unassigned XmlAttribute (xml/XmlAttributes.al:517-522 Attribute_Uninitialized), also after a Get that found
    // nothing (corpus master, corpus PR 886: xml/lt-xml-attr-doc-http-members/AttrCollectionGetMissing.al:76-83).
    if unassigned(&r, "XmlAttribute") {
        return Err(uninitialized("XmlAttribute"));
    }
    let h = recv(r)?;
    Ok(Value::Text(
        with(&h, |d| match d {
            // An element in a namespace without a prefix reads `:name` (xml/XmlElement.al:Create_Namespace).
            XData::Element { prefix, local, ns, .. } if prefix.is_empty() && !ns.is_empty() => format!(":{local}"),
            XData::Element { prefix, local, .. } | XData::Attribute { prefix, local, .. } => qname(prefix, local),
            _ => String::new(),
        })
        .into(),
    ))
}

pub(super) fn local_name(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> NResult {
    let h = recv(r)?;
    Ok(Value::Text(
        with(&h, |d| match d {
            XData::Element { local, .. } | XData::Attribute { local, .. } => local.clone(),
            _ => String::new(),
        })
        .into(),
    ))
}

pub(super) fn namespace_uri(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> NResult {
    let h = recv(r)?;
    Ok(Value::Text(
        with(&h, |d| match d {
            XData::Element { ns, .. } | XData::Attribute { ns, .. } => ns.clone(),
            _ => String::new(),
        })
        .into(),
    ))
}

pub(super) fn namespace_prefix(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> NResult {
    let h = recv(r)?;
    Ok(Value::Text(
        with(&h, |d| match d {
            XData::Attribute { prefix, .. } => prefix.clone(),
            _ => String::new(),
        })
        .into(),
    ))
}

pub(super) fn is_namespace_declaration(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> NResult {
    Ok(Value::Boolean(is_xmlns_decl(&recv(r)?)))
}

pub(super) fn get_prefix_of_namespace(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let h = recv(r)?;
    let uri = arg_text(a, 0)?;
    let mut cur = Some(h);
    while let Some(e) = cur {
        for at in attrs_of(&e) {
            let hit = with(&at, |d| match d {
                XData::Attribute {
                    prefix,
                    local,
                    ns,
                    value,
                } if ns == XMLNS_URI && *value == uri => {
                    Some(if prefix == "xmlns" { Some(local.clone()) } else { None })
                }
                _ => None,
            });
            match hit {
                Some(Some(p)) => {
                    a[1] = Value::Text(p.into());
                    return Ok(Value::Boolean(true));
                }
                // xml/TestXmlRuntimeContracts.al:XmlElement_NamespaceLookups: the default namespace
                // has an empty prefix.
                Some(None) => {
                    a[1] = Value::Text(String::new().into());
                    return Ok(Value::Boolean(true));
                }
                None => {}
            }
        }
        cur = parent_of(&e);
    }
    // No declaration in scope: Yes, the reserved namespaces give `xml` and `xmlns` and any other (the empty one, one no
    // element declares, on a loaded or a detached element) the empty prefix (xml/lt-tf-xml-ns/XmlNsPrefixes.al:55-108
    // Prefix_OfUndeclaredNamespace; xml/XmlXPathNamespaces.al:594 prefix_of_missing). With the result unused there is no
    // error (misc/lt-probes-needed-1/PnAXmlParentDocNs.al:56,104 op10).
    let p = match uri.as_str() {
        "http://www.w3.org/XML/1998/namespace" => "xml",
        XMLNS_URI => "xmlns",
        _ => "",
    };
    a[1] = Value::Text(p.to_string().into());
    Ok(Value::Boolean(true))
}

pub(super) fn get_namespace_of_prefix(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let h = recv(r)?;
    let prefix = arg_text(a, 0)?;
    // The empty prefix has no namespace and leaves the variable alone
    // (xml/TestXmlRuntimeContracts.al:XmlElement_NamespaceLookups).
    if prefix.is_empty() && c.ret_used() {
        return Ok(Value::Boolean(false));
    }
    if prefix.is_empty() {
        // xml/TestProbeXmlContracts.al:GetNamespaceOfPrefix_Empty_Statement
        return Err(nav_argument());
    }
    // The reserved prefixes have their namespaces without a declaration (xml/lt-tf-xml-ns/XmlNsPrefixes.al:76-81, :98-101
    // root_xml, root_xmlns). Pinned with the result used only.
    let reserved = match prefix.as_str() {
        "xml" => Some("http://www.w3.org/XML/1998/namespace"),
        "xmlns" => Some(XMLNS_URI),
        _ => None,
    };
    if let Some(u) = reserved {
        if !c.ret_used() {
            return unsupported("GetNamespaceOfPrefix of a reserved prefix, result unused (not pinned by the corpus)");
        }
        a[1] = Value::Text(u.to_string().into());
        return Ok(Value::Boolean(true));
    }
    match lookup_namespace(&h, &prefix) {
        Some(u) => {
            a[1] = Value::Text(u.into());
            Ok(Value::Boolean(true))
        }
        None if c.ret_used() => Ok(Value::Boolean(false)),
        None => Err(nav_argument()),
    }
}
