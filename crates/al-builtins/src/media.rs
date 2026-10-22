//! `Media` and `MediaSet` fields (corpus media/).
//!
//! A Media field holds the id of its media object; a MediaSet field holds the id of its set, and
//! the set holds the ids of its media objects. Both are stored as a Guid, null for an empty field
//! (media/TestMediaEmptyMediaId.al). The media objects and the sets live in the session's
//! `MediaStore`, the stand-in for the platform tables "Tenant Media" and "Tenant Media Set"
//! (ADR-013 gives those tables rows). The field only holds the id once the record is written,
//! so an import that was not followed by `Modify` is not seen by the next `Get`.

use crate::describe::Describe;
use crate::native::{Ctx, NativeFn, Signal};
use crate::value::{LibraryValue, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub type Id = [u8; 16];

/// One imported media object.
#[derive(Debug, Clone)]
pub struct MediaObject {
    pub bytes: Vec<u8>,
    pub mime: String,
    pub description: String,
    /// The file name of the four argument import (`ImportStream(Stream, Description, MimeType, FileName)`).
    /// Kept for the Tenant Media row, whose `File Name` and the description and MIME type the platform
    /// derives from it are not pinned: a four argument import's Tenant Media row is refused.
    pub file_name: String,
    /// The Tenant Media `File Name` of a media an ImportFile read.
    pub file_column: Option<FileColumn>,
    /// Width and height of a PNG picture; 0 for content that is no PNG (not pinned).
    pub width: i32,
    pub height: i32,
    /// ADR-044: the stream this media was imported from held a picture that was never produced (a
    /// `System.Drawing.Bitmap` saved into it): the object has a value but no bytes. Any look at its content is refused.
    pub pending: Option<std::sync::Arc<str>>,
}

/// The Tenant Media `File Name` of a media an ImportFile read.
#[derive(Debug, Clone)]
pub enum FileColumn {
    /// `Media.ImportFile`: the path as given (onprem media/MediaFiles.al:337 'media file name' = '<tmp><name>').
    Path(String),
    /// `MediaSet.ImportFile`: not pinned (MediaFiles.al:340 ignores it); a read is refused.
    Unpinned,
}

/// The eight bytes every PNG starts with.
const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

/// The size a JPEG's start-of-frame segment gives (`FF C0`..`FF CF` but `C4`, `C8`, `CC`: height, then width, big endian).
fn jpeg_size(b: &[u8]) -> Option<(i32, i32)> {
    if !b.starts_with(&[0xFF, 0xD8]) {
        return None;
    }
    let mut i = 2;
    while i + 4 <= b.len() {
        if b[i] != 0xFF {
            return None;
        }
        let marker = b[i + 1];
        let len = u16::from_be_bytes([b[i + 2], b[i + 3]]) as usize;
        if (0xC0..=0xCF).contains(&marker) && !matches!(marker, 0xC4 | 0xC8 | 0xCC) {
            let h = u16::from_be_bytes([*b.get(i + 5)?, *b.get(i + 6)?]);
            let w = u16::from_be_bytes([*b.get(i + 7)?, *b.get(i + 8)?]);
            return Some((i32::from(w), i32::from(h)));
        }
        i += 2 + len;
    }
    None
}

/// The size a PNG header gives (`IHDR` at byte 16: width and height, big endian).
fn png_size(b: &[u8]) -> Option<(i32, i32)> {
    if b.len() < 24 || b[..8] != PNG_SIGNATURE {
        return None;
    }
    let w = u32::from_be_bytes(b[16..20].try_into().ok()?);
    let h = u32::from_be_bytes(b[20..24].try_into().ok()?);
    Some((i32::try_from(w).ok()?, i32::try_from(h).ok()?))
}

/// The session's media: objects by id and the members of each set by set id.
///
/// The maps are shared handles: the `Tenant Media` table of the session reads the same objects.
#[derive(Debug, Default, Clone)]
pub struct MediaStore {
    pub objects: Rc<RefCell<HashMap<Id, MediaObject>>>,
    pub sets: Rc<RefCell<HashMap<Id, Vec<Id>>>>,
    /// Media a Modify or Delete of one holder removed from Tenant Media while a row of another table, or another field,
    /// still holds the id (media/TestMediaPngImport.al:441-674 QE_Media_*): that holder has no value any more.
    pub removed: Rc<RefCell<std::collections::HashSet<Id>>>,
    /// Media imported into a field of a Media Resources row, which BC keeps out of Tenant Media
    /// (platform/lt-platform-tables/PtCloudTables.al:352): their Tenant Media row is refused.
    pub outside_tenant: Rc<RefCell<std::collections::HashSet<Id>>>,
}

type Out = Result<Value, Signal>;

fn unsupported<T>(what: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(what.into()))
}

