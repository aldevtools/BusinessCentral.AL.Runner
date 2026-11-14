//! Attribute and `XmlAttributeCollection` natives.

use super::*;

pub(super) fn name_table(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> NResult {
    recv(r)?;
    Ok(Value::Library(
        "XmlNameTable".into(),
        Rc::new(RefCell::new(LibraryValue::XmlNameTable)),
    ))
}

// ----- attributes

pub(super) fn set_attribute(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let h = recv(r)?;
    let (name, value) = (arg_text(a, 0)?, arg_text(a, 1)?);
    // `SetAttribute('xmlns:q', ..)` raises the XML name error at the colon and adds nothing
    // (xml/XmlAttributes.al:497-507 Attribute_Xmlns_Handling `af404`, `set_xmlns_q`). Other prefixed names are not pinned.
    if name.strip_prefix("xmlns:").is_some_and(is_ncname) {
        verify_ncname(&name)?;
    }
    check_name(&name)?;
    let existing = attrs_of(&h).into_iter().find(|x| {
        with(
            x,
            |d| matches!(d, XData::Attribute { local, ns, .. } if *local == name && ns.is_empty()),
        )
    });
    match existing {
        Some(x) => with(&x, |d| {
            if let XData::Attribute { value: v, .. } = d {
                *v = value;
            }
        }),
        None => {
            let at = new_node(XData::Attribute {
                prefix: String::new(),
                local: name,
                ns: String::new(),
                value,
            });
            insert(&h, None, vec![at])?;
        }
    }
    Ok(Value::Null)
}

pub(super) fn remove_attribute(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let h = recv(r)?;
    let target = match a.first() {
        // An attribute object names the attribute to remove from the receiver by its name: one that belongs to another element
        // removes nothing there and leaves its own element alone, and one that no element holds removes the receiver's attribute of
        // that name; removing it again is silent (recordref-fieldref/fieldref/XmlElementRemoveAttributeObject.al:
        // RemoveAttribute_ByObject).
        Some(Value::Library(..)) => {
            let attr = node_of(&a[0])?;
            let key = with(&attr, |d| match d {
                XData::Attribute { local, ns, .. } => Some((local.clone(), ns.clone())),
                _ => None,
            });
            let Some((name, ns)) = key else {
                return unsupported("RemoveAttribute with something that is no attribute");
            };
            let found = attrs_of(&h).into_iter().find(|x| {
                with(
                    x,
                    |d| matches!(d, XData::Attribute { local, ns: n, .. } if *local == name && *n == ns),
                )
            });
            match found {
                Some(x) => x,
                None => return Ok(Value::Null),
            }
        }
        _ => {
            let name = arg_text(a, 0)?;
            let ns = if a.len() > 1 { arg_text(a, 1)? } else { String::new() };
            match find_attribute_to_remove(&h, &name, &ns)? {
                Some(x) => x,
                None => return Ok(Value::Null),
            }
        }
    };
    detach(&target);
    Ok(Value::Null)
}

/// The attribute `RemoveAttribute(Name)` or `RemoveAttribute(LocalName, NamespaceUri)` takes off `h`: the name is a local
/// name without a prefix (`p:a` raises the XML name error, xml/AyXmlRemoveNsAttribute.al:71
/// RemoveAttribute_ByName_PrefixedPA, :174 `ns_prefixed_local`), matched with the namespace (none for the one-argument form:
/// `b` leaves `p:b`, :103 RemoveAttribute_ByName_LocalB; :146 `ns_a_p`, `ns_a_empty`). A name nothing matches removes nothing
/// (xml/XmlAttributes.al:259 RemoveAttribute `remove_missing`; AyXmlRemoveNsAttribute.al:174 `ns_unknown`). Other invalid
/// names are not pinned.
pub(super) fn find_attribute_to_remove(h: &Handle, name: &str, ns: &str) -> Result<Option<Handle>, Signal> {
    if let Err(e) = verify_ncname(name) {
        let chars: Vec<char> = name.chars().collect();
        let first_bad = chars.iter().position(|c| !is_name_char(*c));
        if chars.first().copied().is_some_and(is_name_start) && first_bad.is_some_and(|i| chars[i] == ':') {
            return Err(e);
        }
        return unsupported(format!("RemoveAttribute of the name `{name}` (not pinned)"));
    }
    Ok(attrs_of(h).into_iter().find(|x| {
        with(
            x,
            |d| matches!(d, XData::Attribute { local, ns: n, .. } if local == name && n == ns),
        )
    }))
}

/// `XmlAttributeCollection.Remove`: by attribute (one this element holds: xml/xml/
/// P104AttrCollection.al:31 P104_Remove_Attribute, its value stays readable), by name or by local name and namespace (:126
/// P104_Remove_ByName_Overloads; a missing one removes nothing). An attribute of another element, and a name with a prefix,
/// are not pinned.
pub(super) fn attrs_remove(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let owner = owner_of(r)?;
    if let Some(Value::Library(..)) = a.first() {
        let attr = node_of(&a[0])?;
        if !attrs_of(&owner).iter().any(|x| Rc::ptr_eq(x, &attr)) {
            return unsupported("XmlAttributeCollection.Remove of an attribute its element does not hold (not pinned)");
        }
        detach(&attr);
        return Ok(Value::Null);
    }
    let name = arg_text(a, 0)?;
    if !is_ncname(&name) {
        return unsupported(format!(
            "XmlAttributeCollection.Remove of the name `{name}` (not pinned)"
        ));
    }
    let ns = if a.len() > 1 { arg_text(a, 1)? } else { String::new() };
    if let Some(x) = find_attribute_to_remove(&owner, &name, &ns)? {
        detach(&x);
    }
    Ok(Value::Null)
}

pub(super) fn remove_all_attributes(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> NResult {
    let h = recv(r)?;
    for a in attrs_of(&h) {
        detach(&a);
    }
    Ok(Value::Null)
}

pub(super) fn attributes(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> NResult {
    let h = recv(r)?;
    Ok(Value::Library(
        "XmlAttributeCollection".into(),
        Rc::new(RefCell::new(LibraryValue::XmlAttrs(h))),
    ))
}

pub(super) fn owner_of(r: Option<&mut Value>) -> Result<Handle, Signal> {
    match r {
        Some(Value::Library(_, h)) => match &*h.borrow() {
            LibraryValue::XmlAttrs(o) => Ok(o.clone()),
            _ => unsupported("XmlAttributeCollection method on something else"),
        },
        _ => unsupported("XmlAttributeCollection method without a receiver"),
    }
}

pub(super) fn attrs_count(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> NResult {
    Ok(Value::Integer(attrs_of(&owner_of(r)?).len() as i32))
}

/// `Set(Name, Value)` adds the attribute or gives the one of that name the value, as `XmlElement.SetAttribute` does
/// (xml/XmlAttributes.al:342-361 Collection_Set `set`, `reset`; P104AttrCollection.al:74 `set_after`).
pub(super) fn attrs_set(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let owner = owner_of(r)?;
    let mut owner_value = tagged("XmlElement", owner);
    set_attribute(c, Some(&mut owner_value), a)
}

/// `Set(Name, NamespaceUri, Value)` adds an attribute in that namespace; the writer gives it a generated prefix
/// (xml/XmlAttributes.al:342-361 Collection_Set `set_ns`, see `Writer::element`). Pinned only on an element with no
/// namespace in scope and no other attribute in a namespace; giving an existing one a new value is not pinned.
pub(super) fn attrs_set_ns(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let owner = owner_of(r)?;
    if let Some(mut plain) = empty_namespace_args(&owner, a)? {
        let mut owner_value = tagged("XmlElement", owner);
        return set_attribute(c, Some(&mut owner_value), &mut plain);
    }
    add_namespaced_attribute(owner, a, false)
}

/// `Set(Name, '', Value)` and `SetAttribute(Name, '', Value)` are the two-argument forms: the attribute has no namespace and
/// replaces the one of that name in place (xml/WdXmlEmptyNsAndUnassigned.al WD_XmlAttrCollection_Set_EmptyNamespace,
/// WD_XmlElement_SetAttribute_EmptyNamespace; corpus branch al-runtime-rs-staging-1030c, al-runner issue 6303). Pinned for a
/// plain name on an element without a namespace; a qualified name or an element in a namespace stays refused.
pub(super) fn empty_namespace_args(owner: &Handle, a: &[Value]) -> Result<Option<Vec<Value>>, Signal> {
    let (name, ns) = (arg_text(a, 0)?, arg_text(a, 1)?);
    if !ns.is_empty() {
        return Ok(None);
    }
    let in_ns = with(owner, |d| matches!(d, XData::Element { ns, .. } if !ns.is_empty()));
    if name.contains(':') || in_ns {
        return unsupported(
            "XmlAttributeCollection.Set with an empty namespace URI and a qualified name or on an element in a namespace (not pinned)",
        );
    }
    Ok(Some(vec![a[0].clone(), a[2].clone()]))
}

/// `XmlElement.SetAttribute(Name, NamespaceUri, Value)`: as `XmlAttributeCollection.Set` with a namespace
/// (xml/xml/P104AttrCollection.al:18,50 `<e a="1" p1:c="3" xmlns:p1="urn:x" />`).
pub(super) fn set_attribute_ns(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let owner = recv(r)?;
    if let Some(mut plain) = empty_namespace_args(&owner, a)? {
        let mut owner_value = tagged("XmlElement", owner);
        return set_attribute(c, Some(&mut owner_value), &mut plain);
    }
    add_namespaced_attribute(owner, a, true)
}

/// `RemoveAll()` of the collection takes every attribute off its element and leaves the children
/// (P104AttrCollection.al:P104_RemoveAll, P104_RemoveAll_KeepsChildren).
pub(super) fn attrs_remove_all(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> NResult {
    for a in attrs_of(&owner_of(r)?) {
        detach(&a);
    }
    Ok(Value::Null)
}

/// `by_set_attribute`: called for `XmlElement.SetAttribute`, whose attribute keeps that origin (`set_by_set_attribute`).
pub(super) fn add_namespaced_attribute(owner: Handle, a: &mut [Value], by_set_attribute: bool) -> NResult {
    let (name, ns, value) = (arg_text(a, 0)?, arg_text(a, 1)?, arg_text(a, 2)?);
    check_name(&name)?;
    if ns.is_empty() {
        return unsupported("XmlAttributeCollection.Set with an empty namespace URI (not pinned)");
    }
    // On the element itself other attributes and declarations are fine: the writer resolves them as it does for `Create` and
    // `Add` (xml/xml/XmlNsPrefixes.al:Attribute_PrefixNumbering `setattribute_ns`). The same name again replaces the value
    // and is not pinned. An element with a namespace, or an ancestor with a namespace or declaration, is not pinned either.
    if attrs_of(&owner).iter().any(|x| {
        with(
            x,
            |d| matches!(d, XData::Attribute { local, ns: n, .. } if *local == name && *n == ns),
        )
    }) {
        return unsupported("XmlElement.SetAttribute with a namespace for an attribute that exists (not pinned)");
    }
    let mut at = Some(owner.clone());
    let mut first = true;
    while let Some(h) = at {
        let in_ns = with(&h, |d| matches!(d, XData::Element { ns, .. } if !ns.is_empty()));
        let declares = !first
            && attrs_of(&h)
                .iter()
                .any(|x| with(x, |d| matches!(d, XData::Attribute { ns, .. } if !ns.is_empty())));
        if in_ns || declares {
            return unsupported(
                "XmlAttributeCollection.Set with a namespace on an element with a namespace or a namespaced attribute in scope (the prefix it gets is not pinned)",
            );
        }
        first = false;
        at = parent_of(&h);
    }
    let attr = new_node(XData::Attribute {
        prefix: String::new(),
        local: name,
        ns,
        value,
    });
    if let LibraryValue::Xml(cell) = &mut *attr.borrow_mut() {
        cell.set_with_namespace = by_set_attribute;
    }
    insert(&owner, None, vec![attr])?;
    Ok(Value::Null)
}

pub(super) fn attrs_get_index(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let owner = owner_of(r)?;
    let Value::Integer(i) = a[0] else {
        return unsupported("attribute index that is not an Integer");
    };
    match usize::try_from(i - 1)
        .ok()
        .and_then(|i| attrs_of(&owner).get(i).cloned())
    {
        Some(at) => {
            a[1] = tagged("XmlAttribute", at);
            Ok(Value::Boolean(true))
        }
        None if c.ret_used() => Ok(Value::Boolean(false)),
        None => unsupported("XmlAttributeCollection.Get out of range, result unused (error text not pinned yet)"),
    }
}

pub(super) fn attrs_get_name(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let owner = owner_of(r)?;
    let name = arg_text(a, 0)?;
    // An attribute SetAttribute made with a namespace is not found by its plain name (`set_by_set_attribute`).
    let found = attrs_of(&owner).into_iter().find(|x| {
        with(
            x,
            |d| matches!(d, XData::Attribute { prefix, local, .. } if qname(prefix, local) == name),
        ) && !set_by_set_attribute(x)
    });
    match found {
        Some(at) => {
            a[1] = tagged("XmlAttribute", at);
            Ok(Value::Boolean(true))
        }
        None if c.ret_used() => Ok(Value::Boolean(false)),
        // The statement form when only an attribute in a namespace has that local name is not pinned.
        None if namespaced_local(&owner, &name) => unsupported(
            "XmlAttributeCollection.Get by the plain name of an attribute in a namespace, result unused (not pinned)",
        ),
        // The statement raises XmlNodeNotFound naming the attribute (corpus master, corpus PR 886:
        // xml/xml/AttrCollectionGetMissing.al:113 `missing, two arguments`). A qualified name is not pinned.
        None if !name.contains(':') => Err(attr_not_found(&name)),
        None => unsupported(
            "XmlAttributeCollection.Get of a missing qualified name, result unused (error text not pinned yet)",
        ),
    }
}

/// Whether `owner` has an attribute [`set_by_set_attribute`] whose local name is `name`.
fn namespaced_local(owner: &Handle, name: &str) -> bool {
    attrs_of(owner)
        .iter()
        .any(|x| set_by_set_attribute(x) && with(x, |d| matches!(d, XData::Attribute { local, .. } if local == name)))
}

/// An attribute that `XmlElement.SetAttribute(Name, NamespaceUri, Value)` made with a namespace and that still has no prefix:
/// Get by its plain name does not find it (xml/XmlAttributes.al:216 with :230 SetGet_Attribute_Namespace `get_no_ns`: No). The
/// gate is that origin, kept on the attribute's cell: an attribute made by `XmlAttribute.Create(Name, NamespaceUri, Value)` and
/// added, set through `XmlAttributeCollection.Set` with a namespace, or set in the xmlns namespace, is not measured and keeps
/// main's answer (found; a default declaration `xmlns` read from text is found, :493 Attribute_Xmlns_Handling `get_xmlns`).
fn set_by_set_attribute(h: &Handle) -> bool {
    match &*h.borrow() {
        LibraryValue::Xml(c) => {
            c.set_with_namespace
                && matches!(&c.data, XData::Attribute { prefix, ns, .. } if prefix.is_empty() && !ns.is_empty() && ns != XMLNS_URI)
        }
        _ => false,
    }
}

/// `The XML node with the name '<name>' could not be found.` (XmlNodeNotFound): `<name>` is the name for the two-argument
/// Get and `{<namespace>}<local name>` for the three-argument one (AttrCollectionGetMissing.al:109-113).
pub(super) fn attr_not_found(name: &str) -> Signal {
    error(
        format!("The XML node with the name '{name}' could not be found."),
        "XmlNodeNotFound",
    )
}

/// `Get(LocalName, NamespaceUri, var Attr)` finds the attribute by its local name and namespace: a prefixed attribute read
/// from text (xml/XmlAttributes.al:245 `get_p_a`), one set with a namespace (:218 `get_ns`, :577), not one in another
/// namespace (:221 `get_wrong_ns`), and an unprefixed attribute is not in the default namespace (:496
/// `get_a_default_ns`). An empty namespace URI or a qualified name is not pinned.
pub(super) fn attrs_get_name_ns(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let owner = owner_of(r)?;
    let (name, uri) = (arg_text(a, 0)?, arg_text(a, 1)?);
    // An empty namespace URI finds the attribute of that name that has no namespace (xml/WdXmlEmptyNsAndUnassigned.al
    // WD_XmlAttrCollection_Set_EmptyNamespace `Get with empty namespace` Yes; corpus branch al-runtime-rs-staging-1030c). When
    // there is none (also when the name is only in a namespace) it answers No and leaves the variable as it was, and the statement
    // raises XmlNodeNotFound for `{}<name>` (corpus master, corpus PR 886: xml/xml/
    // AttrCollectionGetMissing.al:68-83 and :109-111).
    if uri.is_empty() && !name.contains(':') {
        let found = attrs_of(&owner).into_iter().find(|x| {
            with(
                x,
                |d| matches!(d, XData::Attribute { local, ns, prefix, .. } if *local == name && ns.is_empty() && prefix.is_empty()),
            )
        });
        let Some(at) = found else {
            if c.ret_used() {
                return Ok(Value::Boolean(false));
            }
            return Err(attr_not_found(&format!("{{}}{name}")));
        };
        a[2] = tagged("XmlAttribute", at);
        return Ok(Value::Boolean(true));
    }
    if uri.is_empty() || name.contains(':') {
        return unsupported("XmlAttributeCollection.Get with an empty namespace URI or a qualified name (not pinned)");
    }
    let found = attrs_of(&owner).into_iter().find(|x| {
        with(
            x,
            |d| matches!(d, XData::Attribute { local, ns, .. } if *local == name && *ns == uri),
        )
    });
    match found {
        Some(at) => {
            a[2] = tagged("XmlAttribute", at);
            Ok(Value::Boolean(true))
        }
        None if c.ret_used() => Ok(Value::Boolean(false)),
        // AttrCollectionGetMissing.al:112 `missing, other ns`: `{urn:x}zz`.
        None => Err(attr_not_found(&format!("{{{uri}}}{name}"))),
    }
}

// ----- node lists
