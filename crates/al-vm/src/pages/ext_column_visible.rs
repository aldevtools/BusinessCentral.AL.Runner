//! The Visible of a repeater column that a page extension's `modify()` binds to a Boolean variable the extension sets to true
//! in its OnAfterGetRecord (pages/extensions/PageExtModifyVisibleVariable.al:519-547, `ColRec=Yes` after OpenEdit and
//! OpenView, also after a GoToKey or a Last). The column reads Yes when the first OnAfterGetRecord of the open's read set
//! the variable; every other control keeps the value frozen at open (`frozen_visible`).
//!
//! The rule enters only on the measured page, every fact by equality: the page (:86-108), its one extension (:175-204) and
//! what the three triggers do (`measured_columns`), the two rows (:213-226), and the calls the test made since the open
//! (`ext_column_shown`).

use super::pinned_shape::TestCall;
use super::*;
use al_builtins::page_op::PageMethod;

/// The properties page 61822 declares (:88-91).
const PAGE_PROPERTIES: [&str; 4] = ["applicationarea", "pagetype", "sourcetable", "usagecategory"];
/// The triggers page extension 61824 declares (:188-203).
const EXT_TRIGGERS: [&str; 3] = ["onaftergetcurrrecord", "onaftergetrecord", "onopenpage"];

impl Session {
    /// `load_first_row` for the open itself: the read the measured columns take their Visible in.
    pub(super) fn load_first_row_at_open(&mut self, st: &St, mode: PageMode, with_record: bool) -> Result<(), Signal> {
        st.borrow_mut().ext_columns_shown = Vec::new();
        st.borrow_mut().before_first_open_row = true;
        let r = self.load_first_row(st, mode, with_record);
        st.borrow_mut().before_first_open_row = false;
        r
    }

    /// After the page trigger `name` ran: the first OnAfterGetRecord of the open's read notes the measured columns, false at
    /// open, whose variable it set to true (:519-547 `ColRec`).
    pub(super) fn show_ext_columns_after_first_open_row(&mut self, st: &St, name: &str) -> Result<(), Signal> {
        if name != "onaftergetrecord" || !std::mem::take(&mut st.borrow_mut().before_first_open_row) {
            return Ok(());
        }
        let columns = self.measured_columns(st);
        // The two rows of :213-226, in the plain view.
        if columns.is_empty() || !self.rec_view_is(st, 0) || self.view_row_count(st)? != 2 {
            return Ok(());
        }
        let mut shown = Vec::new();
        for idx in columns {
            let frozen = st.borrow().frozen_visible.get(idx).copied();
            if frozen == Some(false) && self.prop_bool_now(st, idx, "visible", true)? {
                shown.push(idx);
            }
        }
        st.borrow_mut().ext_columns_shown = shown;
        Ok(())
    }

    /// Whether control `idx` is a measured column that reads Yes in place of its value frozen at open: after the calls the
    /// cells made since the page opened, Visible() reads and one GoToKey after OpenEdit (:525-528) or one Last after OpenView
    /// (:541-544). After any other call the column reads its frozen value again.
    pub(super) fn ext_column_shown(&self, st: &St, idx: usize) -> bool {
        let s = st.borrow();
        if !s.ext_columns_shown.contains(&idx) || s.test_calls_overflow {
            return false;
        }
        let moved = match s.mode {
            PageMode::Edit => PageMethod::GoToKey,
            PageMode::View => PageMethod::Last,
            _ => return false,
        };
        let moves = s.test_calls.iter().filter(|c| **c == TestCall::Page(moved)).count();
        moves <= 1
            && s.test_calls
                .iter()
                .all(|c| matches!(c, TestCall::Other(PageMethod::Visible)) || *c == TestCall::Page(moved))
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
