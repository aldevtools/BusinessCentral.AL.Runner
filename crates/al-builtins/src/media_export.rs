//! Which media `Media.ExportStream` gives back byte for byte.

use crate::media::{MediaObject, StreamCall};

/// Content that is no picture comes back unchanged from ExportStream: UTF-8 text (ASCII and non-ASCII) and binary bytes,
/// imported with no MIME type or with `text/plain`, with or without a file name (.txt, .pdf, none), and the export of an
/// export imported again (streams/media/Y44MediaExportStream.al:131-133 DocAttach_TextTxt_ExportStream, :160-164
/// _RoundTrip, :197-204 Media_TextWithMime_ExportStream, :221-230 DocAttach_NonAscii_And_NoExtension, :262-264
/// Media_Binary_ExportStream; the two argument import of text: streams/StTMedia.al TR_MediaImportStream_TextContent_ExportStream).
/// A picture is re-encoded (Y44MediaExportStream.al:245-250 DocAttach_Png_ExportStream gives the 99 bytes of one 68-byte PNG only),
/// so anything with a picture signature, another MIME type, a file column, empty content or a pending import mark keeps the
/// refusal of a look at what ExportStream wrote.
pub(crate) fn exports_unchanged(o: &MediaObject) -> bool {
    matches!(o.mime.as_str(), "" | "text/plain")
        && o.pending.is_none()
        && o.file_column.is_none()
        && !o.bytes.is_empty()
        && !has_picture_signature(&o.bytes)
}

/// The measured Word package imported by a three-argument `Media.ImportStream` as `application/zip` into a Media field that held no
/// media comes back unchanged (corpus PR 1469, merged, not pinned yet: MediaDocxRezip.al:677 Docx_MediaImportStream_ZipMime, the call
/// at :690, 'stored is the file' and 'export is the stored content' Yes on Windows BC 27.5, 28.5 and 29.0; assumption
/// `media-zip-mime-export-kept`, which the caller marks). Other content, another call or a field that held a media keeps the refusal.
pub(crate) fn zip_package_exports_unchanged(o: &MediaObject) -> bool {
    o.mime == "application/zip"
        && o.pending.is_none()
        && o.stream_call
            == Some(StreamCall {
                args: 3,
                into_empty_media: true,
            })
        && crate::media_word_package::is_measured_package(&o.bytes)
}

/// PNG, JPEG, GIF, BMP, TIFF, ICO/CUR and WebP signatures.
fn has_picture_signature(b: &[u8]) -> bool {
    const SIGNATURES: [&[u8]; 9] = [
        b"\x89PNG",
        b"\xFF\xD8\xFF",
        b"GIF8",
        b"BM",
        b"II*\0",
        b"MM\0*",
        b"\0\0\x01\0",
        b"\0\0\x02\0",
        b"RIFF",
    ];
    SIGNATURES.iter().any(|s| b.starts_with(s))
}
