//! The text encoding a VariableText import decodes its source with: the encoding in force when the port reads, which is the
//! declared one unless the program set another with TextEncoding(...) (al-runner issue 7775; pinned by
//! xmlports/import/XpVarTextWindowsImport.al, BC 27.5, 28.5 and 29.0 alike).

use super::*;
use al_builtins::stream::Encoding;

/// The member indexes `TextEncoding(...)` stores (`assigned::set_encoding`): MSDos 0, UTF8 1, UTF16 2, Windows 3.
const SET_UTF8: i64 = 1;
const SET_WINDOWS: i64 = 3;

/// The encoding a VariableText import decodes with, for the pinned pairs of declared and set encodings; `None` for a port that
/// declares no encoding (the source is read as before: UTF-8, UTF-16 by its byte order mark). Pinned pairs:
/// - declared WINDOWS, nothing set or Windows set in OnPreXmlPort: Windows-1252 (XpVarTextWindowsImport.al:349
///   VTW_Columns_WindowsFile_SetWindows_Semicolon_Import, :361 VTW_Columns_WindowsFile_Declared_Semicolon_Import, :433
///   VTW_Fields_Windows_WindowsFile_Import; UTF-8 bytes read one character per byte, :373, :445);
/// - declared WINDOWS, UTF8 set in OnPreXmlPort (:337, :385, :421) or on the variable before Import (:457): UTF-8;
/// - declared UTF8, nothing set: UTF-8, U+FFFD for each byte that is not UTF-8 (:469, :481).
///
/// Declared WINDOWS with MSDos or UTF16 set is refused; every other port reads as before (`None`).
pub(super) fn text_import_encoding(inst: &Inst, xp: &ir::XmlPort) -> Result<Option<Encoding>, Signal> {
    if xp.format != ir::XmlFormat::VariableText {
        return Ok(None);
    }
    let set = assigned::set_encoding(inst);
    match (xp.text_encoding.as_deref(), xp.utf8, set) {
        (Some("windows"), _, None | Some(SET_WINDOWS)) => Ok(Some(Encoding::Windows)),
        (Some("windows"), _, Some(SET_UTF8)) => Ok(Some(Encoding::Utf8)),
        (Some("windows"), _, Some(_)) => {
            unsupported("a VariableText import into a port that declares WINDOWS with MSDos or UTF16 set (not pinned)")
        }
        (None, true, None) => Ok(Some(Encoding::Utf8)),
        // Every other port reads its source as before; `xp_import_document` refuses what is not pinned.
        _ => Ok(None),
    }
}

/// Decodes the source `bytes` of a VariableText import under Windows-1252 (`text_import_encoding`). A byte order mark, and the
/// five bytes code page 1252 leaves undefined, are not pinned and refused. (UTF-8 is read by `xp_read_source` as before, with
/// U+FFFD for each byte that is not UTF-8.)
pub(super) fn decode_windows_source(bytes: &[u8]) -> Result<String, Signal> {
    if has_byte_order_mark(bytes) {
        return unsupported(
            "a VariableText import under TextEncoding = WINDOWS of a source with a byte order mark (not pinned)",
        );
    }
    if bytes.iter().any(|b| matches!(b, 0x81 | 0x8D | 0x8F | 0x90 | 0x9D)) {
        return unsupported(
            "a VariableText import under TextEncoding = WINDOWS of a byte code page 1252 leaves undefined (not pinned)",
        );
    }
    Ok(Encoding::Windows.decode(bytes))
}

/// The `Instance::xp_text` kind that holds the source bytes of a `defers_decoding` import, one char per byte, from the read before
/// OnPreXmlPort until the import decodes them.
const RAW_SOURCE: u8 = 18;

/// Whether the import reads its source before OnPreXmlPort (P112XpCsv.al:P112_Import_EmptySource) but decodes it after: a
/// VariableText port that declares WINDOWS decodes with the encoding in force when it reads, also one OnPreXmlPort set
/// (XpVarTextWindowsImport.al:337 VTW_Columns_Utf8File_SetUtf8_Semicolon_Import, :385 VTW_Columns_WindowsFile_SetUtf8_Semicolon_Import).
pub(super) fn defers_decoding(xp: &ir::XmlPort) -> bool {
    xp.format == ir::XmlFormat::VariableText && xp.text_encoding.as_deref() == Some("windows")
}

