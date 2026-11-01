//! Constructors and properties of the document, name, text, doctype, declaration and processing-instruction nodes.

use super::*;

pub(super) fn doc_create(_: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let doc = new_node(XData::Document {
        decl: None,
        children: Vec::new(),
    });
    for arg in a.iter() {
        if doc_blank_text(&doc, arg)? {
            continue;
        }
        insert(&doc, None, content_nodes(std::slice::from_ref(arg))?)?;
    }
    Ok(tagged("XmlDocument", doc))
}

/// `XmlDocument.Create` and `Add` of an empty or blank text to a document (misc/lt-probes-needed-1/PnAXmlNodeAdds.al
/// TT_EmptyOrBlankTextIntoXmlDocument on corpus writer TT's branch al-runtime-rs-staging-tt; former assumption `xml-doc-blank-text` (pinned on corpus master since bump 1030)): an
/// empty Text adds no node; a blank Text, or an empty or blank XmlText, is appended as a node before or after the root, and
/// `WriteTo` writes it as it is. A second text node in the document is not pinned. `false` for any other argument.
pub(super) fn doc_blank_text(doc: &Handle, arg: &Value) -> Result<bool, Signal> {
    if kind(doc) != "XmlDocument" {
        return Ok(false);
    }
    let node = match arg {
        Value::Text(t) | Value::Code(t) if t.is_empty() => {
            // Pinned by misc/lt-probes-needed-1/PnAXmlNodeAdds.al TT_EmptyOrBlankTextIntoXmlDocument (was assumption `xml-doc-blank-text`).
            return Ok(true);
        }
        Value::Text(t) | Value::Code(t) if t.trim().is_empty() => new_node(XData::Text(t.to_string())),
        Value::Library(_, h)
            if is_xml(h)
                && parent_of(h).is_none()
                && with(h, |d| matches!(d, XData::Text(t) if t.trim().is_empty())) =>
        {
            h.clone()
        }
        _ => return Ok(false),
    };
    if children_of(doc)
        .iter()
        .any(|c| with(c, |d| matches!(d, XData::Text(_))))
    {
        return unsupported("a second empty or blank text added to an XmlDocument (not pinned)");
    }
    // Pinned by misc/lt-probes-needed-1/PnAXmlNodeAdds.al TT_EmptyOrBlankTextIntoXmlDocument (was assumption `xml-doc-blank-text`).
    with(doc, |d| {
        if let XData::Document { children, .. } = d {
            children.push(node.clone());
        }
    });
    set_parent(&node, Some(doc));
    Ok(true)
}

pub(super) fn check_name(n: &str) -> Result<(), Signal> {
    if is_ncname(n) {
        Ok(())
    } else {
        unsupported(format!("XML name `{n}` (only plain names without a prefix are pinned)"))
    }
}

pub(super) fn element_create_name(_: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let name = arg_text(a, 0)?;
    check_name(&name)?;
    let el = new_node(XData::Element {
        prefix: String::new(),
        local: name,
        ns: String::new(),
        attrs: Vec::new(),
        children: Vec::new(),
        open: false,
    });
    insert(&el, None, content_nodes(&a[1..])?)?;
    Ok(tagged("XmlElement", el))
}

pub(super) fn element_create_ns(_: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let (local, ns) = (arg_text(a, 0)?, arg_text(a, 1)?);
    check_name(&local)?;
    let el = new_node(XData::Element {
        prefix: String::new(),
        local,
        ns,
        attrs: Vec::new(),
        children: Vec::new(),
        open: false,
    });
    insert(&el, None, content_nodes(&a[2..])?)?;
    Ok(tagged("XmlElement", el))
}

pub(super) fn plain_attribute(name: String, value: String) -> NResult {
    check_name(&name)?;
    Ok(tagged(
        "XmlAttribute",
        new_node(XData::Attribute {
            prefix: String::new(),
            local: name,
            ns: String::new(),
            value,
        }),
    ))
}

