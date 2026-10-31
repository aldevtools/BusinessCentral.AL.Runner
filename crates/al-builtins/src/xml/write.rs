//! Serialising a node tree to XML text.

use super::*;

// ---------------------------------------------------------- serialising

pub(crate) fn escape_text(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\r' => out.push_str("&#xD;"),
            c => out.push(c),
        }
    }
}

/// Text of a DOM node: a carriage return stays as it is, and the writer turns every line end into CR LF
/// (xml/XmlEntitiesMisc.al:Entities_CharRefs, `&#13;` writes `\r\n`).
pub(super) fn escape_dom_text(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            c => out.push(c),
        }
    }
}

pub(crate) fn escape_attr(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\n' => out.push_str("&#xA;"),
            '\r' => out.push_str("&#xD;"),
            '\t' => out.push_str("&#x9;"),
            c => out.push(c),
        }
    }
}

/// The prefix mapping visible while writing: `(prefix, uri)`, innermost last.
pub(super) type Scope = Vec<(String, String)>;

pub(super) fn mapped(scope: &Scope, prefix: &str) -> Option<String> {
    scope.iter().rev().find(|(p, _)| p == prefix).map(|(_, u)| u.clone())
}

/// The state of .NET's indenting text writer, which BC writes through: two spaces per level, LF,
/// and no indentation inside an element that has text (mixed content). The rules, each pinned by
/// xml/TestXmlRuntimeContracts.al:XmlDocument_WriteTo_HasDeclarationAndTwoSpaceIndentation,
/// :XmlElement_WriteTo_MixedContentIsNotIndented and
/// :XmlDocument_ReadFrom_KeepsWhiteSpaceInsideAndOutsideTheRoot.
pub(super) struct Writer {
    out: String,
    indent: bool,
    level: usize,
    /// The current element has had text, CDATA or white space written into it.
    mixed: bool,
    /// `mixed` of the enclosing elements.
    saved: Vec<bool>,
    /// Where the last text ended; a markup node starts on a new line when more was written since.
    text_end: usize,
    scope: Scope,
    /// Line ends in text are written as CR LF (not in `InnerXml`).
    crlf: bool,
}

impl Writer {
    fn new(indent: bool) -> Writer {
        Writer {
            out: String::new(),
            indent,
            level: 0,
            mixed: false,
            saved: Vec::new(),
            text_end: 0,
            scope: Vec::new(),
            crlf: true,
        }
    }

    fn break_line(&mut self) {
        if self.indent && !self.mixed && self.out.len() != self.text_end {
            self.out.push_str("\r\n");
            for _ in 0..self.level {
                self.out.push_str("  ");
            }
        }
    }

    /// Every line end the writer is given is written as CR LF, as .NET's writer replaces them (corpus
    /// xml/TestXmlRuntimeContracts.al, misc/lt-probes-needed-1/PnAXmlWriteTo.al: `\r\n` where the source has `\n`).
    fn push_text(&mut self, s: &str) {
        if self.crlf && s.contains(['\n', '\r']) {
            self.out
                .push_str(&s.replace("\r\n", "\n").replace('\r', "\n").replace('\n', "\r\n"));
        } else {
            self.out.push_str(s);
        }
    }

    fn text(&mut self, s: &str) {
        self.mixed = true;
        self.push_text(s);
        self.text_end = self.out.len();
    }

    /// White space between the nodes of a document (outside the root): written as it is and not content of any element, so
    /// it does not make the root's content mixed (xml/TestXmlRuntimeContracts.al:XmlDocument_ReadFrom_KeepsWhiteSpaceInsideAndOutsideTheRoot).
    fn document_text(&mut self, s: &str) {
        self.push_text(s);
        self.text_end = self.out.len();
    }

    fn markup(&mut self, s: &str) {
        self.break_line();
        self.push_text(s);
    }

