//! `Media` and `MediaSet` fields (corpus media/).
//!
//! A Media field holds the id of its media object; a MediaSet field holds the id of its set, and
//! the set holds the ids of its media objects. Both are stored as a Guid, null for an empty field
//! (streams/media/TestMediaEmptyMediaId.al). The media objects and the sets live in the session's
//! `MediaStore`, the stand-in for the platform tables "Tenant Media" and "Tenant Media Set"
//! (ADR-013 gives those tables rows). The field only holds the id once the record is written,
//! so an import that was not followed by `Modify` is not seen by the next `Get`.
//!
//! Entry points: `Id`, `MediaObject`, `MediaStore`.

use crate::describe::Describe;
use crate::native::{Ctx, NativeFn, Signal, unsupported};
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
    /// The `ImportStream` call that made this media; `None` for a media an ImportFile made.
    pub stream_call: Option<StreamCall>,
    /// The import stored the Guided Experience icon of a logo of open corpus PR 1594 (`measured_logo_icon`), with the bytes
    /// Windows BC stores (`windows_stored_icon_bytes`): its Tenant Media row rests on assumption `app-logo-jpeg-sizes`.
    pub logo_icon: Option<MeasuredJpegLogo>,
}

/// An `ImportStream` call: the arguments it was given and whether it went into a Media field that held no media (`false` for an
/// import into a MediaSet).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamCall {
    pub args: usize,
    pub into_empty_media: bool,
}

/// The Tenant Media `File Name` of a media an ImportFile read.
#[derive(Debug, Clone)]
pub enum FileColumn {
    /// `Media.ImportFile`: the path as given (onprem streams/media/MediaFiles.al:337 'media file name' = '<tmp><name>').
    Path(String),
    /// `MediaSet.ImportFile`: the bare file name, without the folder (onprem streams/media/MediaFiles.al:351 'mediaset file name' =
    /// '<name>', :256-259 `filename_longer_than_name=No`; Windows BC 27.5, 28.5 and 29.0 and bc-linux dc007079, corpus bump 62).
    Bare(String),
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

/// The size (width, height) of a JPEG built as the one JPEG app logo measured (corpus PR 1412, tests/al-language-xapp-e/
/// logo.jpg, 8 x 8, 307 bytes), each part by equality: SOI, an APP0 `JFIF` segment first, then only quantization tables
/// (`FF DB`), one baseline start-of-frame segment (`FF C0`) of precision 8 and 3 components, Huffman tables (`FF C4`), a
/// start-of-scan segment (`FF DA`) of 3 components, at least one byte of coded data in which every `FF` is followed by
/// `00`, and then `FF D9` as the last two bytes. The size is read from the frame; width and height are at least 1.
/// `None` for any other file.
pub fn jfif_logo_size(b: &[u8]) -> Option<(i32, i32)> {
    jfif_logo(b, 0xC0)
}

/// The size (width, height) of a progressive JPEG built as the progressive logo of open corpus PR 1594
/// (tests/al-language-xapp-k/logo.jpg, 535 x 534, 9857 bytes): as [`jfif_logo_size`], but with a progressive start-of-frame
/// segment (`FF C2`), then one or more start-of-scan segments of 1 to 3 components, each followed by at least one byte of
/// coded data without a restart or other marker, Huffman tables between them, and `FF D9` as the last two bytes.
pub fn jfif_progressive_logo_size(b: &[u8]) -> Option<(i32, i32)> {
    jfif_logo(b, 0xC2)
}

/// The walk of [`jfif_logo_size`] (`frame` `FF C0`, one scan) and [`jfif_progressive_logo_size`] (`frame` `FF C2`, scans).
fn jfif_logo(b: &[u8], frame: u8) -> Option<(i32, i32)> {
    let framed = b.starts_with(&[0xFF, 0xD8, 0xFF, 0xE0]) && b.get(6..11) == Some(&b"JFIF\0"[..]);
    framed.then_some(())?;
    let progressive = frame == 0xC2;
    let mut size = None;
    let mut scans = 0;
    let mut i = 2;
    loop {
        (*b.get(i)? == 0xFF).then_some(())?;
        let marker = *b.get(i + 1)?;
        if progressive && marker == 0xD9 {
            return size.filter(|_| scans > 0 && i + 2 == b.len());
        }
        let len = usize::from(u16::from_be_bytes([*b.get(i + 2)?, *b.get(i + 3)?]));
        let segment = b.get(i + 4..i + 2 + len)?;
        match (marker, segment) {
            (0xE0, _) if i == 2 => {}
            (0xDB | 0xC4, _) => {}
            (m, [8, h1, h2, w1, w2, 3, ..]) if m == frame && len == 17 && size.is_none() => {
                let (w, h) = (u16::from_be_bytes([*w1, *w2]), u16::from_be_bytes([*h1, *h2]));
                (w >= 1 && h >= 1).then_some(())?;
                size = Some((i32::from(w), i32::from(h)));
            }
            (0xDA, [3, ..]) if !progressive && len == 12 => {
                return size.filter(|_| coded_data_then_end(&b[i + 2 + len..]));
            }
            (0xDA, [n @ 1..=3, ..]) if progressive && size.is_some() && len == 6 + 2 * usize::from(*n) => {
                i += 2 + len + coded_data_len(&b[i + 2 + len..])?;
                scans += 1;
                continue;
            }
            _ => return None,
        }
        i += 2 + len;
    }
}

/// The length of the coded data of one scan of the measured progressive logo: at least one byte, in which every `FF` is
/// followed by `00`, up to the next marker (`FF` and a byte other than `00` and the restart markers `D0`..`D7`). `None` when
/// a restart marker comes first, or no marker follows.
fn coded_data_len(rest: &[u8]) -> Option<usize> {
    let end = (0..rest.len().saturating_sub(1)).find(|&k| rest[k] == 0xFF && rest[k + 1] != 0)?;
    (end > 0 && !(0xD0..=0xD7).contains(&rest[end + 1])).then_some(end)
}

/// Whether the bytes after the measured logo's start-of-scan header are at least one byte of coded data, in which every
/// `FF` is followed by `00` (no restart or other marker), and then `FF D9` as the last two bytes.
fn coded_data_then_end(rest: &[u8]) -> bool {
    let Some(coded) = rest.strip_suffix(&[0xFF, 0xD9]) else {
        return false;
    };
    !coded.is_empty()
        && coded
            .iter()
            .enumerate()
            .all(|(k, &x)| x != 0xFF || coded.get(k + 1) == Some(&0))
}

/// A JPEG app logo file of open corpus PR 1594 (tests/al-language-onprem/session-system/UsAppLogoJpegSizes.al; al-runner issue
/// 9111), which [`measured_jpeg_logo`] recognizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasuredJpegLogo {
    /// Fixture E's 8 x 8 baseline logo (tests/al-language-xapp-e/logo.jpg). Its Published Application logo is pinned
    /// (UsAppLogoJpeg.al:82-120); its Guided Experience icon is measured by the probe only (UsLogo_GuidedExperienceIcon_Jpeg8x8_AppE).
    Fixture8x8,
    /// One of the logos of fixtures G, H, J and K, width by height: measured by the probe only, as a Published Application logo
    /// and as a Guided Experience icon.
    Sized(i32, i32),
}

