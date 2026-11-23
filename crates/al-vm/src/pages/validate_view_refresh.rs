//! A control's OnValidate that calls `CurrPage.Update` on the measured page shapes, with or without a change of the page's
//! view (al-runner issues 8698 and 8717), measured by corpus pull request 1518 (merged as 691b244d, not pinned yet),
//! tests/al-language/pages/currpage/ListValidateFilterRefresh.al. On BC the refresh reads the row again from the table, so a
//! typed value is gone. On a List with OnAfterGetRecord (`reread_view_from_row`) it then reads the rows after the row in
//! the view and the rows before it back to the first; without OnAfterGetCurrRecord not the row again, with it the row twice
//! and OnAfterGetCurrRecord. On a List with OnAfterGetCurrRecord only and on a Card it reads the row as main's refresh does.
//! On a List without OnAfterGetCurrRecord a SetValue, the action and Close read the row first when the page's last read was
//! of another row (`validate_view_read_row`). Main kept the typed value, and on a List the rows the client held.
//!
//! On a tree (ShowAsTree; al-runner issue 8833, corpus pull request 1539, tests/al-language/pages/currpage/
//! TreeValidateFilterRefresh.al, open) Update(false) saves the row as Update(true) does, and the refresh reads as on the List
//! of the same row triggers, over the tree's hidden rows too; then the tree shows the rows of the new view
//! (`validate_view_tree_refresh`). Main read only the typed row and saved nothing.
//!
//! Only on the probe's pages, line for line (`validate_view_cells.rs`; the repros of 8698, 8717 and 8833 read as three of
//! them), as the test called them in the cells: assumption `list-validate-view-refresh` on a List without
//! OnAfterGetCurrRecord, `tree-validate-view-refresh` on a tree, `validate-update-false-rereads-row` on the other pages. Any
//! other page, or calls the cells do not make before the rule changed anything, keep main's path; after it did they refuse.

use super::open::TREE_GROUP;
use super::pinned_body::{Callees, Names, callees_with_calls_are, page_is, table_is};
use super::pinned_shape::TestCall;
use super::validate_view_cells::{CellPage, Change, Ctl, Step, Tail, Upd};
use super::*;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::filter::{FieldFilter, FilterExpr};
use al_builtins::page_op::PageMethod;
use al_builtins::record_fields::Fields;
use al_builtins::record_op::RecordOp;

use Change::{Drop, Keep, KeepText, Reset, Same};
use Step::{Action, Fail, Set};

/// The refusal of a call the cells do not make, after the rule ran.
const UNMEASURED: &str = "a call on a page of ListValidateFilterRefresh.al after the page read its row or the view again as the cells measure it, which the cells do not make (not measured)";
const TREE_UNMEASURED: &str = "a call on a tree page of TreeValidateFilterRefresh.al after the page read its row or the view again as the cells measure it, which the cells do not make (not measured)";

/// The refusal of a call the cells of `page` do not make, after the rule ran.
fn unmeasured<T>(page: Option<CellPage>) -> Result<T, Signal> {
    unsupported(if page.is_some_and(CellPage::tree) {
        TREE_UNMEASURED
    } else {
        UNMEASURED
    })
}

/// The SetValue or action that runs now, or the last one until the next call takes it.
#[derive(Debug, Clone, Default)]
pub(crate) struct Pending {
    /// The control typed into; `None` for the action.
    ctl: Option<Ctl>,
    /// Rec's filters and primary key when the call began.
    filters: Vec<FieldFilter>,
    key: Vec<Value>,
    /// The last `CurrPage.Update(save)` in it.
    update: Option<bool>,
    /// Its place in the page's calls (`PageState::tree_calls`).
    call: usize,
}

/// What the rule noted on a page since it opened (`PageState::validate_view`).
#[derive(Debug, Clone, Default)]
pub(crate) struct ValidateView {
    /// The page is none of the cells', or the test left the cells' calls before the rule changed anything.
    off: bool,
    /// The page, once checked at the first call after the open.
    page: Option<CellPage>,
    /// The steps so far.
    steps: Vec<Step>,
    /// `First` came after the cell's last step, and then `Next`.
    walked: bool,
    nexted: bool,
    pending: Option<Pending>,
    /// The primary key of the row the page's OnAfterGetRecord last ran for.
    last_read: Option<Vec<Value>>,
    /// The rule changed what a call did: every later call must be one the cells make.
    ran: bool,
}

