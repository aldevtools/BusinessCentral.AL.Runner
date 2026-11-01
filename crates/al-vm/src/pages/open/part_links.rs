use super::*;
use al_builtins::record_op::RecordOp;

impl Session {
    /// The blank line of a part that shows no row of an editable host, as the host opens or the part is linked afresh: the
    /// client runs the part's OnNewRecord three times, the link field validated before the first and the third
    /// (pages/lt-part-line-before-host-key/PartLineBeforeHostKey.al:672 PYB_OpenNew_Document_LogOnly `L.OnValidate(Document No.): |
    /// LP.OnNewRecord: | LP.OnNewRecord: | L.OnValidate(Document No.): | LP.OnNewRecord:`, :688 a host without OnNewRecord, :704
    /// an unlinked part (three runs, no validation), :738 without AutoSplitKey, :754 two linked parts one after the other, :770 a
    /// Card host, :809 OpenEdit of a host without lines, :603 after the host key is typed). OnAfterGetCurrRecord runs right
    /// after each OnNewRecord run (:1349 QH_EmptyPart_OpenNew_CurrRecordOrder `CP.OnNewRecord: below=Yes |
    /// CP.OnAfterGetCurrRecord:/0`, three times; :1381 OpenEdit, :1418 after the host key is typed, :1495 host Next; :1439 and
    /// :1455 a part that declares only OnAfterGetCurrRecord runs it three times). A host's New() runs only the host's
    /// OnNewRecord, never the part's (:827 PYB_OpenEdit_HostWithoutLines `host_new`, :879 PYB_OpenEdit_ThenHostNew, :861
    /// PYB_OpenNew_ThenHostNew); whether its OnAfterGetCurrRecord runs then is not pinned, so a part that can observe it is refused.
    pub(super) fn land_on_part_open_line(&mut self, st: &St) -> Result<(), Signal> {
        let rec = self.page_rec(st);
        let table = rec.borrow().table as usize;
        self.note_lower_key(st)?;
        let fresh = self.initial_fields(table);
        rec.borrow_mut().fields = fresh;
        {
            let mut s = st.borrow_mut();
            s.placeholder_line = true;
            s.split_burned = false;
            s.cursor = Cursor::NewLine;
            s.draft = true;
            s.typed_before_key = false;
        }
        let object = st.borrow().object as usize;
        let observes_curr = self.declares(st, "onaftergetcurrrecord")
            || self.engine().events.is_subscribed(object, "onaftergetcurrrecordevent");
        if self.host_new_running {
            // Whether and how often the part's OnAfterGetCurrRecord runs then is not pinned (the PYB pages declare none).
            if observes_curr {
                return unsupported(
                    "OnAfterGetCurrRecord of an empty part while its host runs New() (whether and how often BC runs it is not pinned)",
                );
            }
            self.set_xrec(st);
            return Ok(());
        }
        // A link value the part's table relation refuses: the client stamps it without validating (no OnBeforeValidate, no
        // OnValidate), runs the part's OnNewRecord and OnAfterGetCurrRecord once, then the open fails with code TestWrapped and the
        // relation's text (PartLineBeforeHostKey.al:1600-1680 QH_ConstLink_Missing_OpenNew_Trace `[CP.OnOpenPage |
        // CP.OnNewRecord:D9 below=Yes | CP.OnAfterGetCurrRecord:D9/0]`, QH_ConstLink_Missing_OpenNew_Events `[LP.OnNewRecord:D9]`,
        // QH_ConstLink_Missing_OpenEdit; :776 PYB_OpenNew_ConstLinkedPart).
        if let Some(e) = self.part_link_relation_error(st)? {
            self.stamp_from_filters(st, false)?;
            self.page_trigger(st, "onnewrecord", vec![Value::Boolean(true)])?;
            self.page_event(st, "onnewrecordevent", None, Some(true))?;
            self.set_xrec(st);
            self.page_trigger(st, "onaftergetcurrrecord", vec![])?;
            self.page_event(st, "onaftergetcurrrecordevent", None, None)?;
            return Err(error_code(e.message.clone(), "TestWrapped"));
        }
        // OnNewRecord sees the xRec the part held before its first and third run, a blank one before the second, after any move
        // of the host: GoToKey (pages/lt-qj-part-propagation/QjPartPropagation.al QL_PartLastNew_NoLines `log-open`: `S.NEW(x=3
        // x.ln=30000) S.AGCR(0 r=0 x=0) S.NEW(x=0 x.ln=0) S.AGCR(0 r=0 x=0) S.NEW(x=3 x.ln=30000)`), Next (:2468 QN_EmptyPart_HostNext),
        // Previous (:2499), First (:2530), Last (:2561); a part that held no row before (OpenNew :2591 QN_EmptyPart_HostOpenNew,
        // OpenEdit onto an empty first host :2441) sees a blank xRec all three times. The part's Last() and New() see it again
        // (`PageState::empty_part_xrec`).
        let held = self.page_xrec_fields(st);
        st.borrow_mut().empty_part_xrec = Some(held.clone());
        // A value typed on this blank line without New() is stored with the first AutoSplitKey number, 10000
        // (pages/lt-new-line-stamps-filters/NewLineStampsFilters.al QP_TwoKeyLink_TypeFirst_BlankLine `db.first =
        // Order|H1|10000|...`); a line New() starts from it burns one interval (20000: QP_TwoKeyLink_TypeFirst_NewLine,
        // PartLineBeforeHostKey.al:900 PartLine_EnteredAfterHostKey_Baseline). `auto_split_key` reads it on the placeholder line only.
        st.borrow_mut().split_starts_low = true;
        let blank_held = self.blank_held_xrec(st, &held);
        let table_blank = self.initial_fields(table);
        for (n, validate) in [true, false, true].into_iter().enumerate() {
            let first = n == 0;
            if validate {
                self.set_xrec_fields(st, held.clone());
            } else {
                // The second run sees the table's blank record as xRec, not the line as stamped (QjPartPropagation.al:3501
                // QT_ValPartSplit_Empty_Last `log-openedit`: the second `V.NEW(H1 0 x= 0 below=Yes)`).
                self.set_xrec_fields(st, table_blank.clone());
            }
            if validate {
                // Each run starts from a fresh record: an OnValidate of a link field that appends to another field leaves
                // one entry, not two (pages/lt-new-line-stamps-filters/NewLineStampsFilters.al QP_TwoKeyLink_TypeFirst_BlankLine
                // `draft = Order|H1|type:no=[H1];no:type=[Order];` after the host's GoToKey).
                let rec = self.page_rec(st);
                let table = rec.borrow().table as usize;
                let fresh = self.initial_fields(table);
                rec.borrow_mut().fields = fresh;
                let stamped = match blank_held.clone() {
                    Some(before) => self.stamp_from_filters_seen_as(st, before),
                    None => self.stamp_from_filters(st, true),
                };
                if let Err(Signal::Error(e)) = stamped {
                    return self.part_open_link_validate_failed(st, e, first);
                }
                stamped?;
            }
            self.page_trigger(st, "onnewrecord", vec![Value::Boolean(true)])?;
            self.page_event(st, "onnewrecordevent", None, Some(true))?;
            self.set_xrec(st);
            self.page_trigger(st, "onaftergetcurrrecord", vec![])?;
            self.page_event(st, "onaftergetcurrrecordevent", None, None)?;
        }
        Ok(())
    }

