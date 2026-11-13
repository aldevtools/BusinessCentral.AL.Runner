//! The List-host cells of the empty ListPart Rec (`empty_part_rec.rs`): a ListPart that shows no row, on a top-level List page,
//! holds the single values of its SubPageLink on Rec, as on a Card or Document host. Al-runner issue 6924, comment 6822249976
//! (the reporter's host is a List page with Base Application page 1178 in its FactBoxes). It runs through the assumption
//! `empty-part-rec-list-host` until corpus pull request 1339 (pages/parts/EmptyPartRecLinkList.al) is pinned.
//!
//! `list_host_cell` is the allowlist: every fact the probe holds fixed is required by equality, each against the probe object
//! that holds it. Two kinds of cell: the probe's first hosts, whose facts the probe varies one by one (`HOST_PROPS`,
//! `PART_CONTROL_PROPS` and the landings of `list_landing_measured`), and the forms of `FORM_CELLS` (page 76854 and the
//! reporter's host form, pages 76855-76859), each measured only in its own open forms and landings (`form_landing`) and with
//! the facts all its cells share (`form_cell_facts`). A shape with any other fact gets the part's filtered fields unknown, and
//! their read is refused with the first such fact (`empty_part_rec.rs`; `suite/empty_part_rec_list.rs`
//! `empty_list_part_rec_list_host_unmeasured_shapes_refuse`, `empty_list_part_rec_list_host_neighbors_refuse`,
//! `empty_list_part_rec_widened_cells_neighbors_refuse`, `empty_list_part_rec_tied_to_its_landing`). Rec holds the values of a
//! cell only while a read the cell measured runs (`Session::reading_empty_list_rec`). The facts no probe object varies are named
//! in the entry's text.

use super::pinned_shape::{PartStep, TestCall};
use super::*;
use al_builtins::page_op::PageMethod;
use al_builtins::record_op::RecordOp;
use al_data::TableDef;

/// Page properties of the probe's List hosts (EmptyPartRecLinkList.al page 76843 "PG EPL List ConstField" and its siblings, and
/// page 76855 "PG EPL List Rep", which has a Caption), by lower-case name with the value they hold (`None`: any value; the
/// property names an object, a usage or a label, not a behavior).
const HOST_PROPS: [(&str, Option<&str>); 5] = [
    ("pagetype", Some("list")),
    ("sourcetable", None),
    ("applicationarea", None),
    ("usagecategory", None),
    ("caption", None),
];

/// Page properties of the probe's parts: page 76842 "PG EPL Part" (the first four and UsageCategory), page 76851 "PG EPL Part
/// Doc" (also DeleteAllowed and DelayedInsert, the properties of page 1178), and a Caption (see `HOST_PROPS`).
const PART_PROPS: [(&str, Option<&str>); 8] = [
    ("pagetype", Some("listpart")),
    ("sourcetable", None),
    ("insertallowed", Some("false")),
    ("applicationarea", None),
    ("usagecategory", None),
    ("deleteallowed", Some("true")),
    ("delayedinsert", Some("true")),
    ("caption", None),
];

/// The one page property of a `FORM_CELLS` host besides `HOST_PROPS`, if any (`FormCell::host_extra`).
type HostExtra = Option<(&'static str, Option<&'static str>)>;

/// The part properties every probe part sets: a ListPart without the new-row line (page 76842, as page 1178).
const PART_REQUIRED: [(&str, &str); 2] = [("pagetype", "listpart"), ("insertallowed", "false")];

/// The part properties of page 76851 "PG EPL Part Doc" besides `PART_REQUIRED` (page 1178's), which every `FORM_CELLS` part has.
const PART_DOC_REQUIRED: [(&str, &str); 2] = [("deleteallowed", "true"), ("delayedinsert", "true")];

/// The page triggers of a probe part, sorted: page 76842 (OnAfterGetCurrRecord), page 76851 (page 1178's set).
const PART_TRIGGERS: [&[&str]; 2] = [
    &["onaftergetcurrrecord"],
    &["onaftergetcurrrecord", "ondeleterecord", "oninit"],
];