impl Session {
    /// The cells' page `st` is, every fact by equality: a top-level List or Card opened by the test with OpenEdit, no part, Rec
    /// over the database table, and its code, its table and the procedures they call line for line as the probe's (`page_is`,
    /// `table_is`, `callees_with_calls_are`; nothing subscribes to their events).
    fn validate_view_page(&mut self, st: &St) -> Option<CellPage> {
        {
            let s = st.borrow();
            if !s.test_open_edit() || !s.parts.is_empty() || !s.hidden_parts.is_empty() || !s.open {
                return None;
            }
        }
        if !(self.is_list_page(st) || self.is_card(st)) || !self.has_source(st) || self.page_rec(st).borrow().temporary
        {
            return None;
        }
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let table = self.page_rec(st).borrow().table as usize;
        CellPage::ALL.into_iter().find(|page| {
            let (want_page, want_table, want_callees) = page.code();
            let (mut callees, mut names) = (Callees::new(), Names::default());
            page_is(&engine, object, want_page, &mut callees, &mut names)
                && table_is(&engine, table, want_table, &mut callees, &mut names)
                && callees_with_calls_are(&engine, &callees, want_callees, &mut names)
        })
    }

    /// The test calls `call` on `st`, on `target` when it is a control's call (`test_page_op_held`, `test_control_op_held`).
    /// A control's `Value()` read and the page's filter pass. On the cells' page any other call joins the steps
    /// (`validate_view_step`), the other read-only calls as `Step::Reads` where the cells measure them; a call the cells do
    /// not make turns the rule off, or refuses once the rule ran.
    pub(super) fn validate_view_call(
        &mut self,
        st: &St,
        call: Option<TestCall>,
        target: Option<&ControlTarget>,
        args: &[Value],
    ) -> Result<(), Signal> {
        let Some(call) = call else {
            return Ok(());
        };
        if matches!(
            call,
            TestCall::Read | TestCall::Page(PageMethod::Filter) | TestCall::Other(PageMethod::GetFilter)
        ) || st.borrow().validate_view.off
        {
            return Ok(());
        }
        let mut v = std::mem::take(&mut st.borrow_mut().validate_view);
        let r = self.validate_view_step(st, &mut v, (call, target, args));
        let measured = matches!(r, Ok(true));
        v.off = !measured && !v.ran;
        let refuse = !measured && v.ran;
        st.borrow_mut().validate_view = v;
        r?;
        if refuse {
            return unmeasured(st.borrow().validate_view.page);
        }
        Ok(())
    }

