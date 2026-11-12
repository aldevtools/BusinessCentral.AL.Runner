//! The facts an entry predicate of a rule that is pinned for ONE corpus scenario compares with that scenario: the code the
//! page and its table can run, the properties the page declares, and the calls the test made on the page since it opened.
//! Each predicate states the pinned value of every fact and requires equality (al-runner PR 7807 review, round 3: a rule that
//! asks only "is not X" also runs on shapes no probe shows). The predicates are next to their rules: `long_view.rs`,
//! `open/temp_ws_preloaded.rs`, `currpage_run_typed_key.rs`, and for a part on a host `split_key_view.rs`, `part_unsettled.rs`,
//! `part_filter_find.rs` and `end_move_no_read.rs` (al-runner PR 7805 review, round 3), which compare the calls on the host and
//! on the part in the order the test made them (`part_steps`) and the pages of the pin (`PinnedPart`). A rule pinned for an
//! outside reporter's repro compares the objects line for line (`pinned_body.rs`). A rule pinned for one repro or one corpus
//! scenario lets through only the calls the scenario makes after it (`PinnedEnd`).

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

/// One call on a page or on one of its direct parts (`PageState::tree_calls`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeCall {
    /// The control index of the part the call was made on; `None` for the page itself.
    pub part: Option<usize>,
    pub call: TestCall,
    /// The call answered No (a move that did not move, `GoToKey` that found nothing).
    pub no: bool,
    /// The call raised an error (a SetValue whose validation or save failed).
    pub failed: bool,
}

/// One call of `PageState::tree_calls` as a part of the page sees it (`Session::part_steps`). The reads of the part's own
/// controls and of its filter are left out: every pin reads between its steps, and a read changes nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartStep {
    /// A call on the host or on one of its controls, a read too, that answered Yes or nothing.
    Host(TestCall),
    /// A call on the host that answered No.
    HostNo(TestCall),
    /// A call on the part or on one of its controls that answered Yes or nothing.
    Own(TestCall),
    /// A call on the part that answered No.
    OwnNo(TestCall),
    /// A call on another part of the host.
    Sibling,
}

/// What a page lets through after a rule pinned for one scenario ran (`PageState::pinned_end`): Close of the page, with
/// `reads` a read of a control's value or an `AssertEquals`, with `one_action` one action's Invoke, and with `first_ends` a
/// `First`, which ends the limit, as the scenario makes them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PinnedEnd {
    /// The refusal of any other call.
    pub reason: &'static str,
    pub reads: bool,
    /// One `Invoke` of an action is let through; a second one is refused.
    pub one_action: bool,
    /// `First` is let through and ends the limit: the page then reads its view again, as on main's path.
    pub first_ends: bool,
}

/// More calls on a page and its parts than this are not kept; no pinned scenario of a part has that many.
const KEPT_TREE_CALLS: usize = 32;

/// The pages of a pinned part scenario (see `Session::part_pages_are`): each list is what the pages of the pin declare, and
/// the pages asked may declare nothing outside it.
pub(super) struct PinnedPart<'a> {
    pub host_type: ir::PageType,
    /// The host's own triggers, lower case.
    pub host_triggers: &'a [&'a str],
    /// The part's own triggers, lower case.
    pub part_triggers: &'a [&'a str],
    /// The properties the part may declare next to the plain ones (see `PLAIN_PROPERTIES`), lower case.
    pub part_properties: &'a [&'a str],
    /// The part's controls may be bound to a page variable (else: fields of Rec only). No control of Rec has a trigger.
    pub part_variable_controls: bool,
    /// A control bound to a page variable may have an OnValidate trigger.
    pub part_variable_validate: bool,
    /// A control bound to a field of Rec may have an OnValidate trigger whose only statement is `CurrPage.Update(true)`.
    pub part_field_validate_update: bool,
    /// The part's controls may show an expression.
    pub part_expression_controls: bool,
    /// The part may have no SubPageLink.
    pub unlinked: bool,
    /// The host may show other parts next to this one.
    pub sibling_parts: bool,
    /// The test's own filter (filter group 0) may be set on the part, next to the link's.
    pub user_filter: bool,
}

/// Properties that run nothing and change no read.
pub(super) const PLAIN_PROPERTIES: [&str; 5] =
    ["pagetype", "sourcetable", "applicationarea", "usagecategory", "caption"];

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
    /// The properties an Action control of the page may set to an expression, lower case; its other properties are
    /// literal. Empty: the page has no Action control.
    pub action_props: &'a [&'a str],
}

