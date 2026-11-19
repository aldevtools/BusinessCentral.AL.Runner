//! The page with code of al-runner issue 8434: a List page like the FactBox host of `code_inserted_new_line.rs`, which also
//! declares Caption, Editable, Extensible and a SourceTableView, runs code in OnOpenPage, OnAfterGetRecord and OnDeleteRecord,
//! carries control expressions and actions, and shows the Notes system part and up to three CardPart FactBoxes with triggers
//! (one enqueues a page background task). Corpus pull request 1454 (open), tests/al-language/pages/currpage/
//! ListNewLineFactboxesCode.al, measures it on Business Central 29.1 (11 of 11 green), under the assumption
//! `list-new-line-coded-factboxes` until a bump pins it. Opened by a Page.RunModal statement on a record filtered to one group,
//! on an empty view (:1401-1476):
//! - the open runs the parts' OnAfterGetCurrRecord on the blank record (`open=P.OPEN;W.AGCR();`), `coded_open_on_empty_view`;
//! - the first SetValue after `New()` runs it again, before the validation (`setname=W.AGCR();T.VN(Station);...`),
//!   `coded_line_before_set_value`;
//! - the validation lands on the row as on the FactBox host (`code_inserted_cell`), and its refresh reads the row twice in a
//!   secondary key and once in the primary key, then the parts (`P.AGR(Station/);P.AGR(Station/);R.AGR(Station/0);
//!   W.AGCR(Station/s);W.DONE(K2);`, :1407 against :1445), `coded_refresh`;
//! - up to two later SetValues whose validation saves the row and calls `CurrPage.Update(false)` refresh the same way
//!   (:1407 `setvalue`, :1539 the value already there, :1576 a second one), then `OK()` runs nothing (`ok=`).
//!
//! The page's facts are in `coded_page_unmeasured`. The procedure the controls call may write other tables between the save
//! and the update, never the page's table (`coded_rows`). The table may have manual subscribers nobody binds (:255-275, :1592
//! binds one: refused). A row before the line (:1477), an error in the validation (:1499), a Value before any Name (:1553)
//! and every other call are refused.

use super::pinned_shape::{TestCall, code_stmts};
use super::state::{CodedLine, FactboxStep};
use super::*;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::page_op::PageMethod;

/// The page properties the probe's pages declare (:497-507, :818-827).
const PAGE_PROPERTIES: [&str; 9] = [
    "pagetype",
    "sourcetable",
    "autosplitkey",
    "applicationarea",
    "usagecategory",
    "caption",
    "editable",
    "extensible",
    "sourcetableview",
];

/// The page triggers of the probe's pages (:743-780).
const PAGE_TRIGGERS: [&str; 3] = ["onopenpage", "onaftergetrecord", "ondeleterecord"];

/// The properties of the probe's field controls (:515-667).
const FIELD_PROPERTIES: [&str; 15] = [
    "applicationarea",
    "caption",
    "tooltip",
    "styleexpr",
    "style",
    "tablerelation",
    "editable",
    "visible",
    "blankzero",
    "drilldown",
    "lookup",
    "captionclass",
    "obsoletestate",
    "obsoletetag",
    "obsoletereason",
];

/// The properties of the probe's actions (:696-737).
const ACTION_PROPERTIES: [&str; 6] = ["applicationarea", "caption", "image", "tooltip", "visible", "enabled"];

/// The page properties of the probe's CardParts (:310-496).
const PART_PAGE_PROPERTIES: [&str; 7] = [
    "caption",
    "editable",
    "pagetype",
    "sourcetable",
    "usagecategory",
    "applicationarea",
    "extensible",
];

/// The properties of the field controls of the probe's CardParts (:310-496).
const PART_FIELD_PROPERTIES: [&str; 7] = [
    "applicationarea",
    "caption",
    "showcaption",
    "multiline",
    "drilldown",
    "style",
    "styleexpr",
];

/// The triggers of the probe's CardParts (:333, :443-496).
const PART_TRIGGERS: [&str; 4] = [
    "onaftergetrecord",
    "onaftergetcurrrecord",
    "onpagebackgroundtaskcompleted",
    "onpagebackgroundtaskerror",
];

