//! The Visible of a repeater column that a page extension's `modify()` binds to a Boolean variable of the extension: on a List
//! opened by OpenEdit or OpenView, the value counts as it stands after the extension's first OnAfterGetRecord of the open's
//! read, and before OnAfterGetCurrRecord (pages/extensions/PageExtModifyVisibleVariable.al:519-547: `ColRec` set in
//! OnAfterGetRecord reads Yes, `ColCurr` set in OnAfterGetCurrRecord reads No, `ColOpen` set in OnOpenPage reads Yes, also
//! after a move). A later OnAfterGetRecord of the same read that gives another value is not measured and is refused. Every
//! other control keeps the value frozen at open (`frozen_visible`).

use super::state::ExtColumnsAtOpen;
use super::*;

impl Session {
    /// `load_first_row` for the open itself: the read the extension columns' Visible counts after.
    pub(super) fn load_first_row_at_open(&mut self, st: &St, mode: PageMode, with_record: bool) -> Result<(), Signal> {
        st.borrow_mut().ext_columns_at_open = ExtColumnsAtOpen::Open;
        let r = self.load_first_row(st, mode, with_record);
        st.borrow_mut().ext_columns_at_open = ExtColumnsAtOpen::Off;
        r
    }

    /// After the page trigger `name` ran: in the open's read on the pinned shape, the first OnAfterGetRecord takes the
    /// columns' Visible, a later one must give the same values.
    pub(super) fn refreeze_ext_columns_after_open_read(&mut self, st: &St, name: &str) -> Result<(), Signal> {
        let at_open = st.borrow().ext_columns_at_open;
        if name != "onaftergetrecord" || at_open == ExtColumnsAtOpen::Off {
            return Ok(());
        }
        let columns = self.pinned_ext_columns(st);
        for idx in columns {
            let now = self.prop_bool_now(st, idx, "visible", true)?;
            let mut s = st.borrow_mut();
            let Some(slot) = s.frozen_visible.get_mut(idx) else {
                continue;
            };
            if at_open == ExtColumnsAtOpen::Taken && *slot != now {
                return unsupported(
                    "Visible of a page extension column whose variable differs between the rows the page reads as it opens \
                     (not pinned)",
                );
            }
            *slot = now;
        }
        st.borrow_mut().ext_columns_at_open = ExtColumnsAtOpen::Taken;
        Ok(())
    }

    /// The controls of the pinned shape, every fact by equality; empty for every other page.
    fn pinned_ext_columns(&self, st: &St) -> Vec<usize> {
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
        // A page of type List (:88) that declares no trigger (:86-108).
        let page = page_of(&engine, st);
        if engine.object_facts[object].page_type != Some(al_ir::PageType::List) || !page.triggers.is_empty() {
            return Vec::new();
        }
        // One page extension declares OnOpenPage, OnAfterGetRecord and OnAfterGetCurrRecord (:175-204), no other trigger.
        let Some(&(_, ext, _)) = page.ext_triggers.first() else {
            return Vec::new();
        };
        let mut names: Vec<&str> = page.ext_triggers.iter().map(|(n, _, _)| n.as_str()).collect();
        names.sort_unstable();
        if page.ext_triggers.iter().any(|&(_, e, _)| e != ext)
            || names != ["onaftergetcurrrecord", "onaftergetrecord", "onopenpage"]
        {
            return Vec::new();
        }
        let ext_object = &engine.program.objects[ext];
        (0..page.controls.len())
            .filter(|&idx| {
                let c = &page.controls[idx];
                // A field whose parent is the repeater (:97-104) and declares no Visible (:99-104).
                let in_repeater = c
                    .parent
                    .is_some_and(|p| page.controls[p].kind == ir::ControlKind::Repeater);
                let own_visible = c.ext_replaced.iter().any(|p| p == "visible");
                // Visible bound by that extension's modify() to one of its global Boolean variables (:177-180, :182-186).
                let bound = matches!(c.prop("visible"), Some(ir::PropVal::Ext(e, r))
                    if *e == ext && returns_boolean_global(ext_object, *r));
                c.kind == ir::ControlKind::Field && in_repeater && !own_visible && bound
            })
            .collect()
    }
}

/// Whether routine `r` of `object` only returns one of the object's global Boolean variables.
fn returns_boolean_global(object: &ir::Object, r: usize) -> bool {
    let Some(routine) = object.routines.get(r) else {
        return false;
    };
    let mut stmts = routine.body.stmts().iter().filter(|s| !matches!(s, ir::Stmt::Pos(_)));
    let global = match (stmts.next(), stmts.next()) {
        (Some(ir::Stmt::Exit(Some(ir::Expr::Load(ir::Place::Global(g))))), None) => *g,
        _ => return false,
    };
    object
        .globals
        .get(global)
        .is_some_and(|slot| slot.ty == al_builtins::value::RtType::Boolean)
}
