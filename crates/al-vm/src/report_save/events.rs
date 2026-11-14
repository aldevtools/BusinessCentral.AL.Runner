//! The layout events BC raises around a render that this runtime does not raise: a subscriber the test can see refuses the
//! save or run (the `events` column of a row, `cell::Events`).

use super::Schema;
use crate::{Session, Signal};

/// The events of codeunit "Reporting Triggers" BC raises around a render (reports/layouts-saveas, events/platform-
/// triggers/PeVTests.al:721 `saveas static html`: SubstituteReport, SelectReportLayoutCode, HasCustomLayout,
/// SelectedBuiltinLayoutType, ApplicationReportMergeStrategy; :750 OnDocumentReady after a render), and the ones
/// Base Application answers for a render (FetchReportLayoutByCode, OnIntermediateDocumentReady, GetCompanyMetadata).
/// This runtime raises none of them for a placeholder.
const LAYOUT_EVENTS: &[(&str, &str)] = &[
    ("substitutereport", "SubstituteReport"),
    ("selectreportlayoutcode", "SelectReportLayoutCode"),
    ("hascustomlayout", "HasCustomLayout"),
    ("selectedbuiltinlayouttype", "SelectedBuiltinLayoutType"),
    ("applicationreportmergestrategy", "ApplicationReportMergeStrategy"),
    ("ondocumentready", "OnDocumentReady"),
    ("onintermediatedocumentready", "OnIntermediateDocumentReady"),
    ("fetchreportlayoutbycode", "FetchReportLayoutByCode"),
    ("getcompanymetadata", "GetCompanyMetadata"),
];

/// The integration events Base Application's ReportManagement raises from its subscribers to [`LAYOUT_EVENTS`]: a
/// subscriber to one of them would see the call.
const REPORT_MANAGEMENT_EVENTS: &[&str] = &[
    "onaftersubstitutereport",
    "onselectreportlayoutcode",
    "onafterhascustomlayout",
    "onafterdocumentready",
    "onafterintermediatedocumentready",
    "onfetchreportlayoutbycode",
];

const REPORT_MANAGEMENT: &str = "ReportManagement";

impl Session {
    /// Refuses `what` while a subscriber the test can see listens to a layout event this runtime does not raise (`except`, and
    /// the subscribers of object `report_management` to `al_ir::REPORT_MANAGEMENT_LAYOUT_EVENTS`, which `fact_selection`
    /// raises).
    pub(crate) fn refuse_layout_event_subscribers(
        &self,
        schema: &Schema,
        what: &str,
        except: &[&str],
        report_management: Option<usize>,
    ) -> Result<(), Signal> {
        if let Some(who) = self.layout_event_subscriber(except, report_management) {
            return Err(Signal::unsupported(format!(
                "{what} of report \"{}\" with {who} (BC raises the layout events around a render; this runtime \
                 writes a placeholder or answers without a render and raises none of them)",
                schema.name()
            )));
        }
        Ok(())
    }

    /// The first subscriber to a layout event (other than those in `except`, and those of object `report_management` to the
    /// events `fact_selection` raises to it) that refuses a placeholder, as text for a refusal.
    fn layout_event_subscriber(&self, except: &[&str], report_management: Option<usize>) -> Option<String> {
        let events = &self.engine.events;
        let program = &self.engine.program;
        // A subscriber the test cannot see (it names neither the subscriber's codeunit nor a SingleInstance codeunit the
        // subscriber holds) cannot tell whether the event ran, as for CaptionClassTranslate (`Registry::unraised_subscriber`).
        let test = self.test_object.and_then(|t| program.objects.get(t));
        for (key, name) in LAYOUT_EVENTS.iter().filter(|(k, _)| !except.contains(k)) {
            let raised = al_ir::REPORT_MANAGEMENT_LAYOUT_EVENTS
                .iter()
                .any(|e| e.eq_ignore_ascii_case(key));
            let but = report_management.filter(|_| raised);
            if let Some((_, sub)) =
                events.unraised_subscriber_but("reporting triggers", Some(key), test, self.test_object, but)
            {
                return Some(format!("a subscriber to {name} ({sub})"));
            }
            let key = (crate::events::REPORTING_TRIGGERS, key.to_string());
            if let Some(m) = events.missing.get(&key).and_then(|list| {
                list.iter()
                    .find(|m| !m.subscriber.contains(&format!("\"{REPORT_MANAGEMENT}\"")))
            }) {
                return Some(format!("a subscriber to {name} ({})", m.subscriber));
            }
        }
        REPORT_MANAGEMENT_EVENTS
            .iter()
            .filter(|e| !except.contains(e))
            .find_map(|e| {
                events
                    .unraised_subscriber("reportmanagement", Some(e), test, self.test_object)
                    .map(|(_, sub)| format!("a subscriber to ReportManagement's {e} ({sub})"))
            })
    }
}
