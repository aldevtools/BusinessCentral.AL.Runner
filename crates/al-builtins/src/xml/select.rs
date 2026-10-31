//! `SelectNodes` and `SelectSingleNode` natives.

use super::*;

/// Runs an XPath for a SelectNodes or SelectSingleNode call: `None` is a path that is no XPath (or names a prefix nobody
/// declared) when the result is used, which answers false; the statement form raises the pinned "not a node set" error and
/// refuses the other cases (xml/TestProbeXmlContracts.al:XPath_MalformedStatement, XPath_NamespaceManager).
pub(super) fn select(
    c: &mut dyn Ctx,
    h: &Handle,
    path: &str,
    prefixes: Prefixes,
) -> Result<Option<Vec<Handle>>, Signal> {
    // An XmlDeclaration selects nothing, whatever the path, even one that is no XPath (xml/lt-xml-remaining-node-methods/
    // P104SelDecl.al: every case answers `No:0`).
    if with(h, |d| matches!(d, XData::Declaration { .. })) {
        return Ok(None);
    }
    // So does a document type, when the result is used (P104SelDocType.al: every case answers `No:` or `No:0`); the
    // statement form is not pinned.
    if with(h, |d| matches!(d, XData::DocType { .. })) {
        if c.ret_used() {
            return Ok(None);
        }
        return unsupported(
            "SelectNodes or SelectSingleNode of an XmlDocumentType with the result unused (not pinned)",
        );
    }
    match xpath(h, path, prefixes) {
        Ok(v) => Ok(Some(v)),
        Err(XFail::Signal(s)) => Err(s),
        Err(XFail::NotNodeSet) if !c.ret_used() => Err(error("Expression must evaluate to a node-set.", "Xml:XPath")),
        // The statement form raises the text the result-used form leaves as the last error: the text belongs to the path
        // (xml/XmlXPathNamespaces.al:640 SelectNodes_Backslash_Path: `nodes_unused` and `nodes_if` alike).
        Err(XFail::Malformed(why)) if !c.ret_used() => match xpath_error_text(&why, path) {
            Some(text) => Err(error(text, "Xml:XPath")),
            None => unsupported(format!("XPath `{path}`: {why} (statement form, error text not pinned)")),
        },
        // A call whose result is used answers false and leaves the error as the last error (misc/lt-probes-needed-1/
        // PnAXmlXPath.al:NamespacePaths, MalformedPaths: `Yes|-1` with the text and the code `Xml:XPath` readable afterwards).
        Err(XFail::NotNodeSet) => {
            set_xpath_error(c, "Expression must evaluate to a node-set.");
            Ok(None)
        }
        Err(XFail::Malformed(why)) => {
            if let Some(text) = xpath_error_text(&why, path) {
                set_xpath_error(c, &text);
            }
            Ok(None)
        }
    }
}

pub(super) fn set_xpath_error(c: &mut dyn Ctx, text: &str) {
    c.set_last_error(crate::native::AlError {
        message: text.to_string(),
        code: "Xml:XPath".into(),
        call_stack: String::new(),
        ..Default::default()
    });
}

