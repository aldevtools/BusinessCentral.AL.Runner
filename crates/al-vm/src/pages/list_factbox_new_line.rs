//! A top-level List page's CardPart FactBox, linked to the list's key, around the list's new line (al-runner issue 8188).
//!
//! Corpus pull request 1426 (merged, not pinned yet), tests/al-language/pages/parts/ListHostFactBoxNewLine.al, measured on
//! Business Central 29.1 and green on every supported major in its CI, runs only through the assumption `list-factbox-new-line`.
//! On its List page (:28-62) with one CardPart FactBox (:64-90), BC:
//! - opens an empty view with the part's OnAfterGetCurrRecord on the blank record (:162 `open=F.AGCR();`);
//! - runs nothing at `New()` below the view's one row: the part keeps showing that row (:181 `new=`, `partnew=Alpha`);
//! - on the SetValue of the key of the new line runs the part's OnAfterGetCurrRecord on the blank record, twice after such a
//!   `New()`, the first for the line the part had not read yet (:162 `setno=F.AGCR();`, :181 `setno=F.AGCR();F.AGCR();`);
//! - on `Next()` back from the row before onto the row the line saved runs no OnAfterGetRecord of the host, which holds that
//!   row (:183 `nextlog=F.AGR(B);F.AGCR(B);`);
//! - on the `Next()` after it, onto the new line, runs the part's OnAfterGetCurrRecord on the blank record (:183
//!   `next2log=F.AGCR();`).
//!
//! Corpus pull request 1425 (merged, not pinned yet), tests/al-language/pages/currpage/ListNewLineCodeInsertUpdate.al, shows the same at the
//! open, `New()` and these moves on another host (two CardPart FactBoxes on a two-field key, AutoSplitKey, a control's
//! OnValidate; :713-740 `open=F.AGCR();`, `newparts=A/`, `nextlog=F.AGR(X);F.AGCR(X);`, `nextlog=T.M(X/V);F.AGCR();`), which
//! this module does not admit.
//!
//! Each step runs only on the probe's page shapes (`factbox_line_pages`) and after the probe's earlier steps
//! (`FactboxLine`, `steps_are`); everything else keeps the path it had, and a SetValue that raises an error ends the steps. After
//! a `New()` that kept the part, the host's reads and property calls, a key's SetValue the probe does not make and Close keep
//! main's answers; a call that reads or moves the part otherwise, an action, another SetValue or a move of the host is refused
//! (`factbox_line_call`).

use super::pinned_shape::{PinnedCode, TestCall, code_stmts, rec_text_log_call};
use super::*;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::page_op::{PageMethod, PageOp, PageRecv};

/// The last measured step of the host's FactBox around the new line (`PageState::factbox_line`). Each step requires the one
/// before it; the open, every close and a SetValue that raises an error (`factbox_line_after_set_value`) start again from `Off`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum FactboxLine {
    #[default]
    Off,
    /// The host opened on an empty view and the part ran OnAfterGetCurrRecord on the blank record (:163).
    EmptyOpen,
    /// `New()` below the view's one row left the part on that row (:187).
    Held,
    /// The key's SetValue after `Held` let the part read the new line first (:190, the first `F.AGCR();`).
    HeldRead,
    /// The key's SetValue on the new line ran the part's OnAfterGetCurrRecord on the blank record (:166, :190).
    Keyed,
    /// `Next()` from the row before onto the row the line saved read that row from the rows the host holds (:199).
    Back,
}

/// The calls of the RowBefore test up to each step, reads left out (ListHostFactBoxNewLine.al:184-202): `New()`, the key's
/// SetValue, the SetValue of another field, `Previous()`, `Next()` and `Next()`.
enum Seq {
    New,
    Key,
    Other,
    Previous,
    Next,
}

