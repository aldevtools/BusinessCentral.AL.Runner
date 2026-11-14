//! `ReplaceWith` of an attribute of an element by new attributes, in the cells the corpus measured.

use super::*;

/// `ReplaceWith` on an attribute of an element, as a statement, by one or two new attributes: they take its place, in
/// argument order (xml/XmlAttributes.al:463 Attribute_ReplaceWith `out` `<e c="3" b="2" />`; xml/xml/P97AttrEdit.al:249
/// ReplaceWith_Attached_Typed `t6`, :294 `t11` `<r p="1" ta="1" tb="2" q="3">`; xml/xml/P97AttrMisc.al:211
/// GetParent_AfterReplaceAndMove `replaced`, `old_found` No, `new_name` r). `None` keeps the general path for every
/// other shape.
pub(super) fn replace_attribute(c: &mut dyn Ctx, h: &Handle, args: &[Value]) -> Option<Value> {
    let (p, i) = measured_replace(c, h, args)?;
    with(&p, |d| {
        if let XData::Element { attrs, .. } = d {
            attrs.remove(i);
        }
    });
    set_parent(h, None);
    for (k, a) in args.iter().enumerate() {
        let Value::Library(_, n) = a else {
            unreachable!("checked by measured_replace")
        };
        set_parent(n, Some(&p));
        with(&p, |d| {
            if let XData::Element { attrs, .. } = d {
                attrs.insert(i + k, n.clone());
            }
        });
    }
    Some(Value::Boolean(true))
}

/// The element and the position of `h` when the call is one the pins measured, each fact by equality:
/// - the statement form (every pin);
/// - `h` is an attribute of an element without a namespace, and every attribute of that element, `h` included, has no
///   prefix and no namespace (`<e a="1" b="2"/>` read, :413; `r` built with `p`, `x`, `q`, P97AttrEdit.al:26-33);
/// - one or two arguments, each a typed `XmlAttribute` without a parent, prefix or namespace, whose name no attribute of
///   the element and no other argument has (`c`; `ta`, `tb`; `y`). The pins make each one with `XmlAttribute.Create`
///   (:463; P97AttrEdit.al:189, :198; P97AttrMisc.al:210); an attribute removed from an element has the same state here
///   and passes too, unmeasured.
fn measured_replace(c: &mut dyn Ctx, h: &Handle, args: &[Value]) -> Option<(Handle, usize)> {
    if c.ret_used() || kind(h) != "XmlAttribute" || !matches!(args.len(), 1 | 2) {
        return None;
    }
    let p = parent_of(h)?;
    let element_plain = with(
        &p,
        |d| matches!(d, XData::Element { prefix, ns, .. } if prefix.is_empty() && ns.is_empty()),
    );
    let attrs = attrs_of(&p);
    if !element_plain || !attrs.iter().all(plain_attribute) {
        return None;
    }
    let i = attrs.iter().position(|a| Rc::ptr_eq(a, h))?;
    let mut names: Vec<String> = attrs.iter().map(local_name).collect();
    for a in args {
        let Value::Library(tag, n) = a else { return None };
        let fresh = tag == "XmlAttribute"
            && is_xml(n)
            && kind(n) == "XmlAttribute"
            && parent_of(n).is_none()
            && !parent_dropped(n)
            && plain_attribute(n);
        if !fresh {
            return None;
        }
        let name = local_name(n);
        if names.contains(&name) {
            return None;
        }
        names.push(name);
    }
    Some((p, i))
}

fn plain_attribute(h: &Handle) -> bool {
    with(
        h,
        |d| matches!(d, XData::Attribute { prefix, ns, .. } if prefix.is_empty() && ns.is_empty()),
    )
}

fn local_name(h: &Handle) -> String {
    with(h, |d| match d {
        XData::Attribute { local, .. } => local.clone(),
        _ => String::new(),
    })
}
