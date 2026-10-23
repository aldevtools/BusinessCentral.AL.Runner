//! The `File` data type, on the host file system capability (ADR-006, `al-host::FileSystem`).
//!
//! The corpus pins none of this. Its only reach is Base Application report 91 "Export
//! Consolidation" and the File Management codeunit behind it (handlers/TestPrecompiledOptionCaption.al
//! and four more files run the report), which need the calls to lower. What runs is what Microsoft
//! Learn's "File data type" page documents plainly, on a file system that is in memory unless the
//! command line gives a directory; every assumption is a row in docs/probes-needed.md ("File").
//! Everything else is refused when it runs, never guessed at:
//!
//! - Browser round trips (`Upload`, `Download`, `UploadIntoStream`, `DownloadFromStream`, `View`)
//!   need a client. AL Runner v2 refuses `Upload` and `Download` the same way
//!   (`AlRunner/Patches/FilePatches.cs`, docs/scope.md section 3.4).
//! - Binary mode, reads of other types, `Seek`, `Trunc`, `Pos` and file stamps have no pinned form.
//!
//! A file is a buffer ([`crate::stream::BlobRef`]), so `CreateInStream` and `CreateOutStream` are the
//! stream types the corpus already pins for Blob fields. `Create` and `Open` fill it, `Close` writes
//! it to the host file system. A temporary file never reaches the host file system: it is a name and
//! a buffer.

use crate::describe::Describe;
use crate::native::{Ctx, NativeFn, Signal};
use crate::signature::Signature;
use crate::stream::{BlobRef, Encoding, InState, new_blob, write_bytes};
use crate::value::{LibraryValue, Value};

type Out = Result<Value, Signal>;

/// Where the names of temporary files live. `IsPathTemporary` is true under it.
pub const TEMP_DIR: &str = "temp/";

/// What a `File` variable holds.
#[derive(Debug)]
pub struct FileState {
    pub name: String,
    pub blob: BlobRef,
    pub text_mode: bool,
    pub write_mode: bool,
    pub enc: Encoding,
    /// Between `Create` / `Open` / `CreateTempFile` and `Close`.
    pub open: bool,
    /// Whether writes are allowed (`Create` and temporary files, or `Open` with `WriteMode`).
    pub writable: bool,
    pub temp: bool,
}

impl Default for FileState {
    fn default() -> FileState {
        FileState {
            name: String::new(),
            blob: new_blob(),
            text_mode: false,
            write_mode: false,
            enc: Encoding::default(),
            open: false,
            writable: false,
            temp: false,
        }
    }
}

fn unsupported<T>(what: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(what.into()))
}

fn with_file<R>(r: Option<&mut Value>, f: impl FnOnce(&mut FileState) -> Result<R, Signal>) -> Result<R, Signal> {
    let Some(Value::Library(_, cell)) = r else {
        return unsupported("File method without a receiver");
    };
    let mut lib = cell.borrow_mut();
    if matches!(&*lib, LibraryValue::Empty) {
        *lib = LibraryValue::File(FileState::default());
    }
    match &mut *lib {
        LibraryValue::File(s) => f(s),
        other => unsupported(format!("File method on {}", other.describe())),
    }
}

fn path(a: &[Value], i: usize) -> Result<String, Signal> {
    match a.get(i).and_then(crate::native::text_of) {
        Some(t) => Ok(t.to_string()),
        None => unsupported(format!("a file name, got {}", a.get(i).describe())),
    }
}

fn fs_failure(c: &dyn Ctx, what: &str) -> Out {
    if c.ret_used() {
        Ok(Value::Boolean(false))
    } else {
        unsupported(format!("{what} (error text not pinned)"))
    }
}

fn open_file_needed(s: &FileState, what: &str) -> Result<(), Signal> {
    if s.open {
        Ok(())
    } else {
        unsupported(format!("{what} on a file that is not open (error text not pinned)"))
    }
}

