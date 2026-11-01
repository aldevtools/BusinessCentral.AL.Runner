//! `HttpContent`: writing and reading the body.

use super::*;

// ------------------------------------------------------------------------------------------------ content

pub(super) fn write_from(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let t = text(a, 0)?;
    with_content(r, |s| s.write_body(t.into_bytes())).map(|()| Value::Null)
}

/// `WriteFrom(SecretText)`: the body is the secret's text and the content is secret: `IsSecretContent` is true, `ReadAs(SecretText)`
/// gives it back, `ReadAs(Text)` answers false, and the headers no longer list the Content-Type
/// (xml/lt-xml-attr-doc-http-members/P97ContentSecret.al:83-101, :103-117, :119-150, :152-168; al-runner issue 6161). A later
/// WriteFrom of text or a stream, and Clear, make it plain again (:119-150, :170-190).
pub(super) fn write_from_secret(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let (t, _) = text_or_secret(a, 0)?;
    with_content(r, |s| {
        s.write_body(t.into_bytes());
        s.secret = true;
        let mut bag = s.headers.borrow_mut();
        bag.items.retain(|(n, _)| !n.eq_ignore_ascii_case("Content-Type"));
        bag.unnamed_secret_header = true;
    })
    .map(|()| Value::Null)
}

/// `IsSecretContent()`: the last WriteFrom took a SecretText (P97ContentSecret.al:83-150; network/lt-uf-http-clear/UfHttpClear.al:102
/// after a Clear and a WriteFrom of text).
pub(super) fn is_secret_content(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_content(r, |s| Value::Boolean(s.secret))
}

/// `ReadAs(var SecretText)` with its result used answers true and gives the body as a secret, of a secret or a plain content, a
/// new one included (P97ContentSecret.al:21-28 with :83-150, :215-234). The statement form and the content of a request that has
/// none are not pinned.
pub(super) fn read_as_secret(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    if !c.ret_used() {
        return unsupported("HttpContent.ReadAs(SecretText) as a statement (not pinned)");
    }
    let (data, uninit) = with_content(r, |s| (s.data.clone(), s.uninitialized))?;
    if uninit {
        return unsupported("HttpContent.ReadAs(SecretText) on the content of a request that has none (not pinned)");
    }
    let Ok(t) = String::from_utf8(data) else {
        return unsupported("HttpContent.ReadAs(SecretText) of a body that is not UTF-8 (not pinned)");
    };
    a[0] = lib("SecretText", LibraryValue::Text(t));
    Ok(Value::Boolean(true))
}

/// `WriteFrom(InStream)` takes the bytes the stream has left. The source then is not at its end
/// (network/lt-db-http-leads/DbHttpContentLeads.al:310); any other use of it is not pinned and refused (`stream::hand_to_content`).
pub(super) fn write_from_stream(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let bytes = crate::stream::hand_to_content(&mut a[0])?;
    with_content(r, |s| {
        s.write_body(bytes);
        s.from_stream = true;
    })
    .map(|()| Value::Null)
}