/// The control triggers of a probe part, sorted: page 76842 (the action that reads Rec), page 76851 (and a field's OnDrillDown).
const PART_CONTROL_TRIGGERS: [&[&str]; 2] = [&["onaction"], &["onaction", "ondrilldown"]];

/// The page triggers of a probe host, sorted: page 76843 (none), page 76853 "PG EPL List Triggers".
const HOST_TRIGGERS: [&[&str]; 2] = [&[], &["onaftergetcurrrecord", "onaftergetrecord", "onopenpage"]];

/// Properties of the part control on the probe's first hosts (page 76843 `part(Box; ..)`).
const PART_CONTROL_PROPS: [(&str, Option<&str>); 2] = [("applicationarea", None), ("subpagelink", None)];

/// Where the part read again, in the forms of `FORM_CELLS`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Landing {
    /// As the host opened, on its first row.
    Open,
    /// After a GoToRecord that moved the host to the last of its rows.
    GoToLastRow,
    /// After a GoToRecord to the row the host stands on, its only row.
    GoToOnlyRow,
    /// After a Next that moved the host to the last of its rows.
    NextToLastRow,
}

/// The rows of the host's view before and after the row it stands on, each counted up to 3 (`Session::host_rows`).
type Rows = (i32, i32);

/// A landing a host with the triggers of probe 1378 measured: the open form, the landing and the rows around the host's row.
type TriggerLanding = (PageMode, Landing, Rows);

/// The landings of the first List host shape with the triggers of probe 1378 (EmptyPartRecListHostTriggers.al pages 76865-76867,
/// heads A and B): as opened on A (:593 OpenEdit, :613 OpenView), after GoToRecord to B (:633, :655) and after Next (:677, :698),
/// and the same cells with the other trigger sets, CurrPage.Editable and notifications (:720-:1639).
const FIRST_FORM_TRIGGER_LANDINGS: &[TriggerLanding] = &[
    (PageMode::Edit, Landing::Open, (0, 1)),
    (PageMode::View, Landing::Open, (0, 1)),
    (PageMode::Edit, Landing::GoToLastRow, (1, 0)),
    (PageMode::View, Landing::GoToLastRow, (1, 0)),
    (PageMode::Edit, Landing::NextToLastRow, (1, 0)),
    (PageMode::View, Landing::NextToLastRow, (1, 0)),
];

/// The landings of the reporter's form with the triggers of probe 1378 (EmptyPartRecListHostTriggers.al pages 76868-76870, heads
/// RA, RB and RC): as opened on RA and after GoToRecord to RC, opened for edit and for view (:1659-:2363).
const REPORTER_TRIGGER_LANDINGS: &[TriggerLanding] = &[
    (PageMode::Edit, Landing::Open, (0, 2)),
    (PageMode::View, Landing::Open, (0, 2)),
    (PageMode::Edit, Landing::GoToLastRow, (2, 0)),
    (PageMode::View, Landing::GoToLastRow, (2, 0)),
];

/// A List-host cell part `st` landed in (`Session::list_host_cell`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ListCell {
    /// The number of reads by an action of the part the cell measured after its landing (`Session::measured_read`).
    pub(super) reads: usize,
    /// The host has the page triggers of probe 1378 (assumption `empty-part-rec-list-host-triggers`).
    pub(super) triggers: bool,
}

/// A host form the probe measured in its own open forms and landings: the part control's properties, exactly; the one page
/// property the host has besides `HOST_PROPS`, if any; and each open form with the landing measured. Every form's part is page
/// 76851 "PG EPL Part Doc" (page 1178's shape) in area(FactBoxes), and its link a `const(..)` on an Integer field then a field
/// link (`form_cell_facts`). The host has no trigger, or the triggers of probe 1378 in `trigger_landings` only.
struct FormCell {
    control: &'static [(&'static str, Option<&'static str>)],
    host_extra: Option<(&'static str, Option<&'static str>)>,
    landings: &'static [(PageMode, Landing)],
    trigger_landings: &'static [TriggerLanding],
}

