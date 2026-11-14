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
//!   bytes give 164 characters, no trailing break);
//! - the stream forms read all bytes of the InStream / write the decoded bytes to the OutStream.
//!
//! System.app has `Base64Convert` only from BC 28; System Application 27 converts in AL through DotNet, so these
//! natives run on BC 28 and later. Corpus PR 1394 (tests/al-language/streams/Base64LineBreaks.al, not pinned yet: assumption
//! `base-encode-stream-line-breaks-code-pages`) measured on 28.5 and 29.0 the InStream forms with `InsertLineBreaks` (CR LF
//! after each 76 characters; the whole stream, which does not move) and more `FromBase64` code pages (`from_base64_encoding`).
//!
//! A code page or a character the pins do not cover (for `ToBase64` a code page other than 0, 1252, 28591 and
//! 65001; a character that Windows-1252 and Latin-1 encode differently) is refused.

use crate::describe::Describe;
use crate::native::{Ctx, NativeFn, Signal, raise, unsupported};
use crate::stream::{drain_in, write_raw_bytes};
use crate::value::Value;

type Out = Result<Value, Signal>;

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
    Entry {
        object: "MCP Utilities",
        name: "GetSystemToolsInDynamicMode",
        params: &[],
        id: "MCPUtilities.GetSystemToolsInDynamicMode",
        run: crate::mcp_natives::system_tools_in_dynamic_mode,
    },
    Entry {
        object: "MCP Utilities",
        name: "GetSystemToolsInDataQuery",
        params: &[],
        id: "MCPUtilities.GetSystemToolsInDataQuery",
        run: crate::mcp_natives::system_tools_in_data_query,
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

/// Every entry, in table order.
pub fn entries() -> impl Iterator<Item = (&'static str, NativeFn)> {
    TABLE.iter().map(|e| (e.id, e.run))
}

// ---------------------------------------------------------------- agents and feature access

// "Agent Utilities" (System.app 2000000022) in a session that is not an agent session: task id 0
// (metadata/session-system/RhAgentSession.al:47 through "Agent Session"; al-language-onprem/metadata/
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
/// whatever ThrowError says (al-language-onprem/metadata/session-system/RhAgentNatives.al:104, :126, :156-178).
fn not_microsoft<T>() -> Result<T, Signal> {
    raise(
        "Feature availability can only be checked by Microsoft applications.",
        "InvalidOperation",
    )
}

// AgentManagementAllowed / AgentTaskManagementPreviewEnabled. From the System Application with ThrowError true it returns
// without an error ("Agent Task Builder".Initialize and SetExternalId, metadata/session-system/RhAgentSession.al:66-69).
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
    /// Code pages 850 and 437: only `FromBase64` gives them (`from_base64_encoding`).
    Ibm850,
    Ibm437,
}

