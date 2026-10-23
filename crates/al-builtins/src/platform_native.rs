//! `[Native]` procedures of the platform's own codeunits (System.app): their AL body is empty and the
//! service tier implements them. A procedure listed here runs this implementation; one that is not
//! stays refused (`lower.rs`, "[Native] procedure ... has no AL body").
//!
//! `Base64Convert` (System.app codeunit 2000000024, which System Application's "Base64 Convert Impl."
//! calls) is pinned by the corpus staging branch (al-runtime-rs-staging 8217373), types/PVBase64.al:
//! - the text is encoded with UTF-8, UTF-16 (little endian, no byte order mark) or an 8-bit code page,
//!   and a code page argument decides: 1252 and 28591 give the Latin bytes (`äöü` is `5Pb8`), 65001 UTF-8.
//!   With code page 0, UTF8, Windows and MSDos all give the UTF-8 bytes (`w6TDtsO8`) and UTF16 the
//!   UTF-16 bytes (`5AD2APwA`);
//! - `FromBase64` reverses it; UTF-8 that is not valid reads as U+FFFD for each bad byte;
//! - input that is not Base-64 (a bad character, a single character, a missing padding) raises
//!   "The input is not a valid Base-64 string as it contains a non-base 64 character, more than two
//!   padding characters, or an illegal character among the padding characters." with the code
//!   `AL:Format`; white space inside is ignored;
//! - `InsertLineBreaks` on a text of more than 76 output characters puts CR LF after each 76 (120
//!   bytes give 164 characters, no trailing break); on an InStream it changes nothing (a 100 byte stream);
//! - the stream forms read all bytes of the InStream / write the decoded bytes to the OutStream.
//!
//! A code page or a character the pin does not cover (a code page other than 0, 1252, 28591 and 65001;
//! a character that Windows-1252 and Latin-1 encode differently) is refused.

use crate::describe::Describe;
use crate::native::{Ctx, NativeFn, Signal, raise};
use crate::stream::{drain_in, write_raw_bytes};
use crate::value::Value;

type Out = Result<Value, Signal>;

fn unsupported<T>(what: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(what.into()))
}

/// One platform native: where it is declared, how it is called, what runs.
pub struct Entry {
    pub object: &'static str,
    pub name: &'static str,
    /// The parameter types as written in AL, lower case.
    pub params: &'static [&'static str],
    pub id: &'static str,
    pub run: NativeFn,
}