impl MeasuredJpegLogo {
    /// Width and height of the picture.
    pub fn size(self) -> (i32, i32) {
        match self {
            MeasuredJpegLogo::Fixture8x8 => (8, 8),
            MeasuredJpegLogo::Sized(w, h) => (w, h),
        }
    }
}

/// The sizes of the baseline logos of open corpus PR 1594 (UsLogo_Jpeg535x534_AppG, UsLogo_Jpeg16x8_AppH,
/// UsLogo_Jpeg400x300_AppJ), width by height.
const PROBED_BASELINE_SIZES: [(i32, i32); 3] = [(535, 534), (16, 8), (400, 300)];

/// The size of the progressive logo of open corpus PR 1594 (UsLogo_JpegProgressive535x534_AppK).
const PROBED_PROGRESSIVE_SIZE: (i32, i32) = (535, 534);

/// Which logo of open corpus PR 1594 `b` is built as, on a major the probe measured on Windows BC
/// ([`al_version::app_logo_jpeg_sizes_measured`]: 27 and 29): the probe's JFIF header by equality, then a baseline walk
/// ([`jfif_logo_size`]) of 8 x 8 (fixture E) or of one of the baseline sizes, or a progressive walk ([`jfif_progressive_logo_size`])
/// of the progressive size. `None` for every other file and major (BC 28: only BC on Linux ran the probe, whose Mime Type
/// differs from Windows on 27.5).
pub fn measured_jpeg_logo(b: &[u8], target: Option<&al_version::Target>) -> Option<MeasuredJpegLogo> {
    if !al_version::app_logo_jpeg_sizes_measured(target) || !jfif_header_of_measured_logos(b) {
        return None;
    }
    match (jfif_logo_size(b), jfif_progressive_logo_size(b)) {
        (Some((8, 8)), None) => Some(MeasuredJpegLogo::Fixture8x8),
        (Some((w, h)), None) if PROBED_BASELINE_SIZES.contains(&(w, h)) => Some(MeasuredJpegLogo::Sized(w, h)),
        (None, Some((w, h))) if (w, h) == PROBED_PROGRESSIVE_SIZE => Some(MeasuredJpegLogo::Sized(w, h)),
        _ => None,
    }
}

