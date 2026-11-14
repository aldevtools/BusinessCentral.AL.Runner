//! A second `Open` on a query that was read from while rows were left, with no filter change between (al-runner issue 7827).
//!
//! Pinned by queries/read-lifecycle/QueryClosedAfterLastRow.al:162-182 (Query_RowsLeft_OpenAgainAndSetRange): a query of one
//! data item with two plain columns (:10-22), no filter set (`Seed` clears it, :42), opened and read once with rows left
//! (:165-167); `Open` as a statement raises nothing (:168, :178); then `SetRange` on a column with one value (:169) and `Open`
//! (:170) read the rows of the new filter (:171, :182). What any other call does right after the second `Open` (a `Read`, a
//! column read, another `Open`, `SetFilter`, `Close`) is not pinned and refuses, as does every other shape of the second `Open`.

use crate::Session;
use al_builtins::native::{Signal, unsupported};
use al_builtins::value::{QueryRun, QueryState};
use al_ir as ir;
use std::cell::RefCell;
use std::rc::Rc;

type Qs = Rc<RefCell<QueryState>>;

impl Session {
    /// Whether this `Open` of a started query is the pinned second `Open` (the module comment). Each fact is a line of the pin.
    pub(crate) fn query_reopen_rows_left_pinned(&self, def: &ir::Query, object: usize, qs: &Qs) -> bool {
        let engine = self.engine.clone();
        let no_on_before_open = !engine.program.objects[object]
            .routines
            .iter()
            .any(|r| r.is_trigger && r.name.eq_ignore_ascii_case("OnBeforeOpen"));
        // One data item without a table filter, plain columns, no OrderBy and no top (QueryClosedAfterLastRow.al:10-22).
        let plain_query = matches!(&def.items[..], [item] if item.parent.is_none() && item.table_filter.is_empty())
            && def.columns.iter().all(|c| {
                c.method.is_none()
                    && !c.filter_element
                    && !c.reverse_sign
                    && c.static_filter.is_none()
                    && !c.flow_below_join
            })
            && def.order_by.is_empty()
            && def.top == 0;
        let now = self.query_stamps(def);
        let q = qs.borrow();
        // No filter, top or security filtering set (:42); read once with rows left and nothing written since (:165-167).
        let state = q.filters.is_empty()
            && q.top.is_none()
            && q.security_filtering.is_none()
            && q.run.as_ref().is_some_and(|r| {
                r.started
                    && !r.filters_changed
                    && !r.reopened_rows_left
                    && r.returned == 1
                    && r.current.is_some()
                    && r.next < r.rows.len()
                    && r.stamps == now
            });
        // `Open` as a statement (:48, TryOpen).
        no_on_before_open && plain_query && state && !self.ret_used
    }

    /// The pinned second `Open`: it raises nothing and leaves the query in the state that lets only the pinned next calls through.
    pub(crate) fn query_reopen_rows_left(qs: &Qs) {
        qs.borrow_mut().run = Some(QueryRun {
            started: false,
            filters_changed: false,
            rows: Vec::new(),
            next: 0,
            current: None,
            last_key: None,
            returned: 0,
            stamps: Vec::new(),
            reopened_rows_left: true,
        });
    }

    /// After the pinned second `Open`, only `SetRange` on a column with one value (:169), and then `Open` (:170), are pinned.
    pub(crate) fn query_reopen_gate(qs: &Qs, method: &str, args: &[al_builtins::value::Value]) -> Result<(), Signal> {
        let Some(run) = qs.borrow().run.as_ref().filter(|r| r.reopened_rows_left).cloned() else {
            return Ok(());
        };
        let pinned = (method == "SetRange" && args.len() == 2) || (method == "Open" && run.filters_changed);
        if pinned {
            return Ok(());
        }
        unsupported(format!(
            "Query.{method} on a query opened again while rows were left, with no filter change (not pinned)"
        ))
    }

    /// A column read right after the pinned second `Open` is not pinned.
    pub(crate) fn query_reopen_column_gate(qs: &QueryState) -> Result<(), Signal> {
        if qs.run.as_ref().is_some_and(|r| r.reopened_rows_left) {
            return unsupported(
                "a column read on a query opened again while rows were left, with no filter change (not pinned)",
            );
        }
        Ok(())
    }
}
