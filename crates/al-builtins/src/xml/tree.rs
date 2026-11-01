//! Natives that add, remove, replace and navigate to nodes.

use super::*;

// ----- structure

pub(super) fn add(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    // Add of one node to an unassigned XmlElement raises NotInitialized (xml/XmlElement.al:822 Default_Element_Uninitialized
    // `add`, an XmlText); other arguments are not pinned and stay refused below.
    if unassigned(&r, "XmlElement") && matches!(a, [Value::Library(_, v)] if is_xml(v)) {
        return Err(uninitialized("XmlElement"));
    }
    let h = recv(r)?;
    reject_unassigned_element_content(&h, a, "Add")?;
    // A text given as a string joins the text node it follows; an XmlText node is its own node
    // (xml/TestProbeXmlContracts.al:AdjacentTextNodes_NotMerged).
    let last = a.len().saturating_sub(1);
    for (i, arg) in a.iter().enumerate() {
        // An XmlWriteOptions after a node, given to Add of a document or an element, is accepted and adds nothing
        // (writer TV, corpus branch al-runtime-rs-staging-tv 8ae7ac0b, xml/lt-de-xml-leads/DeXmlWriteLeads.al
        // TV_XmlWriteOptions_AsContent_DocumentAdd and _ElementAdd: one child, written as before).
        if i == last
            && i > 0
            && matches!(arg, Value::Library(name, _) if name.as_str() == "XmlWriteOptions")
            && matches!(kind(&h), "XmlDocument" | "XmlElement")
        {
            // Pinned by xml/lt-de-xml-leads/DeXmlWriteLeads.al TV_WriteToOutStream_BuiltWithoutDeclaration_Options, TV_WriteToOutStream_Utf8Declaration_OtherStreams, TV_WriteToOutStream_Utf16AndNoEncodingDeclarations, TV_XmlWriteOptions_AsContent_DocumentAdd (was assumption `xml-write-to-outstream-built`).
            continue;
        }
        if doc_blank_text(&h, arg)? {
            continue;
        }
        if let Value::Text(t) | Value::Code(t) = arg {
            let last = children_of(&h).last().cloned();
            if let Some(last) = last.filter(|l| with(l, |d| matches!(d, XData::Text(_)))) {
                with(&last, |d| {
                    if let XData::Text(x) = d {
                        x.push_str(t);
                    }
                });
                continue;
            }
        }
        let nodes = content_nodes(std::slice::from_ref(arg))?;
        reject_declaration_in_document(&h, &nodes)?;
        insert(&h, None, nodes)?;
    }
    Ok(Value::Boolean(true))
}

/// `Add` of an XML declaration to a document raises (a declaration comes with `Create` or `SetDeclaration`:
/// misc/lt-probes-needed-1/PnAXmlNodeAdds.al `declaration-into-document`, an Argument error).
pub(super) fn reject_declaration_in_document(container: &Handle, nodes: &[Handle]) -> Result<(), Signal> {
    if kind(container) == "XmlDocument" && nodes.iter().any(|n| kind(n) == "XmlDeclaration") {
        return Err(nav_argument());
    }
    Ok(())
}

/// An attribute is not a sibling: `AddAfterSelf`, `AddBeforeSelf` and `ReplaceWith` of an attribute raise an Argument error
/// (xml/lt-xml-cdata-comment-pi-text/P90*Edit.al:AfterSelf_Attached_InvalidKinds `k6`).
pub(super) fn reject_attributes(nodes: &[Handle]) -> Result<(), Signal> {
    if nodes.iter().any(|n| kind(n) == "XmlAttribute") {
        return Err(nav_argument());
    }
    Ok(())
}

pub(super) fn add_first(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let h = recv(r)?;
    reject_unassigned_element_content(&h, a, "AddFirst")?;
    let nodes = content_nodes(a)?;
    reject_declaration_in_document(&h, &nodes)?;
    insert(&h, Some(0), nodes)?;
    Ok(Value::Boolean(true))
}

