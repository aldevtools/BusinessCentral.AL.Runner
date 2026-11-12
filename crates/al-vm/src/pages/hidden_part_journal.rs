//! The journal shape of a hidden part (al-runner issue 7922; Base Application's journals show the Workflow Status FactBox so):
//! a Worksheet or List host the test opened with OpenView shows one unlinked ListPart with a repeater, hidden by a Visible
//! expression, whose rows the host's code refills through a procedure of the part page. Under the assumption
//! `hidden-part-journal-shape` (open corpus PR 1367, testpage/parts/HiddenPartJournalShape.al, group G2) the runner reads it as
//! that file measured:
//! - an action's body that turns it visible: the part reads its rows before the host's refresh and again in it, as the pinned
//!   Document turn (`provider_hidden.rs`), and a temporary part a third time after the refresh on the majors that do
//!   (`journal_part_after_action_refresh`; :537, :563, :589, :615; the stored part :672 does not);
//! - a GoToRecord that stays on the host's row: the part reads nothing (`journal_part_stays`; every test's `gotorecord`);
//! - Next and Previous of the host: the part reads its one row when the host's refresh left it visible, and a part the move
//!   emptied and hid runs OnAfterGetCurrRecord on its blank row only when it is temporary and on the majors that do
//!   (`journal_part_after_host_move`; `next` of :537 to :642, :672; `previous` of :642).
//!
//! Every fact of the pages, the opener and the part's state at each step is required by equality (`journal_host_miss`,
//! `journal_part_miss`); any other shape keeps the earlier path (a move) or is refused (a turn by an action).

use super::*;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::page_op::PageMethod;

/// The host's own triggers in the measured pages, lower case (HiddenPartJournalShape.al pages 75315, 75316, 75317).
const HOST_TRIGGERS: [&str; 3] = ["onopenpage", "onaftergetcurrrecord", "onclosepage"];
/// The part page's own triggers (pages 75313, 75314).
const PART_TRIGGERS: [&str; 4] = ["onopenpage", "onaftergetrecord", "onaftergetcurrrecord", "onclosepage"];
/// The properties the part control declares next to the Visible expression, each the literal the measured pages give it
/// (`None`: any literal; ApplicationArea runs nothing).
const PART_CONTROL_PROPS: [(&str, Option<&str>); 4] = [
    ("applicationarea", None),
    ("editable", Some("false")),
    ("enabled", Some("false")),
    ("showfilter", Some("false")),
];

/// The source of the measured part page: temporary rows its procedure refills (page 75313, `SourceTableTemporary = true`), or
/// stored rows it filters (page 75314).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum JournalPart {
    Temporary,
    Stored,
}

impl Session {
    /// Whether page `st` is a Worksheet (:537, :563, :672) or a List (:589, :615), the host types of the journal shape.
    pub(super) fn journal_host_type(&self, st: &St) -> bool {
        matches!(
            self.engine.object_facts[st.borrow().object as usize].page_type,
            Some(al_ir::PageType::Worksheet | al_ir::PageType::List)
        )
    }

