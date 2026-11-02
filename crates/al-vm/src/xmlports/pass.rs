//! Table element passes of an export: the row loop, key snapshots, link filters.

use super::*;

impl Session {
    /// One element per row of the table element, in the order of the table view (primary key
    /// order by default), after the table view and the link to the record above
    /// (xmlport/TestXmlPortAdvanced.al:XmlPort_NestedTableElements_ExportIncludesChildRecordsAndAttribute).
    pub(super) fn xp_export_table(&mut self, inst: &Inst, schema: &Schema, node: usize) -> Result<Vec<Elem>, Signal> {
        let unlinked = unlinked_below_table(schema, node);
        if unlinked {
            self.xp_unlinked_pass_start(inst, schema, node)?;
        }
        let out = self.xp_export_table_pass(inst, schema, node);
        if unlinked && out.is_ok() {
            self.xp_unlinked_pass_end(inst, schema, node)?;
        }
        out
    }

    /// The start of a pass of table element `node` without LinkTable right below another table element (writer TU's branch
    /// al-runtime-rs-staging-tu, QrXpContainerOccurs.al TU_NestedReal_ItemFilterFirstPass `pre item f= at=12` at the second
    /// line after the first pass set a filter, TU_NestedTemp_TwoTwoTwoFiltered; former assumption `xmlport-nested-unlinked` (pinned on corpus master since bump 1030)): a later
    /// pass starts without the filters an earlier pass left, and the record keeps its row. Pinned only when nothing changed
    /// the record's filters or key between the passes and the key is the one of the first pass; filters of a SourceTableView,
    /// a filter group or marks, and filters of the code's own at the first pass, are not pinned.
    pub(super) fn xp_unlinked_pass_start(&mut self, inst: &Inst, schema: &Schema, node: usize) -> Result<(), Signal> {
        let n = schema.node(node);
        let rec = self.xp_record(inst, n)?;
        let addr = Rc::as_ptr(inst) as usize;
        let refuse = |what: &str| {
            unsupported(format!(
                "XmlPort export of table element {} (no LinkTable, below another table element): {what} at the start of a pass (not pinned)",
                n.name
            ))
        };
        let now = UnlinkedPass::of(&rec.borrow());
        if rec.borrow().marked_only || now.filter_group != 0 {
            return refuse("marks or a filter group");
        }
        match self
            .xp_unlinked_passes
            .iter()
            .find(|(a, i, _)| *a == addr && *i == node)
        {
            None => {
                if !now.filters.is_empty() {
                    return refuse("filters");
                }
                self.xp_unlinked_passes.push((addr, node, now));
            }
            Some((_, _, last)) => {
                if last.filters != now.filters {
                    return refuse("filters set between two passes");
                }
                if last.first_key != now.first_key {
                    return refuse("a key or direction other than at the first pass");
                }
                // Pinned by xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al TU_NestedReal_ItemFilterFirstPass, TU_NestedReal_SlotNoFilter, TU_NestedTemp_TwoTwoNoneFiltered (was assumption `xmlport-nested-unlinked`).
                if n.source_view.is_some() || rec.borrow().filters.iter().any(|f| f.group == XP_VIEW_GROUP) {
                    return refuse("a SourceTableView");
                }
                rec.borrow_mut().filters.clear();
            }
        }
        Ok(())
    }

    /// The end of a pass of table element `node` (`xp_unlinked_pass_start`).
    pub(super) fn xp_unlinked_pass_end(&mut self, inst: &Inst, schema: &Schema, node: usize) -> Result<(), Signal> {
        let rec = self.xp_record(inst, schema.node(node))?;
        let addr = Rc::as_ptr(inst) as usize;
        let now = UnlinkedPass::of(&rec.borrow());
        if let Some((_, _, last)) = self
            .xp_unlinked_passes
            .iter_mut()
            .find(|(a, i, _)| *a == addr && *i == node)
        {
            last.filters = now.filters;
            last.filter_group = now.filter_group;
        }
        Ok(())
    }

