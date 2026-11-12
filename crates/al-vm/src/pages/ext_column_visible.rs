//! The Visible of a repeater column that a page extension's `modify()` binds to a Boolean variable the extension sets to true
//! in its OnAfterGetRecord (pages/extensions/PageExtModifyVisibleVariable.al:519-547, `ColRec=Yes` after OpenEdit and
//! OpenView, also after a move). The column reads the variable after the first OnAfterGetRecord of the open's read; every
//! other control keeps the value frozen at open (`frozen_visible`).
//!
//! The rule enters only on the measured page, every fact by equality (`measured_columns`): the page (:86-108), its one
//! extension (:175-204) and what the three triggers do.

use super::*;

/// The properties page 61822 declares (:88-91).
const PAGE_PROPERTIES: [&str; 4] = ["applicationarea", "pagetype", "sourcetable", "usagecategory"];
/// The triggers page extension 61824 declares (:188-203).
const EXT_TRIGGERS: [&str; 3] = ["onaftergetcurrrecord", "onaftergetrecord", "onopenpage"];

impl Session {
    /// `load_first_row` for the open itself: the read the measured columns take their Visible in.
    pub(super) fn load_first_row_at_open(&mut self, st: &St, mode: PageMode, with_record: bool) -> Result<(), Signal> {
        st.borrow_mut().before_first_open_row = true;
        let r = self.load_first_row(st, mode, with_record);
        st.borrow_mut().before_first_open_row = false;
        r
    }

    /// After the page trigger `name` ran: the first OnAfterGetRecord of the open's read turns the measured columns, false at
    /// open (:519-547 `ColRec`), to true.
    pub(super) fn show_ext_columns_after_first_open_row(&mut self, st: &St, name: &str) -> Result<(), Signal> {
        if name != "onaftergetrecord" || !std::mem::take(&mut st.borrow_mut().before_first_open_row) {
            return Ok(());
        }
        for idx in self.measured_columns(st) {
            let now = self.prop_bool_now(st, idx, "visible", true)?;
            if let Some(slot) = st.borrow_mut().frozen_visible.get_mut(idx)
                && !*slot
                && now
            {
                *slot = true;
            }
        }
        Ok(())
    }

    /// The columns of the measured shape; empty for every other page.
    fn measured_columns(&self, st: &St) -> Vec<usize> {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        {
            let s = st.borrow();
            // A top-level page the test opened with OpenEdit or OpenView (:525, :541), not modal, not a part.
            let opened = s.opened_by_test
                && matches!(s.mode, PageMode::Edit | PageMode::View)
                && !s.modal
                && s.host.is_none()
                && !s.frozen_visible.is_empty();
            if !opened {
                return Vec::new();
            }
        }
        let page = page_of(&engine, st);
        let Some(ext) = measured_extension(&engine, object, page) else {
            return Vec::new();
        };
        let ext_object = &engine.program.objects[ext];
        let Some(set_in_rec) = sets_one_global_true(ext_object, "onaftergetrecord", page) else {
            return Vec::new();
        };
        (0..page.controls.len())
            .filter(|&idx| {
                let c = &page.controls[idx];
                // A column that declares ApplicationArea only (:104) and whose Visible that extension's modify() binds to the
                // variable its OnAfterGetRecord sets (:180, :186, :198).
                let mut props: Vec<&str> = c.props.iter().map(|(n, _)| n.as_str()).collect();
                props.sort_unstable();
                let bound = matches!(c.prop("visible"), Some(ir::PropVal::Ext(e, r))
                    if *e == ext && returned_global(ext_object, *r) == Some(set_in_rec));
                c.kind == ir::ControlKind::Field
                    && c.ext_replaced.is_empty()
                    && props == ["applicationarea", "visible"]
                    && bound
            })
            .collect()
    }
}

