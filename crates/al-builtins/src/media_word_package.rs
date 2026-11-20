//! What `Media.ImportFile`, `MediaSet.ImportFile` and the `ImportStream` of both make of a Word document (corpus PR 1469, merged,
//! not in the pinned corpus yet: onprem streams/media/MediaDocxRezip.al, measured on Windows BC 27.5, 28.5 and 29.0, which agree
//! except for the stored length). A Word package imported as a Word document is stored re-zipped: the media exists, its bytes
//! are not the file's and depend on the host (facts header :10-17), so the media carries the mark `stream::MEDIA_DOCX_REZIPPED`
//! and any look at its bytes is refused. Plain text imported as a Word document gives no media (:21-22).

use crate::assumptions::{Assumption, assume};
use crate::native::Signal;

/// The Word document MIME type (MediaDocxRezip.al:44).
pub(crate) const DOCX_MIME: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.document";
/// The Tenant Media MIME type of a `.dotx` file (MediaDocxRezip.al:830).
const DOTX_MIME: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.template";
/// The Tenant Media MIME type of a `.docm` file (MediaDocxRezip.al:866).
const DOCM_MIME: &str = "application/vnd.ms-word.document.macroEnabled.12";

/// What an `ImportFile` of a Word document gives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileImport {
    /// A media of this MIME type whose bytes are a re-zipped package: it carries `MEDIA_DOCX_REZIPPED`.
    Rezipped(&'static str),
    /// No media: the call returns the null Guid and the field keeps no value.
    Nothing,
}

/// What an `ImportStream` with a Word MIME type gives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StreamImport {
    /// A media with the MIME type as given whose bytes are a re-zipped package: it carries `MEDIA_DOCX_REZIPPED`.
    Rezipped,
    /// No media: the call returns the null Guid and the field keeps no value.
    Nothing,
}

/// A zip whose first local file header names `[Content_Types].xml`: the package every cell of MediaDocxRezip.al imports
/// (its `stored parts` start with that entry, :668).
pub(crate) fn is_package(b: &[u8]) -> bool {
    const FIRST: &[u8] = b"[Content_Types].xml";
    b.starts_with(b"PK\x03\x04")
        && b.len() >= 30 + FIRST.len()
        && usize::from(u16::from_le_bytes([b[26], b[27]])) == FIRST.len()
        && &b[30..30 + FIRST.len()] == FIRST
}

/// Printable ASCII and not empty: the measured content that is no package is the text `hello docx` (MediaDocxRezip.al:800, :971).
fn is_plain_text(b: &[u8]) -> bool {
    !b.is_empty() && b.iter().all(|c| (0x20..=0x7E).contains(c))
}

/// `Media.ImportFile` of the file `name` into a field that holds no media when `empty`; `None` leaves the file to the other rules
/// of `media::import_file`.
pub(crate) fn media_import_file(name: &str, bytes: &[u8], empty: bool) -> Option<FileImport> {
    let ext = name.rsplit_once('.').map(|(_, e)| e).unwrap_or_default();
    // A .docx in any letter case holding a zip: the import MediaFiles.al:470-526 RD_Media_ImportFile_Docx pins (HasValue Yes, the Word
    // MIME type, width and height 0), stored re-zipped on every major (MediaDocxRezip.al:393 Docx_MediaImportFile_Stored, :755
    // _UpperCaseExtension). The mark only refuses; the import answers what main answered.
    if ext.eq_ignore_ascii_case("docx") && bytes.starts_with(b"PK\x03\x04") {
        return Some(FileImport::Rezipped(DOCX_MIME));
    }
    let template = match ext {
        "dotx" => Some(DOTX_MIME),
        "docm" => Some(DOCM_MIME),
        _ => None,
    };
    if let Some(mime) = template.filter(|_| empty && is_package(bytes)) {
        // MediaDocxRezip.al:821 Docx_MediaImportFile_DotxExtension, :857 _DocmExtension.
        assume(Assumption::MediaWordPackageRezipped);
        return Some(FileImport::Rezipped(mime));
    }
    if ext == "docx" && empty && is_plain_text(bytes) {
        // MediaDocxRezip.al:790 Docx_MediaImportFile_NotAPackage.
        assume(Assumption::MediaWordImportNotAPackage);
        return Some(FileImport::Nothing);
    }
    None
}

/// `MediaSet.ImportFile` of the file `name` into a set of `held` members; `None` leaves the file to the other rules of
/// `media::set_import_file`.
pub(crate) fn set_import_file(name: &str, bytes: &[u8], held: usize) -> Option<&'static str> {
    let ext = name.rsplit_once('.').map(|(_, e)| e).unwrap_or_default();
    // MediaDocxRezip.al:996 Docx_MediaSetImportFile_Stored: count 1, the Word MIME type, stored re-zipped.
    (ext == "docx" && held == 0 && is_package(bytes)).then(|| {
        assume(Assumption::MediaWordPackageRezipped);
        DOCX_MIME
    })
}

/// The `ImportStream` of `bytes` with `mime`, with a fourth argument (a file name) when `file_name_given`, into an empty Media
/// field when `empty_media`. `Ok(None)`: not a Word MIME type, the import keeps the rules of `media::import_object`.
pub(crate) fn stream_import(
    bytes: &[u8],
    mime: &str,
    file_name_given: bool,
    empty_media: bool,
) -> Result<Option<StreamImport>, Signal> {
    if ![DOCX_MIME, DOTX_MIME, DOCM_MIME]
        .iter()
        .any(|m| m.eq_ignore_ascii_case(mime))
    {
        return Ok(None);
    }
    // A Word MIME type re-zips (facts header :10-11; MediaDocxRezip.al:594 Docx_MediaImportStream_DocxMime, :636 _DocxFileName,
    // :917 _UpperCaseDocxMime, :1035 Docx_MediaSetImportStream_Stored). The import answers what main answered; the mark only refuses.
    if bytes.starts_with(b"PK\x03\x04") {
        return Ok(Some(StreamImport::Rezipped));
    }
    if mime == DOCX_MIME && !file_name_given && empty_media && is_plain_text(bytes) {
        // MediaDocxRezip.al:960 Docx_MediaImportStream_NotAPackageDocxMime.
        assume(Assumption::MediaWordImportNotAPackage);
        return Ok(Some(StreamImport::Nothing));
    }
    Err(Signal::Unsupported(
        "ImportStream of content that is no zip package with a Word MIME type: only plain text with the lower-case Word document \
         MIME type, no file name, into an empty Media is measured (it gives no media); what BC makes of other content is not"
            .into(),
    ))
}