    pub(super) fn xp_export_table_pass(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        node: usize,
    ) -> Result<Vec<Elem>, Signal> {
        let n = schema.node(node);
        let mut out = Vec::new();
        let mut rows = Vec::new();
        self.xp_pass_start(inst, schema, node)?;
        // A linked element is filtered to its parent's row before OnPreXmlItem: it counts the parent's rows only
        // (xmlport/lt-xmlport-trigger-flow/XmlPortTriggerFlow.al:325 `Chi.PreXmlItem count=2`, then `count=1`).
        self.xp_link_filter(inst, schema, node)?;
        // A Break in OnPreXmlItem leaves the element without rows and without the empty pass (P112_Export_Break, preItem1); a Quit
        // leaves what was written so far.
        match self.xp_site(inst, schema, node, "onprexmlitem") {
            Ok(Some(Flow::Break)) => return Ok(out),
            Err(Signal::Exit(None)) if Self::xp_quit(inst) => return Ok(out),
            other => {
                other?;
            }
        }
        // MaxOccurs = Once keeps the first row and below tells two, three and more rows apart: four are enough to read, which
        // also ends the read of an Integer root over `Number = filter(1 ..)` (xmlport/lt-y17-xmlport-ns/Y17XmlPortNamespaces.al).
        let cap = if n.max_unbounded == Some(false) { 4 } else { usize::MAX };
        self.xp_rows_upto(inst, schema, node, &mut rows, cap)?;
        // MaxOccurs = Once writes the first row only (xmlport/lt-sg-xmlport-occurs/SgXmlPortOccurs.al:259 SG_OnceTemp_Export_TwoRowsEach,
        // :390 SG_OnceReal_Export `two`). Pinned without export triggers; whether a trigger sees the rows after the first is not.
        // With export triggers and two rows (QrXpContainerOccurs.al TR_Booking_TwoReplyRows on branch al-runtime-rs-staging-tr):
        // only the first row's OnAfterGetRecord runs and the record holds the second row afterwards. With three rows only the
        // first is written, with one OnAfterGetRecord (TS_Again_TrMember on branch al-runtime-rs-staging-pub-ts); the row the
        // record holds afterwards is not pinned, so its buffer is marked ([`crate::records::XP_ROW_AFTER_ONCE`]).
        let mut ends_on = None;
        let mut ends_unpinned = false;
        let mut nested_after = false;
        let nested_once = n.max_unbounded == Some(false) && nested_table(schema, node) && n.link_node.is_some();
        if nested_once {
            // A linked one below another table element (writer TT's branch al-runtime-rs-staging-tt, QrXpContainerOccurs.al
            // TT_Confirm_Export_NoPreFilter: five rows, `get detail 1` only): the first row of each pass, one OnAfterGetRecord; the row
            // the record holds afterwards is not pinned.
            // Pinned by xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al TT_Confirm_Export_NpFilters, TT_Confirm_Export_NoKindFilter, TT_Confirm_Export_NoPreFilter (was assumption `xmlport-linked-once`).
            // With LinkFields, MinOccurs = Once and no export triggers, the record holds the second of two rows afterwards and a
            // later pass without rows writes it; a first pass without rows writes the record blank (corpus PR 896,
            // misc/lt-np4-probes/Np4XmlPort.al Y9_XmlPort_LinkedOnceOnce_ZeroOneTwoRows `held 3/10/A/h3l10`,
            // _TwoRowsThenNone `<Line head="1" no="10">` under heads 2 and 3, _OneRowThenNone). Of three or more rows it is not
            // pinned.
            let pinned_two =
                rows.len() == 2 && !n.link_fields.is_empty() && !n.min_zero && !export_triggers_below(schema, node);
            if pinned_two {
                ends_on = rows.pop();
            } else if rows.len() > 1 {
                rows.truncate(1);
                nested_after = true;
            }
        } else if n.max_unbounded == Some(false) && rows.len() > 1 {
            // Without export triggers, a table element without LinkTable below another one with MinOccurs = Once holds the second of
            // two rows afterwards: the next pass without rows writes it (misc/lt-np4-probes/Np4XmlPort.al:381
            // NP4_XmlPort_OnceOnce_TwoRows `<Child cid="12">` under head 2, corpus master), and the second of three as well (corpus
            // PR 896, Np4XmlPort.al Y9_XmlPort_OnceOnce_ThreeRowsThenNone `<Child cid="12">`). Of four or more it is not pinned.
            let once_once = unlinked_below_table(schema, node) && !n.min_zero && !export_triggers_below(schema, node);
            if once_once && (rows.len() == 2 || rows.len() == 3) {
                ends_on = Some(rows[1].clone());
            } else if once_once {
                ends_unpinned = true;
            } else if export_triggers_below(schema, node) {
                if rows.len() > 2 && unlinked_below_table(schema, node) {
                    // Below another table element without LinkTable the record holds the second row afterwards, also of six
                    // (QrXpContainerOccurs.al TU_NestedReal_SlotNoFilter on writer TU's branch: `get slot 101;get item 12;pre slot
                    // f= at=102`).
                    // Pinned by xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al TU_NestedReal_ItemFilterFirstPass, TU_NestedReal_SlotNoFilter, TU_NestedTemp_TwoTwoNoneFiltered (was assumption `xmlport-nested-unlinked`).
                    ends_on = Some(rows[1].clone());
                } else if rows.len() > 2 {
                    // Pinned by xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al TS_Again_SameData, TS_Again_AssignBetween, TS_Again_TransferFieldsBetween, TS_Again_InitBetween, TS_Again_KeyBetween, TS_Again_LineFilter, TS_View_SecondExport, TS_Again_TrBooking, TS_Again_TrMember (was assumption `xmlport-export-again`).
                    ends_unpinned = true;
                } else {
                    ends_on = rows.pop();
                }
            }
            rows.truncate(1);
        }
        let rec = self.xp_record(inst, n)?;
        if rows.is_empty() && !n.min_zero && rec.borrow().unpinned_buffer == Some(crate::records::XP_ROW_AFTER_ONCE) {
            return unsupported(format!(
                "XmlPort export of a table element without rows that writes its record as it is: {}",
                crate::records::XP_ROW_AFTER_ONCE
            ));
        }
        // A table element without rows still passes its fields once, without a row: OnBeforePassField runs and no
        // OnAfterGetRecord (P94XpPathFlow.al:Export_PathsNoRows). Unless it is optional, the element is written once with the
        // values of the record as it stands (xmlport/lt-xmlport-query-datatransfer/XpTExportBasic.al:Export_StaticWithRecordNoRows,
        // Export_EmptyTable: 462 bytes, a row of `0`, empty text, `0.00`, `No`).
        if rows.is_empty() && !n.min_zero {
            let mut e = Elem::new(&doc_name(schema, node));
            let r = self.xp_export_content(inst, schema, node, &mut e);
            out.push(e);
            match r {
                Err(Signal::Exit(None)) if Self::xp_quit(inst) => return Ok(out),
                other => other?,
            }
        } else if rows.is_empty() && n.link_node.is_some() {
            // An optional linked element without rows passes no field (XmlPortTriggerFlow.al:353 Export_TriggerOrder_EmptyTables:
            // `Chi.PreXmlItem count=0` and no `Chi.Id.BeforePassField`).
        } else if rows.is_empty() && !n.parent.is_some_and(|p| schema.node(p).kind == ir::XmlNodeKind::Table) {
            // An optional element that is not linked and not below another table element runs no trigger of its nodes without rows:
            // only its OnPreXmlItem ran (corpus writer TT's branch al-runtime-rs-staging-tt, QrXpContainerOccurs.al
            // TT_Optional_Unlinked_NoRows: `pre opt;` and no `pass opt no;`; former assumption `xmlport-optional-no-rows` (pinned on corpus master since bump 1030)).
            // Pinned by xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al TT_Optional_Unlinked_NoRows (was assumption `xmlport-optional-no-rows`).
        } else if rows.is_empty() {
            // P94XpPathFlow.al:Export_PathsNoRows pins the empty pass for an element that is not optional only; for an optional
            // element below another table element that is not linked, whether its fields are passed is not pinned.
            let passes = schema.children(node).into_iter().any(|c| {
                let k = schema.node(c);
                matches!(k.kind, ir::XmlNodeKind::Field | ir::XmlNodeKind::FieldAttr)
                    && k.trigger("onbeforepassfield").is_some()
            });
            if passes {
                return unsupported(
                    "XmlPort export of an optional table element without rows whose fields have OnBeforePassField (not pinned)",
                );
            }
        }
        for row in rows {
            self.load(&rec, row)?;
            if matches!(
                rec.borrow().unpinned_buffer,
                Some(crate::records::XP_ROW_AFTER_ONCE | crate::records::XP_NESTED_ROW_AFTER_ONCE)
            ) {
                rec.borrow_mut().unpinned_buffer = None;
            }
            self.xp_calc_listed_fields(schema, node, &rec)?;
            // Skip leaves the row out, Break ends the rows; BreakUnbound ends the trigger and the row goes on (P112_Export_Skip,
            // _Break, _BreakUnbound at getRec2).
            match self.xp_site(inst, schema, node, "onaftergetrecord") {
                Ok(Some(Flow::Break | Flow::Skip)) if ends_on.is_some() || nested_after => {
                    return unsupported(
                        "XmlPort export of a table element with MaxOccurs = Once and two rows whose first OnAfterGetRecord skips or breaks (not pinned)",
                    );
                }
                Ok(Some(Flow::Break)) => break,
                Ok(Some(Flow::Skip)) => continue,
                Err(Signal::Exit(None)) if Self::xp_quit(inst) => return Ok(out),
                Err(e) => return Err(e),
                Ok(_) => {}
            }
            let mut e = Elem::new(&doc_name(schema, node));
            let r = self.xp_export_content(inst, schema, node, &mut e);
            out.push(e);
            match r {
                Err(Signal::Exit(None)) if Self::xp_quit(inst) => return Ok(out),
                other => other?,
            }
            if Self::xp_quit(inst) {
                return Ok(out);
            }
        }
        if let Some(row) = ends_on {
            self.load(&rec, row)?;
        }
        if ends_unpinned {
            rec.borrow_mut().unpinned_buffer = Some(crate::records::XP_ROW_AFTER_ONCE);
        }
        if nested_after {
            rec.borrow_mut().unpinned_buffer = Some(crate::records::XP_NESTED_ROW_AFTER_ONCE);
        }
        Ok(out)
    }