/// The one page extension of the measured page, when page and extension are as measured: a List that declares the four
/// properties of :88-91, no trigger and no action, and one repeater holding every field (:93-106); one extension that
/// declares the three triggers of :188-203, each setting one Boolean variable of its own to true, and no other code but the
/// Visible expressions of its `modify()` blocks (:177-180).
fn measured_extension(engine: &Engine, object: usize, page: &ir::Page) -> Option<usize> {
    let mut properties: Vec<String> = engine.program.objects[object]
        .properties
        .iter()
        .map(|(n, _)| n.to_ascii_lowercase())
        .collect();
    properties.sort_unstable();
    let is_list = engine.object_facts[object].page_type == Some(al_ir::PageType::List);
    let repeaters = page
        .controls
        .iter()
        .filter(|c| c.kind == ir::ControlKind::Repeater && c.parent.is_none())
        .count();
    let fields_in_repeater = page.controls.iter().all(|c| match c.kind {
        ir::ControlKind::Repeater => c.parent.is_none(),
        ir::ControlKind::Field => c
            .parent
            .is_some_and(|p| page.controls[p].kind == ir::ControlKind::Repeater),
        _ => false,
    });
    let &[ext] = page.extensions.as_slice() else {
        return None;
    };
    let mut names: Vec<&str> = page.ext_triggers.iter().map(|(n, _, _)| n.as_str()).collect();
    names.sort_unstable();
    let base = is_list
        && properties == PAGE_PROPERTIES
        && page.triggers.is_empty()
        && page.action_areas.is_empty()
        && repeaters == 1
        && fields_in_repeater;
    if !base || names != EXT_TRIGGERS || page.ext_triggers.iter().any(|&(_, e, _)| e != ext) {
        return None;
    }
    let ext_object = &engine.program.objects[ext];
    // Three different variables, one per trigger (:190, :195, :200).
    let mut set: Vec<usize> = EXT_TRIGGERS
        .iter()
        .filter_map(|name| sets_one_global_true(ext_object, name, page))
        .collect();
    set.sort_unstable();
    set.dedup();
    let triggers: Vec<usize> = page.ext_triggers.iter().map(|&(_, _, r)| r).collect();
    let only_measured_code =
        (0..ext_object.routines.len()).all(|r| triggers.contains(&r) || returned_global(ext_object, r).is_some());
    (set.len() == EXT_TRIGGERS.len() && only_measured_code).then_some(ext)
}

/// The global Boolean variable the extension's page trigger `name` sets to true, when that is all the trigger does.
fn sets_one_global_true(ext_object: &ir::Object, name: &str, page: &ir::Page) -> Option<usize> {
    let &(_, _, r) = page.ext_triggers.iter().find(|(n, _, _)| n == name)?;
    let mut stmts = ext_object
        .routines
        .get(r)?
        .body
        .stmts()
        .iter()
        .filter(|s| !matches!(s, ir::Stmt::Pos(_)));
    match (stmts.next(), stmts.next()) {
        (Some(ir::Stmt::Assign(ir::Place::Global(g), ir::Expr::Const(ir::Const::Boolean(true)))), None) => {
            boolean_global(ext_object, *g)
        }
        _ => None,
    }
}

/// The global Boolean variable routine `r` of `object` returns, when that is all the routine does.
fn returned_global(object: &ir::Object, r: usize) -> Option<usize> {
    let mut stmts = object
        .routines
        .get(r)?
        .body
        .stmts()
        .iter()
        .filter(|s| !matches!(s, ir::Stmt::Pos(_)));
    match (stmts.next(), stmts.next()) {
        (Some(ir::Stmt::Exit(Some(ir::Expr::Load(ir::Place::Global(g))))), None) => boolean_global(object, *g),
        _ => None,
    }
}

fn boolean_global(object: &ir::Object, g: usize) -> Option<usize> {
    object
        .globals
        .get(g)
        .is_some_and(|slot| slot.ty == al_builtins::value::RtType::Boolean)
        .then_some(g)
}
