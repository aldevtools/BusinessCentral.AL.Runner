//! A control's OnValidate that calls `CurrPage.Update` on a List page with OnAfterGetRecord and without OnAfterGetCurrRecord,
//! with or without a change of the page's view (al-runner issue 8698), measured by corpus pull request 1518,
//! tests/al-language/pages/currpage/ListValidateFilterRefresh.al (open). On BC the refresh reads the row again from the table,
//! so a typed value is gone; then the rows after it in the view and the rows before it back to the first, and not the row
//! again (`reread_view_from_row`). A SetValue, the action and Close read the row first when the page's last read was of
//! another row (`validate_view_read_row`). Main kept the typed value and the rows the client held, and read nothing first.
//!
//! Only on the probe's page 97584 and the issue's page (the probe's page 97583 and the reporter's page, line for line), over
//! the probe's table, as the test called them in the cells (`CELLS`, `ISSUE_CELLS`): assumption `list-validate-view-refresh`.
//! Any other page, or calls the cells do not make before the rule changed anything, keep main's path; after it did they
//! refuse.

use super::pinned_body::{Callees, Names, callees_with_calls_are, page_is, table_is};
use super::pinned_shape::TestCall;
use super::*;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::filter::{FieldFilter, FilterExpr};
use al_builtins::page_op::PageMethod;
use al_builtins::record_op::RecordOp;

/// The probe's table 97581 "LT RFU Line" (ListValidateFilterRefresh.al:17-41), and the reporter's table: `«L»` is the log
/// codeunit.
const TABLE: [&str; 7] = [
    "DataClassification = CustomerContent",
    "field 1 \"Group Code\": Code[10]",
    "field 2 \"Entry No.\": Integer",
    "field 3 \"Name\": Text[30]",
    "field 4 \"Value\": Text[30]",
    "key \"Group Code\", \"Entry No.\" {Clustered=true}",
    "trigger \"Name\".OnValidate: var Log: Codeunit \"«L»\"; begin Log.Add((('T.V(' + Rec.\"Name\") + ')')); end",
];

/// The issue's page: the probe's page 97583 "LT RFU Issue Lines" (:88-131) and the reporter's page 50122 read the same; `«T»`
/// is the table.
const ISSUE_PAGE: [&str; 10] = [
    "PageType = List",
    "SourceTable = \"«T»\"",
    "ApplicationArea = All",
    "UsageCategory = None",
    "repeater Lines in - {}",
    "field Name in Lines = Name {applicationarea=All}",
    "  trigger onvalidate: begin Rec.SetRange(field#0, 'G1'); CurrPage.Update(false); end",
    "field Value in Lines = Value {applicationarea=All}",
    "  trigger onvalidate: begin CurrPage.Update(false); end",
    "trigger onaftergetrecord: begin Log.Add((('P.AGR(' + Rec.\"Name\") + ')')); end",
];

/// The probe's page 97584 "LT RFU Lines" (:134-214): the Name control's OnValidate and the action run ChangeView.
const LINES_PAGE: [&str; 13] = [
    "PageType = List",
    "SourceTable = \"«T»\"",
    "ApplicationArea = All",
    "UsageCategory = None",
    "repeater Lines in - {}",
    "field Name in Lines = Name {applicationarea=All}",
    "  trigger onvalidate: begin Log.Add((('P.V(' + Rec.\"Name\") + ')')); ChangeView(); end",
    "field Value in Lines = Value {applicationarea=All}",
    "  trigger onvalidate: begin CurrPage.Update(false); end",
    "action ChangeViewAction in - {applicationarea=All}",
    "  trigger onaction: begin Log.Add((('P.A(' + Rec.\"Name\") + ')')); ChangeView(); end",
    "area Processing: ChangeViewAction",
    "trigger onaftergetrecord: begin Log.Add((('P.AGR(' + Rec.\"Name\") + ')')); end",
];

