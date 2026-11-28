//! An AL error that leaves the run of a `Report.SaveAs` (moved out of `report_save_core`): whether the SaveAs raises it or
//! answers false, and what the last error holds then. One rule (corpus pull request 1579, issue 8486): SaveAs never swallows
//! the error. The statement form raises it; the Boolean form answers false and holds it as the last error. Each shape it
//! runs for is pinned or under the assumption `saveas-trigger-error-reaches-caller`; [`Session::trigger_error_cover`] is the
//! one place that names them. Every other shape keeps main's answer or main's refusal.

use super::*;
use crate::report_placeholder::SaveOutput;
use crate::report_save::{self as save, DataItems, Extensions, Kind, ReportFacts, Selection, Stored};
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
    /// What the save table planned before the run (`static_or_variable_save`): for Pdf and Word its fact vector.
    pub output: &'a SaveOutput,
    /// The store's change counter when the run started ([`Session::start_save_run`]).
    pub changes: u64,
}

impl<'a> FailedSave<'a> {
    /// The call: `at` 0 is a report variable; the target, the parameters string and the RecordRef.
    pub(super) fn of(
        format: &'a str,
        at: usize,
        (target, parameters, record_ref): (&SaveTarget, &al_builtins::text::Txt, Option<&Value>),
        result_used: bool,
    ) -> Self {
        FailedSave {
            format,
            by_variable: at == 0,
            plain_stream: matches!(target, SaveTarget::Stream(_)) && parameters.is_empty() && record_ref.is_none(),
            result_used,
            output: &SaveOutput::Dataset,
            changes: 0,
        }
    }