impl Session {
    /// The part control and the part page of host `host` when both are the probe's pages, every fact required
    /// (ListHostFactBoxNewLine.al:12-112):
    /// - the host: a top-level List page (:30) the test opened with OpenEdit (:163, :184), over a stored table in its primary
    ///   key with no filter; its only trigger OnAfterGetRecord (:55-58), its controls a repeater without a property (:39-40),
    ///   its fields of Rec without a trigger and with no property but ApplicationArea (:41-42) and one part in its FactBoxes
    ///   (:45-52), no other page property, no page extension, no subscriber;
    /// - the part control: no property but ApplicationArea, a SubPageLink of one `field(..)` entry that links the one field of
    ///   the primary key to itself (:50), no SubPageView, no Provider;
    /// - the part: a CardPart over the host's table (:66-67), shown, whose only triggers are OnAfterGetRecord and
    ///   OnAfterGetCurrRecord (:78-86), its controls fields of Rec without a trigger and with no property but ApplicationArea
    ///   (:74), no other page property, no page extension, no subscriber; its Rec stored, not marked, filtered by the link only;
    /// - each trigger of both pages only calls a log procedure of a codeunit variable with a text of constants and fields of Rec
    ///   (`rec_text_log_call`; :57, :80, :85, the procedure :99-102);
    /// - the table: no property but DataClassification (:14), its own fields without a property (:18-19), one Code field in its
    ///   primary key (:18, :24), no trigger, no procedure, no table extension, no subscriber.
    pub(super) fn factbox_line_pages(&mut self, host: &St) -> Result<Option<(usize, St)>, Signal> {
        let (open_edit, parts, hidden) = {
            let h = host.borrow();
            (h.open && h.test_open_edit(), h.parts.clone(), h.hidden_parts.is_empty())
        };
        let [(idx, part)] = parts.as_slice() else {
            return Ok(None);
        };
        if !(open_edit && hidden && self.is_list_page(host) && self.has_source(host) && self.has_source(part)) {
            return Ok(None);
        }
        let host_code = PinnedCode {
            triggers: &["onaftergetrecord"],
            control_triggers: &[],
            properties: &[],
            action_props: None,
            part_props: Some(&[]),
            table_triggers: &[],
        };
        let part_code = PinnedCode {
            triggers: &["onaftergetrecord", "onaftergetcurrrecord"],
            control_triggers: &[],
            properties: &[],
            action_props: None,
            part_props: None,
            table_triggers: &[],
        };
        if !(self.page_code_is(host, &host_code)? && self.page_code_is(part, &part_code)?) {
            return Ok(None);
        }
        let (host_rec, part_rec) = (self.page_rec(host), self.page_rec(part));
        let table = self.table(&host_rec)?;
        let &[key] = table.keys[0].as_slice() else {
            return Ok(None);
        };
        let same_table = {
            let (h, p) = (host_rec.borrow(), part_rec.borrow());
            h.table == p.table
                && !h.temporary
                && !p.temporary
                && !p.marked_only
                && p.filters
                    .iter()
                    .all(|f| f.group == super::parts::LINK_GROUP && f.field == key)
        };
        if !(same_table && self.rec_view_is(host, 0)) {
            return Ok(None);
        }
        let engine = self.engine();
        let program = &engine.program;
        let control = &page_of(&engine, host).controls[*idx];
        let link = control.part_link.as_ref().is_some_and(|l| {
            l.provider.is_none()
                && l.view.is_none()
                && matches!(l.entries.as_slice(), [ir::LinkEntry::Field(pf, hf)] if *pf == key && *hf == key)
        });
        let part_control = control.factbox
            && control
                .props
                .iter()
                .all(|(k, _)| k == "applicationarea" || k == "subpagelink")
            && engine.object_facts[part.borrow().object as usize].page_type
                == Some(ir::PageType::Other("cardpart".into()));
        let logs_only = |p: &St| {
            let object = p.borrow().object as usize;
            page_of(&engine, p).triggers.iter().all(|(_, r)| {
                let stmts = code_stmts(program, object, *r);
                !stmts.is_empty()
                    && stmts
                        .iter()
                        .all(|s| matches!(s, ir::Stmt::Expr(e) if rec_text_log_call(program, object, e)))
            })
        };
        // The field controls carry no property but ApplicationArea (:41-42, :74), the repeater none (:39-40); the one part
        // control is `part_control`.
        let plain_controls = |p: &St, part_at: Option<usize>| {
            page_of(&engine, p)
                .controls
                .iter()
                .enumerate()
                .all(|(i, c)| match c.kind {
                    ir::ControlKind::Field => c.props.iter().all(|(k, _)| k == "applicationarea"),
                    ir::ControlKind::Repeater => c.props.is_empty(),
                    ir::ControlKind::Part => part_at == Some(i),
                    _ => false,
                })
        };
        // The table declares no property but DataClassification (:14) and its own fields none (:18-19).
        let table_object = &program.objects[table.object as usize];
        let plain_table = table_object
            .properties
            .iter()
            .all(|(k, _)| k.eq_ignore_ascii_case("dataclassification"))
            && table_object.table.as_ref().is_some_and(|t| {
                t.fields
                    .iter()
                    .filter(|f| f.declared_in == Some(table.object))
                    .all(|f| f.properties.is_empty())
            });
        let pinned = link
            && part_control
            && plain_controls(host, Some(*idx))
            && plain_controls(part, None)
            && plain_table
            && matches!(table.fields[key].ty, RtType::Code(_))
            && logs_only(host)
            && logs_only(part);
        Ok(pinned.then(|| (*idx, part.clone())))
    }