/// The procedures the pages and the table call: codeunit 97582 "LT RFU Log" (`«L»`, :43-84 Add, FilterStep, UpdateStep) and
/// the page's ChangeView (`«P»`, :195-213).
const CALLEES: [&str; 4] = [
    "\"«L»\".Add: begin Buffer := (Buffer + (Entry + ';')); end",
    "\"«L»\".FilterStep: begin exit(Filtering); end",
    "\"«L»\".UpdateStep: begin exit(Updating); end",
    "\"«P»\".ChangeView: begin case Log.FilterStep() of 1: begin Rec.SetRange(field#0, 'G1'); end; 2: begin \
     Rec.SetFilter(field#0, 'G1'); end; 3: begin Rec.Reset(); end; 4: begin Rec.SetRange(field#0, 'G0'); end; end; case \
     Log.UpdateStep() of 1: begin CurrPage.Update(false); end; 2: begin CurrPage.Update(true); end; end; end",
];

/// Which measured page `st` is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CellPage {
    Issue,
    Lines,
}

/// The control a SetValue typed into: the one bound to Name (field 3) or to Value (field 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Ctl {
    Name,
    Value,
}

/// The `CurrPage.Update` a step ran: `Update(false)`, `Update(true)`, or none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Upd {
    False,
    True,
    None,
}

/// What a step did to the page's view: nothing; a filter on one value of the first key field set where there was none, as
/// SetRange sets it, the row the step started on still in the view (`Keep`) or not (`Drop`); the same as SetFilter sets it,
/// the row still in the view (`KeepText`); a filter SetRange set taken off again (`Reset`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Change {
    Same,
    Keep,
    KeepText,
    Drop,
    Reset,
    Other,
}

/// One step of the test the cells take, reads left out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Step {
    Last,
    Previous,
    /// A SetValue, with the rows of the view after it and the index of the row its refresh settled on (`None`: no refresh).
    Set(Ctl, Upd, Change, Option<(usize, usize)>),
    /// A SetValue that raised an error before its control's OnValidate (a value too long for the field).
    Fail(Ctl),
    /// The action ChangeViewAction (the refresh after it is main's).
    Action(Upd, Change),
    /// A step the cells do not take.
    Other,
}

use Change::{Drop, Keep, KeepText, Reset, Same};
use Ctl::{Name, Value as Val};
use Step::{Action, Fail, Last, Previous, Set};

/// The calls of a cell after its steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tail {
    /// First, Next until it answers No, and Close.
    Walk,
    /// First, then Close.
    FirstClose,
    /// Close.
    Close,
}

/// The steps of one cell and the calls after them.
struct Cell {
    steps: &'static [Step],
    tail: Tail,
}

