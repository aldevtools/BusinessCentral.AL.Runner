//! Members of an XmlElement nothing assigned (or cleared: a cleared one is an unassigned one, xml/xml/UlClearXmlRef.al:90-102)
//! that tests/al-language/xml/UhUnassignedXmlElement.al:94-221 measures (codeunit 66992; al-runner issue 7760). Every other
//! member, form and argument goes on to its native, which refuses an unassigned element (`dom.rs` `node_of`).
//!
//! `AsXmlNode` is not in [`measured`]: the probe's step (:45-50, test :134) is `N := E.AsXmlNode(); N.WriteTo(T)`, so it
//! does not show which call raised, and an outside report measured that `AsXmlNode` returns (al-runner issue 8157). See
//! [`as_xml_node`].

use super::*;

/// A member of XmlElement the probe called on an unassigned variable.
#[derive(Clone, Copy)]
pub(super) enum Member {
    InnerXml,
    SetAttribute,
    GetChildElements,
    Attributes,
    GetChildNodes,
    IsEmpty,
    Remove,
}

/// The type the error names for `member` called as the probe called it (`used`: whether the result is used, `a`: the
/// arguments), or `None` for another form. Each line cites the probe's step and its test.
fn measured(member: Member, a: &[Value], used: bool) -> Option<&'static str> {
    use Member::*;
    let none = a.is_empty();
    match member {
        // :28-29 `innerxml`, test :94; :35-39 `getchildelements` (Count of the list), test :114; :65-69 `getchildnodes`,
        // test :184; :72-73 `isempty`, test :204. The result is used.
        InnerXml | GetChildElements | GetChildNodes | IsEmpty if used && none => Some("XmlElement"),
        // :30-34 `setattribute`, `E.SetAttribute('a', 'b')` as a statement, test :104.
        SetAttribute
            if !used && matches!(a, [Value::Text(n), Value::Text(v)] if n.as_str() == "a" && v.as_str() == "b") =>
        {
            Some("XmlAttributeCollection")
        }
        // :40-44 `attributes` and :70-71 `hasattributes` (`E.Attributes().Count()`), tests :124, :194. The result is used.
        Attributes if used && none => Some("XmlAttributeCollection"),
        // :74-78 `remove`, a statement, test :214.
        Remove if !used && none => Some("XmlNode"),
        _ => None,
    }
}

/// The error of `member` on the receiver `r` when `r` is an unassigned XmlElement and the call is a measured form; `Ok` to go on.
pub(super) fn check(r: &Option<&mut Value>, member: Member, a: &[Value], used: bool) -> Result<(), Signal> {
    if !unassigned(r, "XmlElement") {
        return Ok(());
    }
    match measured(member, a, used) {
        Some(nav) => Err(uninitialized(nav)),
        None => Ok(()),
    }
}

/// `N := E.AsXmlNode()` on an unassigned XmlElement `r` (the result used, no arguments): the XmlNode it returns, which holds
/// [`LibraryValue::XmlNodeOfUnassignedElement`]. `None` for any other receiver or form, which goes on as before.
///
/// Assumed (`xml-element-unassigned-as-xml-node`): the reporter measured that the call returns on real BC (al-runner issue 8157);
/// corpus PR 1421 measures it with the node's uses. A cleared element is an unassigned one (xml/xml/UlClearXmlRef.al:90-102).
pub(super) fn as_xml_node(r: &Option<&mut Value>, a: &[Value], used: bool) -> Option<Value> {
    if !(unassigned(r, "XmlElement") && used && a.is_empty()) {
        return None;
    }
    crate::assumptions::assume(crate::assumptions::Assumption::XmlElementUnassignedAsXmlNode);
    Some(tagged(
        "XmlNode",
        Rc::new(RefCell::new(LibraryValue::XmlNodeOfUnassignedElement)),
    ))
}

/// Whether `v` is an XmlNode that holds the node [`as_xml_node`] returned (any copy of it shares the handle). Generic readers
/// that would answer a value on it (the Variant Is* checks, the List searches) refuse through this.
pub fn is_node_of_unassigned_element(v: &Value) -> bool {
    matches!(v, Value::Library(name, h)
        if name.as_str() == "XmlNode" && matches!(&*h.borrow(), LibraryValue::XmlNodeOfUnassignedElement))
}

fn is_its_node(r: &Option<&mut Value>) -> bool {
    r.as_deref().is_some_and(is_node_of_unassigned_element)
}

/// `B := N.IsXmlElement()` on the node [`as_xml_node`] returned (the result used; the overload has no arguments): Yes,
/// through the assumption (corpus PR 1421, xml/xml/UhAsXmlNodeUnassignedElement.al:33-37, test :95-101, the same on every
/// major of its CI run). `None` for any other receiver or the statement form, which goes on as before (refused, `dom.rs`).
pub(super) fn is_xml_element(r: &Option<&mut Value>, used: bool) -> Option<Value> {
    if !(is_its_node(r) && used) {
        return None;
    }
    crate::assumptions::assume(crate::assumptions::Assumption::XmlElementUnassignedAsXmlNode);
    Some(Value::Boolean(true))
}

/// `N.WriteTo(T)` as a statement on the node [`as_xml_node`] returned: `NotInitialized` naming `XmlNode`. With the
/// assumption that `AsXmlNode` returns, xml/UhUnassignedXmlElement.al:134 (step :45-50) pins this error on the `WriteTo`.
/// `Ok` for every other receiver. The used form (`Ok := N.WriteTo(T)`) is refused: not measured. Both pass through the
/// assumption, also when an earlier test made the node (a codeunit global).
pub(super) fn write_to_text(r: &Option<&mut Value>, used: bool) -> Result<(), Signal> {
    if !is_its_node(r) {
        return Ok(());
    }
    crate::assumptions::assume(crate::assumptions::Assumption::XmlElementUnassignedAsXmlNode);
    if used {
        return unsupported(
            "the result of XmlNode.WriteTo(Text) on the XmlNode of an unassigned XmlElement (not measured)",
        );
    }
    Err(uninitialized("XmlNode"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assumptions::{Assumption, clear_hits, take_hits};

    fn node() -> Value {
        tagged(
            "XmlNode",
            Rc::new(RefCell::new(LibraryValue::XmlNodeOfUnassignedElement)),
        )
    }

    fn marked() -> bool {
        take_hits().contains(&Assumption::XmlElementUnassignedAsXmlNode)
    }

    /// A node made in an earlier test (a codeunit global) still marks the run that uses it: the WriteTo error and the refusal
    /// of the used form both pass through the assumption.
    #[test]
    fn write_to_text_marks_the_assumption_on_every_run() {
        let mut n = node();
        clear_hits();
        assert!(write_to_text(&Some(&mut n), false).is_err());
        assert!(marked());
        clear_hits();
        assert!(matches!(
            write_to_text(&Some(&mut n), true),
            Err(Signal::Unsupported(_))
        ));
        assert!(marked());
    }
}
