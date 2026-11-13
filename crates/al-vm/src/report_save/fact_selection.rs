//! The layout selection facts of a save: [`Selection`], [`Stored`] and whether a selection row names the report. Step 1 round
//! 1 reads them from the rows as main did; round 2 takes them from the answers of the platform events Microsoft's
//! ReportManagement subscribes to (ADR-040 amendment 2026-11-13, part 1, point 3).
//!
//! Base Application's ReportManagement answers SelectedBuiltinLayoutType and SelectReportLayoutCode from table 9651 "Report
//! Layout Selection" and from the SingleInstance codeunit 9654 "Design-time Report Selection" (ReportManagement.Codeunit.al,
//! Base Application 29); the platform reads Tenant Report Layout Selection. A row of the company that selects the built-in
//! RDLC or Word layout of the default layout's kind, or a tenant row that names such a layout the report declares, is
//! [`Selection::SameKind`] (reports/html-email-body/RxLayoutSelectionTests.al:403, :428, :501, corpus pull request 1371); one
//! of the other kind is [`Selection::OtherKind`]; any other row is [`Selection::Unknown`]. Rows of another company are not
//! read (:795, :1022).
//!
//! The temporary selection (ADR-040 amendment 2026-11-12, A7): `Report Selections`.`SaveReportAsHTMLImpl` sets one before
//! every e-mail body save. For an Html save to a stream of a Word layout report that no extension extends, while the
//! "Design-time Report Selection" of Base Application holds a selection, SelectReportLayoutCode is raised to
//! ReportManagement's subscriber only, with the entry values events/platform-triggers/PeVTests.al:721 pins, so Microsoft's
//! selection code runs as AL (it clears the selection). When it answers a selected layout BC raises no
//! SelectedBuiltinLayoutType (corpus pull request 1362, reports/layouts-html-text/HtRenderedTextTests.al:494, :530). When the
//! answer names exactly one user-defined Word layout of the report the fact is [`Selection::TempWord`]; any other answer
//! refuses (al-runner issue 6924; assumption `report-layout-temp-selection`, marked by the rows that answer).

use std::sync::Arc;

use super::facts::{Extensions, Kind, LayoutFile, Selection, Stored, Target};
use super::{Format, SaveTarget, Schema};
use crate::tenant_layouts::{TENANT_REPORT_LAYOUT, WORD};
use crate::{Session, Signal};
use al_builtins::value::Value;

/// Base Application's table "Report Layout Selection" and codeunit "Design-time Report Selection".
const REPORT_LAYOUT_SELECTION: u32 = 9651;
const DESIGN_TIME_REPORT_SELECTION: u32 = 9654;
/// The platform table Tenant Report Layout Selection.
const TENANT_REPORT_LAYOUT_SELECTION: u32 = 2_000_000_233;
/// Base Application's table "Custom Report Layout".
const CUSTOM_REPORT_LAYOUT: u32 = 9650;
/// Members of "Report Layout Selection".Type (ReportLayoutSelection.Table.al, Base Application of the three majors).
const RDLC_BUILT_IN: i32 = 0;
const WORD_BUILT_IN: i32 = 1;
const CUSTOM_LAYOUT: i32 = 2;
/// The members of the events' `LayoutType` option (codeunit Reporting Triggers).
const LAYOUT_TYPES: [&str; 5] = ["None", "RDLC", "Word", "Excel", "Custom"];
/// `LayoutType::None`: the entry value of SelectReportLayoutCode (PeVTests.al:721 `in=/None/No`).
const NONE: i32 = 0;

fn text(v: &Value) -> Option<&str> {
    match v {
        Value::Text(s) | Value::Code(s) if !s.has_marker() => Some(s.as_str()),
        _ => None,
    }
}

fn ordinal(v: &Value) -> Option<i32> {
    match v {
        Value::Option(n, _) | Value::Integer(n) => Some(*n),
        _ => None,
    }
}

fn field(t: &al_data::TableDef, name: &str) -> Option<usize> {
    t.fields.iter().position(|f| f.name.eq_ignore_ascii_case(name))
}

fn of_report(t: &al_data::TableDef, row: &al_data::Row, id: i32) -> bool {
    field(t, "Report ID").is_some_and(|r| matches!(row[r], Value::Integer(n) if n == id))
}

