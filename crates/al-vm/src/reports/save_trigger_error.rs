//! An AL error that leaves the run of a `Report.SaveAs` (moved out of `report_save_core`): whether the SaveAs raises it or
//! answers false, and what the last error holds then. One rule (corpus pull request 1579, issue 8486): SaveAs never swallows
//! the error. The statement form raises it; the Boolean form answers false and holds it as the last error. Each shape it
//! runs for is pinned or under the assumption `saveas-trigger-error-reaches-caller`; [`Session::trigger_error_cover`] is the
//! one place that names them.

use super::*;
use al_builtins::assumptions::{self, Assumption};
use al_builtins::native::{AlError, ExceptionKind};

/// The SaveAs an error left.
pub(super) struct FailedSave<'a> {
    pub format: &'a str,
    /// `Rep.SaveAs(..)` on a report variable, not the static `Report.SaveAs(Id, ..)`.
    pub by_variable: bool,
    /// The target is a stream, the parameters string is empty and no RecordRef is given.
    pub plain_stream: bool,
    pub result_used: bool,
}

/// What measures the error of a SaveAs.
#[derive(PartialEq)]
enum Cover {
    /// text/PnBReportSaveAs.al:698 PW_TriggerError_InPreReport_LastError.
    Pinned,
    /// The cells of corpus pull request 1579 (`saveas-trigger-error-reaches-caller`).
    Assumed,
    Not,
}

impl Session {
    pub(super) fn save_trigger_error(
        &mut self,
        e: Box<AlError>,
        schema: &Schema,
        call: FailedSave<'_>,
    ) -> Result<Value, Signal> {
        let site = self.report.failed_trigger.take();
        let cover = self.trigger_error_cover(&e, site, schema, &call);
        let report = schema.report();
        match (report.layout, !report.extensions.is_empty()) {
            // With no layout the error reaches the caller
            // (reportextension/TestReportExtensionTriggers.al:SaveAsXml_ExtPreReportError_FailsTheSave).
            (ir::ReportLayout::None, _) if !call.result_used => Err(Signal::Error(e)),
            // With the answer used, a report without a layout answers false and raises nothing
            // (text/PnBReportSaveAs.al:412-413, TriggerError_InPreReport). The error of a DataItemTableView `const(0D)`
            // stays the last error (streams/CjDataset.al:190-211: `save err|code` Evaluate), and so does the error of
            // the shape `Cover::Pinned` names. For any other error what the last error holds is not pinned:
            // GetLastErrorText and GetLastErrorCode are refused.
            (ir::ReportLayout::None, false) => {
                if std::mem::take(&mut self.report.zero_date_view_failed) || cover == Cover::Pinned {
                    self.hold_error(*e)?;
                } else {
                    self.last_error = al_builtins::native::unpinned_error();
                }
                Ok(Value::Boolean(false))
            }
            // An error of a trigger reaches a SaveAs whose result is not used, also with a layout
            // (streams/TestReportCurrReportLanguage.al:TestReport_SettingCurrReportLanguageToZero_IsRejectedAsInvalid:
            // `asserterror Probe.SaveAs(..)` on a report with an RDLC layout). With the result used SaveAs answers false
            // and leaves the stream alone
            // (streams/TestReportSaveAsRecordRefFilter.al:TestReport_SaveAs_NoRecordRef_LeavesDataItemUnfiltered); that
            // the last error then holds the error is measured by corpus pull request 1579
            // (reports/layouts-saveas/SaveAsTriggerErrors.al, the SaveAs_Rdlc_* tests), not pinned yet.
            (ir::ReportLayout::Rdlc, false) if !call.result_used => Err(Signal::Error(e)),
            (ir::ReportLayout::Rdlc, false) => {
                self.hold_error(*e)?;
                Ok(Value::Boolean(false))
            }
            _ if cover == Cover::Assumed => {
                assumptions::assume(Assumption::SaveasTriggerErrorReachesCaller);
                if !call.result_used {
                    return Err(Signal::Error(e));
                }
                self.hold_error(*e)?;
                Ok(Value::Boolean(false))
            }
            _ => unsupported(format!(
                "an error in a SaveAs of report \"{}\", which has a layout of this kind or an extension \
                 (whether SaveAs raises it or answers false is not pinned): {}",
                schema.name(),
                e.message
            )),
        }
    }