/// `Create` and `Open` with no file name raise `MissingFileName`, and on a variable whose file is still open `FileAlreadyOpened`
/// (al-language-onprem/lt-file-type/FileErrors.al:P110_InvalidPaths `create_empty_name`, `open_empty_name`, `open_in_missing_dir`;
/// P110_CreateTwiceWithoutClose). Which of the two wins when both apply is not pinned: the open file is checked first.
fn already_open_or_nameless(r: Option<&Value>, name: &str) -> Result<(), Signal> {
    if let Some(Value::Library(_, cell)) = r {
        if let LibraryValue::File(s) = &*cell.borrow() {
            if s.open {
                return crate::native::raise(
                    "The operation cannot be performed while the file is open.",
                    "FileAlreadyOpened",
                );
            }
        }
    }
    if name.is_empty() {
        return crate::native::raise("File name was not specified.", "MissingFileName");
    }
    Ok(())
}

/// `Create(Name [, Encoding])`: a new empty file, or an existing one emptied; open for writing.
fn create(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let name = path(a, 0)?;
    let enc = Encoding::from_arg(a)?;
    already_open_or_nameless(r.as_deref(), &name)?;
    if let Err(e) = c.fs_write(&name, &[]) {
        return fs_failure(c, &format!("File.Create({name}): {e}"));
    }
    with_file(r, |s| {
        s.name = name;
        s.blob = new_blob();
        s.enc = enc;
        s.open = true;
        s.writable = true;
        s.temp = false;
        Ok(())
    })?;
    Ok(Value::Boolean(true))
}

/// `Open(Name [, Encoding])`: an existing file, for reading, and for writing if `WriteMode` is on.
fn open(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let name = path(a, 0)?;
    let enc = Encoding::from_arg(a)?;
    already_open_or_nameless(r.as_deref(), &name)?;
    // A file of content carried unread in the company build (owner decision 2026-10-23 "Carry bytes unread"): the open file is
    // an opaque buffer, so its streams only move it on (CopyStream) and any look at it is refused.
    let carried = c.fs_carried_mark(&name);
    let bytes = match &carried {
        Some(_) => Vec::new(),
        None => match c.fs_read(&name)? {
            Some(bytes) => bytes,
            None => return fs_failure(c, &format!("File.Open of the missing file {name}")),
        },
    };
    with_file(r, |s| {
        let blob = new_blob();
        blob.borrow_mut().bytes = bytes;
        if let Some(mark) = carried {
            crate::stream::mark_carried(&blob, mark);
            blob.borrow_mut().written = false;
        }
        s.name = name;
        s.blob = blob;
        s.enc = enc;
        s.open = true;
        s.writable = s.write_mode;
        s.temp = false;
        Ok(())
    })?;
    Ok(Value::Boolean(true))
}

/// `CreateTempFile([Encoding])`: a file that lives in memory and is gone on `Close`.
fn create_temp(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let enc = match a.first() {
        None => Encoding::default(),
        Some(_) => {
            // The encoding is the first argument here, not the second.
            let shifted = [Value::Null, a[0].clone()];
            Encoding::from_arg(&shifted)?
        }
    };
    let name = c.fs_temp_name();
    with_file(r, |s| {
        s.name = name;
        s.blob = new_blob();
        s.enc = enc;
        s.open = true;
        s.writable = true;
        s.temp = true;
        Ok(())
    })?;
    Ok(Value::Boolean(true))
}

/// `Close()`: writes a file opened for writing to the host. Closing a file that is not open does
/// nothing (assumed, docs/probes-needed.md).
fn close(c: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    let pending = with_file(r, |s| {
        let was = std::mem::take(&mut s.open);
        Ok((was && s.writable && !s.temp).then(|| {
            let b = s.blob.borrow();
            (s.name.clone(), b.bytes.clone(), b.render_pending.clone())
        }))
    })?;
    if let Some((name, bytes, mark)) = pending {
        // Content carried unread in the company build stays unread in the file it is written to.
        let wrote = match mark.filter(|m| c.company_build() && crate::stream::is_carried_mark(m)) {
            Some(mark) => c.fs_write_unproduced(&name, &mark),
            None => c.fs_write(&name, &bytes),
        };
        if let Err(e) = wrote {
            return unsupported(format!("File.Close({name}): {e}"));
        }
    }
    Ok(Value::Null)
}

