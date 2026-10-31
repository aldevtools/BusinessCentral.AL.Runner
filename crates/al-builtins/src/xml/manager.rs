//! `XmlNamespaceManager`: scopes and natives.

use super::*;

/// One namespace declaration of an `XmlNamespaceManager`: its prefix, its namespace (None once RemoveNamespace dropped it) and
/// the scope it was made in.
#[derive(Debug, Clone, PartialEq)]
pub struct NsDecl {
    prefix: String,
    uri: Option<String>,
    scope: u32,
}

/// The declarations of an `XmlNamespaceManager` and its scope depth (0 before any PushScope). The behavior is pinned by
/// xml/lt-xml-remaining-node-methods/P104NsManager.al (BC 27, 28, 29); each method cites its tests.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct NsScopes {
    decls: Vec<NsDecl>,
    scope: u32,
}

impl NsScopes {
    /// The newest declaration of `prefix`, if any, and whether an older live one stands behind a removed newest one.
    fn newest(&self, prefix: &str) -> Result<Option<&NsDecl>, Signal> {
        let mut it = self.decls.iter().rev().filter(|d| d.prefix == prefix);
        let newest = it.next();
        // A declaration removed in an inner scope while an outer one declares the same prefix: whether the outer one shows
        // again is not pinned (P104_RemoveNamespace_InScope removes only a prefix the inner scope declared alone).
        if newest.is_some_and(|d| d.uri.is_none()) && it.any(|d| d.uri.is_some()) {
            return unsupported(
                "XmlNamespaceManager: a prefix removed in a scope over an older declaration of it (not pinned)",
            );
        }
        Ok(newest)
    }

    /// The namespace `prefix` stands for: the newest declaration, else `xml` and the empty prefix, which are known without
    /// being added (xml/XmlXPathNamespaces.al:408 NamespaceManager_AddLookup `lookup_xml`, `lookup_empty`).
    fn lookup(&self, prefix: &str) -> Result<Option<String>, Signal> {
        Ok(match self.newest(prefix)? {
            Some(d) => d.uri.clone(),
            None => match prefix {
                "xml" => Some(XML_URI.to_string()),
                "" => Some(String::new()),
                _ => None,
            },
        })
    }

    /// The (prefix, namespace) pairs in force, oldest first (an XPath takes the last pair of a prefix).
    pub(super) fn pairs(&self) -> Result<Vec<(String, String)>, Signal> {
        let mut out = Vec::new();
        for d in &self.decls {
            if let Some(u) = &d.uri {
                out.push((d.prefix.clone(), u.clone()));
            } else {
                self.newest(&d.prefix)?;
            }
        }
        Ok(out)
    }
}

/// The receiver of an `XmlNamespaceManager` method, made on first use.
pub(super) fn manager_cell(r: Option<&mut Value>) -> Result<Handle, Signal> {
    match r {
        Some(Value::Library(_, h)) => {
            if matches!(&*h.borrow(), LibraryValue::Empty) {
                *h.borrow_mut() = LibraryValue::XmlNamespaces(NsScopes::default());
            }
            if matches!(&*h.borrow(), LibraryValue::XmlNamespaces(_)) {
                Ok(h.clone())
            } else {
                unsupported("an XmlNamespaceManager method on something else")
            }
        }
        _ => unsupported("an XmlNamespaceManager method without a receiver"),
    }
}

pub(super) fn with_manager<R>(
    r: Option<&mut Value>,
    f: impl FnOnce(&mut NsScopes) -> Result<R, Signal>,
) -> Result<R, Signal> {
    let h = manager_cell(r)?;
    let mut b = h.borrow_mut();
    match &mut *b {
        LibraryValue::XmlNamespaces(m) => f(m),
        _ => unsupported("an XmlNamespaceManager method on something else"),
    }
}

/// `NameTable(NewValue)` is accepted and has no effect (names are compared as text);
/// reading it back is not pinned.
/// (xml/TestXmlRuntimeContracts.al:XmlDocument_SelectNodes_DefaultNamespace_NeedsAPrefixedName)
pub(super) fn manager_name_table(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    manager_cell(r)?;
    if a.is_empty() {
        return unsupported("XmlNamespaceManager.NameTable read back (not pinned)");
    }
    Ok(Value::Null)
}

/// `AddNamespace(Prefix, Uri)`: a prefix the current scope already declares gets the new namespace in place, so a lookup of
/// the old namespace finds the next prefix that still stands for it (P104NsManager.al:P104_LookupPrefix `redefined_old`
/// `q`, `redefined_new` `p`; P104_RemoveNamespace_InScope `p_redefined_remove_old`); otherwise it is a new declaration of the
/// current scope.
pub(super) fn manager_add_namespace(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let (prefix, uri) = (arg_text(a, 0)?, arg_text(a, 1)?);
    with_manager(r, |m| {
        let scope = m.scope;
        match m
            .decls
            .iter_mut()
            .rev()
            .take_while(|d| d.scope == scope)
            .find(|d| d.prefix == prefix)
        {
            Some(d) => d.uri = Some(uri),
            None => m.decls.push(NsDecl {
                prefix,
                uri: Some(uri),
                scope,
            }),
        }
        Ok(Value::Null)
    })
}

