//! The journal shape of a hidden part (al-runner issue 7922; Base Application's journals show the Workflow Status FactBox so):
//! a Worksheet or List host the test opened with OpenView shows one unlinked ListPart with a repeater, hidden by a Visible
//! expression, whose rows the host's code refills through a procedure of the part page. Under the assumption
//! `hidden-part-journal-shape` (open corpus PR 1367, testpage/parts/HiddenPartJournalShape.al, group G2) the runner reads it as
//! that file measured, and only in the steps that file makes:
//! - a GoToRecord that stays on the host's row, as the test's first call after the open: the part reads nothing
//!   (`journal_part_stays`; every test's `gotorecord`);
//! - after that GoToRecord, an action's body that turns the part visible with one row: the part reads its rows before the host's
//!   refresh and again in it, as the pinned Document turn (`provider_hidden.rs`), and a temporary part a third time after the
//!   refresh on the majors that do (`journal_turn`, `journal_part_after_action_refresh`; :537, :563, :589, :615; the stored part
//!   :672 does not). After the turn the test may only read the part's Visible, call Next once and close ([`TURN_END`]);
//! - Next of the host that hides a part shown with one row, and Previous that shows a hidden temporary part with one row
//!   (`journal_part_after_host_move`; `next` of :537 to :642, :672; `previous` of :642).
//!
//! Every fact of the pages, the opener, the BC major, the test's calls and the part's state before and after each step is
//! required by equality (`journal_host_miss`, `journal_part_miss`); any other shape keeps the earlier path (a GoToRecord, a
//! move of a page outside the shape or between two hidden states) or is refused (a turn by an action, a step after it, a
//! move between other states of the part).

use super::pinned_shape::{NextEnd, PinnedCode, PinnedEnd, TestCall, code_stmts, codeunit_var_call};
use super::*;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::page_op::PageMethod;

/// The host's own triggers in the measured pages, lower case (HiddenPartJournalShape.al pages 75315, 75316, 75317).
const HOST_TRIGGERS: [&str; 3] = ["onopenpage", "onaftergetcurrrecord", "onclosepage"];
/// The part page's own triggers (pages 75313, 75314).
const PART_TRIGGERS: [&str; 4] = ["onopenpage", "onaftergetrecord", "onaftergetcurrrecord", "onclosepage"];
/// The properties the part control declares next to the Visible expression, all of them, each the literal the measured pages
/// give it (`None`: any literal; ApplicationArea runs nothing).
const PART_CONTROL_PROPS: [(&str, Option<&str>); 4] = [
    ("applicationarea", None),
    ("editable", Some("false")),
    ("enabled", Some("false")),
    ("showfilter", Some("false")),
];

/// What the host lets through after the turn (HiddenPartJournalShape.al:537 `send`, then `P.Status.Visible()`, `P.Next()`,
/// `P.Status.Visible()`, `P.Close()`).
const TURN_END: PinnedEnd = PinnedEnd {
    reason: "a call of the test after an action turned the hidden part of the journal shape visible, other than the part's \
             Visible, one Next and Close (not pinned)",
    reads: false,
    one_action: false,
    first_ends: false,
    part_visible: true,
    next: NextEnd::Once,
};

/// The source of the measured part page: temporary rows its procedure refills (page 75313, `SourceTableTemporary = true`), or
/// stored rows it filters (page 75314).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum JournalPart {
    Temporary,
    Stored,
}

/// The part of an open host of the journal shape (`Session::journal_tree`).
struct Journal {
    /// The part control's index on the host.
    i: usize,
    child: St,
    kind: JournalPart,
    /// `al_version::journal_temp_part_reads_after_refresh` on the run's major.
    temp_reads_again: bool,
}

/// What a journal host showed before a GoToRecord, Next or Previous (`Session::journal_before_move`).
pub(super) struct JournalBefore {
    /// The key of the row the host stood on.
    row: Vec<Value>,
    /// The part's Visible value and the rows of its view.
    part: (bool, i32),
}