/// `Write(Text)` in text mode: the text in the file's encoding, then a carriage return and line
/// feed. Other values and binary mode have no pinned form.
fn write(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    with_file(r, |s| {
        open_file_needed(s, "File.Write")?;
        s.blob.borrow().observe()?;
        if !s.writable {
            return unsupported("File.Write on a file opened without WriteMode (error text not pinned)");
        }
        if !s.text_mode {
            return unsupported("File.Write in binary mode (the binary form of the value is not pinned)");
        }
        let Some(mut text) = a.first().and_then(crate::native::text_of) else {
            return unsupported(format!(
                "File.Write of {} (only a text is documented plainly)",
                a.first().describe()
            ));
        };
        text.push_str("\r\n");
        write_bytes(&s.blob, &s.enc.encode(&text));
        Ok(Value::Null)
    })
}

fn len(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_file(r, |s| {
        open_file_needed(s, "File.Len")?;
        // The length of content carried unread (`stream::is_carried_mark`) is a look at it.
        s.blob.borrow().observe()?;
        Ok(Value::Integer(s.blob.borrow().bytes.len() as i32))
    })
}

fn name(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_file(r, |s| Ok(Value::Text(s.name.clone().into())))
}

fn create_out_stream(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let (blob, enc) = with_file(r, |s| {
        open_file_needed(s, "File.CreateOutStream")?;
        if !s.writable {
            return unsupported("File.CreateOutStream on a file opened without WriteMode (error text not pinned)");
        }
        if s.blob.borrow().write_pos != 0 {
            return unsupported("File.CreateOutStream after the file was written (where it writes is not pinned)");
        }
        Ok((s.blob.clone(), s.enc))
    })?;
    match &a[0] {
        Value::Library(_, h) => *h.borrow_mut() = LibraryValue::OutStream(blob, enc),
        other => return unsupported(format!("a stream variable was expected, got {}", other.describe())),
    }
    Ok(Value::Null)
}

fn create_in_stream(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let (blob, enc) = with_file(r, |s| {
        open_file_needed(s, "File.CreateInStream")?;
        Ok((s.blob.clone(), s.enc))
    })?;
    match &a[0] {
        Value::Library(_, h) => *h.borrow_mut() = LibraryValue::InStream(InState::live(blob, enc)),
        other => return unsupported(format!("a stream variable was expected, got {}", other.describe())),
    }
    Ok(Value::Null)
}

/// `TextMode` and `WriteMode`: with an argument they set the mode (and the result is not pinned),
/// without one they answer it. Both only matter before the file is opened.
fn flag(
    which: fn(&mut FileState) -> &mut bool,
    what: &'static str,
) -> impl Fn(&mut dyn Ctx, Option<&mut Value>, &mut [Value]) -> Out {
    move |c, r, a| {
        with_file(r, |s| match a.first() {
            None => Ok(Value::Boolean(*which(s))),
            Some(Value::Boolean(b)) => {
                if c.ret_used() {
                    return unsupported(format!("the result of File.{what}(Mode) (not pinned)"));
                }
                *which(s) = *b;
                Ok(Value::Null)
            }
            Some(other) => unsupported(format!("File.{what} with {}", other.describe())),
        })
    }
}

fn text_mode(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    flag(|s| &mut s.text_mode, "TextMode")(c, r, a)
}

fn write_mode(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    flag(|s| &mut s.write_mode, "WriteMode")(c, r, a)
}

fn exists(c: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    let name = path(a, 0)?;
    Ok(Value::Boolean(c.fs_exists(&name)?))
}

fn erase(c: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    let name = path(a, 0)?;
    // The full text is pinned (al-language-onprem/lt-file-type/FileEraseAndTempPath.al:Erase_EmptyName_FullErrorText; the shorter
    // FileErrors.al:P110_InvalidPaths `erase_empty_name` cuts it at the quote).
    if name.is_empty() {
        return crate::native::raise("The value cannot be an empty string. (Parameter 'path')", "InvalidPath");
    }
    if c.fs_remove(&name)? {
        Ok(Value::Boolean(true))
    } else {
        fs_failure(c, &format!("File.Erase of the missing file {name}"))
    }
}

fn copy(c: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    let (from, to) = (path(a, 0)?, path(a, 1)?);
    let Some(bytes) = c.fs_read(&from)? else {
        return fs_failure(c, &format!("File.Copy of the missing file {from}"));
    };
    if c.fs_exists(&to)? {
        return unsupported(format!("File.Copy onto the existing file {to} (not pinned)"));
    }
    match c.fs_write(&to, &bytes) {
        Ok(()) => Ok(Value::Boolean(true)),
        Err(e) => fs_failure(c, &format!("File.Copy to {to}: {e}")),
    }
}

