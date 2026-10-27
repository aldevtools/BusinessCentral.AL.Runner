//! Which `DotNet` calls answer from their arguments alone (and the .NET runtime that runs them). The install baseline
//! kept on disk (al-testrun `baseline_cache`) is keyed on every input its code can read; a call outside this list may
//! read files, the environment, the clock or an assembly the key does not name, so a baseline whose code made one is
//! not kept (`Engine::dotnet_called`). The list is short on purpose: a member that is not here only costs the next
//! run its install code.

/// The member name of a constructor.
pub(crate) const CONSTRUCT: &str = ".ctor";

/// (type, members). What the install code of Microsoft's apps calls (Base Application's `GetUrl` helpers and XML
/// building): text, URI and XML in memory. The sidecar runs with the culture en-US and the time zone UTC whatever the
/// machine says (tools/dotnet-sidecar/Program.cs, al-testrun `dotnet.rs`), and the .NET runtime is part of the key
/// (`DotNetHost::identity`). `XmlDocument.Load` (a path or a URL) is not here, nor any member that names a file.
const PURE: &[(&str, &[&str])] = &[
    ("System.String", &["Length", "LastIndexOf"]),
    ("System.Uri", &["ToString"]),
    ("System.UriBuilder", &[CONSTRUCT, "Path", "Query", "Uri"]),
    // .NET's XmlDocument resolves no external entity unless a resolver is set, and no member here sets one.
    ("System.Xml.XmlDocument", &[CONSTRUCT, "CreateElement"]),
    ("System.Xml.XmlElement", &["InnerText", "InnerXml"]),
];

/// Whether `member` of `clr_type` answers from its arguments alone.
pub(crate) fn answers_from_arguments(clr_type: &str, member: &str) -> bool {
    PURE.iter()
        .any(|(t, members)| *t == clr_type && members.iter().any(|m| m.eq_ignore_ascii_case(member)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_listed_members_answer_from_their_arguments() {
        assert!(answers_from_arguments("System.UriBuilder", CONSTRUCT));
        assert!(answers_from_arguments("System.Xml.XmlElement", "innerxml"));
        assert!(!answers_from_arguments("System.Xml.XmlDocument", "Load"));
        assert!(!answers_from_arguments("System.DateTime", "Now"));
        assert!(!answers_from_arguments("System.Type", "GetType"));
        assert!(!answers_from_arguments("MyCompany.Settings", "Value"));
    }
}