    /// The key each table element with LinkTable and no LinkFields below another table element has once the SourceTableViews are
    /// applied, before the first row of the export is read (`xp_pass_start`). Only the view's sorting (ascending; a later export on
    /// the same instance keeps the record's key, pinned only while it is the view's) or the primary key ascending without a
    /// sorting in the view is pinned (writer TU's branch
    /// al-runtime-rs-staging-tu, QrXpContainerOccurs.al TU_LinkedKey_NoChange `pre c1 [VERSION(1) SORTING(Field2)]` with
    /// `sorting(A)`, `pre d1 [VERSION(1) SORTING(Field1)]` without a view).
    pub(super) fn xp_pass_key_snapshots(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        view_keys: &[(usize, ViewKey)],
    ) -> Result<(), Signal> {
        let addr = Rc::as_ptr(inst) as usize;
        for node in 0..schema.xp().nodes.len() {
            let n = schema.node(node);
            if !nested_table(schema, node) || n.link_node.is_none() || !n.link_fields.is_empty() {
                continue;
            }
            let view = n
                .source_view
                .as_deref()
                .unwrap_or("")
                .to_ascii_lowercase()
                .replace(' ', "");
            let rec = self.xp_record(inst, n)?;
            let r = rec.borrow();
            // A later export on the same instance keeps the record's key (`xp_apply_source_views`): pinned only while it is still the
            // view's own.
            let ascending = r.ascending && r.field_descending == 0;
            let by_view = view.contains("sorting(")
                && ascending
                && view_keys
                    .iter()
                    .any(|(i, (k, a))| *i == node && *k == r.key && *a == r.adhoc);
            let primary = !view.contains("sorting(") && r.key == 0 && r.adhoc.is_none() && ascending;
            let v = PassView {
                filters: Vec::new(),
                filter_group: r.filter_group,
                key: r.key,
                adhoc: r.adhoc.clone(),
                ascending: r.ascending,
                field_descending: r.field_descending,
                pinned_key: !view.contains("order(") && (by_view || primary),
                started: false,
            };
            drop(r);
            self.xp_pass_views.push((addr, node, v));
        }
        Ok(())
    }