    /// `validate_view_call` on the taken state: `false` when the call is not one the cells make. A SetValue or the action reads
    /// the page's row first, and Close reads it, when the page holds no edit and its last read was of another row
    /// (`validate_view_read_row`).
    fn validate_view_step(
        &mut self,
        st: &St,
        v: &mut ValidateView,
        (call, target, args): (TestCall, Option<&ControlTarget>, &[Value]),
    ) -> Result<bool, Signal> {
        let page = match v.page {
            Some(page) => page,
            None => match self.validate_view_page(st) {
                Some(page) => {
                    // The open read the view from its first row, ending on it (ListValidateFilterRefresh.al:815 and every
                    // cell, `open` `P.AGR(A);P.AGR(B);P.AGR(C);P.AGR(A);`).
                    v.last_read = self.validate_view_first_key(st)?;
                    *v.page.insert(page)
                }
                None => return Ok(false),
            },
        };
        if let Some(p) = v.pending.take() {
            let step = self.validate_view_unrefreshed(st, v, &p)?;
            v.steps.push(step);
        }
        // The cells' Expand is Expand(true) on B, which has a child (TreeValidateFilterRefresh.al:902).
        if v.steps.last() == Some(&Step::Expand) && !self.validate_view_expanded(st)? {
            return Ok(false);
        }
        let cells = page.cells();
        if !Self::validate_view_calls_ok(st, page) || !cells.iter().any(|c| c.steps.starts_with(&v.steps)) {
            return Ok(false);
        }
        let tail = |t: Tail| cells.iter().any(|c| c.steps == v.steps.as_slice() && c.tail == t);
        let next = match call {
            TestCall::Page(PageMethod::First) if (tail(Tail::Walk) || tail(Tail::FirstClose)) && !v.walked => {
                v.walked = true;
                return Ok(true);
            }
            TestCall::Page(PageMethod::Next) if v.walked && tail(Tail::Walk) => {
                v.nexted = true;
                return Ok(true);
            }
            TestCall::Page(PageMethod::Close)
                if match (v.walked, v.nexted) {
                    (false, _) => tail(Tail::Close),
                    (true, true) => tail(Tail::Walk),
                    (true, false) => tail(Tail::FirstClose),
                } =>
            {
                self.validate_view_read_row(st, v)?;
                return Ok(true);
            }
            TestCall::Page(PageMethod::Last) => Step::Last,
            TestCall::Page(PageMethod::Previous) => Step::Previous,
            TestCall::Page(PageMethod::GoToKey) => Step::GoToKey,
            TestCall::Page(PageMethod::Expand) => Step::Expand,
            TestCall::Write(i) => match self.validate_view_ctl(st, i) {
                Some(c) => Set(c, Upd::None, Same, None),
                None => return Ok(false),
            },
            TestCall::Other(PageMethod::Invoke) => Action(Upd::None, Same),
            _ if self.validate_view_reads(st, page, call, target, args) => match v.steps.last() {
                // Read-only calls in a row are one step; after First they are no step of a cell.
                Some(Step::Reads) if !v.walked => return Ok(true),
                _ => Step::Reads,
            },
            _ => return Ok(false),
        };
        // The kind of a SetValue or of the action is known once its code ran (`validate_view_refresh`,
        // `validate_view_unrefreshed`).
        let continues = cells.iter().any(|c| {
            c.steps.starts_with(&v.steps)
                && match (c.steps.get(v.steps.len()), next) {
                    (Some(Set(a, ..) | Fail(a)), Set(b, ..)) => *a == b,
                    (Some(Action(..)), Action(..)) => true,
                    (Some(a), b) => *a == b,
                    (None, _) => false,
                }
        });
        if !continues || v.walked {
            return Ok(false);
        }
        let ctl = match next {
            Set(c, ..) => Some(c),
            Action(..) => None,
            step => {
                v.steps.push(step);
                return Ok(true);
            }
        };
        let rec = self.page_rec(st);
        // Every cell's first SetValue or action runs with no filter on Rec in any group; the gate checks it here instead of
        // taking it from the calls alone (a filter that keeps the three rows would otherwise match the no-change cells). No
        // test reaches this check today: in the runner a TestPage filter before OpenEdit fails (the page is not open), and
        // one after it turns the rule off at its SetFilter call.
        let first = !v.steps.iter().any(|s| matches!(s, Set(..) | Fail(_) | Action(..)));
        if (first && !Self::view_filters(&rec).is_empty()) || !self.validate_view_table_fits(st, &rec, page)? {
            return Ok(false);
        }
        let table = self.table(&rec)?;
        let key = table.primary_key(&rec.borrow().fields);
        let filters = Self::view_filters(&rec);
        let call = st.borrow().tree_calls.len().saturating_sub(1);
        // Before a SetValue, also one that fails on its value (:1413 `error` `P.AGR(C);`), and before the action (:1054
        // `P.AGR(C);P.AGR(C);P.A(C);`, the second read is the one before every action).
        self.validate_view_read_row(st, v)?;
        v.pending = Some(Pending {
            ctl,
            filters,
            key,
            update: None,
            call,
        });
        Ok(true)
    }

