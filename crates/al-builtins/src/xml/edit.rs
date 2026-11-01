//! Inserting, detaching and replacing nodes in a tree.

use super::*;

/// Nodes from `Content: Any*` arguments; a node that already has a parent is copied.
pub(super) fn content_nodes(args: &[Value]) -> Result<Vec<Handle>, Signal> {
    let mut out = Vec::new();
    for a in args {
        let h = match a {
            Value::Library(name, h)
                if matches!(&*h.borrow(), LibraryValue::Empty)
                    && matches!(name.as_str(), "XmlDocumentType" | "XmlDeclaration") =>
            {
                return Err(uninitialized(name));
            }
            Value::Library(_, h) if is_xml(h) => h.clone(),
            // Text is accepted as content and becomes a text node
            // (xml/TestXmlRuntimeContracts.al:XmlElement_Add_Text_IsAccepted_AndAdjacentTextsMerge).
            Value::Text(t) | Value::Code(t) => {
                out.push(new_node(XData::Text(t.to_string())));
                continue;
            }
            other => {
                return unsupported(format!(
                    "{} as XML content (only XML nodes and text are pinned)",
                    other.describe()
                ));
            }
        };
        if with(&h, |d| matches!(d, XData::Document { .. })) {
            // xml/TestProbeXmlContracts.al:Add_NodeIntoWrongContainer_Error, `document_into_element_via_add_node`.
            return Err(nav_argument());
        }
        out.push(if parent_of(&h).is_some() { deep_clone(&h) } else { h });
    }
    Ok(out)
}

/// Whether `node` may be added to `container`. `removed` are children that go first (a replace).
pub(super) fn check_fits(
    container: &Handle,
    node: &Handle,
    removed: &[Handle],
    at: Option<usize>,
) -> Result<(), Signal> {
    if is_ancestor_or_self(node, container) {
        return unsupported("adding an XML node to itself or its own descendant");
    }
    let c_kind = kind(container);
    let n_kind = kind(node);
    let ok = match (c_kind, n_kind) {
        ("XmlDocument", "XmlComment" | "XmlProcessingInstruction" | "XmlDeclaration") => true,
        // One document type, before the root element (xml/lt-xml-doctype-declaration/DocTypeInDoc.al:DocumentAdd_*).
        ("XmlDocument", "XmlDocumentType") => {
            let kids: Vec<Handle> = children_of(container);
            let keep = |c: &&Handle| !removed.iter().any(|r| Rc::ptr_eq(r, c));
            let has_type = kids
                .iter()
                .filter(keep)
                .any(|c| with(c, |d| matches!(d, XData::DocType { .. })));
            let before: Vec<&Handle> = match at {
                Some(i) => kids.iter().take(i).filter(keep).collect(),
                None => kids.iter().filter(keep).collect(),
            };
            let root_before = before.iter().any(|c| with(c, |d| matches!(d, XData::Element { .. })));
            if has_type || root_before {
                return Err(error(
                    "The requested operation cannot be performed in this context.",
                    "InvalidOperation",
                ));
            }
            true
        }
        ("XmlElement", "XmlDocumentType") => return Err(nav_argument()),
        // An XML declaration added to an element becomes its text (P90CDataEdit.al:AfterSelf_Attached_InvalidKinds `k7`).
        ("XmlElement", "XmlDeclaration") => true,
        ("XmlDocument", "XmlElement") => {
            let has_root = children_of(container)
                .iter()
                .filter(|c| !removed.iter().any(|r| Rc::ptr_eq(r, c)))
                .any(|c| with(c, |d| matches!(d, XData::Element { .. })));
            if has_root {
                // xml/TestXmlRuntimeContracts.al:XmlDocument_Add_SecondRoot_Throws
                return Err(error(
                    "The requested operation cannot be performed in this context.",
                    "InvalidOperation",
                ));
            }
            true
        }
        ("XmlElement", "XmlElement" | "XmlText" | "XmlCData" | "XmlComment" | "XmlProcessingInstruction") => true,
        ("XmlElement", "XmlAttribute") => true,
        _ => false,
    };
    if ok {
        Ok(())
    } else {
        match (c_kind, n_kind) {
            // An empty or blank text at document level is not pinned (.NET takes white space there): every pin below adds a
            // non-blank text (xml/TestProbeXmlContracts.al:268, xml/XmlDocReadWrite.al:556, misc/lt-probes-needed-1/PnAXmlNodeAdds.al:38).
            ("XmlDocument", "XmlText") if with(node, |d| matches!(d, XData::Text(t) if t.trim().is_empty())) => {
                unsupported("an empty or blank text added to an XmlDocument (not pinned)")
            }
            // xml/TestProbeXmlContracts.al:Add_NodeIntoWrongContainer_Error: a text or an attribute into a document.
            ("XmlDocument", "XmlText" | "XmlCData" | "XmlAttribute") => Err(nav_argument()),
            _ => unsupported(format!("adding {n_kind} to {c_kind} (error text not pinned yet)")),
        }
    }
}

