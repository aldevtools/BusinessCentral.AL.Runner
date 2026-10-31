//! The platform table Tenant Report Layout Cfg (2000000249, BC 29 on), a stored table with these pinned limits
//! (platform/lt-platform-tables/PtCloudTables.al Y17_TenantReportLayoutCfg_InsertModifyDelete and
//! Y17_TenantReportLayoutCfg_Shapes, corpus master 97c29f86):
//!
//! - A fresh tenant holds no row. Insert, a duplicate Insert (false), Modify, Delete, an Insert without triggers, DeleteAll,
//!   a filtered Count and `ModifyAll("Header Part Name", '')` work as on any stored table.
//! - An Insert, with or without triggers, checks the parts. "Header Part Name" must name a Header/Footer-subtype layout as
//!   `<app id in lower case>::<layout name>`, of the row's report or of a report extension of report 2000000001 "Tenant
//!   Report Defaults" (for Report ID 0 only the latter). "Layout Name" must be blank or name a Body-subtype layout of the
//!   row's report the same way. Report ID 99999990 (no such report) and any Company Name are accepted. The texts of the
//!   errors are pinned; their code is not. When the header part and the theme part are both wrong, the header part's error
//!   is raised.
//!
//! Not pinned, so refused: a blank header part, a wrong theme part (no error text is pinned), a wrong header part and a wrong
//! body together, a body for Report ID 0 or for a report the program does not hold, a part that names a layout of a report
//! extension of the row's report, a wrong part while the tenant holds user-defined layouts (Tenant Report Layout rows, which
//! Microsoft's Composite Layout tests use as parts), a Modify that sets a part to another text than its stored one or blank,
//! a ModifyAll of anything but a part to blank, a Rename, and a wrong Insert that event subscribers would see.

use crate::Session;
use al_builtins::native::{AlError, Signal};
use al_builtins::record_op::RecordOp;
use al_builtins::value::{RecordState, Value};
use al_data::TableDef;
use std::cell::RefCell;
use std::rc::Rc;

type Rec = Rc<RefCell<RecordState>>;

const TENANT_REPORT_LAYOUT_CFG: u32 = 2_000_000_249;
const TENANT_REPORT_LAYOUT: u32 = 2_000_000_232;
/// The platform report "Tenant Report Defaults", whose report extensions hold the shared header/footer and theme parts.
const TENANT_REPORT_DEFAULTS: u32 = 2_000_000_001;

fn unsupported<T>(what: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(format!(
        "Tenant Report Layout Cfg: {}",
        what.into()
    )))
}

/// What a part of a row names.
enum Part {
    Valid,
    /// Wrong: the pinned error is raised.
    Wrong,
    /// Not pinned: refused, with why.
    Unknown(String),
}

impl Session {
    /// The checks of a write to Tenant Report Layout Cfg (module comment). `args` are the record method's arguments.
    pub(crate) fn tenant_layout_cfg_check(
        &mut self,
        t: &TableDef,
        rec: &Rec,
        method: RecordOp,
        args: &[Value],
    ) -> Result<(), Signal> {
        if rec.borrow().temporary
            || self.install_scope
            || self.engine.program.objects[t.object as usize].id != Some(TENANT_REPORT_LAYOUT_CFG)
        {
            return Ok(());
        }
        match method {
            RecordOp::Insert => self.cfg_insert(t, rec),
            RecordOp::Modify => self.cfg_modify(t, rec),
            RecordOp::ModifyAll => {
                let pos = crate::records::arg_pos(args, 0)?;
                let blank = matches!(args.get(1), Some(Value::Text(v) | Value::Code(v)) if v.is_empty());
                if [4, 5].contains(&t.fields[pos].id) && blank {
                    Ok(())
                } else {
                    unsupported("a ModifyAll of another field than a part, or to a value other than blank (not pinned)")
                }
            }
            RecordOp::Rename => unsupported("Rename (not pinned)"),
            _ => Ok(()),
        }
    }

    fn cfg_text(t: &TableDef, rec: &Rec, id: u32) -> String {
        let r = rec.borrow();
        t.fields
            .iter()
            .position(|f| f.id == id)
            .and_then(|p| match &r.fields[p] {
                Value::Text(v) | Value::Code(v) => Some(v.to_string()),
                _ => None,
            })
            .unwrap_or_default()
    }