    /// Whether the calls the test made on host `host` and its part since the open, reads of a control left out, are `want`,
    /// each answered Yes or nothing without an error, the running one last, and the store is as the last call left it (the test
    /// wrote nothing between two calls). `key` is the control of the key field; `Seq::Other` writes a control of a field outside
    /// the key (`control_outside_key`).
    fn steps_are(&self, host: &St, key: usize, want: &[Seq]) -> bool {
        let h = host.borrow();
        let unchanged = h.tree_calls_mark.is_none_or(|m| m == self.write_mark());
        if h.tree_calls_unknown || !unchanged {
            return false;
        }
        let calls: Vec<_> = h.tree_calls.iter().filter(|c| c.call != TestCall::Read).collect();
        calls.len() == want.len()
            && calls.iter().zip(want).all(|(c, w)| {
                let call = match w {
                    Seq::New => c.call == TestCall::Page(PageMethod::New),
                    Seq::Key => c.call == TestCall::Write(key),
                    Seq::Other => matches!(c.call, TestCall::Write(i) if self.control_outside_key(host, i)),
                    Seq::Previous => c.call == TestCall::Page(PageMethod::Previous),
                    Seq::Next => c.call == TestCall::Page(PageMethod::Next),
                };
                call && c.part.is_none() && !c.no && !c.failed
            })
    }

    /// The control of host `host` bound to the key field of its table, when there is exactly one.
    fn only_key_control(&self, host: &St) -> Result<Option<usize>, Signal> {
        let key = self.table(&self.page_rec(host))?.keys[0].clone();
        let engine = self.engine();
        let mut found = page_of(&engine, host)
            .controls
            .iter()
            .enumerate()
            .filter(|(_, c)| matches!(c.binding, ir::Binding::Field(f) if key == [f]))
            .map(|(i, _)| i);
        Ok(match (found.next(), found.next()) {
            (Some(i), None) => Some(i),
            _ => None,
        })
    }

    /// Whether host `host` stands on the new line nobody typed on and stored: the line a `New()` started, or the one the open
    /// or a move landed on.
    fn on_untouched_line(host: &St) -> bool {
        let h = host.borrow();
        h.cursor == Cursor::NewLine && h.draft && !h.dirty && !h.draft_stored
    }