    fn node(&mut self, h: &Handle) -> Result<(), Signal> {
        enum Plan {
            Element {
                prefix: String,
                local: String,
                ns: String,
                attrs: Vec<Handle>,
                children: Vec<Handle>,
                open: bool,
            },
            Text(String),
            CData(String),
            Markup(String),
            Quote,
            Skip,
        }
        let plan = with(h, |d| match d {
            XData::Element {
                prefix,
                local,
                ns,
                attrs,
                children,
                open,
            } => Plan::Element {
                prefix: prefix.clone(),
                local: local.clone(),
                ns: ns.clone(),
                attrs: attrs.clone(),
                children: children.clone(),
                open: *open,
            },
            XData::Text(t) => {
                let mut s = String::new();
                escape_dom_text(t, &mut s);
                Plan::Text(s)
            }
            // An end marker inside the text splits the section (xml/XmlNodes.al:CData_Create `contains_end`: `a]]>b` is written
            // `<![CDATA[a]]]]><![CDATA[>b]]>`).
            XData::CData(t) => Plan::CData(format!("<![CDATA[{}]]>", t.replace("]]>", "]]]]><![CDATA[>"))),
            // Two hyphens in a row are separated by a blank (xml/XmlNodes.al:Comment_Create `double_dash`: `a--b` is written `<!--a- -b-->`).
            XData::Comment(t) => Plan::Markup(format!(
                "<!--{}{}-->",
                t.replace("--", "- -"),
                if t.ends_with('-') { " " } else { "" }
            )),
            // The end marker inside the data is written `? >` (xml/lt-xml-cdata-comment-pi-text/P90PIRead.al:1989 `pi-end-x`,
            // :2110 `pi-end-doc`).
            XData::Pi { target, data } => Plan::Markup(if data.is_empty() {
                format!("<?{target}?>")
            } else {
                format!("<?{target} {}?>", data.replace("?>", "? >"))
            }),
            XData::DocType {
                name,
                public,
                system,
                subset,
                ready,
                args,
            } => {
                if !*ready {
                    // A type made without a subset (xml/lt-xml-remaining-node-methods/P104WriteDocType.al:146..157,
                    // P104DocTypeCreate.al `doc1`..`doc3`): `<!DOCTYPE r >` for a name only, `<!DOCTYPE r PUBLIC "pub" "sys">`
                    // with a public identifier. An empty public identifier with Create(Name, Public[, System]) is not pinned.
                    if *args == 1 {
                        Plan::Markup(format!("<!DOCTYPE {name} >"))
                    } else if public.is_empty() || public.contains('"') || system.contains('"') {
                        Plan::Skip
                    } else {
                        Plan::Markup(format!("<!DOCTYPE {name} PUBLIC \"{public}\" \"{system}\">"))
                    }
                } else if public.contains('"') || system.contains('"') {
                    // A quote cannot be written inside the quoted identifiers (DocTypeSetters.al:WriteToVariants, af214).
                    Plan::Quote
                } else {
                    Plan::Markup(format!("<!DOCTYPE {name} PUBLIC \"{public}\" \"{system}\"[{subset}]>"))
                }
            }
            XData::Declaration {
                version,
                encoding,
                standalone,
                explicit,
            } => Plan::Markup(declaration_text(version, encoding, standalone, *explicit)),
            XData::Attribute { .. } | XData::Document { .. } => Plan::Skip,
        });
        match plan {
            Plan::Text(s) | Plan::CData(s) => self.text(&s),
            Plan::Markup(s) => self.markup(&s),
            Plan::Quote => return Err(nav_argument()),
            Plan::Skip => return unsupported(format!("serialising {} as markup", kind(h))),
            Plan::Element {
                prefix,
                local,
                ns,
                attrs,
                children,
                open,
            } => self.element(&prefix, &local, &ns, &attrs, &children, open)?,
        }
        Ok(())
    }