    /// The error of the table relation of the first link value `stamp_from_filters` would validate on the part's new line, when
    /// the related table lacks it; the line is left as it was.
    pub(super) fn part_link_relation_error(
        &mut self,
        st: &St,
    ) -> Result<Option<Box<al_builtins::native::AlError>>, Signal> {
        // Pinned for a const link (`SubPageLink = "Document No." = const(D9)`); a field link keeps the validation in its turn.
        let Some((host, idx)) = st
            .borrow()
            .host
            .as_ref()
            .and_then(|(h, c)| h.upgrade().map(|h| (h, *c)))
        else {
            return Ok(None);
        };
        let const_link = page_of(&self.engine(), &host).controls[idx]
            .part_link
            .as_ref()
            .is_some_and(|l| !l.entries.is_empty() && l.entries.iter().all(|e| matches!(e, ir::LinkEntry::Filter(..))));
        if !const_link || !self.has_source(st) {
            return Ok(None);
        }
        let rec = self.page_rec(st);
        let before = rec.borrow().fields.clone();
        self.stamp_from_filters(st, false)?;
        let stamped = rec.borrow().fields.clone();
        rec.borrow_mut().fields = before.clone();
        let table = self.table(&rec)?;
        for (pos, (was, now)) in before.iter().zip(&stamped).enumerate() {
            if Self::same_values(std::slice::from_ref(was), std::slice::from_ref(now)) {
                continue;
            }
            match self.validate_relation(&rec, &table, pos, now.clone()) {
                Err(Signal::Error(e)) => return Ok(Some(e)),
                other => other?,
            }
        }
        Ok(None)
    }

