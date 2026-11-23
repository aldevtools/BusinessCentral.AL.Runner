//! `TestPage.GetField(Id)` and `TestPart.GetField(Id)` (al-runner issue 8832): the id is the one the compiler gives a page
//! control, `pages::control_id` of the page and the control name (the compiled app's SymbolReference.json lists the same ids),
//! measured by corpus pull request 1538, testpage/GetFieldByControlId.al (open; assumption `testpage-getfield-control-id`).
//! The page's own field control with the id is the field (a field bound to a field of Rec, to a page variable, or a label, in a
//! repeater, a group, a nested group, a fixed or a grid, :449 :503 :623, a base field a page extension modifies, :698); the handle
//! reads and writes as the control by its name in the calls the cells make (`BY_ID_CALLS`, :570 :661), a hidden control included
//! (:530, :698 not found). A field in a cuegroup is refused: BC shows `0` for its Text, through GetField and by its name (:646),
//! where the control by its name answers the text here. The id of a part, a cuegroup, a fixed or a grid gives a field whose Value
//! is empty (:545, :623). Any other id is not found: a group, a repeater, an action, an action group, an area, a field a page
//! extension adds (by either object's id), a control of another page or of the part's page (:468 :484 :545 :588 :604 :698;
//! testpage/TestPageRuntimeContracts.al:274-275 the code).

use super::*;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::page_op::PageMethod;

/// The calls the cells make on what GetField found for a field control (GetFieldByControlId.al:449-:698).
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
        let object = st.borrow().object;
        let page_id = self.object_id_of(object)?;
        let engine = self.engine();
        let page = page_of(&engine, st);
        let Some(idx) = page
            .controls
            .iter()
            .position(|c| c.owner.is_none() && c.action_ref.is_none() && control_id(page_id, &c.name) == i64::from(id))
        else {
            return not_found();
        };
        let ctl = &page.controls[idx];
        let by_id = match ctl.kind {
            ir::ControlKind::Field if Self::in_cuegroup(page, idx) => {
                return unsupported(
                    "TestPage.GetField of a field in a cuegroup (its value is not modelled as BC shows it)",
                );
            }
            ir::ControlKind::Field => ById::Field,
            ir::ControlKind::Part | ir::ControlKind::CueGroup => ById::Empty,
            ir::ControlKind::Group if ctl.fixed_or_grid => ById::Empty,
            _ => return not_found(),
        };
        drop(engine);
        assume(Assumption::TestPageGetFieldControlId);
        Ok(test_control_by_id(st.clone(), idx, by_id))
    }

    /// Control `idx` of `page` is inside a cuegroup.
    fn in_cuegroup(page: &ir::Page, idx: usize) -> bool {
        let mut at = page.controls[idx].parent;
        while let Some(p) = at {
            if page.controls[p].kind == ir::ControlKind::CueGroup {
                return true;
            }
            at = page.controls[p].parent;
        }
        false
    }

    /// A call `m` on what GetField found (`ControlRef::by_id`): `Some` answers it, `None` lets it run as on the control by its
    /// name. The empty field of a part, a cuegroup, a fixed or a grid answers Value only (GetFieldByControlId.al:545 :623; its
    /// Caption, the part page's, and its Visible are measured at :685 and refused here).
    pub(super) fn by_id_call(c: &ControlRef, m: PageMethod) -> Result<Option<Value>, Signal> {
        match (c.by_id, m) {
            (None, _) => Ok(None),
            (Some(ById::Empty), PageMethod::Value) => Ok(Some(Value::Text("".into()))),
            (Some(ById::Field), m) if BY_ID_CALLS.contains(&m) => Ok(None),
            (Some(_), m) => unsupported(format!(
                "TestField.{} on what TestPage.GetField found by a control id (not measured)",
                m.name()
            )),
        }
    }
}
