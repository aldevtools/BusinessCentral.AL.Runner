//! The facts an entry predicate of a rule that is pinned for ONE corpus scenario compares with that scenario: the code the
//! page and its table can run, the properties the page declares, and the calls the test made on the page since it opened.
//! Each predicate states the pinned value of every fact and requires equality (al-runner PR 7807 review, round 3: a rule that
//! asks only "is not X" also runs on shapes no probe shows). The predicates are next to their rules: `long_view.rs`,
//! `open/temp_ws_preloaded.rs`, `currpage_run_typed_key.rs`.

use super::*;
use al_builtins::page_op::{PageMethod, PageOp, PageRecv};

/// One call of the test client on a page (a TestPage, TestField, TestAction or TestFilter method; the page's own `CurrPage`
/// calls and a Page variable's methods are not test calls).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestCall {
    /// A method of the page: `First`, `Next`, `Last`, `Editable`, ...
    Page(PageMethod),
    /// A control's value read: `P.Name.Value()`.
    Read,
    /// A value typed into the control at this index of the page's control list: `SetValue`, `Value := x`.
    Write(usize),
    /// Any other method of a control, an action or the filter.
    Other(PageMethod),
}

/// More calls than this are not kept; a page with that many is no pinned scenario (`test_calls_overflow`).
const KEPT_CALLS: usize = 256;

/// The code and the properties of a pinned page (see `Session::page_code_is`).
pub(super) struct PinnedCode<'a> {
    /// The page's own triggers, lower case: exactly these.
    pub triggers: &'a [&'a str],
    /// The triggers of the page's controls, lower case, over all controls: exactly these (a name once per control that has it).
    pub control_triggers: &'a [&'a str],
    /// The properties the page may declare, lower case, next to PageType, SourceTable, ApplicationArea, UsageCategory and
    /// Caption (which run nothing and change no read). The predicate checks the value of each one it lists.
    pub properties: &'a [&'a str],
}

impl Session {
    /// The test client calls `method` on `st`, on the page itself or on `control`.
    pub(super) fn note_test_call(st: &St, method: PageOp, control: Option<&ControlTarget>, args: usize) {
        if method.recv() == PageRecv::Page {
            return;
        }
        let m = method.method();
        let call = match (control, m) {
            (None, _) => TestCall::Page(m),
            (Some(_), PageMethod::Value) if args == 0 => TestCall::Read,
            (Some(ControlTarget::Control(i)), PageMethod::Value | PageMethod::SetValue) => TestCall::Write(*i),
            (Some(_), _) => TestCall::Other(m),
        };
        let mut s = st.borrow_mut();
        if s.test_calls.len() < KEPT_CALLS {
            s.test_calls.push(call);
        } else {
            s.test_calls_overflow = true;
        }
    }

    /// The calls the test made on the page since it opened, the running one last; `None` when there were too many to keep.
    pub(super) fn test_calls(st: &St) -> Option<Vec<TestCall>> {
        let s = st.borrow();
        (!s.test_calls_overflow).then(|| s.test_calls.clone())
    }

    /// Whether the only code the page and its source table can run is what `pin` lists, and the page declares no property
    /// outside `pin.properties`:
    /// - the page's own triggers are exactly `pin.triggers`; no page extension extends it; nothing subscribes to an event of it;
    /// - every control is the repeater, a group or a field bound to a field of Rec (no part, no action, no variable or
    ///   expression source), declared by the page itself, with literal properties only; the control triggers are exactly
    ///   `pin.control_triggers`; no extension adds one;
    /// - the source table has no trigger, no procedure, no field trigger, no table extension and no subscriber.
    pub(super) fn page_code_is(&self, st: &St, pin: &PinnedCode) -> Result<bool, Signal> {
        const PLAIN: [&str; 5] = ["pagetype", "sourcetable", "applicationarea", "usagecategory", "caption"];
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let page = page_of(&engine, st);
        let same_names = |have: Vec<&str>, want: &[&str]| {
            let mut have = have;
            let mut want = want.to_vec();
            have.sort_unstable();
            want.sort_unstable();
            have == want
        };
        let page_code = same_names(page.triggers.iter().map(|(n, _)| n.as_str()).collect(), pin.triggers)
            && page.extensions.is_empty()
            && page.ext_triggers.is_empty()
            && !engine.events.any_subscriber_of(object);
        let properties = engine.program.objects[object].properties.iter().all(|(k, _)| {
            let k = k.to_ascii_lowercase();
            PLAIN.contains(&k.as_str()) || pin.properties.contains(&k.as_str())
        });
        let controls = page.controls.iter().all(|c| {
            let kind = match c.kind {
                ir::ControlKind::Repeater | ir::ControlKind::Group => c.binding == ir::Binding::None,
                ir::ControlKind::Field => matches!(c.binding, ir::Binding::Field(_)),
                _ => false,
            };
            kind && c.owner.is_none()
                && c.ext_triggers.is_empty()
                && c.props.iter().all(|(_, v)| matches!(v, ir::PropVal::Lit(_)))
        }) && same_names(
            page.controls
                .iter()
                .flat_map(|c| c.triggers.iter().map(|(n, _)| n.as_str()))
                .collect(),
            pin.control_triggers,
        );
        if !(page_code && properties && controls) {
            return Ok(false);
        }
        let table = self.table(&self.page_rec(st))?.object as usize;
        let t = &engine.program.objects[table];
        Ok(t.routines.is_empty()
            && t.table.as_ref().is_some_and(|x| x.extensions.is_empty())
            && !engine.events.any_subscriber_of(table))
    }

    /// Whether Rec is in the plain view of a pinned page: `key` (a position in the table's key list), ascending, no filter in
    /// any group, no marks.
    pub(super) fn rec_view_is(&self, st: &St, key: usize) -> bool {
        let rec = self.page_rec(st);
        let r = rec.borrow();
        r.key == key
            && r.adhoc.is_none()
            && r.ascending
            && r.field_descending == 0
            && r.filters.is_empty()
            && !r.marked_only
    }
}
