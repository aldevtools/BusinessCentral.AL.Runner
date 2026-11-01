//! The Description, Mime Type and File Name of the Tenant Media row of an imported media.

use al_builtins::media::MediaObject;

/// The three columns of `o`'s Tenant Media row, or why they are not known.
/// - A two or three argument import of content that is no PNG, with no MIME type: `application/octet-stream`
///   (media/lt-y48-tenant-media/Y48TenantMediaRows.al:399 `tm row before` of a three argument import keeps its type; :484 `item 1
///   tm row` and :333 TM_DeleteRowOfHeldMedia, two argument imports into a MediaSet and a Media field). A PNG's type is
///   `image/png` (media/PXMediaFileName.al), set by the import.
/// - A four argument import with a blank description and MIME type (Base Application's Document Attachment.ImportAttachment):
///   Description blank, File Name the given name, Mime Type from its extension: `.txt` text/plain, none
///   application/octet-stream, `.PDF` application/pdf (Y48TenantMediaRows.al:312-329 TM_RowOfFourArgImport, BC 27.5, 28.5,
///   29.0 and a SaaS sandbox). Another extension, or a description or MIME type next to the file name, is not pinned.
pub(super) fn columns(o: &MediaObject) -> Result<(String, String, String), String> {
    const OCTET: &str = "application/octet-stream";
    if o.file_name.is_empty() {
        let mime = if o.mime.is_empty() { OCTET } else { &o.mime };
        return Ok((o.description.clone(), mime.to_string(), String::new()));
    }
    let not_pinned = || {
        "a Tenant Media row of a media imported with a file name: its Description and Mime Type are pinned only for a blank \
         description and MIME type and a file name ending in .txt, .pdf or no extension"
            .to_string()
    };
    if !o.description.is_empty() || !o.mime.is_empty() {
        return Err(not_pinned());
    }
    let extension = o.file_name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase());
    let mime = match extension.as_deref() {
        None => OCTET,
        Some("txt") => "text/plain",
        Some("pdf") => "application/pdf",
        Some(_) => return Err(not_pinned()),
    };
    Ok((String::new(), mime.to_string(), o.file_name.clone()))
}