/// Unicode of the code page 850 bytes 0x80..=0xFF, as .NET's code page 850 decodes them (Base64LineBreaks.al:516,539
/// measured E4 F6 FC as U+00F5 U+00F7 U+00B3).
const CP850_HIGH: [char; 128] = [
    '\u{00C7}', '\u{00FC}', '\u{00E9}', '\u{00E2}', '\u{00E4}', '\u{00E0}', '\u{00E5}', '\u{00E7}', '\u{00EA}',
    '\u{00EB}', '\u{00E8}', '\u{00EF}', '\u{00EE}', '\u{00EC}', '\u{00C4}', '\u{00C5}', '\u{00C9}', '\u{00E6}',
    '\u{00C6}', '\u{00F4}', '\u{00F6}', '\u{00F2}', '\u{00FB}', '\u{00F9}', '\u{00FF}', '\u{00D6}', '\u{00DC}',
    '\u{00F8}', '\u{00A3}', '\u{00D8}', '\u{00D7}', '\u{0192}', '\u{00E1}', '\u{00ED}', '\u{00F3}', '\u{00FA}',
    '\u{00F1}', '\u{00D1}', '\u{00AA}', '\u{00BA}', '\u{00BF}', '\u{00AE}', '\u{00AC}', '\u{00BD}', '\u{00BC}',
    '\u{00A1}', '\u{00AB}', '\u{00BB}', '\u{2591}', '\u{2592}', '\u{2593}', '\u{2502}', '\u{2524}', '\u{00C1}',
    '\u{00C2}', '\u{00C0}', '\u{00A9}', '\u{2563}', '\u{2551}', '\u{2557}', '\u{255D}', '\u{00A2}', '\u{00A5}',
    '\u{2510}', '\u{2514}', '\u{2534}', '\u{252C}', '\u{251C}', '\u{2500}', '\u{253C}', '\u{00E3}', '\u{00C3}',
    '\u{255A}', '\u{2554}', '\u{2569}', '\u{2566}', '\u{2560}', '\u{2550}', '\u{256C}', '\u{00A4}', '\u{00F0}',
    '\u{00D0}', '\u{00CA}', '\u{00CB}', '\u{00C8}', '\u{0131}', '\u{00CD}', '\u{00CE}', '\u{00CF}', '\u{2518}',
    '\u{250C}', '\u{2588}', '\u{2584}', '\u{00A6}', '\u{00CC}', '\u{2580}', '\u{00D3}', '\u{00DF}', '\u{00D4}',
    '\u{00D2}', '\u{00F5}', '\u{00D5}', '\u{00B5}', '\u{00FE}', '\u{00DE}', '\u{00DA}', '\u{00DB}', '\u{00D9}',
    '\u{00FD}', '\u{00DD}', '\u{00AF}', '\u{00B4}', '\u{00AD}', '\u{00B1}', '\u{2017}', '\u{00BE}', '\u{00B6}',
    '\u{00A7}', '\u{00F7}', '\u{00B8}', '\u{00B0}', '\u{00A8}', '\u{00B7}', '\u{00B9}', '\u{00B3}', '\u{00B2}',
    '\u{25A0}', '\u{00A0}',
];

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

/// The encoding of `FromBase64(Text, TextEncoding, Integer)`. The cells listed here are measured; every other pair keeps
/// [`encoding`]'s answer (the `ToBase64` cells, which also answer UTF8 with 0 or 65001, Windows and MSDos with 0, 1252,
/// 28591 or 65001, and UTF16 with 0), and its refusal of every other code page.
fn from_base64_encoding(encoding_arg: &Value, codepage: &Value, windows_text: bool) -> Result<Enc, Signal> {
    use crate::assumptions::{Assumption::BaseEncodeStreamLineBreaksCodePages as A, assume};
    let Value::Integer(cp) = codepage else {
        return unsupported("a code page that is not an Integer");
    };
    let name = member_name(encoding_arg)?;
    match (name.to_ascii_lowercase().as_str(), *cp) {
        // The overload without a code page passes 1252: UTF8 decodes UTF-8 (types/PVBase64.al:137,139) and UTF16 UTF-16 (:141).
        ("utf8", 1252) => Ok(Enc::Utf8),
        ("utf16", 1252) => Ok(Enc::Utf16),
        // UTF16 and UTF8 ignore the code page (Base64LineBreaks.al:456-471, :483-491).
        ("utf16", 65001 | 1200 | 1201 | 28591 | 99999) => {
            assume(A);
            Ok(Enc::Utf16)
        }
        ("utf8", 28591 | 1200 | 99999) => {
            assume(A);
            Ok(Enc::Utf8)
        }
        // Windows and MSDos decode as the code page says (Base64LineBreaks.al:504-505,515-516; :528-529,539-540).
        ("windows", 1200) => {
            assume(A);
            Ok(Enc::Utf16)
        }
        ("windows" | "msdos", 850) => {
            assume(A);
            Ok(Enc::Ibm850)
        }
        ("msdos", 437) => {
            assume(A);
            Ok(Enc::Ibm437)
        }
        // Held cells (Base64LineBreaks.al header, :19-22): BC 28 and 29 fail the test itself with an exception that
        // asserterror and TryFunction do not catch.
        ("windows" | "msdos", 99999 | 70000) => unsupported(format!(
            "FromBase64 with code page {cp} and TextEncoding::{name}: real BC raises an unexpected CLR exception no AL code catches"
        )),
        _ => encoding(encoding_arg, codepage, windows_text),
    }
}

