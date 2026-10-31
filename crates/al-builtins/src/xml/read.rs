//! `XmlDocument.ReadFrom` natives.

use super::*;

pub(super) fn read_from_text(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    read_text_into(c, r, a, 1)
}

/// `XmlDocument.ReadFrom(Text, XmlReadOptions, var XmlDocument)`: the document reads as the form without options
/// does, whether the options are the default or `PreserveWhitespace` is true or false, for white space inside the
/// text, around the root and between tags (misc/W4XmlJsonHttp.al: W4 Xml Options Tests.ReadFrom_*, BC 27.5 and 29.0).
pub(super) fn read_from_text_options(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    read_text_into(c, r, a, 2)
}

/// `XmlReadOptions.PreserveWhitespace(Value)`: the probes see no difference in what is read, so the setting changes
/// nothing there; the variable remembers it for the getter (xml/XmlDocReadWrite.al:215-217).
pub(super) fn read_options_preserve_whitespace(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let Some(Value::Library(_, cell)) = r else {
        return unsupported("PreserveWhitespace on something that is not an XmlReadOptions");
    };
    let Some(Value::Boolean(on)) = a.first() else {
        return unsupported("PreserveWhitespace with something that is not a Boolean");
    };
    *cell.borrow_mut() = LibraryValue::XmlReadOptions {
        preserve_whitespace: *on,
    };
    Ok(Value::Null)
}

/// `XmlReadOptions.PreserveWhitespace()` and `XmlWriteOptions.PreserveWhitespace()`: No until set.
pub(super) fn options_preserve_whitespace_get(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> NResult {
    let Some(Value::Library(_, cell)) = r else {
        return unsupported("PreserveWhitespace on something that is not an options value");
    };
    Ok(Value::Boolean(match &*cell.borrow() {
        LibraryValue::XmlReadOptions { preserve_whitespace }
        | LibraryValue::XmlWriteOptions {
            preserve_whitespace, ..
        } => *preserve_whitespace,
        _ => false,
    }))
}

pub(super) fn read_text_into(c: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value], out: usize) -> NResult {
    let text = arg_text(a, 0)?;
    match parse_document_detail(&text, true)? {
        Ok(doc) => {
            a[out] = tagged("XmlDocument", doc);
            Ok(Value::Boolean(true))
        }
        Err(f) if c.ret_used() => {
            // The text stays in GetLastErrorText (xml/XmlDocReadWrite.al:94-95, ReadFrom_Invalid).
            if let Some(f) = f {
                c.set_last_error(crate::native::AlError {
                    message: format!(
                        "The following exception was encountered when processing XML data: '{}' at line {} and position {}.",
                        f.message, f.line, f.pos
                    ),
                    code: "Xml:Xml".into(),
                    call_stack: String::new(),
                    ..Default::default()
                });
            }
            Ok(Value::Boolean(false))
        }
        Err(Some(Failure {
            message,
            line,
            pos,
            dom: true,
        })) => Err(error(
            format!(
                "The following exception was encountered when processing XML data: '{message}' at line {line} and position {pos}."
            ),
            "Xml:Xml",
        )),
        Err(_) => unsupported(
            "XmlDocument.ReadFrom of invalid XML, result unused (the error text is pinned only for a mismatched end tag)",
        ),
    }
}

/// `XmlDocument.ReadFrom(InStream, var XmlDocument)`: reads the stream to its end and parses it as
/// the text form does (handlers/TestReportDatasetHelpLink.al:RenderDataset reads what `Report.SaveAs`
/// wrote, a UTF-8 document with a byte order mark). The encoding is the byte order mark's, else UTF-8;
/// a UTF-16 document and bytes that are not UTF-8 are refused (not pinned).
pub(super) fn read_from_stream(c: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> NResult {
    // The stream is read from its position without moving it: `EOS` stays false afterwards (xml/TestProbeXmlContracts.al:
    // ReadFrom_Stream_Encodings, `ok=Yes eos=No`).
    let bytes = match a.first() {
        Some(Value::Library(_, h)) => match &*h.borrow() {
            LibraryValue::InStream(s) => s.rest(),
            _ => return unsupported("XmlDocument.ReadFrom of something that is not an InStream"),
        },
        other => return unsupported(format!("XmlDocument.ReadFrom of {}", other.describe())),
    };
    // A byte order mark names the encoding; without one, UTF-8, else UTF-16 little endian when the second byte is zero
    // (a text written with TextEncoding::UTF16). Bytes that are neither read as no document.
    let decoded: Option<String> = if bytes.starts_with(&[0xFF, 0xFE]) || bytes.starts_with(&[0xFE, 0xFF]) {
        let le = bytes[0] == 0xFF;
        let units: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|p| {
                if le {
                    u16::from_le_bytes([p[0], p[1]])
                } else {
                    u16::from_be_bytes([p[0], p[1]])
                }
            })
            .collect();
        String::from_utf16(&units).ok()
    } else if bytes.len() >= 2 && bytes.len() % 2 == 0 && bytes[0] != 0 && bytes[1] == 0 {
        let units: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|p| u16::from_le_bytes([p[0], p[1]]))
            .collect();
        String::from_utf16(&units).ok()
    } else {
        let body = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&bytes);
        std::str::from_utf8(body).ok().map(str::to_string)
    };
    if let Some(text) = &decoded {
        let utf16 = bytes.len() >= 2 && (bytes[0] == 0xFF || bytes[0] == 0xFE || bytes[1] == 0);
        if utf16 {
            if let Some(r) = utf16_declared_encoding(c, text, bytes.starts_with(&[0xFF, 0xFE])) {
                return r;
            }
        }
    }
    let Some(text) = decoded else {
        return if c.ret_used() {
            Ok(Value::Boolean(false))
        } else {
            unsupported(
                "XmlDocument.ReadFrom of a stream whose bytes are no UTF-8 or UTF-16 text, result unused (error text not pinned)",
            )
        };
    };
    let mut args = [Value::Text(text.into()), a[1].clone()];
    let result = read_from_text(c, None, &mut args)?;
    a[1] = args[1].clone();
    Ok(result)
}