fn id_of(r: &Option<&mut Value>) -> Result<Id, Signal> {
    match r {
        Some(Value::Guid(g)) => Ok(*g),
        other => unsupported(format!("Media method on {}", other.describe())),
    }
}

fn store(c: &mut dyn Ctx) -> Result<&mut MediaStore, Signal> {
    match c.media_store() {
        Some(s) => Ok(s),
        None => unsupported("media in a context without a media store"),
    }
}

fn text_arg(a: &[Value], i: usize) -> Result<String, Signal> {
    match a.get(i).and_then(crate::native::text_of) {
        Some(t) => Ok(t.to_string()),
        None => unsupported(format!("a text argument, got {}", a.get(i).describe())),
    }
}

/// Reads the rest of an `InStream` (the bytes it has not handed out yet).
fn drain(v: &Value) -> Result<Vec<u8>, Signal> {
    let Value::Library(_, h) = v else {
        return unsupported(format!("an InStream was expected, got {}", v.describe()));
    };
    match &mut *h.borrow_mut() {
        LibraryValue::InStream(s) => Ok(s.drain()),
        other => unsupported(format!("an InStream was expected, got {}", other.describe())),
    }
}

/// The picture a stream holds when it was never produced (ADR-044).
fn pending_of(v: Option<&Value>) -> Option<std::sync::Arc<str>> {
    match v {
        Some(Value::Library(_, h)) => match &*h.borrow() {
            LibraryValue::InStream(s) => s.render_pending(),
            _ => None,
        },
        _ => None,
    }
}

fn import_object(c: &mut dyn Ctx, a: &[Value]) -> Result<Id, Signal> {
    // ImportStream(Stream, Description[, MimeType[, FileName]]).
    let pending = pending_of(a.first());
    let bytes = if pending.is_some() {
        Vec::new()
    } else {
        drain(
            a.first()
                .ok_or_else(|| Signal::Unsupported("ImportStream without a stream".into()))?,
        )?
    };
    let description = text_arg(a, 1)?;
    let mime = match a.get(2) {
        None | Some(Value::Null) => String::new(),
        Some(_) => text_arg(a, 2)?,
    };
    let file_name = match a.get(3) {
        None | Some(Value::Null) => String::new(),
        Some(_) => text_arg(a, 3)?,
    };
    let id = c.new_guid();
    // A PNG without a MIME type is `image/png`, and its Tenant Media row has its size (corpus
    // media/PXMediaFileName.al: `Mime Type` is `image/png` also when it is omitted, 1 x 1).
    let size = png_size(&bytes);
    let mime = if mime.is_empty() && size.is_some() {
        "image/png".to_string()
    } else {
        mime
    };
    let (width, height) = size.unwrap_or((0, 0));
    store(c)?.objects.borrow_mut().insert(
        id,
        MediaObject {
            bytes,
            mime,
            description,
            file_name,
            file_column: None,
            width,
            height,
            pending,
        },
    );
    Ok(id)
}

fn has_value(c: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    let id = id_of(&r)?;
    if id == [0; 16] {
        return Ok(Value::Boolean(false));
    }
    // A holder whose media another holder let go keeps the id but has no value (media/TestMediaPngImport.al:441-470
    // QE_Media_OtherTable_Assigned_SourceRowDeleted, `after delete has_value`).
    Ok(Value::Boolean(
        c.media_store().is_none_or(|m| !m.removed.borrow().contains(&id)),
    ))
}

fn media_id(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    Ok(Value::Guid(id_of(&r)?))
}