    fn element(
        &mut self,
        prefix: &str,
        local: &str,
        ns: &str,
        attrs: &[Handle],
        children: &[Handle],
        open: bool,
    ) -> Result<(), Signal> {
        let mark = self.scope.len();
        // Declarations written on this element: explicit ones first, in their order.
        let mut written: Vec<String> = Vec::new();
        let mut plain: Vec<(String, String, String, String)> = Vec::new(); // prefix, local, ns, value
        for a in attrs {
            let (ap, al, ans, av) = with(a, |d| match d {
                XData::Attribute {
                    prefix,
                    local,
                    ns,
                    value,
                } => (prefix.clone(), local.clone(), ns.clone(), value.clone()),
                _ => unreachable!("attributes hold attributes"),
            });
            if ans == XMLNS_URI {
                let declared = if ap == "xmlns" { al.clone() } else { String::new() };
                self.scope.push((declared.clone(), av.clone()));
                let mut s = String::new();
                escape_attr(&av, &mut s);
                written.push(if declared.is_empty() {
                    format!(" xmlns=\"{s}\"")
                } else {
                    format!(" xmlns:{declared}=\"{s}\"")
                });
            } else if ap.is_empty() && al == "xmlns" && ans.is_empty() {
                // `XmlAttribute.Create('xmlns', Uri)` on an element: the same URI as the element's own namespace is written once,
                // another one raises when the start tag is written (xml/lt-de-xml-leads/DeXmlWriteLeads.al:246-310). On an element
                // with a prefix it is not pinned.
                if !prefix.is_empty() {
                    return unsupported("an xmlns attribute on an element with a prefix (not pinned)");
                }
                if av != ns {
                    return Err(error(
                        format!(
                            "The following exception was encountered when processing XML data: 'The prefix '' cannot be redefined from '{ns}' to '{av}' within the same start element tag.' at line 0 and position 0."
                        ),
                        "Xml:Xml",
                    ));
                }
                if ns.is_empty() {
                    return unsupported("an empty xmlns attribute on an element without a namespace (not pinned)");
                }
            } else {
                plain.push((ap, al, ans, av));
            }
        }
        // The element's own name may need a declaration the tree does not carry.
        let mut extra = String::new();
        let visible = mapped(&self.scope, prefix);
        if visible.as_deref().unwrap_or("") != ns {
            let mut s = String::new();
            escape_attr(ns, &mut s);
            extra = if prefix.is_empty() {
                format!(" xmlns=\"{s}\"")
            } else {
                format!(" xmlns:{prefix}=\"{s}\"")
            };
            self.scope.push((prefix.to_string(), ns.to_string()));
        }
        self.break_line();
        self.level += 1;
        // An element inside mixed content is written inline with everything it holds, as .NET's indenting writer does (the
        // mixed-content bit is inherited, then restored at the end tag): P90CDataEdit.al:AfterSelf_Attached_ExistingNodes `k12`.
        self.saved.push(self.mixed);
        self.out.push('<');
        self.out.push_str(&qname(prefix, local));
        self.out.push_str(&extra);
        for w in &written {
            self.out.push_str(w);
        }
        let mut generated: Option<String> = None;
        for (ap, al, ans, av) in &plain {
            let use_prefix = if ans.is_empty() {
                String::new()
            } else if !ap.is_empty() && mapped(&self.scope, ap).as_deref() == Some(ans.as_str()) {
                ap.clone()
            } else if ap == "xml" && ans == XML_URI {
                // The `xml` prefix is bound without a declaration and written as it is (xml/XmlDocReadWrite.al:259-265
                // ReadFrom_Whitespace_TextNode `xmlspace`: `<a xml:space="preserve">` is written back unchanged).
                ap.clone()
            } else if let Some((p, _)) = self.scope.iter().rev().find(|(p, u)| !p.is_empty() && u == ans) {
                p.clone()
            } else if ap.is_empty() && self.scope.is_empty() && generated.is_none() && children.is_empty() {
                // The writer makes up `p1` and declares it after the attributes (xml/XmlAttributes.al:358 Collection_Set
                // `<e a="2" p1:a="3" xmlns:p1="urn:x" />`). A second one, or one under a namespace in scope, is not pinned.
                generated = Some(ans.clone());
                "p1".to_string()
            } else {
                return unsupported("serialising a namespaced attribute without a declared prefix");
            };
            let mut s = String::new();
            escape_attr(av, &mut s);
            self.out.push_str(&format!(" {}=\"{s}\"", qname(&use_prefix, al)));
        }
        if let Some(uri) = generated {
            let mut s = String::new();
            escape_attr(&uri, &mut s);
            self.out.push_str(&format!(" xmlns:p1=\"{s}\""));
        }
        if children.is_empty() && !open {
            self.out.push_str(" />");
            self.level -= 1;
        } else {
            self.out.push('>');
            let content_start = self.out.len();
            for c in children {
                self.node(c)?;
            }
            self.level -= 1;
            if self.out.len() != content_start {
                // The end tag goes on its own line, unless the element has text.
                self.break_line();
            }
            self.out.push_str("</");
            self.out.push_str(&qname(prefix, local));
            self.out.push('>');
        }
        self.mixed = self.saved.pop().unwrap_or(false);
        self.scope.truncate(mark);
        Ok(())
    }
}