/// `XmlDocument.ReadFrom(InStream, XmlReadOptions, var XmlDocument)` (al-runner issue 6049): reads as the stream form
/// without options does. PreserveWhitespace true is pinned (xml/XmlDocReadWrite.al:290-307 ReadFrom_InStream_Options);
/// PreserveWhitespace false and a document that does not parse (false, the text in GetLastErrorText) are pinned by
/// xml/XmlDocReadWrite.al ReadFrom_InStream_Options_WhitespaceNodes and ReadFrom_InStream_Options_Invalid. The
/// statement form of a read that fails is not pinned and is refused.
pub(super) fn read_from_stream_options(c: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> NResult {
    // Both PreserveWhitespace values read as the stream form does.
    match a.get(1) {
        Some(Value::Library(..)) => {}
        other => return unsupported(format!("XmlDocument.ReadFrom with options of {}", other.describe())),
    }
    let mut args = [a[0].clone(), a[2].clone()];
    match read_from_stream(c, None, &mut args) {
        Ok(v) => {
            a[2] = args[1].clone();
            Ok(v)
        }
        Err(e @ Signal::Unsupported(_)) => Err(e),
        Err(_) => unsupported(
            "XmlDocument.ReadFrom(InStream, XmlReadOptions) of a document that does not parse, statement form (not pinned)",
        ),
    }
}

/// The encoding a declaration names in a UTF-16 document. `None`: no declaration, no encoding or UTF-16; the document
/// reads as UTF-16 (xml/lt-ay-xml/AyXmlReadFromStream.al:90-98 ReadFrom_le_bom_decl16). With a UTF-16 LE byte order
/// mark and exactly `<?xml version="1.0" encoding="utf-8"?>` followed by `<`, BC switches to UTF-8 after the declaration
/// and meets the zero byte of `<` (AyXmlReadFromStream.al:103-112 ReadFrom_le_bom_decl8, statement form in a
/// TryFunction). Every other shape is refused: no byte order mark, big endian, other text after the declaration, another
/// declaration or encoding, the form whose result is used.
pub(super) fn utf16_declared_encoding(c: &mut dyn Ctx, text: &str, le_bom: bool) -> Option<NResult> {
    let rest = text.strip_prefix("<?xml")?;
    let end = rest.find("?>")?;
    let decl = &rest[..end];
    let enc_at = decl.find("encoding")?;
    let after = decl[enc_at + "encoding".len()..]
        .trim_start()
        .strip_prefix('=')?
        .trim_start();
    let quote = after.chars().next()?;
    let value = after[1..].split(quote).next().unwrap_or("");
    if value.eq_ignore_ascii_case("utf-16") {
        return None;
    }
    const DECL8: &str = "<?xml version=\"1.0\" encoding=\"utf-8\"?>";
    if le_bom && !c.ret_used() && text.starts_with(DECL8) && text[DECL8.len()..].starts_with('<') {
        // `<` is at position 39 of line 1, its zero byte at 40.
        let pos = DECL8.chars().count() + 2;
        return Some(Err(error(
            format!(
                "The following exception was encountered when processing XML data: 'Name cannot begin with the '.' character, hexadecimal value 0x00. Line 1, position {pos}.' at line 1 and position {pos}."
            ),
            "Xml:Xml",
        )));
    }
    Some(unsupported(format!(
        "XmlDocument.ReadFrom of UTF-16 bytes whose declaration names the encoding `{value}` (only a UTF-16 LE byte order mark with exactly a utf-8 declaration followed by `<`, statement form, is pinned)"
    )))
}
