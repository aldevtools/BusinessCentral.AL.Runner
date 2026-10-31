//! Natives that test node kinds and list children and descendants.

use super::*;

pub(super) fn as_token(tag: &'static str, r: Option<&mut Value>) -> NResult {
    Ok(tagged(tag, recv(r)?))
}
pub(super) fn as_xml_node(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> NResult {
    // An unassigned document type or declaration converts to an unassigned node (DocTypeErrors.al:Uninitialised_SettersAndMethods).
    if let Some(Value::Library(name, h)) = r.as_deref() {
        if matches!(&*h.borrow(), LibraryValue::Empty) && matches!(name.as_str(), "XmlDocumentType" | "XmlDeclaration")
        {
            return Ok(tagged("XmlNode", h.clone()));
        }
    }
    as_token("XmlNode", r)
}

macro_rules! node_kind {
    ($is:ident, $as:ident, $kind:literal) => {
        pub(super) fn $is(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> NResult {
            let h = recv(r)?;
            Ok(Value::Boolean(kind(&h) == $kind))
        }
        pub(super) fn $as(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> NResult {
            let h = recv(r)?;
            if kind(&h) == $kind {
                Ok(tagged($kind, h))
            } else {
                // xml/TestProbeXmlContracts.al:XPath_CountsForPaths (AsXmlElement on an attribute and on a text).
                Err(error(
                    format!(
                        "Unable to convert from Microsoft.Dynamics.Nav.Runtime.Nav{} to Microsoft.Dynamics.Nav.Runtime.Nav{}.",
                        kind(&h),
                        $kind
                    ),
                    "Conversion",
                ))
            }
        }
    };
}
node_kind!(is_attribute, as_attribute, "XmlAttribute");
node_kind!(is_cdata, as_cdata, "XmlCData");
node_kind!(is_comment, as_comment, "XmlComment");
node_kind!(is_declaration, as_declaration, "XmlDeclaration");
node_kind!(is_doctype, as_doctype, "XmlDocumentType");
node_kind!(is_document, as_document, "XmlDocument");
node_kind!(is_element_node, as_element_node, "XmlElement");

/// An unassigned or cleared XmlNode is no element, and AsXmlElement of it raises a Conversion error (corpus branch
/// al-runtime-rs-staging-ul b753d3fb7a, xml/lt-ul-clear-ref/UlClearXmlRef.al:106-142 ClearXmlNode_SecondVariable).
pub(super) fn is_element(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    if unassigned(&r, "XmlNode") {
        // Pinned by xml/lt-ul-clear-ref/UlClearXmlRef.al (was assumption `clear-xml-ref`).
        return Ok(Value::Boolean(false));
    }
    is_element_node(c, r, a)
}
pub(super) fn as_element(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    if unassigned(&r, "XmlNode") {
        // Pinned by xml/lt-ul-clear-ref/UlClearXmlRef.al (was assumption `clear-xml-ref`).
        return Err(error(
            "Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavXmlNode to Microsoft.Dynamics.Nav.Runtime.NavXmlElement.",
            "Conversion",
        ));
    }
    as_element_node(c, r, a)
}
node_kind!(is_pi, as_pi, "XmlProcessingInstruction");

/// `IsXmlText` is true for a text node only; a CDATA section is not text
/// (xml/TestXmlRuntimeContracts.al:XmlNode_CDataIsNotText).
pub(super) fn is_text(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> NResult {
    let h = recv(r)?;
    Ok(Value::Boolean(kind(&h) == "XmlText"))
}
pub(super) fn as_text(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> NResult {
    let h = recv(r)?;
    if kind(&h) == "XmlText" {
        Ok(tagged("XmlText", h))
    } else {
        Err(error(
            format!(
                "Unable to convert from Microsoft.Dynamics.Nav.Runtime.Nav{} to Microsoft.Dynamics.Nav.Runtime.NavXmlText.",
                kind(&h)
            ),
            "Conversion",
        ))
    }
}

pub(super) fn has_attributes(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> NResult {
    Ok(Value::Boolean(!attrs_of(&recv(r)?).is_empty()))
}

pub(super) fn has_elements(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> NResult {
    let h = recv(r)?;
    Ok(Value::Boolean(
        children_of(&h)
            .iter()
            .any(|c| with(c, |d| matches!(d, XData::Element { .. }))),
    ))
}

pub(super) fn is_empty(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> NResult {
    Ok(Value::Boolean(children_of(&recv(r)?).is_empty()))
}

pub(super) fn inner_text_fn(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> NResult {
    // xml/XmlElement.al:824 Default_Element_Uninitialized `inner`.
    if unassigned(&r, "XmlElement") {
        return Err(uninitialized("XmlElement"));
    }
    let mut s = String::new();
    inner_text(&recv(r)?, &mut s);
    Ok(Value::Text(s.into()))
}

pub(super) fn inner_xml(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> NResult {
    let h = recv(r)?;
    Ok(Value::Text(serialize_inner(&h)?.into()))
}

pub(super) fn child_elements(h: &Handle, name: Option<(&str, Option<&str>)>) -> Vec<Handle> {
    children_of(h)
        .into_iter()
        .filter(|c| {
            with(c, |d| match (d, name) {
                (XData::Element { .. }, None) => true,
                (XData::Element { local, ns, .. }, Some((n, uri))) => local == n && ns == uri.unwrap_or(""),
                _ => false,
            })
        })
        .collect()
}

pub(super) fn get_child_elements(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let h = recv(r)?;
    Ok(match a.len() {
        0 => list_of(child_elements(&h, None)),
        1 => {
            let n = arg_text(a, 0)?;
            check_name(&n)?;
            list_of(child_elements(&h, Some((&n, None))))
        }
        _ => {
            let (n, u) = (arg_text(a, 0)?, arg_text(a, 1)?);
            verify_ncname(&n)?;
            list_of(child_elements(&h, Some((&n, Some(&u)))))
        }
    })
}

pub(super) fn get_child_nodes(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> NResult {
    Ok(list_of(children_of(&recv(r)?)))
}

pub(super) fn all_descendants(h: &Handle, out: &mut Vec<Handle>) {
    for c in children_of(h) {
        out.push(c.clone());
        all_descendants(&c, out);
    }
}

pub(super) fn get_descendant_nodes(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> NResult {
    let mut v = Vec::new();
    all_descendants(&recv(r)?, &mut v);
    Ok(list_of(v))
}

pub(super) fn get_descendant_elements(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let h = recv(r)?;
    let mut v = Vec::new();
    all_descendants(&h, &mut v);
    let want: Option<(String, String)> = match a.len() {
        0 => None,
        1 => {
            let n = arg_text(a, 0)?;
            check_name(&n)?;
            Some((n, String::new()))
        }
        _ => {
            let (n, u) = (arg_text(a, 0)?, arg_text(a, 1)?);
            verify_ncname(&n)?;
            Some((n, u))
        }
    };
    v.retain(|c| {
        with(c, |d| match (d, &want) {
            (XData::Element { .. }, None) => true,
            (XData::Element { local, ns, .. }, Some((n, u))) => local == n && ns == u,
            _ => false,
        })
    });
    Ok(list_of(v))
}
