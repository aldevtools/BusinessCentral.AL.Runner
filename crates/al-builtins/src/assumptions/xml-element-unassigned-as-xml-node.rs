XmlElementUnassignedAsXmlNode => {
    id: "xml-element-unassigned-as-xml-node",
    assumed: "XmlElement.AsXmlNode() on an XmlElement variable that nothing assigned, or that was cleared, returns without an error when its result is used and it has no arguments (`N := E.AsXmlNode()`; al-builtins xml/unassigned_element.rs `as_xml_node`). The node it returns raises `Microsoft.Dynamics.Nav.Runtime.NavXmlNode variable not initialized.` (NotInitialized) at `N.WriteTo(T)` as a statement, also through a copy of the node (pinned with this assumption by xml/UhUnassignedXmlElement.al:134). Every other use of that node is refused (IsXmlElement, AsXmlElement, the other WriteTo forms, the used form of WriteTo(Text), passing it to another XML call). AsXmlNode as a statement keeps main's refusal.",
    tests: "No Microsoft test. An outside report: the reporter's own repro passes on real BC (al-runner issue 8157, https://github.com/aldevtools/BusinessCentral.AL.Runner/issues/8157, codeunit 51581 `N := E.AsXmlNode();`).",
    probe: "Written: corpus PR 1421, tests/al-language/xml/xml/UhAsXmlNodeUnassignedElement.al (codeunit 66997). Its CI run 43756437348 gives, the same on every major of the run: AsXmlNode returns (directly, in a TryFunction, on a cleared element), IsXmlElement of the node is Yes, AsXmlElement of it returns, WriteTo(Text) of it raises the NotInitialized text above. IsXmlElement and AsXmlElement stay refused here until a bump pins that file; what the element AsXmlElement returns does is not measured.",
    awaiting: Awaiting {
        issues: &[8157],
        files: &["tests/al-language/xml/xml/UhAsXmlNodeUnassignedElement.al"],
        since: "2026-11-15",
    },
}
