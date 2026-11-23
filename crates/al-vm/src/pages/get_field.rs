//! `TestPage.GetField(Id)` and `TestPart.GetField(Id)` (al-runner issue 8832): the id is the one the compiler gives a page
//! control, `pages::control_id` of the page and the control name (the compiled app's SymbolReference.json lists the same ids),
//! measured by corpus pull request 1538, testpage/GetFieldByControlId.al (open; assumption `testpage-getfield-control-id`).
//! The page's own field control with the id is the field (a field bound to a field of Rec, to a page variable, or a label, in a
//! repeater, a group, a nested group, a fixed, a grid or a cuegroup, :449 :503 :630 :653, a base field a page extension
//! modifies, :705); the handle reads and writes as the control by its name in the calls the cells make (`BY_ID_CALLS`, :570
//! :668), a hidden control included (:530, :705 not found); a field in a cuegroup reads as by its name (`cue_value.rs`, :653,
//! :742). The id of a part, a cuegroup, a fixed or a grid gives a field whose Value is empty (:545, :630), unless it is hidden
//! (no cell measures that; it refuses). Any other id is not
//! found: a group, a repeater, an action, an action group, an area, a field a page extension adds (by the base page's id; by
//! the extension's id from BC 28 when it states no ApplicationArea, :589: BC 27 finds it,
//! `al_version::getfield_finds_extension_fields`; one with ApplicationArea = All is found by the extension's id, :2804,
//! al-runner issue 8944), a control of another page
//! or of the part's page (:468 :484 :545 :611 :705; testpage/TestPageRuntimeContracts.al:274-275 the code).
//! On the other page kinds the cells measure an id no control has, a Worksheet's action ids and a field's Value by its own id
//! (al-runner issue 8887, :1631-:2702); the ids of page extension controls refuse (:3015-:3051; `get_field_id_owner`).
//! `get_field_kind`, `get_field_missing`.
//!
//! `TestPage.GetField(Id)` / `TestPart.GetField(Id)` by the compiler's control id
//! (`pages::control_id`; al-runner issues 8832 and 8887, assumption `testpage-getfield-control-id`): the page's
//! own field control (a hidden one is not found), or an empty field for a part, a cuegroup, a fixed
//! or a grid, on the page kinds the cells open (`get_field_kind`); any other id is not found where
//! the cells measure it (`get_field_missing`). Entries: `get_field_by_id`, `by_id_call` (the calls on
//! what it found, by `pageops.rs`).

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

/// How far the cells measure `GetField` on a page kind (`Session::get_field_kind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GetFieldKind {
    Full,
    Worksheet,
    FieldValue,
    Missing,
}

/// Who else has an id no control of the page has as its own (`Session::get_field_id_owner`).
enum IdOwner {
    None,
    /// A field a page extension adds, by the base page's id (:2576).
    ExtensionField,
    OtherOfPage,
    /// A control of the host page itself, by the host's id (:2591).
    Host,
    PartPage,
    /// A control a page extension adds, by the extension's id on the page (an actionref), the host or a part's page, or by the
    /// host's id on the host (:3015, :3035, :3051).
    ExtensionControl,
}

/// The page kind `t` the compiler gives no variant of its own (`cardpart`, `rolecenter`, ...).
fn is_other(t: &Option<al_ir::PageType>, k: &str) -> bool {
    matches!(t, Some(al_ir::PageType::Other(o)) if o == k)
}

/// [`Session::get_field_kind`] of the page type `kind` and, for a part, its host's type `host`.
fn get_field_kind_of(kind: &Option<al_ir::PageType>, host: &Option<Option<al_ir::PageType>>) -> GetFieldKind {
    use al_ir::PageType as T;
    let other = |k: &str| is_other(kind, k);
    match host {
        Some(_) if other("listpart") => GetFieldKind::Full,
        Some(Some(T::Card)) if other("cardpart") => GetFieldKind::FieldValue,
        Some(_) => GetFieldKind::Missing,
        None if matches!(kind, Some(T::Card | T::List | T::Document)) || other("cardpart") || other("listpart") => {
            GetFieldKind::Full
        }
        None if matches!(kind, Some(T::Worksheet)) => GetFieldKind::Worksheet,
        None if matches!(kind, Some(T::ListPlus)) => GetFieldKind::FieldValue,
        None => GetFieldKind::Missing,
    }
}

