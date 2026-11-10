//! Which arm of a conditional TableRelation a TestPage `Lookup()` opens.

use super::*;

impl Session {
    /// The arm of the relation of `field` (the source field of control `idx`) that applies to the page's row, for
    /// `TestField.Lookup`: `if (Kind = const(1)) A else if (Kind = const(2)) B` opens the lookup page of the table whose
    /// condition holds for the row as the page shows it, also after a SetValue on the condition field
    /// (testpage/P133RelationLookups.al:262-296 first and second arm, :318-338 the condition changed on the page, :340-356
    /// cancel; al-runner issue 7454). `None` when no arm applies: no page opens, no handler is needed and the value stays
    /// (:298-316). Conditions other than `Field = const(x)` on a stored field are not pinned for a lookup and refused.
    pub(super) fn lookup_relation_arm(
        &mut self,
        st: &St,
        idx: usize,
        field: &ir::Field,
    ) -> Result<Option<ir::RelationArm>, Signal> {
        // One arm without a condition, as before (testpage/TestPageFieldRelationLookup_Tests.al:104).
        if let [arm] = field.relation_arms.as_slice() {
            if arm.conditions.is_empty() {
                return Ok(Some(arm.clone()));
            }
        }
        if field.relation_arms.is_empty() {
            return unsupported(
                "Lookup over a TableRelation with filter() conditions or an unreadable one (not pinned)",
            );
        }
        let engine = self.engine();
        let ir::Binding::Field(pos) = page_of(&engine, st).controls[idx].binding else {
            return unsupported("TestField.Lookup on a control that is not bound to a table field");
        };
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        let on_flow_field = field
            .relation_arms
            .iter()
            .flat_map(|arm| &arm.conditions)
            .any(|c| !table.fields[c.field].stored);
        if on_flow_field {
            return unsupported(
                "Lookup over a TableRelation whose condition names a FlowField or a FlowFilter (not pinned)",
            );
        }
        self.relation_arm(&rec, &table, pos)
    }
}