impl Session {
    /// Whether page `st` is the page with code (`coded_page_unmeasured`).
    pub(super) fn coded_page(&self, st: &St) -> Result<bool, Signal> {
        Ok(self.coded_page_unmeasured(st)?.is_none())
    }

    /// The first fact of page `st` the probe's pages do not have, `None` when it has only theirs:
    /// - a List page over a stored table, without page extension or subscriber, with the properties of `PAGE_PROPERTIES`
    ///   (AutoSplitKey and Editable true, Extensible false, a SourceTableView that sorts ascending and filters nothing) and the triggers of `PAGE_TRIGGERS`, OnAfterGetRecord among them;
    /// - a repeater with Caption and IndentationColumn at most, field controls over Rec or a variable with the properties of
    ///   `FIELD_PROPERTIES` and an OnValidate at most, actions in groups with the properties of `ACTION_PROPERTIES` and an
    ///   OnAction at most; every property that is an expression reads variables, fields, constants and array elements at
    ///   constant indexes, and calls nothing (`plain_property`);
    /// - three CardParts (`coded_part`) in the measured roles: one with OnAfterGetRecord, one without a trigger, one with the
    ///   background task's triggers; no hidden part;
    /// - the table: no table extension, and no subscriber but manual ones (:255-275). Whether one is bound matters only from the
    ///   landing on, where `no_active_subscriber` refuses it (:1592 binds one).
    fn coded_page_unmeasured(&self, st: &St) -> Result<Option<&'static str>, Signal> {
        // Manual subscribers to the page's events are allowed (:255-275); whether one is bound matters from the landing on.
        let unextended = {
            let engine = self.engine();
            let object = st.borrow().object as usize;
            page_of(&engine, st).extensions.is_empty() && engine.events.manual_subscribers_of(object).is_some()
        };
        if !(self.is_list_page(st) && self.has_source(st) && unextended) {
            return Ok(Some("page"));
        }
        let rec = self.page_rec(st);
        if rec.borrow().temporary {
            return Ok(Some("temporary"));
        }
        let table = self.table(&rec)?;
        let parts = st.borrow().parts.clone();
        if !st.borrow().hidden_parts.is_empty() || parts.len() != 3 {
            return Ok(Some("parts"));
        }
        let mut profiles = Vec::new();
        for (idx, part) in &parts {
            if !self.coded_part(st, *idx, part)? {
                return Ok(Some("part"));
            }
            let engine = self.engine();
            let mut names: Vec<String> = page_of(&engine, part).triggers.iter().map(|(n, _)| n.clone()).collect();
            names.sort_unstable();
            profiles.push(names.join(","));
        }
        // The three measured roles (:310-347, :348-398, :399-496): OnAfterGetRecord; no trigger; the background task, with
        // OnPageBackgroundTaskError, or without it as in the repro of al-runner issue 8434 (green on BC 29.1). A worker that
        // fails on this page is refused (`bg_drain`), so the error trigger never runs here.
        profiles.sort_unstable();
        let measured = |task: &str| profiles == ["", task, "onaftergetrecord"];
        if !(measured("onaftergetcurrrecord,onpagebackgroundtaskcompleted,onpagebackgroundtaskerror")
            || measured("onaftergetcurrrecord,onpagebackgroundtaskcompleted"))
        {
            return Ok(Some("part roles"));
        }
        let engine = self.engine();
        let program = &engine.program;
        let object = st.borrow().object as usize;
        let page = page_of(&engine, st);
        let property = |(name, value): &(String, String)| {
            let value = value.trim().to_ascii_lowercase();
            match name.to_ascii_lowercase().as_str() {
                "autosplitkey" | "editable" => value == "true",
                "extensible" => value == "false",
                "sourcetableview" => {
                    value.starts_with("sorting(") && value.ends_with("order(ascending)") && !value.contains("where")
                }
                n => PAGE_PROPERTIES.contains(&n),
            }
        };
        let properties = &program.objects[object].properties;
        if !(properties.iter().all(property) && properties.iter().any(|(n, _)| n.eq_ignore_ascii_case("autosplitkey")))
        {
            return Ok(Some("page property"));
        }
        let triggers_measured = page.triggers.iter().all(|(n, _)| PAGE_TRIGGERS.contains(&n.as_str()))
            && page.triggers.iter().any(|(n, _)| n == "onaftergetrecord");
        if !triggers_measured {
            return Ok(Some("page trigger"));
        }
        let only = |c: &ir::Control, props: &[&str], triggers: &[&str]| {
            c.owner.is_none()
                && c.ext_triggers.is_empty()
                && c.triggers.iter().all(|(n, _)| triggers.contains(&n.as_str()))
                && c.props
                    .iter()
                    .all(|(k, v)| props.contains(&k.as_str()) && plain_property(program, object, v))
        };
        let controls = page.controls.iter().all(|c| match c.kind {
            ir::ControlKind::Repeater => {
                c.binding == ir::Binding::None
                    && only(c, &["caption", "indentationcolumn"], &[])
                    && c.props
                        .iter()
                        .all(|(k, v)| k != "indentationcolumn" || matches!(v, ir::PropVal::Lit(l) if l.trim() == "0"))
            }
            ir::ControlKind::Field => match c.binding {
                ir::Binding::Field(_) | ir::Binding::Var(_) => only(c, &FIELD_PROPERTIES, &["onvalidate"]),
                // An element of an array variable (:632 `CustomValue[1]`): the expression reads it, `__sourceset` writes it.
                ir::Binding::Expr(r) => {
                    plain_property(program, object, &ir::PropVal::Routine(r))
                        && only(c, &FIELD_PROPERTIES, &["onvalidate", "__sourceset"])
                }
                _ => false,
            },
            ir::ControlKind::Action => only(c, &ACTION_PROPERTIES, &["onaction"]),
            ir::ControlKind::ActionGroup => only(c, &["caption"], &[]),
            ir::ControlKind::Part => true,
            _ => false,
        });
        if !controls {
            return Ok(Some("control"));
        }
        let table_object = table.object as usize;
        let plain_table = program.objects[table_object]
            .table
            .as_ref()
            .is_some_and(|t| t.extensions.is_empty())
            && engine.events.manual_subscribers_of(table_object).is_some();
        if !plain_table {
            return Ok(Some("table"));
        }
        Ok(None)
    }

    /// Whether part control `idx` of host `st` and its page `part` are a FactBox of the probe (:310-496): a CardPart over the
    /// host's table, linked on every primary key field to itself, the control with ApplicationArea, SubPageLink and
    /// UpdatePropagation SubPart at most; the part's page with the properties of `PART_PAGE_PROPERTIES` (Editable and Extensible
    /// false) and the triggers of `PART_TRIGGERS`; its controls fields over Rec, a variable or a field of a record variable with
    /// the properties of `PART_FIELD_PROPERTIES` and an OnDrillDown at most, cue groups with a Caption, and actions with Caption,
    /// Image and ToolTip and an OnAction; its Rec stored, not marked, filtered by the link only; no page extension, no subscriber.
    fn coded_part(&self, st: &St, idx: usize, part: &St) -> Result<bool, Signal> {
        if !self.has_source(part) || !self.unextended_unsubscribed(part) {
            return Ok(false);
        }
        let (host_rec, part_rec) = (self.page_rec(st), self.page_rec(part));
        let key = self.table(&host_rec)?.keys[0].clone();
        let same_table = {
            let (h, p) = (host_rec.borrow(), part_rec.borrow());
            h.table == p.table
                && !p.temporary
                && !p.marked_only
                && p.filters
                    .iter()
                    .all(|f| f.group == super::parts::LINK_GROUP && key.contains(&f.field))
        };
        let engine = self.engine();
        let program = &engine.program;
        let part_object = part.borrow().object as usize;
        let control = &page_of(&engine, st).controls[idx];
        let link = control.part_link.as_ref().is_some_and(|l| {
            l.provider.is_none()
                && l.view.is_none()
                && l.entries.len() == key.len()
                && l.entries
                    .iter()
                    .zip(&key)
                    .all(|(e, k)| matches!(e, ir::LinkEntry::Field(pf, hf) if pf == k && hf == k))
        });
        let part_control = control.factbox
            && control.props.iter().all(|(k, v)| match k.as_str() {
                "applicationarea" | "subpagelink" => true,
                "updatepropagation" => matches!(v, ir::PropVal::Lit(l) if unquote(l).eq_ignore_ascii_case("subpart")),
                _ => false,
            });
        let card_part = engine.object_facts[part_object].page_type == Some(ir::PageType::Other("cardpart".into()));
        let part_properties =
            program.objects[part_object]
                .properties
                .iter()
                .all(|(k, v)| match k.to_ascii_lowercase().as_str() {
                    "editable" | "extensible" => v.trim().eq_ignore_ascii_case("false"),
                    n => PART_PAGE_PROPERTIES.contains(&n),
                });
        let part_page = page_of(&engine, part);
        let part_triggers = part_page
            .triggers
            .iter()
            .all(|(n, _)| PART_TRIGGERS.contains(&n.as_str()));
        let only = |c: &ir::Control, props: &[&str], triggers: &[&str]| {
            c.owner.is_none()
                && c.ext_triggers.is_empty()
                && c.triggers.iter().all(|(n, _)| triggers.contains(&n.as_str()))
                && c.props.iter().all(|(k, v)| {
                    props.contains(&k.as_str())
                        && plain_property(program, part_object, v)
                        && (k != "showcaption"
                            || matches!(v, ir::PropVal::Lit(l) if l.trim().eq_ignore_ascii_case("false")))
                })
        };
        let part_controls = part_page.controls.iter().all(|c| match (&c.kind, &c.binding) {
            (ir::ControlKind::Field, ir::Binding::Field(_) | ir::Binding::Var(_)) => {
                only(c, &PART_FIELD_PROPERTIES, &["ondrilldown"])
            }
            // A field of a temporary record variable (the cue group's `TempKind.Name`): the expression reads it.
            (ir::ControlKind::Field, ir::Binding::Expr(r)) => {
                plain_property(program, part_object, &ir::PropVal::Routine(*r))
                    && only(c, &PART_FIELD_PROPERTIES, &["ondrilldown", "__sourceset"])
            }
            (ir::ControlKind::CueGroup, _) => only(c, &["caption"], &[]),
            (ir::ControlKind::Action, _) => only(c, &["caption", "image", "tooltip"], &["onaction"]),
            _ => false,
        });
        Ok(same_table && link && part_control && card_part && part_properties && part_triggers && part_controls)
    }

    /// Whether every subscriber to an event of table object `table` is a manual one with no instance bound now (the probe's
    /// :255-275, which its tests do not bind but :1592).
    pub(super) fn no_active_subscriber(&self, table: usize) -> bool {
        let engine = self.engine();
        let subscribers = engine.events.manual_subscribers_of(table);
        drop(engine);
        subscribers.is_some_and(|objects| objects.iter().all(|&o| !self.is_bound_object(o)))
    }

    /// The parts of host `st` as it opens. On the page with code opened as measured on an empty view, they run
    /// OnAfterGetCurrRecord on the blank record (:1407 `open=P.OPEN;W.AGCR();`) and the steps of the page with code begin;
    /// `false` for every other page and open.
    pub(super) fn coded_open_on_empty_view(&mut self, st: &St) -> Result<bool, Signal> {
        let line = {
            let s = st.borrow();
            s.cursor == Cursor::NewLine && s.draft && !s.dirty && !s.draft_stored && s.host.is_none()
        };
        if !(line
            && self.code_inserted_coded_open(st)?
            && self.code_inserted_coded_kind(st)?
            && self.view_row_count(st)? == 0)
        {
            return Ok(false);
        }
        assume(Assumption::ListNewLineCodedFactboxes);
        st.borrow_mut().coded_line = CodedLine::EmptyOpen;
        self.with_blank_landing(st, |s| s.sync_parts_at_open(st))?;
        Ok(true)
    }

    /// The test's `call` on host `st`, before it runs. After the open of `coded_open_on_empty_view`, the first SetValue of a
    /// control of the host after `New()` lets the parts read the new line first, OnAfterGetCurrRecord on the blank record (:1407
    /// `setname=W.AGCR();T.VN(Station);...`), and the validation must then land (`coded_must_have_landed`); any other call
    /// ends the steps.
    pub(super) fn coded_line_before_set_value(&mut self, st: &St, call: Option<TestCall>) -> Result<(), Signal> {
        if st.borrow().coded_line != CodedLine::EmptyOpen {
            return Ok(());
        }
        let Some(call) = call else {
            return Ok(());
        };
        let first_write = matches!(call, TestCall::Write(_))
            && matches!(
                Self::test_calls(st).as_deref(),
                Some([TestCall::Page(PageMethod::New), TestCall::Write(_)])
            )
            && {
                let s = st.borrow();
                s.cursor == Cursor::NewLine && s.draft && !s.dirty && !s.draft_stored && s.new_by_call
            };
        match call {
            _ if first_write => {
                assume(Assumption::ListNewLineCodedFactboxes);
                st.borrow_mut().coded_line = CodedLine::LineRead;
                self.with_blank_landing(st, |s| s.sync_parts(st))
            }
            TestCall::Read | TestCall::Page(PageMethod::New) => Ok(()),
            _ => {
                st.borrow_mut().coded_line = CodedLine::Off;
                Ok(())
            }
        }
    }

    /// Whether the validation of the SetValue that let the parts read the new line has landed on its row: before the landing an
    /// error (:1499) or a validation that does not land (:1553, Value before any Name) is not measured, and refused.
    pub(super) fn coded_must_have_landed(&self, st: &St) -> Result<(), Signal> {
        if st.borrow().coded_line != CodedLine::LineRead {
            return Ok(());
        }
        st.borrow_mut().forget_code_inserted_line();
        unsupported(
            "the validation of the first SetValue on a List page with code and FactBoxes, whose parts read its new line, did not \
             leave the line for the row its table field inserted as measured (not measured)",
        )
    }

    /// A SetValue on the tree of host `st` ended with `result`. When it was the first SetValue that let the parts read the new
    /// line (`CodedLine::LineRead`), or a SetValue on the landed row (`FactboxStep::CodedTyped`), and it raised an error before
    /// or in its validation, or ended without the landing or the refresh it must ask for, it is not measured, and refused.
    pub(super) fn coded_after_set_value(&self, st: &St, result: Result<Value, Signal>) -> Result<Value, Signal> {
        let (root, _) = Self::tree_root(st);
        let pending = {
            let r = root.borrow();
            r.coded_line == CodedLine::LineRead
                || matches!(
                    r.code_inserted_row.as_ref().map(|c| c.factbox),
                    Some(FactboxStep::CodedTyped { .. })
                )
        };
        match result {
            Err(Signal::Error(_)) | Ok(_) if pending => {
                root.borrow_mut().forget_code_inserted_line();
                unsupported(
                    "a SetValue on a List page with code and FactBoxes, on its new line or on the row it became, ended without the \
                     landing or the save and update the probe measures (not measured)",
                )
            }
            r => r,
        }
    }

    /// Whether control `idx` of host `st` takes a SetValue on the row its new line became (ListNewLineFactboxesCode.al:1407
    /// `setvalue`, :1576 `setvalue2`): a field control over Rec outside the primary key, with a TableRelation and an OnValidate
    /// of its own, other than the control whose SetValue landed.
    pub(super) fn coded_value_control(&self, st: &St, idx: usize) -> bool {
        let landing = match Self::test_calls(st).as_deref() {
            Some([TestCall::Page(PageMethod::New), TestCall::Write(landing), ..]) => *landing,
            _ => return false,
        };
        let engine = self.engine();
        let Some(control) = page_of(&engine, st).controls.get(idx) else {
            return false;
        };
        let ir::Binding::Field(f) = control.binding else {
            return false;
        };
        idx != landing
            && control.trigger("onvalidate").is_some()
            && control.props.iter().any(|(k, _)| k == "tablerelation")
            && self.table(&self.page_rec(st)).is_ok_and(|t| !t.keys[0].contains(&f))
    }

    /// The refresh of the page with code after its new line landed, or after a later SetValue saved the row
    /// (`code_inserted_refresh`): the row once more first in a secondary key (:1407 `P.AGR(Station/);P.AGR(Station/);`, :1445
    /// once in the primary key), then as on the FactBox host. The page's and the parts' triggers leave the rows of the page's
    /// table, Rec as stored and Rec's view as they were; otherwise not measured, and refused.
    pub(super) fn coded_refresh(&mut self, st: &St, without_curr: bool, without_parts: bool) -> Result<(), Signal> {
        let rows = self.coded_rows(st)?;
        let view = self.page_rec(st).borrow().clone();
        let primary_key = {
            let rec = self.page_rec(st);
            let r = rec.borrow();
            r.key == 0 && r.adhoc.is_none()
        };
        if !primary_key {
            self.populate_current(st)?;
        }
        self.reread_view_from_row(st, without_curr, without_parts, false)?;
        let rec = self.page_rec(st);
        let stored = self.stored_row(&rec)?;
        let rows_kept = match (rows, self.coded_rows(st)?) {
            (Some(a), Some(b)) => a.len() == b.len() && a.iter().zip(&b).all(|(x, y)| Self::same_values(x, y)),
            _ => false,
        };
        let view_kept = {
            let r = rec.borrow();
            crate::records::same_filters(&r.filters, &view.filters)
                && r.filter_group == view.filter_group
                && r.key == view.key
                && r.adhoc == view.adhoc
                && r.ascending == view.ascending
                && r.field_descending == view.field_descending
                && r.marked_only == view.marked_only
        };
        let kept = rows_kept && view_kept && stored.is_some_and(|s| Self::same_values(&rec.borrow().fields, &s));
        if !kept {
            st.borrow_mut().forget_code_inserted_line();
            return unsupported(
                "a trigger of a List page with code and FactBoxes, or of its parts, wrote or changed Rec when the refresh read the \
                 row its new line became (not measured)",
            );
        }
        let steps = match st.borrow().code_inserted_row.as_ref().map(|r| r.factbox) {
            Some(FactboxStep::CodedTyped { steps }) => steps + 1,
            _ => 0,
        };
        if let Some(row) = st.borrow_mut().code_inserted_row.as_mut() {
            row.factbox = FactboxStep::Coded { steps };
        }
        Ok(())
    }

    /// Every stored row of the table of the page's record, in the primary key: what the code between the save and the update
    /// must leave as it is on the page with code (`SavedView::rows`). `None` past 1000 rows.
    pub(super) fn coded_rows(&mut self, st: &St) -> Result<Option<Vec<Vec<Value>>>, Signal> {
        let probe = Rc::new(RefCell::new(self.page_rec(st).borrow().clone()));
        {
            let mut p = probe.borrow_mut();
            p.filters.clear();
            p.marks.clear();
            p.marked_only = false;
            p.key = 0;
            p.ascending = true;
            p.adhoc = None;
            p.field_descending = 0;
        }
        let rows = self.view_rows(&probe, 1001)?;
        Ok((rows.len() <= 1000).then_some(rows))
    }
}

