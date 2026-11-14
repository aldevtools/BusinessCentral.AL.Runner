//! The node tree: cell and data types, handles, the shared helpers on them and namespace lookup.

use super::*;

pub const XMLNS_URI: &str = "http://www.w3.org/2000/xmlns/";
pub(super) const XML_URI: &str = "http://www.w3.org/XML/1998/namespace";

#[derive(Debug)]
pub struct XmlCell {
    pub data: XData,
    pub parent: Option<Weak<RefCell<LibraryValue>>>,
    /// Set the first time the node gets a parent and never cleared: a node taken off its parent is not a new one
    /// (`never_attached`).
    pub had_parent: bool,
    /// An attribute that `XmlElement.SetAttribute(Name, NamespaceUri, Value)` made (`attrs::set_attribute_ns`); a copy does not
    /// keep it.
    pub set_with_namespace: bool,
}

#[derive(Debug)]
pub enum XData {
    Document {
        decl: Option<Handle>,
        children: Vec<Handle>,
    },
    Element {
        prefix: String,
        local: String,
        ns: String,
        attrs: Vec<Handle>,
        children: Vec<Handle>,
        /// Parsed from `<a></a>`: written with an end tag although empty (xml/TestProbeXmlContracts.al:
        /// WriteTo_EmptyElementFromParsedText); `<a/>` and a created element are written `<a />`.
        open: bool,
    },
    Attribute {
        prefix: String,
        local: String,
        ns: String,
        value: String,
    },
    Text(String),
    CData(String),
    Comment(String),
    Pi {
        target: String,
        data: String,
    },
    /// `XmlDocumentType`. `ready` is false for one made by `Create(Name)` or `Create(Name, Public, System)`: its members raise
    /// `MemberNotInitialized` (xml/xml/DocTypeCreate.al:Create_NameOnly, Create_ThreeArguments).
    DocType {
        name: String,
        public: String,
        system: String,
        subset: String,
        ready: bool,
        /// How many arguments `Create` had (4 for a type read from text); it decides how a type that is not `ready` is written.
        args: u8,
    },
    Declaration {
        version: String,
        encoding: String,
        standalone: String,
        /// Made by `XmlDeclaration.Create`: the encoding and standalone attributes are written even when empty
        /// (xml/TestProbeXmlContracts.al:WriteTo_DeclarationAndIndent).
        explicit: bool,
    },
}

/// `Exception of type 'Microsoft.Dynamics.Nav.Types.Exceptions.NavNCLArgumentException' was thrown.` (code `Argument`).
pub(super) fn nav_argument() -> Signal {
    error(
        "Exception of type 'Microsoft.Dynamics.Nav.Types.Exceptions.NavNCLArgumentException' was thrown.",
        "Argument",
    )
}

pub(super) fn error(message: impl Into<String>, code: &str) -> Signal {
    Signal::Error(Box::new(crate::native::AlError {
        message: message.into(),
        code: code.into(),
        call_stack: String::new(),
        ..Default::default()
    }))
}

/// `Microsoft.Dynamics.Nav.Runtime.NavXmlDocumentType variable not initialized.` (code NotInitialized): a variable nobody assigned,
/// used where the type is known (xml/xml/DocTypeErrors.al:Uninitialised_*).
pub(crate) fn uninitialized(nav_type: &str) -> Signal {
    error(
        format!("Microsoft.Dynamics.Nav.Runtime.Nav{nav_type} variable not initialized."),
        "NotInitialized",
    )
}