/// The text .NET gives for the reasons the corpus pins (PnAXmlXPath.al:NamespacePaths, MalformedPaths); the other reasons leave no text.
/// - Tokens after a complete expression, or a character no token starts with: `'<path>' has an invalid token.`
///   (xml/XmlXPathNamespaces.al:150 SelectNodes_Invalid_XPath `two_names` `a b`, :640 `\\contract`;
///   xml/lt-xml-cdata-comment-pi-text/P90CDataRead.al:1266 `a b`).
/// - A variable: the namespace manager text (P90CDataRead.al:1239 `$v`).
/// - A missing expression, step or `]`: `Expression must evaluate to a node-set.` (XmlXPathNamespaces.al:150 `/a/[`, ``,
///   `///`, `foo(`; P90CDataRead.al:1225 ``, ` `, `[`, `//`, `self::`, `/r/a[`).
pub(super) fn xpath_error_text(why: &str, path: &str) -> Option<String> {
    const NS_MANAGER: &str =
        "Namespace Manager or XsltContext needed. This query has a prefix, variable, or user-defined function.";
    const KNOWN: [&str; 17] = [
        "boolean",
        "contains",
        "count",
        "false",
        "last",
        "local-name",
        "name",
        "namespace-uri",
        "normalize-space",
        "not",
        "number",
        "position",
        "starts-with",
        "string",
        "string-length",
        "true",
        "concat",
    ];
    if why == "a prefixed name needs a namespace manager" {
        return Some(NS_MANAGER.into());
    }
    if let Some(p) = why
        .strip_prefix("prefix ")
        .and_then(|r| r.strip_suffix(" is not declared"))
    {
        return Some(format!("Namespace prefix '{p}' is not defined."));
    }
    if why == "unterminated literal" {
        return Some("This is an unclosed string.".into());
    }
    if why == "trailing characters" || why == "an invalid token" {
        return Some(format!("'{path}' has an invalid token."));
    }
    if why == "variables are not supported" {
        return Some(NS_MANAGER.into());
    }
    if let Some(name) = why.strip_prefix("function ").and_then(|r| r.split(" with ").next()) {
        return (!KNOWN.contains(&name)).then(|| NS_MANAGER.to_string());
    }
    matches!(why, "empty expression" | "unclosed [" | "a step without a name")
        .then(|| "Expression must evaluate to a node-set.".to_string())
}

pub(super) fn select_single_node(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let h = recv(r)?;
    let found = select(c, &h, &arg_text(a, 0)?, None)?;
    single_result(c, found, a, 1)
}

pub(super) fn single_result(c: &mut dyn Ctx, found: Option<Vec<Handle>>, a: &mut [Value], slot: usize) -> NResult {
    let path = arg_text(a, 0)?;
    match found.and_then(|f| f.into_iter().next()) {
        Some(n) => {
            a[slot] = tagged("XmlNode", n);
            Ok(Value::Boolean(true))
        }
        None if c.ret_used() => Ok(Value::Boolean(false)),
        // The statement form raises (xml/lt-xml-cdata-comment-pi-text/P90*Read.al:SelectSingle_Absolute_2 `x2`).
        None => Err(error(
            format!("The XPath query '{path}' does not match any nodes."),
            "InvalidXPathSelection",
        )),
    }
}

pub(super) fn select_nodes(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let h = recv(r)?;
    // A document type has no nodes to select: false and an empty list (DocTypeNode.al:SelectNodes_OnDocType).
    if with(&h, |d| matches!(d, XData::DocType { .. })) {
        a[1] = list_of(Vec::new());
        return Ok(Value::Boolean(false));
    }
    let found = select(c, &h, &arg_text(a, 0)?, None)?;
    a[1] = list_of(found.clone().unwrap_or_default());
    Ok(Value::Boolean(found.is_some()))
}

/// The prefixes of the `XmlNamespaceManager` in argument `i`.
pub(super) fn manager_prefixes(a: &[Value], i: usize) -> Result<Vec<(String, String)>, Signal> {
    match a.get(i) {
        Some(Value::Library(_, h)) => match &*h.borrow() {
            LibraryValue::XmlNamespaces(v) => v.pairs(),
            LibraryValue::Empty => Ok(Vec::new()),
            _ => unsupported("an XmlNamespaceManager that is not one"),
        },
        other => unsupported(format!("{} as an XmlNamespaceManager", other.describe())),
    }
}

pub(super) fn select_nodes_ns(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let h = recv(r)?;
    let prefixes = manager_prefixes(a, 1)?;
    let found = select(c, &h, &arg_text(a, 0)?, Some(&prefixes))?;
    a[2] = list_of(found.clone().unwrap_or_default());
    Ok(Value::Boolean(found.is_some()))
}

pub(super) fn select_single_node_ns(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let h = recv(r)?;
    let prefixes = manager_prefixes(a, 1)?;
    let found = select(c, &h, &arg_text(a, 0)?, Some(&prefixes))?;
    single_result(c, found, a, 2)
}
