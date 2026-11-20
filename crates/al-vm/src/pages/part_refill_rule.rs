//! The pages and tables whose part's calls after the test's own filter answer from the rows that filter read again (al-runner
//! issue 8537, `part_update_left_view.rs`): a rule over the facts the probes varied, instead of the objects of one probe. Every
//! fact cites the cells that measured it; a fact no cell varied is required as the cells had it.
//!
//! The cells: pages/parts/PartRefillTestFilter.al (corpus PR 1474, merged, not pinned yet; the two-field key cell
//! PartRefill_TypedKeyTriggerPartActionThenFilterFirst_FindsRefilledRow in corpus PR 1475, open) and pages/parts/
//! PartRefillShapes.al (corpus PR 1487, open), each green on a local Business Central 29.1, and the reporter's repro of issue
//! 8537. In every one the test's filter read the refilled rows again and the part's calls answered from them.

use super::*;

/// The page triggers the part may declare: none (PartRefillTestFilter.al:90 "LT PR Load Part"), or OnOpenPage and
/// OnAfterGetRecord (:323 "LT PR Load Trig Part", PartRefillShapes.al `LT PS Group Part`, `LT PS Drill Part`, `LT PS Full Part`).
const PART_TRIGGERS: [&str; 2] = ["onopenpage", "onaftergetrecord"];

/// The page triggers the host may declare: OnAfterGetCurrRecord (every cell), OnOpenPage (`LT PS Visible Host`, `LT PS Trig
/// Host`), OnAfterGetRecord (`LT PS Trig Host`), OnPageBackgroundTaskCompleted (:248 "LT PR Load Bg Host") and
/// OnPageBackgroundTaskError (`LT PS BgErr Host`); `LT PS Full Host` has them all.
const HOST_TRIGGERS: [&str; 5] = [
    "onopenpage",
    "onaftergetrecord",
    "onaftergetcurrrecord",
    "onpagebackgroundtaskcompleted",
    "onpagebackgroundtaskerror",
];

impl Session {
    /// The position of the part's key field the test's filter must be on, when host `host` and part `part` have the facts every
    /// cell shared; `None` otherwise:
    /// - the host: a top-level page (no part of a part), PageType Worksheet (every cell but one) or List (:200 "LT PR Load List
    ///   Host"), page triggers among `HOST_TRIGGERS`, no page extension and no subscriber. Its controls, actions, procedures and
    ///   the code of its triggers are not compared: the cells varied them (`LT PS Code Host`, `LT PS Filter Host`, `LT PS Full
    ///   Host`) without a change in what the part shows;
    /// - its other parts: ListParts and CardParts, with any code (`LT PS Parts Host`, `LT PS PartsCode Host`);
    /// - the part's control: in the content area, not linked (no SubPageLink, SubPageView or provider), with no property but
    ///   ApplicationArea and Visible, Visible a literal or a page variable (`LT PS Visible Host`);
    /// - the part: PageType ListPart over a temporary source, page triggers among `PART_TRIGGERS`, no page extension and no
    ///   subscriber, no action; field controls bound to a field of Rec with no trigger but OnDrillDown (`LT PS Drill Part`), a
    ///   control bound to a page variable with no trigger but OnValidate (`LT PS Group Part`). Its procedures are not compared:
    ///   the cells varied them (enum parameters, `LT PS Enum Part`; Rec handed by var to a codeunit, `LT PS Filler Part`);
    /// - the part's table: no trigger, no table extension, no subscriber; a primary key of one field (:52 "LT PR Load Line") or
    ///   two (PR 1475's "LT PR Load Line Typed", Base Application's Load Buffer in `LT PS Buffer Part`), the filter on its last.
    pub(super) fn refill_rule(&self, host: &St, part: &St) -> Option<usize> {
        let engine = self.engine();
        let host_object = host.borrow().object as usize;
        let part_object = part.borrow().object as usize;
        let host_type = engine.object_facts[host_object].page_type.clone();
        let top_host =
            host.borrow().host.is_none() && matches!(host_type, Some(ir::PageType::Worksheet | ir::PageType::List));
        let list_part = engine.object_facts[part_object].page_type == Some(ir::PageType::Other("listpart".into()));
        if !(top_host
            && list_part
            && page_code_within(&engine, host, &HOST_TRIGGERS)
            && page_code_within(&engine, part, &PART_TRIGGERS)
            && self.part_control_measured(&engine, host, part_object)
            && part_controls_measured(&engine, part))
        {
            return None;
        }
        let rec = self.page_rec(part);
        if !rec.borrow().temporary {
            return None;
        }
        let table = self.table(&rec).ok()?;
        let table_object = table.object as usize;
        let plain_table = engine.program.objects[table_object]
            .routines
            .iter()
            .all(|r| !r.is_trigger)
            && engine.program.objects[table_object]
                .table
                .as_ref()
                .is_some_and(|t| t.extensions.is_empty())
            && !engine.events.any_subscriber_of(table_object);
        if !plain_table {
            return None;
        }
        match table.keys[0].as_slice() {
            [key] | [_, key] => Some(*key),
            _ => None,
        }
    }

