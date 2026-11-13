//! The layout selection facts of a save: [`Selection`], [`Stored`] and whether a selection row names the report, taken from
//! the answers of the platform events Microsoft's ReportManagement subscribes to (ADR-040 amendment 2026-11-13, part 1,
//! point 3). The runtime raises SelectReportLayoutCode and SelectedBuiltinLayoutType to Base Application's ReportManagement
//! only, so Microsoft's code reads table 9651 "Report Layout Selection" and the SingleInstance codeunit 9654 "Design-time
//! Report Selection" as AL; this module does not read table 9651. The platform table Tenant Report Layout Selection is read
//! here, as the platform reads it.
//!
//! BC raises SelectReportLayoutCode, HasCustomLayout and SelectedBuiltinLayoutType for a rendering save, in that order
//! (events/platform-triggers/PeVTests.al:713, `saveas static html`; entry values LayoutCode `''`, LayoutType None, Success
//! false). When SelectReportLayoutCode answers a selected layout BC raises no SelectedBuiltinLayoutType
//! (reports/layouts-html-text/HtRenderedTextTests.al:494, :530, corpus pull request 1362). ReportManagement's subscriber to
//! SelectedBuiltinLayoutType only reads the company's row (`Get(ReportID, CompanyName)`), so the runtime also raises it to
//! that subscriber alone when a selected layout was answered: no code can see that call.
//!
//! The mapping:
//! - SelectedBuiltinLayoutType answers RDLC or Word for a layout of the default layout's kind: [`Selection::SameKind`]
//!   (reports/html-email-body/RxLayoutSelectionTests.al:403, :428, :501, corpus pull request 1371); of the other kind, which
//!   the rendering section declares: [`Selection::OtherKind`]. For a save of a report whose `WordLayout` and `RDLCLayout`
//!   properties declare its layouts, RDLC is the other kind too (RxdDeclLayoutTests.al:130-:293, corpus pull request 1381).
//!   Any other answer that is not None: [`Selection::Unknown`]. A row of another company or with an empty Company Name is
//!   not read by Microsoft's code (RxLayoutSelectionTests.al:795, :1022; RxdDeclLayoutTests.al:916, :1126).
//! - A Tenant Report Layout Selection row that names a declared RDLC or Word layout is classed the same way; any other row is
//!   [`Selection::Unknown`].
//! - SelectReportLayoutCode answers Success with a name and LayoutType None while "Design-time Report Selection" held a
//!   selection (a temporary selection): the platform looks the name up in Tenant Report Layout. Exactly one user-defined
//!   Word layout of the report: [`Selection::TempWord`] while SelectedBuiltinLayoutType answers None,
//!   [`Selection::TempWordOverStored`] while it answers RDLC or Word (RxdDeclLayoutTests.al:811, :1336, :1651). Anything else
//!   is [`Selection::Unknown`].
//! - The same answer while no selection was held (the name is the company's Custom Layout row's code): a name of no Tenant
//!   Report Layout row, while SelectedBuiltinLayoutType answers None, is [`Stored::CustomUnknownCode`]
//!   (RxLayoutSelectionTests.al:746, :771). Any other answer is [`Selection::Unknown`].
//!
//! A run, and a save that does not reach the layout selection (processing only, a layout of Type Custom, the company build),
//! raises no SelectReportLayoutCode; while "Design-time Report Selection" holds a selection its fact is
//! [`Selection::DesignTimeHeld`].

use std::sync::Arc;

use super::Schema;
use super::facts::{Declares, Selection, Stored};
use crate::tenant_layouts::{TENANT_REPORT_LAYOUT, WORD};
use crate::{Session, Signal};
use al_builtins::value::Value;