/// How one selection row answers for the report: as its default layout, as the layout of the other kind, or unknown.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum RowClass {
    SameKind,
    OtherKind,
    Unknown,
}

/// What the entry of A7 needs of the call and the report.
pub(super) struct TempEntry<'a> {
    pub format: Format,
    pub target: Target,
    pub kind: Kind,
    pub extensions: Extensions,
    pub processing_only: bool,
    pub file: LayoutFile,
    pub custom: bool,
    pub what: &'a str,
}

impl Session {
    fn report_id(&self, schema: &Schema) -> i32 {
        self.object_id_of(schema.object as u32) as i32
    }

    /// The layout selection BC follows for a save of `schema`; `entry` is the save, `None` for a run (which never takes the
    /// temporary selection). With a temporary selection resolved, its layout's name.
    pub(super) fn selection_fact(
        &mut self,
        schema: &Schema,
        entry: Option<&TempEntry>,
    ) -> Result<(Selection, Option<Arc<str>>), Signal> {
        if let Some(e) = entry
            && self.temp_selection_entry(e)
        {
            let name = self.temp_selected_layout(schema, e.what)?;
            return Ok((Selection::TempWord, Some(name)));
        }
        let class = self.selection_rows_class(schema)?;
        if self.design_time_selection().is_some() {
            return Ok((
                if class == Some(RowClass::Unknown) {
                    Selection::Unknown
                } else {
                    Selection::DesignTimeHeld
                },
                None,
            ));
        }
        Ok((
            match class {
                None => Selection::None,
                Some(RowClass::SameKind) => Selection::SameKind,
                Some(RowClass::OtherKind) => Selection::OtherKind,
                Some(RowClass::Unknown) => Selection::Unknown,
            },
            None,
        ))
    }

    /// The worst class of the company's Report Layout Selection rows and the Tenant Report Layout Selection rows of the
    /// report; `None` when there is none.
    fn selection_rows_class(&self, schema: &Schema) -> Result<Option<RowClass>, Signal> {
        let report = schema.report();
        let Some(id) = self.engine.program.objects[schema.object()].id else {
            return Ok(None);
        };
        let id = id as i32;
        let has_kind =
            |kind: &str| report.default_layout_kind == kind || report.rendering_layouts.iter().any(|l| l.kind == kind);
        let class_of = |kind: &str| match kind {
            "rdlc" | "word" if report.default_layout_kind == kind => RowClass::SameKind,
            "rdlc" | "word" if has_kind(kind) => RowClass::OtherKind,
            _ => RowClass::Unknown,
        };
        let mut worst = None;
        let cx = self.cx.clone();
        if let Some(t) = self
            .table_def(REPORT_LAYOUT_SELECTION)
            .filter(|t| t.name.eq_ignore_ascii_case("Report Layout Selection"))
            && let (Some(c), Some(k)) = (field(&t, "Company Name"), field(&t, "Type"))
            && field(&t, "Report ID").is_some()
        {
            for row in self.all_rows(&cx, &t)? {
                let company = match &row[c] {
                    Value::Text(s) | Value::Code(s) => s.to_string(),
                    _ => String::new(),
                };
                if !of_report(&t, &row, id) || !company.eq_ignore_ascii_case(&self.cx.company) {
                    continue;
                }
                let class = match &row[k] {
                    Value::Integer(RDLC_BUILT_IN) | Value::Option(RDLC_BUILT_IN, _) => class_of("rdlc"),
                    Value::Integer(WORD_BUILT_IN) | Value::Option(WORD_BUILT_IN, _) => class_of("word"),
                    _ => RowClass::Unknown,
                };
                worst = worst.max(Some(class));
            }
        }
        if let Some(t) = self.table_def(TENANT_REPORT_LAYOUT_SELECTION)
            && let Some(n) = field(&t, "Layout Name")
            && field(&t, "Report ID").is_some()
        {
            for row in self.all_rows(&cx, &t)? {
                if !of_report(&t, &row, id) {
                    continue;
                }
                let name = match &row[n] {
                    Value::Text(s) | Value::Code(s) => s.to_string(),
                    _ => String::new(),
                };
                let class = report
                    .rendering_layouts
                    .iter()
                    .find(|l| l.name.eq_ignore_ascii_case(&name))
                    .map_or(RowClass::Unknown, |l| class_of(&l.kind));
                worst = worst.max(Some(class));
            }
        }
        Ok(worst)
    }