    /// The page reads its row (OnAfterGetRecord) when it holds no edit and its last read was of another row: before a SetValue
    /// or the action, and at Close (ListValidateFilterRefresh.al:832 `setname` and `setvalue` begin `P.AGR(C);`, after the open
    /// and after a refresh that read back to the first row; :1003 `P.AGR(B);` on the middle row after the open, and nothing
    /// before its second SetValue, whose refresh ended on B; :1037 nothing on the first row; :1413 nothing before the SetValue
    /// after the failed one, which read C; :1397 and :1413 `close` `P.AGR(C);`, :1381 `close` empty after Previous onto B).
    /// After the walk Close reads nothing: Next until it answers No ends on the new line, which holds no stored row (:832 `walk`
    /// `B,C,,`, `close` empty).
    fn validate_view_read_row(&mut self, st: &St, v: &mut ValidateView) -> Result<(), Signal> {
        if !v.page.is_some_and(CellPage::reads_first) {
            return Ok(());
        }
        let rec = self.page_rec(st);
        let shape = {
            let s = st.borrow();
            !s.dirty && !s.draft && s.cursor == Cursor::Row && s.open
        };
        if !shape || self.stored_row(&rec)?.is_none() {
            return Ok(());
        }
        let key = self.table(&rec)?.primary_key(&rec.borrow().fields);
        if v.last_read.as_ref().is_some_and(|k| Self::same_key(k, &key)) {
            return Ok(());
        }
        assume(match v.page {
            Some(page) if page.tree() => Assumption::TreeValidateViewRefresh,
            _ => Assumption::ListValidateViewRefresh,
        });
        v.ran = true;
        self.populate_current(st)?;
        v.last_read = Some(key);
        Ok(())
    }

    /// `call` with `args` is one of the read-only calls the read cells measure on `page` between its steps (`Step::Reads`,
    /// `CellPage::read_calls`): on a control bound to a field of the type the read was measured on (`CellPage::read_fits`),
    /// or on the page. They log nothing, and the cell reads on as without them.
    fn validate_view_reads(
        &self,
        st: &St,
        page: CellPage,
        call: TestCall,
        target: Option<&ControlTarget>,
        args: &[Value],
    ) -> bool {
        let (control, on_page) = page.read_calls();
        match (call, target) {
            // GetField by the Name control's id, then its Value, which passes as any Value read (ListValidateViewReads.al
            // `ReadsLines` block 3, :598, :615, :632 on page 97498; `get_field.rs` finds the control by the compiler's id).
            (TestCall::Page(PageMethod::GetField), _) => {
                page == CellPage::Wide
                    && matches!(args, [Value::Integer(id)] if self
                        .own_control_by_id(st, *id)
                        .ok()
                        .flatten()
                        .is_some_and(|i| self.validate_view_ctl(st, i) == Some(Ctl::Name)))
            }
            (TestCall::Page(m), _) => on_page.contains(&m),
            (TestCall::Other(m), Some(ControlTarget::Control(i))) if control.contains(&m) => {
                let engine = self.engine();
                let ir::Binding::Field(f) = page_of(&engine, st).controls[*i].binding else {
                    return false;
                };
                let table = self.page_rec(st).borrow().table as usize;
                engine.tables[table]
                    .as_ref()
                    .and_then(|t| t.fields.get(f))
                    .is_some_and(|field| page.read_fits(m, &field.ty, args))
            }
            _ => false,
        }
    }

    /// The primary key of the first row of the page's view.
    fn validate_view_first_key(&mut self, st: &St) -> Result<Option<Vec<Value>>, Signal> {
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        Ok(self
            .view_rows(&Self::full_view(&rec), 1)?
            .first()
            .map(|r| table.primary_key(r)))
    }

    /// Every call the test made on `st`, a page of kind `page`, since it opened ended as the cells' calls do: an error only
    /// from a SetValue, No only from a Next or one of the page's own read-only calls (`CellPage::read_calls`: HideValue(),
    /// ShowMandatory(), IsExpanded()); the test wrote nothing to the store between two calls (`tree_calls_unknown`).
    fn validate_view_calls_ok(st: &St, page: CellPage) -> bool {
        let s = st.borrow();
        if s.host.is_some() || s.tree_calls_unknown {
            return false;
        }
        let Some((_, earlier)) = s.tree_calls.split_last() else {
            return false;
        };
        s.tree_calls.iter().all(|c| c.part.is_none())
            && earlier.iter().all(|c| {
                (!c.failed || matches!(c.call, TestCall::Write(_)))
                    && (!c.no
                        || c.call == TestCall::Page(PageMethod::Next)
                        || match c.call {
                            TestCall::Other(m) => page.read_calls().0.contains(&m),
                            TestCall::Page(m) => page.read_calls().1.contains(&m),
                            _ => false,
                        })
            })
    }