    /// Remembers the stored row the cursor stands on as the lower neighbour of the row about to
    /// start, for AutoSplitKey.
    pub(in crate::pages) fn note_lower_key(&mut self, st: &St) -> Result<(), Signal> {
        let lower = if st.borrow().cursor == Cursor::Row && !st.borrow().draft {
            let rec = self.page_rec(st);
            Some(self.row_key(&rec)?)
        } else {
            None
        };
        st.borrow_mut().lower_key = lower;
        Ok(())
    }

    /// What the client's NewRecord copies onto a fresh buffer of a part: a field whose combined
    /// filter is a single value, when the field is part of the primary key or the page sets
    /// PopulateAllFields; then the table's OnValidate runs for each (corpus
    /// handlers/TestPageSubpagePartFieldLink.al, TestPageSubpagePartConstFilter.al,
    /// TestPageSubpagePartNewRecordInit.al:New_StampsTheLinkedFieldAndValidatesIt).
    pub(in crate::pages) fn stamp_from_filters(&mut self, st: &St, validate: bool) -> Result<(), Signal> {
        if !self.has_source(st) {
            return Ok(());
        }
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let populate_all = engine.program.objects[object]
            .properties
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("PopulateAllFields") && is_true(v));
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        let mut stamped: Vec<(usize, Value)> = Vec::new();
        for pos in 0..table.fields.len() {
            if !(populate_all || table.keys[0].contains(&pos)) {
                continue;
            }
            let single: Vec<Option<Value>> = rec
                .borrow()
                .filters
                .iter()
                .filter(|f| f.field == pos)
                .map(|f| match &f.expr {
                    al_builtins::filter::FilterExpr::Eq(v) => Some(v.clone()),
                    al_builtins::filter::FilterExpr::Range(Some(a), Some(b))
                        if ops::compare(a, b).is_ok_and(|o| o.is_eq()) =>
                    {
                        Some(a.clone())
                    }
                    _ => None,
                })
                .collect();
            // The combined filter of a field is the intersection of its filters, in every filter group: several
            // that all name the same value are that single value (Payment Journal: the record the page was run
            // with carries the batch in group 0 and `OpenJnl` sets it again in group 2; Microsoft's Incom. Doc.
            // Attach. FactBox tests, unprobed, docs/probes-needed.md).
            let Some(Some(v)) = single.first() else { continue };
            if single
                .iter()
                .any(|o| !o.as_ref().is_some_and(|w| ops::compare(v, w).is_ok_and(|c| c.is_eq())))
            {
                continue;
            }
            if single.len() > 1 {
                al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::NewRowEqualFiltersSeveralGroups);
            }
            stamped.push((pos, v.clone()));
        }
        // One validated value is pinned: the link field's OnValidate and table relation see the rest of the line as it
        // was, a SourceTableView value of a field outside the key is not stamped (pages/lt-new-line-stamps-filters/
        // NewLineStampsFilters.al:214-237, :264-287). With two or more validated values, whether the later ones are on
        // the record while the first validates is not pinned. Every value goes on first; when code can see the order (an
        // OnValidate, a table relation, a table extension's validate trigger or a subscriber to the validate events of a
        // stamped field) that runs under Assumption::NewLineStampsAllBeforeValidating (kept, not pinned: refusing it costs
        // 7 baseline cases, handlers/TestPageActionRunPageView.al and misc/lt-probes-needed-7/PnGRefresh.al).
        //
        // Pinned since: two primary key fields linked by a part, each with an OnValidate (one with a TableRelation whose
        // where() names the other): both are on the record before either validates, and they validate in field order,
        // which is the key order there, whatever order the SubPageLink names them in (NewLineStampsFilters.al:
        // QP_TwoKeyLink_TypeFirst_NewLine, _TypeFirst_BlankLine, _NoFirst_NewLine, _WithRelation_NewLine, tables 87149 and
        // 87150). More values, PopulateAllFields, a value from SourceTableView, a key order that differs from the field
        // order, a table extension's validate trigger or a validate event subscriber keep the mark.
        if validate && stamped.len() > 1 {
            let mut seen = false;
            let mut beyond_pin = stamped.len() > 2 || populate_all;
            if let [(a, _), (b, _)] = stamped.as_slice() {
                let key = &table.keys[0];
                let (ia, ib) = (key.iter().position(|k| k == a), key.iter().position(|k| k == b));
                beyond_pin |= !matches!((ia, ib), (Some(x), Some(y)) if x < y);
            }
            let schema = engine.program.objects[table.object as usize]
                .table
                .as_ref()
                .expect("table");
            for (pos, _) in &stamped {
                let f = &schema.fields[*pos];
                let extra = !f.ext_before_validate.is_empty()
                    || !f.ext_after_validate.is_empty()
                    || self.listened(&table, "onbeforevalidateevent", Some(&f.name))?
                    || self.listened(&table, "onaftervalidateevent", Some(&f.name))?;
                // Pinned for values the SubPageLink gives (the link filter group), not for a SourceTableView value.
                let linked = rec
                    .borrow()
                    .filters
                    .iter()
                    .filter(|x| x.field == *pos)
                    .all(|x| x.group == super::parts::LINK_GROUP);
                beyond_pin |= extra || !linked;
                seen |= f.on_validate.is_some() || f.relation || extra;
            }
            if seen && beyond_pin {
                al_builtins::assumptions::assume(
                    al_builtins::assumptions::Assumption::NewLineStampsAllBeforeValidating,
                );
            }
        }
        for (pos, v) in &stamped {
            rec.borrow_mut().fields[*pos] = v.clone();
        }
        if validate {
            for (pos, v) in stamped {
                self.ret_used = false;
                self.record_op(RecordOp::Validate, &rec, &mut [Value::Integer(pos as i32), v])?;
            }
        }
        Ok(())
    }

    /// The first validation of a part's link value raised an error as the host opened: the error is swallowed, OnNewRecord and
    /// OnAfterGetCurrRecord run once on the line as stamped and the part runs nothing more; the open goes on (pages/
    /// lt-qj-part-propagation/QjPartPropagation.al:3766 QT_ValPartSplit_Empty_Last_KeyValidateErrors `open = ok`, `log-open =
    /// T.Doc.V(H1 x= ln=0) V.NEW(H1 0 x= 0 below=Yes) V.AGCR(H1 0 a=0 x=H1 0)`; :3825 QT_ValPartPlain_). The part then answers
    /// Last() with No and runs nothing, and a write raises TestRowNotFound (`last = ok No`, `log-last` empty, `type|code =
    /// TestRowNotFound`). Pinned for a part with AutoSplitKey and DelayedInsert and for one with neither, on the first validation
    /// of a host's open; an error elsewhere, or on another part shape, is raised as before.
    pub(super) fn part_open_link_validate_failed(
        &mut self,
        st: &St,
        e: Box<al_builtins::native::AlError>,
        first: bool,
    ) -> Result<(), Signal> {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let on = |name: &str| {
            engine.program.objects[object]
                .properties
                .iter()
                .any(|(k, v)| k.eq_ignore_ascii_case(name) && is_true(v))
        };
        let shape_pinned = on("AutoSplitKey") == on("DelayedInsert");
        let host_document = st
            .borrow()
            .host
            .as_ref()
            .and_then(|(h, _)| h.upgrade())
            .is_some_and(|h| {
                self.engine.object_facts[h.borrow().object as usize].page_type == Some(al_ir::PageType::Document)
            });
        if !first || !self.part_open_running || !shape_pinned || !host_document {
            if !first || !self.part_open_running {
                return Err(Signal::Error(e));
            }
            return unsupported(
                "a part's link value failed its validation as the host opened, on a part with only one of AutoSplitKey and DelayedInsert or another host than a Document page (not pinned)",
            );
        }
        self.page_trigger(st, "onnewrecord", vec![Value::Boolean(true)])?;
        self.page_event(st, "onnewrecordevent", None, Some(true))?;
        self.set_xrec(st);
        self.page_trigger(st, "onaftergetcurrrecord", vec![])?;
        self.page_event(st, "onaftergetcurrrecordevent", None, None)?;
        // What GetLastErrorText reads after the swallowed error is not pinned.
        self.last_error = al_builtins::native::unpinned_error();
        st.borrow_mut().link_validate_failed = true;
        Ok(())
    }

    /// `stamp_from_filters` with validation, the validate triggers seeing `before` as xRec: a part's blank line that held no row
    /// validates its link value with a blank xRec, not with the line as stamped (pages/lt-qj-part-propagation/
    /// QjPartPropagation.al:3501 QT_ValPartSplit_Empty_Last `log-openedit` and `log-last`: `T.Doc.V(H1 x= ln=0)`, :3583
    /// QT_ValPartDelayed_, :3665 QT_ValPartPlain_; :3937 QT_ValPartSplit_Empty_New_ThenType `log-new`). Callers use it only when
    /// `before` is the table's blank record; with another held row the xRec is not pinned and the line as stamped stays.
    pub(in crate::pages) fn stamp_from_filters_seen_as(&mut self, st: &St, before: Vec<Value>) -> Result<(), Signal> {
        let rec = self.page_rec(st);
        let handle = Rc::as_ptr(&rec) as usize;
        if self.validating.iter().any(|(h, _)| *h == handle) {
            return self.stamp_from_filters(st, true);
        }
        self.validating.push((handle, before));
        let r = self.stamp_from_filters(st, true);
        self.validating.retain(|(h, _)| *h != handle);
        r
    }

    /// The xRec a part's blank line validates its link value with: `held` when it is the table's blank record (the part held
    /// no row), else `None` (see `stamp_from_filters_seen_as`).
    pub(in crate::pages) fn blank_held_xrec(&mut self, st: &St, held: &[Value]) -> Option<Vec<Value>> {
        let table = self.page_rec(st).borrow().table as usize;
        let fresh = self.initial_fields(table);
        Self::same_values(held, &fresh).then_some(fresh)
    }

    /// AutoSplitKey: the last field of the primary key of a new line is numbered from its
    /// neighbours when it is saved (corpus handlers/TestPageAutoSplitKey_Tests.al,
    /// TestPageAutoSplitKeyRanges_Tests.al). Only the shapes the corpus pins are numbered; any
    /// other raises Signal::Unsupported.
    pub(in crate::pages) fn auto_split_key(&mut self, st: &St) -> Result<(), Signal> {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let on = engine.program.objects[object]
            .properties
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("AutoSplitKey") && is_true(v));
        if !on {
            return Ok(());
        }
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        let Some(&last) = table.keys[0].last() else {
            return Ok(());
        };
        let ty = table.fields[last].ty.clone();
        if !matches!(ty, RtType::Integer | RtType::BigInteger | RtType::Decimal) {
            return unsupported(format!("AutoSplitKey on a key field of type {}", ty.describe()));
        }
        let dec = |v: &Value| match v {
            Value::Integer(i) => Some(rust_decimal::Decimal::from(*i)),
            Value::BigInteger(i) => Some(rust_decimal::Decimal::from(*i)),
            Value::Decimal(d) => Some(*d),
            _ => None,
        };
        // A key the test typed stays; the number is assigned to an unset one.
        let current = rec.borrow().fields[last].clone();
        if dec(&current).is_none_or(|d| !d.is_zero()) {
            return Ok(());
        }
        if rec.borrow().key != 0 || !rec.borrow().ascending {
            return unsupported("AutoSplitKey on a part that is not read in primary key order");
        }
        let burned = st.borrow().split_burned;
        let lower = st
            .borrow()
            .lower_key
            .as_ref()
            .and_then(|k| k.get(table.keys[0].len() - 1))
            .and_then(dec);
        // The next line after the lower one in the part's view, within the same leading key.
        let upper = match lower {
            None => None,
            Some(l) => {
                let probe = Rc::new(RefCell::new(rec.borrow().clone()));
                {
                    let fields = rec.borrow().fields.clone();
                    let mut p = probe.borrow_mut();
                    for &k in &table.keys[0][..table.keys[0].len() - 1] {
                        p.filters.push(al_builtins::filter::FieldFilter {
                            field: k,
                            expr: al_builtins::filter::FilterExpr::Range(
                                Some(fields[k].clone()),
                                Some(fields[k].clone()),
                            ),
                            group: 99,
                        });
                    }
                    let lv = ops::convert(Value::Decimal(l), &ty)?;
                    p.filters.push(al_builtins::filter::FieldFilter {
                        field: last,
                        expr: al_builtins::filter::FilterExpr::Gt(lv),
                        group: 99,
                    });
                }
                self.ret_used = true;
                if matches!(
                    self.record_op(RecordOp::FindFirst, &probe, &mut [])?,
                    Value::Boolean(true)
                ) {
                    dec(&probe.borrow().fields[last])
                } else {
                    None
                }
            }
        };
        use rust_decimal::Decimal as D;
        let step = D::from(10000);
        let is_decimal = matches!(ty, RtType::Decimal);
        let starts_low = {
            let s = st.borrow();
            s.split_starts_low && (s.host.is_none() || s.placeholder_line)
        };
        let number = match (lower, upper) {
            // The blank line of a top-level list that took key filter values, typed without New(), starts at 10000
            // (misc/lt-probes-needed-7/PnGRefresh.al:672 QL_Jnl_SheetFG2_DeleteNoUpdate_OnlyRow, :792 QL_Jnl_SheetFG2_OpenOnNoRow),
            // and so does the placeholder line of an empty part (NewLineStampsFilters.al QP_TwoKeyLink_TypeFirst_BlankLine).
            (None, _) if starts_low && !burned => step,
            // An empty grid burns one interval on the blank placeholder row.
            (None, _) => D::from(20000),
            // Appended after the last line (handlers/TestPageAutoSplitKeyMoreShapes.al:104-117,
            // corpus handlers/TestPageAutoSplitKeyRanges_Tests.al): last + 10000. After a negative
            // line the corpus pins exactly these values; the rule behind them is not known, so
            // any other negative last line is refused.
            (Some(l), None) if l >= D::ZERO && burned => l + step + step,
            (Some(_), _) if burned => {
                return unsupported(
                    "AutoSplitKey of a line started from the placeholder line below a negative line or above another line (not pinned)",
                );
            }
            (Some(l), None) if l >= D::ZERO => l + step,
            (Some(l), None)
                if !is_decimal && [-30000, -20000, -10000, -5, -1].contains(&ToPrimitive::to_i64(&l).unwrap_or(0)) =>
            {
                D::from(match ToPrimitive::to_i64(&l).unwrap_or(0) {
                    -30000 => -22500,
                    -20000 => -15000,
                    -10000 => -6667,
                    -5 => -4,
                    _ => 1,
                })
            }
            // Between two lines (handlers/TestPageAutoSplitKeyMoreShapes.al:119-166 and
            // :186-205 for Decimal; handlers/TestPageAutoSplitKeyRanges_Tests.al for the cap): the
            // lower line plus half the gap, at most 10000. A gap that crosses zero gives one less
            // (the number zero is reserved: -10000 and 10000 give -1, -1 and 1 give -0.5), and a
            // result that lands on zero by the cap goes to 1. No whole number between two adjacent
            // lines raises "The line cannot be split." (code TestWrapped).
            (Some(l), Some(u)) if u > l => {
                let crossing = l < D::ZERO && u > D::ZERO;
                let gap = if crossing { u - l - D::ONE } else { u - l };
                let mut half = gap / D::from(2);
                if !is_decimal {
                    half = half.trunc();
                    if half < D::ONE {
                        return Err(error_code("The line cannot be split.", "TestWrapped"));
                    }
                }
                let mut n = l + half.min(step);
                if n.is_zero() {
                    if is_decimal {
                        return unsupported("AutoSplitKey landing on zero in a Decimal key (not pinned)");
                    }
                    n = D::ONE;
                }
                n
            }
            (l, u) => {
                return unsupported(format!(
                    "AutoSplitKey between {l} and {u} (the corpus does not pin it)",
                    l = l.describe(),
                    u = u.describe()
                ));
            }
        };
        rec.borrow_mut().fields[last] = ops::convert(Value::Decimal(number), &ty)?;
        Ok(())
    }
}
