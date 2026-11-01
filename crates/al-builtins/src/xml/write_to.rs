//! `WriteTo` natives.

use super::*;

pub(super) fn write_to_text(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    // An unassigned XmlElement raises NotInitialized naming XmlNode (xml/XmlElement.al:826 Default_Element_Uninitialized `writeto`).
    if unassigned(&r, "XmlElement") {
        return Err(uninitialized("XmlNode"));
    }
    let h = recv_or_uninit(r, "XmlNode")?;
    a[0] = Value::Text(serialize(&h)?.into());
    Ok(Value::Boolean(true))
}

/// `WriteTo(XmlWriteOptions, var Text)`.
/// - A text, CDATA, comment, processing instruction, document type or declaration is written as by `WriteTo(Text)`, whatever
///   the options say (xml/lt-xml-remaining-node-methods/P104Write{Text,CData,Comment,PI,DocType,Decl}.al:P104_Text_Detached,
///   P104_Text_InDoc `m2`..`m4`; P104DocTypeCreate.al:P104_Create_WriteTo_Overloads `opt_true`, `opt_false`); an unassigned
///   document type or declaration raises `NotInitialized` naming `XmlNode` (P104WriteDecl.al:340 `uninit2`).
/// - A document or element with `PreserveWhitespace(true)` is written unindented, as by `WriteTo(XmlWriteOptions, OutStream)`
///   (xml/XmlDocReadWrite.al:392 WriteTo_Options, :418 WriteTo_Options_Whitespace; P104WriteNode.al `v2_m2`).
/// - An attribute is not pinned (P97AttrWrite.al:WriteTo_TextOptions asserts nothing).
pub(super) fn write_to_text_options(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let h = recv_or_uninit(r, "XmlNode")?;
    let indent = match &a[0] {
        Value::Library(_, cell) => match &*cell.borrow() {
            LibraryValue::XmlWriteOptions { unindented, .. } => !unindented,
            LibraryValue::Empty => true,
            _ => return unsupported("WriteTo with something that is not an XmlWriteOptions"),
        },
        _ => return unsupported("WriteTo with something that is not an XmlWriteOptions"),
    };
    let untouched = matches!(&a[0], Value::Library(_, cell) if matches!(&*cell.borrow(), LibraryValue::Empty));
    let text = match kind(&h) {
        "XmlAttribute" => return unsupported("XmlAttribute.WriteTo(XmlWriteOptions, Text) (not pinned)"),
        "XmlDocument" | "XmlElement" if untouched => {
            return unsupported(
                "WriteTo(XmlWriteOptions, Text) of a document or element with options never set (not pinned)",
            );
        }
        "XmlDocument" | "XmlElement" => serialize_with(&h, indent)?,
        _ => serialize(&h)?,
    };
    a[1] = Value::Text(text.into());
    Ok(Value::Boolean(true))
}

/// `XmlAttribute.WriteTo(OutStream)`: the attribute's text in UTF-8, without a byte order mark
/// (xml/lt-xml-attr-doc-http-members/P97AttrWrite.al:150 WriteTo_OutStream `k1`..`k6`: `a="plain"` is 9 bytes, the first
/// character `a`). Pinned only for a stream nothing was written to: a stream that holds bytes is refused.
pub(super) fn attribute_write_to_outstream(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let h = recv(r)?;
    let text = serialize(&h)?;
    crate::stream::write_raw_bytes_fresh(&mut a[0], text.as_bytes())?;
    Ok(Value::Boolean(true))
}