pub const TABLE: &[Entry] = &[
    Entry {
        object: "Base64Convert",
        name: "ToBase64",
        params: &["text", "boolean", "textencoding", "integer"],
        id: "Base64Convert.ToBase64#text",
        run: to_base64_text,
    },
    Entry {
        object: "Base64Convert",
        name: "ToBase64",
        params: &["text", "boolean", "textencoding", "integer", "outstream"],
        id: "Base64Convert.ToBase64#text-out",
        run: to_base64_text_out,
    },
    Entry {
        object: "Base64Convert",
        name: "ToBase64",
        params: &["instream", "boolean"],
        id: "Base64Convert.ToBase64#stream",
        run: to_base64_stream,
    },
    Entry {
        object: "Base64Convert",
        name: "ToBase64",
        params: &["instream", "boolean", "outstream"],
        id: "Base64Convert.ToBase64#stream-out",
        run: to_base64_stream_out,
    },
    Entry {
        object: "Base64Convert",
        name: "FromBase64",
        params: &["text", "textencoding", "integer"],
        id: "Base64Convert.FromBase64#text",
        run: from_base64_text,
    },
    Entry {
        object: "Base64Convert",
        name: "FromBase64",
        params: &["text", "textencoding", "integer", "outstream"],
        id: "Base64Convert.FromBase64#text-enc-out",
        run: from_base64_text_enc_out,
    },
    Entry {
        object: "Base64Convert",
        name: "FromBase64",
        params: &["text", "outstream"],
        id: "Base64Convert.FromBase64#text-out",
        run: from_base64_text_out,
    },
    Entry {
        object: "Base64Convert",
        name: "FromBase64",
        params: &["instream"],
        id: "Base64Convert.FromBase64#stream",
        run: from_base64_stream,
    },
    Entry {
        object: "Base64Convert",
        name: "FromBase64",
        params: &["instream", "outstream"],
        id: "Base64Convert.FromBase64#stream-out",
        run: from_base64_stream_out,
    },
    Entry {
        object: "Agent Utilities",
        name: "GetCurrentSessionAgentTaskId",
        params: &[],
        id: "AgentUtilities.GetCurrentSessionAgentTaskId",
        run: agent_task_id,
    },
    Entry {
        object: "Agent Utilities",
        name: "GetCurrentSessionAgentMetadataProviderType",
        params: &[],
        id: "AgentUtilities.GetCurrentSessionAgentMetadataProviderType",
        run: agent_provider_type,
    },
    Entry {
        object: "Feature Access Management",
        name: "AgentManagementAllowedInternal",
        params: &["boolean"],
        id: "FeatureAccessManagement.AgentManagementAllowedInternal",
        run: fam_agent_management_allowed,
    },
    Entry {
        object: "Feature Access Management",
        name: "IsSandboxEnvironmentOrFirstPartyCodeInternal",
        params: &["boolean"],
        id: "FeatureAccessManagement.IsSandboxEnvironmentOrFirstPartyCodeInternal",
        run: fam_microsoft_only,
    },
    Entry {
        object: "Feature Access Management",
        name: "IsEnvironmentPositiveListedInternal",
        params: &[],
        id: "FeatureAccessManagement.IsEnvironmentPositiveListedInternal",
        run: fam_microsoft_only,
    },
    Entry {
        object: "Feature Access Management",
        name: "IsTestSessionInternal",
        params: &[],
        id: "FeatureAccessManagement.IsTestSessionInternal",
        run: fam_microsoft_only,
    },
    Entry {
        object: "Feature Access Management",
        name: "IsSandboxEnvironmentInternal",
        params: &[],
        id: "FeatureAccessManagement.IsSandboxEnvironmentInternal",
        run: fam_is_sandbox,
    },
];

/// The native for the procedure `name` of the codeunit `object` with parameters of these types.
pub fn find(object: &str, name: &str, params: &[String]) -> Option<&'static Entry> {
    TABLE.iter().find(|e| {
        e.object.eq_ignore_ascii_case(object)
            && e.name.eq_ignore_ascii_case(name)
            && e.params.len() == params.len()
            && e.params.iter().zip(params).all(|(a, b)| a.eq_ignore_ascii_case(b))
    })
}

/// The implementation registered under `id`.
pub fn lookup_by_scan(id: &str) -> Option<NativeFn> {
    TABLE.iter().find(|e| e.id == id).map(|e| e.run)
}

/// Every entry, in table order.
pub fn entries() -> impl Iterator<Item = (&'static str, NativeFn)> {
    TABLE.iter().map(|e| (e.id, e.run))
}

// ---------------------------------------------------------------- agents and feature access

// "Agent Utilities" (System.app 2000000022) in a session that is not an agent session: task id 0
// (platform/lt-rh-agent-session/RhAgentSession.al:47 through "Agent Session"; al-language-onprem/platform/
// lt-rh-agent-natives/RhAgentNatives.al:82 called directly) and provider type -1 (RhAgentNatives.al:83).
fn agent_task_id(_: &mut dyn Ctx, _: Option<&mut Value>, _: &mut [Value]) -> Out {
    Ok(Value::Integer(0))
}

fn agent_provider_type(_: &mut dyn Ctx, _: Option<&mut Value>, _: &mut [Value]) -> Out {
    Ok(Value::Integer(-1))
}

/// Whether the code that called into "Feature Access Management" belongs to a Microsoft app: the publisher of the
/// first app on the call stack below System.app's codeunit. `None` when no other app is on the stack.
fn caller_is_microsoft(ctx: &dyn Ctx) -> Option<bool> {
    ctx.calling_app_publisher().map(|p| p == "Microsoft")
}