    /// The index of the row with primary key `key` in the view of `rec`, up to four rows; `None` when it is not there.
    fn view_index(&mut self, rec: &Rec, key: &[Value]) -> Result<Option<usize>, Signal> {
        let table = self.table(rec)?;
        let rows = self.view_rows(&Self::full_view(rec), 4)?;
        Ok(rows.iter().position(|r| Self::same_key(&table.primary_key(r), key)))
    }

    /// The row the tree stands on is expanded.
    fn validate_view_expanded(&mut self, st: &St) -> Result<bool, Signal> {
        let key = self.row_key(&self.page_rec(st))?;
        Ok(st.borrow().expanded.iter().any(|k| Self::same_key(k, &key)))
    }

    /// Rec's filters without the tree's cut (`apply_tree_cut`), which no cell's code sets.
    fn view_filters(rec: &Rec) -> Vec<FieldFilter> {
        rec.borrow()
            .filters
            .iter()
            .filter(|f| f.group != TREE_GROUP)
            .cloned()
            .collect()
    }

    /// `rec` without the tree's cut: the view the cells count, a tree's hidden rows included (TreeValidateFilterRefresh.al:794
    /// `setname` reads the hidden child C).
    fn full_view(rec: &Rec) -> Rec {
        let full = Rc::new(RefCell::new(rec.borrow().clone()));
        full.borrow_mut().filters.retain(|f| f.group != TREE_GROUP);
        full
    }

    /// The step a SetValue or the action `p` was, when this module's refresh did not run for it: a SetValue that failed or ran
    /// no CurrPage.Update, or the action. After the action the page's last read is that of main's refresh of a list without
    /// OnAfterGetCurrRecord: back to the first row of the view when rows stand before the row, else the row
    /// (ListValidateFilterRefresh.al:1054 `P.AGR(C);P.AGR(C);P.AGR(B);`, :1071 `P.AGR(A);P.AGR(A);`).
    fn validate_view_unrefreshed(&mut self, st: &St, v: &mut ValidateView, p: &Pending) -> Result<Step, Signal> {
        let failed = st.borrow().tree_calls.get(p.call).is_some_and(|c| c.failed);
        if failed {
            return Ok(p.ctl.map_or(Step::Other, Fail));
        }
        let change = self.validate_view_change(st, p)?;
        let update = match p.update {
            None => Upd::None,
            Some(false) => Upd::False,
            Some(true) => Upd::True,
        };
        if p.ctl.is_none() {
            let rec = self.page_rec(st);
            let key = self.table(&rec)?.primary_key(&rec.borrow().fields);
            v.last_read = match self.view_index(&rec, &key)? {
                Some(0) => Some(key),
                _ => self.validate_view_first_key(st)?,
            };
        }
        Ok(match p.ctl {
            Some(c) if update == Upd::None => Set(c, Upd::None, change, None),
            Some(_) => Step::Other,
            None => Action(update, change),
        })
    }