/// The node behind a receiver that may be a variable nobody assigned: the node-level methods (`WriteTo`, `Remove`) name
/// `XmlNode` in the error, the type's own methods name the type. Pinned for a document type and a declaration
/// (xml/xml/DocTypeErrors.al:Uninitialised_SettersAndMethods, P104WriteDecl.al:340), and for a text, CDATA section, comment
/// and processing instruction (xml/xml/P90{Text,CData,Comment,PI}Read.al UninitialisedVariable_2 and _3: Remove, AddAfterSelf,
/// AddBeforeSelf, GetDocument, GetParent, ReplaceWith, SelectNodes, SelectSingleNode and WriteTo name `XmlNode`; Value,
/// GetData and SetData name the type; P104TreeB{Text,CData,Comment,PI}.al P104_Remove_Uninitialised).
pub(super) fn recv_or_uninit(r: Option<&mut Value>, nav_type: &str) -> Result<Handle, Signal> {
    if let Some(Value::Library(name, h)) = r.as_deref() {
        if matches!(&*h.borrow(), LibraryValue::Empty)
            && matches!(
                name.as_str(),
                "XmlDocumentType"
                    | "XmlDeclaration"
                    | "XmlText"
                    | "XmlCData"
                    | "XmlComment"
                    | "XmlProcessingInstruction"
            )
        {
            return Err(uninitialized(nav_type));
        }
    }
    recv(r)
}

/// The node behind the receiver of a node-level method that only the text kinds pin for an unassigned variable (AddAfterSelf,
/// AddBeforeSelf, ReplaceWith, SelectNodes, SelectSingleNode; xml/xml/P90{Text,CData,Comment,PI}Read.al
/// UninitialisedVariable_2): an unassigned text, CDATA section, comment or processing instruction raises `NotInitialized`
/// naming `XmlNode`; another unassigned kind is refused as before.
pub(super) fn recv_text_kind_or_uninit(r: Option<&mut Value>) -> Result<Handle, Signal> {
    if let Some(Value::Library(name, h)) = r.as_deref() {
        if matches!(&*h.borrow(), LibraryValue::Empty)
            && matches!(
                name.as_str(),
                "XmlText" | "XmlCData" | "XmlComment" | "XmlProcessingInstruction"
            )
        {
            return Err(uninitialized("XmlNode"));
        }
    }
    recv(r)
}

/// The node behind the receiver of `WriteTo`: an unassigned node of the kinds [`recv_or_uninit`] names, or an unassigned
/// `XmlNode`, raises `NotInitialized` naming `XmlNode` (xml/xml/P104WriteNode.al:337-355 P104_WriteTo_Uninitialised,
/// `WriteTo(Text)`, `WriteTo(XmlWriteOptions, Text)`, `WriteTo(OutStream)` and `WriteTo(XmlWriteOptions, OutStream)`; the
/// same for a document type, P104WriteDocType.al:337-355).
pub(super) fn recv_write_to(r: Option<&mut Value>) -> Result<Handle, Signal> {
    if unassigned(&r, "XmlNode") {
        return Err(uninitialized("XmlNode"));
    }
    recv_or_uninit(r, "XmlNode")
}

pub(super) fn new_node(data: XData) -> Handle {
    let h = Rc::new(RefCell::new(LibraryValue::Xml(XmlCell {
        data,
        parent: None,
        had_parent: false,
        set_with_namespace: false,
    })));
    adopt_children(&h);
    h
}

pub(super) fn tagged(name: &str, h: Handle) -> Value {
    Value::Library(name.into(), h)
}

pub(super) fn is_xml(h: &Handle) -> bool {
    matches!(&*h.borrow(), LibraryValue::Xml(_))
}

/// The node behind an XML handle value.
pub(super) fn node_of(v: &Value) -> Result<Handle, Signal> {
    match v {
        Value::Library(name, h) => {
            if is_xml(h) {
                Ok(h.clone())
            } else if matches!(&*h.borrow(), LibraryValue::Empty) {
                // An XmlDocument variable that was never assigned is an empty document
                // (xml/TestXmlRuntimeContracts.al:XmlDocument_Add_SecondRoot_Throws adds to one).
                if name == "XmlDocument" {
                    *h.borrow_mut() = LibraryValue::Xml(XmlCell {
                        data: XData::Document {
                            decl: None,
                            children: Vec::new(),
                        },
                        parent: None,
                        had_parent: false,
                        set_with_namespace: false,
                    });
                    Ok(h.clone())
                } else {
                    unsupported(format!("use of an unassigned {name}"))
                }
            } else {
                unsupported(format!("{name} used as an XML node"))
            }
        }
        other => unsupported(format!("{} used as an XML node", other.describe())),
    }
}