/// Called from a non-Microsoft app, the access checks of "Feature Access Management" (System.app 2000000020) raise this
/// whatever ThrowError says (al-language-onprem/platform/lt-rh-agent-natives/RhAgentNatives.al:104, :126, :156-178).
fn not_microsoft<T>() -> Result<T, Signal> {
    raise(
        "Feature availability can only be checked by Microsoft applications.",
        "InvalidOperation",
    )
}

// AgentManagementAllowed / AgentTaskManagementPreviewEnabled. From the System Application with ThrowError true it returns
// without an error ("Agent Task Builder".Initialize and SetExternalId, platform/lt-rh-agent-session/RhAgentSession.al:66-69).
// The answer is not pinned; every Microsoft caller (System Application and Base Application 29) discards it, so no AL code
// can read it. Microsoft code with ThrowError false is not pinned: refused.
fn fam_agent_management_allowed(ctx: &mut dyn Ctx, _: Option<&mut Value>, args: &mut [Value]) -> Out {
    match (caller_is_microsoft(ctx), args.first()) {
        (Some(false), _) => not_microsoft(),
        (Some(true), Some(Value::Boolean(true))) => Ok(Value::Boolean(true)),
        _ => unsupported(
            "Feature Access Management.AgentManagementAllowed called by Microsoft code without ThrowError (not pinned)",
        ),
    }
}

// IsSandboxEnvironmentOrFirstPartyCode, IsEnvironmentPositiveListed, IsTestSession: pinned from a non-Microsoft app only.
fn fam_microsoft_only(ctx: &mut dyn Ctx, _: Option<&mut Value>, _: &mut [Value]) -> Out {
    match caller_is_microsoft(ctx) {
        Some(false) => not_microsoft(),
        _ => unsupported("a Feature Access Management check called by Microsoft code (what it answers is not pinned)"),
    }
}

// IsSandboxEnvironment answers Yes, with no error, called from a non-Microsoft app (RhAgentNatives.al:156, the corpus CI
// container). From Microsoft code it is not pinned: refused.
fn fam_is_sandbox(ctx: &mut dyn Ctx, _: Option<&mut Value>, _: &mut [Value]) -> Out {
    match caller_is_microsoft(ctx) {
        Some(false) => Ok(Value::Boolean(true)),
        _ => unsupported(
            "Feature Access Management.IsSandboxEnvironment called by Microsoft code (what it answers is not pinned)",
        ),
    }
}

// ---------------------------------------------------------------- encodings

#[derive(Clone, Copy, PartialEq)]
enum Enc {
    Utf8,
    Utf16,
    Latin1,
    Windows1252,
}

fn member_name(v: &Value) -> Result<String, Signal> {
    match v {
        Value::Option(i, members) => members
            .get(usize::try_from(*i).unwrap_or(usize::MAX))
            .cloned()
            .map_or_else(|| unsupported("a TextEncoding that is not a member"), Ok),
        other => unsupported(format!("TextEncoding given as {}", other.describe())),
    }
}

/// `windows_text`: from BC 28 the Windows and MSDos members with code page 0 are code page 1252
/// (`al_version::Gate::Base64TextIsWindows1252From28`, types/PVBase64.al:83,88).
fn encoding(encoding: &Value, codepage: &Value, windows_text: bool) -> Result<Enc, Signal> {
    let Value::Integer(cp) = codepage else {
        return unsupported("a code page that is not an Integer");
    };
    let name = member_name(encoding)?;
    match *cp {
        0 => match name.to_ascii_lowercase().as_str() {
            "utf16" => Ok(Enc::Utf16),
            "windows" | "msdos" if windows_text => Ok(Enc::Windows1252),
            "utf8" | "windows" | "msdos" => Ok(Enc::Utf8),
            other => unsupported(format!("TextEncoding::{other}")),
        },
        65001
            if name.eq_ignore_ascii_case("utf8")
                || name.eq_ignore_ascii_case("windows")
                || name.eq_ignore_ascii_case("msdos") =>
        {
            Ok(Enc::Utf8)
        }
        1252 if !name.eq_ignore_ascii_case("utf16") => Ok(Enc::Windows1252),
        28591 if !name.eq_ignore_ascii_case("utf16") => Ok(Enc::Latin1),
        other => unsupported(format!(
            "Base64 conversion with code page {other} and TextEncoding::{name} (only 0, 1252, 28591 and 65001 are pinned)"
        )),
    }
}

