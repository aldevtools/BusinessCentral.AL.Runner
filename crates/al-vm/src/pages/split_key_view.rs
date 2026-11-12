//! AutoSplitKey on a ListPart that is not read in primary key order: a SubPageView or the part's own SourceTableView in a
//! secondary key, or the primary key descending (pages/parts/PartSplitKeyOtherSortKey.al, codeunit 62622, measured on BC 27.5,
//! 28.5 and 29.0). The number is still numbered in the primary key, from the row the line was started below (`lower`) and the
//! row the grid shows after it (`upper`):
//! - an `upper` whose number is below `lower` raises "The line cannot be split." at `New()` (`split_view_check_at_new`);
//! - otherwise the number is the primary-key rule of `auto_split_key` (half the gap, at most 10000; `lower` + 10000 without
//!   `upper`), and when a stored row already holds that number, it is numbered once more between that row and the next stored
//!   row in primary key order (`split_view_free`).
//!
//! Only the shapes the corpus pins take this path (`auto_split_part_pinned`); any other part read in another order is refused as
//! before.

use super::pinned_shape::{PartStep, PinnedPart, TestCall};
use super::*;
use al_builtins::page_op::PageMethod;
use al_builtins::record_op::RecordOp;
use rust_decimal::Decimal as D;

/// "The line cannot be split." (code TestWrapped, testpage/TestPageAutoSplitKeyMoreShapes.al:134-149).
const CANNOT_SPLIT: &str = "The line cannot be split.";

impl Session {
    /// The part reads its rows in another order than the primary key ascending.
    pub(in crate::pages) fn reads_in_view_order(rec: &Rec) -> bool {
        let r = rec.borrow();
        r.key != 0 || !r.ascending
    }

    /// The pinned scenario (PartSplitKeyOtherSortKey.al, pages 62613..62621 and codeunit 62622), every fact required; any
    /// other part read in another order is refused as before:
    /// - the pages (`part_pages_are`): a Card host with no trigger whose controls are fields of Rec and this part; a ListPart
    ///   that declares AutoSplitKey, with or without DelayedInsert, a SourceTableView that only sorts, and no trigger but
    ///   OnNewRecord and OnInsertRecord (:104, :111; they only log, so a part with fewer runs less code); controls over fields
    ///   of Rec without a trigger; no page extension, table extension or subscriber;
    /// - the link and the view: `SubPageLink = "Head Code" = field(Code)` (:211) on every primary key field but the last,
    ///   which is an Integer (:50); a SubPageView that only sorts (:233, :256, :278); no filter in any filter group but the
    ///   link's (no `where`, no filter the part's code or the test set), a stored table, no marks;
    /// - how the host opened: the test's OpenEdit, or the static `Page.RunModal` whose handler gets the page (:560);
    /// - the test's calls since then (`split_calls_pinned`).
    pub(in crate::pages) fn auto_split_part_pinned(&mut self, st: &St, _rec: &Rec) -> Result<bool, Signal> {
        const PAGES: PinnedPart = PinnedPart {
            host_type: ir::PageType::Card,
            host_triggers: &[],
            part_triggers: &["onnewrecord", "oninsertrecord"],
            part_properties: &["autosplitkey", "delayedinsert", "sourcetableview"],
            part_variable_controls: false,
            part_variable_validate: false,
            part_expression_controls: false,
            unlinked: false,
            sibling_parts: false,
            user_filter: false,
        };
        let by_test = match Self::part_host_opened(st) {
            Some((true, PageMode::Edit, false, false)) => true,
            Some((false, PageMode::Edit, true, true)) => false,
            _ => return Ok(false),
        };
        Ok(self.has_auto_split_key(st)
            && self.part_pages_are(st, &PAGES)?
            && self.part_key_is_link_then_integer(st)?
            && Self::split_calls_pinned(st, by_test))
    }

    /// The calls of the pinned tests on the host and the part, the running one last (reads of the part's fields stand between
    /// them): `GoToKey` on a host the test opened (:386, none in the modal handler :568); then no move (:479), `Last` (:387),
    /// `First` (:420) or `First` and a `Next` that moved (:454); then `New` and one or two values typed into the row (:394,
    /// :553), at most twice (:538 PkAsc_TwoNewRowsInARow); then the host's `Close` or its OK action (:571). Any other call, a
    /// call on the host between them, or a write to the store between two calls is not pinned.
    fn split_calls_pinned(st: &St, by_test: bool) -> bool {
        use PageMethod as M;
        use PartStep::{Host, Own};
        use TestCall::{Other, Page, Write};
        let Some(steps) = Self::part_steps(st) else {
            return false;
        };
        let mut s = &steps[..];
        s = match (by_test, s) {
            (true, [Host(Page(M::GoToKey)), rest @ ..]) => rest,
            (false, _) => s,
            _ => return false,
        };
        s = match s {
            [Own(Page(M::First)), Own(Page(M::Next)), rest @ ..] => rest,
            [Own(Page(M::First | M::Last)), rest @ ..] => rest,
            _ => s,
        };
        for _ in 0..2 {
            let [Own(Page(M::New)), rest @ ..] = s else {
                break;
            };
            let typed = rest.iter().take_while(|c| matches!(c, Own(Write(_)))).count();
            if typed == 0 {
                // The running `New`, or the one that raised "The line cannot be split.".
                return rest.is_empty();
            }
            if typed > 2 {
                return false;
            }
            s = &rest[typed..];
        }
        match s {
            [] => true,
            [Host(Page(M::Close))] => by_test,
            [Host(Page(M::OK))] | [Host(Page(M::OK)), Host(Other(M::Invoke))] | [Host(Other(M::Invoke))] => !by_test,
            _ => false,
        }
    }