impl Session {
    /// After a rule pinned for one repro ran on the page tree of `st` (`PageState::pinned_end`), only the calls the repro makes
    /// after it are let through; `call` is the test's call on `st` (`note_test_call`).
    pub(super) fn refuse_after_pinned_end(st: &St, call: Option<TestCall>) -> Result<(), Signal> {
        let (root, part) = Self::tree_root(st);
        let Some(end) = root.borrow().pinned_end else {
            return Ok(());
        };
        let let_through = match call {
            // Not a call of the test client (`note_test_call`).
            None => true,
            Some(TestCall::Page(PageMethod::Close)) => part.is_none(),
            Some(TestCall::Page(PageMethod::First)) if end.first_ends && part.is_none() => {
                root.borrow_mut().pinned_end = None;
                true
            }
            Some(TestCall::Read) => end.reads,
            Some(TestCall::Other(PageMethod::Invoke)) if end.one_action && part.is_none() => {
                root.borrow_mut().pinned_end = Some(PinnedEnd {
                    one_action: false,
                    ..end
                });
                true
            }
            Some(TestCall::Other(m)) => end.reads && (m == PageMethod::AssertEquals || Self::field_read(m)),
            _ => false,
        };
        if let_through { Ok(()) } else { unsupported(end.reason) }
    }

    /// The test client calls `method` on `st`, on the page itself or on `control`; the call as the pins see it.
    pub(super) fn note_test_call(
        &self,
        st: &St,
        method: PageOp,
        control: Option<&ControlTarget>,
        args: usize,
    ) -> Option<TestCall> {
        if method.recv() == PageRecv::Page {
            return None;
        }
        let m = method.method();
        let call = match (control, m) {
            (None, _) => TestCall::Page(m),
            (Some(_), PageMethod::Value) if args == 0 => TestCall::Read,
            (Some(ControlTarget::Control(i)), PageMethod::Value | PageMethod::SetValue) => TestCall::Write(*i),
            (Some(_), _) => TestCall::Other(m),
        };
        self.note_tree_call(st, call);
        let mut s = st.borrow_mut();
        if s.test_calls.len() < KEPT_CALLS {
            s.test_calls.push(call);
        } else {
            s.test_calls_overflow = true;
        }
        Some(call)
    }

    /// The page `tree_calls` of `st` are kept on (`st` itself, or the host of a part) and the part's control index there.
    fn tree_root(st: &St) -> (St, Option<usize>) {
        match st
            .borrow()
            .host
            .as_ref()
            .and_then(|(h, i)| h.upgrade().map(|h| (h, *i)))
        {
            Some((host, idx)) => (host, Some(idx)),
            None => (st.clone(), None),
        }
    }

    /// `call` on `st` joins the calls of its page tree. A write to the store since the call before it ended was made by the
    /// test between two calls: no pin does that, so the calls no longer tell what the page shows.
    fn note_tree_call(&self, st: &St, call: TestCall) {
        let (root, part) = Self::tree_root(st);
        let nested = root.borrow().host.is_some();
        let mark = self.write_mark();
        let mut r = root.borrow_mut();
        if nested || r.tree_calls.len() >= KEPT_TREE_CALLS || r.tree_calls_mark.is_some_and(|m| m != mark) {
            r.tree_calls_unknown = true;
        }
        if r.tree_calls.len() < KEPT_TREE_CALLS {
            r.tree_calls.push(TreeCall {
                part,
                call,
                no: false,
                failed: false,
            });
        }
    }

    /// The test's call on `st` ended with `result`: its answer, whether it raised, and the store's write mark are noted.
    pub(super) fn test_call_ended(&self, st: &St, result: &Result<Value, Signal>) {
        let (root, _) = Self::tree_root(st);
        let mut r = root.borrow_mut();
        r.tree_calls_mark = Some(self.write_mark());
        if result.is_err()
            && let Some(last) = r.tree_calls.last_mut()
        {
            last.failed = true;
        }
        if matches!(result, Ok(Value::Boolean(false)))
            && let Some(last) = r.tree_calls.last_mut()
        {
            last.no = true;
        }
    }