    /// What the call of `p` did to the view of the page's record (see `Change`), in the primary key, ascending, filter group
    /// 0, without marks.
    fn validate_view_change(&mut self, st: &St, p: &Pending) -> Result<Change, Signal> {
        let rec = self.page_rec(st);
        let plain = {
            let r = rec.borrow();
            r.key == 0
                && r.ascending
                && r.field_descending == 0
                && r.adhoc.is_none()
                && !r.marked_only
                && r.filter_group == 0
        };
        if !plain {
            return Ok(Change::Other);
        }
        // `Some(true)` for SetRange's form of one value, `Some(false)` for SetFilter's.
        let one_value = |f: &[FieldFilter]| match f {
            [f] if f.field == 0 && f.group == 0 => match &f.expr {
                FilterExpr::Eq(_) => Some(false),
                FilterExpr::Range(Some(a), Some(b))
                    if Self::same_values(std::slice::from_ref(a), std::slice::from_ref(b)) =>
                {
                    Some(true)
                }
                _ => None,
            },
            _ => None,
        };
        let now = Self::view_filters(&rec);
        let same = p.filters.len() == now.len()
            && p.filters.iter().zip(&now).all(|(a, b)| {
                a.field == b.field
                    && a.group == b.group
                    && matches!((self.filter_text(&a.expr), self.filter_text(&b.expr)), (Ok(x), Ok(y)) if x == y)
            });
        if same {
            return Ok(Same);
        }
        let kept = self.view_index(&rec, &p.key)?.is_some();
        Ok(
            match (
                p.filters.is_empty(),
                one_value(&now),
                one_value(&p.filters),
                now.is_empty(),
            ) {
                (true, Some(true), ..) if kept => Keep,
                (true, Some(true), ..) => Drop,
                (true, Some(false), ..) if kept => KeepText,
                (false, _, Some(true), true) => Reset,
                _ => Change::Other,
            },
        )
    }

    /// `CurrPage.Update(save)` ran in the SetValue or action of the cells that runs now.
    pub(super) fn validate_view_update(st: &St, save: bool) {
        if let Some(p) = st.borrow_mut().validate_view.pending.as_mut() {
            p.update = Some(save);
        }
    }

