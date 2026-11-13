//! Resolves report extension data items and binds the records their owners supply.
use super::*;

pub(super) fn item_anchor(report: &ir::Report, owner: usize, anchor: ir::ItemAnchor) -> Option<ItemRef> {
    match anchor {
        ir::ItemAnchor::Base(item) => Some(ItemRef::Base(item)),
        ir::ItemAnchor::Added(item) => Some(ItemRef::Added(owner, item)),
        ir::ItemAnchor::Foreign(item) => report
            .extensions
            .iter()
            .position(|&e| e == item.extension)
            .map(|k| ItemRef::Added(k, item.item)),
    }
}

impl Session {
    pub(super) fn bind_report_item_aliases(
        &self,
        report: &ir::Report,
        inst: &Inst,
        ext_inst: &Inst,
        eir: &ir::ReportExt,
        exts: &[(Inst, usize)],
    ) -> Result<(), Signal> {
        for &(anchor, slot) in &eir.aliases {
            let record = match anchor {
                ir::ItemAnchor::Base(item) => {
                    let base_slot = report.items[item].slot.expect("an alias has a record");
                    inst.borrow().globals[base_slot].clone()
                }
                ir::ItemAnchor::Foreign(item) => {
                    let (owner, e) = exts
                        .iter()
                        .find(|(_, e)| *e == item.extension)
                        .ok_or_else(|| Signal::unsupported("the owner of a report data item did not load"))?;
                    let owner_ir = self.engine.program.objects[*e]
                        .report_ext
                        .as_ref()
                        .expect("a report data item has a report extension owner");
                    let owner_slot = owner_ir.added[item.item]
                        .item
                        .slot
                        .ok_or_else(|| Signal::unsupported("a dependency report data item has no record slot"))?;
                    owner.borrow().globals[owner_slot].clone()
                }
                ir::ItemAnchor::Added(_) => unreachable!("an extension owns its local records"),
            };
            ext_inst.borrow_mut().globals[slot] = record;
        }
        Ok(())
    }
}

/// ADR-075: only static SaveAs to an XML stream with empty parameters and no RecordRef is pinned.
pub(crate) fn require_pinned_entry(report: &ir::Report, pinned: bool) -> Result<(), Signal> {
    if !pinned && uses_dependency_items(report) {
        return unsupported(
            "dependency report data items require static Report.SaveAs to an XML stream with empty parameters and no RecordRef",
        );
    }
    Ok(())
}

impl Session {
    pub(super) fn dataset_extension_instance(
        &mut self,
        inst: &Inst,
        extension: usize,
        report: &ir::Report,
    ) -> Result<Inst, Signal> {
        if uses_dependency_items(report) {
            // The static XML path already ran OnInitReport. This is dataset setup, not a report-variable call.
            Ok(self.report_extension_state(inst, extension))
        } else {
            self.report_extension_instance(inst, extension)
        }
    }

    pub(super) fn report_extension_state(&mut self, base: &Inst, ext: usize) -> Inst {
        if let Some((_, inst)) = base.borrow().exts.iter().find(|(e, _)| *e as usize == ext) {
            return inst.clone();
        }
        let inst = self.extension_instance(ext, base);
        base.borrow_mut().exts.push((ext as u32, inst.clone()));
        inst
    }
}

pub(super) fn check_static_save(
    schema: &Schema,
    format: &str,
    parameters: &str,
    record_ref: Option<&Value>,
    target: &SaveTarget,
) -> Result<bool, Signal> {
    let pinned =
        format == "Xml" && parameters.is_empty() && record_ref.is_none() && matches!(target, SaveTarget::Stream(_));
    require_pinned_entry(schema.report(), pinned)?;
    Ok(pinned)
}

impl Session {
    // A report variable remembers an earlier SaveAs or Execute on the same instance.
    pub(super) fn save_variable_start(&mut self, inst: &Inst, result_used: bool) -> Result<Option<Value>, Signal> {
        let ran = |list: &Vec<std::rc::Weak<std::cell::RefCell<Instance>>>| {
            list.iter().any(|w| w.upgrade().is_some_and(|i| Rc::ptr_eq(&i, inst)))
        };
        let tried = ran(&self.report.report_save_tried);
        if ran(&self.report.report_executed) {
            if !tried {
                return unsupported("a SaveAs on a report variable that an Execute ran (not pinned)");
            }
            if !result_used {
                return unsupported("a second SaveAs on one report variable, with the answer dropped (not pinned)");
            }
            // The text is pinned (PnBReportSaveAs.al:387), and the code (:749-750, :755-756 PW_SecondSaveAs_LastErrorCode).
            self.last_error = running::already_run_error();
            return Ok(Some(Value::Boolean(false)));
        }
        if tried {
            return unsupported("a second SaveAs on one report variable after the first ended early (not pinned)");
        }
        self.report.report_save_tried.retain(|w| w.strong_count() > 0);
        self.report.report_save_tried.push(Rc::downgrade(inst));
        Ok(None)
    }
}

fn uses_dependency_items(report: &ir::Report) -> bool {
    report
        .assumes
        .contains(&al_builtins::assumptions::Assumption::DependencyReportDataItems)
}