/// The part control of page 76855 "PG EPL List Rep" (the reporter's form), of pages 76857 and 76859.
const REPORTER_CONTROL: &[(&str, Option<&str>)] = &[
    ("applicationarea", None),
    ("caption", None),
    ("subpagelink", None),
    ("updatepropagation", Some("both")),
];

/// GoToRecord to the last of three rows, opened for edit and for view.
const LAST_ROW_EDIT_VIEW: &[(PageMode, Landing)] = &[
    (PageMode::Edit, Landing::GoToLastRow),
    (PageMode::View, Landing::GoToLastRow),
];

/// The forms of EmptyPartRecLinkList.al (corpus pull request 1339, commit 831ae544), each cell `act=76840/<link values>/0/`:
/// observed on BC 27.5, 28.5 and 29.0 alike.
const FORM_CELLS: [FormCell; 5] = [
    // Page 76854 "PG EPL List DocBoth": OpenEdit as opened (:1800), GoToRecord to the last of two rows (:1818), GoToRecord to the
    // only row opened for edit (:1836) and for view (:1854).
    FormCell {
        control: &[
            ("applicationarea", None),
            ("subpagelink", None),
            ("updatepropagation", Some("both")),
        ],
        host_extra: None,
        landings: &[
            (PageMode::Edit, Landing::Open),
            (PageMode::Edit, Landing::GoToLastRow),
            (PageMode::Edit, Landing::GoToOnlyRow),
            (PageMode::View, Landing::GoToOnlyRow),
        ],
        trigger_landings: &[],
    },
    // Page 76855 "PG EPL List Rep": as opened for edit (:1871) and for view (:1887), GoToRecord to the last row (:1905, :1923).
    FormCell {
        control: REPORTER_CONTROL,
        host_extra: None,
        landings: &[
            (PageMode::Edit, Landing::Open),
            (PageMode::View, Landing::Open),
            (PageMode::Edit, Landing::GoToLastRow),
            (PageMode::View, Landing::GoToLastRow),
        ],
        trigger_landings: REPORTER_TRIGGER_LANDINGS,
    },
    // Page 76856 "PG EPL List RepCap": the part control with Caption and no UpdatePropagation (:1941, :1959).
    FormCell {
        control: &[("applicationarea", None), ("caption", None), ("subpagelink", None)],
        host_extra: None,
        landings: LAST_ROW_EDIT_VIEW,
        trigger_landings: &[],
    },
    // Page 76857 "PG EPL List RepRO": the host with Editable = false (:1977, :1995).
    FormCell {
        control: REPORTER_CONTROL,
        host_extra: Some(("editable", Some("false"))),
        landings: LAST_ROW_EDIT_VIEW,
        trigger_landings: &[],
    },
    // Page 76859 "PG EPL List RepCard": the host with CardPageId (:2013, :2031).
    FormCell {
        control: REPORTER_CONTROL,
        host_extra: Some(("cardpageid", None)),
        landings: LAST_ROW_EDIT_VIEW,
        trigger_landings: &[],
    },
];

impl Session {
    /// Whether `host` is a top-level List page (the probe's hosts, EmptyPartRecLinkList.al page 76843 and its siblings).
    pub(super) fn top_list(&self, host: &St) -> bool {
        let h = host.borrow();
        h.host.is_none() && self.engine().object_facts[h.object as usize].page_type == Some(al_ir::PageType::List)
    }

    /// Whether `st` is a part whose host is a top-level List page.
    pub(super) fn part_of_top_list(&self, st: &St) -> bool {
        let host = st.borrow().host.as_ref().and_then(|(h, _)| h.upgrade());
        host.is_some_and(|h| self.top_list(&h))
    }

