//! The DotNet `System.IO.File.Delete` of a file that holds a placeholder report output (assumption
//! dotnet-file-delete-placeholder), and the session facts about a path that its gate needs. The marks themselves are
//! `Session::render_pending_files` (`ctx.rs`).
//!
//! The delete runs for one call sequence only, the one corpus probe pull request 1368 measures
//! (al-language-onprem/reports/layouts-saveas/HtmlServerFileRoundTrip.al; lines of commit 06fa058c):
//!
//! 1. `Report.SaveAs` of a report whose default layout is a Word layout, as Html or Pdf, into a stream (:51, :85, :164, :225);
//! 2. the export, File Management.InstreamExportToServerFile or BLOBExportToServerFile on a path that has no file (:160,
//!    :201-204; by hand :314-318): `WriteMode(true)`, `Create(Name)`, one `CreateOutStream`, `CopyStream`, `Close`;
//! 3. the import, File Management.BLOBImportFromServerFile (:139; by hand :322-326): `Open(Name)`, one `CreateInStream`,
//!    `CopyStream`, `Close`;
//! 4. File Management.DeleteServerFile (:149): `Exists`, then the DotNet delete.
//!
//! [`Session::dotnet_delete_placeholder_file`] is the one gate. It requires each fact by equality; every other file that
//! holds a placeholder refuses there, as on main.
//!
//! Every call that reaches a fact kept here:
//!
//! | Call | `chain` of the path | `open` of the path |
//! |---|---|---|
//! | `File.Create` | ended (`Ctx::fs_write` empties the file) | +1 (`Ctx::fs_opened`) |
//! | `File.Open` | kept | +1 (`Ctx::fs_opened`) |
//! | `File.CreateTempFile` | none: a temporary file is not on the host file system | none |
//! | `File.CreateOutStream`, `CreateInStream` | none; counted in the handle (`al_builtins::file::FileHandle`) | none |
//! | `File.WriteMode`, `TextMode` | none; read from the handle at its `Close` | none |
//! | `File.Close` of a writable handle | ended by its write, then `Exported` when the handle is [`EXPORT`] and the file holds a placeholder | -1 |
//! | `File.Close` of a read-only handle | `Exported` to `Imported` when the handle is [`IMPORT`]; any other handle, or a second import, ends it | -1 |
//! | `File.Erase`, `File.Rename` (its remove) | ended (`Ctx::fs_remove`) | kept |
//! | `File.Copy`, `File.Rename` onto the path | ended (`Ctx::fs_write`); from a placeholder file both stop at the read, as on main | kept |
//! | a rendering save, a DotNet document save, the request page's SaveAsXml files to the path | ended ([`PlaceholderFiles::written`]) | kept |
//! | the measured DotNet delete | ended | kept (it is 0) |
//! | a File variable that leaves scope while open | kept | kept: the path counts as held for the rest of the session |
//! | `Clear` of a File variable | refused before this module (`Clear of a File value`) | |
//! | end of the test | kept, as the files and their marks are: the facts live as long as the session's file system | kept |
//!
//! A path that stays counted as held only refuses a later DotNet delete of a placeholder file at that path.

use std::collections::HashMap;
use std::sync::Arc;

use al_builtins::assumptions::{Assumption, assume};
use al_builtins::file::FileHandle;
use al_builtins::native::Signal;
use al_builtins::placeholder_output as po;

use crate::Session;
use crate::ctx::pending_key;
use crate::report_placeholder::Format;

/// The handle of File Management.InstreamExportToServerFile and BLOBExportToServerFile at its `Close`
/// (HtmlServerFileRoundTrip.al:160, :204, :314-318).
const EXPORT: FileHandle = FileHandle {
    created: true,
    new_file: true,
    encoding_given: false,
    write_mode: true,
    text_mode: false,
    out_streams: 1,
    in_streams: 0,
};

/// The handle of File Management.BLOBImportFromServerFile at its `Close` (HtmlServerFileRoundTrip.al:139, :322-326). Whether
/// the stream was read is not a fact of the handle.
const IMPORT: FileHandle = FileHandle {
    created: false,
    new_file: false,
    encoding_given: false,
    write_mode: false,
    text_mode: false,
    out_streams: 0,
    in_streams: 1,
};

