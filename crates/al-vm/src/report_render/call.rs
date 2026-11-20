//! The facts of the `Report.SaveAs` call of the Html hash cells (pinned corpus,
//! reports/html-email-body/RxHtmlBodyFixtures.al:120-126 `TrySave`): `TempBlob.CreateOutStream(OutS, TextEncoding::UTF8)` on
//! a new Temp Blob, then `Ok := Report.SaveAs(ReportId, '', ReportFormat::Html, OutS)`, in a session of language 1033.

use al_builtins::stream::{BlobRef, Encoding};

/// What the call hands the save, as the gate compares it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct SaveCall {
    /// `Report.SaveAs(Number, ..)`, not the method of a report variable.
    pub(crate) by_number: bool,
    /// The parameters text is not empty.
    pub(crate) parameters: bool,
    /// A RecordRef argument follows the stream.
    pub(crate) record_ref: bool,
    pub(crate) stream: Stream,
    /// The session language (GlobalLanguage) when the call starts.
    pub(crate) language: i32,
}

/// The OutStream the save writes to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Stream {
    /// UTF-8, and its buffer holds no byte, was never written by a stream, holds no mark, and the stream writes at its start.
    NewUtf8,
    Other,
}

/// The call of every cell.
pub(crate) const MEASURED_CALL: SaveCall = SaveCall {
    by_number: true,
    parameters: false,
    record_ref: false,
    stream: Stream::NewUtf8,
    language: 1033,
};

impl crate::Session {
    /// The facts of a `Report.SaveAs` to the OutStream on `blob` with `encoding` (`parameters`: the parameters text is not
    /// empty), taken before anything of the save runs.
    pub(crate) fn save_call(
        &self,
        by_number: bool,
        parameters: bool,
        record_ref: bool,
        blob: &BlobRef,
        encoding: Encoding,
    ) -> SaveCall {
        let b = blob.borrow();
        let new = b.bytes.is_empty()
            && b.write_pos == 0
            && !b.written
            && b.render_pending.is_none()
            && !b.first_write_truncates
            && !b.db_row
            && !b.file
            && b.dotnet_view == 0;
        SaveCall {
            by_number,
            parameters,
            record_ref,
            stream: if new && encoding == Encoding::Utf8 {
                Stream::NewUtf8
            } else {
                Stream::Other
            },
            language: self.global_language(),
        }
    }
}

/// What the render gate takes from the save beyond its fact vector: the call, and the kinds of value the top-level columns
/// held (`crate::dataset::ColumnKinds`).
pub(crate) struct RenderInputs<'a> {
    pub(crate) call: SaveCall,
    pub(crate) kinds: &'a [(String, Option<crate::dataset::ValueKind>)],
}
