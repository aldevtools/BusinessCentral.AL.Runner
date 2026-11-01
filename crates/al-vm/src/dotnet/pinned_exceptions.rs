//! The .NET exceptions whose error text AL sees is pinned: `A call to <type>[.<member>] failed with this message: <.NET
//! message>` with code `DotNetInvoke:<exception name without "Exception">` (ADR-014 section 7). Moved out of `dotnet_apply`.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_host::dotnet::DotNetException;

use super::registry::MemberKind;
use super::xml_errors::xml_load_member;

/// `Err` with the error AL sees when the exception `e` of `clr_type.member` is pinned; `Ok` when it is not (the caller refuses).
pub(super) fn signal(
    kind: MemberKind,
    is_static: bool,
    clr_type: &str,
    member: &str,
    values: &[Value],
    e: &DotNetException,
) -> Result<(), Signal> {
    // Pinned for the MailAddress constructor only, exactly these two exception types:
    // corpus tests/al-language-onprem-dotnet/lt-af-dotnet/AfMailAddress.al:77 (Format),
    // :88 and :99 (Argument, Format). Any other exception stays a refusal (not pinned).
    if kind == MemberKind::Construct && clr_type.eq_ignore_ascii_case("System.Net.Mail.MailAddress") {
        let code = match e.clr_type.as_str() {
            "System.FormatException" => Some("DotNetInvoke:Format"),
            "System.ArgumentException" => Some("DotNetInvoke:Argument"),
            _ => None,
        };
        if let Some(code) = code {
            return al_builtins::native::raise(
                format!("A call to {clr_type} failed with this message: {}", e.message),
                code,
            );
        }
    }
    // `new System.Uri(text)` of a text that is no URI (UriFormatException, as System Application Uri.IsValidUri meets
    // it), and XmlDocument.LoadXml of a text with no root element (XmlException): the error AL sees is `A call to
    // <type>[.<member>] failed with this message: <.NET message>`, code `DotNetInvoke:UriFormat` / `DotNetInvoke:Xml`,
    // caught by a TryFunction and by Codeunit.Run (onprem-dotnet lt-wd-dotnet/WdTryFunctionDotNetException.al:50-102,
    // corpus branch al-runtime-rs-staging-1030c, al-runner issue 6305). XmlDocument.Load and LoadXml raise any XmlException
    // in the same form (lt-ae-dotnet/AeXmlDocumentLoadErrors.al:112-128 LoadXml_NotXml, :149-166 Load_EmptyStream). Other
    // exceptions of these members stay refused.
    let pinned = match (kind, e.clr_type.as_str()) {
        (MemberKind::Construct, "System.UriFormatException") if clr_type.eq_ignore_ascii_case("System.Uri") => {
            Some((clr_type.to_string(), "DotNetInvoke:UriFormat"))
        }
        (MemberKind::Call, "System.Xml.XmlException") if clr_type.eq_ignore_ascii_case("System.Xml.XmlDocument") => {
            xml_load_member(member).map(|m| (format!("{clr_type}.{m}"), "DotNetInvoke:Xml"))
        }
        _ => None,
    };
    if let Some((called, code)) = pinned {
        return al_builtins::native::raise(
            format!("A call to {called} failed with this message: {}", e.message),
            code,
        );
    }
    // System.Enum.Parse(type, name) of a name that is not a member (also one in another case): writer TV, corpus
    // branch al-runtime-rs-staging-tv 8ae7ac0b, lt-bb-dotnet/BbGetDotNetType.al
    // TV_EnumParse_GetDotNetTypeOfUnassignedX509ContentType (assumption dotnet-type-unassigned-enum).
    if kind == MemberKind::Call
        && is_static
        && clr_type.eq_ignore_ascii_case("System.Enum")
        && member.eq_ignore_ascii_case("Parse")
        && values.len() == 2
        && e.clr_type == "System.ArgumentException"
        && e.message.starts_with("Requested value '")
    {
        // Pinned by tests/al-language-onprem-dotnet/lt-bb-dotnet/BbGetDotNetType.al TV_GetDotNetType_UnassignedEnumVariables, TV_EnumParse_GetDotNetTypeOfUnassignedX509ContentType (was assumption `dotnet-type-unassigned-enum`).
        return al_builtins::native::raise(
            format!("A call to System.Enum.Parse failed with this message: {}", e.message),
            "DotNetInvoke:Argument",
        );
    }
    // Encoding.GetEncoding: the sidecar registers the code-page provider, as BC's server answers code pages
    // (corpus tests/al-language-onprem-dotnet/lt-ra-dotnet/RaEncodingCodePages.al:115, :131, :152, :244).
    // Pinned failures: a code page out of range (ArgumentOutOfRange, :168 and :244, 99999) and an unknown
    // name (Argument, :168). A code page in range that .NET has no data for (NotSupported) is not pinned.
    if clr_type.eq_ignore_ascii_case("System.Text.Encoding") && member.eq_ignore_ascii_case("GetEncoding") {
        let code = match (e.clr_type.as_str(), values.first()) {
            ("System.ArgumentOutOfRangeException", Some(Value::Integer(_))) => Some("DotNetInvoke:ArgumentOutOfRange"),
            ("System.ArgumentException", Some(Value::Text(_) | Value::Code(_))) => Some("DotNetInvoke:Argument"),
            _ => None,
        };
        if let Some(code) = code {
            return al_builtins::native::raise(
                format!(
                    "A call to {clr_type}.GetEncoding failed with this message: {}",
                    e.message
                ),
                code,
            );
        }
    }
    zip_archive(kind, clr_type, member, e)
}

/// System.IO.Compression.ZipArchive in Create mode (al-language-onprem-dotnet/lt-y44-zip-create/Y44ZipCreateMode.al, corpus PR
/// 962): a CreateEntry while an entry stream is still open (IOException, Zip_OpenKeptInVariable_SecondCreate :276-277), Entries
/// in Create mode, also after Dispose (NotSupportedException, Zip_EntriesInCreateMode :323-337), and a CreateEntry after
/// Dispose (ObjectDisposedException, Zip_CreateEntry_AfterDispose :389-390). Other exceptions of these members stay refused.
fn zip_archive(kind: MemberKind, clr_type: &str, member: &str, e: &DotNetException) -> Result<(), Signal> {
    if !clr_type.eq_ignore_ascii_case("System.IO.Compression.ZipArchive") {
        return Ok(());
    }
    let is = |m: &str| member.eq_ignore_ascii_case(m);
    let (name, code) = match (kind, e.clr_type.as_str()) {
        (MemberKind::Call, "System.IO.IOException") if is("CreateEntry") => ("CreateEntry", "DotNetInvoke:IO"),
        (MemberKind::Call, "System.ObjectDisposedException") if is("CreateEntry") => {
            ("CreateEntry", "DotNetInvoke:ObjectDisposed")
        }
        // `ZipArchive.Entries.Count` reads the property, `foreach ... in ZipArchive.Entries()` calls it.
        (MemberKind::Get | MemberKind::Call, "System.NotSupportedException") if is("Entries") => {
            ("Entries", "DotNetInvoke:NotSupported")
        }
        _ => return Ok(()),
    };
    al_builtins::native::raise(
        format!(
            "A call to System.IO.Compression.ZipArchive.{name} failed with this message: {}",
            e.message
        ),
        code,
    )
}