/// An XML declaration as BC writes it: the encoding and standalone attributes are written even when empty for one made by
/// `Create`.
pub(super) fn declaration_text(version: &str, encoding: &str, standalone: &str, explicit: bool) -> String {
    let mut t = format!("<?xml version=\"{version}\"");
    if explicit || !encoding.is_empty() {
        t.push_str(&format!(" encoding=\"{encoding}\""));
    }
    if explicit || !standalone.is_empty() {
        t.push_str(&format!(" standalone=\"{standalone}\""));
    }
    t.push_str("?>");
    t
}

/// The text of `h` as BC writes it. A document gets its XML declaration first (its own, else
/// `version="1.0" encoding="utf-16"`). The namespace declarations of `h`'s ancestors do not
/// matter when a node is written on its own: it declares what it needs.
pub(super) fn serialize(h: &Handle) -> Result<String, Signal> {
    serialize_with(h, true)
}

/// An attribute written on its own: `name="value"` with the value escaped as in an element
/// (xml/lt-xml-attr-doc-http-members/P97AttrWrite.al:86 WriteTo_Text_Kinds `t1`..`t4`, `t9`, `t10`, `newline_tab`;
/// :261 WriteTo_Detached_And_Removed, attached and removed alike; xml/XmlAttributes.al:56 Attribute_Create). A namespace
/// declaration is `xmlns:prefix="uri"` (`t7`); a detached attribute with a namespace and no prefix gets `p0` (`t5`, `t6`,
/// `t8`). Other namespaced shapes (a prefix of its own, attached to an element, `xmlns="uri"`) are not pinned. `None` for a
/// node that is no attribute.
pub(super) fn attribute_markup(h: &Handle) -> Option<Result<String, Signal>> {
    let (prefix, local, ns, value) = with(h, |d| match d {
        XData::Attribute {
            prefix,
            local,
            ns,
            value,
        } => Some((prefix.clone(), local.clone(), ns.clone(), value.clone())),
        _ => None,
    })?;
    let mut v = String::new();
    escape_attr(&value, &mut v);
    let name = if ns.is_empty() && prefix.is_empty() {
        local
    } else if ns == XMLNS_URI && prefix == "xmlns" {
        format!("xmlns:{local}")
    } else if !ns.is_empty() && prefix.is_empty() && parent_of(h).is_none() && !parent_dropped(h) {
        format!("p0:{local}")
    } else {
        return Some(unsupported(
            "serialising a namespaced attribute on its own (a prefix of its own, attached, or a default namespace declaration; not pinned)",
        ));
    };
    Some(Ok(format!("{name}=\"{v}\"")))
}