    /// The start of a pass of table element `node` with LinkTable and no LinkFields below another table element: LinkTable adds no
    /// filter, and every pass after the first starts from the filters the record had at the first, its SourceTableView included,
    /// whatever OnPreXmlItem of an earlier pass set or reset (writer TT's branch al-runtime-rs-staging-tt, QrXpContainerOccurs.al
    /// TT_Confirm_Export_NpFilters `pre detail a2 filters= count=5` after a1's Reset and filters, TT_Confirm_Export_TwoReplies
    /// `pre ticket filters=` at reply 2; former assumption `xmlport-link-pass-reset` (pinned on corpus master since bump 1030)), and from the SourceTableView's key, or the primary
    /// key without a sorting in the view, ascending, whatever key or direction OnPreXmlItem of an earlier pass set (writer TU's
    /// branch al-runtime-rs-staging-tu, QrXpContainerOccurs.al TU_LinkedKey_KeyAtFirstPass, _KeyDescendingAtFirstPass,
    /// _DescendingAtFirstPass, _ResetAtFirstPass, _KeyAtSecondPass, _KeyAtEveryPass). Filters, marks, a key or a direction of the
    /// code's own on the record at the first pass are not pinned.
    pub(super) fn xp_pass_start(&mut self, inst: &Inst, schema: &Schema, node: usize) -> Result<(), Signal> {
        let n = schema.node(node);
        if !nested_table(schema, node) || n.link_node.is_none() || !n.link_fields.is_empty() {
            return Ok(());
        }
        let rec = self.xp_record(inst, n)?;
        let addr = Rc::as_ptr(inst) as usize;
        let Some(pos) = self.xp_pass_views.iter().position(|(a, i, _)| *a == addr && *i == node) else {
            return unsupported(format!(
                "XmlPort export of table element {} (LinkTable without LinkFields below another table element) outside an XML export (not pinned)",
                n.name
            ));
        };
        let v = self.xp_pass_views[pos].2.clone();
        if v.started {
            // Pinned by xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al TT_Confirm_Export_NpFilters, TT_Confirm_Export_NoKindFilter, TT_Confirm_Export_TwoReplies, TU_LinkedKey_KeyAtFirstPass, TU_LinkedKey_DescendingAtFirstPass (was assumption `xmlport-link-pass-reset`).
            let mut r = rec.borrow_mut();
            r.filters = v.filters;
            r.filter_group = v.filter_group;
            r.key = v.key;
            r.adhoc = v.adhoc;
            r.ascending = v.ascending;
            r.field_descending = v.field_descending;
            r.marked_only = false;
            return Ok(());
        }
        let r = rec.borrow();
        if !v.pinned_key
            || r.key != v.key
            || r.adhoc != v.adhoc
            || r.ascending != v.ascending
            || r.field_descending != v.field_descending
        {
            return unsupported(format!(
                "XmlPort export of table element {} (LinkTable without LinkFields below another table element) whose record has a key or direction other than its SourceTableView's sorting or the primary key ascending at the first pass (not pinned)",
                n.name
            ));
        }
        if r.marked_only || r.filters.iter().any(|f| f.group != XP_VIEW_GROUP) {
            return unsupported(format!(
                "XmlPort export of table element {} (LinkTable without LinkFields below another table element) whose record has filters or marks of the code's own (not pinned)",
                n.name
            ));
        }
        let filters = r.filters.clone();
        let filter_group = r.filter_group;
        drop(r);
        let e = &mut self.xp_pass_views[pos].2;
        e.filters = filters;
        e.filter_group = filter_group;
        e.started = true;
        Ok(())
    }