/// Whether property value `v` of a control of `object` is a literal, or an expression that reads only (`plain_expr`).
fn plain_property(program: &ir::Program, object: usize, v: &ir::PropVal) -> bool {
    match v {
        ir::PropVal::Lit(_) => true,
        ir::PropVal::Routine(r) => {
            matches!(code_stmts(program, object, *r).as_slice(), [ir::Stmt::Exit(Some(e))] if plain_expr(e))
        }
        ir::PropVal::Ext(..) => false,
    }
}

/// Whether `e` reads variables, fields, constants and array elements at constant indexes, with operators, and calls nothing
/// (the probe's `Rec.Style`, `ValueEditable`, `Rec."Has Quantity"`, `CustomHeader[1]`, `(Rec."Kind Type" = Rec."Kind Type"::Option)`,
/// `Rec."Kind ID" <> 0`).
fn plain_expr(e: &ir::Expr) -> bool {
    match e {
        ir::Expr::Const(_) => true,
        ir::Expr::Load(p) => plain_place(p),
        ir::Expr::Unary(_, a) | ir::Expr::Convert(_, a) => plain_expr(a),
        ir::Expr::Binary(_, a, b) => plain_expr(a) && plain_expr(b),
        _ => false,
    }
}

fn plain_place(p: &ir::Place) -> bool {
    match p {
        ir::Place::Local(_) | ir::Place::Global(_) => true,
        ir::Place::Field(base, _) => plain_place(base),
        ir::Place::Index(base, at) => plain_place(base) && at.iter().all(|e| matches!(e, ir::Expr::Const(_))),
        _ => false,
    }
}
