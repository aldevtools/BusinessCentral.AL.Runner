//! What `Media.ImportFile`, `MediaSet.ImportFile` and the `ImportStream` of both make of an Office document. Word documents:
//! corpus PR 1469 (merged, not in the pinned corpus yet: onprem streams/media/MediaDocxRezip.al, measured on Windows BC 27.5, 28.5
//! and 29.0, which agree except for the stored length). A Word package imported as a Word document is stored re-zipped: the media exists, its bytes
//! are not the file's and depend on the host (facts header :10-17), so the media carries the mark `stream::MEDIA_DOCX_REZIPPED`
//! and any look at its bytes is refused. Plain text imported as a Word document gives no media (:21-22). A four-argument
//! `ImportStream` with a blank MIME type reads a `.xlsx` or `.docx` file name as an Office document (`office_header_check`).
//!
//! What the ImportFile and ImportStream of Media and MediaSet make of an Office document: the
//! measured Word package re-zipped (marked `MEDIA_DOCX_REZIPPED`, its bytes refused), no media for
//! plain text (corpus PR 1469), and the Office header error of a four-argument import named `.xlsx`
//! or `.docx`. Entry points: `media_import_file`, `set_import_file`, `office_header_check`,
//! `stream_import`, `is_measured_package`.

use crate::assumptions::{Assumption, assume};
use crate::native::Signal;

/// The Word document MIME type (MediaDocxRezip.al:44; Y48TenantMediaRows.al:626 'attach H').
pub(crate) const DOCX_MIME: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.document";
/// The Tenant Media MIME type of a `.dotx` file (MediaDocxRezip.al:830).
const DOTX_MIME: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.template";
/// The Tenant Media MIME type of a `.docm` file (MediaDocxRezip.al:866).
const DOCM_MIME: &str = "application/vnd.ms-word.document.macroEnabled.12";
/// The Excel workbook MIME type of the four-argument import of a `.xlsx` (Y48TenantMediaRows.al:618 'attach D').
const XLSX_MIME: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";
/// The package every cell of MediaDocxRezip.al imports: the 911-byte .docx of MediaFiles.al (MediaDocxRezip.al:46-57 DocxBase64; its
/// parts :383). Only these bytes are measured: a zip Data Compression builds of the same parts is stored as it is (facts header :18).
const MEASURED: &[u8] = include_bytes!("media_word_package.docx");

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

/// The measured package, byte for byte (`MEASURED`).
pub(crate) fn is_measured_package(b: &[u8]) -> bool {
    b == MEASURED
}

/// A four-argument import (blank description and MIME type) named `.xlsx` or `.docx` reads the content as an Office document:
/// text content raises MediaUnsupportedFormat and no media is made (streams/media/Y48TenantMediaRows.al:601
/// TM_RowOfFourArgImport_MoreNames, :618 `attach D`, :626 `attach H`, writer Y57). Other content under those names imports as before.
pub(crate) fn office_header_check(bytes: &[u8], description: &str, mime: &str, file_name: &str) -> Result<(), Signal> {
    if file_name.is_empty() || !description.is_empty() || !mime.is_empty() {
        return Ok(());
    }
    let office = match file_name
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("xlsx") => XLSX_MIME,
        Some("docx") => DOCX_MIME,
        _ => return Ok(()),
    };
    // Other content imports as before this check.
    if bytes.is_empty()
        || !bytes
            .iter()
            .all(|b| b.is_ascii_graphic() || b.is_ascii_whitespace() || *b == b' ')
    {
        return Ok(());
    }
    crate::native::raise(
        format!("Microsoft Office document cannot be imported. File header is corrupt (MIME type: {office}."),
        "MediaUnsupportedFormat",
    )
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
    // MIME type, width and height 0), stored re-zipped on every major (MediaDocxRezip.al:392 Docx_MediaImportFile_Stored, :754
    // _UpperCaseExtension). The mark only refuses; the import answers what main answered.
    if ext.eq_ignore_ascii_case("docx") && bytes.starts_with(b"PK\x03\x04") {
        return Some(FileImport::Rezipped(DOCX_MIME));
    }
    let template = match ext {
        "dotx" => Some(DOTX_MIME),
        "docm" => Some(DOCM_MIME),
        _ => None,
    };
    if let Some(mime) = template.filter(|_| empty && is_measured_package(bytes)) {
        // MediaDocxRezip.al:820 Docx_MediaImportFile_DotxExtension, :856 _DocmExtension (the measured package into a new row).
        assume(Assumption::MediaWordPackageRezipped);
        return Some(FileImport::Rezipped(mime));
    }
    if ext == "docx" && empty && is_plain_text(bytes) {
        // MediaDocxRezip.al:789 Docx_MediaImportFile_NotAPackage.
        assume(Assumption::MediaWordImportNotAPackage);
        return Some(FileImport::Nothing);
    }
    None
}

/// `MediaSet.ImportFile` of the file `name` into a MediaSet field whose set id is null when `null_set` (a field that was never
/// filled); `None` leaves the file to the other rules of `media::set_import_file`.
pub(crate) fn set_import_file(name: &str, bytes: &[u8], null_set: bool) -> Option<&'static str> {
    let ext = name.rsplit_once('.').map(|(_, e)| e).unwrap_or_default();
    // MediaDocxRezip.al:995 Docx_MediaSetImportFile_Stored, the measured package into a new row: count 1, the Word MIME type, stored
    // re-zipped.
    (ext == "docx" && null_set && is_measured_package(bytes)).then(|| {
        assume(Assumption::MediaWordPackageRezipped);
        DOCX_MIME
    })
}

/// The `ImportStream` of `bytes` with `mime`, after `office_header_check` (which reads a blank MIME type only, this function a
/// Word one only), with a fourth argument (a file name) when `file_name_given`, into an empty Media
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
    // A Word MIME type re-zips (facts header :10-11; MediaDocxRezip.al:593 Docx_MediaImportStream_DocxMime, :635 _DocxFileName,
    // :916 _UpperCaseDocxMime, :1034 Docx_MediaSetImportStream_Stored). The import answers what main answered; the mark only refuses.
    if bytes.starts_with(b"PK\x03\x04") {
        return Ok(Some(StreamImport::Rezipped));
    }
    if mime == DOCX_MIME && !file_name_given && empty_media && is_plain_text(bytes) {
        // MediaDocxRezip.al:959 Docx_MediaImportStream_NotAPackageDocxMime.
        assume(Assumption::MediaWordImportNotAPackage);
        return Ok(Some(StreamImport::Nothing));
    }
    Err(Signal::Unsupported(
        "ImportStream of content that is no zip package with a Word MIME type: only plain text with the lower-case Word document \
         MIME type, no file name, into an empty Media is measured (it gives no media); what BC makes of other content is not"
            .into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::{MEASURED as SOURCE, is_measured_package};

    #[test]
    fn only_the_measured_bytes_are_the_measured_package() {
        assert!(is_measured_package(SOURCE));
        // Same parts, names, sizes and CRC-32, other compressed bytes: a zip Data Compression builds of these parts is stored as it is
        // (MediaDocxRezip.al:18, :386 'rebuilt is the file' No), so it is not the measured package.
        let mut other = SOURCE.to_vec();
        other[60] ^= 1;
        assert!(!is_measured_package(&other));
        assert!(!is_measured_package(&SOURCE[..910]));
    }
}