    /// Runs `read` (a read of the parts of host `host`) with `PageState::factbox_blank_landing` on.
    pub(super) fn with_blank_landing(
        &mut self,
        host: &St,
        read: impl FnOnce(&mut Self) -> Result<(), Signal>,
    ) -> Result<(), Signal> {
        host.borrow_mut().factbox_blank_landing = true;
        let r = read(self);
        host.borrow_mut().factbox_blank_landing = false;
        r
    }

    /// Whether part `st`, which found no row, lands inside a measured step of its host (`with_blank_landing`): it then runs
    /// OnAfterGetCurrRecord on the blank record (`card_part_blank_on_new_host`).
    pub(super) fn factbox_blank_landing(&self, st: &St) -> bool {
        let host = st.borrow().host.as_ref().and_then(|(h, _)| h.upgrade());
        host.is_some_and(|h| h.borrow().factbox_blank_landing)
    }

    /// The parts of host `st` read as the host opens; on an empty view of the probe's pages the part runs OnAfterGetCurrRecord
    /// on the blank record (:162 `open=F.AGCR();`, `part=`).
    pub(super) fn sync_parts_at_list_open(&mut self, st: &St) -> Result<(), Signal> {
        let empty =
            Self::on_untouched_line(st) && self.factbox_line_pages(st)?.is_some() && self.view_row_count(st)? == 0;
        if !empty {
            if self.coded_open_on_empty_view(st)? {
                return Ok(());
            }
            return self.sync_parts_at_open(st);
        }
        assume(Assumption::ListFactboxNewLine);
        self.with_blank_landing(st, |s| s.sync_parts_at_open(st))?;
        st.borrow_mut().factbox_line = FactboxLine::EmptyOpen;
        Ok(())
    }

    /// The parts of host `st` after its `New()` (`new_from_row`: from a row). On the probe's pages, from the view's one row,
    /// as the first call but reads, the part keeps that row and reads nothing (:181 `new=`, `partnew=Alpha`).
    pub(super) fn sync_parts_after_new(&mut self, st: &St, new_from_row: bool) -> Result<(), Signal> {
        if new_from_row && self.new_holds_factbox(st)? {
            assume(Assumption::ListFactboxNewLine);
            st.borrow_mut().factbox_line = FactboxLine::Held;
            return Ok(());
        }
        // The parts a host's New() links afresh run no OnNewRecord (see `land_on_part_open_line`).
        let before = self.host_new_running.replace(new_from_row);
        let r = self.sync_parts(st);
        self.host_new_running = before;
        r
    }

    /// `New()` on host `st` as the probe makes it (:187): the probe's pages, the first call but reads, from the view's one row,
    /// whose key is not blank (`after_new`), onto an untouched new line, the part on that row.
    fn new_holds_factbox(&mut self, st: &St) -> Result<bool, Signal> {
        let Some((_, part)) = self.factbox_line_pages(st)? else {
            return Ok(false);
        };
        let Some(key) = self.only_key_control(st)? else {
            return Ok(false);
        };
        let anchor = st.borrow().after_new.clone();
        let table = self.table(&self.page_rec(st))?;
        let anchor_keyed = anchor.is_some_and(|a| {
            let k = table.primary_key(&a);
            !Self::same_values(&k, &table.primary_key(&self.initial_fields(table.object as usize)))
        });
        let part_on_row = {
            let p = part.borrow();
            p.cursor == Cursor::Row && !p.draft && !p.dirty
        };
        Ok(anchor_keyed
            && part_on_row
            && st.borrow().new_by_call
            && st.borrow().factbox_line == FactboxLine::Off
            && Self::on_untouched_line(st)
            && self.steps_are(st, key, &[Seq::New])
            && self.view_row_count(st)? == 1)
    }

