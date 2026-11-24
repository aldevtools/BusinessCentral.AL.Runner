//! The decoder behind `GZipStream.CopyTo` (Data Compression's `GZipDecompress`): the bytes a valid gzip stream
//! gives, BC's error for the exact corrupt streams the corpus measured, and a refusal for every other corrupt stream.
//!
//! Pinned by tests/al-language/types/PZGzip.al (round trips, an empty stream, text that is not gzip, a stream cut in
//! half) and tests/al-language/streams/blob-tempblob/GZipShortAndJoined.al (the streams of 1 to 10 bytes it lists, a
//! valid header followed by twenty bytes 255 or 65, a 12-byte stream with method 99, a flipped byte in the deflate
//! data and in the size field of an 83-byte member, two members joined, a member followed by bytes that are not a
//! member). Every corrupt stream BC answered there raises the same error, with nothing written to the target stream.
//!
//! what `GZipStream.CopyTo` decodes and raises (GZipShortAndJoined.al): the measured whole streams
//! (the short ones and the corrupt ones that raise InvalidData), CRC32 and size checks, a refusal for
//! every other corrupt stream. Entry points: `inflate`.

use al_builtins::native::{Signal, unsupported};
use flate2::{Crc, Decompress, FlushDecompress, Status};

const MAGIC: [u8; 2] = [0x1f, 0x8b];
/// The gzip header without its optional fields.
const HEADER_LEN: usize = 10;
/// Compression method 8 (deflate), the only one BC decodes in the corpus.
const DEFLATE: u8 = 8;
/// The plain text of GZipShortAndJoined.al:253, whose packed stream with a flipped byte (:262, :272) raises with
/// nothing written: the only decoded length a CRC32 or size mismatch is measured at.
const CORRUPT_MEMBER_TEXT_LEN: usize =
    "corrupt me after the header corrupt me after the header corrupt me after the header".len();

/// The plain gzip header of GZipShortAndJoined.al:188-197 followed by twenty bytes `fill` (:198-199, :214-215).
const fn header_then_twenty(fill: u8) -> [u8; HEADER_LEN + 20] {
    let header = [31, 139, 8, 0, 0, 0, 0, 0, 0, 3];
    let mut stream = [fill; HEADER_LEN + 20];
    let mut i = 0;
    while i < HEADER_LEN {
        stream[i] = header[i];
        i += 1;
    }
    stream
}
const HEADER_THEN_FF: [u8; HEADER_LEN + 20] = header_then_twenty(255);
const HEADER_THEN_TEXT: [u8; HEADER_LEN + 20] = header_then_twenty(65);

/// The whole streams GZipShortAndJoined.al:115-237 decompresses that the member decoder below does not answer:
/// the ones shorter than a gzip header, and the measured corrupt ones. `true`: BC raises; `false`: an empty result.
const MEASURED_STREAMS: [(&[u8], bool); 9] = [
    (&[31], false),                                    // :121-123, :170
    (&[65], false),                                    // :126-128, :171
    (&[31, 139], false),                               // :131-134, :172
    (&[31, 139, 8, 0, 0], false),                      // :137-143, :173
    (&[1, 2, 3, 4, 5], true),                          // :146-152, :174
    (&[31, 139, 8, 0, 0, 0, 0, 0, 0], false),          // :155-165, :175
    (&HEADER_THEN_FF, true),                           // :188-201, :234
    (&HEADER_THEN_TEXT, true),                         // :204-217, :235
    (&[31, 139, 99, 0, 0, 0, 0, 0, 0, 3, 1, 2], true), // :220-233, :236
];

/// The error of `GZipStream.CopyTo` over data it cannot decode (PZGzip.al:201-205, GZipShortAndJoined.al:174, :234-236,
/// :279-280).
fn invalid_data<T>() -> Result<T, Signal> {
    al_builtins::native::raise(
        "A call to System.IO.Compression.GZipStream.CopyTo failed with this message: The archive entry was compressed using an unsupported compression method.",
        "DotNetInvoke:InvalidData",
    )
}