/// `ReadAs(var Text)` gives the body back, UTF-8 decoded; a content nobody wrote to reads as an empty text and
/// answers true (misc/W4XmlJsonHttp.al:ReadAs_FreshContent). On the content a request has when none was set, three shapes are
/// pinned:
/// - `Req.Content.ReadAs(Txt)` with its result used answers false and leaves the text (network/lt-http/HttpRequest.al:220,
///   FreshRequestContent);
/// - `Req.Content.ReadAs(Txt)` as a statement raises `NotInitialized` (misc/lt-probes-needed-5/PnE12HttpContent.al:205,
///   RequestWithoutContent, under asserterror);
/// - a copy of it in a variable (`Content := Req.Content`), called as a statement, raises `NotInitialized`
///   (misc/W4XmlJsonHttp.al:342-343, RequestWithoutContent_ReadAs, through a TryFunction).
///
/// A copy with the result used is refused. Probe wanted (docs/probes-needed.md).
pub(super) fn read_as(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let mut r = r;
    // A secret content does not read as text: with the result used the call answers false and leaves the text
    // (P97ContentSecret.al:30-38 with :83-101, :119-168). The statement form is not pinned.
    if with_content(r.as_deref_mut(), |s| s.secret)? {
        if c.ret_used() {
            return Ok(Value::Boolean(false));
        }
        return unsupported(
            "HttpContent.ReadAs(Text) as a statement on a content written from a SecretText (not pinned)",
        );
    }
    let (data, uninit, copied, charset, taken) = with_content(r, |s| {
        let charset = s
            .headers
            .borrow()
            .items
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case("Content-Type"))
            .and_then(|(_, v)| v.first().cloned())
            .and_then(|t| {
                t.split(';').skip(1).find_map(|p| {
                    let (k, v) = p.split_once('=')?;
                    k.trim()
                        .eq_ignore_ascii_case("charset")
                        .then(|| v.trim().trim_matches('"').to_ascii_lowercase())
                })
            });
        // Reading the body makes the content buffer it, and its headers then list a Content-Length of the body's length when they
        // hold none (misc/lt-probes-needed-5/PnE12HttpContent.al:WriteFromText_ThenReadAndHeaders reads `5` after WriteFrom('hello')
        // and ReadAs; network/lt-ay-http/AyContentLength.al:ContentLength_* and PY_*: none after ReadAs(InStream), an empty body reads
        // `0`). A Content-Length the headers already hold stays as it is: one an earlier read put there (PY_ContentLength_LongerBodyAfterRead
        // `second_read` Yes[5]) and one added by hand (AyContentLength.al:324-343 SE_ContentLength_AddedByHandBeforeRead `Yes[3]`).
        if !s.uninitialized {
            let mut bag = s.headers.borrow_mut();
            if find(&bag.items, "Content-Length").is_none() {
                bag.items
                    .push(("Content-Length".into(), vec![s.data.len().to_string()]));
            }
        }
        let taken = s.stream_taken && (s.copied || s.from_stream);
        (s.data.clone(), s.uninitialized, s.copied, charset, taken)
    })?;
    if uninit {
        return match (copied, c.ret_used()) {
            (false, true) => Ok(Value::Boolean(false)),
            (_, false) => raise(
                "Microsoft.Dynamics.Nav.Runtime.NavHttpContent variable not initialized.",
                "NotInitialized",
            ),
            (true, true) => unsupported(
                "HttpContent.ReadAs with its result used, on a copy of the content of a request that has none (not pinned)",
            ),
        };
    }
    // Once the body was handed out as a stream, whether that stream was read to its end, in part or not at all: a copy of a
    // content (network/lt-db-http-leads/DbHttpContentLeads.al:146, :430-480 QG_RespContentCopy_*) and a content written from a
    // stream (:336, :552-600 QG_StreamContent_*) read an empty text; the response's own content (:113, :384-428 QG_RespContent_*)
    // and a variable written from a text (xml/lt-xml-attr-doc-http-members/P97ContentRead.al:89, DbHttpContentLeads.al:506-550
    // QG_TextContent_*) still read the body.
    if taken {
        a[0] = Value::Text(String::new().into());
        return Ok(Value::Boolean(true));
    }
    // The body is read with the charset of the Content-Type header (lt-http-header-rules/P87ContentType.al:ReadAsIgnoresCharsetHeader,
    // whatever its name says: iso-8859-1 reads each byte as a character, utf-16 reads little-endian units, a lone last byte is U+FFFD;
    // no charset, or another type, is UTF-8). A charset not measured stays refused.
    let decoded = match charset.as_deref() {
        None | Some("utf-8") => String::from_utf8_lossy(&data).into_owned(),
        Some("iso-8859-1" | "latin1") => data.iter().map(|b| char::from(*b)).collect(),
        Some("utf-16" | "utf-16le") => {
            let mut units: Vec<u16> = data.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
            let mut text = String::from_utf16_lossy(&units);
            if data.len() % 2 == 1 {
                text.push('\u{FFFD}');
            }
            units.clear();
            text
        }
        Some(other) => {
            return unsupported(format!(
                "HttpContent.ReadAs(Text) with the charset {other}: not measured"
            ));
        }
    };
    a[0] = Value::Text(decoded.into());
    Ok(Value::Boolean(true))
}

/// `ReadAs(var InStream)` hands the body out once: a second read gets an empty stream
/// (network/lt-http/HttpContent.al:ReadAsInStreamTwice).
pub(super) fn read_as_stream(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let mut r = r;
    // On a secret content the stream is left not initialized (P97ContentSecret.al:40-51: its EOS raises NotInitialized); what the
    // call answers is not pinned.
    if with_content(r.as_deref_mut(), |s| s.secret)? {
        return unsupported(
            "HttpContent.ReadAs(InStream) on a content written from a SecretText (its result is not pinned)",
        );
    }
    let (data, uninit) = with_content(r, |s| {
        let out = if s.stream_taken { Vec::new() } else { s.data.clone() };
        s.stream_taken = true;
        (out, s.uninitialized)
    })?;
    // On the content of a request that has none, only ReadAs(Text) is pinned (see `read_as`). Probe wanted (docs/probes-needed.md).
    if uninit {
        return unsupported("HttpContent.ReadAs(InStream) on the content of a request that has none (not pinned)");
    }
    let stream = crate::stream::InState::snapshot(data, crate::stream::Encoding::Utf8);
    crate::stream::set_stream(&a[0], LibraryValue::InStream(stream))?;
    Ok(Value::Boolean(true))
}