/// `LookupNamespace(Prefix, var Uri)` (xml/TestProbeXmlContracts.al:XPath_NamespaceManager: `p` gives `Yes urn:p`, an unknown
/// prefix `No`). An inner scope sees the outer declarations and its own shadow them; PopScope brings the outer ones back
/// (P104NsManager.al:P104_PushScope_PopScope, P104_Scopes_Nested). A removed prefix is unknown (P104_RemoveNamespace `p_removed`).
pub(super) fn manager_lookup_namespace(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let prefix = arg_text(a, 0)?;
    let found = with_manager(r, |m| m.lookup(&prefix))?;
    match found {
        Some(u) => {
            a[1] = Value::Text(u.into());
            Ok(Value::Boolean(true))
        }
        None if c.ret_used() => Ok(Value::Boolean(false)),
        None => unsupported("XmlNamespaceManager.LookupNamespace of an unknown prefix, result unused (not pinned)"),
    }
}

/// `HasNamespace(Prefix)` answers only for the current scope: an inner scope does not have the outer declarations
/// (P104NsManager.al:P104_PushScope_PopScope `inner_sees_outer` No, P104_Scopes_Nested `level3_a`), and `xml` is not one
/// (P104_RemoveNamespace `xml_after`). The empty prefix declared with an empty namespace is not pinned.
pub(super) fn manager_has_namespace(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let prefix = arg_text(a, 0)?;
    with_manager(r, |m| {
        let scope = m.scope;
        let d = m
            .decls
            .iter()
            .rev()
            .take_while(|d| d.scope == scope)
            .find(|d| d.prefix == prefix);
        match d.and_then(|d| d.uri.as_deref()) {
            Some("") if prefix.is_empty() => unsupported(
                "XmlNamespaceManager.HasNamespace of the empty prefix declared with no namespace (not pinned)",
            ),
            Some(_) => Ok(Value::Boolean(true)),
            None => Ok(Value::Boolean(false)),
        }
    })
}

/// `PushScope()` opens a scope; `PopScope()` drops the declarations of the current scope. A PopScope with no scope open
/// does nothing and raises nothing (P104NsManager.al:P104_PopScope_Unbalanced). Its result is not pinned.
pub(super) fn manager_push_scope(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> NResult {
    with_manager(r, |m| {
        m.scope += 1;
        Ok(Value::Null)
    })
}

pub(super) fn manager_pop_scope(c: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> NResult {
    if c.ret_used() {
        return unsupported("the result of XmlNamespaceManager.PopScope (not pinned)");
    }
    with_manager(r, |m| {
        if m.scope > 0 {
            let scope = m.scope;
            m.decls.retain(|d| d.scope != scope);
            m.scope -= 1;
        }
        Ok(Value::Null)
    })
}

/// `RemoveNamespace(Prefix, Uri)` drops the declaration of the current scope that has both; anything else (an outer
/// declaration, another namespace, an unknown or empty prefix, `xml`) is left and raises nothing
/// (P104NsManager.al:P104_RemoveNamespace, P104_RemoveNamespace_InScope `inner_p_removed`, `p_redefined_remove_old`).
pub(super) fn manager_remove_namespace(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let (prefix, uri) = (arg_text(a, 0)?, arg_text(a, 1)?);
    with_manager(r, |m| {
        let scope = m.scope;
        for d in m.decls.iter_mut().rev().take_while(|d| d.scope == scope) {
            if d.prefix == prefix && d.uri.as_deref() == Some(uri.as_str()) {
                d.uri = None;
            }
        }
        Ok(Value::Null)
    })
}

/// `LookupPrefix(Uri, var Prefix)` gives the newest prefix that still stands for the namespace, compared with case
/// (P104NsManager.al:P104_LookupPrefix: `two_prefixes_same_uri` q, `redefined_old` q, `case_differs` No;
/// P104_PushScope_PopScope `inner_lookup_outer_uri` No while `p` is shadowed). The empty namespace is the empty prefix's until a
/// prefix is declared for it (`empty_uri`, `empty_uri_added` r); the xml and xmlns namespaces give `xml` and `xmlns`
/// (`xml_uri`, `xmlns_uri`). Not found answers false and leaves the variable (`missing`); unused, it is not pinned.
pub(super) fn manager_lookup_prefix(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let uri = arg_text(a, 0)?;
    let found = with_manager(r, |m| {
        for d in m.decls.iter().rev() {
            if d.uri.as_deref() == Some(uri.as_str()) && m.lookup(&d.prefix)?.as_deref() == Some(uri.as_str()) {
                return Ok(Some(d.prefix.clone()));
            }
        }
        Ok(match uri.as_str() {
            "" if m.lookup("")?.as_deref() == Some("") => Some(String::new()),
            XML_URI => Some("xml".to_string()),
            "http://www.w3.org/2000/xmlns/" => Some("xmlns".to_string()),
            _ => None,
        })
    })?;
    match found {
        Some(p) => {
            a[1] = Value::Text(p.into());
            Ok(Value::Boolean(true))
        }
        None if c.ret_used() => Ok(Value::Boolean(false)),
        None => unsupported("XmlNamespaceManager.LookupPrefix of an unknown namespace, result unused (not pinned)"),
    }
}