    /// With what the save table planned and the change counter at the start of the run.
    pub(super) fn after(self, output: &'a SaveOutput, changes: u64) -> Self {
        FailedSave {
            output,
            changes,
            ..self
        }
    }
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
    /// Called before the run of a SaveAs: no report trigger has failed yet. Returns the store's change counter, which
    /// [`FailedSave::changes`] carries.
    pub(super) fn start_save_run(&mut self) -> u64 {
        self.report.failed_trigger = None;
        self.store.change_counter()
    }

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
                if std::mem::take(&mut self.report.zero_date_view_failed) {
                    self.last_error = *e;
                } else if cover == Cover::Pinned {
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
            // (streams/TestReportSaveAsRecordRefFilter.al:TestReport_SaveAs_NoRecordRef_LeavesDataItemUnfiltered). That
            // the last error then holds the error is measured by corpus pull request 1579 (the SaveAs_Rdlc_* tests): the
            // shapes of those cells mark the assumption, every other shape keeps this answer unmarked.
            (ir::ReportLayout::Rdlc, false) => {
                if cover == Cover::Assumed {
                    assumptions::assume(Assumption::SaveasTriggerErrorReachesCaller);
                }
                if !call.result_used {
                    return Err(Signal::Error(e));
                }
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

    /// Which measurement covers an error that left a SaveAs from the report trigger `site` (`failed_trigger`). Each fact is
    /// read by equality with the cells; anything else is `Cover::Not`. The report's facts come from the save table's owner
    /// (`report_save::report_facts_of`). For Pdf and Word the session's facts (layout selection, stored row, custom layout,
    /// allow-list, company build, the call) come from the fact vector the save table computed before the run. For Xml only
    /// the company build and the allow-list are read: the layout selection, stored row and custom layout facts are not read
    /// and not measured (an Xml save writes no layout).
    ///
    /// Assumed (corpus pull request 1579, reports/layouts-saveas/SaveAsTriggerErrors.al, issue 8486):
    /// - the error and its trigger as the cells pair them (`Mode.Fire` :113-130, called from each trigger): `Error('literal')`
    ///   (E1, E2, E3) in OnPreReport, OnAfterGetRecord or OnPostReport, and the TestField of an empty field in OnPreDataItem;
    ///   the same error raised in the trigger body itself, not through a procedure, is not measured and taken as the same;
    /// - the run wrote no row to the database before the error;
    /// - the static `Report.SaveAs(Id, '', Format, OutStream)` as Pdf, Word or Xml, as a statement or in the Boolean form
    ///   ("SAE Cells" :377-424), not the company build and not an allow-listed report, and for Pdf and Word with no layout
    ///   selection row and no custom or stored layout;
    /// - the report: not processing-only, one data item, no OnInitReport and no request page trigger (`init_triggers`),
    ///   exactly one layout, in its rendering section and named by DefaultRenderingLayout, and no extension layout.
    ///   Word ("SAE Word Rpt" :132, codeunit 77388 SaveAs_Word_*); RDLC ("SAE Rdlc Rpt", SaveAs_Rdlc_*: that arm keeps
    ///   main's answer and only marks it); Word or RDLC with one report extension that only adds a column ("SAE Word
    ///   Col Ext", "SAE Rdlc Col Ext", codeunit 77395); or, as Xml only, Excel with no extension ("SAE Excel Rpt",
    ///   SaveAs_Excel_*: as Pdf and Word the platform's error comes first, `cells_platform_first`); or, as Xml only, no
    ///   layout of its own and one report extension that only adds a Word layout ("SAE No Layout Rpt",
    ///   SaveAs_NoLayoutExtWord_*).
    ///
    /// Pinned: text/PnBReportSaveAs.al:698 PW_TriggerError_InPreReport_LastError, `Error('literal')` in OnPreReport of "LT PnB
    /// Rep" (errors/PnBObjects.al): no layout and no extension, two top-level data items and one inside the first, the used
    /// `Rep.SaveAs('', Xml, OutStream)` on a variable, no row written. Not read: the pinned report's OnInitReport and request
    /// page, and writes before the report body (the change counter starts at the run); none of them changes what the last
    /// error holds.
    fn trigger_error_cover(
        &self,
        e: &AlError,
        site: Option<(usize, TrigKind)>,
        schema: &Schema,
        call: &FailedSave,
    ) -> Cover {
        let Some((_, kind)) = site.filter(|(object, _)| *object == schema.object) else {
            return Cover::Not;
        };
        let literal = e.code == "Dialog" && e.exception == ExceptionKind::Dialog;
        let measured_pair = match kind {
            TrigKind::Pre | TrigKind::Row | TrigKind::PostReport => literal,
            TrigKind::PreItem => crate::recmeta::is_empty_field_error(e),
            _ => false,
        };
        let wrote = self.store.change_counter() != call.changes;
        if !measured_pair || wrote || !call.plain_stream {
            return Cover::Not;
        }
        let r = save::report_facts_of(&self.engine.program, schema);
        if self.measured_trigger_error_save(&r, schema, call) {
            return Cover::Assumed;
        }
        let report = schema.report();
        let pin_items = matches!(report.items.as_slice(), [a, b, c]
            if a.parent.is_none() && b.parent == Some(0) && c.parent.is_none());
        let pinned = call.by_variable
            && call.format == "Xml"
            && kind == TrigKind::Pre
            && pin_items
            && report.extensions.is_empty()
            && report.layout == ir::ReportLayout::None
            && report.rendering_layouts.is_empty()
            && report.layout_properties.is_empty()
            && report.extension_layouts.is_empty();
        if pinned { Cover::Pinned } else { Cover::Not }
    }

    /// The call, session and report facts of the assumed cells (see [`Session::trigger_error_cover`]).
    fn measured_trigger_error_save(&self, r: &ReportFacts, schema: &Schema, call: &FailedSave) -> bool {
        let session = match call.output {
            SaveOutput::Rendered(plan) if matches!(call.format, "Pdf" | "Word") => {
                let f = plan.facts();
                f.call == save::Call::StaticPlain
                    && f.target == save::Target::Stream
                    && !f.custom
                    && !f.company_build
                    && !f.listed
                    && f.selection == Selection::None
                    && f.stored == Stored::None
            }
            SaveOutput::Dataset if call.format == "Xml" => !self.company_build && !self.output_is_ignored(schema),
            _ => false,
        };
        let report = !call.by_variable && !r.processing_only && r.data_items == DataItems::One && !r.init_triggers;
        let own_layout = r.sole_layout && r.decl == save::Decl::Rendering;
        let shape = match (r.kind, r.extensions) {
            (Kind::Word | Kind::Rdlc, Extensions::None | Extensions::OneAddingColumns) => own_layout,
            (Kind::Excel, Extensions::None) => own_layout && call.format == "Xml",
            // No layout of its own; the extension's Word layout is not the default ("SAE No Layout Rpt", SaveAs_NoLayoutExtWord_*:
            // as Pdf and Word ReportNoLayout comes first, `cells_platform_first`).
            (Kind::Other, Extensions::OneAddingWordLayout) => call.format == "Xml",
            _ => false,
        };
        session && report && shape
    }
}