/// `GetHeaders(var Headers)` hands out the content's own header list (network/TestHttpRequestMessage.al:39); the
/// uninitialized content of a request raises `NotInitialized` (network/lt-http/HttpRequest.al:FreshRequestContent).
pub(super) fn get_headers(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let (list, uninit) = with_content(r, |s| (s.headers.clone(), s.uninitialized))?;
    if uninit {
        return raise(
            "Microsoft.Dynamics.Nav.Runtime.NavHttpContent variable not initialized.",
            "NotInitialized",
        );
    }
    hand_out_headers(a.first(), &list)?;
    Ok(Value::Boolean(true))
}

/// `Clear` empties the body and gives the content its own fresh headers, the Content-Type among them
/// (network/lt-http/HttpContent.al:ClearContent).
pub(super) fn content_clear(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_content(r, |s| s.clear_body()).map(|()| Value::Null)
}

impl HttpContentState {
    /// A content with its own data and its own headers.
    pub fn deep_copy(&self) -> HttpContentState {
        let bag = self.headers.borrow();
        let copy = new_header_list(bag.kind);
        {
            let mut c = copy.borrow_mut();
            c.items = bag.items.clone();
            c.secret = bag.secret.clone();
            c.unnamed_secret_header = bag.unnamed_secret_header;
        }
        HttpContentState {
            data: self.data.clone(),
            headers: copy,
            uninitialized: self.uninitialized,
            stream_taken: self.stream_taken,
            written: self.written,
            copied: true,
            from_stream: self.from_stream,
            secret: self.secret,
        }
    }

    /// `Clear`: the content gets no body and a header collection of its own that holds the Content-Type; a header
    /// collection handed out before keeps what it held (network/lt-http/HttpContent.al:ClearContent,
    /// lt-http-header-rules/P87Content.al:ClearAfterAdd, ClearBeforeWrite, ClearFresh).
    fn clear_body(&mut self) {
        let headers = new_header_list(HeaderKind::Content);
        headers
            .borrow_mut()
            .items
            .push(("Content-Type".into(), vec!["text/plain; charset=utf-8".into()]));
        self.data = Vec::new();
        self.headers = headers;
        self.uninitialized = false;
        self.stream_taken = false;
        self.written = false;
        self.from_stream = false;
        self.secret = false;
    }

    /// `WriteFrom` of text or a stream: the body is replaced, the header collection stays the one every `HttpHeaders` handed out
    /// shows, with its Content-Type set to `text/plain; charset=utf-8` (put first when it was removed) and the other headers kept
    /// (lt-http-header-rules/P87Content.al:AddThenWrite_Text, WriteThenAdd_Text, RewriteKeepsHeader, P87ContentType.al:HeaderBeforeWrite,
    /// P87ContentStream.al, HttpWriteFromStream.al:P100_*). A Content-Length the headers already hold (see `read_as`) is not
    /// rewritten: it stays for an empty body (network/lt-ay-http/AyContentLength.al:ContentLength_SecondWriteFrom `empty`
    /// `Yes[13]`), for a body of the same length (P100_SecondWrite_ReplacesBody, 5 bytes twice) and for a longer, a shorter or a
    /// stream body (AyContentLength.al:PY_ContentLength_LongerBodyAfterRead, _ShorterBodyAfterRead, _InStreamAfterRead `Yes[5]`).
    /// Pinned for a value a read put there (AyContentLength.al PY_ContentLength_*) and for one added by hand, before or after a
    /// read, with or without a body before it (AyContentLength.al:366-460, Y2_ContentLength_AddedByHand*).
    fn write_body(&mut self, data: Vec<u8>) {
        {
            let mut bag = self.headers.borrow_mut();
            match bag
                .items
                .iter_mut()
                .find(|(n, _)| n.eq_ignore_ascii_case("Content-Type"))
            {
                Some((_, v)) => *v = vec!["text/plain; charset=utf-8".into()],
                None => bag
                    .items
                    .insert(0, ("Content-Type".into(), vec!["text/plain; charset=utf-8".into()])),
            }
        }
        self.data = data;
        self.uninitialized = false;
        self.stream_taken = false;
        self.written = true;
        self.from_stream = false;
        self.secret = false;
    }
}