    /// `Ok` when part `st` of `host` is a List-host cell the probe measured (see the module): the host, the part control on
    /// it, the link and the part page each hold only facts a probe cell holds; with the number of reads by an action of the part
    /// the cell measured after its landing (`Session::measured_read`). Else the first fact no probe cell holds.
    pub(super) fn list_host_cell(&mut self, st: &St, host: &St) -> Result<Result<ListCell, &'static str>, Signal> {
        let Some(control) = st.borrow().host.as_ref().map(|(_, c)| *c) else {
            return Ok(Err("the part has no host"));
        };
        let (host_extra, triggers) = match self.list_host_measured(host, st) {
            Ok(facts) => facts,
            Err(miss) => return Ok(Err(miss)),
        };
        if !self.list_part_page_measured(st) {
            return Ok(Err(
                "the part page has a property, a trigger, a field control's source, a page extension or an event subscriber no \
                 probe part has",
            ));
        }
        let host_table = self.table(&self.page_rec(host))?;
        let part_table = self.table(&self.page_rec(st))?;
        if let Some(miss) = self.list_part_control_miss(host, control, &host_table, &part_table) {
            return Ok(Err(miss));
        }
        if host_extra.is_none() && self.part_control_props_in(host, control, &PART_CONTROL_PROPS) {
            if triggers {
                return self.first_form_trigger_cell(st, host, control);
            }
            if !self.list_landing_measured(st, host) {
                return Ok(Err(
                    "the part read again at a host move other than the open, or a GoToRecord or a Next as the first call since the \
                     open on a host without a filter",
                ));
            }
            // Two reads after a GoToRecord that left the row (EmptyPartRecLinkList.al:1370 at commit 831ae544,
            // EmptyPart_ListOpenEdit_GoToRecord_FactBoxConstField_ActionTwice `act=76840/B/0/ | act=76840/B/0/`); one read after
            // the open, the other GoToRecord landings and Next.
            let goto_left = self.host_move_running == Some(HostMove::moved(PageMethod::GoToRecord));
            let reads = if goto_left { 2 } else { 1 };
            return Ok(Ok(ListCell { reads, triggers }));
        }
        let Some(cell) = FORM_CELLS.iter().find(|cell| {
            cell.host_extra == host_extra
                && self.part_control_props_in(host, control, cell.control)
                && self.part_control_sets_all(host, control, cell.control)
        }) else {
            return Ok(Err(
                "the part control has a set of properties, or the List host a page property, no probe cell has",
            ));
        };
        if triggers {
            return self.trigger_landing_cell(st, host, cell.trigger_landings);
        }
        if !self.form_cell_facts(st, host, control, &HOST_TRIGGERS[..1]) {
            return Ok(Err(
                "the part page, the part control's area or triggers, the link or the host's triggers differ from the cells of its \
                 host form",
            ));
        }
        let mode = host.borrow().mode;
        let landing = self.form_landing(st, host)?;
        if !landing.is_some_and(|l| cell.landings.contains(&(mode, l))) {
            return Ok(Err(
                "the part read again at a host move or in an open form no probe cell of its host form has",
            ));
        }
        // Every cell of a host form reads once, with one action (EmptyPartRecLinkList.al:1800-2031).
        Ok(Ok(ListCell { reads: 1, triggers }))
    }

    /// The first List host shape with the triggers of probe 1378 (EmptyPartRecListHostTriggers.al pages 76865-76867): the part is
    /// page 76863 "PG EPT Part" (as EmptyPartRecLinkList.al page 76842: InsertAllowed = false and no DeleteAllowed, DelayedInsert
    /// or Caption, OnAfterGetCurrRecord and an action's OnAction only), in area(FactBoxes), linked by a `const(..)` then a field
    /// link (:247 `"Table ID" = const(76860), "No." = field("No.")`), at a landing of `FIRST_FORM_TRIGGER_LANDINGS`.
    fn first_form_trigger_cell(
        &mut self,
        st: &St,
        host: &St,
        control: usize,
    ) -> Result<Result<ListCell, &'static str>, Signal> {
        let shape = {
            let engine = self.engine();
            let page = page_of(&engine, st);
            let props = &engine.program.objects[st.borrow().object as usize].properties;
            let part_props = props_measured(props, &PART_PROPS[..5]);
            let part_triggers = trigger_set_measured(&page.triggers, &PART_TRIGGERS[..1]);
            let control_triggers = control_trigger_set(page) == PART_CONTROL_TRIGGERS[0];
            let c = &page_of(&engine, host).controls[control];
            part_props && part_triggers && control_triggers && c.factbox && const_then_field(c)
        };
        if !shape {
            return Ok(Err(
                "the part page, the part control's area or the link differ from the cells of a List host with page triggers",
            ));
        }
        self.trigger_landing_cell(st, host, FIRST_FORM_TRIGGER_LANDINGS)
    }

    /// The landing of a host with the triggers of probe 1378 is one of `landings` (its open form, the landing as the test's first
    /// call since the open on a host without a filter, and the rows around the host's row), and the test binds no
    /// SendNotificationHandler but the probe's (EmptyPartRecListHostTriggers.al:567-572; :1109 and :1639 bind none). One read.
    fn trigger_landing_cell(
        &mut self,
        st: &St,
        host: &St,
        landings: &[TriggerLanding],
    ) -> Result<Result<ListCell, &'static str>, Signal> {
        let handlers = self.bound_handlers(crate::UiKind::SendNotification);
        let engine = self.engine();
        let handlers_measured = handlers
            .iter()
            .all(|&(o, r)| super::empty_part_rec_list_triggers::notification_handler_measured(&engine.program, o, r));
        drop(engine);
        if !handlers_measured {
            return Ok(Err("the test binds a SendNotificationHandler no probe test has"));
        }
        let landing = if self.part_open_running && self.page_rec(host).borrow().filters.is_empty() {
            Some(Landing::Open)
        } else {
            match self.host_move_running {
                Some(m) if m == HostMove::moved(PageMethod::GoToRecord) && self.first_host_step(st, host, m.method) => {
                    Some(Landing::GoToLastRow)
                }
                Some(m) if m == HostMove::moved(PageMethod::Next) && self.first_host_step(st, host, m.method) => {
                    Some(Landing::NextToLastRow)
                }
                _ => None,
            }
        };
        let mode = host.borrow().mode;
        let rows = self.host_rows(host)?;
        if !landing.is_some_and(|l| landings.contains(&(mode, l, rows))) {
            return Ok(Err(
                "the part read again at a host move, in an open form or with a number of host rows no probe cell of a List host \
                 with page triggers has",
            ));
        }
        Ok(Ok(ListCell {
            reads: 1,
            triggers: true,
        }))
    }

    /// The rows of the host's view before and after the row it stands on, each counted up to 3.
    fn host_rows(&mut self, host: &St) -> Result<Rows, Signal> {
        let rec = self.page_rec(host);
        Ok((-self.host_rows_beyond(&rec, -3)?, self.host_rows_beyond(&rec, 3)?))
    }

    /// The host of part `st`: a top-level List page the test opened with TestPage.OpenEdit or OpenView, standing on a stored row,
    /// with the properties and triggers of a probe host, every field control bound to a field of Rec, no page extension and no
    /// event subscriber. `Ok` with the one property of a `FORM_CELLS` host it has besides `HOST_PROPS`, if any, and whether its
    /// triggers are those of probe 1378 (`empty_part_rec_list_triggers::host_trigger_cell`); else the fact.
    fn list_host_measured(&self, host: &St, st: &St) -> Result<(HostExtra, bool), &'static str> {
        let part = st.borrow().object as usize;
        let engine = self.engine();
        let (object, opened, stored_row) = {
            let h = host.borrow();
            (
                h.object as usize,
                h.opened_by_test && matches!(h.mode, PageMode::Edit | PageMode::View),
                h.cursor == Cursor::Row && !h.draft,
            )
        };
        if !self.top_list(host) {
            return Err("the host is not a top-level List page");
        }
        if !opened {
            return Err("the List host was not opened by the test's OpenEdit or OpenView");
        }
        if !stored_row {
            return Err("the host stands on no stored row");
        }
        let extra: Vec<&(String, String)> = engine.program.objects[object]
            .properties
            .iter()
            .filter(|p| !props_measured(std::slice::from_ref(*p), &HOST_PROPS))
            .collect();
        let host_extra = match extra.as_slice() {
            [] => None,
            [(k, v)] => {
                let found = FORM_CELLS.iter().filter_map(|c| c.host_extra).find(|(name, want)| {
                    k.eq_ignore_ascii_case(name) && want.is_none_or(|w| v.trim().eq_ignore_ascii_case(w))
                });
                Some(found.ok_or("the List host has a page property no probe host has")?)
            }
            _ => return Err("the List host has a page property no probe host has"),
        };
        let triggers = !trigger_set_measured(&page_of(&engine, host).triggers, &HOST_TRIGGERS);
        if triggers && !super::empty_part_rec_list_triggers::host_trigger_cell(&engine.program, object, part) {
            return Err("the List host has a set of triggers no probe host has");
        }
        if !self.unextended_unsubscribed(host) {
            return Err("the List host has a page extension or an event subscriber");
        }
        if !field_controls_bound_to_rec(page_of(&engine, host)) {
            return Err("the List host has a field control whose source is not a field of Rec");
        }
        Ok((host_extra, triggers))
    }

    /// The part lands as the host opens (EmptyPartRecLinkList.al :347), after a GoToRecord that left the row (:379), after one
    /// that stayed on it (a List with one row and the first of two rows, EmptyPartRecLinkList.al
    /// EmptyPart_ListOpenEdit_OneHead_GoToSameRow_FactBox_ActionReadsRec, EmptyPart_ListOpenEdit_GoToFirstRow_FactBox_ActionReadsRec)
    /// or after Next (:434), the move as the test's first call since the open (`first_host_step`).
    fn list_landing_measured(&self, st: &St, host: &St) -> bool {
        match self.host_move_running {
            _ if self.part_open_running => true,
            Some(HostMove {
                method: method @ (PageMethod::GoToRecord | PageMethod::Next),
                ..
            }) => self.first_host_step(st, host, method),
            _ => false,
        }
    }

    /// The running host move `method` is the test's first call on the host and its parts since the open, with no write to the
    /// store between them, on a host without a filter: every cell of probe 1339 that moves the host (EmptyPartRecLinkList.al at
    /// commit 831ae544: `Host.OpenEdit()` or `Host.OpenView()`, then one `Host.GoToRecord(Head)` or `Host.Next()`, then the
    /// read).
    fn first_host_step(&self, st: &St, host: &St, method: PageMethod) -> bool {
        Self::part_steps(st) == Some(vec![PartStep::Host(TestCall::Page(method))])
            && self.page_rec(host).borrow().filters.is_empty()
    }

    /// The landing of a `FORM_CELLS` form: the open, a GoToRecord that moved the host to the last of its rows, or a GoToRecord
    /// that stayed on the host's only row, the GoToRecord as the test's first call since the open (`first_host_step`;
    /// EmptyPartRecLinkList.al:1818-2031). `None` for any other.
    fn form_landing(&mut self, st: &St, host: &St) -> Result<Option<Landing>, Signal> {
        if self.part_open_running {
            return Ok(Some(Landing::Open));
        }
        let Some(HostMove {
            method: PageMethod::GoToRecord,
            stayed,
        }) = self.host_move_running
        else {
            return Ok(None);
        };
        if !self.first_host_step(st, host, PageMethod::GoToRecord) {
            return Ok(None);
        }
        let rec = self.page_rec(host);
        let row_after = self.host_row_beyond(&rec, 1)?;
        let row_before = self.host_row_beyond(&rec, -1)?;
        Ok(match (stayed, row_after, row_before) {
            (false, false, true) => Some(Landing::GoToLastRow),
            (true, false, false) => Some(Landing::GoToOnlyRow),
            _ => None,
        })
    }

    /// Whether the host's view has a row `step` (1 or -1) from the row `rec` stands on.
    fn host_row_beyond(&mut self, rec: &Rec, step: i32) -> Result<bool, Signal> {
        Ok(self.host_rows_beyond(rec, step)? != 0)
    }

    /// How many rows the host's view moves from the row `rec` stands on, up to `step` (signed, as Next returns it).
    fn host_rows_beyond(&mut self, rec: &Rec, step: i32) -> Result<i32, Signal> {
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        let ret_used = std::mem::replace(&mut self.ret_used, true);
        let moved = self.record_op(RecordOp::Next, &probe, &mut [Value::Integer(step)]);
        self.ret_used = ret_used;
        Ok(match moved? {
            Value::Integer(n) => n,
            _ => 0,
        })
    }

    /// Every property of the part control is one of `allowed`, with a literal value it names.
    fn part_control_props_in(&self, host: &St, control: usize, allowed: &[(&str, Option<&str>)]) -> bool {
        let engine = self.engine();
        let props = &page_of(&engine, host).controls[control].props;
        props.iter().all(|(k, v)| {
            allowed.iter().any(|(name, want)| {
                k == name
                    && match (want, v) {
                        (None, ir::PropVal::Lit(_)) => true,
                        (Some(want), ir::PropVal::Lit(v)) => v.trim().eq_ignore_ascii_case(want),
                        _ => false,
                    }
            })
        })
    }

    /// The part control sets every property of `wanted`.
    fn part_control_sets_all(&self, host: &St, control: usize, wanted: &[(&str, Option<&str>)]) -> bool {
        let engine = self.engine();
        let props = &page_of(&engine, host).controls[control].props;
        wanted.iter().all(|(name, _)| props.iter().any(|(k, _)| k == name))
    }

    /// The facts every `FORM_CELLS` cell holds besides its host form's (EmptyPartRecLinkList.al pages 76854-76859 at commit
    /// 831ae544): the part is page 76851 "PG EPL Part Doc" (page 1178's triggers, DeleteAllowed = true and DelayedInsert = true,
    /// an action's OnAction and a field's OnDrillDown), its control sits in area(FactBoxes), the host has no trigger, and the link
    /// is a `const(..)` on an Integer field then a field link (page 76854 `"Table ID" = const(..), "No." = field("No.")`,
    /// page 76855 `"Table ID" = const(..), "No." = field("Ref Code")`). A `filter(..)` in place of the `const(..)` is no cell.
    fn form_cell_facts(&self, st: &St, host: &St, control: usize, host_triggers: &[&[&str]]) -> bool {
        let engine = self.engine();
        let page = page_of(&engine, st);
        let host_page = page_of(&engine, host);
        let props = &engine.program.objects[st.borrow().object as usize].properties;
        let doc_props = props_set(props, &PART_DOC_REQUIRED);
        let doc_triggers = trigger_set_measured(&page.triggers, &PART_TRIGGERS[1..]);
        let doc_control_triggers = control_trigger_set(page) == PART_CONTROL_TRIGGERS[1];
        let c = &host_page.controls[control];
        let host_triggers = host_triggers.is_empty() || trigger_set_measured(&host_page.triggers, host_triggers);
        doc_props && doc_triggers && doc_control_triggers && c.factbox && host_triggers && const_then_field(c)
    }

    /// The part control: the List page's own (no page extension), with no Provider and the Provider of no other part, and a
    /// link of `const(..)`, `filter(..)` and `field(..)` entries of the probe's types: a const or filter on an Integer field, a
    /// field link from a Code field to a Code field, none on the last key field.
    /// `None` when it is; else the fact.
    fn list_part_control_miss(
        &self,
        host: &St,
        control: usize,
        host_table: &TableDef,
        part_table: &TableDef,
    ) -> Option<&'static str> {
        let engine = self.engine();
        let page = page_of(&engine, host);
        let c = &page.controls[control];
        let Some(link) = c.part_link.as_ref() else {
            return Some("the part control has no SubPageLink");
        };
        let provider_of_none = page
            .controls
            .iter()
            .all(|o| o.part_link.as_ref().and_then(|l| l.provider) != Some(control));
        let short_of_last_key = |f: usize| {
            part_table.keys[0]
                .iter()
                .position(|k| *k == f)
                .is_some_and(|i| i + 1 < part_table.keys[0].len())
        };
        let integer = |f: usize| part_table.fields[f].ty == al_builtins::value::RtType::Integer;
        let code = |ty: &al_builtins::value::RtType| matches!(ty, al_builtins::value::RtType::Code(_));
        let entries = link.entries.iter().all(|e| match e {
            ir::LinkEntry::Filter(f, _) => integer(*f) && short_of_last_key(*f),
            ir::LinkEntry::Field(f, hf) => {
                code(&part_table.fields[*f].ty) && code(&host_table.fields[*hf].ty) && short_of_last_key(*f)
            }
            ir::LinkEntry::FieldFilter(..) => false,
        });
        if c.owner.is_some() {
            return Some("the part control is added by a page extension");
        }
        if !(link.view.is_none() && link.provider.is_none() && provider_of_none) {
            return Some("the part control has a SubPageView, a Provider or is the Provider of another part");
        }
        if !entries {
            return Some("the SubPageLink has an entry of a form, a field type or a key position no probe link has");
        }
        None
    }

    /// The part page: a ListPart on a stored table with the properties and triggers of a probe part, every field control
    /// bound to a field of Rec, no page extension and no event subscriber.
    fn list_part_page_measured(&self, st: &St) -> bool {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let page = page_of(&engine, st);
        let props = &engine.program.objects[object].properties;
        props_measured(props, &PART_PROPS)
            && props_set(props, &PART_REQUIRED)
            && trigger_set_measured(&page.triggers, &PART_TRIGGERS)
            && PART_CONTROL_TRIGGERS.contains(&control_trigger_set(page).as_slice())
            && field_controls_bound_to_rec(page)
            && self.unextended_unsubscribed(st)
    }
}

