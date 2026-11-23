//! `TestPage.GetField(Id)` and `TestPart.GetField(Id)` (al-runner issue 8832): the id is the one the compiler gives a page
//! control, `pages::control_id` of the page and the control name (the compiled app's SymbolReference.json lists the same ids),
//! measured by corpus pull request 1538, testpage/GetFieldByControlId.al (open; assumption `testpage-getfield-control-id`).
//! The page's own field control with the id is the field (a field bound to a field of Rec, to a page variable, or a label, in a
//! repeater, a group, a nested group, a fixed, a grid or a cuegroup, :449 :503 :630 :653, a base field a page extension
//! modifies, :705); the handle reads and writes as the control by its name in the calls the cells make (`BY_ID_CALLS`, :570
//! :668), a hidden control included (:530, :705 not found); a field in a cuegroup reads as by its name (`cue_value.rs`, :653,
//! :742). The id of a part, a cuegroup, a fixed or a grid gives a field whose Value is empty (:545, :630). Any other id is not
//! found: a group, a repeater, an action, an action group, an area, a field a page extension adds (by either object's id; from
//! BC 28, :589: BC 27 finds it by the extension's id, `al_version::getfield_finds_extension_fields`), a control of another page
//! or of the part's page (:468 :484 :545 :611 :705; testpage/TestPageRuntimeContracts.al:274-275 the code).
//!
//! `TestPage.GetField(Id)` / `TestPart.GetField(Id)` by the compiler's control id
//! (`pages::control_id`; al-runner issue 8832, assumption `testpage-getfield-control-id`): the page's
//! own field control (a hidden one is not found), or an empty field for a part, a cuegroup, a fixed
//! or a grid, on the page kinds the cells open (`get_field_page_measured`); any other id is not
//! found. Entries: `get_field_by_id`, `by_id_call` (the calls on what it found, by `pageops.rs`).

use super::*;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::page_op::PageMethod;

/// The calls the cells make on what GetField found for a field control (GetFieldByControlId.al:449-:705).
const BY_ID_CALLS: [PageMethod; 11] = [
    PageMethod::Value,
    PageMethod::Caption,
    PageMethod::SetValue,
    PageMethod::Visible,
    PageMethod::Enabled,
    PageMethod::Editable,
    PageMethod::AsDecimal,
    PageMethod::HideValue,
    PageMethod::ShowMandatory,
    PageMethod::ValidationErrorCount,
    PageMethod::AssertEquals,
];

impl Session {
    /// The page's own control (no action, no page extension's) whose compiler id is `id`.
    pub(super) fn own_control_by_id(&self, st: &St, id: i32) -> Result<Option<usize>, Signal> {
        let page_id = self.object_id_of(st.borrow().object)?;
        let engine = self.engine();
        Ok(page_of(&engine, st)
            .controls
            .iter()
            .position(|c| c.owner.is_none() && c.action_ref.is_none() && control_id(page_id, &c.name) == i64::from(id)))
    }

    /// `GetField(id)` on the open page `st`.
    pub(super) fn get_field_by_id(&mut self, st: &St, id: i32) -> Result<Value, Signal> {
        let not_found = || {
            Err(error_code(
                format!("The field with ID = {id} is not found on the page."),
                "TestFieldNotFound",
            ))
        };
        if self.is_request_page(st) {
            return not_found();
        }
        self.get_field_page_measured(st)?;
        let Some(idx) = self.own_control_by_id(st, id)? else {
            return self.get_extension_field_by_id(st, id);
        };
        let engine = self.engine();
        let page = page_of(&engine, st);
        let ctl = &page.controls[idx];
        let by_id = match ctl.kind {
            ir::ControlKind::Field => ById::Field,
            ir::ControlKind::Part | ir::ControlKind::CueGroup => ById::Empty,
            ir::ControlKind::Group if ctl.fixed_or_grid => ById::Empty,
            _ => return not_found(),
        };
        drop(engine);
        // A hidden field is not found by GetField itself, whatever is called on it (:1126 Visible() and Caption() raise as
        // Value() does at :530 and :705): Visible = false, in a group with Visible = false, or removed by the application area.
        if by_id == ById::Field && (self.control_eliminated(st, idx) || self.off_page(st, idx)) {
            return not_found();
        }
        assume(Assumption::TestPageGetFieldControlId);
        Ok(test_control_by_id(st.clone(), idx, by_id))
    }