pub(super) fn recv(r: Option<&mut Value>) -> Result<Handle, Signal> {
    match r {
        Some(v) => node_of(v),
        None => unsupported("XML method without a receiver"),
    }
}

pub(super) fn with<T>(h: &Handle, f: impl FnOnce(&mut XData) -> T) -> T {
    match &mut *h.borrow_mut() {
        LibraryValue::Xml(c) => f(&mut c.data),
        _ => unreachable!("checked by node_of"),
    }
}

pub(super) fn parent_of(h: &Handle) -> Option<Handle> {
    match &*h.borrow() {
        LibraryValue::Xml(c) => c.parent.as_ref().and_then(Weak::upgrade),
        _ => None,
    }
}

/// Whether `h` had a parent that no variable references any more.
pub(super) fn parent_dropped(h: &Handle) -> bool {
    match &*h.borrow() {
        LibraryValue::Xml(c) => c.parent.as_ref().is_some_and(|w| w.upgrade().is_none()),
        _ => false,
    }
}

pub(super) fn set_parent(child: &Handle, parent: Option<&Handle>) {
    if let LibraryValue::Xml(c) = &mut *child.borrow_mut() {
        c.parent = parent.map(Rc::downgrade);
        c.had_parent |= parent.is_some();
    }
}

/// Whether `h` is an attribute without a prefix and without a namespace.
pub(super) fn attribute_without_namespace(h: &Handle) -> bool {
    with(
        h,
        |d| matches!(d, XData::Attribute { prefix, ns, .. } if prefix.is_empty() && ns.is_empty()),
    )
}

/// Whether `h` was made by a Create (or a copy) and never had a parent: neither attached now nor taken off one, nor left by a
/// parent no variable references any more.
pub(super) fn never_attached(h: &Handle) -> bool {
    match &*h.borrow() {
        LibraryValue::Xml(c) => c.parent.is_none() && !c.had_parent,
        _ => false,
    }
}

pub(super) fn adopt_children(h: &Handle) {
    let (kids, extra): (Vec<Handle>, Vec<Handle>) = with(h, |d| match d {
        XData::Document { decl, children } => (children.clone(), decl.iter().cloned().collect()),
        XData::Element { attrs, children, .. } => (children.clone(), attrs.clone()),
        _ => (Vec::new(), Vec::new()),
    });
    for k in kids.iter().chain(&extra) {
        set_parent(k, Some(h));
    }
}

pub(super) fn children_of(h: &Handle) -> Vec<Handle> {
    with(h, |d| match d {
        XData::Document { children, .. } | XData::Element { children, .. } => children.clone(),
        _ => Vec::new(),
    })
}

pub(super) fn attrs_of(h: &Handle) -> Vec<Handle> {
    with(h, |d| match d {
        XData::Element { attrs, .. } => attrs.clone(),
        _ => Vec::new(),
    })
}

pub(super) fn kind(h: &Handle) -> &'static str {
    with(h, |d| match d {
        XData::Document { .. } => "XmlDocument",
        XData::Element { .. } => "XmlElement",
        XData::Attribute { .. } => "XmlAttribute",
        XData::Text(_) => "XmlText",
        XData::CData(_) => "XmlCData",
        XData::Comment(_) => "XmlComment",
        XData::Pi { .. } => "XmlProcessingInstruction",
        XData::Declaration { .. } => "XmlDeclaration",
        XData::DocType { .. } => "XmlDocumentType",
    })
}