    /// The calls the test made on the host of `part` and on its parts since the host opened, as `part` sees them, the running
    /// one last. `None` when the calls do not tell what the page shows (`PageState::tree_calls_unknown`), and for a page
    /// that is no direct part of a top-level page.
    pub(super) fn part_steps(part: &St) -> Option<Vec<PartStep>> {
        let (host, idx) = Self::tree_root(part);
        let idx = idx?;
        let h = host.borrow();
        if h.host.is_some() || h.tree_calls_unknown {
            return None;
        }
        let read = |c: &TestCall| match c {
            // `P.Part.Filter` only hands out the filter.
            TestCall::Read | TestCall::Page(PageMethod::Filter) => true,
            TestCall::Other(m) => *m == PageMethod::GetFilter || Self::field_read(*m),
            _ => false,
        };
        Some(
            h.tree_calls
                .iter()
                .filter(|c| !(c.part == Some(idx) && read(&c.call)))
                .map(|c| match c.part {
                    None if c.no => PartStep::HostNo(c.call),
                    None => PartStep::Host(c.call),
                    Some(i) if i != idx => PartStep::Sibling,
                    Some(_) if c.no => PartStep::OwnNo(c.call),
                    Some(_) => PartStep::Own(c.call),
                })
                .collect(),
        )
    }

    /// Whether page `p` declares no trigger outside `triggers` and no property outside the plain ones and `properties` (lower
    /// case), has no page extension and no event subscriber, and its controls come from the page itself, with literal
    /// properties and no extension trigger.
    pub(super) fn page_declares_within(&self, p: &St, triggers: &[&str], properties: &[&str]) -> bool {
        let engine = self.engine();
        let object = p.borrow().object as usize;
        let page = page_of(&engine, p);
        page.triggers.iter().all(|(n, _)| triggers.contains(&n.as_str()))
            && page.extensions.is_empty()
            && page.ext_triggers.is_empty()
            && !engine.events.any_subscriber_of(object)
            && engine.program.objects[object].properties.iter().all(|(k, _)| {
                let k = k.to_ascii_lowercase();
                PLAIN_PROPERTIES.contains(&k.as_str()) || properties.contains(&k.as_str())
            })
            && page.controls.iter().all(|c| {
                c.owner.is_none()
                    && c.ext_triggers.is_empty()
                    && c.props.iter().all(|(_, v)| matches!(v, ir::PropVal::Lit(_)))
            })
    }