/// Inserts `nodes` into `container` at child position `at` (`None` appends).
pub(super) fn insert(container: &Handle, at: Option<usize>, nodes: Vec<Handle>) -> Result<(), Signal> {
    for n in &nodes {
        check_fits(container, n, &[], at)?;
    }
    // An XML declaration added to an element is written as text: its markup (escaped on output, P90CDataEdit.al `k7`).
    let nodes: Vec<Handle> = if kind(container) == "XmlElement" {
        nodes
            .into_iter()
            .map(|n| {
                match with(&n, |d| match d {
                    XData::Declaration {
                        version,
                        encoding,
                        standalone,
                        explicit,
                    } => Some(declaration_text(version, encoding, standalone, *explicit)),
                    _ => None,
                }) {
                    Some(markup) => new_node(XData::Text(markup)),
                    None => n,
                }
            })
            .collect()
    } else {
        nodes
    };
    let mut at = at;
    for n in nodes {
        match kind(&n) {
            "XmlAttribute" => {
                let dup = attrs_of(container).iter().any(|a| same_attr_name(a, &n));
                if dup {
                    // xml/TestProbeXmlContracts.al:Add_AttributeTwice_Error (the same name, and the same instance)
                    return Err(error(
                        "The requested operation cannot be performed in this context.",
                        "InvalidOperation",
                    ));
                }
                with(container, |d| {
                    if let XData::Element { attrs, .. } = d {
                        attrs.push(n.clone());
                    }
                });
            }
            "XmlDeclaration" => {
                with(container, |d| {
                    if let XData::Document { decl, .. } = d {
                        *decl = Some(n.clone());
                    }
                });
            }
            _ => with(container, |d| {
                if let XData::Document { children, .. } | XData::Element { children, .. } = d {
                    let pos = at.unwrap_or(children.len()).min(children.len());
                    children.insert(pos, n.clone());
                    at = Some(pos + 1);
                }
            }),
        }
        set_parent(&n, Some(container));
    }
    Ok(())
}

pub(super) fn same_attr_name(a: &Handle, b: &Handle) -> bool {
    let key = |h: &Handle| {
        with(h, |d| match d {
            XData::Attribute { local, ns, .. } => (local.clone(), ns.clone()),
            _ => (String::new(), String::new()),
        })
    };
    key(a) == key(b)
}

pub(super) fn detach(h: &Handle) {
    let Some(p) = parent_of(h) else { return };
    with(&p, |d| match d {
        XData::Document { decl, children } => {
            children.retain(|c| !Rc::ptr_eq(c, h));
            if decl.as_ref().is_some_and(|x| Rc::ptr_eq(x, h)) {
                *decl = None;
            }
        }
        XData::Element { attrs, children, .. } => {
            children.retain(|c| !Rc::ptr_eq(c, h));
            attrs.retain(|c| !Rc::ptr_eq(c, h));
        }
        _ => {}
    });
    set_parent(h, None);
}

