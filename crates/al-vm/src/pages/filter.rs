//! `TestPage.Filter` / `TestPart.Filter` (the `TestFilter` handle): the test's way to change which
//! rows an open page has and in what order. It writes the page's user filter, filter group 0
//! (corpus testfilter/TestTestFilter.al, TestTestFilterFilterGroups.al).

use super::*;
use al_builtins::describe::Describe;
use al_builtins::filter::FieldFilter;
use al_builtins::page_op::PageMethod;

impl Session {
    pub(super) fn filter_handle(&self, st: &St) -> Value {
        Value::TestControl(Rc::new(ControlRef {
            page: st.clone(),
            target: ControlTarget::Filter,
        }))
    }

    /// The `TestFilter.*` methods on the page `st`.
    pub(super) fn test_filter_op(&mut self, method: PageMethod, st: &St, args: &mut [Value]) -> Result<Value, Signal> {
        if !self.has_source(st) {
            return unsupported("TestFilter on a page without a source table");
        }
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        match method {
            // The first filter on the field across the filter groups, in the order they were
            // written (corpus testfilter/TestTestFilterFilterGroups.al:3 and 4).
            PageMethod::GetFilter => {
                let pos = field_arg(args, 0)?;
                let first = rec.borrow().filters.iter().find(|f| f.field == pos).cloned();
                if st.borrow().unpinned_filter_fields.contains(&pos) {
                    return unsupported(
                        "TestFilter.GetFilter of a SourceTableView const(0D) or filter(0D) (the filter text is not pinned)",
                    );
                }
                if st.borrow().or_group_dropped {
                    return unsupported(
                        "TestFilter.GetFilter on a page whose OnOpenPage left FilterGroup(-1) filters (not pinned)",
                    );
                }
                use al_builtins::filter::FilterExpr;
                use al_builtins::value::RtType;
                match (&table.fields[pos].ty, first.as_ref().map(|f| &f.expr)) {
                    // A typed single value on an Enum field (SetRange on the record handed to the page, a RunPageLink `field()`, a
                    // SourceTableView `const`) reads as the ordinal; a filter given as text keeps the names (pages/lt-ce-page-run/
                    // CePageRun.al SetTableView_ConflictingFilterOnTheSameField `Kind=1`, SetTableView_FilterOnAnotherField `Kind=0`,
                    // SetTableView_WiderFilterOnTheSameField `Kind=A|B`; pages/lt-cd-testpage-leads/CdTestPageLeads.al:626
                    // Item61_RunPageLinkEnumKey_ByField_New `2`, :650 _ByConst_New `Second`).
                    (RtType::Enum(_), Some(FilterExpr::Range(Some(a), Some(b)))) => {
                        let (Value::Enum(_, x), Value::Enum(_, y)) = (a, b) else {
                            return unsupported("TestFilter.GetFilter of an Enum range that holds no Enum value");
                        };
                        if x != y {
                            return unsupported("TestFilter.GetFilter of an Enum range of two values (not pinned)");
                        }
                        return Ok(Value::Text(x.to_string().into()));
                    }
                    // A Boolean that a SourceTableView (group 2) or an action's RunPageView (group 3) filters with `const(true)`
                    // reads `1` (pages/lt-rz-outside-page-moves/RzRunPageLinkView.al:62 RZ_Link_WhereView_I1, :112
                    // RZ_LinkAndRunPageViewWhere_DescendingView_I1). `const(false)` there is not pinned.
                    (RtType::Boolean, Some(e @ (FilterExpr::Range(Some(_), Some(_)) | FilterExpr::Eq(_))))
                        if matches!(first.as_ref().map(|f| f.group), Some(2 | 3)) =>
                    {
                        let ends = match e {
                            FilterExpr::Range(Some(a), Some(b)) => (a, b),
                            FilterExpr::Eq(a) => (a, a),
                            _ => unreachable!("matched above"),
                        };
                        return match ends {
                            (Value::Boolean(true), Value::Boolean(true)) => Ok(Value::Text("1".to_string().into())),
                            _ => unsupported(
                                "TestFilter.GetFilter of a Boolean that a page or action view filters to false (not pinned)",
                            ),
                        };
                    }
                    (RtType::Option(_), Some(FilterExpr::Range(Some(_), Some(_)))) => {
                        return unsupported("TestFilter.GetFilter of an Option field filtered by a value (not pinned)");
                    }
                    _ => {}
                }
                match first {
                    Some(f) => Ok(Value::Text(self.filter_text(&f.expr)?.into())),
                    None => Ok(Value::Text(String::new().into())),
                }
            }
            PageMethod::SetFilter => {
                let pos = field_arg(args, 0)?;
                let text = match args.get(1) {
                    Some(Value::Text(t) | Value::Code(t)) => t.clone(),
                    other => return unsupported(format!("TestFilter.SetFilter with {}", other.describe())),
                };
                let ty = table.fields[pos].ty.clone();
                // `Filter.SetFilter(Active, 'true')` and `'false'` on a Boolean field change nothing the handler of a modal list sees: the rows
                // stay as the record handed to RunModal left them (pages/lt-modal-list-new-row/ModalListNewRowKeyFilters.al:
                // P157_HandlerSetFilter_NoRecFilters, _WithTwoKeyFilters, _MatchesNothing, _NonKeyRecFilter). Whether it is the Boolean
                // words or the modal list that matters is not probed (docs/probes-needed.md).
                if matches!(ty, al_builtins::value::RtType::Boolean)
                    && matches!(text.trim().to_ascii_lowercase().as_str(), "true" | "false")
                {
                    return Ok(Value::Null);
                }
                // An empty text takes the user's filter off the field (Record.SetFilter and FieldRef.SetFilter do:
                // errors/lt-ms-refusals-134/P134DateTextFilters.al:P134_FieldRef_Date_EmptyAndBlankForms); for a page filter
                // that is an assumption.
                if text.trim().is_empty() {
                    al_builtins::assumptions::assume(
                        al_builtins::assumptions::Assumption::TestFilterSetFilterEmptyClears,
                    );
                    rec.borrow_mut().filters.retain(|f| !(f.field == pos && f.group == 0));
                    self.filter_changed(st)?;
                    return Ok(Value::Null);
                }
                let expr = self
                    .parse_filter_text(&text, &ty)
                    .map_err(|e| Signal::Unsupported(format!("TestFilter.SetFilter: {e}")))?;
                // The user filter is group 0, whatever group OnOpenPage left active; setting a
                // field's filter again replaces the group-0 one.
                {
                    // In place when there is one: group 0 keeps its place ahead of the groups the
                    // page wrote after it (corpus testfilter/TestTestFilterFilterGroups.al:4).
                    let mut r = rec.borrow_mut();
                    match r.filters.iter().position(|f| f.field == pos && f.group == 0) {
                        Some(i) => r.filters[i].expr = expr,
                        None => r.filters.push(FieldFilter {
                            field: pos,
                            expr,
                            group: 0,
                        }),
                    }
                }
                self.filter_changed(st)?;
                Ok(Value::Null)
            }
            PageMethod::CurrentKey => {
                let key = rec.borrow().key;
                let names: Vec<&str> = table.keys[key].iter().map(|&p| table.fields[p].name.as_str()).collect();
                Ok(Value::Text(names.join(",").into()))
            }
            PageMethod::SetCurrentKey => {
                let mut wanted = Vec::new();
                for i in 0..args.len() {
                    wanted.push(field_arg(args, i)?);
                }
                let Some(key) = table
                    .keys
                    .iter()
                    .position(|k| k.len() >= wanted.len() && k[..wanted.len()] == wanted[..])
                else {
                    return unsupported("TestFilter.SetCurrentKey without a matching key (error text not pinned)");
                };
                rec.borrow_mut().key = key;
                self.filter_changed(st)?;
                Ok(Value::Null)
            }
            PageMethod::Ascending => match args.first() {
                Some(v) => {
                    rec.borrow_mut().ascending = ops::truthy(v)?;
                    self.filter_changed(st)?;
                    Ok(Value::Null)
                }
                None => Ok(Value::Boolean(rec.borrow().ascending)),
            },
            other => unsupported(format!("TestFilter.{other}")),
        }
    }