    /// The refresh `CurrPage.Update` asked for in the OnValidate of the cells' SetValue that runs now
    /// (`refresh_after_validate_filter`). The row is read again from the table, so a typed value is gone (Update(true) saved it
    /// first); a row the new view no longer holds gives way to the next row of the view after its key, else the one before it.
    /// On a List without OnAfterGetCurrRecord the page then reads the view from that row (`reread_view_from_row`): the row,
    /// the rows after it, back to the first row, and not the row again
    /// (ListValidateFilterRefresh.al:832 `P.AGR(C);T.V(C2);P.V(C2);P.AGR(C);P.AGR(B);`, shows `C/`; :900 the row dropped,
    /// `...P.AGR(A);`; :1003 from the middle row `...P.AGR(B);P.AGR(C);P.AGR(B);`; :951 with no change of view
    /// `...P.AGR(C);P.AGR(B);P.AGR(A);`; :849 Update(true) `...P.AGR(C2);P.AGR(B);`, C2 saved). The reads leave no mark that
    /// ends the next refresh: the next SetValue's Update(false) reads again (:832 `setvalue` `P.AGR(C);P.AGR(C);P.AGR(B);`).
    /// `false` when main's refresh runs: on a List with OnAfterGetCurrRecord only and on a Card after the edit is dropped, and
    /// when the cells do not measure this refresh and the rule changed nothing on the page yet.
    pub(super) fn validate_view_refresh(
        &mut self,
        st: &St,
        without_curr: bool,
        without_parts: bool,
    ) -> Result<bool, Signal> {
        let (pending, ran, page) = {
            let s = st.borrow();
            let v = &s.validate_view;
            (v.pending.clone().filter(|_| !v.off), v.ran, v.page)
        };
        let (Some((p, ctl)), Some(page)) = (pending.and_then(|p| p.ctl.map(|c| (p, c))), page) else {
            return Ok(false);
        };
        let update = match p.update {
            Some(false) => Upd::False,
            Some(true) => Upd::True,
            None => Upd::None,
        };
        let change = self.validate_view_change(st, &p)?;
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        let key = table.primary_key(&rec.borrow().fields);
        let settled = match self.view_index(&rec, &key)? {
            Some(_) => None,
            None => self.validate_view_settle(&rec)?,
        };
        let at_key = settled.as_ref().map_or_else(|| key.clone(), |f| table.primary_key(f));
        let rows = self.view_rows(&Self::full_view(&rec), 4)?.len();
        let at = self.view_index(&rec, &at_key)?;
        let mut steps = st.borrow().validate_view.steps.clone();
        steps.push(Set(ctl, update, change, at.map(|at| (rows, at))));
        let measured = page.cells().iter().any(|c| c.steps.starts_with(&steps))
            && self.validate_view_table_fits(st, &rec, page)?;
        if !measured {
            if ran {
                return unmeasured(Some(page));
            }
            let mut s = st.borrow_mut();
            s.validate_view.off = true;
            s.validate_view.pending = None;
            return Ok(false);
        }
        // On a tree Update(false) saves the row too, before the refresh reads it (TreeValidateFilterRefresh.al:794 `stored1`
        // `A/,C2/,C/,`; the List of ListValidateFilterRefresh.al:832 keeps `A/,B/,C/,`).
        if page.tree() && update == Upd::False && st.borrow().dirty {
            self.save_row(st)?;
        }
        let mut pk = key;
        self.ret_used = true;
        self.record_op(RecordOp::Get, &rec, &mut pk)?;
        if let Some(fields) = settled {
            rec.borrow_mut().fields = fields;
        }
        if page.tree() {
            let done = self.validate_view_tree_refresh(st, page, at, at_key, without_curr, without_parts)?;
            let mut s = st.borrow_mut();
            s.validate_view.steps = steps;
            s.validate_view.pending = None;
            s.validate_view.ran = true;
            return Ok(done);
        }
        let done = match page {
            CellPage::Issue | CellPage::Wide | CellPage::Lines => {
                assume(Assumption::ListValidateViewRefresh);
                self.reread_view_from_row(st, without_curr, without_parts, false)?;
                // The note `read_list_again_from_row_as` leaves for a refresh without OnAfterGetCurrRecord would end the next
                // one.
                st.borrow_mut().refresh.spend_reads_done();
                // The reads went back to the first row of the view when rows stood before the row, else they ended on the row.
                let last_read = match at {
                    Some(0) => Some(at_key),
                    _ => self.validate_view_first_key(st)?,
                };
                st.borrow_mut().validate_view.last_read = last_read;
                true
            }
            // With OnAfterGetCurrRecord the row is read twice more after the way back, then OnAfterGetCurrRecord
            // (:1105 `T.V(C2);P.V(C2);P.AGR(C);P.AGR(B);P.AGR(C);P.AGR(C);P.AGCR(C);`, :1139 the row dropped
            // `...P.AGR(A);P.AGR(A);P.AGCR(A);`, :1156 no change `...P.AGR(C);P.AGR(B);P.AGR(A);P.AGR(C);P.AGR(C);P.AGCR(C);`).
            CellPage::Both => {
                assume(Assumption::ValidateUpdateFalseRereadsRow);
                self.reread_view_from_row(st, without_curr, without_parts, true)?;
                true
            }
            // The stored row, with no edit, goes on through main's refresh: OnAfterGetRecord and OnAfterGetCurrRecord of the
            // row (:1190 `T.V(C2);P.V(C2);P.AGCR(C);`, :1258 `T.V(C2);P.V(C2);P.AGR(C);`, shows `C/`; :1632 `AGR(Alpha);`).
            CellPage::Curr | CellPage::Card | CellPage::EditCard => {
                assume(Assumption::ValidateUpdateFalseRereadsRow);
                st.borrow_mut().dirty = false;
                false
            }
            // Named one by one, so that a new page kind must choose its arm here.
            CellPage::TreeIssue
            | CellPage::Tree
            | CellPage::TreeBoth
            | CellPage::TreeCurr
            | CellPage::TreeCollapsed => {
                unreachable!("a tree page refreshes in validate_view_tree_refresh")
            }
        };
        let mut s = st.borrow_mut();
        s.validate_view.steps = steps;
        s.validate_view.pending = None;
        s.validate_view.ran = true;
        Ok(done)
    }

