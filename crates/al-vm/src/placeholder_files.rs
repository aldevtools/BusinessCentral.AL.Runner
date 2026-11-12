//! How a file that holds a placeholder report output came to hold it, and whether an AL File holds it open: the facts the
//! DotNet `System.IO.File.Delete` of such a file needs (assumption dotnet-file-delete-placeholder). The marks themselves are
//! `Session::render_pending_files` (`ctx.rs`).

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use al_builtins::assumptions::{Assumption, assume};
use al_builtins::native::Signal;
use al_builtins::placeholder_output as po;

use crate::Session;
use crate::ctx::pending_key;

/// Per session, by the path key of `ctx::pending_key`.
#[derive(Debug, Default)]
pub(crate) struct PlaceholderFiles {
    /// Files whose placeholder an AL File wrote on `Close` from a stream copy (File Management.TempBlobToServerFile).
    streamed: HashSet<String>,
    /// AL Files open on a placeholder file and not closed yet, per path.
    open: HashMap<String, usize>,
}

impl PlaceholderFiles {
    pub(crate) fn note_streamed(&mut self, path: &str) {
        self.streamed.insert(pending_key(path));
    }
    /// The file at `path` holds something else now, or a rendering save wrote the placeholder itself.
    pub(crate) fn forget_streamed(&mut self, path: &str) {
        if !self.streamed.is_empty() {
            self.streamed.remove(&pending_key(path));
        }
    }
    pub(crate) fn note_open(&mut self, path: &str) {
        *self.open.entry(pending_key(path)).or_default() += 1;
    }
    pub(crate) fn note_closed(&mut self, path: &str) {
        if let Some(n) = self.open.get_mut(&pending_key(path)) {
            *n = n.saturating_sub(1);
        }
    }
    fn held_open(&self, path: &str) -> bool {
        self.open.get(&pending_key(path)).is_some_and(|&n| n > 0)
    }
}

impl Session {
    /// DotNet `System.IO.File.Delete` of a file that holds a placeholder report output: `Ok(true)` when it was one and the
    /// delete may go on, `Ok(false)` when the path holds none. The measured shape (HtmlServerFileRoundTrip.al, corpus probe
    /// pull request 1368: Html and Pdf, written to the file by `File.Close` from a stream copy, the file closed again, then
    /// DeleteServerFile) runs under assumption dotnet-file-delete-placeholder; every other file holding a placeholder (a Word
    /// or Excel one, one a rendering save wrote itself, one an AL File holds open) refuses.
    pub(crate) fn dotnet_delete_placeholder_file(&mut self, path: &str) -> Result<bool, Signal> {
        let Some(mark) = self.placeholder_file_mark(path) else {
            return Ok(false);
        };
        let measured = (po::is_html(&mark) || po::is_pdf(&mark))
            && self.placeholder_files.streamed.contains(&pending_key(path))
            && !self.placeholder_files.held_open(path);
        if !measured {
            return Err(refusal(&mark));
        }
        assume(Assumption::DotNetFileDeletePlaceholder);
        self.placeholder_files.forget_streamed(path);
        Ok(self.forget_placeholder_file(path))
    }
}

fn refusal(mark: &Arc<str>) -> Signal {
    Signal::unsupported(format!(
        "DotNet System.IO.File.Delete of a file holding {mark}: only an Html or Pdf placeholder written to the file through a \
         stream and closed is measured (not a Word or Excel one, not a file the rendering save wrote itself, not one an AL File \
         holds open)"
    ))
}