/// Base Application's codeunit "Design-time Report Selection" and table "Report Layout Selection" (only whether the program
/// holds it).
const DESIGN_TIME_REPORT_SELECTION: u32 = 9654;
const REPORT_LAYOUT_SELECTION: u32 = 9651;
/// The platform table Tenant Report Layout Selection.
const TENANT_REPORT_LAYOUT_SELECTION: u32 = 2_000_000_233;
/// The members of the events' `LayoutType` option (codeunit Reporting Triggers).
const LAYOUT_TYPES: [&str; 5] = ["None", "RDLC", "Word", "Excel", "Custom"];
/// `LayoutType::None`, the entry value (PeVTests.al:713 `in=/None/No`, `in=None`), and the answers RDLC and Word.
const NONE: i32 = 0;
const RDLC: i32 = 1;
const WORD_TYPE: i32 = 2;

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

/// How one selection answers for the report: as its default layout, as the layout of the other kind, or unknown.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum RowClass {
    SameKind,
    OtherKind,
    Unknown,
}

/// What ReportManagement answered to SelectReportLayoutCode.
enum Select {
    /// Not raised: a run, or a save that does not reach the layout selection.
    NotRaised,
    /// Success false.
    Nothing,
    /// Success with a layout name and LayoutType None.
    Named(String),
    /// Any other answer.
    Other,
}

/// The selection facts of one save or run.
pub(super) struct SelectionFacts {
    pub selection: Selection,
    pub stored: Stored,
    /// The layout a selected name resolved to.
    pub selected: Option<Arc<str>>,
}

impl Session {
    fn report_id(&self, schema: &Schema) -> i32 {
        self.object_id_of(schema.object as u32) as i32
    }

    /// The layout selection BC follows for `schema`. `save`: the save reaches the layout selection, so SelectReportLayoutCode
    /// is raised; `declares` is the report's [`Declares`] fact (a run passes `None`).
    pub(super) fn selection_fact(&mut self, schema: &Schema, save: Option<Declares>) -> Result<SelectionFacts, Signal> {
        // Whether the name SelectReportLayoutCode answers comes from "Design-time Report Selection" (a temporary selection) or
        // from the company's Custom Layout row: BC answers the two differently when the name matches nothing
        // (HtRenderedTextTests.al:543-603 false without an error; RxLayoutSelectionTests.al:746 ReportNoLayout).
        let held = self.design_time_selection().is_some();
        let select = match save {
            Some(_) => self.select_layout_code(schema)?,
            None => Select::NotRaised,
        };
        let builtin = self.builtin_layout_type(schema)?;
        let tenant = self.tenant_selection_class(schema)?;
        let unknown = SelectionFacts {
            selection: Selection::Unknown,
            stored: Stored::None,
            selected: None,
        };
        match select {
            Select::Named(_) if tenant.is_some() => Ok(unknown),
            Select::Named(name) if held => {
                if !self.user_word_layout(schema, &name)? {
                    return Ok(unknown);
                }
                let selection = match builtin {
                    NONE => Selection::TempWord,
                    RDLC | WORD_TYPE => Selection::TempWordOverStored,
                    _ => return Ok(unknown),
                };
                Ok(SelectionFacts {
                    selection,
                    stored: Stored::None,
                    selected: Some(name.into()),
                })
            }
            Select::Named(name) => {
                let stored = if builtin == NONE && self.no_tenant_layout_named(&name)? {
                    Stored::CustomUnknownCode
                } else {
                    Stored::None
                };
                Ok(SelectionFacts { stored, ..unknown })
            }
            Select::Other => Ok(unknown),
            Select::Nothing | Select::NotRaised => {
                let property_rdlc = save == Some(Declares::WordAndRdlcProperties);
                let class = match builtin {
                    NONE => None,
                    RDLC => Some(class_of(schema, "rdlc", property_rdlc)),
                    WORD_TYPE => Some(class_of(schema, "word", false)),
                    _ => Some(RowClass::Unknown),
                };
                let class = class.max(tenant);
                let held = held && matches!(select, Select::NotRaised);
                let selection = match class {
                    Some(RowClass::Unknown) => Selection::Unknown,
                    _ if held => Selection::DesignTimeHeld,
                    None => Selection::None,
                    Some(RowClass::SameKind) => Selection::SameKind,
                    Some(RowClass::OtherKind) => Selection::OtherKind,
                };
                let report = schema.report();
                let declares_rdlc = report.rendering_layouts.iter().any(|l| l.kind == "rdlc") || property_rdlc;
                let stored = if builtin == RDLC && tenant.is_none() && !held && declares_rdlc {
                    Stored::RdlcBuiltin
                } else {
                    Stored::None
                };
                Ok(SelectionFacts {
                    selection,
                    stored,
                    selected: None,
                })
            }
        }
    }

