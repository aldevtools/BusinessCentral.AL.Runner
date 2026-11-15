//! Members of an XmlElement nothing assigned (or cleared: a cleared one is an unassigned one, xml/xml/UlClearXmlRef.al:90-102)
//! that corpus PR 1408 measured (tests/al-language/xml/UhUnassignedXmlElement.al, BC 27.5, 28.5 and 29.0; al-runner issue 7760).
//! The probe is merged and not yet pinned: every raise here marks `xmlelement-unassigned-members`. Every other member, form
//! and argument goes on to its native, which refuses an unassigned element (`dom.rs` `node_of`).

use super::*;
use crate::assumptions::{Assumption, assume};

/// A member of XmlElement the probe called on an unassigned variable.
#[derive(Clone, Copy)]
pub(super) enum Member {
    InnerXml,
    SetAttribute,
    GetChildElements,
    Attributes,
    AsXmlNode,
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
        // :45-50 `asxmlnode` (N := E.AsXmlNode()), test :134. The result is used.
        AsXmlNode if used && none => Some("XmlNode"),
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
        Some(nav) => {
            assume(Assumption::XmlElementUnassignedMembers);
            Err(uninitialized(nav))
        }
        None => Ok(()),
    }
}