/// The Guided Experience icon of a logo of open corpus PR 1594: what `GuidedExperienceImpl.InsertGuidedExperienceItem` stores
/// with `Icon.ImportStream(IconInStream, ExtensionId)` (two arguments, the app id as a Guid, into the Icon of a new item) when the
/// stream holds such a logo (UsAppLogoJpegSizes.al UsLogo_GuidedExperienceIcon_*: Height and Width of the picture on every
/// host, Mime Type image/jpeg on Windows BC 27.5 and 29.0). `description_is_guid`: the second argument was a Guid.
fn measured_logo_icon(
    c: &dyn Ctx,
    bytes: &[u8],
    call: StreamCall,
    description_is_guid: bool,
) -> Option<MeasuredJpegLogo> {
    let icon_call = StreamCall {
        args: 2,
        into_empty_media: true,
    };
    (call == icon_call && description_is_guid)
        .then(|| c.measured_jpeg_logo(bytes))
        .flatten()
}

/// Whether a JPEG's APP0 segment is the JFIF header of the logos of open corpus PR 1594 (tests/al-language-xapp-e, -g, -h, -j,
/// -k/logo.jpg), each byte by equality: length 16, "JFIF\0", version 1.01, density units 0, density 1 x 1, no thumbnail.
fn jfif_header_of_measured_logos(b: &[u8]) -> bool {
    b.get(2..20)
        == Some(
            &[
                0xFF, 0xE0, 0x00, 0x10, b'J', b'F', b'I', b'F', 0, 1, 1, 0, 0, 1, 0, 1, 0, 0,
            ][..],
        )
}

/// The bytes Windows BC stores for the Guided Experience icon of a logo of open corpus PR 1594 ([`measured_logo_icon`]): the file
/// with 3 bytes of its JFIF header rewritten, density units (index 13) 0 -> 1 and the low bytes of the X and Y density (indexes 15
/// and 17) 1 -> 0. Windows BC 27.5 and 29.0 stored all 5 icons so (UsAppLogoJpegSizes.al:20-23; local observe of corpus PR 1594
/// at d030ce03, 'icon equals file' No and 'icon vs logo media, differing bytes' = '14:1/0 16:0/1 18:0/1 count 3' for fixtures E,
/// G, H, J and K); BC on Linux keeps the file. The length does not change ('icon length' = the file's on every host). `bytes`
/// holds the header [`jfif_header_of_measured_logos`] checked.
fn windows_stored_icon_bytes(mut bytes: Vec<u8>) -> Vec<u8> {
    bytes[13] = 1;
    bytes[15] = 0;
    bytes[17] = 0;
    bytes
}

/// Whether a JPEG starts with a JFIF header of units 1 (dots per inch) and density 300 x 300: SOI, APP0, "JFIF\0", version,
/// then units at byte 13 and the two densities.
fn jfif_300_dpi(b: &[u8]) -> bool {
    b.len() > 18
        && b.starts_with(&[0xFF, 0xD8, 0xFF, 0xE0])
        && &b[6..11] == b"JFIF\0"
        && b[13..18] == [1, 0x01, 0x2C, 0x01, 0x2C]
}

/// The size a PNG header gives (`IHDR` at byte 16: width and height, big endian).
/// A media whose name made it `image/jpeg` while it holds a PNG (ReMediaJpgHoldingPng.al:108): BC stores the picture
/// re-encoded (111 bytes on Linux, other bytes on Windows, the test ignores both), so its Tenant Media Content is not pinned.
pub fn jpg_holding_png(o: &MediaObject) -> bool {
    o.mime == "image/jpeg" && png_size(&o.bytes).is_some()
}