    fn cfg_insert(&mut self, t: &TableDef, rec: &Rec) -> Result<(), Signal> {
        let report = match t
            .fields
            .iter()
            .position(|f| f.id == 1)
            .map(|p| rec.borrow().fields[p].clone())
        {
            Some(Value::Integer(id)) => id,
            _ => return unsupported("an Insert whose Report ID is not an integer"),
        };
        let (body, header, theme) = (
            Self::cfg_text(t, rec, 2),
            Self::cfg_text(t, rec, 4),
            Self::cfg_text(t, rec, 5),
        );
        if header.is_empty() {
            return unsupported("an Insert with a blank Header Part Name (not pinned)");
        }
        let header_part = self.cfg_part(report, &header, "HeaderFooter", false);
        let theme_part = if theme.is_empty() {
            Part::Valid
        } else {
            self.cfg_part(report, &theme, "Theme", false)
        };
        let body_part = if body.is_empty() {
            Part::Valid
        } else if report == 0 {
            Part::Unknown("a Layout Name for Report ID 0".into())
        } else {
            self.cfg_part(report, &body, "Body", true)
        };
        let error = match (header_part, body_part, theme_part) {
            (Part::Unknown(why), _, _) | (_, Part::Unknown(why), _) | (_, _, Part::Unknown(why)) => {
                return unsupported(format!("{why} (not pinned)"));
            }
            (Part::Valid, Part::Valid, Part::Valid) => return Ok(()),
            // PtCloudTables.al Y17_TenantReportLayoutCfg_Shapes 'invalid header part' (the theme part is wrong too),
            // 'header part of the theme subtype', 'global default, parts of an ordinary report'.
            (Part::Wrong, Part::Valid, _) => format!(
                "The Header/Footer part '{header}' must reference a Header/Footer-subtype layout in the tenants Report Layout List."
            ),
            // 'body layout by plain name', 'default subtype layout', 'undeclared layout'.
            (Part::Valid, Part::Wrong, Part::Valid) => {
                format!(
                    "The Body part '{body}' must reference a Body-subtype layout in the tenants Report Layout List."
                )
            }
            _ => return unsupported("an Insert with more than one wrong part, or a wrong theme part (not pinned)"),
        };
        if self.cfg_user_layouts_present() {
            return unsupported(
                "a wrong part while the tenant holds user-defined layouts (whether one of them is meant is not pinned)",
            );
        }
        if self.listened(t, "onbeforeinsertevent", None)? || self.listened(t, "onafterinsertevent", None)? {
            return unsupported("a wrong part with insert event subscribers (when the check runs is not pinned)");
        }
        Err(Signal::Error(Box::new(AlError {
            message: error,
            code: al_builtins::native::UNPINNED_ERROR.into(),
            call_stack: String::new(),
            ..Default::default()
        })))
    }

    /// Whether `value` names a layout of subtype `subtype` that a row of report `report` may use: as a body (`body`), one
    /// of the report itself; as a part, one of the report or of a report extension of Tenant Report Defaults.
    fn cfg_part(&self, report: i32, value: &str, subtype: &str, body: bool) -> Part {
        let program = &self.engine.program;
        let mut seen_elsewhere = false;
        for o in program.objects.iter() {
            let own = report != 0 && o.kind == al_ir::ObjKind::Report && o.id == Some(report as u32);
            let defaults =
                !body && o.kind == al_ir::ObjKind::ReportExtension && o.extends_id == Some(TENANT_REPORT_DEFAULTS);
            let ext_of_report =
                report != 0 && o.kind == al_ir::ObjKind::ReportExtension && o.extends_id == Some(report as u32);
            if !(own || defaults || ext_of_report || o.id == Some(TENANT_REPORT_DEFAULTS)) {
                continue;
            }
            let Some(app) = program.apps.get(o.app as usize) else {
                continue;
            };
            let app_id = app.id.to_ascii_lowercase();
            for l in &o.layouts {
                let name = format!("{app_id}::{}", l.name);
                if !name.eq_ignore_ascii_case(value) {
                    continue;
                }
                if name != value || !(own || defaults) {
                    seen_elsewhere = true;
                    continue;
                }
                let declared = l
                    .properties
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case("Subtype"))
                    .map(|(_, v)| v.trim().trim_matches('"').to_string());
                return match declared {
                    Some(s) if s.eq_ignore_ascii_case(subtype) => Part::Valid,
                    _ => Part::Wrong,
                };
            }
        }
        if seen_elsewhere {
            return Part::Unknown(format!(
                "'{value}' names a layout of a report extension of the row's report, or differs from one only in case"
            ));
        }
        // PtCloudTables.al Y17_TenantReportLayoutCfg_Shapes 'unknown report': a report the program does not hold takes the
        // default parts, a body there is not pinned.
        if body
            && report != 0
            && !program
                .objects
                .iter()
                .any(|o| o.kind == al_ir::ObjKind::Report && o.id == Some(report as u32))
        {
            return Part::Unknown(format!(
                "a Layout Name for report {report}, which the program does not hold"
            ));
        }
        Part::Wrong
    }

    /// Whether the tenant holds user-defined layouts (stored Tenant Report Layout rows).
    fn cfg_user_layouts_present(&self) -> bool {
        let program = &self.engine.program;
        let Some(oi) = program
            .objects
            .iter()
            .position(|o| o.kind == al_ir::ObjKind::Table && o.id == Some(TENANT_REPORT_LAYOUT))
        else {
            return false;
        };
        let Some(Some(def)) = self.engine.tables.get(oi) else {
            return false;
        };
        !self.store.stored_rows(&self.cx, def).is_empty()
    }

    /// A Modify keeps each part as stored or sets it blank (PtCloudTables.al Y17_TenantReportLayoutCfg_InsertModifyDelete:
    /// `4=` after a valid Insert); anything else is not pinned.
    fn cfg_modify(&mut self, t: &TableDef, rec: &Rec) -> Result<(), Signal> {
        let pk: Vec<Value> = {
            let r = rec.borrow();
            t.keys[0].iter().map(|&f| r.fields[f].clone()).collect()
        };
        let Ok(Some(stored)) = al_data::Store::get(&self.store, &self.cx, t, &pk) else {
            return Ok(());
        };
        for id in [4, 5] {
            let Some(p) = t.fields.iter().position(|f| f.id == id) else {
                continue;
            };
            let new = Self::cfg_text(t, rec, id);
            let old = match &stored[p] {
                Value::Text(v) | Value::Code(v) => v.to_string(),
                _ => String::new(),
            };
            if !new.is_empty() && new != old {
                return unsupported("a Modify that sets a part to another non-blank text (not pinned)");
            }
        }
        Ok(())
    }
}
