//! A save as Html while Base Application's "Design-time Report Selection" holds a temporary layout selection (ADR-040
//! amendment 2026-11-12, A7). `Report Selections`.`SaveReportAsHTMLImpl` sets one before every e-mail body save
//! (`Report Layout Selection`.`SetTempLayoutSelectedName`), and BC asks the layout selection events, which Base Application's
//! ReportManagement answers from it. Here SelectReportLayoutCode (`al_ir::REPORT_MANAGEMENT_LAYOUT_EVENTS`) is raised to
//! ReportManagement's subscriber only, so Microsoft's selection code runs as AL (it clears the selection), with the entry values
//! events/platform-triggers/PeVTests.al:721 pins. When it answers a selected layout BC raises no SelectedBuiltinLayoutType
//! (corpus probe pull request 1362, reports/layouts-html-text/HtRenderedTextTests.al:494 a declared layout by name, :530 a
//! user-defined row named by a GUID: `SubstituteReport(SaveAs);SelectReportLayoutCode;OnPreReport;...`, the selection empty after
//! the save at :537-538). When the answer names exactly one user-defined Word layout of the report (a Tenant Report Layout row),
//! the save writes the Html placeholder of that layout.
//!
//! The pinned corpus does not reach it yet: it runs under the second exception of CLAUDE.md rule 3 (al-runner issue 6924: the
//! reporter's tests pass on BC and need the save to use the selected Word layout), with the probe above open, and marks
//! `report-layout-temp-selection` on every run. The allow-list is [`Session::temp_selection_entry`] and the checks of
//! [`Session::temp_selected_layout`]; every other selection keeps the refusal of `Session::layout_selection`.

use std::sync::Arc;

use super::{Format, REPORT_LAYOUT_SELECTION, TENANT_REPORT_LAYOUT_SELECTION};
use crate::reports::{SaveTarget, Schema};
use crate::tenant_layouts::{TENANT_REPORT_LAYOUT, WORD};
use crate::{Session, Signal};
use al_builtins::value::Value;

/// The members of the events' `LayoutType` option (codeunit Reporting Triggers).
const LAYOUT_TYPES: [&str; 5] = ["None", "RDLC", "Word", "Excel", "Custom"];
/// `LayoutType::None`: the entry value of SelectReportLayoutCode (PeVTests.al:721 `in=/None/No`).
const NONE: i32 = 0;

fn layout_type(n: i32) -> Value {
    Value::Option(n, LAYOUT_TYPES.iter().map(|m| m.to_string()).collect())
}

fn ordinal(v: &Value) -> Option<i32> {
    match v {
        Value::Option(n, _) | Value::Integer(n) => Some(*n),
        _ => None,
    }
}

fn text(v: &Value) -> Option<&str> {
    match v {
        Value::Text(s) | Value::Code(s) if !s.has_marker() => Some(s.as_str()),
        _ => None,
    }
}

impl Session {
    /// The entry of A7: a save as Html to a stream of a report whose default layout is Word and that no report extension
    /// extends, while the "Design-time Report Selection" of the app with Base Application's id holds a selection.
    fn temp_selection_entry(&self, schema: &Schema, format: Format, target: &SaveTarget) -> bool {
        let report = schema.report();
        let program = &self.engine.program;
        format == Format::Html
            && matches!(target, SaveTarget::Stream(_))
            && report.default_layout_kind == "word"
            && report.extensions.is_empty()
            && self.design_time_selection().is_some_and(|oi| {
                program
                    .apps
                    .get(program.objects.heads()[oi].app as usize)
                    .is_some_and(|a| a.id.eq_ignore_ascii_case(al_ir::BASE_APPLICATION_APP_ID))
            })
    }