/// `WriteTo([XmlWriteOptions,] OutStream)` of a declaration: an unassigned one raises `NotInitialized` naming `XmlNode`
/// (P104WriteDecl.al:340 `uninit3`, `uninit4`). What an assigned one writes is one byte longer than its markup
/// (P104WriteDecl.al:219 `v1_m1_len` 56 for 55 characters) and that byte is not pinned.
pub(super) fn declaration_write_to_outstream(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> NResult {
    let _ = recv_or_uninit(r, "XmlNode")?;
    unsupported("XmlDeclaration.WriteTo(OutStream): the byte after the markup is not pinned")
}

/// `WriteTo([XmlWriteOptions,] OutStream)`: UTF-8 with a byte order mark, whatever the encoding of the
/// stream, the declaration saying `utf-8`; `PreserveWhitespace(true)` writes the document unindented
/// (xml/TestXmlDocumentWriteToOutStreamContracts.al). A document without a `utf-8` declaration is written as
/// [`outstream_declaration`] says; another shape is refused.
pub(super) fn write_to_outstream(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let h = recv(r)?;
    let (options, stream) = match a.len() {
        1 => (None, 0),
        _ => (Some(&a[0]), 1),
    };
    let indent = match options {
        None => true,
        Some(Value::Library(_, cell)) => match &*cell.borrow() {
            LibraryValue::XmlWriteOptions { unindented, .. } => !unindented,
            LibraryValue::Empty => true,
            _ => return unsupported("WriteTo with something that is not an XmlWriteOptions"),
        },
        Some(_) => return unsupported("WriteTo with something that is not an XmlWriteOptions"),
    };
    let text = serialize_with(&h, indent)?;
    let is_doc = with(&h, |d| matches!(d, XData::Document { .. }));
    let mut utf16 = false;
    let text = if is_doc && !text.starts_with("<?xml version=\"1.0\" encoding=\"utf-8\"") {
        match outstream_declaration(&h, options) {
            Some(OutDecl::AddUtf8) => text.replacen(
                "<?xml version=\"1.0\" encoding=\"utf-16\"?>",
                "<?xml version=\"1.0\" encoding=\"utf-8\"?>",
                1,
            ),
            Some(OutDecl::Keep) => text,
            Some(OutDecl::Utf16) => {
                utf16 = true;
                text
            }
            None => {
                return unsupported(
                    "WriteTo(OutStream) of a document that does not declare encoding utf-8 (not pinned)",
                );
            }
        }
    } else {
        text
    };
    let bytes = if utf16 {
        let mut b = vec![0xFF, 0xFE];
        for u in text.encode_utf16() {
            b.extend_from_slice(&u.to_le_bytes());
        }
        b
    } else {
        let mut b = vec![0xEF, 0xBB, 0xBF];
        b.extend_from_slice(text.as_bytes());
        b
    };
    // A text, CDATA, comment or processing instruction leaves the stream where it was, as JSON does: a second WriteTo writes over the
    // first one's bytes (xml/lt-xml-cdata-comment-pi-text/P90CDataRead.al:WriteTo_OutStream_TwiceIntoSameStream and its three siblings:
    // one copy, 16 bytes). Pinned for a stream nothing else was written to. A document and an element keep appending.
    // A document type is pinned only for a stream nothing was written to (P104WriteDocType.al:199): a second WriteTo
    // into the same stream, or one after other bytes, is refused.
    if with(&h, |d| matches!(d, XData::DocType { .. })) {
        crate::stream::write_raw_bytes_fresh(&mut a[stream], &bytes)?;
    } else if with(&h, |d| !matches!(d, XData::Document { .. } | XData::Element { .. })) {
        crate::stream::write_raw_bytes_rewinding(&mut a[stream], &bytes)?;
    } else {
        crate::stream::write_raw_bytes(&mut a[stream], &bytes)?;
    }
    Ok(Value::Boolean(true))
}

/// What `WriteTo(OutStream)` does with the declaration of a document that does not say `utf-8`.
pub(super) enum OutDecl {
    /// No declaration, or one without an encoding: the declaration says `utf-8`.
    AddUtf8,
    /// `encoding="UTF-8"`: kept as written.
    Keep,
    /// `encoding="utf-16"`: UTF-16 LE with its byte order mark.
    Utf16,
}

/// The pinned shapes, all of a document with one root element and nothing else:
/// - read by `ReadFrom` from text and written without XmlWriteOptions, with no declaration, a `version="1.0"` declaration
///   without encoding, `encoding="utf-16"` or `encoding="UTF-8"` (xml/lt-xml-attr-doc-http-members/P97DocWrite.al:334
///   QZ_WriteTo_OutStream_ReadFromText);
/// - without a declaration, also with XmlWriteOptions that `PreserveWhitespace(true)` set: the declaration says `utf-8`
///   (xml/XmlDocReadWrite.al:442-459 WriteTo_OutStream_Options);
/// - built in code (writer TV, corpus branch al-runtime-rs-staging-tv 8ae7ac0b, xml/lt-de-xml-leads/DeXmlWriteLeads.al
///   TV_WriteToOutStream_*): with a declaration of `XmlDeclaration.Create('1.0', ..)`, `UTF-8` with standalone `yes` or
///   `no` is kept as written (`yes` also with those options), and `utf-16` with standalone empty or `yes`, and an empty
///   encoding with standalone empty, are written in UTF-16 LE with its byte order mark, the declaration as written.
///
/// Anything else is not pinned.
pub(super) fn outstream_declaration(h: &Handle, options: Option<&Value>) -> Option<OutDecl> {
    let kids = children_of(h);
    if kids.len() != 1 || !with(&kids[0], |d| matches!(d, XData::Element { .. })) {
        return None;
    }
    // Options: none, or a value that `PreserveWhitespace(true)` set (it writes unindented).
    let preserve = match options {
        None => false,
        Some(Value::Library(_, cell))
            if matches!(
                &*cell.borrow(),
                LibraryValue::XmlWriteOptions {
                    preserve_whitespace: true,
                    unindented: true
                }
            ) =>
        {
            true
        }
        Some(_) => return None,
    };
    let decl = with(h, |d| match d {
        XData::Document { decl, .. } => decl.clone(),
        _ => None,
    });
    // No declaration, also with those options: xml/XmlDocReadWrite.al:442-459 WriteTo_OutStream_Options (read from text).
    let Some(decl) = decl else {
        return Some(OutDecl::AddUtf8);
    };
    with(&decl, |d| match d {
        XData::Declaration {
            version,
            encoding,
            standalone,
            explicit: false,
        } if version == "1.0" && standalone.is_empty() && !preserve => match encoding.as_str() {
            "" => Some(OutDecl::AddUtf8),
            "UTF-8" => Some(OutDecl::Keep),
            "utf-16" => Some(OutDecl::Utf16),
            _ => None,
        },
        XData::Declaration {
            version,
            encoding,
            standalone,
            explicit: true,
        } if version == "1.0" => match (encoding.as_str(), standalone.as_str(), preserve) {
            ("UTF-8", "yes", _) | ("UTF-8", "no", false) => {
                // Pinned by xml/lt-de-xml-leads/DeXmlWriteLeads.al TV_WriteToOutStream_BuiltWithoutDeclaration_Options, TV_WriteToOutStream_Utf8Declaration_OtherStreams, TV_WriteToOutStream_Utf16AndNoEncodingDeclarations, TV_XmlWriteOptions_AsContent_DocumentAdd (was assumption `xml-write-to-outstream-built`).
                Some(OutDecl::Keep)
            }
            ("utf-16", "" | "yes", false) | ("", "", false) => {
                // Pinned by xml/lt-de-xml-leads/DeXmlWriteLeads.al TV_WriteToOutStream_BuiltWithoutDeclaration_Options, TV_WriteToOutStream_Utf8Declaration_OtherStreams, TV_WriteToOutStream_Utf16AndNoEncodingDeclarations, TV_XmlWriteOptions_AsContent_DocumentAdd (was assumption `xml-write-to-outstream-built`).
                Some(OutDecl::Utf16)
            }
            _ => None,
        },
        _ => None,
    })
}

/// `XmlWriteOptions.PreserveWhitespace(Value)` (xml/TestXmlDocumentWriteToOutStreamContracts.al).
pub(super) fn write_options_preserve_whitespace(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let Some(Value::Library(_, cell)) = r else {
        return unsupported("PreserveWhitespace on something that is not an XmlWriteOptions");
    };
    let Some(Value::Boolean(on)) = a.first() else {
        return unsupported("PreserveWhitespace with something that is not a Boolean");
    };
    // `PreserveWhitespace(false)` on a new value writes unindented (xml/lt-xml-attr-doc-http-members/P97DocWrite.al:123
    // WriteTo_TextOptions_Variants `drop2`..`drop4`), while `true` and then `false` on one value writes indented again
    // (xml/XmlDocReadWrite.al:392 WriteTo_Options `preserve_false`, :418; xml/TestXmlDocumentWriteToOutStreamContracts.al:98;
    // P104Element.al:187): the setting follows only a change of the flag.
    let (was, unindented) = match &*cell.borrow() {
        LibraryValue::XmlWriteOptions {
            preserve_whitespace,
            unindented,
        } => (*preserve_whitespace, *unindented),
        _ => (false, true),
    };
    let unindented = if *on != was { *on } else { unindented };
    *cell.borrow_mut() = LibraryValue::XmlWriteOptions {
        preserve_whitespace: *on,
        unindented,
    };
    Ok(Value::Null)
}