    /// Whether `part` and its host are pages as `pin` lists them, every fact required:
    /// - the host: a top-level page of `pin.host_type` that declares no property but the plain ones and no trigger outside
    ///   `pin.host_triggers`; its controls are fields of Rec without a trigger, groups and this one part (other parts with
    ///   `pin.sibling_parts`);
    /// - the part: a ListPart that declares no property outside `pin.part_properties` and no trigger outside
    ///   `pin.part_triggers`; its controls are a repeater and fields of Rec without a trigger (with
    ///   `pin.part_variable_controls` also page variables, with `pin.part_variable_validate` an OnValidate trigger on them,
    ///   with `pin.part_expression_controls` also expressions);
    /// - neither page has a page extension or an event subscriber, and neither does the part's source table have a table
    ///   extension or a subscriber;
    /// - the part control: a SubPageLink of `field(..)` entries only, on the leading fields of the part's primary key in
    ///   their order (none at all with `pin.unlinked`), no Provider, and a SubPageView that only sorts (no `where`); the same
    ///   for the part's own SourceTableView;
    /// - the part's Rec: a stored table, no marks, and no filter in any filter group but the link's on the link's fields (with
    ///   `pin.user_filter` also the test's own, filter group 0).
    pub(super) fn part_pages_are(&self, part: &St, pin: &PinnedPart) -> Result<bool, Signal> {
        let (host, Some(idx)) = Self::tree_root(part) else {
            return Ok(false);
        };
        if host.borrow().host.is_some() {
            return Ok(false);
        }
        let engine = self.engine();
        let object_of = |p: &St| p.borrow().object as usize;
        let page_type = |p: &St| engine.object_facts[object_of(p)].page_type.clone();
        let declares_within =
            |p: &St, triggers: &[&str], properties: &[&str]| self.page_declares_within(p, triggers, properties);
        let plain_field = |c: &ir::Control| matches!(c.binding, ir::Binding::Field(_)) && c.triggers.is_empty();
        let host_page = page_of(&engine, &host);
        let host_ok = page_type(&host) == Some(pin.host_type.clone())
            && declares_within(&host, pin.host_triggers, &[])
            && host_page.controls.iter().enumerate().all(|(i, c)| match c.kind {
                ir::ControlKind::Group => true,
                ir::ControlKind::Field => plain_field(c),
                ir::ControlKind::Part => i == idx || pin.sibling_parts,
                _ => false,
            });
        let part_page = page_of(&engine, part);
        let part_ok = page_type(part) == Some(ir::PageType::Other("listpart".into()))
            && declares_within(part, pin.part_triggers, pin.part_properties)
            && part_page.controls.iter().all(|c| match c.kind {
                ir::ControlKind::Repeater => true,
                ir::ControlKind::Field => match c.binding {
                    ir::Binding::Field(_) => {
                        c.triggers.is_empty()
                            || (pin.part_field_validate_update
                                && matches!(c.triggers.as_slice(), [(n, r)] if n == "onvalidate"
                                    && matches!(code_stmts(&engine.program, object_of(part), *r).as_slice(),
                                        [s] if is_curr_page_update_with(s, part_page.curr_page_slot, Some(true)))))
                    }
                    ir::Binding::Var(_) => {
                        pin.part_variable_controls
                            && (c.triggers.is_empty()
                                || (pin.part_variable_validate && c.triggers.iter().all(|(n, _)| n == "onvalidate")))
                    }
                    ir::Binding::Expr(_) => pin.part_expression_controls && c.triggers.is_empty(),
                    _ => false,
                },
                _ => false,
            });
        if !host_ok || !part_ok {
            return Ok(false);
        }
        let rec = self.page_rec(part);
        let table = self.table(&rec)?;
        let table_object = table.object as usize;
        let table_ok = engine.program.objects[table_object]
            .table
            .as_ref()
            .is_some_and(|x| x.extensions.is_empty())
            && !engine.events.any_subscriber_of(table_object);
        let sorts_only = |view: Option<&str>| view.is_none_or(|v| !v.to_ascii_lowercase().contains("where"));
        let link = host_page.controls[idx].part_link.as_ref();
        let link_fields: Option<Vec<usize>> = match link {
            None => Some(Vec::new()),
            Some(l) => l
                .entries
                .iter()
                .map(|e| match e {
                    ir::LinkEntry::Field(part_field, _) => Some(*part_field),
                    _ => None,
                })
                .collect(),
        };
        let Some(link_fields) = link_fields else {
            return Ok(false);
        };
        let key = &table.keys[0];
        let link_ok = link.is_none_or(|l| l.provider.is_none() && sorts_only(l.view.as_deref()))
            && if link_fields.is_empty() {
                pin.unlinked
            } else {
                link_fields.len() < key.len() && key[..link_fields.len()] == link_fields[..]
            }
            && sorts_only(
                engine.program.objects[object_of(part)]
                    .properties
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case("SourceTableView"))
                    .map(|(_, v)| v.as_str()),
            );
        let r = rec.borrow();
        let rec_ok = !r.temporary
            && !r.marked_only
            && r.filters.iter().all(|f| {
                (f.group == super::parts::LINK_GROUP && link_fields.contains(&f.field))
                    || (pin.user_filter && f.group == 0)
            });
        Ok(table_ok && link_ok && rec_ok)
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
                ir::ControlKind::Action => !pin.action_props.is_empty() && c.binding == ir::Binding::None,
                _ => false,
            };
            let action = c.kind == ir::ControlKind::Action;
            kind && c.owner.is_none()
                && c.ext_triggers.is_empty()
                && c.props.iter().all(|(k, v)| {
                    matches!(v, ir::PropVal::Lit(_))
                        || (action && pin.action_props.contains(&k.to_ascii_lowercase().as_str()))
                })
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

    /// The stored rows of the table of `rec` that pass `filters` alone (none of `rec`'s own filters or marks): a fact the
    /// entry predicates compare with the rows their pin stored.
    pub(super) fn stored_rows_passing(
        &mut self,
        rec: &Rec,
        filters: Vec<al_builtins::filter::FieldFilter>,
    ) -> Result<i32, Signal> {
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        {
            let mut rows = probe.borrow_mut();
            rows.filters = filters;
            rows.marks.clear();
            rows.marked_only = false;
        }
        self.ret_used = true;
        Ok(
            match self.record_op(al_builtins::record_op::RecordOp::Count, &probe, &mut [])? {
                Value::Integer(n) => n,
                _ => 0,
            },
        )
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

impl Session {
    /// Whether the last field of the primary key of the part's table is an Integer and the SubPageLink covers every field
    /// before it (the line tables of the pins: document number, line number).
    pub(super) fn part_key_is_link_then_integer(&self, part: &St) -> Result<bool, Signal> {
        let (host, Some(idx)) = Self::tree_root(part) else {
            return Ok(false);
        };
        let engine = self.engine();
        let linked = page_of(&engine, &host).controls[idx]
            .part_link
            .as_ref()
            .map_or(0, |l| l.entries.len());
        let table = self.table(&self.page_rec(part))?;
        let key = &table.keys[0];
        Ok(key.len() == linked + 1
            && key
                .last()
                .is_some_and(|&last| matches!(table.fields[last].ty, RtType::Integer)))
    }

    /// How the host of `part` was opened: by the test's own Open call or not, its mode, modal or not, as a lookup or not.
    pub(super) fn part_host_opened(part: &St) -> Option<(bool, PageMode, bool, bool)> {
        let (host, _) = Self::tree_root(part);
        let h = host.borrow();
        h.open.then_some((h.opened_by_test, h.mode, h.modal, h.lookup))
    }
}

/// The statements of routine `r` of `object` that run code: position, assumption and `--tdd` markers left out.
pub(super) fn code_stmts(program: &ir::Program, object: usize, r: usize) -> Vec<&ir::Stmt> {
    program.objects[object]
        .routines
        .get(r)
        .map_or_else(Vec::new, |routine| {
            routine
                .body
                .stmts()
                .iter()
                .filter(|s| !matches!(s, ir::Stmt::Pos(_) | ir::Stmt::Assume(_) | ir::Stmt::TddMark(_)))
                .collect()
        })
}

/// `e` calls a procedure of a codeunit that a global variable of `object` holds (`Cfg.Query('doc', CloseAction)`), not
/// as a try call.
pub(super) fn codeunit_var_call(program: &ir::Program, object: usize, e: &ir::Expr) -> bool {
    let e = match e {
        ir::Expr::Convert(_, inner) => inner,
        e => e,
    };
    match e {
        ir::Expr::Call {
            instance: Some(inst),
            try_call: false,
            ..
        } => matches!(&**inst, ir::Expr::Load(ir::Place::Global(g))
            if program.objects[object].globals.get(*g).is_some_and(|s| matches!(s.ty, RtType::Codeunit(_)))),
        _ => false,
    }
}

/// Whether trigger `name` (routine `r`) of page `object` has the body the pages of the close-errors fixtures give it
/// (CemFixtures.al:201-224 CEM Card, :412-423 CEM Doc): OnModifyRecord and OnInsertRecord call a procedure of a codeunit
/// variable, then `exit(true)`; OnQueryClosePage is the `exit` of such a call; OnClosePage is such a call. `false` for
/// every other trigger and body.
pub(super) fn cem_trigger_body(program: &ir::Program, object: usize, name: &str, r: usize) -> bool {
    cem_trigger_call(program, object, name, r).is_some()
}

/// The call of a procedure of a codeunit variable in trigger `name` (routine `r`) of page `object`, when the trigger has
/// the body of `cem_trigger_body`; `None` for every other trigger and body.
pub(super) fn cem_trigger_call<'p>(
    program: &'p ir::Program,
    object: usize,
    name: &str,
    r: usize,
) -> Option<&'p ir::Expr> {
    let call = |e: &'p ir::Expr| codeunit_var_call(program, object, e).then_some(e);
    let mut stmts = code_stmts(program, object, r);
    // The lowering ends a trigger with a return value by an `exit` of its default, unreachable after an `exit`.
    if let [.., ir::Stmt::Exit(Some(_)), ir::Stmt::Exit(Some(_))] = stmts.as_slice() {
        stmts.pop();
    }
    match (name, stmts.as_slice()) {
        ("onmodifyrecord" | "oninsertrecord", [ir::Stmt::Expr(e), ir::Stmt::Exit(Some(t))]) => {
            let exits_true = matches!(t, ir::Expr::Const(ir::Const::Boolean(true)))
                | matches!(t, ir::Expr::Convert(_, b) if matches!(**b, ir::Expr::Const(ir::Const::Boolean(true))));
            call(e).filter(|_| exits_true)
        }
        ("onqueryclosepage", [ir::Stmt::Exit(Some(e))]) | ("onclosepage", [ir::Stmt::Expr(e)]) => call(e),
        _ => None,
    }
}