/// `Media.ImportStream` gives the field a new media. `MediaSet.ImportStream` fills an empty set only;
/// extending what a set already holds is not pinned, so it is refused.
fn media_import(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    // A field that already holds a media takes the new one (corpus media/PXMediaFileName.al:
    // ImportStream_DescriptionOnly_AndEmptyDescription imports into the same field twice); what becomes
    // of the old media object is not pinned, it stays.
    id_of(&r)?;
    // An empty stream imports no media: the field holds none afterwards (streams/StTMedia.al:MediaImportEmptyStream, `HasValue` No).
    // What the call answers is not pinned; the null Guid is assumed.
    if pending_of(a.first()).is_none()
        && matches!(a.first(), Some(Value::Library(_, h))
            if matches!(&*h.borrow(), LibraryValue::InStream(s) if s.rest().is_empty()))
    {
        if let Some(slot) = r {
            *slot = Value::Guid([0; 16]);
        }
        return Ok(Value::Guid([0; 16]));
    }
    let id = import_object(c, a)?;
    if let Some(slot) = r {
        *slot = Value::Guid(id);
    }
    Ok(Value::Guid(id))
}

fn set_import(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    if id_of(&r)? != [0; 16] {
        return unsupported("MediaSet.ImportStream into a set that already has members (not pinned)");
    }
    // The set's own id is what the call returns and what MediaId answers; the member has another
    // (media/TestMediaSetPersistence.al:127-155).
    let member = import_object(c, a)?;
    let set = c.new_guid();
    store(c)?.sets.borrow_mut().insert(set, vec![member]);
    if let Some(slot) = r {
        *slot = Value::Guid(set);
    }
    Ok(Value::Guid(set))
}

fn media_export(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let id = id_of(&r)?;
    if id == [0; 16] {
        // Custom Report Layout.InitBuiltInLayout exports the Layout of a built-in Report Layout List row: the corpus pins its
        // HasValue as No (report/PZReportLayoutMedia.al:51-53), not what the export answers or writes (docs/probes-needed.md).
        return unsupported(
            "Media.ExportStream of an empty field (not pinned; this includes the Layout of a built-in layout in Report Layout List, whose content the corpus does not pin)",
        );
    }
    let Some(obj) = store(c)?.objects.borrow().get(&id).cloned() else {
        // A media another holder let go exports nothing and answers false (media/TestMediaPngImport.al:441-470
        // QE_Media_OtherTable_Assigned_SourceRowDeleted, `after delete export_result`, `export_nonempty`).
        if store(c)?.removed.borrow().contains(&id) {
            return Ok(Value::Boolean(false));
        }
        return unsupported("Media.ExportStream of a media the session does not hold");
    };
    if let Some(report) = &obj.pending {
        return Err(crate::stream::render_pending_error(report));
    }
    let Some(Value::Library(_, h)) = a.first() else {
        return unsupported("Media.ExportStream without an OutStream");
    };
    match &*h.borrow() {
        LibraryValue::OutStream(blob, _) => {
            // An imported PNG comes back re-encoded on every major: 68 bytes export as 99 other bytes on Windows BC 27, 28 and 29
            // (types/W7Media.al:141-145, ExportStream_OfAnImportedPicture, corpus bump 34; bc-linux #161). The export itself
            // succeeds (streams/StTMedia.al:170 and :213, MediaExportStream and MediaExportAfterReload). The re-encoded bytes are not
            // pinned, and neither is what BC exports for content that is no PNG: the stream's buffer is marked like an unproduced
            // report (ADR-040) and any look at it is refused. Probe wanted (docs/probes-needed.md).
            let mut buf = blob.borrow_mut();
            buf.bytes.clear();
            buf.loaded = true;
            buf.written = true;
            buf.render_pending = Some(crate::stream::MEDIA_REENCODED.into());
        }
        other => return unsupported(format!("an OutStream was expected, got {}", other.describe())),
    }
    Ok(Value::Boolean(true))
}

fn set_count(c: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    let id = id_of(&r)?;
    if id == [0; 16] {
        return Ok(Value::Integer(0));
    }
    let n = store(c)?.sets.borrow().get(&id).map_or(0, Vec::len);
    Ok(Value::Integer(n as i32))
}