    /// Filters linked table element `node` to the row its LinkTable holds.
    pub(super) fn xp_link_filter(&mut self, inst: &Inst, schema: &Schema, node: usize) -> Result<(), Signal> {
        let n = schema.node(node);
        let Some(link) = n.link_node else { return Ok(()) };
        let rec = self.xp_record(inst, n)?;
        let parent = self.xp_record(inst, schema.node(link))?;
        for &(own, other) in &n.link_fields {
            let v = parent.borrow().fields[other].clone();
            self.ret_used = false;
            self.record_op(RecordOp::SetRange, &rec, &mut [Value::Integer(own as i32), v])?;
        }
        Ok(())
    }

    /// The rows of table element `node` in export order.
    pub(super) fn xp_rows(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        node: usize,
        rows: &mut Vec<Vec<Value>>,
    ) -> Result<(), Signal> {
        self.xp_rows_upto(inst, schema, node, rows, usize::MAX)
    }

    /// As `xp_rows`, reading at most `cap` rows.
    pub(super) fn xp_rows_upto(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        node: usize,
        rows: &mut Vec<Vec<Value>>,
        cap: usize,
    ) -> Result<(), Signal> {
        let n = schema.node(node);
        let rec = self.xp_record(inst, n)?;
        let t = self.table(&rec)?;
        self.xp_link_filter(inst, schema, node)?;
        let (key, asc) = {
            let r = rec.borrow();
            (r.key, r.ascending)
        };
        let mut after = None;
        while rows.len() < cap
            && let Some(row) = self.scan(&rec, &t, asc, after.take())?
        {
            after = Some(position(&t, key, &row));
            rows.push(row);
        }
        Ok(())
    }
}