/// Content of a sibling operation of a text, CDATA, comment, processing instruction, document type or declaration
/// (the receivers xml/lt-xml-remaining-node-methods/P104Tree*.al pin):
/// - an Integer becomes its text (P104TreeACData.al:198 `c8` AddAfterSelf(42), :309 AddBeforeSelf(42); P104TreeBText.al:321
///   ReplaceWith(42)). Only 0..=999 runs: whether a larger number is written with digit grouping (Format) or without
///   (.NET) is not pinned, nor is the sign of a negative;
/// - an empty text adds nothing (P104TreeAComment.al:208, :319 `c15`; P104TreeBComment.al:331 `c15`).
///
/// For other receivers neither is pinned: an Integer is refused by content_nodes and an empty text here.
pub(super) fn sibling_content_nodes(h: &Handle, args: &[Value]) -> Result<Vec<Handle>, Signal> {
    let pinned = with(h, |d| {
        !matches!(
            d,
            XData::Element { .. } | XData::Document { .. } | XData::Attribute { .. }
        )
    });
    let mut conv: Vec<Value> = Vec::with_capacity(args.len());
    for a in args {
        conv.push(match a {
            Value::Integer(n) if pinned && (0..=999).contains(n) => Value::Text(n.to_string().into()),
            Value::Text(t) | Value::Code(t) if t.is_empty() => {
                if !pinned {
                    return unsupported(format!(
                        "an empty text as sibling content of a {} (not pinned)",
                        kind(h)
                    ));
                }
                continue;
            }
            other => other.clone(),
        });
    }
    content_nodes(&conv)
}

/// A text, CDATA, comment or processing instruction whose sibling operation gets one unassigned `XmlNode` raises
/// `NotInitialized` and changes nothing, attached or detached (xml/lt-xml-cdata-comment-pi-text/P90*Edit.al:
/// AfterSelf_UninitialisedArgument, BeforeSelf_UninitialisedArgument, ReplaceWith_UninitialisedArgument, P90CDataEdit.al:404).
/// Several arguments with an unassigned one among them are not pinned (the content content_nodes refuses).
pub(super) fn reject_unassigned_sibling_arg(h: &Handle, a: &[Value]) -> Result<(), Signal> {
    if let [Value::Library(name, v)] = a {
        if name == "XmlNode"
            && matches!(&*v.borrow(), LibraryValue::Empty)
            && with(h, |d| {
                matches!(
                    d,
                    XData::Text(_) | XData::CData(_) | XData::Comment(_) | XData::Pi { .. }
                )
            })
        {
            return Err(uninitialized("XmlNode"));
        }
    }
    Ok(())
}

pub(super) fn is_text_like(h: &Handle) -> bool {
    with(h, |d| {
        matches!(
            d,
            XData::Text(_) | XData::CData(_) | XData::Comment(_) | XData::Pi { .. }
        )
    })
}

/// Inserts the content of a sibling operation of a text, CDATA, comment or processing instruction. An attribute among
/// the nodes raises an Argument error after the nodes before it are in (xml/lt-xml-cdata-comment-pi-text/
/// P90CDataEdit.al:887 AfterSelf_ThreeNodes_OneInvalid `t1_6_3` keeps `e1`, `t1_3_6` keeps `e1` and `c3`, `t6_1_3` adds nothing).
/// Other receivers refuse attributes before inserting anything (reject_attributes).
pub(super) fn insert_siblings(h: &Handle, p: &Handle, at: usize, nodes: Vec<Handle>) -> Result<(), Signal> {
    // A declaration next to a document type raises an Argument error, as `Add` of one to a document does
    // (xml/lt-xml-remaining-node-methods/P104TreeADocType.al:211 `c12`, :341; P104TreeBDocType.al:333).
    if with(h, |d| matches!(d, XData::DocType { .. })) {
        reject_declaration_in_document(p, &nodes)?;
    }
    if !is_text_like(h) {
        reject_attributes(&nodes)?;
        return insert(p, Some(at), nodes);
    }
    match nodes.iter().position(|n| kind(n) == "XmlAttribute") {
        Some(k) => {
            insert(p, Some(at), nodes[..k].to_vec())?;
            Err(nav_argument())
        }
        None => insert(p, Some(at), nodes),
    }
}