/// How far the measured sequence has come on a path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Chain {
    /// An [`EXPORT`] handle wrote a placeholder to the file and closed; nothing wrote the path since.
    Exported,
    /// Then one [`IMPORT`] handle opened and closed.
    Imported,
}

/// Per session. Paths by the key of `ctx::pending_key`.
#[derive(Debug, Default)]
pub(crate) struct PlaceholderFiles {
    chain: HashMap<String, Chain>,
    /// AL Files open on a path and not closed yet (every `File.Create` and `File.Open`, whatever the file holds).
    open: HashMap<String, usize>,
    /// Per placeholder mark: whether every rendering save of this session that gave the mark was of a report whose default
    /// layout is a Word layout. The mark names the report by name only, so two reports of one name are told apart here.
    word_layout: HashMap<Arc<str>, bool>,
}

impl PlaceholderFiles {
    /// A rendering save wrote a placeholder with `mark` for a report whose default layout is of `layout_kind`.
    pub(crate) fn saved(&mut self, mark: &Arc<str>, layout_kind: &str) {
        let word = layout_kind == "word";
        self.word_layout
            .entry(mark.clone())
            .and_modify(|w| *w &= word)
            .or_insert(word);
    }

    /// Something wrote or removed the file at `path`: the sequence ends there.
    pub(crate) fn written(&mut self, path: &str) {
        if !self.chain.is_empty() {
            self.chain.remove(&pending_key(path));
        }
    }

    pub(crate) fn opened(&mut self, path: &str) {
        *self.open.entry(pending_key(path)).or_default() += 1;
    }

    /// `File.Close` closed `handle` on `path`, after its write; `holds_placeholder` is whether the file holds a placeholder
    /// now.
    pub(crate) fn closed(&mut self, path: &str, handle: &FileHandle, holds_placeholder: bool) {
        let key = pending_key(path);
        if let Some(n) = self.open.get_mut(&key) {
            *n -= 1;
            if *n == 0 {
                self.open.remove(&key);
            }
        }
        let next = match self.chain.get(&key) {
            None if *handle == EXPORT && holds_placeholder => Some(Chain::Exported),
            Some(Chain::Exported) if *handle == IMPORT => Some(Chain::Imported),
            _ => None,
        };
        match next {
            Some(step) => self.chain.insert(key, step),
            None => self.chain.remove(&key),
        };
    }
}

impl Session {
    /// DotNet `System.IO.File.Delete` of a file that holds a placeholder report output: `Ok(true)` when it was one and the
    /// delete may go on, `Ok(false)` when the path holds none. Only the sequence of the module comment runs, under
    /// assumption dotnet-file-delete-placeholder; every other file holding a placeholder refuses.
    pub(crate) fn dotnet_delete_placeholder_file(&mut self, path: &str) -> Result<bool, Signal> {
        let Some(mark) = self.placeholder_file_mark(path) else {
            return Ok(false);
        };
        let key = pending_key(path);
        // Html and Pdf (HtmlServerFileRoundTrip.al:164, :225), and the bytes the stream copy alone leaves in the file.
        let format = match () {
            () if po::is_html(&mark) => Some(Format::Html),
            () if po::is_pdf(&mark) => Some(Format::Pdf),
            () => None,
        };
        let measured = format.is_some_and(|f| self.host.files.read(path) == Some(f.bytes()))
            // The report's default layout is a Word layout (:51).
            && self.placeholder_files.word_layout.get(&mark) == Some(&true)
            // One export to a new file, then one import (:160, :139).
            && self.placeholder_files.chain.get(&key) == Some(&Chain::Imported)
            // Every AL File that opened the path closed it (:318, :326).
            && !self.placeholder_files.open.contains_key(&key);
        if !measured {
            return Err(refusal(&mark));
        }
        assume(Assumption::DotNetFileDeletePlaceholder);
        self.placeholder_files.written(path);
        Ok(self.forget_placeholder_file(path))
    }
}

fn refusal(mark: &Arc<str>) -> Signal {
    Signal::unsupported(format!(
        "DotNet System.IO.File.Delete of a file holding {mark}: only the Html or Pdf of a Word layout report that one File \
         wrote to a new file from a stream copy and one File read back, both closed, is measured"
    ))
}