/// [`serialize`] with or without the indentation (`XmlWriteOptions.PreserveWhitespace` writes none).
pub(super) fn serialize_with(h: &Handle, indent: bool) -> Result<String, Signal> {
    if let Some(t) = attribute_markup(h) {
        return t;
    }
    let mut w = Writer::new(indent);
    let is_doc = with(h, |d| matches!(d, XData::Document { .. }));
    if is_doc {
        let decl = with(h, |d| match d {
            XData::Document { decl, .. } => decl.clone(),
            _ => None,
        });
        // A read declaration that names an encoding is written as its own node; one without an encoding is replaced by the
        // writer's (xml/XmlEntitiesMisc.al:208 `with_nl`, misc/lt-probes-needed-1/PnAXmlWriteTo.al:90: a line end after
        // `<?xml version="1.0"?>` is written once).
        let has_decl = decl.as_ref().is_some_and(|d| {
            with(
                d,
                |x| matches!(x, XData::Declaration { encoding, explicit: false, .. } if !encoding.is_empty()),
            )
        });
        let text = match decl {
            Some(d) => with(&d, |x| match x {
                XData::Declaration {
                    version,
                    encoding,
                    standalone,
                    explicit,
                } => {
                    // A declaration read without an encoding is written with `utf-16` in a document (misc/lt-probes-needed-1/
                    // PnAXmlWriteTo.al:Layout_Declarations `version-only`, `standalone-no`).
                    let encoding = if encoding.is_empty() && !*explicit {
                        "utf-16"
                    } else {
                        encoding.as_str()
                    };
                    declaration_text(version, encoding, standalone, *explicit)
                }
                _ => unreachable!("a declaration slot holds a declaration"),
            }),
            None => "<?xml version=\"1.0\" encoding=\"utf-16\"?>".to_string(),
        };
        w.markup(&text);
        let kids = children_of(h);
        let has = |f: fn(&XData) -> bool| kids.iter().any(|c| with(c, |d| f(d)));
        if !has(|d| matches!(d, XData::Element { .. })) {
            // A document without a root element cannot be written (DocTypeInDoc.al:DocumentCreate_WithDocType, af82;
            // DocTypeRead.al:Read_RootMismatch, af147: the document a failed read left).
            return Err(error(
                "The requested operation cannot be performed in this context.",
                "InvalidOperation",
            ));
        }
        for (i, c) in kids.into_iter().enumerate() {
            match with(&c, |d| match d {
                XData::Text(s) => Some(s.clone()),
                _ => None,
            }) {
                Some(s) => {
                    // A line end read right after the document's own declaration (one that names an encoding) is written on a
                    // line of its own: the declaration ends its line first (xml/XmlDocReadWrite.al:409-430 WriteTo_OutStream_RoundTrip `reread_out`).
                    // Other white space there is written as before.
                    if i == 0 && has_decl && s == "\n" {
                        w.push_text("\n");
                    }
                    let mut esc = String::new();
                    escape_dom_text(&s, &mut esc);
                    w.document_text(&esc);
                }
                None => w.node(&c)?,
            }
        }
    } else {
        w.node(h)?;
    }
    Ok(w.out)
}

/// The content of `h` without indentation (`InnerXml`).
pub(super) fn serialize_inner(h: &Handle) -> Result<String, Signal> {
    let mut w = Writer::new(false);
    // InnerXml keeps the line ends the content has (corpus/tests/al-language/xml/TestProbeXmlContracts.al: WriteTo_EscapesAndAttributes, `innerxml` has a bare line feed).
    w.crlf = false;
    for c in children_of(h) {
        w.node(&c)?;
    }
    Ok(w.out)
}

pub(super) fn inner_text(h: &Handle, out: &mut String) {
    for c in children_of(h) {
        let (leaf, is_el) = with(&c, |d| match d {
            XData::Text(t) | XData::CData(t) => (Some(t.clone()), false),
            XData::Element { .. } => (None, true),
            _ => (None, false),
        });
        if let Some(t) = leaf {
            out.push_str(&t);
        } else if is_el {
            inner_text(&c, out);
        }
    }
}

// ------------------------------------------------------------- content