    /// The page reads its rows again after the filter or the key changed, and stands on the first
    /// match (corpus handlers/TestPageFilterPosition_Tests.al:SetFilter_ExcludingTheCurrentRow_*,
    /// SetFilter_EvenWhenCurrentRowStillQualifies_RepositionsToTheFirstMatch).
    fn filter_changed(&mut self, st: &St) -> Result<(), Signal> {
        self.save_row(st)?;
        {
            let mut s = st.borrow_mut();
            s.cursor = Cursor::None;
            s.draft = false;
            s.dirty = false;
            s.window = Vec::new();
            s.focus = None;
            s.focus_in_row = false;
        }
        let r = self.page_first_row(st);
        self.torn_down_on_error(st, r)?;
        // A page the filter left without a row shows a blank one, its key pre-filled from a
        // single-valued filter (corpus pageupdate/TestPageUpdateGoneRow.al:
        // Card_OpenEdit_FilterMatchesNothing_NoOpAction_StaysOpen).
        if st.borrow().cursor == Cursor::None {
            // A list that offers the blank line stands on it (a page opened with OpenNew starts a new row), with the filter
            // value on it (pages/lt-filter-change-new-line/FilterChangeNewLine.al: OpenNew_*, OpenEdit_EmptyTable_*,
            // OpenEdit_FilterToEmptyResult_*, OpenEdit_FilterOnNonKeyField_*).
            let new_mode = st.borrow().mode == PageMode::New && self.has_source(st);
            if new_mode || (st.borrow().new_line && self.is_list(st)) {
                if new_mode {
                    self.start_draft(st, false)?;
                    // The blank line of a list is read twice here too (pages/lt-filter-change-new-line/FilterChangeNewLine.al:
                    // OpenNew_FilterSetFilter_ThenSetValueDescr_StoresRowWithFilterKey: `NewRecord(G1);AGCR(G1,0)` twice).
                    if self.is_list(st) && st.borrow().host.is_none() {
                        self.start_draft(st, false)?;
                    }
                    // No control has the focus yet, as on the new line an OpenEdit list lands on: a SetValue on a non-key field does not
                    // insert the row (the same test: `log_after_setvalue` has no InsertRecord, the row is stored when the page closes).
                    let mut s = st.borrow_mut();
                    s.focus = None;
                    s.focus_in_row = false;
                } else {
                    self.land_on_new_line(st)?;
                }
            } else {
                let rec = self.page_rec(st);
                let table = rec.borrow().table as usize;
                let fresh = self.initial_fields(table);
                rec.borrow_mut().fields = fresh;
                self.stamp_from_filters(st, false)?;
            }
        }
        if st.borrow().host.is_some() {
            self.sync_dependents(st)
        } else {
            self.sync_parts(st)
        }
    }
}

fn field_arg(args: &[Value], i: usize) -> Result<usize, Signal> {
    match args.get(i) {
        Some(Value::Integer(p)) => Ok(*p as usize),
        other => unsupported(format!("TestFilter field argument {}", other.describe())),
    }
}