/// The cells on page 97584 (ListValidateFilterRefresh.al, the line of each test).
const CELLS: [Cell; 20] = [
    // :832
    Cell {
        steps: &[
            Last,
            Set(Name, Upd::False, Keep, Some((2, 1))),
            Set(Val, Upd::False, Same, Some((2, 1))),
        ],
        tail: Tail::Walk,
    },
    // :883
    Cell {
        steps: &[
            Last,
            Set(Name, Upd::False, KeepText, Some((2, 1))),
            Set(Val, Upd::False, Same, Some((2, 1))),
        ],
        tail: Tail::Walk,
    },
    // :849
    Cell {
        steps: &[
            Last,
            Set(Name, Upd::True, Keep, Some((2, 1))),
            Set(Val, Upd::False, Same, Some((2, 1))),
        ],
        tail: Tail::Walk,
    },
    // :1327
    Cell {
        steps: &[
            Last,
            Set(Name, Upd::True, KeepText, Some((2, 1))),
            Set(Val, Upd::False, Same, Some((2, 1))),
        ],
        tail: Tail::Walk,
    },
    // :866
    Cell {
        steps: &[
            Last,
            Set(Name, Upd::None, Keep, None),
            Set(Val, Upd::False, Same, Some((2, 1))),
        ],
        tail: Tail::Walk,
    },
    // :900
    Cell {
        steps: &[
            Last,
            Set(Name, Upd::False, Drop, Some((1, 0))),
            Set(Val, Upd::False, Same, Some((1, 0))),
        ],
        tail: Tail::Walk,
    },
    // :917
    Cell {
        steps: &[
            Last,
            Set(Name, Upd::True, Drop, Some((1, 0))),
            Set(Val, Upd::False, Same, Some((1, 0))),
        ],
        tail: Tail::Walk,
    },
    // :951 no filter, :1345 Reset of no filter.
    Cell {
        steps: &[
            Last,
            Set(Name, Upd::False, Same, Some((3, 2))),
            Set(Val, Upd::False, Same, Some((3, 2))),
        ],
        tail: Tail::Walk,
    },
    // :968
    Cell {
        steps: &[
            Last,
            Set(Name, Upd::True, Same, Some((3, 2))),
            Set(Val, Upd::False, Same, Some((3, 2))),
        ],
        tail: Tail::Walk,
    },
    // :985
    Cell {
        steps: &[
            Last,
            Set(Name, Upd::False, Keep, Some((2, 1))),
            Set(Name, Upd::False, Reset, Some((3, 2))),
        ],
        tail: Tail::Walk,
    },
    // :1003
    Cell {
        steps: &[
            Last,
            Previous,
            Set(Name, Upd::False, Keep, Some((2, 0))),
            Set(Val, Upd::False, Same, Some((2, 0))),
        ],
        tail: Tail::Walk,
    },
    // :1020
    Cell {
        steps: &[
            Set(Name, Upd::False, Drop, Some((2, 0))),
            Set(Val, Upd::False, Same, Some((2, 0))),
        ],
        tail: Tail::Walk,
    },
    // :1037
    Cell {
        steps: &[
            Set(Name, Upd::False, Same, Some((3, 0))),
            Set(Val, Upd::False, Same, Some((3, 0))),
        ],
        tail: Tail::Walk,
    },
    // :1054
    Cell {
        steps: &[Last, Action(Upd::False, Keep), Set(Val, Upd::False, Same, Some((2, 1)))],
        tail: Tail::Walk,
    },
    // :1071
    Cell {
        steps: &[Last, Action(Upd::False, Drop), Set(Val, Upd::False, Same, Some((1, 0)))],
        tail: Tail::Walk,
    },
    // :1088
    Cell {
        steps: &[Last, Action(Upd::None, Keep), Set(Val, Upd::False, Same, Some((2, 1)))],
        tail: Tail::Walk,
    },
    // :1363
    Cell {
        steps: &[
            Last,
            Set(Name, Upd::None, Drop, None),
            Set(Val, Upd::False, Same, Some((1, 0))),
        ],
        tail: Tail::Walk,
    },
    // :1381
    Cell {
        steps: &[
            Last,
            Set(Name, Upd::False, Keep, Some((2, 1))),
            Set(Val, Upd::False, Same, Some((2, 1))),
            Previous,
        ],
        tail: Tail::Close,
    },
    // :1397
    Cell {
        steps: &[
            Last,
            Set(Name, Upd::False, Keep, Some((2, 1))),
            Set(Val, Upd::False, Same, Some((2, 1))),
            Set(Val, Upd::False, Same, Some((2, 1))),
        ],
        tail: Tail::Close,
    },
    // :1413
    Cell {
        steps: &[
            Last,
            Fail(Name),
            Set(Name, Upd::False, Keep, Some((2, 1))),
            Set(Val, Upd::False, Same, Some((2, 1))),
        ],
        tail: Tail::Close,
    },
];

/// The cells on the issue's page (:815; :1477 the reporter's steps, First then Close; :1444 First right after the SetValue on
/// Name).
const ISSUE_CELLS: [Cell; 3] = [
    Cell {
        steps: &[
            Last,
            Set(Name, Upd::False, Keep, Some((2, 1))),
            Set(Val, Upd::False, Same, Some((2, 1))),
        ],
        tail: Tail::Walk,
    },
    Cell {
        steps: &[
            Last,
            Set(Name, Upd::False, Keep, Some((2, 1))),
            Set(Val, Upd::False, Same, Some((2, 1))),
        ],
        tail: Tail::FirstClose,
    },
    Cell {
        steps: &[Last, Set(Name, Upd::False, Keep, Some((2, 1)))],
        tail: Tail::Walk,
    },
];