    /// Whether a row of Tenant Report Layout Selection or of Report Layout Selection (any company) has the report's id.
    pub(super) fn named_fact(&self, schema: &Schema) -> Result<bool, Signal> {
        Ok(self.selection_row_of_report(schema)?.is_some())
    }

    /// The selection table that holds a row with the report's id, as text.
    fn selection_row_of_report(&self, schema: &Schema) -> Result<Option<&'static str>, Signal> {
        let id = self.report_id(schema);
        let cx = self.cx.clone();
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
            if let Some(t) = self.table_def(table)
                && self.all_rows(&cx, &t)?.iter().any(|row| of_report(&t, row, id))
            {
                return Ok(Some(what));
            }
        }
        Ok(None)
    }

    /// The stored row the probe measured (corpus pull request 1371, RxLayoutSelectionTests.al:452, :477, :746, :771): no
    /// Tenant Report Layout Selection row and exactly one Report Layout Selection row of the report, which is the company's
    /// (its name compared ignoring case), while Design-time Report Selection holds no selection. Type "RDLC (built-in)" with
    /// an empty code of a report that declares an RDLC layout in its rendering section, or Type "Custom Layout" with a code
    /// that names no Custom Report Layout row and no Tenant Report Layout row (:697: one that it names renders).
    pub(super) fn stored_fact(&self, schema: &Schema) -> Result<Stored, Signal> {
        if self.design_time_selection().is_some() {
            return Ok(Stored::None);
        }
        let report = schema.report();
        let Some(id) = self.engine.program.objects[schema.object()].id else {
            return Ok(Stored::None);
        };
        let cx = self.cx.clone();
        // The rows of `table` whose field `key` passes `want`. A table the program does not hold has no row; `None` when the
        // table is not the one named or lacks the field.
        let rows_of = |table: u32, name: &str, key: &str, want: &dyn Fn(&Value) -> bool| -> Result<Option<_>, Signal> {
            let Some(t) = self.table_def(table) else {
                return Ok(Some(Vec::new()));
            };
            let Some(k) = t.name.eq_ignore_ascii_case(name).then(|| field(&t, key)).flatten() else {
                return Ok(None);
            };
            Ok(Some(
                self.all_rows(&cx, &t)?
                    .into_iter()
                    .filter(|r| want(&r[k]))
                    .collect::<Vec<_>>(),
            ))
        };
        let of_id = |v: &Value| matches!(v, Value::Integer(n) if *n as i64 == id as i64);
        match rows_of(
            TENANT_REPORT_LAYOUT_SELECTION,
            "Tenant Report Layout Selection",
            "Report ID",
            &of_id,
        )? {
            Some(rows) if rows.is_empty() => {}
            _ => return Ok(Stored::None),
        }
        let Some(t) = self.table_def(REPORT_LAYOUT_SELECTION) else {
            return Ok(Stored::None);
        };
        let Some(rows) = rows_of(REPORT_LAYOUT_SELECTION, "Report Layout Selection", "Report ID", &of_id)? else {
            return Ok(Stored::None);
        };
        let (Some(company), Some(kind), Some(code)) = (
            field(&t, "Company Name"),
            field(&t, "Type"),
            field(&t, "Custom Report Layout Code"),
        ) else {
            return Ok(Stored::None);
        };
        let [row] = rows.as_slice() else {
            return Ok(Stored::None);
        };
        if !text(&row[company]).is_some_and(|c| c.eq_ignore_ascii_case(&self.cx.company)) {
            return Ok(Stored::None);
        }
        let (Some(kind), Some(code)) = (ordinal(&row[kind]), text(&row[code])) else {
            return Ok(Stored::None);
        };
        if kind == RDLC_BUILT_IN && code.is_empty() && report.rendering_layouts.iter().any(|l| l.kind == "rdlc") {
            return Ok(Stored::RdlcBuiltin);
        }
        if kind == CUSTOM_LAYOUT && !code.is_empty() {
            let named = |v: &Value| text(v).is_none_or(|s| s.eq_ignore_ascii_case(code));
            let custom = rows_of(CUSTOM_REPORT_LAYOUT, "Custom Report Layout", "Code", &named)?;
            let tenant = rows_of(TENANT_REPORT_LAYOUT, "Tenant Report Layout", "Name", &named)?;
            if let (Some(custom), Some(tenant)) = (custom, tenant)
                && custom.is_empty()
                && tenant.is_empty()
            {
                return Ok(Stored::CustomUnknownCode);
            }
        }
        Ok(Stored::None)
    }

    /// The codeunit "Design-time Report Selection" (9654) while its SingleInstance instance holds a selection (a text global
    /// that is not empty).
    fn design_time_selection(&self) -> Option<usize> {
        let program = &self.engine.program;
        let design_time = program.objects.heads().iter().position(|o| {
            o.is_kind(al_ir::ObjKind::Codeunit)
                && o.id == Some(DESIGN_TIME_REPORT_SELECTION)
                && o.name.eq_ignore_ascii_case("Design-time Report Selection")
        })?;
        let inst = self.singletons.get(&design_time)?;
        let holds = inst
            .borrow()
            .globals
            .iter()
            .any(|v| matches!(v, Value::Text(s) | Value::Code(s) if !s.is_empty()));
        holds.then_some(design_time)
    }

    /// The entry of A7: an Html save to a stream of a Word layout report that renders (not processing-only, a layout file,
    /// no layout of Type Custom) and that no report extension extends, while the "Design-time Report Selection" of the app
    /// with Base Application's id holds a selection.
    fn temp_selection_entry(&self, e: &TempEntry) -> bool {
        let program = &self.engine.program;
        e.format == Format::Html
            && e.target == Target::Stream
            && e.kind == Kind::Word
            && e.extensions == Extensions::None
            && !e.processing_only
            && e.file == LayoutFile::Present
            && !e.custom
            && self.design_time_selection().is_some_and(|oi| {
                program
                    .apps
                    .get(program.objects.heads()[oi].app as usize)
                    .is_some_and(|a| a.id.eq_ignore_ascii_case(al_ir::BASE_APPLICATION_APP_ID))
            })
    }

    /// The name of the user-defined Word layout a temporary selection names. Raises SelectReportLayoutCode to Base
    /// Application's ReportManagement only (LayoutCode `''`, LayoutType None and Success false on entry, PeVTests.al:721) and
    /// refuses unless it answers Success with a name and LayoutType None (no Custom Report Layout row has it, and no app id is
    /// appended), no Tenant Report Layout Selection or Report Layout Selection row names the report, and the name is that of
    /// exactly one Tenant Report Layout row of the report: Layout Format Word, a blank Company Name, the null App ID, and no
    /// row or declared layout whose name differs only in case. A subscriber the test can see to a layout event this does not
    /// raise refuses first.
    fn temp_selected_layout(&mut self, schema: &Schema, what: &str) -> Result<Arc<str>, Signal> {
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
        let id = Value::Integer(self.report_id(schema));
        let layout_type = Value::Option(NONE, LAYOUT_TYPES.iter().map(|m| m.to_string()).collect());
        let mut args = [id, Value::Text("".into()), layout_type, Value::Boolean(false)];
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
        Ok(name.into())
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
        // A selection row of either table is not measured together with a temporary selection.
        if let Some(what) = self.selection_row_of_report(schema)? {
            return Ok(Some(what.into()));
        }
        let id = self.report_id(schema);
        let cx = self.cx.clone();
        let Some(t) = self.table_def(TENANT_REPORT_LAYOUT) else {
            return Ok(Some("the program holds no Tenant Report Layout table".into()));
        };
        let (Some(n), Some(app), Some(company), Some(kind)) = (
            field(&t, "Name"),
            field(&t, "App ID"),
            field(&t, "Company Name"),
            field(&t, "Layout Format"),
        ) else {
            return Ok(Some(
                "the Tenant Report Layout table lacks a field the check reads".into(),
            ));
        };
        let rows: Vec<_> = self
            .all_rows(&cx, &t)?
            .into_iter()
            .filter(|row| of_report(&t, row, id) && text(&row[n]).is_some_and(|s| s.eq_ignore_ascii_case(name)))
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

/// The save's target as a fact.
pub(super) fn target_fact(target: &SaveTarget) -> Target {
    match target {
        SaveTarget::Stream(_) => Target::Stream,
        SaveTarget::File(_) => Target::File,
    }
}