pub fn png_size(b: &[u8]) -> Option<(i32, i32)> {
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
    /// still holds the id (streams/media/TestMediaPngImport.al:441-674 QE_Media_*): that holder has no value any more.
    pub removed: Rc<RefCell<std::collections::HashSet<Id>>>,
    /// Media imported into a field of a Media Resources row, which BC keeps out of Tenant Media
    /// (metadata/platform-tables/PtCloudTables.al:352): their Tenant Media row is refused.
    pub outside_tenant: Rc<RefCell<std::collections::HashSet<Id>>>,
}

type Out = Result<Value, Signal>;

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
    // A Guid description is its text form, in braces and upper case: Guided Experience Impl imports an app's logo with
    // `Icon.ImportStream(IconInStream, ExtensionId)` and the Tenant Media Description reads '{E92BE076-...}' (onprem
    // session-system/UsAppLogo.al:170-186, corpus PR 816).
    if let Some(Value::Guid(g)) = a.get(i) {
        return Ok(crate::format::format_guid(g));
    }
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

/// The media an ImportStream makes, or `None` when it makes none (`media_word_package::StreamImport::Nothing`, only into an empty
/// Media: `empty_media`).
fn import_object(c: &mut dyn Ctx, a: &[Value], empty_media: bool) -> Result<Option<Id>, Signal> {
    // ImportStream(Stream, Description[, MimeType[, FileName]]).
    let pending = pending_of(a.first());
    let bytes = match &pending {
        // A placeholder report output is imported byte for byte with its mark (an e-mail attachment: Email Message Impl.
        // InsertAttachment, ADR-040 amendment 2026-11-11); a read of the content stays refused.
        Some(mark) if crate::placeholder_output::is_placeholder(mark) => match a.first() {
            Some(Value::Library(_, h)) => match &mut *h.borrow_mut() {
                LibraryValue::InStream(s) => s.drain_placeholder(),
                _ => Vec::new(),
            },
            _ => Vec::new(),
        },
        Some(_) => Vec::new(),
        None => drain(
            a.first()
                .ok_or_else(|| Signal::Unsupported("ImportStream without a stream".into()))?,
        )?,
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
    crate::media_word_package::office_header_check(&bytes, &description, &mime, &file_name)?;
    let file_name_given = !matches!(a.get(3), None | Some(Value::Null));
    let stream_call = StreamCall {
        args: a.iter().take_while(|v| !matches!(v, Value::Null)).count(),
        into_empty_media: empty_media,
    };
    let word = match pending {
        None => crate::media_word_package::stream_import(&bytes, &mime, file_name_given, empty_media)?,
        Some(_) => None,
    };
    let pending = match word {
        Some(crate::media_word_package::StreamImport::Nothing) => return Ok(None),
        Some(crate::media_word_package::StreamImport::Rezipped) => Some(crate::stream::MEDIA_DOCX_REZIPPED.into()),
        None => pending,
    };
    let id = c.new_guid();
    // A PNG without a MIME type is `image/png`, and its Tenant Media row has its size (corpus
    // streams/media/PXMediaFileName.al: `Mime Type` is `image/png` also when it is omitted, 1 x 1).
    let size = png_size(&bytes);
    let mime = if mime.is_empty() && size.is_some() {
        "image/png".to_string()
    } else {
        mime
    };
    let logo_icon = match &pending {
        None => measured_logo_icon(c, &bytes, stream_call, matches!(a.get(1), Some(Value::Guid(_)))),
        _ => None,
    };
    let (bytes, mime, (width, height)) = match logo_icon {
        Some(logo) => (windows_stored_icon_bytes(bytes), "image/jpeg".to_string(), logo.size()),
        None => (bytes, mime, size.unwrap_or((0, 0))),
    };
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
            stream_call: Some(stream_call),
            logo_icon,
        },
    );
    Ok(Some(id))
}

fn has_value(c: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    let id = id_of(&r)?;
    if id == [0; 16] {
        return Ok(Value::Boolean(false));
    }
    // A holder whose media another holder let go keeps the id but has no value (streams/media/TestMediaPngImport.al:441-470
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
    // A field that already holds a media takes the new one (corpus streams/media/PXMediaFileName.al:
    // ImportStream_DescriptionOnly_AndEmptyDescription imports into the same field twice); what becomes
    // of the old media object is not pinned, it stays.
    let empty = id_of(&r)? == [0; 16];
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
    let id = import_object(c, a, empty)?.unwrap_or([0; 16]);
    if let Some(slot) = r {
        *slot = Value::Guid(id);
    }
    Ok(Value::Guid(id))
}