/// The error GetField raises for an id it does not find (GetFieldByControlId.al:468).
fn not_found<T>(id: i32) -> Result<T, Signal> {
    Err(error_code(
        format!("The field with ID = {id} is not found on the page."),
        "TestFieldNotFound",
    ))
}

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
        if self.is_request_page(st) {
            return not_found(id);
        }
        // The page kind matters only for a control that has the id (`get_field_missing`).
        let Some(idx) = self.own_control_by_id(st, id)? else {
            return self.get_extension_field_by_id(st, id);
        };
        let kind = self.get_field_kind(st);
        let engine = self.engine();
        let ctl = &page_of(&engine, st).controls[idx];
        let by_id = match (kind, &ctl.kind) {
            (GetFieldKind::Full, ir::ControlKind::Field) => ById::Field,
            (GetFieldKind::Full, ir::ControlKind::Part | ir::ControlKind::CueGroup) => ById::Empty,
            (GetFieldKind::Full, ir::ControlKind::Group) if ctl.fixed_or_grid => ById::Empty,
            (GetFieldKind::Full, _) => return not_found(id),
            // A field control's Value on a Worksheet, a ListPlus and a CardPart a TestPart shows (:2676, :2689, :2702).
            (GetFieldKind::FieldValue | GetFieldKind::Worksheet, ir::ControlKind::Field) => ById::ValueOnly,
            // An action or an action group of a Worksheet is not found (:2559).
            (GetFieldKind::Worksheet, ir::ControlKind::Action | ir::ControlKind::ActionGroup) => {
                drop(engine);
                assume(Assumption::TestPageGetFieldControlId);
                return not_found(id);
            }
            _ => return unsupported("TestPage.GetField by a control id on a page kind no cell opens (not measured)"),
        };
        drop(engine);
        // A hidden field is not found by GetField itself, whatever is called on it (:1126 Visible() and Caption() raise as
        // Value() does at :530 and :705): Visible = false, in a group with Visible = false, or removed by the application area.
        if self.control_eliminated(st, idx) || self.off_page(st, idx) {
            if by_id != ById::Field {
                return unsupported(
                    "TestPage.GetField by the id of a hidden part, cuegroup, fixed or grid, or of a hidden field whose Value only is measured (not measured)",
                );
            }
            return not_found(id);
        }
        assume(Assumption::TestPageGetFieldControlId);
        Ok(test_control_by_id(st.clone(), idx, by_id))
    }

    /// The page type of `st` and, for a part, of its host.
    fn get_field_page_types(&self, st: &St) -> (Option<al_ir::PageType>, Option<Option<al_ir::PageType>>) {
        let engine = self.engine();
        let type_of = |s: &St| engine.object_facts[s.borrow().object as usize].page_type.clone();
        let host = st
            .borrow()
            .host
            .as_ref()
            .map(|(h, _)| h.upgrade().and_then(|h| type_of(&h)));
        (type_of(st), host)
    }

    /// How far the cells measure GetField on the kind of `st`. `Full`: a Card, a List, a Document (:1168), a CardPart and a
    /// ListPart opened as a TestPage of their own (:856, :923), and a ListPart a TestPart shows (:611, :630). `Worksheet` and
    /// `FieldValue` (a ListPlus, a CardPart a Card shows): a field control's Value (:2676-:2702). `Missing`: only an id no
    /// control has (`get_field_missing`).
    fn get_field_kind(&self, st: &St) -> GetFieldKind {
        let (kind, host) = self.get_field_page_types(st);
        get_field_kind_of(&kind, &host)
    }

    /// `GetField(id)` where no control of the page has the id as its own, nor a page extension's control by the extension's
    /// id: not found where a cell measures it (al-runner issue 8887), under the entry; any other shape refuses.
    /// - An id no control of the page, of an extension, of the host or of a part's page has, on the kinds of `Full`, and on a
    ///   Worksheet (:1631), a ListPlus (:1648), a CardPart a Card (:1665) or a RoleCenter (:2604) shows, a page without PageType
    ///   (:1682), a RoleCenter (:1729), a NavigatePage (:1746), a StandardDialog (:1763), a ConfirmationDialog (:1780), a
    ///   HeadlinePart (:1797), an API page (:2617), a PromptDialog (:2630), a ReportPreview (:2643) and a ReportProcessingOnly
    ///   page (:2663).
    /// - On a Worksheet the base page's id of a field a page extension adds (:2576).
    /// - On a CardPart a Card shows, the id of a host control (:2591).
    ///
    /// The id of a control a page extension adds, by the extension's id, on the page (an actionref), the host or a part's page,
    /// and a control the host's extension adds by the host's id, refuse (:3015, :3035, :3051 measure some).
    fn get_field_missing(&self, st: &St, id: i32) -> Result<(), Signal> {
        use al_ir::PageType as T;
        let (page_type, host) = self.get_field_page_types(st);
        let kind = get_field_kind_of(&page_type, &host);
        if kind == GetFieldKind::Full {
            return Ok(());
        }
        let other = |k: &str| is_other(&page_type, k);
        let measured = match self.get_field_id_owner(st, id)? {
            IdOwner::None => match host {
                Some(host) => other("cardpart") && (host == Some(T::Card) || is_other(&host, "rolecenter")),
                None => {
                    matches!(
                        page_type,
                        None | Some(
                            T::Worksheet
                                | T::ListPlus
                                | T::NavigatePage
                                | T::StandardDialog
                                | T::ConfirmationDialog
                                | T::PromptDialog
                        )
                    ) || [
                        "rolecenter",
                        "headlinepart",
                        "api",
                        "reportpreview",
                        "reportprocessingonly",
                    ]
                    .iter()
                    .any(|k| other(k))
                }
            },
            IdOwner::ExtensionField => kind == GetFieldKind::Worksheet,
            IdOwner::OtherOfPage => false,
            IdOwner::Host => kind == GetFieldKind::FieldValue && host.is_some(),
            IdOwner::PartPage => false,
            IdOwner::ExtensionControl => {
                return unsupported(
                    "TestPage.GetField with the id of a page extension's control on the page, the host or a part's page (not measured)",
                );
            }
        };
        if !measured {
            return unsupported(
                "TestPage.GetField with an id no control of the page has, on a page kind or of a kind no cell measures (not measured)",
            );
        }
        assume(Assumption::TestPageGetFieldControlId);
        Ok(())
    }

    /// Who else has the id `id` that no control of `st` has as its own: by the base page's id a field a page extension adds or
    /// another control of the page (an actionref, an extension's action), a control of the host, or a control of a part's page;
    /// by the extension's id a control a page extension adds to the page (an actionref), the host or a part's page.
    fn get_field_id_owner(&self, st: &St, id: i32) -> Result<IdOwner, Signal> {
        // The control of `s` whose id by its page's id is `id`: whether a page extension adds it, and whether it is a field.
        let by_page = |s: &St| -> Result<Option<(bool, bool)>, Signal> {
            let page_id = self.object_id_of(s.borrow().object)?;
            let engine = self.engine();
            Ok(page_of(&engine, s)
                .controls
                .iter()
                .find(|c| control_id(page_id, &c.name) == i64::from(id))
                .map(|c| (c.owner.is_some(), c.kind == ir::ControlKind::Field)))
        };
        // Whether a control a page extension adds to `s` has the id `id` by the extension's id.
        let by_extension = |s: &St| -> Result<bool, Signal> {
            let engine = self.engine();
            for c in &page_of(&engine, s).controls {
                if let Some(ext) = c.owner
                    && control_id(self.object_id_of(ext as u32)?, &c.name) == i64::from(id)
                {
                    return Ok(true);
                }
            }
            Ok(false)
        };
        if by_extension(st)? {
            return Ok(IdOwner::ExtensionControl);
        }
        match by_page(st)? {
            Some((true, true)) => return Ok(IdOwner::ExtensionField),
            Some(_) => return Ok(IdOwner::OtherOfPage),
            None => {}
        }
        let host = st.borrow().host.as_ref().and_then(|(h, _)| h.upgrade());
        if let Some(host) = host {
            if by_extension(&host)? {
                return Ok(IdOwner::ExtensionControl);
            }
            match by_page(&host)? {
                Some((false, _)) => return Ok(IdOwner::Host),
                Some((true, _)) => return Ok(IdOwner::ExtensionControl),
                None => {}
            }
        }
        let parts: Vec<St> = {
            let s = st.borrow();
            s.parts.iter().chain(&s.hidden_parts).map(|(_, p)| p.clone()).collect()
        };
        for part in &parts {
            if by_extension(part)? {
                return Ok(IdOwner::ExtensionControl);
            }
            if by_page(part)?.is_some() {
                return Ok(IdOwner::PartPage);
            }
        }
        Ok(IdOwner::None)
    }

    /// `GetField(id)` where no control of the page itself has the id: a field a page extension adds, by the extension's id and
    /// its name (al-runner issue 8944). With `ApplicationArea = All` of its own it is found on a Card (GetFieldByControlId.al
    /// :2804) and a Worksheet (:2576 `ext`) on every version measured; its Value only is measured. With no ApplicationArea it
    /// is found on BC 27 and not found from BC 28 (`al_version::getfield_finds_extension_fields`; :589 and :2822, where BC 27
    /// measured its Value only), on the kinds of `Full`. Every other extension field refuses; any other id is
    /// `get_field_missing`'s.
    fn get_extension_field_by_id(&mut self, st: &St, id: i32) -> Result<Value, Signal> {
        let engine = self.engine();
        let page = page_of(&engine, st);
        let mut found = None;
        for (idx, c) in page.controls.iter().enumerate() {
            if let Some(ext) = c.owner
                && c.action_ref.is_none()
                && control_id(self.object_id_of(ext as u32)?, &c.name) == i64::from(id)
            {
                let area = match c.prop("applicationarea") {
                    None => Some(false),
                    Some(ir::PropVal::Lit(v)) if v.trim().eq_ignore_ascii_case("All") => Some(true),
                    Some(_) => None,
                };
                found = Some((idx, c.kind.clone(), area));
                break;
            }
        }
        drop(engine);
        let Some((idx, ctl_kind, area)) = found else {
            self.get_field_missing(st, id)?;
            return not_found(id);
        };
        let major = self.engine.target.as_ref().map_or(29, |t| t.bc.major);
        let kind = self.get_field_kind(st);
        if area == Some(true) {
            let card = matches!(self.get_field_page_types(st), (Some(al_ir::PageType::Card), None));
            if ctl_kind != ir::ControlKind::Field
                || !(card || kind == GetFieldKind::Worksheet)
                || al_version::getfield_finds_extension_fields(major).is_none()
                || self.control_eliminated(st, idx)
                || self.off_page(st, idx)
            {
                return unsupported(
                    "TestPage.GetField with the id of a page extension's field with ApplicationArea = All, on a page kind or of a shape no cell measures (not measured)",
                );
            }
            assume(Assumption::TestPageGetFieldControlId);
            return Ok(test_control_by_id(st.clone(), idx, ById::ValueOnly));
        }
        if kind != GetFieldKind::Full {
            return unsupported("TestPage.GetField by a control id on a page kind no cell opens (not measured)");
        }
        if area.is_none() {
            return unsupported(
                "TestPage.GetField with the id of a page extension's field with an ApplicationArea other than All (not measured)",
            );
        }
        match (ctl_kind, al_version::getfield_finds_extension_fields(major)) {
            (_, Some(false)) => not_found(id),
            (ir::ControlKind::Field, Some(true)) => {
                assume(Assumption::TestPageGetFieldControlId);
                Ok(test_control_by_id(st.clone(), idx, ById::ValueOnly))
            }
            (_, _) => unsupported(
                "TestPage.GetField with the id of a page extension's control on this BC version, or of one that is no field (not measured)",
            ),
        }
    }

    /// A call `m` on what GetField found (`ControlRef::by_id`): `Some` answers it, `None` lets it run as on the control by its
    /// name. The empty field of a part, a cuegroup, a fixed or a grid answers Value only (GetFieldByControlId.al:545 :630; its
    /// Caption, the part page's, and its Visible are measured at :692 and refused here); so does a page extension's field on
    /// BC 27 (:589), a page extension's field with ApplicationArea = All (:2804, :2576) and a field control on a Worksheet, a
    /// ListPlus or a CardPart a TestPart shows (:2676-:2702).
    pub(super) fn by_id_call(c: &ControlRef, m: PageMethod) -> Result<Option<Value>, Signal> {
        match (c.by_id, m) {
            (None, _) => Ok(None),
            (Some(ById::Empty), PageMethod::Value) => Ok(Some(Value::Text("".into()))),
            (Some(ById::ValueOnly), PageMethod::Value) => Ok(None),
            (Some(ById::Field), m) if BY_ID_CALLS.contains(&m) => Ok(None),
            (Some(_), m) => unsupported(format!(
                "TestField.{} on what TestPage.GetField found by a control id (not measured)",
                m.name()
            )),
        }
    }
}