    /// The refresh of `validate_view_refresh` on a tree page, the row saved and read again. The reads are those of the List of
    /// the same row triggers, over every row of the tree's view, its hidden rows too; then the tree shows the rows of the new
    /// view as it did (`apply_tree_cut`). With OnAfterGetRecord only: the row, the rows after it, back to the first row
    /// (TreeValidateFilterRefresh.al:761 `AGR(B);AGR(C2);AGR(C);AGR(C2);`, shows `C2`, walks `C2,`; :866 no change of view
    /// `...P.AGR(C2);P.AGR(C);P.AGR(C2);P.AGR(A);`; :902 B expanded first, walks `C2,C,`). With both row triggers the row once
    /// more and OnAfterGetCurrRecord, twice when the reads ended on another row (:939 `...P.AGR(C2);P.AGR(C2);P.AGCR(C2);`, :975
    /// `...P.AGR(A);P.AGR(C2);P.AGR(C2);P.AGCR(C2);`). With OnAfterGetCurrRecord only, main's refresh (:1012 `P.V(C2);P.AGCR(C2);`).
    fn validate_view_tree_refresh(
        &mut self,
        st: &St,
        page: CellPage,
        at: Option<usize>,
        at_key: Vec<Value>,
        without_curr: bool,
        without_parts: bool,
    ) -> Result<bool, Signal> {
        assume(Assumption::TreeValidateViewRefresh);
        if page == CellPage::TreeCurr {
            st.borrow_mut().dirty = false;
            return Ok(false);
        }
        let with_curr = page == CellPage::TreeBoth;
        // The reads run over the tree's hidden rows too; the client then holds the rows the tree shows (`read_tree_rows_whole`).
        // When the page's code still runs, its cut is already off.
        let read = |s: &mut Self| s.reread_view_from_row(st, without_curr, without_parts, with_curr);
        if !self.read_tree_rows_whole(st, read)? {
            self.reread_view_from_row(st, without_curr, without_parts, with_curr)?;
            let level = self.tree_level_field(st)?;
            self.split_tree_window(st, level);
        }
        if !with_curr {
            st.borrow_mut().refresh.spend_reads_done();
            let last_read = match at {
                Some(0) => Some(at_key),
                _ => self.validate_view_first_key(st)?,
            };
            st.borrow_mut().validate_view.last_read = last_read;
        }
        Ok(true)
    }

    /// The row a refresh settles on when the view no longer holds the row of `rec`: the next row of the view after its key, else
    /// the one before it (ListValidateFilterRefresh.al:1020 A to B, :900 C to A).
    fn validate_view_settle(&mut self, rec: &Rec) -> Result<Option<Fields>, Signal> {
        for step in [1, -1] {
            let probe = Rc::new(RefCell::new(rec.borrow().clone()));
            self.ret_used = true;
            if matches!(self.record_op(RecordOp::Next, &probe, &mut [Value::Integer(step)])?, Value::Integer(n) if n != 0)
            {
                return Ok(Some(probe.borrow().fields.clone()));
            }
        }
        Ok(None)
    }

    /// The table of `rec` holds the rows the cells of `page` hold: their number (`CellPage::table_rows`) and, on a tree page,
    /// the Indentation of each row in the key, by equality (`CellPage::indentations`).
    fn validate_view_table_fits(&mut self, st: &St, rec: &Rec, page: CellPage) -> Result<bool, Signal> {
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        {
            let mut p = probe.borrow_mut();
            p.filters.clear();
            p.marked_only = false;
        }
        let rows = self.view_rows(&probe, 5)?;
        if rows.len() != page.table_rows() {
            return Ok(false);
        }
        let Some(want) = page.indentations() else {
            return Ok(true);
        };
        let Some(level) = self.tree_level_field(st)? else {
            return Ok(false);
        };
        Ok(rows.len() == want.len()
            && rows
                .iter()
                .zip(want)
                .all(|(r, i)| matches!(r.get(level), Some(Value::Integer(n)) if n == i)))
    }

    /// The control of `st` at `idx` as the cells type into it: bound to the field Name or to the field Value of Rec.
    fn validate_view_ctl(&self, st: &St, idx: usize) -> Option<Ctl> {
        let engine = self.engine();
        let ir::Binding::Field(f) = page_of(&engine, st).controls[idx].binding else {
            return None;
        };
        let table = self.page_rec(st).borrow().table as usize;
        match engine.tables[table].as_ref()?.fields.get(f)?.name.as_str() {
            "Name" => Some(Ctl::Name),
            "Value" => Some(Ctl::Value),
            _ => None,
        }
    }
}