    /// Which measurement covers an error that left a SaveAs from the report trigger `site` (`failed_trigger`). Every fact is
    /// read by equality with the cells; anything else is `Cover::Not`.
    ///
    /// Assumed (corpus pull request 1579, reports/layouts-saveas/SaveAsTriggerErrors.al, issue 8486): the error is
    /// `Error('literal')` (E1, E2, E3) or a TestField of an empty field (`Mode.Fire` :113-130), raised by the report's own
    /// OnPreReport, OnPreDataItem, OnAfterGetRecord or OnPostReport; the call is the static `Report.SaveAs(Id, '', Format,
    /// OutStream)` as Pdf, Word or Xml, as a statement or in the Boolean form ("SAE Cells" :362-424); the report is not
    /// processing-only, has one data item and one layout in its rendering section named by DefaultRenderingLayout: Word
    /// ("SAE Word Rpt" :149, codeunit 77388 SaveAs_Word_*), or Word or RDLC with one report extension that only adds a
    /// column ("SAE Word Col Ext", "SAE Rdlc Col Ext", codeunit 77395).
    ///
    /// Pinned: text/PnBReportSaveAs.al:698 PW_TriggerError_InPreReport_LastError, `Error('literal')` in OnPreReport of a
    /// report without a layout or an extension, `Rep.SaveAs('', Xml, OutStream)` on a variable, the answer used.
    fn trigger_error_cover(
        &self,
        e: &AlError,
        site: Option<(usize, TrigKind)>,
        schema: &Schema,
        call: &FailedSave,
    ) -> Cover {
        let report = schema.report();
        let literal = e.code == "Dialog" && e.exception == ExceptionKind::Dialog;
        let measured_error = literal || e.code == "TestField";
        let Some((_, kind)) = site.filter(|(object, _)| *object == schema.object) else {
            return Cover::Not;
        };
        let measured_trigger = matches!(
            kind,
            TrigKind::Pre | TrigKind::PreItem | TrigKind::Row | TrigKind::PostReport
        );
        if !measured_error || !measured_trigger || report.processing_only || !call.plain_stream {
            return Cover::Not;
        }
        let extension_layout = report
            .output
            .iter()
            .any(|r| r.starts_with("reportextension") && r.contains("(a layout)"));
        let own_layout = |kind: &str| {
            report.layout_properties.is_empty()
                && !extension_layout
                && matches!(report.rendering_layouts.as_slice(), [l] if l.kind == kind
                    && report.default_rendering_layout.as_deref().is_some_and(|d| d.eq_ignore_ascii_case(&l.name)))
        };
        let program = &self.engine.program;
        let column_extension =
            matches!(report.extensions.as_slice(), [x] if crate::report_save::adds_columns_only(program, *x));
        let assumed = !call.by_variable
            && report.items.len() == 1
            && matches!(call.format, "Pdf" | "Word" | "Xml")
            && ((own_layout("word") && (report.extensions.is_empty() || column_extension))
                || (own_layout("rdlc") && column_extension));
        if assumed {
            return Cover::Assumed;
        }
        let pinned = call.by_variable
            && call.format == "Xml"
            && literal
            && kind == TrigKind::Pre
            && report.extensions.is_empty()
            && report.layout == ir::ReportLayout::None
            && report.rendering_layouts.is_empty()
            && report.layout_properties.is_empty()
            && !extension_layout;
        if pinned { Cover::Pinned } else { Cover::Not }
    }
}