    /// The object of Base Application's ReportManagement, whose subscribers to `al_ir::REPORT_MANAGEMENT_LAYOUT_EVENTS` this
    /// module raises.
    pub(super) fn report_management_object(&self) -> Option<usize> {
        self.report_management_subscriber("selectreportlayoutcode")
            .map(|(o, _)| o)
    }

    /// Whether a Tenant Report Layout Selection row names the report, or ReportManagement answers SelectedBuiltinLayoutType
    /// with a layout type (a row of the company selects a built-in layout); read again after the run.
    pub(super) fn named_fact(&mut self, schema: &Schema) -> Result<bool, Signal> {
        Ok(self.tenant_selection_class(schema)?.is_some() || self.builtin_layout_type(schema)? != NONE)
    }

    /// The worst class of the Tenant Report Layout Selection rows of the report; `None` when there is none.
    fn tenant_selection_class(&self, schema: &Schema) -> Result<Option<RowClass>, Signal> {
        let id = self.report_id(schema);
        let cx = self.cx.clone();
        let mut worst = None;
        if let Some(t) = self.table_def(TENANT_REPORT_LAYOUT_SELECTION)
            && let Some(n) = field(&t, "Layout Name")
            && field(&t, "Report ID").is_some()
        {
            for row in self.all_rows(&cx, &t)? {
                if !of_report(&t, &row, id) {
                    continue;
                }
                let name = text(&row[n]).unwrap_or_default();
                let class = schema
                    .report()
                    .rendering_layouts
                    .iter()
                    .find(|l| l.name.eq_ignore_ascii_case(name))
                    .map_or(RowClass::Unknown, |l| class_of(schema, &l.kind, false));
                worst = worst.max(Some(class));
            }
        }
        Ok(worst)
    }

    /// ReportManagement's answer to SelectedBuiltinLayoutType for the report (entry LayoutType None). None when the program
    /// holds no "Report Layout Selection" table; refused when it does and ReportManagement's subscriber is not compiled.
    fn builtin_layout_type(&mut self, schema: &Schema) -> Result<i32, Signal> {
        let Some(to) = self.report_management_subscriber("selectedbuiltinlayouttype") else {
            return self
                .without_report_management(schema, "SelectedBuiltinLayoutType")
                .map(|_| NONE);
        };
        let id = Value::Integer(self.report_id(schema));
        let mut args = [id, layout_type(NONE)];
        self.raise_to_report_management("selectedbuiltinlayouttype", to, &["ObjectID", "LayoutType"], &mut args)?;
        ordinal(&args[1])
            .ok_or_else(|| Signal::unsupported("SelectedBuiltinLayoutType answered no LayoutType".to_string()))
    }

    /// ReportManagement's answer to SelectReportLayoutCode for the report (entry LayoutCode `''`, LayoutType None, Success
    /// false; PeVTests.al:713). Its subscriber clears the selection "Design-time Report Selection" holds.
    fn select_layout_code(&mut self, schema: &Schema) -> Result<Select, Signal> {
        let Some(to) = self.report_management_subscriber("selectreportlayoutcode") else {
            return self
                .without_report_management(schema, "SelectReportLayoutCode")
                .map(|_| Select::Nothing);
        };
        let id = Value::Integer(self.report_id(schema));
        let mut args = [id, Value::Text("".into()), layout_type(NONE), Value::Boolean(false)];
        let names = ["ObjectId", "LayoutCode", "LayoutType", "Success"];
        self.raise_to_report_management("selectreportlayoutcode", to, &names, &mut args)?;
        Ok(match (text(&args[1]), ordinal(&args[2]), &args[3]) {
            (_, _, Value::Boolean(false)) => Select::Nothing,
            (Some(name), Some(NONE), Value::Boolean(true)) if !name.is_empty() && !name.contains("::") => {
                Select::Named(name.to_string())
            }
            _ => Select::Other,
        })
    }

