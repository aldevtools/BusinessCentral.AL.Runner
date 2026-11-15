XmlElementUnassignedAsXmlNode => {
    id: "xml-element-unassigned-as-xml-node",
    assumed: "XmlElement.AsXmlNode() on an XmlElement variable that nothing assigned, or that was cleared, returns without an error when its result is used and it has no arguments (`N := E.AsXmlNode()`; al-builtins xml/unassigned_element.rs `as_xml_node`). On the node it returns, also through a copy: `B := N.IsXmlElement()` (result used) answers Yes (`is_xml_element`), and `N.WriteTo(T)` as a statement raises `Microsoft.Dynamics.Nav.Runtime.NavXmlNode variable not initialized.` (NotInitialized; pinned with this assumption by xml/UhUnassignedXmlElement.al:134). Every other use of that node is refused (AsXmlElement, the statement form of IsXmlElement, the other Is* and As* members, the other WriteTo forms, the used form of WriteTo(Text), passing it to another XML call, the Variant Is* checks on it, and List Contains, IndexOf, LastIndexOf and Remove when it is searched for or in the list). AsXmlNode as a statement keeps main's refusal.",
    tests: "No Microsoft test. An outside report: the reporter's own repro passes on real BC (al-runner issue 8157, codeunit 51581 `N := E.AsXmlNode();`).",
    probe: "Written and merged: corpus PR 1421, tests/al-language/xml/xml/UhAsXmlNodeUnassignedElement.al (codeunit 66997), not pinned yet. Its CI run 43756437348 gives, the same on every major of the run: AsXmlNode returns (directly :78, in a TryFunction :28-32, on a cleared element :50-56), IsXmlElement of the node is Yes (:33-37, test :95-101), AsXmlElement of it returns (:38-43), WriteTo(Text) of it raises the NotInitialized text above (:44-49). AsXmlElement stays refused until the element it returns is measured.",
    awaiting: Awaiting {
        issues: &[8157],
        files: &["tests/al-language/xml/xml/UhAsXmlNodeUnassignedElement.al"],
        since: "2026-11-15",
    },
}