    /// The first fact of the measured host that page `st` does not have, or `None` when it has them all:
    /// - a Worksheet (:537, :563, :672) or a List (:589, :615);
    /// - top-level, opened by the test itself with OpenView (`P.OpenView()` in every test), standing on a stored row;
    /// - its own triggers exactly OnOpenPage, OnAfterGetCurrRecord and OnClosePage, no property but the plain ones, no page
    ///   extension, no subscriber;
    /// - one part control, and no other control with a trigger but actions with OnAction only.
    pub(super) fn journal_host_miss(&self, st: &St) -> Option<&'static str> {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let page = page_of(&engine, st);
        let host_type = self.journal_host_type(st);
        let opened = {
            let s = st.borrow();
            [
                (s.host.is_none(), "the host page is itself a part"),
                (s.opened_by_test, "the test did not open the host page itself"),
                (s.mode == PageMode::View, "the host page was not opened with OpenView"),
                (
                    s.cursor == Cursor::Row && !s.draft,
                    "the host page stands on no stored row",
                ),
            ]
            .into_iter()
            .find_map(|(holds, reason)| (!holds).then_some(reason))
        };
        let mut triggers: Vec<&str> = page.triggers.iter().map(|(n, _)| n.as_str()).collect();
        triggers.sort_unstable();
        let mut want = HOST_TRIGGERS.to_vec();
        want.sort_unstable();
        let code = triggers == want
            && page.extensions.is_empty()
            && page.ext_triggers.is_empty()
            && !engine.events.any_subscriber_of(object)
            && engine.program.objects[object]
                .properties
                .iter()
                .all(|(k, _)| pinned_shape::PLAIN_PROPERTIES.contains(&k.to_ascii_lowercase().as_str()));
        let parts = page.controls.iter().filter(|c| c.kind == ir::ControlKind::Part).count();
        let controls = page.controls.iter().all(|c| {
            c.owner.is_none()
                && c.ext_triggers.is_empty()
                && match c.kind {
                    ir::ControlKind::Action => c.triggers.iter().all(|(n, _)| n == "onaction"),
                    _ => c.triggers.is_empty(),
                }
        });
        [
            (host_type, "the host page is not a Worksheet or a List"),
            (
                code,
                "the host page declares other triggers, properties, an extension or a subscriber than the measured one",
            ),
            (parts == 1, "the host page shows more than one part"),
            (
                controls,
                "a control of the host page has a trigger other than an action's OnAction",
            ),
        ]
        .into_iter()
        .find_map(|(holds, reason)| (!holds).then_some(reason))
        .or(opened)
    }

    /// The measured part of control `i` of host `st` (page `child`), or the first fact it does not have:
    /// - the control has no SubPageLink and no Provider, a Visible expression and only the literal properties of
    ///   [`PART_CONTROL_PROPS`];
    /// - the page is a ListPart with a repeater of fields of Rec without a trigger, `Editable = false`, its own triggers
    ///   exactly [`PART_TRIGGERS`], no other property but `SourceTableTemporary = true` (page 75313) or none (page 75314), no
    ///   page extension, no subscriber; its source table has no code, no table extension and no subscriber.
    pub(super) fn journal_part_miss(&self, st: &St, i: usize, child: &St) -> Result<JournalPart, &'static str> {
        let engine = self.engine();
        let control = &page_of(&engine, st).controls[i];
        let unlinked = control
            .part_link
            .as_ref()
            .is_none_or(|l| l.entries.is_empty() && l.provider.is_none());
        let control_props = control.props.iter().all(|(k, v)| match v {
            ir::PropVal::Lit(lit) => PART_CONTROL_PROPS.iter().any(|(name, want)| {
                k.eq_ignore_ascii_case(name) && want.is_none_or(|w| lit.trim().eq_ignore_ascii_case(w))
            }),
            _ => k.eq_ignore_ascii_case("visible"),
        }) && control
            .props
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("visible") && !matches!(v, ir::PropVal::Lit(_)));
        let object = child.borrow().object as usize;
        let page = page_of(&engine, child);
        let list_part =
            matches!(&engine.object_facts[object].page_type, Some(al_ir::PageType::Other(t)) if t == "listpart");
        let properties = &engine.program.objects[object].properties;
        let prop = |name: &str| {
            properties
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(name))
                .map(|(_, v)| v.trim())
        };
        let kind = match prop("sourcetabletemporary") {
            Some(v) if v.eq_ignore_ascii_case("true") => Some(JournalPart::Temporary),
            None => Some(JournalPart::Stored),
            Some(_) => None,
        };
        let props = prop("editable").is_some_and(|v| v.eq_ignore_ascii_case("false"))
            && properties.iter().all(|(k, _)| {
                let k = k.to_ascii_lowercase();
                pinned_shape::PLAIN_PROPERTIES.contains(&k.as_str()) || k == "editable" || k == "sourcetabletemporary"
            });
        let mut triggers: Vec<&str> = page.triggers.iter().map(|(n, _)| n.as_str()).collect();
        triggers.sort_unstable();
        let mut want = PART_TRIGGERS.to_vec();
        want.sort_unstable();
        let code = triggers == want
            && page.extensions.is_empty()
            && page.ext_triggers.is_empty()
            && !engine.events.any_subscriber_of(object);
        let controls = page.controls.iter().all(|c| {
            let kind = match c.kind {
                ir::ControlKind::Repeater => c.binding == ir::Binding::None,
                ir::ControlKind::Field => matches!(c.binding, ir::Binding::Field(_)),
                _ => false,
            };
            kind && c.owner.is_none() && c.ext_triggers.is_empty() && c.triggers.is_empty()
        }) && self.has_repeater(child);
        let table = self.page_rec(child).borrow().table as usize;
        let t = &engine.program.objects[table];
        let table_code = t.routines.is_empty()
            && t.table.as_ref().is_some_and(|x| x.extensions.is_empty())
            && !engine.events.any_subscriber_of(table);
        [
            (unlinked, "the part has a SubPageLink or a Provider"),
            (
                control_props,
                "the part control declares other properties than the measured Visible expression",
            ),
            (list_part, "the part is not a ListPart"),
            (kind.is_some(), "the part's SourceTableTemporary is not true"),
            (props, "the part page declares other properties than Editable = false"),
            (
                code,
                "the part page declares other triggers, an extension or a subscriber than the measured one",
            ),
            (
                controls,
                "the part page has other controls than a repeater of fields of Rec without a trigger",
            ),
            (
                table_code,
                "the part's source table has code, an extension or a subscriber",
            ),
        ]
        .into_iter()
        .find_map(|(holds, reason)| (!holds).then_some(reason))
        .map_or(Ok(kind.unwrap_or(JournalPart::Stored)), Err)
    }

    /// The one part of host `st` and its kind when host and part have every fact of the journal shape (`journal_host_miss`,
    /// `journal_part_miss`) and the part is open, else `None`.
    fn journal_tree(&mut self, st: &St) -> Option<(usize, St, JournalPart)> {
        if self.journal_host_miss(st).is_some() {
            return None;
        }
        let (i, child) = Self::open_parts_with_hidden(st).into_iter().next()?;
        let kind = self.journal_part_miss(st, i, &child).ok()?;
        Some((i, child, kind))
    }

    /// After the refresh that followed an action (`r`, its result) whose body turned the journal part visible
    /// (`PageState::journal_turned`, cleared here after every action): a temporary part reads its rows once more on the majors
    /// that do (`al_version::journal_temp_part_reads_after_refresh`; HiddenPartJournalShape.al:537 `send` ends `st-agr(1)
    /// st-agr(1) st-agcr(1)` three times on BC 29, twice before); the stored part does not (:672). A major the probe did not
    /// measure is refused.
    pub(super) fn journal_part_after_action_refresh(
        &mut self,
        st: &St,
        r: Result<Value, Signal>,
    ) -> Result<Value, Signal> {
        let turned: Vec<(usize, St)> = Self::open_parts_with_hidden(st)
            .into_iter()
            .filter(|(_, c)| std::mem::take(&mut c.borrow_mut().journal_turned))
            .collect();
        let v = r?;
        for (i, child) in turned {
            let Some((_, _, kind)) = self.journal_tree(st) else {
                return unsupported("the page left the journal shape of its hidden part during an action (not pinned)");
            };
            if kind == JournalPart::Temporary && self.journal_temp_reads_again()? {
                self.sync_part(st, i, &child)?;
            }
        }
        Ok(v)
    }

    /// Whether the temporary journal part reads again on this major; a major the probe did not measure is refused.
    fn journal_temp_reads_again(&self) -> Result<bool, Signal> {
        match al_version::journal_temp_part_reads_after_refresh(self.engine.target.as_ref()) {
            Some(again) => Ok(again),
            None => unsupported("a temporary hidden part of the journal shape on a BC major its probe did not measure"),
        }
    }

    /// The row the journal host stands on before a GoToRecord (`journal_part_stays` compares it), else `None`.
    pub(super) fn journal_row_before(&mut self, st: &St, name: PageMethod) -> Result<Option<Vec<Value>>, Signal> {
        if name != PageMethod::GoToRecord || self.journal_tree(st).is_none() {
            return Ok(None);
        }
        Ok(Some(self.row_key(&self.page_rec(st))?))
    }

    /// A GoToRecord of the journal host that stayed on its row (`before`, from `journal_row_before`): the part reads nothing
    /// (HiddenPartJournalShape.al:537 and every other test of the file, `gotorecord` `[]`, the part hidden or, :642, shown with
    /// its row). `true` when it took the step.
    pub(super) fn journal_part_stays(&mut self, st: &St, before: Option<&[Value]>) -> Result<bool, Signal> {
        let Some(before) = before else {
            return Ok(false);
        };
        if !Self::same_key(&self.row_key(&self.page_rec(st))?, before) || self.journal_tree(st).is_none() {
            return Ok(false);
        }
        assume(Assumption::HiddenPartJournalShape);
        Ok(true)
    }

    /// After Next or Previous of the journal host and its refresh: the part reads where the file measured it, by the step and
    /// the state the host's refresh left it in (its Visible value and its rows):
    /// - Next, hidden with no row (:537, :563, :589, :615, :642 `next`, temporary; :672 stored): nothing, or on the majors that
    ///   do, OnAfterGetCurrRecord on the blank row of a temporary part (`st-agcr(0)`);
    /// - Previous, shown with one row (:642 `previous` `st-agr(1) st-agr(1) st-agcr(1)`, temporary): it reads its row.
    ///
    /// `false` (the earlier path) on a page outside the shape; any other state of the part is refused.
    pub(super) fn journal_part_after_host_move(&mut self, st: &St, name: PageMethod) -> Result<bool, Signal> {
        let Some((i, child, kind)) = self.journal_tree(st) else {
            return Ok(false);
        };
        let visible = self.control_visible(st, i)?;
        let rows = self.view_row_count(&child)?;
        match (name, visible, rows, kind) {
            (PageMethod::Next, false, 0, JournalPart::Temporary) => {
                assume(Assumption::HiddenPartJournalShape);
                if self.journal_temp_reads_again()? {
                    self.sync_part(st, i, &child)?;
                }
            }
            (PageMethod::Next, false, 0, JournalPart::Stored) => assume(Assumption::HiddenPartJournalShape),
            (PageMethod::Previous, true, 1, JournalPart::Temporary) => {
                assume(Assumption::HiddenPartJournalShape);
                self.sync_part(st, i, &child)?;
            }
            _ => {
                return unsupported(
                    "a host move with the hidden part of the journal shape in a state its probe did not measure (not pinned)",
                );
            }
        }
        Ok(true)
    }
}
