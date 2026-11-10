//! `Report.SaveAsHtml(Number, FileName, Record)`: decided before the report runs.

use super::*;

impl Session {
    /// Whether a static Html save with a record argument has the one shape the corpus pins: a report with an RDLC layout
    /// (its file present, no custom layout), not processing-only, the answer used. It answers false with
    /// `ReportUnsupportedFormat` and writes no file (al-language-onprem/reports/layouts-saveas/AMRdlcSaveAs.al:454-469
    /// `static html filtered`, the call at :330). Every other shape is refused here, before anything runs, because what
    /// the record filters is not pinned.
    pub(super) fn html_record_save_is_pinned(&mut self, id: al_ir::BuiltinId, args: &[Value]) -> Result<bool, Signal> {
        if !self.ret_used {
            return Ok(false);
        }
        let (_, schema) = self.report_by_number(id, args.first())?;
        let report = schema.report();
        Ok(report.layout == ir::ReportLayout::Rdlc
            && !report.layout_file_missing
            && !report.processing_only
            && self.custom_layout(&schema)?.is_none())
    }
}
