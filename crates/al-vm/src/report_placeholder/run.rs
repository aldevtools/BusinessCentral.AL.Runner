//! `Report.Run`, `RunModal` and `Execute` of a report that renders output, in a test session (ADR-040 amendment of
//! 2026-11-11): the report runs as on the Xml path (request page, triggers, data items) and writes no output.
//!
//! - RDLC or Word default layout: the run goes on and answers as BC does (reports/runtime/LtTkShowPrintStatus.al:960-984
//!   Layout_T_Run, _RunModal, _Execute and :1009 Layout_F_*: OnPreReport, the rows, OnPostReport, Yes). BC renders and raises
//!   the layout and printer events there (events/platform-triggers/PeVTests.al:740-763, Windows values); this path raises none
//!   of them, so a subscriber the test can see refuses the run. Assumption `report-run-without-output`.
//! - Excel default layout: BC refuses the operation before OnPreReport (reports/CiReportRunLeads.al:236-260
//!   ExcelOnly_Run_RunModal_Execute_Print_SaveAs: `ReportUnsupportedOperation`, no trigger logged). Pinned for a report
//!   without OnInitReport, request page or reportextension; with one of them, whether it runs first is not pinned.
//! - Any other default layout (Custom, none that can be told, a missing layout file) keeps the refusal.
//! - Before any of these: a subscriber the test can see to a layout or printer event, a layout selection, and a run that would
//!   print (SystemPrinter `true`, or a report that declares `UseSystemPrinter = true`) refuse the run. Printing is not part of
//!   the amendment ("What stays refused"), and BC may run an Excel layout report a selection points at a Word layout.

use crate::reports::Schema;
use crate::{Session, Signal, Value};
use al_builtins::native::{AlError, unsupported};

/// The layout and printer events BC raises around the render of a Run, RunModal or Execute (PeVTests.al:740-763) that this
/// runtime does not raise. SubstituteReport is raised by the static forms already.
const RUN_EVENTS: &[&str] = &[
    "getprintername",
    "getpapertrayforreport",
    "setupprinters",
    "ondocumentdownload",
];

impl Session {
    /// Called by a Run, RunModal or Execute of a report that renders (`ir::Report::output` not empty) once its request page
    /// closed with OK or was not shown: `Ok` goes on with the triggers and writes nothing; otherwise the pinned error or a
    /// refusal naming `why` (the first reason the report renders).
    pub(crate) fn run_without_output(&self, schema: &Schema, why: &str) -> Result<(), Signal> {
        let report = schema.report();
        let refused = || {
            unsupported(format!(
                "report \"{}\" renders output, which the runtime does not produce: {why}",
                schema.name()
            ))
        };
        if report.processing_only || report.layout_file_missing {
            return refused();
        }
        if !matches!(report.default_layout_kind.as_str(), "rdlc" | "word" | "excel") {
            return refused();
        }
        self.refuse_unraised_layout_events(schema, "a run", &["substitutereport"], None)?;
        if let Some(who) = self.run_event_subscriber() {
            return unsupported(format!(
                "a run of report \"{}\" with {who} (BC raises the printer and download events around a render; this runtime \
                 renders nothing and raises none of them)",
                schema.name()
            ));
        }
        let declared = self.object_property(schema.object(), "UseSystemPrinter");
        if self.report.system_printer || declared.is_some_and(|v| v.trim().eq_ignore_ascii_case("true")) {
            return unsupported(format!(
                "a run of report \"{}\" on the system printer (printing is not produced; ADR-040 amendment 2026-11-11)",
                schema.name()
            ));
        }
        match report.default_layout_kind.as_str() {
            "rdlc" | "word" => {
                al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::ReportRunWithoutOutput);
                Ok(())
            }
            "excel" if report.on_init.is_none() && !report.has_request_page && report.extensions.is_empty() => {
                Err(Signal::Error(Box::new(AlError {
                    message: "The current report operation is not supported for Excel Layout reports.".into(),
                    code: "ReportUnsupportedOperation".into(),
                    call_stack: String::new(),
                    ..Default::default()
                })))
            }
            _ => refused(),
        }
    }

    /// Runs `run` as a report started by `RunModal` (`modal`) and with the SystemPrinter argument `printer`, and restores the
    /// state of the run around it afterwards (a report run from a trigger of another).
    pub(crate) fn run_started_as(
        &mut self,
        modal: bool,
        printer: Option<&Value>,
        run: impl FnOnce(&mut Self) -> Result<(), Signal>,
    ) -> Result<(), Signal> {
        let outer = std::mem::replace(&mut self.report.run_modal, modal);
        let printer = matches!(printer, Some(Value::Boolean(true)));
        let outer_printer = std::mem::replace(&mut self.report.system_printer, printer);
        let ran = run(self);
        (self.report.run_modal, self.report.system_printer) = (outer, outer_printer);
        ran
    }

    /// The first subscriber the test can see to an event of [`RUN_EVENTS`], as text for a refusal.
    fn run_event_subscriber(&self) -> Option<String> {
        let program = &self.engine.program;
        let test = self.test_object.and_then(|t| program.objects.get(t));
        RUN_EVENTS.iter().find_map(|e| {
            self.engine
                .events
                .unraised_subscriber("reporting triggers", Some(e), test, self.test_object)
                .map(|(_, sub)| format!("a subscriber to {e} ({sub})"))
        })
    }
}