fn encode_text(s: &str, enc: Enc) -> Result<Vec<u8>, Signal> {
    match enc {
        Enc::Utf8 => Ok(s.as_bytes().to_vec()),
        Enc::Utf16 => Ok(s.encode_utf16().flat_map(u16::to_le_bytes).collect()),
        Enc::Latin1 | Enc::Windows1252 => s
            .chars()
            .map(|c| {
                let u = c as u32;
                // Windows-1252 and Latin-1 agree below 0x80 and from 0xA0; the 0x80 to 0x9F block differs.
                if u < 0x80 || (0xA0..=0xFF).contains(&u) {
                    Ok(u as u8)
                } else {
                    unsupported(format!("U+{u:04X} in a Latin code page conversion (not pinned)"))
                }
            })
            .collect(),
    }
}

fn decode_bytes(b: &[u8], enc: Enc) -> Result<String, Signal> {
    match enc {
        Enc::Utf8 => Ok(String::from_utf8_lossy(b).into_owned()),
        Enc::Utf16 => {
            if b.len() % 2 != 0 {
                return unsupported("UTF-16 bytes of an odd length (not pinned)");
            }
            let units: Vec<u16> = b.chunks(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
            Ok(String::from_utf16_lossy(&units))
        }
        Enc::Latin1 | Enc::Windows1252 => {
            if b.iter().any(|x| (0x80..0xA0).contains(x)) {
                return unsupported("bytes 0x80 to 0x9F in a Latin code page conversion (not pinned)");
            }
            Ok(b.iter().map(|&x| char::from(x)).collect())
        }
    }
}

// ---------------------------------------------------------------- base 64

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn b64_encode(bytes: &[u8], line_breaks: bool) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4 + 8);
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        out.push(ALPHABET[(n >> 18) as usize & 63] as char);
        out.push(ALPHABET[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[n as usize & 63] as char
        } else {
            '='
        });
    }
    if !line_breaks || out.len() <= 76 {
        return out;
    }
    let mut broken = String::with_capacity(out.len() + out.len() / 38);
    for (i, line) in out.as_bytes().chunks(76).enumerate() {
        if i > 0 {
            broken.push_str("\r\n");
        }
        broken.push_str(std::str::from_utf8(line).expect("ascii"));
    }
    broken
}

fn invalid() -> Signal {
    match raise::<()>(
        "The input is not a valid Base-64 string as it contains a non-base 64 character, more than two padding characters, or an illegal character among the padding characters.",
        "AL:Format",
    ) {
        Err(e) => e,
        Ok(()) => unreachable!(),
    }
}

/// Decodes like .NET's `Convert.FromBase64String`: space, tab, CR and LF are ignored; the length is a
/// multiple of four; `=` only as the last one or two characters.
pub(crate) fn b64_decode(text: &str) -> Result<Vec<u8>, Signal> {
    let chars: Vec<u8> = text
        .bytes()
        .filter(|b| !matches!(b, b' ' | b'\t' | b'\r' | b'\n'))
        .collect();
    if chars.len() % 4 != 0 {
        return Err(invalid());
    }
    let pad = chars.iter().rev().take_while(|&&c| c == b'=').count();
    if pad > 2 {
        return Err(invalid());
    }
    let body = &chars[..chars.len() - pad];
    let mut out = Vec::with_capacity(chars.len() / 4 * 3);
    let value = |c: u8| ALPHABET.iter().position(|&a| a == c).map(|p| p as u32);
    for group in body.chunks(4) {
        let mut n = 0u32;
        for &c in group {
            n = (n << 6) | value(c).ok_or_else(invalid)?;
        }
        let used = group.len();
        n <<= 6 * (4 - used) as u32;
        let bytes = [(n >> 16) as u8, (n >> 8) as u8, n as u8];
        out.extend_from_slice(&bytes[..used * 6 / 8]);
    }
    Ok(out)
}

fn text_arg(v: &Value) -> Result<String, Signal> {
    match v {
        Value::Text(t) | Value::Code(t) => Ok(t.to_string()),
        other => unsupported(format!("a Text argument given as {}", other.describe())),
    }
}