    /// A TestPage call on the tree of a host whose `New()` kept its part on the row before (`FactboxLine::Held`), before it runs:
    /// - a read of a control of the part goes on (:189 `partnew=Alpha`);
    /// - a call on the host that reads a value or a property and runs no page code (`host_read_call`) goes on: its answer does
    ///   not depend on the part, so it is main's;
    /// - the key's SetValue of a new key as the probe makes it (`held_key_measured`) first lets the part read the new line,
    ///   OnAfterGetCurrRecord on the blank record (:190 `SetValue('B')`, the first `F.AGCR();`), and goes on;
    /// - the key's SetValue of another value, or after another call, lets the part read the new line as main did at `New()`,
    ///   with no trigger (`release_held_part`), and goes on as on main; Close goes on (the part runs nothing there, as on main);
    /// - after the test wrote to the store since the last call, main's read can no longer be made: the key's SetValue is
    ///   refused. The part's other reads, its moves, actions, other SetValues and moves of the host are refused at any time.
    ///
    /// On a host in the step before `Next()` back onto the saved row (`next_back_held`), the host holds that row.
    pub(super) fn factbox_line_call(
        &mut self,
        method: PageOp,
        st: &St,
        target: Option<&ControlTarget>,
        args: &[Value],
    ) -> Result<(), Signal> {
        if method.recv() == PageRecv::Page {
            return Ok(());
        }
        let (root, _) = Self::tree_root(st);
        if root.borrow().factbox_line != FactboxLine::Held {
            if method == PageOp::TestPageNext && Rc::ptr_eq(st, &root) {
                return self.next_back_held(&root);
            }
            return Ok(());
        }
        let on_part = !Rc::ptr_eq(st, &root);
        let unchanged = {
            let r = root.borrow();
            !r.tree_calls_unknown && r.tree_calls_mark == Some(self.write_mark())
        };
        let key = self.only_key_control(&root)?;
        match (method, target) {
            (PageOp::TestFieldValue, Some(ControlTarget::Control(_))) if on_part && args.is_empty() => Ok(()),
            _ if !on_part && Self::host_read_call(method, args) => Ok(()),
            (PageOp::TestPageClose, None) if !on_part => {
                root.borrow_mut().factbox_line = FactboxLine::Off;
                Ok(())
            }
            (PageOp::TestFieldSetValue, Some(ControlTarget::Control(i)))
                if !on_part && unchanged && key == Some(*i) =>
            {
                if !self.held_key_measured(&root, *i, args)? {
                    return self.release_held_part(&root);
                }
                assume(Assumption::ListFactboxNewLine);
                root.borrow_mut().factbox_line = FactboxLine::HeldRead;
                let before = self.host_new_running.replace(true);
                let r = self.with_blank_landing(&root, |s| s.sync_parts(&root));
                self.host_new_running = before;
                Self::factbox_line_after_set_value(method, &root, &r);
                r
            }
            _ => unsupported(format!(
                "{method} after New() on a List page whose CardPart FactBox still shows the row before (only a read of the part, \
                 the host's reads, the SetValue of the key and Close are measured or keep main's read)"
            )),
        }
    }

    /// After a SetValue (`method`) on the tree of host `st`, before it or with it (`result`): one that raised an error ends the
    /// measured steps, `factbox_line` goes back to `Off` and the later calls take main's path.
    pub(super) fn factbox_line_after_set_value<T>(method: PageOp, st: &St, result: &Result<T, Signal>) {
        if method == PageOp::TestFieldSetValue && result.is_err() {
            let (root, _) = Self::tree_root(st);
            root.borrow_mut().factbox_line = FactboxLine::Off;
        }
    }