/// The `Instance::xp_text` kind that holds the encoding set (`assigned::set_encoding`, empty for none) when `take_raw_decoded`
/// decoded the source.
const DECODED_WITH: u8 = 19;

/// Keeps the source `bytes` of a `defers_decoding` import until `take_raw`. It runs after OnInitXmlPort, before OnPreXmlPort: an
/// encoding set on the variable before Import (XpVarTextWindowsImport.al:457) or in OnPreXmlPort (:337, :349, :385) is pinned, one
/// set in OnInitXmlPort is not and refused.
pub(super) fn stash_raw(inst: &Inst, bytes: &[u8]) -> Result<(), Signal> {
    if assigned::set_in_run(inst).is_some() {
        return unsupported(
            "a VariableText import into a port that declares WINDOWS with TextEncoding set in OnInitXmlPort (not pinned)",
        );
    }
    let mut i = inst.borrow_mut();
    i.xp_text.retain(|(k, _)| *k != RAW_SOURCE && *k != DECODED_WITH);
    i.xp_text
        .push((RAW_SOURCE, bytes.iter().map(|&b| char::from(b)).collect()));
    Ok(())
}

/// Drops the source bytes `stash_raw` kept (a run that ends before its import).
pub(super) fn drop_raw(inst: &Inst) {
    inst.borrow_mut()
        .xp_text
        .retain(|(k, _)| *k != RAW_SOURCE && *k != DECODED_WITH);
}

/// Whether this run decoded its source in `take_raw_decoded`, with the encoding set in force then: an encoding set before that
/// is pinned (`stash_raw`).
pub(super) fn decoded_in_run(inst: &Inst) -> bool {
    inst.borrow().xp_text.iter().any(|(k, _)| *k == DECODED_WITH)
}

/// Refuses an import whose triggers set another encoding after `take_raw_decoded` decoded the source (in OnAfterInitRecord, say):
/// whether BC decodes the rest of the source again is not pinned. Checked after each OnAfterInitRecord and when the records end.
pub(super) fn encoding_unchanged_since_decoding(inst: &Inst) -> Result<(), Signal> {
    let then = inst
        .borrow()
        .xp_text
        .iter()
        .find(|(k, _)| *k == DECODED_WITH)
        .map(|(_, v)| v.clone());
    let now = assigned::set_encoding(inst).map_or(String::new(), |e| e.to_string());
    match then {
        Some(then) if then != now => unsupported(
            "a VariableText import into a port that declares WINDOWS with TextEncoding set after the source was decoded (not pinned)",
        ),
        _ => Ok(()),
    }
}

/// The source bytes `stash_raw` kept, decoded with the encoding in force now (`text_import_encoding`).
pub(super) fn take_raw_decoded(inst: &Inst, xp: &ir::XmlPort) -> Result<Option<String>, Signal> {
    let raw = {
        let mut i = inst.borrow_mut();
        let at = i.xp_text.iter().position(|(k, _)| *k == RAW_SOURCE);
        at.map(|a| i.xp_text.remove(a).1)
    };
    let Some(raw) = raw else {
        return Ok(None);
    };
    // Every char of `raw` is one byte (`stash_raw`).
    let bytes: Vec<u8> = raw.chars().map(|c| c as u8).collect();
    let set = assigned::set_encoding(inst).map_or(String::new(), |e| e.to_string());
    inst.borrow_mut().xp_text.push((DECODED_WITH, set));
    match text_import_encoding(inst, xp)? {
        Some(Encoding::Windows) => decode_windows_source(&bytes).map(Some),
        Some(Encoding::Utf8) => {
            if has_byte_order_mark(&bytes) {
                return unsupported(
                    "a VariableText import under TextEncoding UTF8 set over a declared WINDOWS of a source with a byte order mark (not pinned)",
                );
            }
            // XpVarTextWindowsImport.al:385: U+FFFD for each byte that is not UTF-8.
            Ok(Some(String::from_utf8_lossy(&bytes).into_owned()))
        }
        _ => unsupported("a VariableText import into a port that declares WINDOWS decoded otherwise (not pinned)"),
    }
}

/// Whether `bytes` start with a byte order mark: UTF-8 (EF BB BF), UTF-16 little endian (FF FE) or big endian (FE FF). No import
/// under WINDOWS, or under UTF8 set over it, pins one.
fn has_byte_order_mark(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0xEF, 0xBB, 0xBF]) || bytes.starts_with(&[0xFF, 0xFE]) || bytes.starts_with(&[0xFE, 0xFF])
}