fn set_item(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let id = id_of(&r)?;
    let Some(Value::Integer(i)) = a.first() else {
        return unsupported("MediaSet.Item with this index");
    };
    let item = usize::try_from(*i - 1)
        .ok()
        .and_then(|i| store(c).ok()?.sets.borrow().get(&id)?.get(i).copied());
    match item {
        Some(m) => Ok(Value::Guid(m)),
        None => unsupported(format!("MediaSet.Item({i}) outside the set (error text not pinned)")),
    }
}

fn set_remove(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let id = id_of(&r)?;
    let Some(Value::Guid(m)) = a.first() else {
        return unsupported("MediaSet.Remove with this argument");
    };
    let store = store(c)?;
    let mut sets = store.sets.borrow_mut();
    let Some(members) = sets.get_mut(&id) else {
        return Ok(Value::Boolean(false));
    };
    let before = members.len();
    members.retain(|x| x != m);
    Ok(Value::Boolean(members.len() != before))
}

/// `MediaSet.Insert(MediaId)` (media/TestMediaSetPersistence.al:424-600, QV_MediaSet_Insert_*; AL Runner issue 5683):
/// the id of a media that another set holds (another row, another table, another field) goes into an empty set, which
/// then gets a set id of its own, and the call answers true. An id the set already holds, an id of no media and the null
/// id answer false and raise nothing. The field holds the new set id at once, a row not yet inserted too; a Modify or
/// Insert stores it. Not pinned, so refused: adding to a set that holds other media, and the id of a media no set holds
/// (a Media field's).
fn set_insert(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let id = id_of(&r)?;
    let Some(Value::Guid(m)) = a.first() else {
        return unsupported("MediaSet.Insert with this argument");
    };
    let m = *m;
    let s = store(c)?;
    if m == [0; 16] || !s.objects.borrow().contains_key(&m) {
        return Ok(Value::Boolean(false));
    }
    let held = s.sets.borrow().get(&id).cloned().unwrap_or_default();
    if held.contains(&m) {
        return Ok(Value::Boolean(false));
    }
    if !held.is_empty() {
        return unsupported("MediaSet.Insert into a set that already holds other media (not pinned)");
    }
    if !s.sets.borrow().values().any(|s| s.contains(&m)) {
        return unsupported("MediaSet.Insert of a media that no MediaSet holds (not pinned)");
    }
    let set = c.new_guid();
    store(c)?.sets.borrow_mut().insert(set, vec![m]);
    if let Some(slot) = r {
        *slot = Value::Guid(set);
    }
    Ok(Value::Boolean(true))
}

/// `Media.ImportFile` and `MediaSet.ImportFile` (OnPrem): a file name that is empty raises "The specified path is
/// invalid.", one that names no file "Could not find a part of the path '<path>'.", both with the code InvalidPath
/// and nothing imported (onprem session/PZMediaImportFile.al, BC 29.0). Importing a file that exists is not pinned:
/// which MIME type and description the Tenant Media row keeps for it is unknown, so it is refused.
/// The bytes of the file `ImportFile` names, or the error of an empty path or a missing file
/// (session/PZMediaImportFile.al).
fn import_file_bytes(c: &mut dyn Ctx, a: &[Value]) -> Result<(String, Vec<u8>), Signal> {
    let name = text_arg(a, 0)?;
    if name.is_empty() {
        return crate::native::raise("The specified path is invalid.", "InvalidPath");
    }
    match c.fs_read(&name)? {
        Some(bytes) => Ok((name, bytes)),
        None => crate::native::raise(format!("Could not find a part of the path '{name}'."), "InvalidPath"),
    }
}

/// `Media.ImportFile(FileName, Description)`. A `.jpg` file holding a JPEG: the returned id is the media's, the Tenant Media
/// row has the description, `image/jpeg`, the picture's size and the path as File Name (onprem media/MediaFiles.al:293-340
/// QX_MediaAndMediaSet_ImportFile_Jpeg, also for `.JPG`). A `.png` file holding a PNG is answered alike under the
/// assumption `media-importfile-png`. Any other file is refused (which of content and extension decides is not pinned).
fn import_file(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    id_of(&r)?;
    let (name, bytes) = import_file_bytes(c, a)?;
    let ext = name
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default();
    let (mime, (width, height)) = match (ext.as_str(), jpeg_size(&bytes), png_size(&bytes)) {
        ("jpg", Some(size), _) => ("image/jpeg", size),
        ("png", _, Some(size)) => {
            crate::assumptions::assume(crate::assumptions::Assumption::MediaImportFilePng);
            ("image/png", size)
        }
        _ => {
            return unsupported(format!(
                "Media.ImportFile of `{name}`: only a .jpg file holding a JPEG is pinned (what BC makes of other content or extensions is not)"
            ));
        }
    };
    let description = text_arg(a, 1)?;
    let id = c.new_guid();
    store(c)?.objects.borrow_mut().insert(
        id,
        MediaObject {
            bytes,
            mime: mime.into(),
            description,
            file_name: String::new(),
            file_column: Some(FileColumn::Path(name)),
            width,
            height,
            pending: None,
        },
    );
    if let Some(slot) = r {
        *slot = Value::Guid(id);
    }
    Ok(Value::Guid(id))
}