    /// Whether `method` with `args` on a host or one of its controls only reads: a value (`Value()` without an argument, the
    /// `As*` methods, `GetField`, `AssertEquals`) or a property (Caption, Editable, Enabled, Visible, HideValue, ShowMandatory,
    /// the options of a control, the validation errors, IsExpanded). Such a call reads nothing of a part.
    fn host_read_call(method: PageOp, args: &[Value]) -> bool {
        match method.method() {
            PageMethod::Value => args.is_empty(),
            m => {
                Self::field_read(m)
                    || matches!(
                        m,
                        PageMethod::AssertEquals
                            | PageMethod::Caption
                            | PageMethod::Editable
                            | PageMethod::Enabled
                            | PageMethod::Visible
                            | PageMethod::HideValue
                            | PageMethod::ShowMandatory
                            | PageMethod::OptionCount
                            | PageMethod::GetOption
                            | PageMethod::ValidationErrorCount
                            | PageMethod::GetValidationError
                            | PageMethod::IsExpanded
                    )
            }
        }
    }

    /// The part of host `st`, held on the row before since `New()`, reads the new line as main reads it at `New()`: no trigger runs
    /// (the parts a host's New() links afresh run no OnNewRecord, see `land_on_part_open_line`). The host leaves
    /// `FactboxLine::Held`.
    fn release_held_part(&mut self, st: &St) -> Result<(), Signal> {
        st.borrow_mut().factbox_line = FactboxLine::Off;
        let before = self.host_new_running.replace(true);
        let r = self.sync_parts(st);
        self.host_new_running = before;
        r
    }

    /// Whether the key's SetValue (control `key`, `args`) on host `host` is the one the probe makes after `New()` (:190): the
    /// first call but reads since the open, on the untouched new line, of one Text of at most as many characters as the Code key
    /// field holds, which, converted to that field, is not the blank value and is the key of no stored row (`'B'` into a
    /// Code[10] field, :18, on a view of row `A`).
    fn held_key_measured(&mut self, host: &St, key: usize, args: &[Value]) -> Result<bool, Signal> {
        let rec = self.page_rec(host);
        let table = self.table(&rec)?;
        let (field, len) = match table.keys[0].as_slice() {
            [field] => match table.fields[*field].ty {
                RtType::Code(Some(len)) => (*field, len as usize),
                _ => return Ok(false),
            },
            _ => return Ok(false),
        };
        let [Value::Text(text)] = args else {
            return Ok(false);
        };
        if !(Self::on_untouched_line(host) && self.steps_are(host, key, &[Seq::New]) && text.chars().count() <= len) {
            return Ok(false);
        }
        let Ok(typed) = ops::convert(Value::Text(text.clone()), &table.fields[field].ty) else {
            return Ok(false);
        };
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        probe.borrow_mut().fields[field] = typed;
        self.key_is_new(&probe)
    }

    /// The parts of host `st` after the SetValue of its control `idx` (`before`: the row before it). On the probe's pages, the
    /// SetValue of the key of the new line, as the first call but reads after the open of an empty view (:166) or after the
    /// `New()` that kept the part (:190), of a key no row holds: the part runs OnAfterGetCurrRecord on the blank record
    /// (`setno=F.AGCR();`).
    pub(super) fn resync_parts_after_set_value(
        &mut self,
        st: &St,
        idx: usize,
        before: Option<&[Value]>,
    ) -> Result<(), Signal> {
        let line = st.borrow().factbox_line;
        let keyed = matches!(line, FactboxLine::EmptyOpen | FactboxLine::HeldRead)
            && self.factbox_line_pages(st)?.is_some()
            && self.only_key_control(st)? == Some(idx)
            && self.steps_are(
                st,
                idx,
                if line == FactboxLine::EmptyOpen {
                    &[Seq::Key]
                } else {
                    &[Seq::New, Seq::Key]
                },
            )
            && {
                let s = st.borrow();
                s.cursor == Cursor::NewLine && s.draft && !s.draft_stored
            }
            && self.key_is_new(&self.page_rec(st))?;
        if !keyed {
            if matches!(line, FactboxLine::EmptyOpen | FactboxLine::HeldRead) {
                st.borrow_mut().factbox_line = FactboxLine::Off;
            }
            return self.resync_parts_linked_by(st, idx, before);
        }
        assume(Assumption::ListFactboxNewLine);
        self.with_blank_landing(st, |s| s.resync_parts_linked_by(st, idx, before))?;
        st.borrow_mut().factbox_line = FactboxLine::Keyed;
        Ok(())
    }