/// The part control's link is a `const(..)` on its first field then a field link (EmptyPartRecLinkList.al page 76854
/// `"Table ID" = const(..), "No." = field("No.")`).
fn const_then_field(c: &ir::Control) -> bool {
    c.part_link.as_ref().is_some_and(|l| {
        matches!(
            l.entries.as_slice(),
            [ir::LinkEntry::Filter(..), ir::LinkEntry::Field(..)]
        ) && l.consts == [0]
            && l.enum_values.is_empty()
    })
}

/// Every property in `props` is one of `allowed`, with the value it names (case-insensitive).
fn props_measured(props: &[(String, String)], allowed: &[(&str, Option<&str>)]) -> bool {
    props.iter().all(|(k, v)| {
        allowed
            .iter()
            .any(|(name, want)| k.eq_ignore_ascii_case(name) && want.is_none_or(|w| v.trim().eq_ignore_ascii_case(w)))
    })
}

/// Every property of `required` is in `props` with the value it names (case-insensitive).
fn props_set(props: &[(String, String)], required: &[(&str, &str)]) -> bool {
    required.iter().all(|(name, want)| {
        props
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case(name) && v.trim().eq_ignore_ascii_case(want))
    })
}

/// Every field control of the page has a field of Rec as its source: the probe's pages have no other (EmptyPartRecLinkList.al at
/// commit 831ae544, the parts :75-76 and :300-309 `field(LineNoCtl; Rec."Line No.")`, `field(KindCtl; Rec.Kind)`, every host
/// as :120-121 `field(NoCtl; Rec."No.")`, `field(RefCtl; Rec."Ref Code")`). A source expression is code that reads Rec, and when
/// BC runs it is not measured.
fn field_controls_bound_to_rec(page: &ir::Page) -> bool {
    let mut fields = page.controls.iter().filter(|c| c.kind == ir::ControlKind::Field);
    fields.all(|c| matches!(c.binding, ir::Binding::Field(_)))
}

/// The trigger names of the page's controls, sorted, each once.
fn control_trigger_set(page: &ir::Page) -> Vec<&str> {
    let mut names: Vec<&str> = page
        .controls
        .iter()
        .flat_map(|c| c.triggers.iter().map(|(n, _)| n.as_str()))
        .collect();
    names.sort_unstable();
    names.dedup();
    names
}

/// The page's trigger names, sorted, are exactly one of the `measured` sets.
fn trigger_set_measured(triggers: &[(String, usize)], measured: &[&[&str]]) -> bool {
    let mut names: Vec<&str> = triggers.iter().map(|(n, _)| n.as_str()).collect();
    names.sort_unstable();
    measured.contains(&names.as_slice())
}