/// The filters of a table element record at the end of its last pass and its key at the start of the first
/// (`Session::xp_unlinked_pass_start`).
pub(crate) struct UnlinkedPass {
    pub(super) filters: String,
    pub(super) filter_group: i32,
    /// The key and direction at the start of the first pass.
    pub(super) first_key: String,
}

impl UnlinkedPass {
    pub(super) fn of(r: &al_builtins::value::RecordState) -> UnlinkedPass {
        UnlinkedPass {
            filters: if r.filters.is_empty() {
                String::new()
            } else {
                format!("{:?}", r.filters)
            },
            filter_group: r.filter_group,
            first_key: format!("{:?}", (r.key, &r.adhoc, r.ascending, r.field_descending)),
        }
    }
}

/// Whether node `node` is a table element without LinkTable right below another table element.
pub(super) fn unlinked_below_table(schema: &Schema, node: usize) -> bool {
    let n = schema.node(node);
    n.kind == ir::XmlNodeKind::Table
        && n.link_node.is_none()
        && n.parent.is_some_and(|p| schema.node(p).kind == ir::XmlNodeKind::Table)
}

/// Runs `body` once for every row of table element `node`, with the row loaded.
/// The key and ad hoc key a SourceTableView's sorting gives a table element record.
pub(super) type ViewKey = (usize, Option<(usize, Vec<usize>)>);

/// The view of a table element record at its first pass of an export (`Session::xp_pass_start`).
#[derive(Clone)]
pub(crate) struct PassView {
    pub(super) filters: Vec<al_builtins::filter::FieldFilter>,
    pub(super) filter_group: i32,
    pub(super) key: usize,
    pub(super) adhoc: Option<(usize, Vec<usize>)>,
    pub(super) ascending: bool,
    pub(super) field_descending: u32,
    /// Whether the key at the export's start is one TU pinned (the view's sorting or the primary key, ascending).
    pub(super) pinned_key: bool,
    /// Whether the first pass ran (its filters are recorded).
    pub(super) started: bool,
}

/// Whether node `node` is a table element right below another table element.
pub(super) fn nested_table(schema: &Schema, node: usize) -> bool {
    let n = schema.node(node);
    n.kind == ir::XmlNodeKind::Table && n.parent.is_some_and(|p| schema.node(p).kind == ir::XmlNodeKind::Table)
}

/// What the field of a text export gives.
pub(super) enum FieldPass {
    Value(String),
    Omit,
    Stop,
}

pub(super) fn for_each_row(
    me: &mut Session,
    inst: &Inst,
    schema: &Schema,
    node: usize,
    mut body: impl FnMut(&mut Session, &Inst, &Schema) -> Result<(), Signal>,
) -> Result<(), Signal> {
    let mut rows = Vec::new();
    if me.xp_site(inst, schema, node, "onprexmlitem")? == Some(Flow::Break) {
        return Ok(());
    }
    me.xp_rows(inst, schema, node, &mut rows)?;
    let rec = me.xp_record(inst, schema.node(node))?;
    for row in rows {
        me.load(&rec, row)?;
        match me.xp_site(inst, schema, node, "onaftergetrecord")? {
            Some(Flow::Break) => break,
            Some(Flow::Skip) => continue,
            _ => {}
        }
        body(me, inst, schema)?;
    }
    Ok(())
}

/// Whether node `node` or a node below it has a trigger an export runs.
pub(super) fn export_triggers_below(schema: &Schema, node: usize) -> bool {
    let n = schema.node(node);
    ["onaftergetrecord", "onbeforepassfield", "onbeforepassvariable"]
        .iter()
        .any(|t| n.trigger(t).is_some())
        || schema
            .children(node)
            .into_iter()
            .any(|c| export_triggers_below(schema, c))
}

impl Session {
    /// The FlowFields a table element's `CalcFields` lists, calculated on the row an export just read, before its
    /// OnAfterGetRecord (xmlport/lt-y68-xmlport-shapes/Y68XmlPortShapes.al:567-621: `CalcFields = Total` gives `total=3.5
    /// cnt=0` there, also on a temporary table element whose rows OnPreXmlItem copied).
    fn xp_calc_listed_fields(&mut self, schema: &Schema, node: usize, rec: &Rec) -> Result<(), Signal> {
        let listed = &schema.node(node).calc_fields;
        if listed.is_empty() {
            return Ok(());
        }
        let listed = listed.clone();
        self.calc_fields(rec, &listed)
    }
}