fn flag(v: &Value) -> Result<bool, Signal> {
    match v {
        Value::Boolean(b) => Ok(*b),
        other => unsupported(format!("a Boolean argument given as {}", other.describe())),
    }
}

fn to_base64_text(c: &mut dyn crate::native::Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    let bytes = encode_text(
        &text_arg(&a[0])?,
        encoding(&a[2], &a[3], c.gate(al_version::Gate::Base64TextIsWindows1252From28))?,
    )?;
    Ok(Value::Text(b64_encode(&bytes, flag(&a[1])?).into()))
}

fn to_base64_text_out(c: &mut dyn crate::native::Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    let bytes = encode_text(
        &text_arg(&a[0])?,
        encoding(&a[2], &a[3], c.gate(al_version::Gate::Base64TextIsWindows1252From28))?,
    )?;
    let text = b64_encode(&bytes, flag(&a[1])?);
    write_raw_bytes(&mut a[4], text.as_bytes())?;
    Ok(Value::Null)
}

fn to_base64_stream(c: &mut dyn crate::native::Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    // The line breaks flag changes nothing for a stream (corpus types/PVBase64.al: a 100 byte stream).
    flag(&a[1])?;
    // Content carried unread in the company build gives the carried text (owner decision 2026-10-23 "Carry bytes unread").
    if c.company_build() && crate::stream::carried_mark_of_in(&a[0]).is_some() {
        return Ok(Value::Text(crate::native::CARRIED_TEXT_MARK.to_string().into()));
    }
    let bytes = drain_in(&mut a[0])?;
    Ok(Value::Text(b64_encode(&bytes, false).into()))
}

fn to_base64_stream_out(_: &mut dyn crate::native::Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    flag(&a[1])?;
    let bytes = drain_in(&mut a[0])?;
    write_raw_bytes(&mut a[2], b64_encode(&bytes, false).as_bytes())?;
    Ok(Value::Null)
}

fn from_base64_text(c: &mut dyn crate::native::Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    let enc = encoding(&a[1], &a[2], c.gate(al_version::Gate::Base64TextIsWindows1252From28))?;
    Ok(Value::Text(decode_bytes(&b64_decode(&text_arg(&a[0])?)?, enc)?.into()))
}

fn from_base64_text_enc_out(c: &mut dyn crate::native::Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    // The decoded bytes go out as they are; the encoding names the text, which is not built.
    encoding(&a[1], &a[2], c.gate(al_version::Gate::Base64TextIsWindows1252From28))?;
    let bytes = b64_decode(&text_arg(&a[0])?)?;
    write_raw_bytes(&mut a[3], &bytes)?;
    Ok(Value::Null)
}

fn from_base64_text_out(_: &mut dyn crate::native::Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    let bytes = b64_decode(&text_arg(&a[0])?)?;
    write_raw_bytes(&mut a[1], &bytes)?;
    Ok(Value::Null)
}

fn from_base64_stream(_: &mut dyn crate::native::Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    let text = String::from_utf8_lossy(&drain_in(&mut a[0])?).into_owned();
    Ok(Value::Text(decode_bytes(&b64_decode(&text)?, Enc::Utf8)?.into()))
}