    /// Whether record `rec` holds a key that is not the blank one and that no stored row holds (:166, :190 `'B'` on a view
    /// without it): the host's Rec after the key's SetValue, or the candidate key before it (`held_key_measured`).
    fn key_is_new(&mut self, rec: &Rec) -> Result<bool, Signal> {
        let table = self.table(rec)?;
        let blank = table.primary_key(&self.initial_fields(table.object as usize));
        let typed = table.primary_key(&rec.borrow().fields);
        Ok(!Self::same_values(&typed, &blank) && self.stored_row(rec)?.is_none())
    }

    /// `Next()` of host `st` from the row before onto the row the new line saved, as the probe makes it after `Keyed`: the SetValue
    /// of another field saved the line (:193), `Previous()` moved to the view's first row (:196); the view holds those two rows.
    /// The host holds the saved row as the client does: the move reads no row and runs no OnAfterGetRecord (:183
    /// `nextlog=F.AGR(B);F.AGCR(B);`, no `P.AGR(B)`), and the part reads the row as before.
    fn next_back_held(&mut self, st: &St) -> Result<(), Signal> {
        if st.borrow().factbox_line != FactboxLine::Keyed {
            return Ok(());
        }
        st.borrow_mut().factbox_line = FactboxLine::Off;
        let Some(key) = self.only_key_control(st)? else {
            return Ok(());
        };
        let on_first = {
            let s = st.borrow();
            s.cursor == Cursor::Row && !s.draft && !s.dirty && s.window_from_start && s.window.len() == 1
        };
        if !(on_first
            && self.factbox_line_pages(st)?.is_some()
            && self.steps_are(st, key, &[Seq::New, Seq::Key, Seq::Other, Seq::Previous])
            && self.view_row_count(st)? == 2)
        {
            return Ok(());
        }
        let rec = self.page_rec(st);
        let here = self.row_key(&rec)?;
        let held_here = st
            .borrow()
            .window
            .first()
            .is_some_and(|w| Self::same_key(&w.key, &here));
        let Some(saved) = self.neighbor_row(&rec, 1)?.filter(|_| held_here) else {
            return Ok(());
        };
        assume(Assumption::ListFactboxNewLine);
        let table = self.table(&rec)?;
        st.borrow_mut().window.push(crate::pages::WindowRow {
            key: table.primary_key(&saved),
            stored: saved.clone(),
            shown: saved,
        });
        st.borrow_mut().factbox_line = FactboxLine::Back;
        Ok(())
    }

    /// The linked parts of host `st` after its Next (`name`) or Previous. On the probe's pages, the `Next()` after `Back` onto the
    /// new line: the part runs OnAfterGetCurrRecord on the blank record (:183 `next2=T/`, `next2log=F.AGCR();`).
    pub(super) fn sync_parts_after_host_step(&mut self, st: &St, name: PageMethod) -> Result<(), Signal> {
        let onto_line = st.borrow().factbox_line == FactboxLine::Back
            && name == PageMethod::Next
            && Self::on_untouched_line(st)
            && st.borrow().placeholder_line
            && self.factbox_line_pages(st)?.is_some()
            && self.only_key_control(st)?.is_some_and(|key| {
                self.steps_are(
                    st,
                    key,
                    &[Seq::New, Seq::Key, Seq::Other, Seq::Previous, Seq::Next, Seq::Next],
                )
            });
        if !onto_line {
            return self.sync_parts_after_move(st, name == PageMethod::Previous);
        }
        st.borrow_mut().factbox_line = FactboxLine::Off;
        assume(Assumption::ListFactboxNewLine);
        self.with_blank_landing(st, |s| s.sync_parts_after_move(st, false))
    }
}