fn encode_text(s: &str, enc: Enc) -> Result<Vec<u8>, Signal> {
    match enc {
        Enc::Ibm850 | Enc::Ibm437 => unsupported("ToBase64 with code page 850 or 437 (not pinned)"),
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
            // Half of a surrogate pair: real BC keeps it in the text; this runtime cannot hold it until ADR-068 is built,
            // and U+FFFD would be a guessed value (hard rule 4).
            String::from_utf16(&units).map_err(|_| {
                Signal::Unsupported(
                    "a Base64 UTF-16 decode that ends up with half of a surrogate pair (not pinned)".into(),
                )
            })
        }
        Enc::Latin1 | Enc::Windows1252 => {
            if b.iter().any(|x| (0x80..0xA0).contains(x)) {
                return unsupported("bytes 0x80 to 0x9F in a Latin code page conversion (not pinned)");
            }
            Ok(b.iter().map(|&x| char::from(x)).collect())
        }
        Enc::Ibm850 => Ok(oem_decode(b, &CP850_HIGH)),
        Enc::Ibm437 => Ok(oem_decode(b, &crate::stream::CP437_HIGH)),
    }
}

/// Bytes of an OEM code page: below 0x80 ASCII, from 0x80 the code page's table.
fn oem_decode(b: &[u8], high: &[char; 128]) -> String {
    b.iter()
        .map(|&x| {
            if x < 0x80 {
                char::from(x)
            } else {
                high[usize::from(x - 0x80)]
            }
        })
        .collect()
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

/// The bytes `ToBase64(InStream, true[, OutStream])` converts, in the shapes corpus PR 1394 measured on BC 28.5 and 29.0
/// (assumption `base-encode-stream-line-breaks-code-pages`): a stream on a Blob buffer that is not a file (Temp Blob,
/// Base64LineBreaks.al:53-61), every byte of it, and the stream does not move (10 bytes read before: the whole stream,
/// :186-193; a second call gives the same text, :195-198). `from_start_only`: the OutStream form, measured only on a stream
/// nothing was read from (:284-286, :364-367). Every other stream is refused.
fn line_broken_source(stream: &mut Value, from_start_only: bool) -> Result<Vec<u8>, Signal> {
    match (crate::stream::blob_bytes_unmoved(stream)?, from_start_only) {
        (Some((bytes, _)), false) | (Some((bytes, 0)), true) => {
            crate::assumptions::assume(crate::assumptions::Assumption::BaseEncodeStreamLineBreaksCodePages);
            Ok(bytes)
        }
        (Some(_), true) => {
            unsupported("Base64Convert.ToBase64(InStream, true, OutStream) on a stream read part way (not measured)")
        }
        (None, _) => unsupported(
            "Base64Convert.ToBase64 with line breaks of a stream that is not on a Blob buffer, or is on a file (not measured)",
        ),
    }
}

fn to_base64_stream(c: &mut dyn crate::native::Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    let line_breaks = flag(&a[1])?;
    // Content carried unread in the company build gives the carried text (owner decision 2026-10-23 "Carry bytes unread").
    if c.company_build() && crate::stream::carried_mark_of_in(&a[0]).is_some() {
        return Ok(Value::Text(crate::native::CARRIED_TEXT_MARK.to_string().into()));
    }
    // CR LF after each 76 characters, none after the last line (Base64LineBreaks.al:103-168).
    let bytes = if line_breaks {
        line_broken_source(&mut a[0], false)?
    } else {
        // The bytes from the read position on; the stream moves to its end (types/PVBase64.al:208, from position 0).
        drain_in(&mut a[0])?
    };
    Ok(Value::Text(b64_encode(&bytes, line_breaks).into()))
}

fn to_base64_stream_out(_: &mut dyn crate::native::Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    let line_breaks = flag(&a[1])?;
    // Appended to what the target holds, CR LF as in the string form (Base64LineBreaks.al:303-348, :385-402).
    let bytes = if line_breaks {
        line_broken_source(&mut a[0], true)?
    } else {
        drain_in(&mut a[0])?
    };
    write_raw_bytes(&mut a[2], b64_encode(&bytes, line_breaks).as_bytes())?;
    Ok(Value::Null)
}

fn from_base64_text(c: &mut dyn crate::native::Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    let enc = from_base64_encoding(&a[1], &a[2], c.gate(al_version::Gate::Base64TextIsWindows1252From28))?;
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

    // UTF-16 bytes that hold half of a surrogate pair (00 D8 alone): real BC keeps the half in the text, which this runtime
    // cannot hold until ADR-068 is built, so it refuses instead of giving U+FFFD (hard rule 4).
    #[test]
    fn a_utf16_half_pair_refuses() {
        let r = decode_bytes(&[0x00, 0xD8, 0x61, 0x00], enc("UTF16", 0).ok().unwrap());
        assert!(
            matches!(&r, Err(Signal::Unsupported(w)) if w.contains("half of a surrogate pair")),
            "{r:?}"
        );
        assert_eq!(
            decode_bytes(&[0x3D, 0xD8, 0x00, 0xDE], enc("UTF16", 0).ok().unwrap())
                .ok()
                .unwrap(),
            "😀"
        );
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

    // ---- ToBase64 of an InStream and FromBase64 code pages (corpus PR 1394, tests/al-language/streams/Base64LineBreaks.al)

    use crate::assumptions::{Assumption, take_hits};
    use crate::library_kind::LibraryKind;
    use crate::stream::{BlobRef, Encoding, InState, new_blob, write_bytes};
    use crate::value::LibraryValue;
    use std::{cell::RefCell, rc::Rc};

    /// `Digits(Count)` of the probe: '0123456789' repeated, cut to `n` bytes.
    fn digits(n: usize) -> Vec<u8> {
        (0..n).map(|i| b'0' + (i % 10) as u8).collect()
    }

    fn blob_with(bytes: &[u8]) -> BlobRef {
        let b = new_blob();
        write_bytes(&b, bytes);
        b
    }

    fn in_stream_on(b: BlobRef) -> Value {
        let s = InState::live(b, Encoding::default());
        Value::Library(LibraryKind::InStream, Rc::new(RefCell::new(LibraryValue::InStream(s))))
    }

    fn in_stream(bytes: &[u8]) -> Value {
        in_stream_on(blob_with(bytes))
    }

    fn out_stream_on(b: &BlobRef) -> Value {
        Value::Library(
            LibraryKind::OutStream,
            Rc::new(RefCell::new(LibraryValue::OutStream(
                b.clone(),
                Encoding::default(),
                None,
            ))),
        )
    }

    fn pos_of(v: &Value) -> usize {
        match v {
            Value::Library(_, h) => match &*h.borrow() {
                LibraryValue::InStream(s) => s.pos,
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }
    }

    fn set_pos(v: &Value, pos: usize) {
        if let Value::Library(_, h) = v
            && let LibraryValue::InStream(s) = &mut *h.borrow_mut()
        {
            s.pos = pos;
        }
    }

    fn run(id: &str, args: &mut [Value]) -> Out {
        let f = TABLE.iter().find(|e| e.id == id).expect("entry").run;
        let mut ctx = crate::native::tests::TestCtx::default();
        f(&mut ctx, None, args)
    }

    fn stream_text(v: Value, line_breaks: bool) -> String {
        match run("Base64Convert.ToBase64#stream", &mut [v, Value::Boolean(line_breaks)]) {
            Ok(Value::Text(t)) => t.to_string(),
            other => panic!("{other:?}"),
        }
    }

    fn codes(s: &str, from: usize, count: usize) -> Vec<u32> {
        s.chars().skip(from - 1).take(count).map(|c| c as u32).collect()
    }

    // al-runner issue 7996, the reporter's repro: 100 'A' bytes through ToBase64(InStream, true), StrPos of CR LF is 77.
    #[test]
    fn issue_7996_instream_to_base64_keeps_line_breaks() {
        let r = stream_text(in_stream(&[b'A'; 100]), true);
        assert_eq!(r.find("\r\n").map(|i| i + 1), Some(77), "length {}", r.len());
        assert_eq!(r.len(), 138);
    }

    // Base64LineBreaks.al:103-168: lengths and the codes at 76-78 and 153-157 with and without line breaks.
    #[test]
    fn instream_line_breaks_as_the_probe_measured() {
        for (n, with, without) in [(57, 76, 76), (100, 138, 136), (114, 154, 152), (120, 164, 160)] {
            let t = stream_text(in_stream(&digits(n)), true);
            let f = stream_text(in_stream(&digits(n)), false);
            assert_eq!((t.len(), f.len()), (with, without), "{n} bytes");
            assert_eq!(t.replace("\r\n", ""), f, "{n} bytes");
            assert!(!t.ends_with('\n'), "{n} bytes");
        }
        let t = stream_text(in_stream(&digits(120)), true);
        assert_eq!(codes(&t, 76, 3), [50, 13, 10]);
        assert_eq!(codes(&t, 153, 5), [73, 122, 13, 10, 78]);
        let t = stream_text(in_stream(&digits(114)), true);
        assert_eq!(codes(&t, 153, 5), [73, 122]);
        // :221-252, the Text form with true gives the same string.
        assert_eq!(t, b64_encode(&digits(114), true));
        assert_eq!(stream_text(in_stream(&[]), true), "");
        assert!(take_hits().contains(&Assumption::BaseEncodeStreamLineBreaksCodePages));
    }

    // Base64LineBreaks.al:186-216 (BC28PLUS): 10 bytes read before the call, the whole stream is converted; a second call
    // on the same stream gives the same text.
    #[test]
    fn instream_with_line_breaks_converts_the_whole_stream_and_does_not_move() {
        let s = in_stream(&digits(100));
        set_pos(&s, 10);
        let r = stream_text(s.clone(), true);
        assert_eq!(r, b64_encode(&digits(100), true));
        assert_eq!(&r[..8], "MDEyMzQ1");
        let s = in_stream(&digits(100));
        let first = stream_text(s.clone(), true);
        assert_eq!(pos_of(&s), 0);
        assert_eq!(stream_text(s, true), first);
    }

    // Without line breaks main's result stays: the bytes from the read position on, and the stream moves to its end
    // (types/PVBase64.al:208 pins the value from position 0).
    #[test]
    fn instream_without_line_breaks_keeps_mains_result() {
        let s = in_stream(&digits(100));
        set_pos(&s, 10);
        assert_eq!(stream_text(s.clone(), false), b64_encode(&digits(100)[10..], false));
        assert_eq!(pos_of(&s), 100);
        take_hits();
        assert_eq!(stream_text(in_stream(b"Hello"), false), "SGVsbG8=");
        assert!(!take_hits().contains(&Assumption::BaseEncodeStreamLineBreaksCodePages));
    }

    // A stream on a file, and a stream whose bytes are not a Blob buffer, were not measured with line breaks: refused.
    #[test]
    fn instream_with_line_breaks_on_an_unmeasured_source_is_refused() {
        let b = blob_with(&digits(100));
        b.borrow_mut().file = true;
        let r = run(
            "Base64Convert.ToBase64#stream",
            &mut [in_stream_on(b), Value::Boolean(true)],
        );
        assert!(
            matches!(&r, Err(Signal::Unsupported(w)) if w.contains("not measured")),
            "{r:?}"
        );
        let snap = Value::Library(
            LibraryKind::InStream,
            Rc::new(RefCell::new(LibraryValue::InStream(InState::snapshot(
                digits(100),
                Encoding::default(),
            )))),
        );
        let r = run("Base64Convert.ToBase64#stream", &mut [snap, Value::Boolean(true)]);
        assert!(
            matches!(&r, Err(Signal::Unsupported(w)) if w.contains("not measured")),
            "{r:?}"
        );
    }

    // Base64LineBreaks.al:303-348 (BC28PLUS): the OutStream overload writes the same characters; :351-403 a target that
    // held 'AB' keeps it and gets the text after it.
    #[test]
    fn outstream_overload_writes_the_line_broken_text() {
        for (n, with, without) in [(57, 76, 76), (100, 138, 136), (120, 164, 160)] {
            for (flag, len) in [(true, with), (false, without)] {
                let target = new_blob();
                let mut args = [in_stream(&digits(n)), Value::Boolean(flag), out_stream_on(&target)];
                run("Base64Convert.ToBase64#stream-out", &mut args).ok().unwrap();
                let bytes = target.borrow().bytes.clone();
                assert_eq!(bytes.len(), len, "{n} {flag}");
                assert_eq!(bytes, b64_encode(&digits(n), flag).into_bytes());
            }
        }
        let target = blob_with(b"AB");
        let mut args = [in_stream(&digits(100)), Value::Boolean(true), out_stream_on(&target)];
        run("Base64Convert.ToBase64#stream-out", &mut args).ok().unwrap();
        let bytes = target.borrow().bytes.clone();
        assert_eq!(bytes.len(), 140);
        assert_eq!(&bytes[..4], b"ABMD");
        assert_eq!(&bytes[77..80], b"2\r\n");
    }

    // The OutStream overload was measured only on a stream nothing was read from: one read part way is refused.
    #[test]
    fn outstream_overload_with_line_breaks_after_a_read_is_refused() {
        let s = in_stream(&digits(100));
        set_pos(&s, 10);
        let target = new_blob();
        let r = run(
            "Base64Convert.ToBase64#stream-out",
            &mut [s, Value::Boolean(true), out_stream_on(&target)],
        );
        assert!(
            matches!(&r, Err(Signal::Unsupported(w)) if w.contains("not measured")),
            "{r:?}"
        );
    }

    fn from_cp(text: &str, name: &str, cp: i32) -> Result<String, Signal> {
        let members: std::sync::Arc<[String]> = ["MSDos", "UTF8", "UTF16", "Windows"].map(String::from).to_vec().into();
        let i = members.iter().position(|m| m == name).unwrap() as i32;
        let mut args = [Value::Text(text.into()), Value::Option(i, members), Value::Integer(cp)];
        match run("Base64Convert.FromBase64#text", &mut args)? {
            Value::Text(t) => Ok(t.to_string()),
            other => panic!("{other:?}"),
        }
    }

    fn cps(s: &str) -> Vec<u32> {
        s.chars().map(|c| c as u32).collect()
    }

    // al-runner issue 7997 item 2, the reporter's repro: FromBase64('QQBCAA==', UTF16, 1252) = 'AB'.
    #[test]
    fn issue_7997_from_base64_utf16_with_code_page_1252() {
        assert_eq!(from_cp("QQBCAA==", "UTF16", 1252).ok().unwrap(), "AB");
    }

    // Base64LineBreaks.al:452-493 and types/PVBase64.al:137,139,141: UTF16 and UTF8 ignore the code page.
    #[test]
    fn from_base64_utf16_and_utf8_ignore_the_code_page() {
        for cp in [1252, 0, 65001, 1200, 1201, 28591, 99999] {
            assert_eq!(from_cp("QQBCAA==", "UTF16", cp).ok().unwrap(), "AB", "{cp}");
        }
        assert_eq!(from_cp("5AD2APwA", "UTF16", 1252).ok().unwrap(), "äöü");
        assert_eq!(from_cp("w6TDtsO8", "UTF8", 1252).ok().unwrap(), "äöü");
        assert_eq!(from_cp("w6TDtsO8", "UTF8", 99999).ok().unwrap(), "äöü");
        assert_eq!(cps(&from_cp("5Pb8", "UTF8", 1252).ok().unwrap()), [65533; 3]);
        assert_eq!(cps(&from_cp("5Pb8", "UTF8", 28591).ok().unwrap()), [65533; 3]);
        assert_eq!(cps(&from_cp("QQBCAA==", "UTF8", 1200).ok().unwrap()), [65, 0, 66, 0]);
    }

    // Base64LineBreaks.al:496-543: Windows and MSDos decode as the code page says.
    #[test]
    fn from_base64_windows_and_msdos_code_pages() {
        assert_eq!(cps(&from_cp("5Pb8", "Windows", 850).ok().unwrap()), [245, 247, 179]);
        assert_eq!(cps(&from_cp("5Pb8", "MSDos", 850).ok().unwrap()), [245, 247, 179]);
        assert_eq!(cps(&from_cp("5Pb8", "MSDos", 437).ok().unwrap()), [931, 247, 8319]);
        assert_eq!(from_cp("QQBCAA==", "Windows", 1200).ok().unwrap(), "AB");
        assert_eq!(from_cp("5Pb8", "MSDos", 1252).ok().unwrap(), "äöü");
        assert_eq!(from_cp("w6TDtsO8", "MSDos", 65001).ok().unwrap(), "äöü");
        // Pairs outside the new cells keep main's answer, unflagged: Windows 28591 is Latin-1 (types/PVBase64.al:140).
        take_hits();
        assert_eq!(from_cp("5Pb8", "Windows", 28591).ok().unwrap(), "äöü");
        assert_eq!(from_cp("5Pb8", "Windows", 1252).ok().unwrap(), "äöü");
        assert!(!take_hits().contains(&Assumption::BaseEncodeStreamLineBreaksCodePages));
        // Only FromBase64 gives code page 850 or 437; ToBase64 with them would be refused.
        assert!(matches!(encode_text("a", Enc::Ibm850), Err(Signal::Unsupported(_))));
        // Code page 0 on BC 28 and 29 is Windows-1252 (the context has every gate).
        assert_eq!(
            cps(&from_cp("w6TDtsO8", "Windows", 0).ok().unwrap()),
            [195, 164, 195, 182, 195, 188]
        );
    }

    // Code pages above 65535 with Windows or MSDos: BC 28 and 29 fail the test with an exception no AL code catches (held cells
    // of Base64LineBreaks.al, header :19-22). Code pages nobody measured (1361) and 70000 with UTF16 are refused too.
    #[test]
    fn from_base64_unmeasured_code_pages_are_refused() {
        for (name, cp) in [
            ("Windows", 99999),
            ("MSDos", 99999),
            ("Windows", 70000),
            ("MSDos", 70000),
        ] {
            let r = from_cp("5Pb8", name, cp);
            assert!(
                matches!(&r, Err(Signal::Unsupported(w)) if w.contains("CLR exception")),
                "{name} {cp} {r:?}"
            );
        }
        for (name, cp) in [
            ("Windows", 1361),
            ("MSDos", 1200),
            ("Windows", 437),
            ("UTF16", 70000),
            ("UTF8", 850),
        ] {
            assert!(
                matches!(from_cp("5Pb8", name, cp), Err(Signal::Unsupported(_))),
                "{name} {cp}"
            );
        }
    }

    // ToBase64 of a Text keeps main's code page handling: UTF8 with 1252 still gives the Latin bytes (not measured by the
    // probe, which measured FromBase64 only).
    #[test]
    fn to_base64_text_code_pages_keep_mains_result() {
        assert_eq!(to("äöü", "UTF8", 1252), "5Pb8");
        assert!(matches!(enc("UTF16", 1252), Err(Signal::Unsupported(_))));
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
