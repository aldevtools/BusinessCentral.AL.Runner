//! The rest of `ClearAll()` in a page extension: the base page's globals.

use super::*;

impl Session {
    /// `ClearAll()` in a page extension resets the base page's globals as well as its own, and keeps Rec: a page global set in
    /// OnOpenPage and one set in OnAfterGetRecord read 0 once the extension's OnAfterGetRecord called ClearAll, and the row
    /// stays (pages/extensions/CyClearAllExtensions.al:166-193 CY_ClearAll_InPageExtension_OnAfterGetRecord). The extension's
    /// own globals go in a procedure called through a Page variable that is not open and in its OnOpenPage
    /// (record/ProbeClearAllSetKeyHyperlink.al:389-412 ClearAll_InPageExtension_WhatIsCleared). The base page keeps Rec, xRec
    /// and CurrPage, which it shares with the extension. A second extension of the page with globals, or a global temporary
    /// record in the base page, is not pinned and refused.
    pub(crate) fn clear_all_base_page(&mut self, f: &Frame) -> Result<Value, Signal> {
        let own = f.instance.borrow().object as usize;
        let engine = self.engine.clone();
        let objects = &engine.program.objects;
        let curr = objects[own].globals.iter().position(|g| g.name == "CurrPage");
        let st = curr.and_then(|c| match f.instance.borrow().globals.get(c) {
            Some(Value::TestPage(st)) => Some(st.clone()),
            _ => None,
        });
        let Some(st) = st else {
            return Err(Signal::Unsupported(
                "ClearAll in a page extension that has no page instance (not pinned)".into(),
            ));
        };
        let ext_keep = |o: usize| -> Vec<usize> {
            let ob = &objects[o];
            ob.rec_slot
                .into_iter()
                .chain(ob.xrec_slot)
                .chain(ob.globals.iter().position(|g| g.name == "CurrPage"))
                .collect()
        };
        let second = st.borrow().ext_instances.iter().any(|(e, i)| {
            !Rc::ptr_eq(i, &f.instance) && (0..objects[*e].globals.len()).any(|g| !ext_keep(*e).contains(&g))
        });
        if second {
            return Err(Signal::Unsupported(
                "ClearAll in a page extension while a second extension of the page holds globals (not pinned)".into(),
            ));
        }
        let Some(base) = st.borrow().instance.clone() else {
            return Ok(Value::Null);
        };
        let bo = &objects[base.borrow().object as usize];
        let keep: Vec<usize> = bo
            .rec_slot
            .into_iter()
            .chain(bo.xrec_slot)
            .chain(bo.page.as_ref().map(|p| p.curr_page_slot))
            .collect();
        let tys: Vec<RtType> = bo.globals.iter().map(|g| g.ty.clone()).collect();
        let fresh: Vec<Value> = tys.iter().map(|t| self.default_value(t)).collect();
        let mut b = base.borrow_mut();
        for (g, v) in fresh.into_iter().enumerate() {
            if keep.contains(&g) {
                continue;
            }
            if matches!(&b.globals[g], Value::Record(r) if r.borrow().temporary) {
                return Err(Signal::Unsupported(
                    "ClearAll in a page extension whose base page has a global temporary record (not pinned)".into(),
                ));
            }
            b.globals[g] = v;
        }
        Ok(Value::Null)
    }
}