    /// The name of the user-defined Word layout a temporary selection names, when the save is the entry of A7
    /// ([`Session::temp_selection_entry`]); `None` outside it. Raises SelectReportLayoutCode to Base Application's
    /// ReportManagement only (with LayoutCode `''`, LayoutType None and Success false on entry, PeVTests.al:721), and refuses
    /// unless it answers Success with a name and LayoutType None (no Custom Report Layout row has it, and no app id is appended),
    /// no Tenant Report Layout Selection or Report Layout Selection row names the report, and the name is that of exactly one Tenant Report Layout row of the
    /// report: Layout Format Word, a blank Company Name, the null App ID, and no row or declared layout whose name differs only
    /// in case. A subscriber the test can see to a layout event this does not raise refuses as for any placeholder.
    pub(super) fn temp_selected_layout(
        &mut self,
        schema: &Schema,
        format: Format,
        target: &SaveTarget,
        what: &str,
    ) -> Result<Option<Arc<str>>, Signal> {
        if !self.temp_selection_entry(schema, format, target) {
            return Ok(None);
        }
        let refuse = |why: &str| {
            Err(Signal::unsupported(format!(
                "{what} of report \"{}\" while Design-time Report Selection holds a selected layout: {why} (only a \
                 user-defined Word layout of the report, selected by its name without an app id, is resolved)",
                schema.name()
            )))
        };
        let Some(select) = self.report_management_subscriber("selectreportlayoutcode") else {
            return refuse("Base Application's ReportManagement subscriber to SelectReportLayoutCode is not compiled");
        };
        // The save goes on only when SelectReportLayoutCode answers a selected layout, and then BC raises no
        // SelectedBuiltinLayoutType (HtRenderedTextTests.al:494, :530): its subscribers do not refuse here.
        self.refuse_layout_event_subscribers(schema, what, &["selectedbuiltinlayouttype"], Some(select.0))?;
        let id = Value::Integer(self.object_id_of(schema.object as u32) as i32);
        let mut args = [id, Value::Text("".into()), layout_type(NONE), Value::Boolean(false)];
        let names = ["ObjectId", "LayoutCode", "LayoutType", "Success"];
        self.raise_to_report_management("selectreportlayoutcode", select, &names, &mut args)?;
        let name = match (text(&args[1]), ordinal(&args[2]), &args[3]) {
            (Some(name), Some(NONE), Value::Boolean(true)) if !name.is_empty() => name.to_string(),
            _ => return refuse("SelectReportLayoutCode did not answer a layout name with Success and LayoutType None"),
        };
        if name.contains("::") {
            return refuse("the selection names an app id");
        }
        if let Some(why) = self.user_word_layout(schema, &name)? {
            return refuse(&why);
        }
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::ReportLayoutTempSelection);
        Ok(Some(name.into()))
    }

    /// The one subscriber the registry holds for `event` of Reporting Triggers in `al_ir::REPORT_MANAGEMENT_LAYOUT_EVENTS`
    /// (it registers Base Application's ReportManagement only, `events::Registry::build`).
    fn report_management_subscriber(&self, event: &str) -> Option<(usize, usize)> {
        let subs = self
            .engine
            .events
            .subscribers
            .get(&(crate::events::REPORTING_TRIGGERS, event.to_string()))?;
        match subs.as_slice() {
            [s] if !s.manual => Some((s.object, s.routine)),
            _ => None,
        }
    }

    /// Raises `event` of Reporting Triggers to subscriber `to` alone.
    fn raise_to_report_management(
        &mut self,
        event: &'static str,
        to: (usize, usize),
        names: &[&str],
        args: &mut [Value],
    ) -> Result<(), Signal> {
        let names: Vec<String> = names.iter().map(|n| n.to_string()).collect();
        let call = crate::dispatch::EventCall {
            object: crate::events::REPORTING_TRIGGERS,
            event,
            element: None,
            isolated: false,
            names: &names,
            sender: None,
        };
        self.events.only_targets = Some(vec![to]);
        let raised = self.raise(&call, args);
        self.events.only_targets = None;
        raised
    }

    /// Why `name` does not name exactly one user-defined Word layout of the report for every company, without an app id, while
    /// no Tenant Report Layout Selection or Report Layout Selection row names the report; `None` when it does.
    fn user_word_layout(&self, schema: &Schema, name: &str) -> Result<Option<String>, Signal> {
        let id = self.object_id_of(schema.object as u32) as i32;
        let cx = self.cx.clone();
        let of_report = |t: &al_data::TableDef, row: &al_data::Row| {
            t.fields
                .iter()
                .position(|f| f.name.eq_ignore_ascii_case("Report ID"))
                .is_some_and(|r| matches!(row[r], Value::Integer(n) if n == id))
        };
        // A selection row of either table is not measured together with a temporary selection.
        for (table, what) in [
            (
                TENANT_REPORT_LAYOUT_SELECTION,
                "a Tenant Report Layout Selection row names the report",
            ),
            (
                REPORT_LAYOUT_SELECTION,
                "a Report Layout Selection row names the report",
            ),
        ] {
            if let Some(t) = self.table_def(table) {
                if self.all_rows(&cx, &t)?.iter().any(|row| of_report(&t, row)) {
                    return Ok(Some(what.into()));
                }
            }
        }
        let Some(t) = self.table_def(TENANT_REPORT_LAYOUT) else {
            return Ok(Some("the program holds no Tenant Report Layout table".into()));
        };
        let col = |n: &str| t.fields.iter().position(|f| f.name.eq_ignore_ascii_case(n));
        let (Some(n), Some(app), Some(company), Some(kind)) =
            (col("Name"), col("App ID"), col("Company Name"), col("Layout Format"))
        else {
            return Ok(Some(
                "the Tenant Report Layout table lacks a field the check reads".into(),
            ));
        };
        let rows: Vec<_> = self
            .all_rows(&cx, &t)?
            .into_iter()
            .filter(|row| of_report(&t, row) && text(&row[n]).is_some_and(|s| s.eq_ignore_ascii_case(name)))
            .collect();
        let declared = schema
            .report()
            .rendering_layouts
            .iter()
            .any(|l| l.name.eq_ignore_ascii_case(name));
        Ok(match rows.as_slice() {
            [row]
                if !declared
                    && text(&row[n]) == Some(name)
                    && text(&row[company]) == Some("")
                    && matches!(row[app], Value::Guid(g) if g == [0; 16])
                    && ordinal(&row[kind]) == Some(WORD) =>
            {
                None
            }
            [] => Some(format!("no Tenant Report Layout row of the report is named \"{name}\"")),
            _ => Some(format!(
                "the Tenant Report Layout row named \"{name}\" is not one Word layout for every company without an app id"
            )),
        })
    }
}