    /// `New()` below a row whose next row in the grid has a lower number raises "The line cannot be split." before the line
    /// starts (PartSplitKeyOtherSortKey.al:643 SeqAsc_NewAfterFirst, :694 SeqAsc_NewWithNoMove, :741, :759 in a modal handler,
    /// :853 SeqDesc_NewAfterFirstAndNext, :1024 and :1043 PkDesc, :1185 OwnSeq, :1361 SeqDelayed).
    pub(in crate::pages) fn split_view_check_at_new(&mut self, st: &St) -> Result<(), Signal> {
        let rec = self.page_rec(st);
        if !Self::reads_in_view_order(&rec) || !self.has_auto_split_key(st) || !self.auto_split_part_pinned(st, &rec)? {
            return Ok(());
        }
        let Some(lower) = st.borrow().lower_key.clone() else {
            return Ok(());
        };
        let table = self.table(&rec)?;
        let pos = table.keys[0].len() - 1;
        let (Some(l), Some(u)) = (key_number(&lower[pos]), self.split_view_upper(st, &rec, &lower)?) else {
            return Ok(());
        };
        if u < l {
            return Err(error_code(CANNOT_SPLIT, "TestWrapped"));
        }
        Ok(())
    }

    pub(in crate::pages) fn has_auto_split_key(&self, st: &St) -> bool {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        engine.program.objects[object]
            .properties
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("AutoSplitKey") && is_true(v))
    }

    /// The number of the row the grid shows after `lower`: the next row of the part's view, or, for the row this part numbered
    /// last, the row it was numbered below of (the grid keeps a new row where it was started: PartSplitKeyOtherSortKey.al:920
    /// SeqDesc_TwoNewRowsInARow, the second line below 35000 is 37500, not after the last row of the view).
    pub(in crate::pages) fn split_view_upper(
        &mut self,
        st: &St,
        rec: &Rec,
        lower: &[Value],
    ) -> Result<Option<D>, Signal> {
        if let Some((key, upper)) = st.borrow().split_view_last.clone()
            && Self::same_key(&key, lower)
        {
            return Ok(upper.as_ref().and_then(key_number));
        }
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        let mut args = lower.to_vec();
        self.ret_used = true;
        if !matches!(self.record_op(RecordOp::Get, &probe, &mut args)?, Value::Boolean(true)) {
            return Ok(None);
        }
        self.ret_used = true;
        if !matches!(self.record_op(RecordOp::Next, &probe, &mut [Value::Integer(1)])?, Value::Integer(n) if n != 0) {
            return Ok(None);
        }
        let table = self.table(rec)?;
        let last = *table.keys[0].last().expect("a primary key");
        Ok(key_number(&probe.borrow().fields[last]))
    }

    /// `number` when no stored row of the same leading key holds it; else the number between that row and the next stored row
    /// in primary key order (PartSplitKeyOtherSortKey.al:599 SeqAsc_NewAfterLast: below 20000 with no row after it, 30000 is
    /// taken, the line is 35000; :650 SeqAsc_NewAfterFirstAndNext: between 10000 and 40000, 20000 is taken, 25000; :808
    /// SeqDesc_NewAfterFirst 35000; :775 SeqDesc_NewAfterLast: 40000 is taken and no row follows, 50000; :980 PkDesc_NewAfterLast
    /// 25000). No whole number between that row and the next one is not pinned: refused.
    pub(in crate::pages) fn split_view_free(&mut self, rec: &Rec, number: D) -> Result<D, Signal> {
        let Some(after) = self.split_stored_after(rec, number, true)? else {
            return Ok(number);
        };
        let Some(taken) = after.filter(|a| *a == number) else {
            return Ok(number);
        };
        let next = self.split_stored_after(rec, taken, false)?.flatten();
        let step = D::from(10000);
        let free = match next {
            None => taken + step,
            Some(u) => {
                let half = ((u - taken) / D::from(2)).trunc();
                if half < D::ONE {
                    return unsupported(
                        "AutoSplitKey of a part read in another order whose number is taken and no number is free after it (not pinned)",
                    );
                }
                taken + half.min(step)
            }
        };
        Ok(free)
    }

    /// The number `auto_split_key` gives a line of a part read in another order, from the numbers of `lower` and `upper`. Pinned:
    /// an empty part (20000, PartSplitKeyOtherSortKey.al:701 SeqAsc_NewOnEmptyPart, :890, :1067, :1243, :1419), a line after
    /// the last row or between two rows with numbers that are not negative (see `split_view_free`). A line started from the
    /// placeholder line or typed on it, and negative numbers, are not pinned in another order: refused.
    pub(in crate::pages) fn split_in_view_number(
        &mut self,
        st: &St,
        rec: &Rec,
        ty: &RtType,
        lower: Option<D>,
        upper: Option<D>,
        from_placeholder: bool,
    ) -> Result<(), Signal> {
        let step = D::from(10000);
        let number = match (lower, upper) {
            _ if from_placeholder => None,
            (None, _) => Some(self.split_view_free(rec, D::from(20000))?),
            (Some(l), None) if l >= D::ZERO => Some(self.split_view_free(rec, l + step)?),
            (Some(l), Some(u)) if l >= D::ZERO && u > l => {
                let half = ((u - l) / D::from(2)).trunc();
                if half < D::ONE {
                    return Err(error_code(CANNOT_SPLIT, "TestWrapped"));
                }
                Some(self.split_view_free(rec, l + half.min(step))?)
            }
            _ => None,
        };
        let Some(number) = number else {
            return unsupported(
                "AutoSplitKey of a part read in another order, started from the placeholder line or next to a negative number (not pinned)",
            );
        };
        let table = self.table(rec)?;
        let last = *table.keys[0].last().expect("a primary key");
        rec.borrow_mut().fields[last] = ops::convert(Value::Decimal(number), ty)?;
        let key = table.primary_key(&rec.borrow().fields);
        st.borrow_mut().split_view_last = Some((key, upper.map(Value::Decimal)));
        Ok(())
    }

    /// The last row of the view of an AutoSplitKey part read in another order, read again: a line the part inserted goes to its
    /// place in the view, the last one too (PartSplitKeyOtherSortKey.al:920 SeqDesc_TwoNewRowsInARow `INS 35000 ... | NEW 0
    /// x=35000`: Seq No. 0 sorts last in descending order). `None` for any other part (`part_new_xrec` keeps the window's last
    /// row).
    pub(in crate::pages) fn split_view_last_row(&mut self, st: &St) -> Result<Option<Vec<Value>>, Signal> {
        let rec = self.page_rec(st);
        if !Self::reads_in_view_order(&rec) || !self.has_auto_split_key(st) || !self.auto_split_part_pinned(st, &rec)? {
            return Ok(None);
        }
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        self.ret_used = true;
        if matches!(
            self.record_op(RecordOp::FindLast, &probe, &mut [])?,
            Value::Boolean(true)
        ) {
            Ok(Some(probe.borrow().fields.clone()))
        } else {
            Ok(None)
        }
    }

    /// The number of the first stored row of the part's leading key at `from` (`at_or_after`) or after it, in primary key order.
    /// `Ok(None)` when the key holds no number field; `Ok(Some(None))` when no such row exists.
    fn split_stored_after(&mut self, rec: &Rec, from: D, at_or_after: bool) -> Result<Option<Option<D>>, Signal> {
        let table = self.table(rec)?;
        let Some((&last, leading)) = table.keys[0].split_last() else {
            return Ok(None);
        };
        let ty = table.fields[last].ty.clone();
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        {
            let fields = rec.borrow().fields.clone();
            let mut p = probe.borrow_mut();
            p.key = 0;
            p.ascending = true;
            p.filters.clear();
            let mut range = |field: usize, expr| {
                p.filters.push(al_builtins::filter::FieldFilter {
                    lang: al_builtins::filter::filter_language(),
                    field,
                    expr,
                    group: 99,
                })
            };
            for &k in leading {
                range(
                    k,
                    al_builtins::filter::FilterExpr::Range(Some(fields[k].clone()), Some(fields[k].clone())),
                );
            }
            let v = ops::convert(Value::Decimal(from), &ty)?;
            range(
                last,
                if at_or_after {
                    al_builtins::filter::FilterExpr::Range(Some(v), None)
                } else {
                    al_builtins::filter::FilterExpr::Gt(v)
                },
            );
        }
        self.ret_used = true;
        if matches!(
            self.record_op(RecordOp::FindFirst, &probe, &mut [])?,
            Value::Boolean(true)
        ) {
            Ok(Some(key_number(&probe.borrow().fields[last])))
        } else {
            Ok(Some(None))
        }
    }
}

fn key_number(v: &Value) -> Option<D> {
    match v {
        Value::Integer(i) => Some(D::from(*i)),
        Value::BigInteger(i) => Some(D::from(*i)),
        Value::Decimal(d) => Some(*d),
        _ => None,
    }
}