/// `s` is `CurrPage.Update()` without an argument (it saves the row: the parameter defaults to true) on the page whose
/// `CurrPage` is global slot `curr_page`.
pub(super) fn is_curr_page_update(s: &ir::Stmt, curr_page: usize) -> bool {
    is_curr_page_update_with(s, curr_page, None)
}

/// `s` is `CurrPage.Update(save)` with the literal argument `save`, or without an argument for `None`, on the page whose
/// `CurrPage` is global slot `curr_page`.
pub(super) fn is_curr_page_update_with(s: &ir::Stmt, curr_page: usize, save: Option<bool>) -> bool {
    let ir::Stmt::Expr(ir::Expr::Builtin {
        id,
        receiver: Some(receiver),
        args,
        ..
    }) = s
    else {
        return false;
    };
    let args_ok = match (save, args.as_slice()) {
        (None, []) => true,
        (Some(want), [ir::Arg::Value(ir::Expr::Const(ir::Const::Boolean(b)))]) => *b == want,
        _ => false,
    };
    *id == al_builtins::builtin_id::ids::PAGE_UPDATE_1
        && args_ok
        && matches!(&**receiver, ir::Arg::Value(ir::Expr::Load(ir::Place::Global(g))) | ir::Arg::Ref(ir::Place::Global(g))
            if *g == curr_page)
}

