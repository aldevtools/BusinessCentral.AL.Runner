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
                    al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::DependencyReportDataItems);
                    owner.borrow().globals[owner_slot].clone()
                }
                ir::ItemAnchor::Added(_) => unreachable!("an extension owns its local records"),
            };
            ext_inst.borrow_mut().globals[slot] = record;
        }
        Ok(())
    }
}