    /// The host shows part `part_object` in the content area, unlinked, with no property but ApplicationArea and a Visible that is
    /// a literal or a page variable; its other parts are ListParts and CardParts.
    fn part_control_measured(&self, engine: &crate::Engine, host: &St, part_object: usize) -> bool {
        let page = page_of(engine, host);
        let program = &engine.program;
        let host_object = host.borrow().object as usize;
        let mut ours = 0;
        for c in page.controls.iter().filter(|c| c.kind == ir::ControlKind::Part) {
            let Some(p) = c.part_page else {
                return false;
            };
            if p != part_object {
                let kind = engine.object_facts[p].page_type.clone();
                if !matches!(kind, Some(ir::PageType::Other(k)) if k == "listpart" || k == "cardpart") {
                    return false;
                }
                continue;
            }
            ours += 1;
            let plain = !c.factbox
                && c.part_link
                    .as_ref()
                    .is_none_or(|l| l.entries.is_empty() && l.view.is_none() && l.provider.is_none())
                && c.props.iter().all(|(k, v)| match (k.as_str(), v) {
                    ("applicationarea", ir::PropVal::Lit(_)) | ("visible", ir::PropVal::Lit(_)) => true,
                    ("visible", ir::PropVal::Routine(r)) => returns_page_variable(program, host_object, *r),
                    _ => false,
                });
            if !plain {
                return false;
            }
        }
        ours == 1
    }
}

/// Page `st`'s triggers are among `allowed`, and no page extension or subscriber adds code.
fn page_code_within(engine: &crate::Engine, st: &St, allowed: &[&str]) -> bool {
    let object = st.borrow().object as usize;
    let page = page_of(engine, st);
    page.triggers.iter().all(|(n, _)| allowed.contains(&n.as_str()))
        && page.extensions.is_empty()
        && page.ext_triggers.is_empty()
        && !engine.events.any_subscriber_of(object)
}

/// The part's controls: no action; a field of Rec with no trigger but OnDrillDown; a page variable with no trigger but
/// OnValidate; groups and repeaters without triggers.
fn part_controls_measured(engine: &crate::Engine, part: &St) -> bool {
    page_of(engine, part).controls.iter().all(|c| {
        let only = |allowed: &str| c.triggers.iter().all(|(t, _)| t == allowed) && c.ext_triggers.is_empty();
        match (&c.kind, &c.binding) {
            (ir::ControlKind::Field, ir::Binding::Field(_)) => only("ondrilldown"),
            (ir::ControlKind::Field, ir::Binding::Var(_)) => only("onvalidate"),
            (ir::ControlKind::Group | ir::ControlKind::Repeater, ir::Binding::None) => c.triggers.is_empty(),
            _ => false,
        }
    })
}

/// Routine `r` of `object` (an expression property's) is `exit(<page variable>)`.
fn returns_page_variable(program: &ir::Program, object: usize, r: usize) -> bool {
    let Some(routine) = program.objects[object].routines.get(r) else {
        return false;
    };
    let mut stmts = routine.body.stmts().iter().filter(|s| !matches!(s, ir::Stmt::Pos(_)));
    matches!(
        (stmts.next(), stmts.next()),
        (Some(ir::Stmt::Exit(Some(ir::Expr::Load(ir::Place::Global(_))))), None)
    )
}