fn rename(c: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    let (from, to) = (path(a, 0)?, path(a, 1)?);
    let Some(bytes) = c.fs_read(&from)? else {
        return fs_failure(c, &format!("File.Rename of the missing file {from}"));
    };
    if c.fs_exists(&to)? {
        return unsupported(format!("File.Rename onto the existing file {to} (not pinned)"));
    }
    if let Err(e) = c.fs_write(&to, &bytes) {
        return fs_failure(c, &format!("File.Rename to {to}: {e}"));
    }
    c.fs_remove(&from)?;
    Ok(Value::Boolean(true))
}

/// Yes only where real BC on Windows and the Linux container agree (Windows runs 39125184978 BC 27.5 and 39125190236 BC 29.0 of
/// the corpus repo): a name under `TemporaryPath()` (`\TEMP\`, without `..`), the empty and blank text, and a relative path with
/// or without a subfolder (al-language-onprem/lt-file-type/FileTemp.al:140-146, FileEraseAndTempPath.al:IsPathTemporary_Paths).
/// Windows answers No for drive-letter paths, UNC paths and `/`, while the Linux container answers Yes for them: that Yes is a
/// Linux artifact, not BC behavior, so those forms (and every other absolute path) are refused.
fn is_path_temporary(_: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    let p = path(a, 0)?;
    let pinned = p.starts_with(TEMP_DIR)
        || (p.starts_with("\\TEMP\\") && !p.contains(".."))
        || p.trim().is_empty()
        || (!p.starts_with(['/', '\\']) && !p.contains(':'));
    if pinned {
        Ok(Value::Boolean(true))
    } else {
        unsupported(format!(
            "File.IsPathTemporary of `{p}` (Windows and Linux BC differ for absolute, drive-letter and UNC paths)"
        ))
    }
}

/// `Download(FromFile, ...)` and `View(FromFile, ...)` send a file from the server to the client: the file is read
/// first, so the output of a report that was never produced is refused before the round trip is.
fn download_file(c: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    let name = path(a, 0)?;
    c.fs_exists(&name)?;
    unsupported("File.Download needs a client (a browser round trip), which a headless run has not")
}

fn view_file(c: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    let name = path(a, 0)?;
    c.fs_exists(&name)?;
    unsupported("File.View needs a client (a browser round trip), which a headless run has not")
}

/// `DownloadFromStream` and `ViewFromStream` send a stream to the client: the stream is read first.
fn download_stream(c: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    let _ = c;
    if let Some(stream) = a.first_mut() {
        crate::stream::refuse_pending_in(stream)?;
    }
    unsupported("File.DownloadFromStream needs a client (a browser round trip), which a headless run has not")
}

/// A call that needs a client on the other side of the browser.
macro_rules! client_round_trip {
    ($name:literal) => {
        |_, _, _| {
            unsupported(concat!(
                $name,
                " needs a client (a browser round trip), which a headless run has not"
            ))
        }
    };
}

pub fn native_for(sig: &Signature) -> Option<NativeFn> {
    if sig.receiver != "File" {
        return None;
    }
    let f: NativeFn = match sig.name.as_str() {
        "Create" => create,
        "Open" => open,
        "CreateTempFile" => create_temp,
        "Close" => close,
        "Write" => write,
        "Len" => len,
        "Name" => name,
        "CreateOutStream" => create_out_stream,
        "CreateInStream" => create_in_stream,
        "TextMode" => text_mode,
        "WriteMode" => write_mode,
        "Exists" => exists,
        "Erase" => erase,
        "Copy" => copy,
        "Rename" => rename,
        "IsPathTemporary" => is_path_temporary,
        "Upload" => client_round_trip!("File.Upload"),
        "UploadIntoStream" => client_round_trip!("File.UploadIntoStream"),
        "Download" => download_file,
        "DownloadFromStream" => download_stream,
        "View" => view_file,
        "ViewFromStream" => download_stream,
        _ => return None,
    };
    Some(f)
}