/// An element whose `Add`, `AddFirst`, `AddAfterSelf` or `AddBeforeSelf` gets one unassigned `XmlNode` raises
/// `Microsoft.Dynamics.Nav.Runtime.NavXmlNode variable not initialized.` (NotInitialized) and changes nothing; an unassigned
/// `XmlElement` given to `Add` or `AddAfterSelf` names NavXmlElement (xml/lt-wd/WdXmlEmptyNsAndUnassigned.al
/// WD_XmlNode_Unassigned_AsContent, WD_XmlElement_Unassigned_AsContent; corpus branch al-runtime-rs-staging-1030c, al-runner
/// issue 6303). Other receivers, methods and argument lists are not pinned (content_nodes refuses them).
pub(super) fn reject_unassigned_element_content(h: &Handle, a: &[Value], method: &str) -> Result<(), Signal> {
    if let [Value::Library(name, v)] = a {
        let pinned = match name.as_str() {
            "XmlNode" => matches!(method, "Add" | "AddFirst" | "AddAfterSelf" | "AddBeforeSelf"),
            "XmlElement" => matches!(method, "Add" | "AddAfterSelf"),
            _ => false,
        };
        if pinned && matches!(&*v.borrow(), LibraryValue::Empty) && with(h, |d| matches!(d, XData::Element { .. })) {
            return Err(uninitialized(name));
        }
    }
    Ok(())
}

pub(super) fn add_after_self(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let h = recv(r)?;
    reject_unassigned_sibling_arg(&h, a)?;
    reject_unassigned_element_content(&h, a, "AddAfterSelf")?;
    let (p, i) = position_in_parent(&h, a)?;
    let nodes = sibling_content_nodes(&h, a)?;
    insert_siblings(&h, &p, i + 1, nodes)?;
    Ok(Value::Boolean(true))
}

pub(super) fn add_before_self(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let h = recv(r)?;
    reject_unassigned_sibling_arg(&h, a)?;
    reject_unassigned_element_content(&h, a, "AddBeforeSelf")?;
    let (p, i) = position_in_parent(&h, a)?;
    let nodes = sibling_content_nodes(&h, a)?;
    insert_siblings(&h, &p, i, nodes)?;
    Ok(Value::Boolean(true))
}