    /// The page kinds the cells call GetField on: a Card, a List, a Document (:1168), a CardPart and a ListPart opened as a
    /// TestPage of their own (:856, :923), and a ListPart a TestPart shows (:611, :630). Every other one refuses.
    fn get_field_page_measured(&self, st: &St) -> Result<(), Signal> {
        let engine = self.engine();
        let kind = engine.object_facts[st.borrow().object as usize].page_type.clone();
        let part = |k: &str| matches!(&kind, Some(al_ir::PageType::Other(o)) if o == k);
        let measured = if st.borrow().host.is_some() {
            part("listpart")
        } else {
            matches!(
                kind,
                Some(al_ir::PageType::Card | al_ir::PageType::List | al_ir::PageType::Document)
            ) || part("cardpart")
                || part("listpart")
        };
        if measured {
            return Ok(());
        }
        unsupported("TestPage.GetField by a control id on a page kind no cell opens (not measured)")
    }

    /// `GetField(id)` where no control of the page itself has the id: a field a page extension adds, by the extension's id and
    /// its name, is found on BC 27 and not found from BC 28 (`al_version::getfield_finds_extension_fields`;
    /// GetFieldByControlId.al:589, where BC 27 measured its Value only); any other id is not found.
    fn get_extension_field_by_id(&mut self, st: &St, id: i32) -> Result<Value, Signal> {
        let engine = self.engine();
        let page = page_of(&engine, st);
        let mut found = None;
        for (idx, c) in page.controls.iter().enumerate() {
            if let Some(ext) = c.owner
                && c.action_ref.is_none()
                && control_id(self.object_id_of(ext as u32)?, &c.name) == i64::from(id)
            {
                found = Some((idx, c.kind.clone()));
                break;
            }
        }
        drop(engine);
        let major = self.engine.target.as_ref().map_or(29, |t| t.bc.major);
        match (found, al_version::getfield_finds_extension_fields(major)) {
            (None, _) | (Some(_), Some(false)) => Err(error_code(
                format!("The field with ID = {id} is not found on the page."),
                "TestFieldNotFound",
            )),
            (Some((idx, ir::ControlKind::Field)), Some(true)) => {
                assume(Assumption::TestPageGetFieldControlId);
                Ok(test_control_by_id(st.clone(), idx, ById::ExtField))
            }
            (Some(_), _) => unsupported(
                "TestPage.GetField with the id of a page extension's control on this BC version, or of one that is no field (not measured)",
            ),
        }
    }

    /// A call `m` on what GetField found (`ControlRef::by_id`): `Some` answers it, `None` lets it run as on the control by its
    /// name. The empty field of a part, a cuegroup, a fixed or a grid answers Value only (GetFieldByControlId.al:545 :630; its
    /// Caption, the part page's, and its Visible are measured at :692 and refused here); so does a page extension's field on
    /// BC 27 (:589).
    pub(super) fn by_id_call(c: &ControlRef, m: PageMethod) -> Result<Option<Value>, Signal> {
        match (c.by_id, m) {
            (None, _) => Ok(None),
            (Some(ById::Empty), PageMethod::Value) => Ok(Some(Value::Text("".into()))),
            (Some(ById::ExtField), PageMethod::Value) => Ok(None),
            (Some(ById::Field), m) if BY_ID_CALLS.contains(&m) => Ok(None),
            (Some(_), m) => unsupported(format!(
                "TestField.{} on what TestPage.GetField found by a control id (not measured)",
                m.name()
            )),
        }
    }
}
