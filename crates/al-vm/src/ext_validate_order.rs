//! The order of the OnBeforeValidate and OnAfterValidate triggers table extensions add to one field.
//!
//! - Two extensions of one app run in the order of their object ids, both before and after the field's own OnValidate
//!   (record/lt-modify-field-two/ModifyFieldTwoExtensions.al:148-169, P163_Validate_TwoExtensionsOnOneField_Order: 73328 "Ext A"
//!   then 73329 "Ext B"; events/W4ValidateOrderAndCurrFieldNo.al:331 `ext1.before;ext2.before;base.onvalidate;ext1.after;ext2.after;`,
//!   68065 "Ext One" then 68066 "Ext Two"). In both pins the id order is also the order of the names; when the two orders differ,
//!   or with more than two extensions in one app, the order is not pinned and the Validate is refused.
//! - Extensions of different apps run app by app (al-runner issue 6486), in the order BC visits the apps: an app before the
//!   apps that depend on it, and two apps that do not depend on each other in ascending app id
//!   (tableextension/lt-bw-cross-app/Y20CrossAppValidateOrder.al:44-45, `xa;xb;xd;xc;main`: B and D depend on A only, C on A, B
//!   and D, the test app on A, B and C; B (app id c87ced6d) runs before D (e92be076)). This is the visit order of the event
//!   subscribers across apps (`dispatch::visit_orders`); where its readings that no probe tells apart give different orders
//!   for the apps involved, the Validate is refused.

use crate::Session;
use crate::dispatch::visit_orders;
use al_builtins::native::Signal;
use al_data::TableDef;

type Triggers = Vec<(usize, usize)>;

impl Session {
    /// The OnBeforeValidate and OnAfterValidate triggers table extensions declare for `field`, in the order they run.
    pub(crate) fn ext_validate_order(
        &self,
        t: &TableDef,
        field: &al_ir::Field,
    ) -> Result<(Triggers, Triggers), Signal> {
        let (mut before, mut after) = (field.ext_before_validate.clone(), field.ext_after_validate.clone());
        if before.len() <= 1 && after.len() <= 1 {
            return Ok((before, after));
        }
        let refuse = || {
            Err(Signal::Unsupported(format!(
                "Validate of {}.{}: more than one table extension declares OnBeforeValidate or OnAfterValidate for it (their order is not pinned)",
                t.name, field.name
            )))
        };
        let program = &self.engine.program;
        let objects = &program.objects;
        let mut involved: Vec<usize> = before.iter().chain(&after).map(|&(o, _)| o).collect();
        involved.sort_unstable();
        involved.dedup();
        if [&before, &after]
            .iter()
            .any(|l| (1..l.len()).any(|i| l[..i].iter().any(|x| x.0 == l[i].0)))
        {
            return refuse();
        }
        let mut apps: Vec<usize> = involved.iter().map(|&o| objects[o].app as usize).collect();
        apps.sort_unstable();
        apps.dedup();
        // Within one app: at most two extensions, whose id order is their name order.
        for &app in &apps {
            let mut by_id: Vec<usize> = involved
                .iter()
                .copied()
                .filter(|&o| objects[o].app as usize == app)
                .collect();
            if by_id.len() > 2 {
                return refuse();
            }
            by_id.sort_by_key(|&o| objects[o].id);
            let mut by_name = by_id.clone();
            by_name.sort_by_key(|&o| objects[o].name.to_ascii_lowercase());
            if by_id != by_name || by_id.windows(2).any(|w| objects[w[0]].id == objects[w[1]].id) {
                return refuse();
            }
        }
        // Across apps: the visit order, the same under every reading.
        if apps.len() > 1 {
            let mut orders = visit_orders(&program.apps)
                .into_iter()
                .map(|(_, full)| full.into_iter().filter(|a| apps.contains(a)).collect::<Vec<usize>>());
            let first = orders.next().unwrap_or_default();
            if first.len() != apps.len() || orders.any(|o| o != first) {
                return refuse();
            }
            apps = first;
        }
        let rank = |o: usize| (apps.iter().position(|&a| a == objects[o].app as usize), objects[o].id);
        before.sort_by_key(|&(o, _)| rank(o));
        after.sort_by_key(|&(o, _)| rank(o));
        Ok((before, after))
    }
}