pub(super) fn remove(c: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> NResult {
    let h = recv_or_uninit(r, "XmlNode")?;
    // A declaration cannot be removed, from its document or detached; the document keeps it (xml/lt-xml-remaining-node-methods/
    // P104TreeBDecl.al:P104_Remove_InDoc).
    if with(&h, |d| matches!(d, XData::Declaration { .. })) {
        if c.ret_used() {
            return unsupported("Remove of an XmlDeclaration with the result used (not pinned)");
        }
        return Err(error(
            "The requested operation cannot be performed in this context.",
            "InvalidOperation",
        ));
    }
    if parent_of(&h).is_none() {
        // A node of these kinds without a parent (DocTypeNode.al:Remove_DetachedDocType, P90*Edit.al:Remove_Detached_Twice).
        if detached_is_invalid(&h) && !c.ret_used() {
            return Err(error(
                "The requested operation cannot be performed in this context.",
                "InvalidOperation",
            ));
        }
        return if c.ret_used() {
            Ok(Value::Boolean(false))
        } else {
            unsupported("Remove of a node without a parent, result unused (error text not pinned yet)")
        };
    }
    detach(&h);
    Ok(Value::Boolean(true))
}

pub(super) fn replace_with(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let h = recv(r)?;
    reject_unassigned_sibling_arg(&h, a)?;
    if parent_of(&h).is_none() && with(&h, |d| matches!(d, XData::DocType { .. })) {
        // DocTypeNode.al:ReplaceWith_DetachedDocType.
        return Err(error(
            "The requested operation cannot be performed in this context.",
            "InvalidOperation",
        ));
    }
    let (p, i) = position_in_parent(&h, a)?;
    // A comment or a processing instruction of a document that cannot be replaced is gone all the same, whether the nodes
    // are refused as an argument or in the document (xml/lt-xml-cdata-comment-pi-text/P90CommentRead.al:
    // DocLevel_BeforeSelf_AndReplace, P90PIRead.al).
    if kind(&p) == "XmlDocument" && with(&h, |d| matches!(d, XData::Comment(_) | XData::Pi { .. })) {
        detach(&h);
        let nodes = sibling_content_nodes(&h, a)?;
        reject_attributes(&nodes)?;
        for n in &nodes {
            check_fits(&p, n, &[], Some(i))?;
        }
        insert(&p, Some(i), nodes)?;
        return Ok(Value::Boolean(true));
    }
    // A text, CDATA, comment or processing instruction of an element is removed first, then the content is read and
    // added, so a failing argument leaves it removed and a node that is its own argument goes back in its place
    // (xml/lt-xml-cdata-comment-pi-text/P90CDataEdit.al:681 ReplaceWith_Attached_InvalidKinds `k6`..`k9`,
    // ReplaceWith_Attached_ExistingNodes `k11`, ReplaceWith_ThreeNodes_OneInvalid `t1_6_3`).
    if is_text_like(&h) {
        detach(&h);
        let nodes = sibling_content_nodes(&h, a)?;
        insert_siblings(&h, &p, i, nodes)?;
        return Ok(Value::Boolean(true));
    }
    let nodes = sibling_content_nodes(&h, a)?;
    reject_attributes(&nodes)?;
    // A document type that cannot be replaced is removed all the same (DocTypeNode.al:ReplaceWith_Element: the document
    // is `<a />` afterwards).
    if with(&h, |d| matches!(d, XData::DocType { .. })) {
        detach(&h);
        reject_declaration_in_document(&p, &nodes)?;
        for n in &nodes {
            check_fits(&p, n, &[], Some(i))?;
        }
        insert(&p, Some(i), nodes)?;
        return Ok(Value::Boolean(true));
    }
    for n in &nodes {
        check_fits(&p, n, std::slice::from_ref(&h), Some(i))?;
    }
    detach(&h);
    insert(&p, Some(i), nodes)?;
    Ok(Value::Boolean(true))
}

pub(super) fn remove_nodes(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> NResult {
    let h = recv(r)?;
    for c in children_of(&h) {
        detach(&c);
    }
    Ok(Value::Null)
}

pub(super) fn replace_nodes(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let h = recv(r)?;
    let nodes = content_nodes(a)?;
    let old = children_of(&h);
    for n in &nodes {
        check_fits(&h, n, &old, None)?;
    }
    remove_nodes(c, Some(&mut tagged("XmlElement", h.clone())), &mut [])?;
    insert(&h, None, nodes)?;
    Ok(Value::Boolean(true))
}

pub(super) fn get_root(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    // The statement form on an unassigned or cleared document raises XmlNodeNotFound (corpus branch al-runtime-rs-staging-ul
    // b753d3fb7a, xml/lt-ul-clear-ref/UlClearXmlRef.al:168-204 ClearXmlDocument_SecondVariable `cleared doc|root`).
    if !c.ret_used() && unassigned(&r, "XmlDocument") {
        // Pinned by xml/lt-ul-clear-ref/UlClearXmlRef.al (was assumption `clear-xml-ref`).
        return Err(error("The XML node is not set.", "XmlNodeNotFound"));
    }
    let h = recv(r)?;
    let root = children_of(&h)
        .into_iter()
        .find(|k| with(k, |d| matches!(d, XData::Element { .. })));
    match root {
        Some(e) => {
            a[0] = tagged("XmlElement", e);
            Ok(Value::Boolean(true))
        }
        None if c.ret_used() => Ok(Value::Boolean(false)),
        None => unsupported("GetRoot of a document without a root, result unused (error text not pinned yet)"),
    }
}

pub(super) fn get_declaration(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let h = recv(r)?;
    let d = with(&h, |d| match d {
        XData::Document { decl, .. } => decl.clone(),
        _ => None,
    });
    match d {
        Some(x) => {
            a[0] = tagged("XmlDeclaration", x);
            Ok(Value::Boolean(true))
        }
        None if c.ret_used() => Ok(Value::Boolean(false)),
        // DocTypeErrors.al:Uninitialised_DocumentMembers (an unassigned document answers the same).
        None => Err(error("The XML node is not set.", "XmlNodeNotFound")),
    }
}

pub(super) fn set_declaration(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let h = recv(r)?;
    if let Value::Library(_, d) = &a[0] {
        if matches!(&*d.borrow(), LibraryValue::Empty) {
            return Err(uninitialized("XmlDeclaration"));
        }
    }
    let d = node_of(&a[0])?;
    if kind(&d) != "XmlDeclaration" {
        return unsupported("SetDeclaration with something that is not an XmlDeclaration");
    }
    let d = if parent_of(&d).is_some() { deep_clone(&d) } else { d };
    if let Some(old) = with(&h, |x| match x {
        XData::Document { decl, .. } => decl.take(),
        _ => None,
    }) {
        set_parent(&old, None);
    }
    with(&h, |x| {
        if let XData::Document { decl, .. } = x {
            *decl = Some(d.clone());
        }
    });
    set_parent(&d, Some(&h));
    Ok(Value::Boolean(true))
}

/// The document type of a document: the one a program added (reading a document drops its DOCTYPE, so a read document has
/// none: xml/lt-xml-doctype-declaration/DocTypeRead.al). The Boolean form is false without one and the statement form
/// raises (xml/TestXmlRuntimeContracts.al:XmlDocument_GetDocumentType_WithoutDoctype, DocTypeInDoc.al:GetDocumentType_Statement).
pub(super) fn get_document_type(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let h = recv(r)?;
    match children_of(&h)
        .into_iter()
        .find(|k| with(k, |d| matches!(d, XData::DocType { .. })))
    {
        Some(dt) => {
            a[0] = tagged("XmlDocumentType", dt);
            Ok(Value::Boolean(true))
        }
        None if c.ret_used() => Ok(Value::Boolean(false)),
        None => Err(error("The XML node is not set.", "XmlNodeNotFound")),
    }
}

pub(super) fn get_document(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let h = recv_or_uninit(r, "XmlNode")?;
    let root = root_of(&h);
    // A declaration belongs to no document as far as GetDocument goes (xml/lt-xml-remaining-node-methods/P104TreeBDecl.al:
    // P104_Remove_InDoc `document1`).
    if with(&h, |d| !matches!(d, XData::Declaration { .. })) && with(&root, |d| matches!(d, XData::Document { .. })) {
        a[0] = tagged("XmlDocument", root);
        Ok(Value::Boolean(true))
    } else if c.ret_used() {
        Ok(Value::Boolean(false))
    } else {
        // xml/TestProbeXmlContracts.al:GetDocument_Detached_Statement
        Err(error("The XML node is not set.", "XmlNodeNotFound"))
    }
}

pub(super) fn get_parent(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let h = recv_or_uninit(r, "XmlNode")?;
    match parent_of(&h).filter(|p| with(p, |d| matches!(d, XData::Element { .. }))) {
        Some(p) => {
            a[0] = tagged("XmlElement", p);
            Ok(Value::Boolean(true))
        }
        None if c.ret_used() => Ok(Value::Boolean(false)),
        // xml/TestProbeXmlContracts.al:GetParent_Root_Statement
        None => Err(error("The XML node is not set.", "XmlNodeNotFound")),
    }
}
