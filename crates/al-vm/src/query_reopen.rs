//! A second `Open` on a query that was read from while rows were left, with no filter change between (al-runner issue 7827).
//!
//! Pinned by queries/read-lifecycle/QueryClosedAfterLastRow.al:162-182 (Query_RowsLeft_OpenAgainAndSetRange), and only in
//! its shape. The query (:10-22): one dataitem and two columns, an Integer and a Code[10] over stored fields, no property but
//! `QueryType = Normal`, no trigger, procedure or variable. The variable: a global of the codeunit, cleared (:31, :42). The
//! calls, all through that global and with no commit between: `Open` as a statement (:165, :48), one `Read` whose result is
//! used and one read of the first column (:166, :69-70), the second `Open` as a statement while rows are left and nothing
//! was written (:168). That `Open` raises nothing (:175). Then one `SetRange` with one value on the Code column (:169, :54)
//! and `Open` as a statement (:170) read the rows of the new filter (:171, :179). Each call moves the variable one
//! `ReopenStep` on. A call that is not the next step before the second `Open` leaves the sequence: it and every later call
//! take the usual path. After the second `Open` such a call refuses, until the pinned `Open` ends the sequence.

use crate::Session;
use al_builtins::native::{Signal, unsupported};
use al_builtins::query_reopen_step::{ReopenStep, ReopenTrack};
use al_builtins::value::{QueryRun, QueryState, RtType, Value};
use al_ir as ir;
use std::cell::RefCell;
use std::rc::Rc;

type Qs = Rc<RefCell<QueryState>>;

/// One call on a query variable.
pub(crate) enum ReopenCall<'a> {
    /// A method, with its arguments (a column is its position).
    Method(&'a str, &'a [Value]),
    /// A read of the column at this position.
    Column(usize),
}

impl Session {
    /// Moves the variable one step on before the call runs. `global` is the global variable the call goes through. Returns
    /// true when the call was the pinned second `Open` (it raised nothing, and nothing else runs).
    pub(crate) fn query_reopen_step(
        &mut self,
        def: &ir::Query,
        qs: &Qs,
        call: ReopenCall<'_>,
        global: Option<(usize, usize)>,
    ) -> Result<bool, Signal> {
        let step = std::mem::replace(&mut qs.borrow_mut().reopen, ReopenStep::Off);
        let open = matches!(call, ReopenCall::Method("Open", _)) && !self.ret_used;
        let same = |t: &ReopenTrack, s: &Self| global == Some(t.global) && t.commits == s.commits;
        let next = match (step, &call) {
            (ReopenStep::Fresh, _) if open => match global {
                Some(g) => ReopenStep::Opened(ReopenTrack {
                    global: g,
                    commits: self.commits,
                    stamps: Vec::new(),
                }),
                None => ReopenStep::Off,
            },
            (ReopenStep::Opened(t), ReopenCall::Method("Read", _)) if self.ret_used && same(&t, self) => {
                ReopenStep::ReadOne(t)
            }
            (ReopenStep::ReadOne(t), ReopenCall::Column(0)) if same(&t, self) => ReopenStep::ColumnRead(t),
            (ReopenStep::ColumnRead(mut t), _) if open && same(&t, self) => {
                match self.query_reopen_rows_left(def, qs, &t) {
                    Some(stamps) => {
                        t.stamps = stamps;
                        let mut q = qs.borrow_mut();
                        q.run = Some(QueryRun::opened());
                        q.reopen = ReopenStep::Reopened(t);
                        return Ok(true);
                    }
                    None => ReopenStep::Off,
                }
            }
            (ReopenStep::Reopened(t), ReopenCall::Method("SetRange", [Value::Integer(1), Value::Text(_)]))
                if same(&t, self) && self.query_stamps(def) == t.stamps =>
            {
                ReopenStep::Ranged(t)
            }
            (ReopenStep::Ranged(t), _) if open && same(&t, self) && self.query_stamps(def) == t.stamps => {
                ReopenStep::Off
            }
            (step @ (ReopenStep::Reopened(_) | ReopenStep::Ranged(_)), _) => {
                qs.borrow_mut().reopen = step;
                let what = match call {
                    ReopenCall::Method(m, _) => format!("Query.{m}"),
                    ReopenCall::Column(_) => "a column read".to_string(),
                };
                return unsupported(format!(
                    "{what} on a query opened again while rows were left, with no filter change (not pinned)"
                ));
            }
            _ => ReopenStep::Off,
        };
        qs.borrow_mut().reopen = next;
        Ok(false)
    }

    /// Whether the query and its run are the pinned ones at the second `Open` (the module comment); the versions of the
    /// query's tables the rows were read at, when they are.
    fn query_reopen_rows_left(&self, def: &ir::Query, qs: &Qs, t: &ReopenTrack) -> Option<Vec<(u32, u64)>> {
        let engine = self.engine.clone();
        let objects = &engine.program.objects;
        let query = &objects[qs.borrow().object as usize];
        // A global of a codeunit (:29-31).
        let codeunit = objects[t.global.0].kind == ir::ObjKind::Codeunit;
        // No trigger, procedure or variable; only `QueryType = Normal` (:10-13).
        let plain_object = query.routines.is_empty()
            && query.globals.len() == 1
            && query.rec_slot == Some(0)
            && matches!(&query.properties[..], [(n, v)] if n.eq_ignore_ascii_case("QueryType") && v.trim().eq_ignore_ascii_case("Normal"));
        // One dataitem with no property (:16).
        let item = match &def.items[..] {
            [item] if item.parent.is_none() && item.properties.is_empty() => item,
            _ => return None,
        };
        let fields = &objects[item.table].table.as_ref()?.fields;
        // Exactly `column(Id; Id)` over an Integer and `column(Grp; Grp)` over a Code[10], stored fields, no property
        // (:18-19; the field types: _fixtures/tables/ALTShapeTables.al:266-267).
        let column = |c: &ir::QueryColumn, ty: &RtType| {
            c.item == 0
                && !c.filter_element
                && c.properties.is_empty()
                && c.ty == *ty
                && c.field.is_some_and(|f| fields[f].stored)
        };
        let columns = matches!(&def.columns[..], [id, grp] if column(id, &RtType::Integer) && column(grp, &RtType::Code(Some(10))));
        let now = self.query_stamps(def);
        let q = qs.borrow();
        // Read once, rows left, nothing written since (:166, :174).
        let run = q.run.as_ref().filter(|r| {
            r.started
                && !r.filters_changed
                && r.returned == 1
                && r.current.is_some()
                && r.next < r.rows.len()
                && r.stamps == now
        })?;
        (codeunit && plain_object && columns).then(|| run.stamps.clone())
    }
}