/// The refusal of a call the cells do not make, after the rule ran.
const UNMEASURED: &str = "a call on a page of ListValidateFilterRefresh.al after the page read its row or the view again as the cells measure it, which the cells do not make (not measured)";

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
    /// The cells' page `st` is, every fact by equality: a top-level List opened by the test with OpenEdit, no part, Rec over
    /// the database table, and its code, its table and the procedures they call line for line as the probe's (`page_is`,
    /// `table_is`, `callees_with_calls_are`; nothing subscribes to their events).
    fn validate_view_page(&mut self, st: &St) -> Option<CellPage> {
        {
            let s = st.borrow();
            if !s.test_open_edit() || !s.parts.is_empty() || !s.hidden_parts.is_empty() || !s.open {
                return None;
            }
        }
        if !self.is_list_page(st)
            || !self.has_source(st)
            || self.page_rec(st).borrow().temporary
            || !self.declares(st, "onaftergetrecord")
            || self.declares(st, "onaftergetcurrrecord")
        {
            return None;
        }
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let table = self.page_rec(st).borrow().table as usize;
        [(CellPage::Issue, &ISSUE_PAGE[..]), (CellPage::Lines, &LINES_PAGE[..])]
            .into_iter()
            .find(|(_, want)| {
                let (mut callees, mut names) = (Callees::new(), Names::default());
                page_is(&engine, object, want, &mut callees, &mut names)
                    && table_is(&engine, table, &TABLE, &mut callees, &mut names)
                    && callees_with_calls_are(&engine, &callees, &CALLEES, &mut names)
            })
            .map(|(page, _)| page)
    }

    /// The test calls `call` on `st` (`test_page_op_held`, `test_control_op_held`). On the cells' page the call joins the
    /// steps (`validate_view_step`); a call the cells do not make turns the rule off, or refuses once the rule ran.
    pub(super) fn validate_view_call(&mut self, st: &St, call: Option<TestCall>) -> Result<(), Signal> {
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
        let r = self.validate_view_step(st, &mut v, call);
        let measured = matches!(r, Ok(true));
        v.off = !measured && !v.ran;
        let refuse = !measured && v.ran;
        st.borrow_mut().validate_view = v;
        r?;
        if refuse {
            return unsupported(UNMEASURED);
        }
        Ok(())
    }

    /// `validate_view_call` on the taken state: `false` when the call is not one the cells make. A SetValue or the action reads
    /// the page's row first, and Close reads it, when the page holds no edit and its last read was of another row
    /// (`validate_view_read_row`).
    fn validate_view_step(&mut self, st: &St, v: &mut ValidateView, call: TestCall) -> Result<bool, Signal> {
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
        let cells: &[Cell] = match page {
            CellPage::Issue => &ISSUE_CELLS,
            CellPage::Lines => &CELLS,
        };
        if !Self::validate_view_calls_ok(st) || !cells.iter().any(|c| c.steps.starts_with(&v.steps)) {
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
            TestCall::Page(PageMethod::Last) => Last,
            TestCall::Page(PageMethod::Previous) => Previous,
            TestCall::Write(i) => match page_of(&self.engine(), st).controls[i].binding {
                ir::Binding::Field(2) => Set(Name, Upd::None, Same, None),
                ir::Binding::Field(3) => Set(Val, Upd::None, Same, None),
                _ => return Ok(false),
            },
            TestCall::Other(PageMethod::Invoke) => Action(Upd::None, Same),
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
        if !self.validate_view_three_rows(&rec)? {
            return Ok(false);
        }
        let table = self.table(&rec)?;
        let key = table.primary_key(&rec.borrow().fields);
        let filters = rec.borrow().filters.clone();
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
    fn validate_view_read_row(&mut self, st: &St, v: &mut ValidateView) -> Result<(), Signal> {
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
        assume(Assumption::ListValidateViewRefresh);
        v.ran = true;
        self.populate_current(st)?;
        v.last_read = Some(key);
        Ok(())
    }

    /// The primary key of the first row of the page's view.
    fn validate_view_first_key(&mut self, st: &St) -> Result<Option<Vec<Value>>, Signal> {
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        Ok(self.view_rows(&rec, 1)?.first().map(|r| table.primary_key(r)))
    }

    /// Every call the test made on `st` since it opened ended as the cells' calls do: an error only from a SetValue, No only
    /// from a Next; the test wrote nothing to the store between two calls (`tree_calls_unknown`).
    fn validate_view_calls_ok(st: &St) -> bool {
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
                    && (!c.no || c.call == TestCall::Page(PageMethod::Next))
            })
    }

    /// The index of the row with primary key `key` in the view of `rec`, up to four rows; `None` when it is not there.
    fn view_index(&mut self, rec: &Rec, key: &[Value]) -> Result<Option<usize>, Signal> {
        let table = self.table(rec)?;
        let rows = self.view_rows(rec, 4)?;
        Ok(rows.iter().position(|r| Self::same_key(&table.primary_key(r), key)))
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
        let now = rec.borrow().filters.clone();
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
    /// Then the page reads the view from that row as after a control's OnValidate on a list without OnAfterGetCurrRecord
    /// (`reread_view_from_row`): the row, the rows after it, back to the first row, and not the row again
    /// (ListValidateFilterRefresh.al:832 `P.AGR(C);T.V(C2);P.V(C2);P.AGR(C);P.AGR(B);`, shows `C/`; :900 the row dropped,
    /// `...P.AGR(A);`; :1003 from the middle row `...P.AGR(B);P.AGR(C);P.AGR(B);`; :951 with no change of view
    /// `...P.AGR(C);P.AGR(B);P.AGR(A);`; :849 Update(true) `...P.AGR(C2);P.AGR(B);`, C2 saved). The reads leave no mark that
    /// ends the next refresh: the next SetValue's Update(false) reads again (:832 `setvalue` `P.AGR(C);P.AGR(C);P.AGR(B);`).
    /// `false` when the cells do not measure this refresh and the rule changed nothing on the page yet: main's refresh runs.
    pub(super) fn validate_view_refresh(
        &mut self,
        st: &St,
        without_curr: bool,
        without_parts: bool,
    ) -> Result<bool, Signal> {
        let (pending, ran) = {
            let s = st.borrow();
            let v = &s.validate_view;
            (v.pending.clone().filter(|_| !v.off), v.ran)
        };
        let Some((p, ctl)) = pending.and_then(|p| p.ctl.map(|c| (p, c))) else {
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
        let rows = self.view_rows(&rec, 4)?.len();
        let at = self.view_index(&rec, &at_key)?;
        let mut steps = st.borrow().validate_view.steps.clone();
        steps.push(Set(ctl, update, change, at.map(|at| (rows, at))));
        let cells: &[Cell] = match st.borrow().validate_view.page {
            Some(CellPage::Issue) => &ISSUE_CELLS,
            Some(CellPage::Lines) => &CELLS,
            None => &[],
        };
        let measured = cells.iter().any(|c| c.steps.starts_with(&steps)) && self.validate_view_three_rows(&rec)?;
        if !measured {
            if ran {
                return unsupported(UNMEASURED);
            }
            let mut s = st.borrow_mut();
            s.validate_view.off = true;
            s.validate_view.pending = None;
            return Ok(false);
        }
        assume(Assumption::ListValidateViewRefresh);
        let mut pk = key;
        self.ret_used = true;
        self.record_op(RecordOp::Get, &rec, &mut pk)?;
        if let Some(fields) = settled {
            rec.borrow_mut().fields = fields;
        }
        self.reread_view_from_row(st, without_curr, without_parts, false)?;
        // The note `read_list_again_from_row_as` leaves for a refresh without OnAfterGetCurrRecord would end the next one.
        st.borrow_mut().refresh.spend_reads_done();
        // The reads went back to the first row of the view when rows stood before the row, else they ended on the row.
        let last_read = match at {
            Some(0) => Some(at_key),
            _ => self.validate_view_first_key(st)?,
        };
        let mut s = st.borrow_mut();
        s.validate_view.steps = steps;
        s.validate_view.pending = None;
        s.validate_view.last_read = last_read;
        s.validate_view.ran = true;
        Ok(true)
    }

    /// The row a refresh settles on when the view no longer holds the row of `rec`: the next row of the view after its key, else
    /// the one before it (ListValidateFilterRefresh.al:1020 A to B, :900 C to A).
    fn validate_view_settle(&mut self, rec: &Rec) -> Result<Option<Vec<Value>>, Signal> {
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

    /// The table of `rec` holds three rows, as in every cell (ListValidateFilterRefresh.al:563-565).
    fn validate_view_three_rows(&mut self, rec: &Rec) -> Result<bool, Signal> {
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        {
            let mut p = probe.borrow_mut();
            p.filters.clear();
            p.marked_only = false;
        }
        Ok(self.view_rows(&probe, 4)?.len() == 3)
    }
}
