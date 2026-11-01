use super::*;

/// A stored request page value that shares nothing with the page it came from or goes to: a data item's record is copied, so
/// filters the report body sets after the page closed do not change what is stored, and an edit on a later page does not change
/// it before an OK (al-runner issue 6065). Other values are copied as they are.
fn detached(v: &Value) -> Value {
    match v {
        Value::Record(r) => Value::Record(Rc::new(RefCell::new(r.borrow().clone()))),
        v => v.clone(),
    }
}

impl Session {
    pub(in crate::pages) fn save_values(&self, st: &St) -> bool {
        let object = st.borrow().object as usize;
        self.engine.program.objects[object]
            .properties
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("SaveValues") && v.trim().eq_ignore_ascii_case("true"))
    }

    /// The variables a SaveValues page stores: (owner, global slot) of every control bound to a variable. On a page that
    /// includes the variables of its page extensions, their own and the base page's protected ones they bind
    /// (misc/lt-probes-needed-6/PnF02SaveValuesExt.al:157-205, SaveValues_Ext_OkThenReopen `second_B`, `second_D`,
    /// `second_E`); a request page keeps its own, and the own ones of a reportextension that changes the run
    /// (al-language-onprem/report/lt-y60-reportext-saveasxml/Y60ReportExtSaveAsXml.al ReportExt_SaveValues_SaveAsXml_NextRun:
    /// `basis=Details showall=Yes`); another report extension's is not pinned.
    pub(in crate::pages) fn var_slots(&self, st: &St) -> Vec<(Option<usize>, usize)> {
        let engine = self.engine();
        let request = self.is_request_page(st);
        let object = st.borrow().object as usize;
        let changing = engine.program.objects[object]
            .report
            .as_ref()
            .map_or(&[][..], |r| r.extensions.as_slice());
        let mut slots: Vec<(Option<usize>, usize)> = page_of(&engine, st)
            .controls
            .iter()
            .filter_map(|c| {
                if request {
                    return match (&c.binding, c.owner) {
                        (ir::Binding::Var(slot), None) => Some((None, *slot)),
                        (ir::Binding::Var(slot), Some(e)) if changing.contains(&e) => Some((Some(e), *slot)),
                        _ => None,
                    };
                }
                if !matches!(c.binding, ir::Binding::Var(_) | ir::Binding::BaseVar(_)) {
                    return None;
                }
                c.binding.var_slot(c.owner).map(|(slot, owner)| (owner, slot))
            })
            .collect();
        slots.sort_unstable();
        slots.dedup();
        slots
    }

    /// A SaveValues request page that a reportextension changing the run extends: the close by SaveAsXml stores the values of the
    /// base and the extension, and the close by Cancel stores nothing (al-language-onprem/report/lt-y60-reportext-saveasxml/
    /// Y60ReportExtSaveAsXml.al ReportExt_SaveValues_SaveAsXml_NextRun). The close by `close` (OK, an error) is not pinned.
    pub(in crate::pages) fn refuse_ext_save_values_close(&self, st: &St, close: &str) -> Result<(), Signal> {
        let object = st.borrow().object as usize;
        let o = &self.engine.program.objects[object];
        let changing = o.report.as_ref().map_or(&[][..], |r| r.extensions.as_slice());
        let extended = o
            .page
            .as_ref()
            .is_some_and(|p| p.controls.iter().any(|c| c.owner.is_some_and(|e| changing.contains(&e))));
        if extended && self.save_values(st) {
            return unsupported(format!(
                "a SaveValues request page that a reportextension changing the run extends, closed by {close} (not pinned)"
            ));
        }
        Ok(())
    }

    pub(in crate::pages) fn store_page_values(&mut self, st: &St) {
        if !self.save_values(st) {
            return;
        }
        if st.borrow().instance.is_none() {
            return;
        }
        let saved: Vec<(Option<usize>, usize, Value)> = self
            .var_slots(st)
            .into_iter()
            .map(|(owner, slot)| {
                let inst = self.owner_instance(st, owner);
                let v = detached(&inst.borrow().globals[slot]);
                (owner, slot, v)
            })
            .collect();
        let object = st.borrow().object;
        self.pages.saved_values.insert(object, saved);
    }

    /// A SaveValues request page closed by Cancel: whether a data item's filters the page changed are kept. The stored record
    /// is a copy (`detached`), so they are not; that a Cancel does not store the page's variables is pinned
    /// (handlers/TestReportRequestPageSaveValues.al, misc/lt-probes-needed-6/PnF02SaveValuesExt.al), for a data item's
    /// filters it is assumed (request-page-cancel-keeps-saved-filters, al-runner issue 6065).
    pub(in crate::pages) fn note_cancelled_saved_filters(&mut self, st: &St) {
        if !self.is_request_page(st) || !self.save_values(st) {
            return;
        }
        let object = st.borrow().object;
        let Some(saved) = self.pages.saved_values.get(&object) else {
            return;
        };
        let Some(inst) = st.borrow().instance.clone() else {
            return;
        };
        let filters = |r: &Rc<RefCell<RecordState>>| -> Vec<(usize, i32, Option<String>)> {
            r.borrow()
                .filters
                .iter()
                .map(|f| (f.field, f.group, self.filter_text(&f.expr).ok()))
                .collect()
        };
        let changed = saved
            .iter()
            .filter(|(owner, _, _)| owner.is_none())
            .any(|(_, slot, v)| {
                let cur = inst.borrow().globals[*slot].clone();
                match (v, &cur) {
                    (Value::Record(s), Value::Record(c)) => filters(s) != filters(c),
                    _ => false,
                }
            });
        if changed {
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::RequestPageCancelKeepsSavedFilters);
        }
    }

    pub(in crate::pages) fn restore_page_values(&mut self, st: &St) {
        let object = st.borrow().object;
        let Some(saved) = self.pages.saved_values.get(&object).cloned() else {
            return;
        };
        let Some(inst) = st.borrow().instance.clone() else {
            return;
        };
        // A request page's variables that the caller set before the run (`SetParameters`) keep those values: corpus
        // reportextensiontrigger/TestPrecompiledReportExtensionDataItems.al (Base Application's "Get Demand To Reserve"
        // has SaveValues and an earlier run confirmed another demand type; the later run still goes by the one
        // `SetParameters` gave, `All` included). Which one wins in BC is assumed.
        let caller_set = self.is_request_page(st)
            && self
                .report
                .prepared
                .iter()
                .any(|w| w.upgrade().is_some_and(|i| Rc::ptr_eq(&i, &inst)));
        for (owner, slot, v) in saved {
            // A page extension's variable, or the page's protected one an extension binds: restored as the page's own
            // (misc/lt-probes-needed-6/PnF02SaveValuesExt.al:157-205).
            if owner.is_some() {
                let ext = self.owner_instance(st, owner);
                ext.borrow_mut().globals[slot] = detached(&v);
                continue;
            }
            // A data item the caller gave a view (`SetTableView`, or the record of `Report.Run`/`RunModal`) keeps that view over the
            // filters stored from an earlier run. Which of the two a request page shows is not pinned: the corpus pins that an OK
            // stores the view's filters (report/lt-requestpage-savevalues-date-text-setvalue/P147SaveValuesFilter.al:
            // P147_Filter_SetTableViewVsStored `plain after OK on the SetTableView run`), not what a later view does against them.
            // Microsoft's "Suggest Fin. Charge Memo Lines" (SaveValues) is run once per memo with that memo as the record.
            if self.is_request_page(st) && matches!(&v, Value::Record(_)) {
                if let Value::Record(cur) = &inst.borrow().globals[slot] {
                    if !cur.borrow().filters.is_empty() && self.report.view_filters_of(cur).is_some() {
                        al_builtins::assumptions::assume(
                            al_builtins::assumptions::Assumption::ReportViewOverStoredRequestFilters,
                        );
                        continue;
                    }
                }
            }
            if caller_set && !matches!(v, Value::Record(_)) {
                // The caller's value stays over the stored one (report/lt-requestpage-savevalues-date-text-setvalue/P147SaveValuesText.al:
                // P147_Text_StoredVsCaller_Ok, P147_Boolean_CallerFalseOverStoredTrue, P147_Option_CallerSelectedOverStoredAll).
                continue;
            }
            // A stored data item record is copied into the record the instance holds: a reportextension's triggers reach the base
            // data item through it (Y60ReportExtSaveAsXml.al ReportExt_SaveValues_SaveAsXml_NextRun, `ext line 1` in a run after a
            // saved one).
            let cur = inst.borrow().globals[slot].clone();
            match (&cur, &v) {
                (Value::Record(c), Value::Record(s)) => {
                    let stored = s.borrow().clone();
                    *c.borrow_mut() = stored;
                }
                _ => inst.borrow_mut().globals[slot] = detached(&v),
            }
        }
    }
}