pub(super) fn attribute_create(_: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> NResult {
    plain_attribute(arg_text(a, 0)?, arg_text(a, 1)?)
}

pub(super) fn attribute_create_ns(_: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let (local, ns, value) = (arg_text(a, 0)?, arg_text(a, 1)?, arg_text(a, 2)?);
    if ns.is_empty() {
        return plain_attribute(local, value);
    }
    unsupported("XmlAttribute.Create with a namespace (the prefix it would get is not pinned)")
}

pub(super) fn attribute_create_ns_decl(_: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> NResult {
    // xml/TestXmlDeep.al XmlAttribute_CreateNamespaceDeclaration_HasPrefixAndUri: prefix `xmlns`.
    let (prefix, uri) = (arg_text(a, 0)?, arg_text(a, 1)?);
    let (p, l) = if prefix.is_empty() {
        (String::new(), "xmlns".to_string())
    } else {
        check_name(&prefix)?;
        ("xmlns".to_string(), prefix)
    };
    Ok(tagged(
        "XmlAttribute",
        new_node(XData::Attribute {
            prefix: p,
            local: l,
            ns: XMLNS_URI.into(),
            value: uri,
        }),
    ))
}

pub(super) fn text_like_create(tag: &'static str, a: &[Value]) -> NResult {
    let t = arg_text(a, 0)?;
    Ok(tagged(
        tag,
        new_node(match tag {
            "XmlText" => XData::Text(t),
            "XmlCData" => XData::CData(t),
            _ => XData::Comment(t),
        }),
    ))
}
pub(super) fn text_create(_: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> NResult {
    text_like_create("XmlText", a)
}
pub(super) fn cdata_create(_: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> NResult {
    text_like_create("XmlCData", a)
}
pub(super) fn comment_create(_: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> NResult {
    text_like_create("XmlComment", a)
}

/// The name of a document type, as `XmlConvert.VerifyName` takes it: an empty name is an argument error, a bad character
/// the XML exception with its place (xml/lt-xml-doctype-declaration/DocTypeErrors.al:Create_InvalidNames, DocTypeSetters.al:SetName).
pub(super) fn verify_name(name: &str) -> Result<(), Signal> {
    verify_name_with(name, true)
}

/// [`verify_name`] for a name without a prefix (`XmlConvert.VerifyNCName`): a colon is refused too (xml/lt-xml-attr-doc-http-members/
/// P97DocQuery.al:GetChildElements_LocalNameAndNamespace `*` and `b:r`).
pub(super) fn verify_ncname(name: &str) -> Result<(), Signal> {
    verify_name_with(name, false)
}

pub(super) fn verify_name_with(name: &str, colon: bool) -> Result<(), Signal> {
    if name.is_empty() {
        return Err(nav_argument());
    }
    let hex = |c: char| format!("0x{:02X}", u32::from(c));
    for (i, c) in name.chars().enumerate() {
        let start = c == '_' || (colon && c == ':') || c.is_alphabetic();
        let rest = start || c.is_alphanumeric() || c == '-' || c == '.' || c == '\u{B7}';
        let message = if i == 0 && !start {
            format!(
                "Name cannot begin with the '{c}' character, hexadecimal value {}.",
                hex(c)
            )
        } else if !rest {
            format!(
                "The '{c}' character, hexadecimal value {}, cannot be included in a name.",
                hex(c)
            )
        } else {
            continue;
        };
        return Err(error(
            format!(
                "The following exception was encountered when processing XML data: '{message}' at line 0 and position {}.",
                i + 1
            ),
            "Xml:Xml",
        ));
    }
    Ok(())
}

/// `XmlDocumentType.Create(Name, Public, System, Subset)` makes a usable type. `Create(Name)` and `Create(Name, Public,
/// System)` make one whose members raise `MemberNotInitialized` (DocTypeCreate.al:Create_NameOnly, Create_ThreeArguments);
/// the two-argument form is not pinned.
pub(super) fn doctype_create(_: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let name = arg_text(a, 0)?;
    verify_name(&name)?;
    let (public, system, subset, ready) = match a.len() {
        1 => (String::new(), String::new(), String::new(), false),
        // Written `<!DOCTYPE r PUBLIC "pub" "">` (xml/lt-xml-remaining-node-methods/P104WriteDocType.al:150 `v2_m1`,
        // P104DocTypeCreate.al `doc2`); its getters are not pinned (doctype_get refuses them).
        2 => (arg_text(a, 1)?, String::new(), String::new(), false),
        3 => (arg_text(a, 1)?, arg_text(a, 2)?, String::new(), false),
        4 => (arg_text(a, 1)?, arg_text(a, 2)?, arg_text(a, 3)?, true),
        n => return unsupported(format!("XmlDocumentType.Create with {n} arguments (not pinned)")),
    };
    let args = a.len() as u8;
    Ok(tagged(
        "XmlDocumentType",
        new_node(XData::DocType {
            name,
            public,
            system,
            subset,
            ready,
            args,
        }),
    ))
}

/// The type behind a receiver of a document type method; an unassigned variable raises `NotInitialized`.
pub(super) fn doctype_recv(r: Option<&mut Value>) -> Result<Handle, Signal> {
    recv_or_uninit(r, "XmlDocumentType")
}

macro_rules! doctype_get {
    ($fname:ident, $field:ident) => {
        pub(super) fn $fname(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
            let h = doctype_recv(r)?;
            let (text, ready, args) = with(&h, |d| match d {
                XData::DocType { $field, ready, args, .. } => ($field.clone(), *ready, *args),
                _ => (String::new(), true, 4),
            });
            if args == 2 {
                return unsupported("a getter of a document type made by Create(Name, Public) (not pinned)");
            }
            if !ready {
                return Err(error(
                    "Exception of type 'Microsoft.Dynamics.Nav.Types.Exceptions.NavNCLMemberNotInitializedException' was thrown.",
                    "MemberNotInitialized",
                ));
            }
            a[0] = Value::Text(text.into());
            Ok(Value::Boolean(true))
        }
    };
}
doctype_get!(doctype_get_name, name);
doctype_get!(doctype_get_public, public);
doctype_get!(doctype_get_system, system);
doctype_get!(doctype_get_subset, subset);

macro_rules! doctype_set {
    ($fname:ident, $field:ident, $check:expr) => {
        pub(super) fn $fname(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
            let h = doctype_recv(r)?;
            let v = arg_text(a, 0)?;
            let check: fn(&str) -> Result<(), Signal> = $check;
            check(&v)?;
            let ready = with(&h, |d| match d {
                XData::DocType { ready, .. } => *ready,
                _ => true,
            });
            if !ready {
                return unsupported(
                    "a setter of a document type made by Create(Name) or Create(Name, Public, System) (not pinned)",
                );
            }
            with(&h, |d| {
                if let XData::DocType { $field, .. } = d {
                    *$field = v;
                }
            });
            Ok(Value::Boolean(true))
        }
    };
}
doctype_set!(doctype_set_name, name, verify_name);
doctype_set!(doctype_set_public, public, |_| Ok(()));
doctype_set!(doctype_set_system, system, |_| Ok(()));
doctype_set!(doctype_set_subset, subset, |_| Ok(()));

/// `XmlProcessingInstruction.Create(Target, Data)`: an empty target and `xml` raise an Argument error, a target with a blank the
/// XML name error (xml/XmlNodes.al:203-208 `empty_target`, `xml_target`, `space_target`); data holding `?>` is taken (`bad_data`).
/// Other invalid targets are not pinned.
pub(super) fn pi_create(_: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let target = arg_text(a, 0)?;
    if target == "xml" {
        return Err(nav_argument());
    }
    check_pi_target(&target)?;
    Ok(tagged(
        "XmlProcessingInstruction",
        new_node(XData::Pi {
            target,
            data: arg_text(a, 1)?,
        }),
    ))
}

pub(super) fn decl_create(_: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> NResult {
    Ok(tagged(
        "XmlDeclaration",
        new_node(XData::Declaration {
            version: arg_text(a, 0)?,
            encoding: arg_text(a, 1)?,
            standalone: arg_text(a, 2)?,
            explicit: true,
        }),
    ))
}

macro_rules! decl_prop {
    ($name:ident, $field:ident) => {
        pub(super) fn $name(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
            let h = recv_or_uninit(r, "XmlDeclaration")?;
            if let Some(v) = a.first() {
                let t = crate::native::text_of(v)
                    .ok_or_else(|| Signal::Unsupported("non-text declaration value".into()))?;
                with(&h, |d| {
                    if let XData::Declaration { $field, .. } = d {
                        *$field = t.to_string();
                    }
                });
                return Ok(Value::Null);
            }
            Ok(Value::Text(
                with(&h, |d| match d {
                    XData::Declaration { $field, .. } => $field.clone(),
                    _ => String::new(),
                })
                .into(),
            ))
        }
    };
}
decl_prop!(decl_version, version);
decl_prop!(decl_encoding, encoding);
decl_prop!(decl_standalone, standalone);

pub(super) fn value_prop(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    // Reading the Value of an unassigned XmlAttribute raises NotInitialized (xml/XmlAttributes.al:519-524 Attribute_Uninitialized).
    if a.is_empty() && unassigned(&r, "XmlAttribute") {
        return Err(uninitialized("XmlAttribute"));
    }
    let h = recv(r)?;
    if let Some(v) = a.first() {
        let t = crate::native::text_of(v).ok_or_else(|| Signal::Unsupported("non-text XML value".into()))?;
        with(&h, |d| match d {
            XData::Attribute { value, .. } => *value = t.to_string(),
            XData::Text(s) | XData::CData(s) | XData::Comment(s) => *s = t.to_string(),
            _ => {}
        });
        return Ok(Value::Null);
    }
    Ok(Value::Text(
        with(&h, |d| match d {
            XData::Attribute { value, .. } => value.clone(),
            XData::Text(s) | XData::CData(s) | XData::Comment(s) => s.clone(),
            _ => String::new(),
        })
        .into(),
    ))
}

pub(super) fn pi_get_target(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let h = recv(r)?;
    a[0] = Value::Text(
        with(&h, |d| match d {
            XData::Pi { target, .. } => target.clone(),
            _ => String::new(),
        })
        .into(),
    );
    Ok(Value::Boolean(true))
}

pub(super) fn pi_get_data(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let h = recv(r)?;
    // What GetData reads of data holding the end marker is not pinned (only how it is written).
    if with(&h, |d| matches!(d, XData::Pi { data, .. } if data.contains("?>"))) {
        return unsupported("XmlProcessingInstruction.GetData of data holding `?>` (not pinned)");
    }
    a[0] = Value::Text(
        with(&h, |d| match d {
            XData::Pi { data, .. } => data.clone(),
            _ => String::new(),
        })
        .into(),
    );
    Ok(Value::Boolean(true))
}

/// A processing instruction target: an empty one raises an Argument error and one with a blank (after a valid start) the XML
/// name error (P90PIRead.al:2215, xml/XmlNodes.al:203-208); another valid name is taken. Other invalid names, and `xml` in
/// any case, are not pinned (Create raises for `xml` itself).
pub(super) fn check_pi_target(name: &str) -> Result<(), Signal> {
    if let Err(e) = verify_name(name) {
        let chars: Vec<char> = name.chars().collect();
        let first_bad = chars.iter().position(|c| !is_name_char(*c));
        let pinned = chars.is_empty()
            || (chars.first().copied().is_some_and(is_name_start) && first_bad.is_some_and(|i| chars[i] == ' '));
        if pinned {
            return Err(e);
        }
        return unsupported(format!("the processing instruction target `{name}` (not pinned)"));
    }
    if name.eq_ignore_ascii_case("xml") {
        return unsupported(format!("the processing instruction target `{name}` (not pinned)"));
    }
    Ok(())
}

/// `XmlProcessingInstruction.SetTarget(Name)` (xml/lt-xml-cdata-comment-pi-text/P90PIRead.al:2215 PI_GetSetTargetAndData):
/// `t2` is written `<?t2 data1?>`; an empty name raises an Argument error and a name with a blank the XML name error, both
/// leaving the target as it was. Other invalid names, and names starting with `xml`, are not pinned.
pub(super) fn pi_set_target(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let h = recv(r)?;
    let name = arg_text(a, 0)?;
    check_pi_target(&name)?;
    with(&h, |d| {
        if let XData::Pi { target, .. } = d {
            *target = name;
        }
    });
    Ok(Value::Null)
}

/// `XmlProcessingInstruction.SetData(Data)`: an empty text leaves `<?t2?>` and GetData reads it empty (P90PIRead.al:2215
/// `doc-after-empty-data`; xml/XmlNodes.al:199 `setdata`); `?>` is taken and written `? >` (P90PIRead.al:2110 `pi-end-doc`).
pub(super) fn pi_set_data(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let h = recv(r)?;
    let v = arg_text(a, 0)?;
    with(&h, |d| {
        if let XData::Pi { data, .. } = d {
            *data = v;
        }
    });
    Ok(Value::Null)
}