/// A text, CDATA, comment, processing instruction or document type without a parent: every sibling operation and `Remove` on it
/// raises `InvalidOperation` (xml/lt-xml-cdata-comment-pi-text/P90*Edit.al:AfterSelf_Detached_*, BeforeSelf_Detached_*,
/// ReplaceWith_Detached_*, Remove_Detached_Twice; xml/lt-xml-doctype-declaration/DocTypeNode.al). Other kinds are not pinned.
pub(super) fn detached_is_invalid(h: &Handle) -> bool {
    with(h, |d| {
        matches!(
            d,
            XData::Text(_)
                | XData::CData(_)
                | XData::Comment(_)
                | XData::Pi { .. }
                | XData::DocType { .. }
                | XData::Declaration { .. }
        )
    })
}

/// Why a sibling operation on an element, attribute or document without a parent fails, before its arguments are read.
/// - A detached attribute raises `InvalidOperation` whatever the arguments are, an unassigned `XmlNode` and an Integer
///   included (xml/lt-xml-attr-doc-http-members/P97AttrEdit.al:AddAfterSelf_Detached_Kinds `k1`..`k18`, *_Detached_Typed).
/// - A document raises `InvalidOperation` for every argument kind, except that an unassigned `XmlNode` argument raises
///   `NotInitialized` first (P97DocEdit.al:*_Kinds `k14`, *_Typed `t1`..`t11`; P97DocMisc.al:Uninitialised_Document_Edits).
/// - An element without a parent raises `InvalidOperation` for element arguments (xml/XmlElement.al:521
///   AddAfterSelf_Root_Error `lone_after`, `lone_before`; :601 ReplaceWith_Root_Unparented `lone`); other arguments are
///   not pinned.
pub(super) fn unparented_sibling_error(h: &Handle, args: &[Value]) -> Signal {
    // A node whose parent was dropped (a document held only by a local variable that went out of scope) still has
    // that parent on BC, which keeps it alive; the runtime holds parents weakly and cannot answer for it.
    if parent_dropped(h) {
        return Signal::Unsupported(
            "a sibling operation on a node whose parent is no longer referenced (the runtime does not keep it)".into(),
        );
    }
    let invalid = || {
        error(
            "The requested operation cannot be performed in this context.",
            "InvalidOperation",
        )
    };
    match kind(h) {
        "XmlAttribute" => invalid(),
        "XmlDocument" => {
            for a in args {
                if let Value::Library(name, v) = a {
                    if matches!(&*v.borrow(), LibraryValue::Empty) {
                        if name == "XmlNode" {
                            return uninitialized("XmlNode");
                        }
                        return Signal::Unsupported(format!(
                            "a sibling operation on a document with an unassigned {name} argument (not pinned)"
                        ));
                    }
                }
            }
            invalid()
        }
        "XmlElement"
            if !args.is_empty()
                && args
                    .iter()
                    .all(|a| matches!(a, Value::Library(_, v) if is_xml(v) && kind(v) == "XmlElement")) =>
        {
            invalid()
        }
        _ => Signal::Unsupported(
            "a sibling operation on an element without a parent with an argument other than an element (not pinned)"
                .into(),
        ),
    }
}

pub(super) fn position_in_parent(h: &Handle, args: &[Value]) -> Result<(Handle, usize), Signal> {
    let Some(p) = parent_of(h) else {
        if detached_is_invalid(h) {
            return Err(error(
                "The requested operation cannot be performed in this context.",
                "InvalidOperation",
            ));
        }
        return Err(unparented_sibling_error(h, args));
    };
    let pos = children_of(&p).iter().position(|c| Rc::ptr_eq(c, h));
    match pos {
        Some(i) => Ok((p, i)),
        // A declaration is no child of its document (xml/lt-xml-remaining-node-methods/P104TreeBDecl.al:P104_ReplaceWith_*).
        None if with(h, |d| matches!(d, XData::Declaration { .. })) => Err(error(
            "The requested operation cannot be performed in this context.",
            "InvalidOperation",
        )),
        None => unsupported("a sibling operation on an attribute or declaration"),
    }
}