/// `import_object` answers `None` only for an empty Media.
const IMPORT_INTO_A_SET: &str = "an import into a MediaSet always makes a media";

fn set_import(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let current = id_of(&r)?;
    if current != [0; 16] {
        // Another import adds a member and answers the set's id again (streams/StTMedia.al:MediaSetImportThreeArgs `count2` 2,
        // :MediaSetImportTwiceAdds `count` 2 and `ids-differ` No).
        if !store(c)?.sets.borrow().contains_key(&current) {
            return unsupported("MediaSet.ImportStream into a set the session does not hold (not pinned)");
        }
        let member = import_object(c, a, false)?.expect(IMPORT_INTO_A_SET);
        store(c)?
            .sets
            .borrow_mut()
            .get_mut(&current)
            .expect("checked")
            .push(member);
        return Ok(Value::Guid(current));
    }
    // The set's own id is what the call returns and what MediaId answers; the member has another
    // (streams/media/TestMediaSetPersistence.al:127-155).
    let member = import_object(c, a, false)?.expect(IMPORT_INTO_A_SET);
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
        // An empty field exports without an error, writes nothing and answers false (streams/StTMedia.al:177-192
        // MediaExportStreamEmpty `len` 0, types/W7Media.al:155-171 ExportStream_OfAnEmptyField `exported length` 0; al-runner issue
        // 5993: streams/StTMedia.al SO_MediaExportStream_InitNotInserted, SO_MediaExportStream_InsertedEmpty `returned=No len=0`).
        // The Layout of a built-in Report Layout List row, whose content the corpus does not pin, refuses in the lowering (al-sema
        // call.rs).
        return Ok(Value::Boolean(false));
    }
    let Some(obj) = store(c)?.objects.borrow().get(&id).cloned() else {
        // A media another holder let go exports nothing and answers false (streams/media/TestMediaPngImport.al:441-470
        // QE_Media_OtherTable_Assigned_SourceRowDeleted, `after delete export_result`, `export_nonempty`).
        if store(c)?.removed.borrow().contains(&id) {
            return Ok(Value::Boolean(false));
        }
        return unsupported("Media.ExportStream of a media the session does not hold");
    };
    // A Word package stored re-zipped exports without an error (al-language-onprem/streams/media/MediaFiles.al:500-505
    // RD_Media_ImportFile_Docx; Config. XML Exchange.ExportMedia in the DemoTool company build) and gives the stored bytes (corpus PR
    // 1469, MediaDocxRezip.al:667 'export is the stored content'): the stream is marked with the import's mark, so a look at what it
    // holds is refused (`media_word_package`).
    let docx_mark = obj.pending.as_deref() == Some(crate::stream::MEDIA_DOCX_REZIPPED);
    // A placeholder report output exports byte for byte with its mark (`placeholder_output`).
    if let Some(mark) = obj
        .pending
        .as_ref()
        .filter(|m| crate::placeholder_output::is_placeholder(m))
    {
        let Some(Value::Library(_, h)) = a.first() else {
            return unsupported("Media.ExportStream without an OutStream");
        };
        let LibraryValue::OutStream(blob, ..) = &*h.borrow() else {
            return unsupported("Media.ExportStream to a value that is not an OutStream");
        };
        crate::stream::write_bytes(blob, &obj.bytes);
        blob.borrow_mut().render_pending = Some(mark.clone());
        return Ok(Value::Boolean(true));
    }
    if let Some(report) = obj.pending.as_ref().filter(|_| !docx_mark) {
        return Err(crate::stream::render_pending_error(report));
    }
    let Some(Value::Library(_, h)) = a.first() else {
        return unsupported("Media.ExportStream without an OutStream");
    };
    match &*h.borrow() {
        LibraryValue::OutStream(blob, ..) if docx_mark => {
            let mut buf = blob.borrow_mut();
            buf.bytes.clear();
            buf.loaded = true;
            buf.written = true;
            buf.render_pending = Some(crate::stream::MEDIA_DOCX_REZIPPED.into());
        }
        // A Word package imported as application/zip comes back unchanged (media_export.rs).
        LibraryValue::OutStream(blob, ..) if crate::media_export::zip_package_exports_unchanged(&obj) => {
            crate::assumptions::assume(crate::assumptions::Assumption::MediaZipMimeExportKept);
            crate::stream::write_bytes(blob, &obj.bytes)
        }
        // A 300 dpi JFIF JPEG that Media.ImportFile read comes back unchanged on Windows BC 29.0 (probe cu 61288
        // SC_DemoTool_Media_ExportStream, case JD; the DemoTool's pictures). No corpus test pins it yet: a registered assumption
        // under the Microsoft-test exception (CLAUDE.md rule 3).
        LibraryValue::OutStream(blob, ..)
            if obj.mime == "image/jpeg"
                && obj.file_column.is_some()
                && jfif_300_dpi(&obj.bytes)
                && jpeg_size(&obj.bytes).is_some()
                && c.gate(al_version::Gate::JpegMediaSetKeptFrom29) =>
        {
            crate::assumptions::assume(crate::assumptions::Assumption::MediaExportStreamJfif300DpiKept);
            crate::stream::write_bytes(blob, &obj.bytes)
        }
        // Text and binary content that is no picture comes back as the same bytes (media_export.rs).
        LibraryValue::OutStream(blob, ..) if crate::media_export::exports_unchanged(&obj) => {
            crate::stream::write_bytes(blob, &obj.bytes)
        }
        LibraryValue::OutStream(blob, ..) => {
            // An imported PNG comes back re-encoded on every major: 68 bytes export as 99 other bytes on Windows BC 27, 28 and 29
            // (types/W7Media.al:141-145, ExportStream_OfAnImportedPicture, corpus bump 34; bc-linux #161). The export itself
            // succeeds (streams/StTMedia.al:170 and :213, MediaExportStream and MediaExportAfterReload). The re-encoded bytes are
            // pinned for one 1x1 PNG only (streams/media/Y44MediaExportStream.al DocAttach_Png_ExportStream), which
            // gives no encoder: the stream's buffer is marked like an unproduced report (ADR-040) and any look at it is refused.
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
    let i = *i;
    let item = usize::try_from(i - 1)
        .ok()
        .and_then(|i| store(c).ok()?.sets.borrow().get(&id)?.get(i).copied());
    let known = id == [0; 16] || store(c)?.sets.borrow().contains_key(&id);
    match item {
        Some(m) => Ok(Value::Guid(m)),
        // Index 0 and an index past the members raise (streams/StTMedia.al:MediaSetEmpty `item0`, `item1`;
        // :MediaSetImportTwiceAdds `item3`, `item0`). A negative index is not pinned.
        None if known && i == 0 => crate::native::raise(
            "The parameter index must have a value greater than zero.",
            "IndexOutOfBounds",
        ),
        None if known && i > 0 => crate::native::raise("Argument was out of range: index.", "ArgumentOutOfRange"),
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

/// `MediaSet.Insert(MediaId)` (streams/media/TestMediaSetPersistence.al:424-600, QV_MediaSet_Insert_*; AL Runner issue 5683):
/// the id of a media that another set holds (another row, another table, another field) goes into an empty set, which
/// then gets a set id of its own, and the call answers true. An id the set already holds, an id of no media and the null
/// id answer false and raise nothing. The field holds the new set id at once, a row not yet inserted too; a Modify or
/// Insert stores it. Not pinned, so refused: adding to a set that holds other media, and the id of a media no set holds
/// (a Media field's), and the ID of a Tenant Media row an AL Insert stored.
fn set_insert(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let id = id_of(&r)?;
    let Some(Value::Guid(m)) = a.first() else {
        return unsupported("MediaSet.Insert with this argument");
    };
    let m = *m;
    // The case that stores such a row (streams/media/Y48TenantMediaRows.al:634-649) puts it into no MediaSet.
    if c.is_stored_tenant_media_id(m) {
        return unsupported("MediaSet.Insert of the ID of a Tenant Media row an AL Insert stored (not pinned)");
    }
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
/// and nothing imported (onprem session-system/PZMediaImportFile.al, BC 29.0). Importing a file that exists is not pinned:
/// which MIME type and description the Tenant Media row keeps for it is unknown, so it is refused.
/// The bytes of the file `ImportFile` names, or the error of an empty path or a missing file
/// (session-system/PZMediaImportFile.al).
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
/// row has the description, `image/jpeg`, the picture's size and the path as File Name (onprem streams/media/MediaFiles.al:293-340
/// QX_MediaAndMediaSet_ImportFile_Jpeg, also for `.JPG`). A `.png` file holding a PNG is answered alike under the
/// assumption `media-importfile-png`. A Word document follows `media_word_package::media_import_file`. Any other file is refused
/// (which of content and extension decides is not pinned).
fn import_file(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let empty = id_of(&r)? == [0; 16];
    let (name, bytes) = import_file_bytes(c, a)?;
    let ext = name
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default();
    let word = crate::media_word_package::media_import_file(&name, &bytes, empty);
    let (mime, (width, height)) = match (word, ext.as_str(), jpeg_size(&bytes), png_size(&bytes)) {
        (Some(crate::media_word_package::FileImport::Nothing), ..) => {
            if let Some(slot) = r {
                *slot = Value::Guid([0; 16]);
            }
            return Ok(Value::Guid([0; 16]));
        }
        (Some(crate::media_word_package::FileImport::Rezipped(mime)), ..) => (mime, (0, 0)),
        (None, "jpg", Some(size), _) => ("image/jpeg", size),
        // A .jpg holding a PNG: the name decides the MIME type, the PNG data the size (onprem streams/media/ReMediaJpgHoldingPng.al:108,
        // 'media tenant'). Its stored content is re-encoded, not pinned (`jpg_holding_png`).
        (None, "jpg", None, Some(size)) if name.ends_with(".jpg") => ("image/jpeg", size),
        (None, "png", _, Some(size)) => {
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
    let pending = word.map(|_| crate::stream::MEDIA_DOCX_REZIPPED.into());
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
            pending,
            stream_call: None,
            logo_icon: None,
        },
    );
    if let Some(slot) = r {
        *slot = Value::Guid(id);
    }
    Ok(Value::Guid(id))
}

/// The file name of `path` without its folder: what follows the last `\\` (the corpus imports from TemporaryPath(), MediaFiles.al:52
/// BaseOf).
fn bare_file_name(path: &str) -> &str {
    path.rsplit_once('\\').map_or(path, |(_, name)| name)
}

/// `MediaSet.ImportFile(FileName, Description)`: a `.jpg` file holding a JPEG gives an empty set one member whose Tenant Media
/// row has the description, `image/jpeg` and the picture's size (onprem streams/media/MediaFiles.al:317-344
/// QX_MediaAndMediaSet_ImportFile_Jpeg). A `.PNG` file holding a PNG gives `image/png` and its size, a second file joins a
/// set of one member as Item(2), and the call returns the set's id, which MediaId answers and no item has; the id stays
/// when a member joins (MediaFiles.al:394-447 RD_MediaSet_ImportFile_JpegThenUpperCasePng). A lower-case `.png` is
/// answered alike under the assumption `media-importfile-png`. The File Name of the member's row is the bare file name
/// (`FileColumn::Bare`). A Word package follows `media_word_package::set_import_file`. Any other file, a path with a `/`, and a
/// set of two or more members, is refused.
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
    if name.contains('/') {
        return unsupported(format!(
            "MediaSet.ImportFile of `{name}`: the File Name BC stores for a path with a `/` is not pinned (only `\\` paths are)"
        ));
    }
    let ext = name.rsplit_once('.').map(|(_, e)| e).unwrap_or_default();
    let word = crate::media_word_package::set_import_file(&name, &bytes, set == [0; 16]);
    let (mime, (width, height)) = match (word, ext, jpeg_size(&bytes), png_size(&bytes)) {
        (Some(mime), ..) => (mime, (0, 0)),
        (None, e, Some(size), _) if e.eq_ignore_ascii_case("jpg") => ("image/jpeg", size),
        // ReMediaJpgHoldingPng.al:108 'mediaset item 1 tenant': as for Media.ImportFile.
        (None, "jpg", None, Some(size)) => ("image/jpeg", size),
        (None, "PNG", _, Some(size)) => ("image/png", size),
        (None, e, _, Some(size)) if e.eq_ignore_ascii_case("png") => {
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
            file_column: Some(FileColumn::Bare(bare_file_name(&name).to_string())),
            width,
            height,
            pending: word.map(|_| crate::stream::MEDIA_DOCX_REZIPPED.into()),
            stream_call: None,
            logo_icon: None,
        },
    );
    media.sets.borrow_mut().entry(set).or_default().push(member);
    if let Some(slot) = r {
        *slot = Value::Guid(set);
    }
    Ok(Value::Guid(set))
}

/// `MediaSet.ExportFile(Prefix)` as Config. XML Exchange.ExportMediaSet calls it in the company build (onprem
/// streams/media/MediaFiles.al:776, RJ_MediaSet_ExportFile_ConfigPackageShape): it returns the number of items and writes one file per
/// item, `<Prefix>-1`, `<Prefix>-2` ... with no extension; file n holds the Tenant Media content of the n-th Tenant Media Set
/// row of the set in primary key order (ID, Media ID), not Item(n); only a set of one item is served. Only the company build's file system is served (owner rule
/// 2026-10-22), and only items `MediaSet.ImportFile` read: each is written as a file a look at is refused (a PNG is re-encoded
/// on every major; whether a JPEG keeps its bytes is not pinned since corpus bump 62: Windows BC 29.0 re-encodes it, the Linux
/// image keeps it, MediaFiles.al ignores 'tenant item bytes same as files', corpus issue 723). Refused: any other
/// item, an empty set, a set of two or more items, an empty prefix and a folder that does not exist (the errors are not
/// pinned).
fn set_export_file(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let set = id_of(&r)?;
    let prefix = text_arg(a, 0)?;
    let folder = prefix.rfind(['\\', '/']).map(|i| &prefix[..i]).unwrap_or_default();
    if prefix.is_empty() || !c.fs_serves_path(&prefix) {
        return unsupported(
            "MediaSet.ExportFile outside the company build's temporary folder (what it writes to the server's file system is served only there)",
        );
    }
    if !c.fs_dir_exists(folder) {
        return unsupported("MediaSet.ExportFile into a folder that does not exist (the error is not pinned)");
    }
    let items = if set == [0; 16] {
        Vec::new()
    } else {
        store(c)?.sets.borrow().get(&set).cloned().unwrap_or_default()
    };
    if items.is_empty() {
        return unsupported("MediaSet.ExportFile of an empty set (not pinned)");
    }
    // The files follow the Tenant Media Set key (ID, Media ID); how BC sorts the Media ID Guids there is not pinned, so a set of
    // two or more items is refused (the DemoTool's item pictures are sets of one).
    if items.len() > 1 {
        return unsupported(
            "MediaSet.ExportFile of a set of two or more items (which item each numbered file holds follows the order of Guid keys, which is not pinned)",
        );
    }
    // `None`: BC re-encodes the item (a PNG on every major, W7Media.al:141-145; a .jpg holding a PNG, ReMediaJpgHoldingPng.al:108),
    // or whether it does is not pinned (a JPEG, see above), so the file is written with the mark of a re-encoded media and a look at
    // it is refused (as Media.ExportStream does).
    for id in &items {
        let obj = store(c)?.objects.borrow().get(id).cloned();
        match obj {
            Some(o) if o.pending.is_none() && matches!(o.file_column, Some(FileColumn::Bare(_))) => {
                let is_jpeg = o.mime == "image/jpeg" && jpeg_size(&o.bytes).is_some();
                let is_png = png_size(&o.bytes).is_some() && matches!(o.mime.as_str(), "image/jpeg" | "image/png");
                if !is_jpeg && !is_png {
                    return unsupported(format!(
                        "MediaSet.ExportFile of an item of type `{}` holding neither a JPEG nor a PNG (not pinned)",
                        o.mime
                    ));
                }
            }
            other => {
                let what = match &other {
                    None => "an item the session does not hold".to_string(),
                    Some(o) if o.pending.is_some() => "an item whose content was never produced".to_string(),
                    Some(o) => format!(
                        "an item of type `{}` read by {}",
                        o.mime,
                        match o.file_column {
                            Some(_) => "Media.ImportFile",
                            None => "a stream import",
                        }
                    ),
                };
                return unsupported(format!(
                    "MediaSet.ExportFile of {what}: its Tenant Media content is not pinned (only a MediaSet.ImportFile read is)"
                ));
            }
        }
    }
    for n in 1..=items.len() {
        let name = format!("{prefix}-{n}");
        if let Err(why) = c.fs_write_unproduced(&name, crate::stream::MEDIA_SET_FILE_REENCODED) {
            return unsupported(format!("MediaSet.ExportFile could not write `{name}`: {why}"));
        }
    }
    Ok(Value::Integer(items.len() as i32))
}

pub const REGISTRY: &[(&str, NativeFn)] = &[
    ("MediaSet.ExportFile#1", set_export_file),
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