/// The bytes of a gzip stream as `GZipStream.CopyTo` gives them.
pub(super) fn inflate(bytes: &[u8]) -> Result<Vec<u8>, Signal> {
    // An empty stream gives an empty result (PZGzip.al:224, :238).
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    match MEASURED_STREAMS.iter().find(|(s, _)| *s == bytes) {
        Some((_, true)) => return invalid_data(),
        Some((_, false)) => return Ok(Vec::new()),
        None if bytes.len() < HEADER_LEN => {
            return unsupported(
                "GZipStream decompression of fewer than 10 bytes (the header is 10): BC's answer is not pinned",
            );
        }
        None => {}
    }
    if bytes[..2] != MAGIC {
        return invalid_data();
    }
    let mut out = Vec::new();
    let mut rest = bytes;
    // One member after the other (GZipShortAndJoined.al:285-308); bytes after a member that do not start another one
    // are left alone (:312-343).
    while rest.len() >= HEADER_LEN && rest[..2] == MAGIC {
        let first = rest.len() == bytes.len();
        match member(rest, first, &mut out)? {
            Some(next) => rest = next,
            None => break,
        }
    }
    Ok(out)
}

/// Decodes the member at the start of `bytes` onto `out`; the bytes after it, or `None` when the stream ends inside it.
/// `first`: the member starts the stream.
fn member<'a>(bytes: &'a [u8], first: bool, out: &mut Vec<u8>) -> Result<Option<&'a [u8]>, Signal> {
    let (method, flags) = (bytes[2], bytes[3]);
    let plain_first = first && flags == 0;
    if method != DEFLATE {
        return unsupported(format!(
            "GZipStream decompression of a member with compression method {method}: BC's answer is not pinned"
        ));
    }
    let Some(data) = member_data(bytes) else {
        return Ok(None);
    };
    let start = out.len();
    let Some(used) = deflate(data, out)? else {
        // A stream cut inside the deflate data gives what was decoded before the cut (PZGzip.al:227-239).
        return Ok(None);
    };
    let trailer = &data[used..];
    match trailer_matches(trailer, &out[start..]) {
        Trailer::Matches => Ok(Some(&trailer[8..])),
        Trailer::Cut => Ok(None),
        Trailer::Differs if plain_first && trailer.len() == 8 && out.len() == CORRUPT_MEMBER_TEXT_LEN => {
            // GZipShortAndJoined.al:241-281: a CRC32 mismatch (flip_byte_14) and a size mismatch (flip_last_crc_byte).
            invalid_data()
        }
        Trailer::Differs => unsupported(
            "GZipStream decompression of a member whose CRC32 or size does not match: what BC writes before its error is not pinned",
        ),
    }
}

/// Decodes the raw deflate data of one member onto `out`: how many bytes of `data` it used, or `None` when the data
/// ends before the member does.
fn deflate(data: &[u8], out: &mut Vec<u8>) -> Result<Option<usize>, Signal> {
    let mut d = Decompress::new(false);
    loop {
        out.reserve(8192);
        let (in_before, out_before) = (d.total_in(), d.total_out());
        let status = d
            .decompress_vec(&data[d.total_in() as usize..], out, FlushDecompress::None)
            .map_err(|_| {
                Signal::unsupported("GZipStream decompression of corrupt data: the error BC raises is not pinned")
            })?;
        if status == Status::StreamEnd {
            return Ok(Some(d.total_in() as usize));
        }
        if d.total_in() == in_before && d.total_out() == out_before {
            return Ok(None);
        }
    }
}

enum Trailer {
    Matches,
    Differs,
    /// Fewer than 8 bytes, with a CRC32 that matches when its 4 bytes are there: the stream ends as a cut one does.
    Cut,
}

/// Compares the CRC32 and size fields after a member's deflate data with what the member decoded to.
fn trailer_matches(trailer: &[u8], decoded: &[u8]) -> Trailer {
    let mut crc = Crc::new();
    crc.update(decoded);
    let expected = [crc.sum().to_le_bytes(), (decoded.len() as u32).to_le_bytes()].concat();
    let n = trailer.len().min(8);
    let present = if n >= 4 { n } else { 0 };
    match (trailer[..present] == expected[..present], n) {
        (true, 8) => Trailer::Matches,
        (true, _) => Trailer::Cut,
        (false, _) => Trailer::Differs,
    }
}

/// The deflate data of the gzip member at the start of `bytes` (after the header and its optional fields).
fn member_data(bytes: &[u8]) -> Option<&[u8]> {
    let flags = bytes[3];
    let mut at = HEADER_LEN;
    if flags & 0x04 != 0 {
        let n = u16::from_le_bytes([*bytes.get(at)?, *bytes.get(at + 1)?]) as usize;
        at += 2 + n;
    }
    for bit in [0x08, 0x10] {
        if flags & bit != 0 {
            at += bytes.get(at..)?.iter().position(|b| *b == 0)? + 1;
        }
    }
    if flags & 0x02 != 0 {
        at += 2;
    }
    bytes.get(at..)
}

#[cfg(test)]
#[path = "gzip_inflate_tests.rs"]
mod tests;