fn from_base64_stream_out(_: &mut dyn crate::native::Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    let text = String::from_utf8_lossy(&drain_in(&mut a[0])?).into_owned();
    let bytes = b64_decode(&text)?;
    write_raw_bytes(&mut a[1], &bytes)?;
    Ok(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enc(name: &str, cp: i32) -> Result<Enc, Signal> {
        let members: std::sync::Arc<[String]> = ["MSDos", "UTF8", "UTF16", "Windows"].map(String::from).to_vec().into();
        let i = members.iter().position(|m| m == name).unwrap() as i32;
        encoding(&Value::Option(i, members), &Value::Integer(cp), false)
    }

    fn to(text: &str, name: &str, cp: i32) -> String {
        b64_encode(&encode_text(text, enc(name, cp).ok().unwrap()).ok().unwrap(), false)
    }

    // types/PVBase64.al, ToBase64_Text_AsciiUmlautsEmpty / WithEncodings
    #[test]
    fn encodes_as_the_probe_did() {
        assert_eq!(to("Hello", "UTF8", 0), "SGVsbG8=");
        assert_eq!(to("", "UTF8", 0), "");
        assert_eq!(to("äöü", "UTF8", 0), "w6TDtsO8");
        assert_eq!(to("€", "UTF8", 0), "4oKs");
        assert_eq!(to("ab", "UTF8", 0), "YWI=");
        assert_eq!(to("a", "UTF8", 0), "YQ==");
        assert_eq!(to("abc", "UTF8", 0), "YWJj");
        assert_eq!(to("äöü", "UTF16", 0), "5AD2APwA");
        assert_eq!(to("äöü", "Windows", 0), "w6TDtsO8");
        assert_eq!(to("äöü", "MSDos", 0), "w6TDtsO8");
        assert_eq!(to("äöü", "Windows", 1252), "5Pb8");
        assert_eq!(to("äöü", "Windows", 28591), "5Pb8");
        assert_eq!(to("äöü", "UTF8", 65001), "w6TDtsO8");
    }

    // ToBase64_LongText_LineBreaks: 120 bytes, 160 characters, 164 with breaks, CR LF after 76
    #[test]
    fn line_breaks_after_76_characters() {
        let bytes = vec![b'a'; 120];
        assert_eq!(b64_encode(&bytes, false).len(), 160);
        let with = b64_encode(&bytes, true);
        assert_eq!(with.len(), 164);
        assert_eq!(&with.as_bytes()[76..78], b"\r\n");
        assert!(!with.ends_with('\n'));
        assert_eq!(b64_encode(b"Hello", true), "SGVsbG8=");
    }

    // FromBase64_Text_AndEncodings
    #[test]
    fn decodes_as_the_probe_did() {
        let from = |t: &str, name: &str, cp: i32| {
            decode_bytes(&b64_decode(t).ok().unwrap(), enc(name, cp).ok().unwrap())
                .ok()
                .unwrap()
        };
        assert_eq!(from("SGVsbG8=", "UTF8", 0), "Hello");
        assert_eq!(from("w6TDtsO8", "UTF8", 0), "äöü");
        assert_eq!(from("5Pb8", "Windows", 1252), "äöü");
        assert_eq!(from("5Pb8", "UTF8", 0), "\u{FFFD}\u{FFFD}\u{FFFD}");
        assert_eq!(from("5Pb8", "Windows", 28591), "äöü");
        assert_eq!(from("5AD2APwA", "UTF16", 0), "äöü");
    }

    // FromBase64_Invalid_Text
    #[test]
    fn invalid_input_raises_the_pinned_error_and_white_space_is_ignored() {
        for bad in ["not base64!", "A", "SGVsbG8"] {
            match b64_decode(bad) {
                Err(Signal::Error(e)) => {
                    assert!(
                        e.message.starts_with("The input is not a valid Base-64 string"),
                        "{bad}"
                    );
                    assert_eq!(e.code, "AL:Format");
                }
                other => panic!("{bad}: {:?}", other.is_ok()),
            }
        }
        assert_eq!(b64_decode("SGVs bG8=").ok().unwrap(), b"Hello");
    }

    #[test]
    fn unpinned_code_pages_and_characters_are_refused() {
        assert!(matches!(enc("Windows", 850), Err(Signal::Unsupported(_))));
        assert!(matches!(enc("UTF16", 1252), Err(Signal::Unsupported(_))));
        assert!(matches!(
            encode_text("€", Enc::Windows1252),
            Err(Signal::Unsupported(_))
        ));
    }

    #[test]
    fn the_table_finds_a_procedure_by_object_name_and_parameter_types() {
        let p = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert!(
            find(
                "Base64Convert",
                "ToBase64",
                &p(&["Text", "Boolean", "TextEncoding", "Integer"])
            )
            .is_some()
        );
        assert!(find("Base64Convert", "ToBase64", &p(&["Text"])).is_none());
        assert!(find("Other", "ToBase64", &p(&["Text", "Boolean", "TextEncoding", "Integer"])).is_none());
    }
}