/// `MediaSet.ImportFile(FileName, Description)`: a `.jpg` file holding a JPEG gives an empty set one member whose Tenant Media
/// row has the description, `image/jpeg` and the picture's size (onprem media/MediaFiles.al:317-344
/// QX_MediaAndMediaSet_ImportFile_Jpeg). A `.PNG` file holding a PNG gives `image/png` and its size, a second file joins a
/// set of one member as Item(2), and the call returns the set's id, which MediaId answers and no item has; the id stays
/// when a member joins (MediaFiles.al:394-447 RD_MediaSet_ImportFile_JpegThenUpperCasePng). A lower-case `.png` is
/// answered alike under the assumption `media-importfile-png`. The File Name of the member's row is not pinned (Windows
/// and Linux BC write different names, windows-check-results 2026-10-22 QX item 9; MediaFiles.al:340 ignores it): a read
/// of it is refused. Any other file, and a set of two or more members, is refused.
fn set_import_file(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let set = id_of(&r)?;
    let held = if set == [0; 16] {
        0
    } else {
        store(c)?.sets.borrow().get(&set).map_or(0, Vec::len)
    };
    if held > 1 {
        return unsupported("MediaSet.ImportFile into a set that already has two or more members (not pinned)");
    }
    let (name, bytes) = import_file_bytes(c, a)?;
    let ext = name.rsplit_once('.').map(|(_, e)| e).unwrap_or_default();
    let (mime, (width, height)) = match (ext, jpeg_size(&bytes), png_size(&bytes)) {
        (e, Some(size), _) if e.eq_ignore_ascii_case("jpg") => ("image/jpeg", size),
        ("PNG", _, Some(size)) => ("image/png", size),
        (e, _, Some(size)) if e.eq_ignore_ascii_case("png") => {
            crate::assumptions::assume(crate::assumptions::Assumption::MediaImportFilePng);
            ("image/png", size)
        }
        _ => {
            return unsupported(format!(
                "MediaSet.ImportFile of `{name}`: only a .jpg file holding a JPEG or a .PNG file holding a PNG is pinned (what BC makes of other content or extensions is not)"
            ));
        }
    };
    let description = text_arg(a, 1)?;
    let member = c.new_guid();
    let set = if held == 0 { c.new_guid() } else { set };
    let media = store(c)?;
    media.objects.borrow_mut().insert(
        member,
        MediaObject {
            bytes,
            mime: mime.into(),
            description,
            file_name: String::new(),
            file_column: Some(FileColumn::Unpinned),
            width,
            height,
            pending: None,
        },
    );
    media.sets.borrow_mut().entry(set).or_default().push(member);
    if let Some(slot) = r {
        *slot = Value::Guid(set);
    }
    Ok(Value::Guid(set))
}

pub const REGISTRY: &[(&str, NativeFn)] = &[
    ("Media.ImportFile#1", import_file),
    ("MediaSet.ImportFile#1", set_import_file),
    ("Media.HasValue#1", has_value),
    ("Media.MediaId#1", media_id),
    ("Media.ImportStream#1", media_import),
    ("Media.ImportStream#2", media_import),
    ("Media.ExportStream#1", media_export),
    ("MediaSet.MediaId#1", media_id),
    ("MediaSet.Count#1", set_count),
    ("MediaSet.Item#1", set_item),
    ("MediaSet.ImportStream#1", set_import),
    ("MediaSet.Remove#1", set_remove),
    ("MediaSet.Insert#1", set_insert),
];
