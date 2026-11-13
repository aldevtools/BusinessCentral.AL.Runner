//! A rendering save while a stored Report Layout Selection row (Base Application table 9651) of the company names the report:
//! the two selections corpus pull request 1371 measured that change the answer (open; the same on the three BC majors the
//! corpus runs; reports/html-email-body/RxLayoutSelectionTests.al, report 80020 of RxLayoutSelFixtures.al:15-71: a Word default
//! layout `DefaultRenderingLayout` names, an RDLC layout in the same `rendering` section, no report extension).
//!
//! - Type "RDLC (built-in)" with an empty layout code: the RDLC layout renders. Html answers false with
//!   `ReportUnsupportedFormat` and no trigger runs; Pdf, Word and Excel render (:452 StoredRdlcBuiltin_HtmlPdf, :477
//!   StoredRdlcBuiltin_WordExcel; :1082 and :1116 the same after `SetTempLayoutSelected('')` with a RecordRef argument).
//! - Type "Custom Layout" with a code that names no layout: every format answers false with `ReportNoLayout` and no trigger
//!   runs (:746 StoredCustomLayoutUnknownCode_HtmlPdf, :771 StoredCustomLayoutUnknownCode_WordExcel).
//!
//! Both only with the answer used and a stream target, as measured. They run under the assumption
//! `report-stored-layout-selection` until the probe is pinned. A row of another company or with an empty Company Name is not
//! read (:795, :1022: the default layout renders), which `Session::layout_selection` already does. Every other stored
//! selection keeps what `Session::layout_selection` answers for it.

use super::{REPORT_LAYOUT_SELECTION, TENANT_REPORT_LAYOUT_SELECTION};
use crate::reports::{SaveTarget, Schema};
use crate::tenant_layouts::TENANT_REPORT_LAYOUT;
use crate::{Session, Signal};
use al_builtins::value::Value;

/// Base Application's table "Custom Report Layout".
const CUSTOM_REPORT_LAYOUT: u32 = 9650;
/// Members of "Report Layout Selection".Type (ReportLayoutSelection.Table.al, Base Application of the three majors).
const RDLC_BUILT_IN: i32 = 0;
const CUSTOM_LAYOUT: i32 = 2;

/// What a measured stored selection makes of a save.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Stored {
    /// The report's RDLC layout renders.
    Rdlc,
    /// No layout: the save answers false with `ReportNoLayout`.
    NoLayout,
}

fn text(v: &Value) -> Option<&str> {
    match v {
        Value::Text(s) | Value::Code(s) if !s.has_marker() => Some(s.as_str()),
        _ => None,
    }
}

impl Session {
    /// The measured stored selection of the module's list that is in force for a save of `schema`, `None` when the save is
    /// not exactly one of its cells.
    pub(super) fn stored_selection(
        &self,
        schema: &Schema,
        target: &SaveTarget,
        result_used: bool,
    ) -> Result<Option<Stored>, Signal> {
        let report = schema.report();
        let declares = |kind: &str| report.rendering_layouts.iter().any(|l| l.kind == kind);
        // The call and the report of the cells.
        if !(result_used
            && matches!(target, SaveTarget::Stream(_))
            && report.default_layout_kind == "word"
            && report.default_rendering_layout.is_some()
            && !report.processing_only
            && report.extensions.is_empty()
            && self.design_time_selection().is_none())
        {
            return Ok(None);
        }
        let Some(id) = self.engine.program.objects[schema.object()].id else {
            return Ok(None);
        };
        let cx = self.cx.clone();
        // The rows of `table` whose field `key` is `want`. A table the program does not hold has no row; `None` when the
        // table is not the one named or lacks the field.
        let rows_of = |table: u32, name: &str, key: &str, want: &dyn Fn(&Value) -> bool| -> Result<Option<_>, Signal> {
            let Some(t) = self.table_def(table) else {
                return Ok(Some(Vec::new()));
            };
            let Some(k) = t
                .name
                .eq_ignore_ascii_case(name)
                .then(|| t.fields.iter().position(|f| f.name.eq_ignore_ascii_case(key)))
                .flatten()
            else {
                return Ok(None);
            };
            Ok(Some(
                self.all_rows(&cx, &t)?
                    .into_iter()
                    .filter(|r| want(&r[k]))
                    .collect::<Vec<_>>(),
            ))
        };
        let of_report = |v: &Value| matches!(v, Value::Integer(n) if *n as i64 == id as i64);
        // No Tenant Report Layout Selection row of the report: with one, the layout it names renders (:501-:648).
        match rows_of(
            TENANT_REPORT_LAYOUT_SELECTION,
            "Tenant Report Layout Selection",
            "Report ID",
            &of_report,
        )? {
            Some(rows) if rows.is_empty() => {}
            _ => return Ok(None),
        }
        // Exactly one Report Layout Selection row of the report, and it is the company's.
        let Some(t) = self.table_def(REPORT_LAYOUT_SELECTION) else {
            return Ok(None);
        };
        let Some(rows) = rows_of(
            REPORT_LAYOUT_SELECTION,
            "Report Layout Selection",
            "Report ID",
            &of_report,
        )?
        else {
            return Ok(None);
        };
        let col = |n: &str| t.fields.iter().position(|f| f.name.eq_ignore_ascii_case(n));
        let (Some(company), Some(kind), Some(code)) =
            (col("Company Name"), col("Type"), col("Custom Report Layout Code"))
        else {
            return Ok(None);
        };
        let [row] = rows.as_slice() else {
            return Ok(None);
        };
        if text(&row[company]) != Some(&*self.cx.company) {
            return Ok(None);
        }
        let kind = match &row[kind] {
            Value::Integer(n) | Value::Option(n, _) => *n,
            _ => return Ok(None),
        };
        let Some(code) = text(&row[code]) else {
            return Ok(None);
        };
        if kind == RDLC_BUILT_IN && code.is_empty() && declares("rdlc") {
            return Ok(Some(Stored::Rdlc));
        }
        if kind == CUSTOM_LAYOUT && !code.is_empty() {
            // The code names no Custom Report Layout row and no Tenant Report Layout row of the report (:697: one that it
            // names renders).
            let named = |v: &Value| text(v).is_none_or(|s| s.eq_ignore_ascii_case(code));
            let custom = rows_of(CUSTOM_REPORT_LAYOUT, "Custom Report Layout", "Code", &named)?;
            let tenant = rows_of(TENANT_REPORT_LAYOUT, "Tenant Report Layout", "Name", &named)?;
            if let (Some(custom), Some(tenant)) = (custom, tenant)
                && custom.is_empty()
                && tenant.is_empty()
            {
                return Ok(Some(Stored::NoLayout));
            }
        }
        Ok(None)
    }
}