/// The code table `object` can run: its own triggers (lower case, sorted) and whether it has a field trigger or a
/// procedure; `None` when it has a table extension or an event subscriber.
pub(super) fn table_code(engine: &Engine, object: usize) -> Option<(Vec<String>, bool)> {
    let t = &engine.program.objects[object];
    if !t.table.as_ref().is_some_and(|x| x.extensions.is_empty()) || engine.events.any_subscriber_of(object) {
        return None;
    }
    Some(own_table_code(t))
}

/// The triggers table `t` declares itself (lower case, sorted) and whether it has a field trigger or a procedure; its table
/// extensions and subscribers are the caller's to check.
pub(super) fn own_table_code(t: &ir::Object) -> (Vec<String>, bool) {
    let mut triggers: Vec<String> = t
        .routines
        .iter()
        .filter(|r| r.is_trigger && r.field.is_none())
        .map(|r| r.name.to_ascii_lowercase())
        .collect();
    triggers.sort_unstable();
    let more = t.routines.iter().any(|r| r.field.is_some() || !r.is_trigger);
    (triggers, more)
}

/// A filter value of the kind the pins of SelectionFilterNoRow.al set (`'SELECTED'` on page 67996, `'VISIBLE'` on page
/// 67999): one or more ASCII capital letters.
pub(super) fn capital_word(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|b| b.is_ascii_uppercase())
}

/// The OnOpenPage of `page` (object `object`) is, statement by statement, the one of pages 67996 and 67999
/// (pages/currpage/SelectionFilterNoRow.al): `Rec.FilterGroup(..); Rec.SetRange(..); Rec.FilterGroup(..); if Rec.FindFirst()
/// then;`. The values the filters take are the callers' to check.
pub(super) fn open_page_filters_then_find(program: &ir::Program, object: usize, page: &ir::Page) -> bool {
    use al_builtins::builtin_id::ids;
    let Some(rec) = program.objects[object].rec_slot else {
        return false;
    };
    let Some((_, r)) = page.triggers.iter().find(|(n, _)| n == "onopenpage") else {
        return false;
    };
    let on_rec = |e: &ir::Expr, want: al_builtins::builtin_id::BuiltinId| {
        matches!(e, ir::Expr::Builtin { id, receiver: Some(receiver), .. } if *id == want
            && matches!(&**receiver, ir::Arg::Value(ir::Expr::Load(ir::Place::Global(g))) | ir::Arg::Ref(ir::Place::Global(g))
                if *g == rec))
    };
    let call = |s: &ir::Stmt, want| matches!(s, ir::Stmt::Expr(e) if on_rec(e, want));
    let empty = |b: &[ir::Stmt]| b.iter().all(|s| matches!(s, ir::Stmt::Pos(_)));
    match code_stmts(program, object, *r).as_slice() {
        [group, range, group_back, ir::Stmt::If(find, then, otherwise)] => {
            call(group, ids::RECORD_FILTERGROUP_1)
                && call(range, ids::RECORD_SETRANGE_1)
                && call(group_back, ids::RECORD_FILTERGROUP_1)
                && on_rec(find, ids::RECORD_FINDFIRST_1)
                && empty(then)
                && empty(otherwise)
        }
        _ => false,
    }
}