pub(super) fn deep_clone(h: &Handle) -> Handle {
    let data = with(h, |d| match d {
        XData::Document { decl, children } => XData::Document {
            decl: decl.as_ref().map(deep_clone),
            children: children.iter().map(deep_clone).collect(),
        },
        XData::Element {
            prefix,
            local,
            ns,
            attrs,
            children,
            open,
        } => XData::Element {
            prefix: prefix.clone(),
            local: local.clone(),
            ns: ns.clone(),
            attrs: attrs.iter().map(deep_clone).collect(),
            children: children.iter().map(deep_clone).collect(),
            open: *open,
        },
        XData::Attribute {
            prefix,
            local,
            ns,
            value,
        } => XData::Attribute {
            prefix: prefix.clone(),
            local: local.clone(),
            ns: ns.clone(),
            value: value.clone(),
        },
        XData::Text(t) => XData::Text(t.clone()),
        XData::CData(t) => XData::CData(t.clone()),
        XData::Comment(t) => XData::Comment(t.clone()),
        XData::Pi { target, data } => XData::Pi {
            target: target.clone(),
            data: data.clone(),
        },
        XData::Declaration {
            version,
            encoding,
            standalone,
            explicit,
        } => XData::Declaration {
            version: version.clone(),
            encoding: encoding.clone(),
            standalone: standalone.clone(),
            explicit: *explicit,
        },
        XData::DocType {
            name,
            public,
            system,
            subset,
            ready,
            args,
        } => XData::DocType {
            name: name.clone(),
            public: public.clone(),
            system: system.clone(),
            subset: subset.clone(),
            ready: *ready,
            args: *args,
        },
    });
    new_node(data)
}

pub(super) fn root_of(h: &Handle) -> Handle {
    let mut cur = h.clone();
    while let Some(p) = parent_of(&cur) {
        cur = p;
    }
    cur
}

pub(super) fn is_ancestor_or_self(maybe_ancestor: &Handle, h: &Handle) -> bool {
    let mut cur = Some(h.clone());
    while let Some(c) = cur {
        if Rc::ptr_eq(&c, maybe_ancestor) {
            return true;
        }
        cur = parent_of(&c);
    }
    false
}

// ------------------------------------------------------------- names

pub(super) fn is_name_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

pub(super) fn is_name_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | '\u{b7}')
}

pub(super) fn is_ncname(s: &str) -> bool {
    let mut it = s.chars();
    it.next().is_some_and(is_name_start) && it.all(is_name_char)
}

pub(super) fn qname(prefix: &str, local: &str) -> String {
    if prefix.is_empty() {
        local.to_string()
    } else {
        format!("{prefix}:{local}")
    }
}

pub(super) fn is_xmlns_decl(h: &Handle) -> bool {
    with(h, |d| matches!(d, XData::Attribute { ns, .. } if ns == XMLNS_URI))
}

/// Declared namespace of `prefix` as seen from `h` (walking up through ancestors).
pub(super) fn lookup_namespace(h: &Handle, prefix: &str) -> Option<String> {
    let mut cur = Some(h.clone());
    while let Some(c) = cur {
        for a in attrs_of(&c) {
            let hit = with(&a, |d| match d {
                XData::Attribute {
                    prefix: p,
                    local,
                    ns,
                    value,
                } if ns == XMLNS_URI => {
                    // `xmlns="u"` is local `xmlns` with no prefix; `xmlns:p="u"` is prefix `xmlns`, local `p`.
                    let declared = if p.is_empty() && local == "xmlns" {
                        Some("")
                    } else if p == "xmlns" {
                        Some(local.as_str())
                    } else {
                        None
                    };
                    declared.filter(|d| *d == prefix).map(|_| value.clone())
                }
                _ => None,
            });
            if hit.is_some() {
                return hit;
            }
        }
        cur = parent_of(&c);
    }
    None
}