/// Whether every control of `page` but `except` declares no property but ApplicationArea, as the controls of the measured pages
/// (HiddenPartJournalShape.al pages 75313 to 75317).
fn controls_declare_only_application_area(page: &ir::Page, except: Option<usize>) -> bool {
    page.controls
        .iter()
        .enumerate()
        .all(|(i, c)| Some(i) == except || c.props.iter().all(|(k, _)| k.eq_ignore_ascii_case("applicationarea")))
}

/// Whether the fields of `page` are the columns of its one repeater, and it has no group.
fn one_repeater_of_fields(page: &ir::Page) -> bool {
    let mut repeaters = page
        .controls
        .iter()
        .enumerate()
        .filter(|(_, c)| c.kind == ir::ControlKind::Repeater);
    let (Some((repeater, _)), None) = (repeaters.next(), repeaters.next()) else {
        return false;
    };
    page.controls.iter().all(|c| match c.kind {
        ir::ControlKind::Field => c.parent == Some(repeater),
        ir::ControlKind::Group => false,
        _ => true,
    })
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
    /// - a Worksheet (:537, :563, :672) or a List (:589, :615), on a BC major the probe measured
    ///   (`al_version::journal_temp_part_reads_after_refresh`);
    /// - its own triggers exactly [`HOST_TRIGGERS`], no property but the plain ones, no page extension, no subscriber; its source
    ///   table has no code, no table extension and no subscriber (`page_code_is`);
    /// - its controls: one repeater of fields of Rec, one part in the FactBoxes area, and actions in the Processing area with
    ///   OnAction as their only trigger; no property but ApplicationArea on any control but the part;
    /// - top-level, open, opened by the test itself with OpenView (`P.OpenView()` in every test), standing on a stored row.
    pub(super) fn journal_host_miss(&self, st: &St) -> Result<Option<&'static str>, Signal> {
        if !self.journal_host_type(st) {
            return Ok(Some("the host page is not a Worksheet or a List"));
        }
        if !self.has_source(st) {
            return Ok(Some("the host page has no source table"));
        }
        if al_version::journal_temp_part_reads_after_refresh(self.engine.target.as_ref()).is_none() {
            return Ok(Some("the probe of the journal shape did not measure this BC major"));
        }
        let engine = self.engine();
        let page = page_of(&engine, st);
        let kinds = |kind: ir::ControlKind| page.controls.iter().enumerate().filter(move |(_, c)| c.kind == kind);
        let on_action = vec!["onaction"; kinds(ir::ControlKind::Action).count()];
        let code = self.page_code_is(
            st,
            &PinnedCode {
                triggers: &HOST_TRIGGERS,
                control_triggers: &on_action,
                properties: &[],
                action_props: Some(&[]),
                part_props: Some(&["visible"]),
            },
        )?;
        let mut parts = kinds(ir::ControlKind::Part);
        let part = match (parts.next(), parts.next()) {
            (Some((i, c)), None) if c.factbox => Some(i),
            _ => None,
        };
        let layout = one_repeater_of_fields(page)
            && controls_declare_only_application_area(page, part)
            && page
                .controls
                .iter()
                .enumerate()
                .all(|(i, c)| c.factbox == (Some(i) == part))
            && page
                .action_areas
                .iter()
                .all(|a| a.name.eq_ignore_ascii_case("processing"));
        let opened = {
            let s = st.borrow();
            [
                (s.host.is_none(), "the host page is itself a part"),
                (s.opened_by_test, "the test did not open the host page itself"),
                (s.mode == PageMode::View, "the host page was not opened with OpenView"),
                (
                    s.open && s.cursor == Cursor::Row && !s.draft,
                    "the host page stands on no stored row",
                ),
            ]
            .into_iter()
            .find_map(|(holds, reason)| (!holds).then_some(reason))
        };
        Ok([
            (
                code,
                "the host page or its source table has other code, an extension, a subscriber or other properties than the measured one",
            ),
            (part.is_some(), "the host page does not show exactly one part, in its FactBoxes area"),
            (
                layout,
                "the host page has other controls than one repeater of fields of Rec, the part and actions in the Processing area, or a control property other than ApplicationArea",
            ),
        ]
        .into_iter()
        .find_map(|(holds, reason)| (!holds).then_some(reason))
        .or(opened))
    }

    /// Whether the test's calls on host `st` and its part (control `i`) since the open are exactly `calls` on the host, the
    /// running one last, with reads of the part's Visible between them and no write of the test between two calls
    /// (`PageState::tree_calls`).
    fn journal_calls_are(st: &St, i: usize, calls: &[TestCall]) -> bool {
        let s = st.borrow();
        let visible_read =
            |c: &pinned_shape::TreeCall| c.part == Some(i) && c.call == TestCall::Page(PageMethod::Visible);
        !s.tree_calls_unknown
            && s.tree_calls
                .iter()
                .filter(|c| !visible_read(c))
                .map(|c| (c.part.is_none() && !c.failed).then_some(c.call))
                .eq(calls.iter().map(|c| Some(*c)))
    }

    /// The first fact of the measured turn that host `st` does not have before an action's body, or `None`: the host is the
    /// measured one (`journal_host_miss`), and the test's calls since the open are the GoToRecord and this action's Invoke
    /// (HiddenPartJournalShape.al:537 `P.GoToRecord(Doc)`, `P.SendVal.Invoke()`).
    pub(super) fn journal_turn_host_miss(&self, st: &St) -> Result<Option<&'static str>, Signal> {
        if let Some(miss) = self.journal_host_miss(st)? {
            return Ok(Some(miss));
        }
        let calls = [
            TestCall::Page(PageMethod::GoToRecord),
            TestCall::Other(PageMethod::Invoke),
        ];
        let measured = Self::open_parts_with_hidden(st)
            .first()
            .is_some_and(|(i, _)| Self::journal_calls_are(st, *i, &calls));
        Ok((!measured).then_some("the test's calls since the open are not one GoToRecord, then this action"))
    }

    /// The measured part of control `i` of host `st` (page `child`), or the first fact it does not have:
    /// - the control has no SubPageLink and no Provider, a Visible expression that is a Boolean variable of the host page, and
    ///   next to it exactly the literal properties of [`PART_CONTROL_PROPS`];
    /// - the page is a ListPart, `Editable = false`, no other property but `SourceTableTemporary = true` (page 75313) or none
    ///   (page 75314);
    /// - its own triggers are exactly [`PART_TRIGGERS`], each one call of a procedure of a codeunit variable with value
    ///   arguments (`Log.Add(..)`, :75-93); no page extension, no subscriber; its controls are one repeater of fields of Rec
    ///   without a trigger and with no property but ApplicationArea; its source table has no code, no table extension and no
    ///   subscriber (`page_code_is`).
    pub(super) fn journal_part_miss(&self, st: &St, i: usize, child: &St) -> Result<JournalPart, &'static str> {
        let engine = self.engine();
        let host = st.borrow().object as usize;
        let control = &page_of(&engine, st).controls[i];
        let unlinked = control
            .part_link
            .as_ref()
            .is_none_or(|l| l.entries.is_empty() && l.provider.is_none());
        let literal = |name: &str, want: Option<&str>| {
            control.props.iter().any(|(k, v)| {
                k.eq_ignore_ascii_case(name)
                    && matches!(v, ir::PropVal::Lit(lit) if want.is_none_or(|w| lit.trim().eq_ignore_ascii_case(w)))
            })
        };
        // `Visible = ShowStatus`: the routine only returns a Boolean variable of the page.
        let visible_variable = control.props.iter().any(|(k, v)| {
            k.eq_ignore_ascii_case("visible")
                && matches!(v, ir::PropVal::Routine(r)
                    if matches!(code_stmts(&engine.program, host, *r).as_slice(),
                        [ir::Stmt::Exit(Some(ir::Expr::Load(ir::Place::Global(g))))]
                            if engine.program.objects[host].globals.get(*g).is_some_and(|s| s.ty == RtType::Boolean)))
        });
        let control_props = control.props.len() == PART_CONTROL_PROPS.len() + 1
            && PART_CONTROL_PROPS.iter().all(|(name, want)| literal(name, *want))
            && visible_variable;
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
        let code = self
            .page_code_is(
                child,
                &PinnedCode {
                    triggers: &PART_TRIGGERS,
                    control_triggers: &[],
                    properties: &["editable", "sourcetabletemporary"],
                    action_props: None,
                    part_props: None,
                },
            )
            .is_ok_and(|is| is);
        let bodies = page.triggers.iter().all(|(_, r)| {
            matches!(code_stmts(&engine.program, object, *r).as_slice(), [ir::Stmt::Expr(e)]
                if codeunit_var_call(&engine.program, object, e)
                    && matches!(e, ir::Expr::Call { args, .. } if args.iter().all(|a| matches!(a, ir::Arg::Value(_)))))
        });
        let controls = one_repeater_of_fields(page) && controls_declare_only_application_area(page, None);
        [
            (unlinked, "the part has a SubPageLink or a Provider"),
            (
                control_props,
                "the part control declares other properties than the measured ones and a Visible that is a variable of the host page",
            ),
            (list_part, "the part is not a ListPart"),
            (kind.is_some(), "the part's SourceTableTemporary is not true"),
            (props, "the part page declares other properties than Editable = false"),
            (
                code,
                "the part page or its source table has other code, an extension, a subscriber or other controls than the measured one",
            ),
            (
                bodies,
                "a trigger of the part page does more than call one procedure of a codeunit variable",
            ),
            (
                controls,
                "the part page has other controls than one repeater of fields of Rec with no property but ApplicationArea",
            ),
        ]
        .into_iter()
        .find_map(|(holds, reason)| (!holds).then_some(reason))
        .map_or(Ok(kind.unwrap_or(JournalPart::Stored)), Err)
    }

    /// The one part of host `st` when host and part have every fact of the journal shape (`journal_host_miss`,
    /// `journal_part_miss`) and the part is open, else `None`. The facts a page of another type or with another count of parts
    /// fails are checked first: this runs before every GoToRecord, Next and Previous of a top-level page.
    fn journal_tree(&mut self, st: &St) -> Result<Option<Journal>, Signal> {
        let one_part = {
            let s = st.borrow();
            s.open && s.host.is_none() && s.parts.len() + s.hidden_parts.len() == 1
        };
        if !one_part || self.journal_host_miss(st)?.is_some() {
            return Ok(None);
        }
        let Some(temp_reads_again) = al_version::journal_temp_part_reads_after_refresh(self.engine.target.as_ref())
        else {
            return Ok(None);
        };
        let Some((i, child)) = Self::open_parts_with_hidden(st).into_iter().next() else {
            return Ok(None);
        };
        Ok(self.journal_part_miss(st, i, &child).ok().map(|kind| Journal {
            i,
            child,
            kind,
            temp_reads_again,
        }))
    }

    /// The journal part's Visible value and the rows of its view.
    fn journal_part_state(&mut self, st: &St, j: &Journal) -> Result<(bool, i32), Signal> {
        Ok((self.control_visible(st, j.i)?, self.view_row_count(&j.child)?))
    }

    /// The journal part reads its rows (`sync_part`). Its triggers write nothing in the measured pages (:75-93 log only): a
    /// read that writes to the database or to the part's own table is refused.
    fn journal_read(&mut self, st: &St, i: usize, child: &St) -> Result<(), Signal> {
        let rec = self.page_rec(child);
        let before = (self.store.change_counter(), self.store_for(&rec).change_counter());
        self.sync_part(st, i, child)?;
        if (self.store.change_counter(), self.store_for(&rec).change_counter()) != before {
            return unsupported("a read of the hidden part of the journal shape that writes (not pinned)");
        }
        Ok(())
    }

    /// The action's body on host `st` turned the journal part `child` (control `i`) visible, and `first_turn_as_pinned` found
    /// host and part as measured. `commits`: the session's commits before the body. The turn is the measured one only when the
    /// body did not commit (`AddRequest` inserts one request, :434-447) and the part shows one row (`send` `st-fill(H1,1)`,
    /// :537): then the part reads its row before the host's refresh, and the host lets through only [`TURN_END`].
    pub(super) fn journal_turn(&mut self, st: &St, i: usize, child: &St, commits: u64) -> Result<(), Signal> {
        if self.commits != commits {
            return unsupported(
                "an action that commits and turns the hidden part of the journal shape visible (not pinned)",
            );
        }
        if self.view_row_count(child)? != 1 {
            return unsupported(
                "an action that turns the hidden part of the journal shape visible with other than one row (not pinned)",
            );
        }
        assume(Assumption::HiddenPartJournalShape);
        st.borrow_mut().pinned_end = Some(TURN_END);
        self.journal_read(st, i, child)?;
        child.borrow_mut().journal_turned = Some(self.store.change_counter());
        Ok(())
    }

    /// After the refresh that followed an action (`r`, its result) whose body turned the journal part visible
    /// (`PageState::journal_turned`, taken here after every action): the refresh must have left the page in the shape, the part
    /// visible with its one row, and have written nothing to the database (the measured triggers only log and refill the
    /// part); else the action is refused. A temporary part then reads its rows once more on the majors that do
    /// (`al_version::journal_temp_part_reads_after_refresh`; HiddenPartJournalShape.al:537 `send` ends `st-agr(1) st-agr(1)
    /// st-agcr(1)` three times on BC 29, twice before); the stored part does not (:672).
    pub(super) fn journal_part_after_action_refresh(
        &mut self,
        st: &St,
        r: Result<Value, Signal>,
    ) -> Result<Value, Signal> {
        let turned = {
            let s = st.borrow();
            let mut parts = s.parts.iter().chain(&s.hidden_parts);
            parts.find_map(|(_, c)| c.borrow_mut().journal_turned.take())
        };
        let v = r?;
        let Some(writes) = turned else {
            return Ok(v);
        };
        let left = "the page left the journal shape of its hidden part during an action (not pinned)";
        let Some(j) = self.journal_tree(st)? else {
            return unsupported(left);
        };
        if self.journal_part_state(st, &j)? != (true, 1) || self.store.change_counter() != writes {
            return unsupported(left);
        }
        if j.kind == JournalPart::Temporary && j.temp_reads_again {
            self.journal_read(st, j.i, &j.child)?;
        }
        Ok(v)
    }

    /// What the journal host `st` shows before a GoToRecord, Next or Previous, else `None`.
    pub(super) fn journal_before_move(&mut self, st: &St, name: PageMethod) -> Result<Option<JournalBefore>, Signal> {
        if !matches!(name, PageMethod::GoToRecord | PageMethod::Next | PageMethod::Previous) {
            return Ok(None);
        }
        let Some(j) = self.journal_tree(st)? else {
            return Ok(None);
        };
        Ok(Some(JournalBefore {
            row: self.row_key(&self.page_rec(st))?,
            part: self.journal_part_state(st, &j)?,
        }))
    }

    /// A GoToRecord of the journal host that stayed on its row, as the test's first call since the open, with the part in a
    /// measured state (`before`): the part reads nothing (HiddenPartJournalShape.al:537 and every other test of the file,
    /// `gotorecord` `[]`: the part hidden with no row, temporary or, :672, stored; :642 a temporary part shown with its row).
    /// `true` when it took the step; any other GoToRecord keeps the earlier path.
    pub(super) fn journal_part_stays(
        &mut self,
        st: &St,
        name: PageMethod,
        before: Option<&JournalBefore>,
    ) -> Result<bool, Signal> {
        let Some(before) = before.filter(|_| name == PageMethod::GoToRecord) else {
            return Ok(false);
        };
        if !Self::same_key(&self.row_key(&self.page_rec(st))?, &before.row) {
            return Ok(false);
        }
        let Some(j) = self.journal_tree(st)? else {
            return Ok(false);
        };
        let measured = matches!(
            (before.part, j.kind),
            ((false, 0), _) | ((true, 1), JournalPart::Temporary)
        ) && Self::journal_calls_are(st, j.i, &[TestCall::Page(PageMethod::GoToRecord)]);
        if measured {
            assume(Assumption::HiddenPartJournalShape);
        }
        Ok(measured)
    }

    /// After Next or Previous moved the journal host and it refreshed: the part reads where the file measured it, by the step,
    /// the part's kind and its state before the move (`before`) and after the host's refresh (its Visible value and its rows):
    /// - hidden with no row before and after: the earlier path, which reads nothing (nothing measured, nothing marked);
    /// - Next, shown with one row to hidden with no row, temporary (`next` of :537, :563, :589, :615 after the turn, :642 shown
    ///   from the open): nothing, or on the majors that do, OnAfterGetCurrRecord on the blank row (`st-agcr(0)`);
    /// - the same Next of a stored part: nothing, which is the earlier path (:672 after the turn; marked by the turn);
    /// - Previous, hidden with no row to shown with one row, temporary (:642 `previous` `st-agr(1) st-agr(1) st-agcr(1)`): it
    ///   reads its row.
    ///
    /// Any other pair of states is refused, and so is every state but the first Next's after a turn (`TURN_END`).
    pub(super) fn journal_part_after_host_move(
        &mut self,
        st: &St,
        name: PageMethod,
        before: Option<&JournalBefore>,
    ) -> Result<(), Signal> {
        let after_turn = Self::journal_next_taken(st);
        let unmeasured =
            "a host move with the hidden part of the journal shape in a state its probe did not measure (not pinned)";
        let (Some(before), Some(j)) = (before, self.journal_tree(st)?) else {
            return if after_turn { unsupported(unmeasured) } else { Ok(()) };
        };
        let after = self.journal_part_state(st, &j)?;
        match (name, before.part, after, j.kind) {
            (_, (false, 0), (false, 0), _) if !after_turn => {}
            (PageMethod::Next, (true, 1), (false, 0), JournalPart::Temporary) => {
                assume(Assumption::HiddenPartJournalShape);
                if j.temp_reads_again {
                    self.journal_read(st, j.i, &j.child)?;
                }
            }
            (PageMethod::Next, (true, 1), (false, 0), JournalPart::Stored) => {}
            (PageMethod::Previous, (false, 0), (true, 1), JournalPart::Temporary) => {
                assume(Assumption::HiddenPartJournalShape);
                self.journal_read(st, j.i, &j.child)?;
            }
            _ => return unsupported(unmeasured),
        }
        Ok(())
    }

    /// Whether host `st` just let through its one Next after a turn ([`TURN_END`]).
    fn journal_next_taken(st: &St) -> bool {
        let taken = PinnedEnd {
            next: NextEnd::Taken,
            ..TURN_END
        };
        st.borrow().pinned_end == Some(taken)
    }

    /// The one Next the host lets through after a turn ([`TURN_END`]) answered No (`r`): the measured one moves (:537 `next`
    /// `host-agcr(H2)`), so a Next that stays is refused.
    pub(super) fn journal_next_after_turn(
        st: &St,
        name: PageMethod,
        r: Result<Value, Signal>,
    ) -> Result<Value, Signal> {
        match r {
            Ok(Value::Boolean(false)) if name == PageMethod::Next && Self::journal_next_taken(st) => {
                unsupported(TURN_END.reason)
            }
            r => r,
        }
    }
}