    /// A program without Base Application's "Report Layout Selection" has no selection row: the answer is None. With the table
    /// and no compiled ReportManagement subscriber to `event`, the save refuses.
    fn without_report_management(&self, schema: &Schema, event: &str) -> Result<(), Signal> {
        if self.table_def(REPORT_LAYOUT_SELECTION).is_none() {
            return Ok(());
        }
        Err(Signal::unsupported(format!(
            "a save or run of report \"{}\": Base Application's ReportManagement subscriber to {event} is not compiled, so \
             its layout selection cannot run",
            schema.name()
        )))
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

    /// Whether `name` is exactly one user-defined Word layout of the report for every company, without an app id: one Tenant
    /// Report Layout row of the report with that name (no other row or declared layout whose name differs only in case),
    /// Layout Format Word, a blank Company Name and the null App ID (HtRenderedTextTests.al:494; RxdDeclLayoutTests.al:313).
    fn user_word_layout(&self, schema: &Schema, name: &str) -> Result<bool, Signal> {
        let id = self.report_id(schema);
        let cx = self.cx.clone();
        let Some(t) = self.table_def(TENANT_REPORT_LAYOUT) else {
            return Ok(false);
        };
        let (Some(n), Some(app), Some(company), Some(kind)) = (
            field(&t, "Name"),
            field(&t, "App ID"),
            field(&t, "Company Name"),
            field(&t, "Layout Format"),
        ) else {
            return Ok(false);
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
        Ok(matches!(rows.as_slice(), [row]
            if !declared
                && text(&row[n]) == Some(name)
                && text(&row[company]) == Some("")
                && matches!(row[app], Value::Guid(g) if g == [0; 16])
                && ordinal(&row[kind]) == Some(WORD)))
    }

    /// Whether no Tenant Report Layout row (of any report) has the name `name`, compared ignoring case
    /// (RxLayoutSelectionTests.al:746, :771; :697: one that it names renders).
    fn no_tenant_layout_named(&self, name: &str) -> Result<bool, Signal> {
        let cx = self.cx.clone();
        let Some(t) = self
            .table_def(TENANT_REPORT_LAYOUT)
            .filter(|t| t.name.eq_ignore_ascii_case("Tenant Report Layout"))
        else {
            return Ok(true);
        };
        let Some(n) = field(&t, "Name") else {
            return Ok(false);
        };
        Ok(!self
            .all_rows(&cx, &t)?
            .iter()
            .any(|row| text(&row[n]).is_none_or(|s| s.eq_ignore_ascii_case(name))))
    }
}

/// The class of a built-in or declared layout of `kind` for the report: its default layout's kind, the other kind the
/// rendering section declares (or, with `property_rdlc`, RDLC that the `RDLCLayout` property declares), or unknown.
fn class_of(schema: &Schema, kind: &str, property_rdlc: bool) -> RowClass {
    let report = schema.report();
    let declared = report.rendering_layouts.iter().any(|l| l.kind == kind) || (property_rdlc && kind == "rdlc");
    match kind {
        "rdlc" | "word" if report.default_layout_kind == kind => RowClass::SameKind,
        "rdlc" | "word" if declared => RowClass::OtherKind,
        _ => RowClass::Unknown,
    }
}

fn layout_type(n: i32) -> Value {
    Value::Option(n, LAYOUT_TYPES.iter().map(|m| m.to_string()).collect())
}
