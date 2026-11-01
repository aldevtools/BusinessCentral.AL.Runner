//! The XmlDocument members whose XmlException AL sees as `A call to System.Xml.XmlDocument.<member> failed with this message:
//! <message>`, code `DotNetInvoke:Xml` (onprem-dotnet lt-ae-dotnet/AeXmlDocumentLoadErrors.al:112-128 LoadXml_NotXml, :149-166
//! Load_EmptyStream; lt-wd-dotnet/WdTryFunctionDotNetException.al:50-102).

/// The member's name as BC writes it in the error, for `Load` and `LoadXml`; `None` for any other member.
pub(super) fn xml_load_member(member: &str) -> Option<&'static str> {
    ["Load", "LoadXml"].into_iter().find(|m| m.eq_ignore_ascii_case(member))
}
