use super::*;
use al_builtins::record_op::RecordOp;

impl Session {
    /// The IndentationColumn of the page's repeater when it has ShowAsTree, as written.
    pub(in crate::pages) fn tree_indentation(&self, st: &St) -> Option<String> {
        let engine = self.engine();
        page_of(&engine, st)
            .controls
            .iter()
            .find(|c| {
                c.kind == ir::ControlKind::Repeater
                    && matches!(c.prop("showastree"), Some(ir::PropVal::Lit(v)) if unquote(v).eq_ignore_ascii_case("true"))
            })
            .and_then(|c| match c.prop("indentationcolumn") {
                Some(ir::PropVal::Lit(v)) => Some(unquote(v)),
                _ => None,
            })
    }

    /// A tree page (ShowAsTree) whose repeater says TreeInitialState = ExpandAll or CollapseAll and whose page declares
    /// OnAfterGetRecord or OnAfterGetCurrRecord. When it opens it reads the rows below the level-0 rows too, though it shows only the
    /// level-0 rows (al-runner issue 6119; corpus writer TW, branch al-runtime-rs-staging-tw, pages/lt-page-runtime/TwTreeExpandTriggers.al:123; the reporter's run on real BC: rows 1, 2, 3 (a child of 2) and 4 with Rec on 4 read `R4;R3;R2;R1;R4;
    /// R4;C4`; which rows show is pages/lt-page-controls-aa/AaPartsLists.al:272 AA_Repeater_TreeExpandedRows). CollapseAll reads the
    /// same (al-runner issue 6523;
    /// pageext/lt-page-ext-properties/Y28CollapsedTreeTests.al:38 Y28_CollapsedOpenAndWalk `agr1;agr2;agr3;agr4;agr5;agr1;agr1;agcr1`).
    /// A tree without TreeInitialState is not pinned and stays as it was.
    pub(in crate::pages) fn tree_expand_all_with_row_triggers(&self, st: &St) -> bool {
        let engine = self.engine();
        let pinned = page_of(&engine, st).controls.iter().any(|c| {
            c.kind == ir::ControlKind::Repeater
                && matches!(c.prop("showastree"), Some(ir::PropVal::Lit(v)) if unquote(v).eq_ignore_ascii_case("true"))
                && matches!(c.prop("treeinitialstate"), Some(ir::PropVal::Lit(v))
                    if unquote(v).eq_ignore_ascii_case("expandall") || unquote(v).eq_ignore_ascii_case("collapseall"))
        });
        drop(engine);
        pinned && (self.declares(st, "onaftergetrecord") || self.declares(st, "onaftergetcurrrecord"))
    }

    /// A tree page (ShowAsTree) whose IndentationColumn is not a field of the source table (a page variable). Its first row shows (a
    /// row without a parent), but the rows after it show by the levels the variable takes, which is not modelled: BC walks the
    /// top-level rows (Y28CollapsedTreeTests.al:293 Y28_VariableIndentationColumn, walk 1,4 with the variable set in
    /// OnAfterGetRecord). A move between its rows is refused.
    pub(in crate::pages) fn tree_over_variable(&self, st: &St) -> Result<bool, Signal> {
        let Some(expr) = self.tree_indentation(st) else {
            return Ok(false);
        };
        if !self.has_source(st) {
            return Ok(false);
        }
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        let name = expr
            .trim()
            .strip_prefix("Rec.")
            .unwrap_or(expr.trim())
            .trim_matches('"')
            .to_string();
        Ok(!table.fields.iter().any(|f| f.name.eq_ignore_ascii_case(&name)))
    }

    /// The position of the Integer field a tree indents by, when the page is a tree over one.
    pub(in crate::pages) fn tree_level_field(&self, st: &St) -> Result<Option<usize>, Signal> {
        let Some(expr) = self.tree_indentation(st) else {
            return Ok(None);
        };
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        let name = expr
            .trim()
            .strip_prefix("Rec.")
            .unwrap_or(expr.trim())
            .trim_matches('"')
            .to_string();
        Ok(table
            .fields
            .iter()
            .position(|f| f.name.eq_ignore_ascii_case(&name) && matches!(f.ty, RtType::Integer)))
    }

    /// Every row of the tree's view with its level, in the page's order, the tree's own cut left out.
    pub(super) fn tree_rows(&mut self, rec: &Rec, level: usize) -> Result<Vec<(Vec<Value>, i32)>, Signal> {
        let all = Rc::new(RefCell::new(rec.borrow().clone()));
        all.borrow_mut().filters.retain(|f| f.group != TREE_GROUP);
        let mut rows: Vec<(Vec<Value>, i32)> = Vec::new();
        self.ret_used = true;
        if matches!(self.record_op(RecordOp::FindSet, &all, &mut [])?, Value::Boolean(true)) {
            loop {
                let lvl = match &all.borrow().fields[level] {
                    Value::Integer(n) => *n,
                    _ => 0,
                };
                rows.push((self.row_key(&all)?, lvl));
                self.ret_used = true;
                if !matches!(self.record_op(RecordOp::Next, &all, &mut [Value::Integer(1)])?, Value::Integer(n) if n != 0)
                {
                    break;
                }
            }
        }
        Ok(rows)
    }

    /// The keys of the rows a tree shows: a row without a parent in the view (no row before it, in the page's order, with a lower
    /// level) shows whatever its level; any other row when its parent shows and is expanded (pages/lt-ah-pages/AhPageTests.al:986-993
    /// for Expand; pageext/lt-page-ext-properties/Y28CollapsedTreeTests.al:94
    /// Y28_SetRangeChildrenInOnOpenPage `walk` 2,3,5 for rows below level 0 whose parent the view leaves out).
    pub(super) fn tree_visible(rows: &[(Vec<Value>, i32)], expanded: &[Vec<Value>]) -> Vec<Vec<Value>> {
        let mut stack: Vec<(i32, bool)> = Vec::new();
        let mut shown = Vec::new();
        for (key, lvl) in rows {
            while stack.last().is_some_and(|(l, _)| *l >= *lvl) {
                stack.pop();
            }
            let visible = stack.last().is_none_or(|(_, open)| *open);
            let open = visible && expanded.iter().any(|k| Self::same_key(k, key));
            stack.push((*lvl, open));
            if visible {
                shown.push(key.clone());
            }
        }
        shown
    }

    /// Cuts the rows of a tree page (ShowAsTree over an Integer field) to the rows it shows. The cut is what the TestPage shows only:
    /// the page's AL code sees every row of Rec (`hide_tree_cut`). On BC Rec.Count() in OnOpenPage reads 5 of a collapsed tree of 5
    /// rows, Rec.Reset() there changes nothing, and the TestPage shows the rows without a parent in the view plus the children of
    /// the parent of the row Rec stands on after OnOpenPage or an action (Y28CollapsedTreeTests.al:60 Y28_ResetInOnOpenPage, :77
    /// Y28_FindSetInOnOpenPage `walk` 1,4,5, :111 Y28_GetChildInOnOpenPage 1,2,3,4, :170 Y28_ActionResetAndUpdate, :190
    /// Y28_ActionGetChildAndUpdate). That parent is expanded as Expand(true) would; a row whose grandparent is collapsed too is not
    /// pinned.
    pub(in crate::pages) fn apply_tree_cut(&mut self, st: &St) -> Result<(), Signal> {
        let Some(level) = self.tree_level_field(st)? else {
            return Ok(());
        };
        let rec = self.page_rec(st);
        rec.borrow_mut().filters.retain(|f| f.group != TREE_GROUP);
        let rows = self.tree_rows(&rec, level)?;
        let current = self.row_key(&rec)?;
        let mut expanded = st.borrow().expanded.clone();
        let mut shown = Self::tree_visible(&rows, &expanded);
        if let Some(at) = rows.iter().position(|(k, _)| Self::same_key(k, &current))
            && !shown.iter().any(|k| Self::same_key(k, &current))
        {
            let lvl = rows[at].1;
            let Some(parent) = rows[..at].iter().rev().find(|(_, l)| *l < lvl) else {
                unreachable!("a row without a parent shows");
            };
            if !shown.iter().any(|k| Self::same_key(k, &parent.0)) {
                return unsupported(
                    "a tree whose Rec stands on a row below a collapsed row that is itself hidden (which rows show is not pinned)",
                );
            }
            expanded.push(parent.0.clone());
            shown = Self::tree_visible(&rows, &expanded);
            st.borrow_mut().expanded = expanded.clone();
        }
        let top: Vec<&Vec<Value>> = rows.iter().filter(|(_, l)| *l == 0).map(|(k, _)| k).collect();
        let plain = expanded.is_empty()
            && top.len() == shown.len()
            && top.iter().zip(&shown).all(|(a, b)| Self::same_key(a, b));
        let cut = if plain {
            al_builtins::filter::FieldFilter {
                field: level,
                expr: al_builtins::filter::FilterExpr::Eq(Value::Integer(0)),
                group: TREE_GROUP,
            }
        } else {
            let table = self.table(&rec)?;
            if table.keys[0].len() != 1 {
                return unsupported(
                    "a tree over a table whose primary key has several fields that shows rows below level 0 when it opens or after an action (not pinned)",
                );
            }
            al_builtins::filter::FieldFilter {
                field: table.keys[0][0],
                expr: al_builtins::filter::FilterExpr::Or(
                    shown
                        .iter()
                        .map(|k| al_builtins::filter::FilterExpr::Eq(k[0].clone()))
                        .collect(),
                ),
                group: TREE_GROUP,
            }
        };
        rec.borrow_mut().filters.push(cut);
        st.borrow_mut().tree_shown = shown;
        Ok(())
    }

    /// Takes the tree's cut off the page's Rec before the page's AL code runs: that code sees every row (`apply_tree_cut`).
    /// `None` when Rec holds no cut (not a tree, or the cut is already off for an outer call).
    pub(in crate::pages) fn hide_tree_cut(&mut self, st: &St) -> Option<TreeCut> {
        if st.borrow().instance.is_none() || !self.has_source(st) {
            return None;
        }
        let rec = self.page_rec(st);
        let mut r = rec.borrow_mut();
        if !r.filters.iter().any(|f| f.group == TREE_GROUP) {
            return None;
        }
        let cut: Vec<_> = r.filters.iter().filter(|f| f.group == TREE_GROUP).cloned().collect();
        r.filters.retain(|f| f.group != TREE_GROUP);
        drop(r);
        let view = super::actions::ListView::of(&rec.borrow());
        let key = self.row_key(&rec).ok();
        Some(TreeCut { cut, view, key })
    }

    /// Puts the cut back after the page's AL code ran: as it was when the code left the view and the row as they were, or a row
    /// the tree shows; else it is made again from the view and the row now (`apply_tree_cut`).
    pub(in crate::pages) fn show_tree_cut(&mut self, st: &St, saved: TreeCut) -> Result<(), Signal> {
        let rec = self.page_rec(st);
        let now = super::actions::ListView::of(&rec.borrow());
        let key = self.row_key(&rec).ok();
        let same_view = self.same_list_view(&saved.view, &now);
        let same_row = match (&key, &saved.key) {
            (Some(a), Some(b)) => Self::same_key(a, b),
            _ => false,
        };
        let row_shown = key
            .as_ref()
            .is_some_and(|k| st.borrow().tree_shown.iter().any(|s| Self::same_key(s, k)));
        if same_view && (same_row || row_shown) {
            rec.borrow_mut().filters.extend(saved.cut);
            return Ok(());
        }
        self.apply_tree_cut(st)
    }

    /// `Expand(on)` on the row a tree stands on: Expand(true) shows the row's direct children, Expand(false) hides them again, and
    /// IsExpanded reads which (pages/lt-ah-pages/AhPageTests.al:950-961 Walk, :986-993 Tree_ThroughPageRun `T1 T5` -> `T1 T2 T4 T5`
    /// -> `T1 T5`, GoToKey of a hidden child `No`). A row's parent is the nearest row before it, in the page's order, with a lower
    /// level. Pinned only for a page without row triggers, a primary key of one field, expanding a row with children and collapsing
    /// an expanded one, a second Expand of either, and Expand(true) on a leaf without a parent; anything else is refused.
    pub(in crate::pages) fn tree_expand(&mut self, st: &St, on: bool) -> Result<(), Signal> {
        for t in ["onfindrecord", "onnextrecord"] {
            if self.declares(st, t) {
                return unsupported(
                    "Expand on a tree page with row triggers (which rows are read again is not pinned)",
                );
            }
        }
        // With OnAfterGetRecord or OnAfterGetCurrRecord, Expand(true) and Expand(false) run no row trigger and the row and the values
        // its OnAfterGetRecord computed stay; the next move reads the row it lands on as any move does (al-runner issue 6119; corpus writer TW, branch al-runtime-rs-staging-tw, pages/lt-page-runtime/TwTreeExpandTriggers.al:123; the
        // reporter's run on real BC: `expand=2/PARENT/L2:` then `next=T/3/CHILD/L3:R3;R3;C3`, and after Expand(false) `next=4/ROOT2/L4:
        // R4;R4;C4`). Measured on a tree whose TreeInitialState is ExpandAll, and on CollapseAll by Y28CollapsedTreeTests.al:136 Y28_ActionCountOnExpandedChild; no initial state is not pinned.
        let row_triggers = self.declares(st, "onaftergetrecord") || self.declares(st, "onaftergetcurrrecord");
        // Pinned by pages/lt-page-runtime/TwTreeExpandTriggers.al (was assumption `tree-expand-runs-no-row-trigger`).
        if row_triggers && !self.tree_expand_all_with_row_triggers(st) {
            return unsupported(
                "Expand on a tree page with row triggers whose TreeInitialState is neither ExpandAll nor CollapseAll (which rows are read is not pinned)",
            );
        }
        if st.borrow().cursor != Cursor::Row || st.borrow().draft {
            return unsupported("Expand on a tree page that stands on no stored row (not pinned)");
        }
        let Some(level) = self.tree_level_field(st)? else {
            return unsupported("Expand on a tree whose IndentationColumn is not an Integer field (not pinned)");
        };
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        if table.keys[0].len() != 1 {
            return unsupported("Expand on a tree over a table whose primary key has several fields (not pinned)");
        }
        let pk = table.keys[0][0];
        let current = self.row_key(&rec)?;
        let rows = self.tree_rows(&rec, level)?;
        let at = rows.iter().position(|(k, _)| Self::same_key(k, &current));
        let has_children = at.is_some_and(|i| rows.get(i + 1).is_some_and(|(_, l)| *l > rows[i].1));
        {
            let mut s = st.borrow_mut();
            let known = s.expanded.iter().position(|k| Self::same_key(k, &current));
            match (on, known) {
                (true, None) if has_children => s.expanded.push(current.clone()),
                (false, Some(i)) => {
                    s.expanded.remove(i);
                }
                // A second Expand(true) or Expand(false) on a row with children changes nothing (pages/lt-page-runtime/P82TestPage.al:
                // 283-289 Tree_Expand_IsExpanded, :336-341 Tree_Rows_WhileCollapsed; a page without row triggers).
                (true, Some(_)) | (false, None) if has_children && !row_triggers => return Ok(()),
                // The same with OnAfterGetRecord and OnAfterGetCurrRecord on an ExpandAll tree: no row trigger runs, the cursor stays
                // (pages/lt-y50-tree-expand-leaf/Y50TreeExpandLeaf.al:313 G517_Mixed_ExpandFalseOnCollapsed, :332
                // G517_Mixed_ExpandTrueTwice_Walk).
                (true, Some(_)) | (false, None) if has_children => return Ok(()),
                // Expand(true) on a row without children and without a parent (a level-0 row, or every row at level 1 with no
                // level-0 row) changes nothing: no error, IsExpanded stays No, no row trigger runs, the cursor stays
                // (Y50TreeExpandLeaf.al:263 and :280 FLAT, :296 MIXED row 40, :351 a tree without row triggers or
                // TreeInitialState). A leaf under a parent is not pinned.
                (true, None) if !has_children && at.is_some_and(|i| !rows[..i].iter().any(|(_, l)| *l < rows[i].1)) => {
                    return Ok(());
                }
                _ => {
                    return unsupported(
                        "Expand(true) on a tree row without children or one already expanded, or Expand(false) on a row not expanded (not pinned)",
                    );
                }
            }
        }
        // Which rows show (`tree_visible`).
        let expanded = st.borrow().expanded.clone();
        let shown_keys = Self::tree_visible(&rows, &expanded);
        let shown: Vec<_> = shown_keys
            .iter()
            .map(|k| al_builtins::filter::FilterExpr::Eq(k[0].clone()))
            .collect();
        st.borrow_mut().tree_shown = shown_keys.clone();
        {
            let mut r = rec.borrow_mut();
            r.filters.retain(|f| f.group != TREE_GROUP);
            let expr = if expanded.is_empty() {
                r.filters.push(al_builtins::filter::FieldFilter {
                    field: level,
                    expr: al_builtins::filter::FilterExpr::Eq(Value::Integer(0)),
                    group: TREE_GROUP,
                });
                None
            } else {
                Some(al_builtins::filter::FilterExpr::Or(shown))
            };
            if let Some(expr) = expr {
                r.filters.push(al_builtins::filter::FieldFilter {
                    field: pk,
                    expr,
                    group: TREE_GROUP,
                });
            }
        }
        // The rows the open read stay as they are (see above): the record finds its row again under the new cut, and holds the
        // values the page shows.
        if row_triggers {
            // The client's rows become the rows that show now, in the view's order, from the rows it read; the others wait.
            {
                let mut s = st.borrow_mut();
                let mut read: Vec<_> = std::mem::take(&mut s.window);
                read.append(&mut s.tree_hidden);
                for (key, _) in &rows {
                    if let Some(i) = read.iter().position(|w| Self::same_key(&w.key, key)) {
                        let w = read.remove(i);
                        if shown_keys.iter().any(|k| Self::same_key(k, key)) {
                            s.window.push(w);
                        } else {
                            s.tree_hidden.push(w);
                        }
                    }
                }
                // A row the view no longer holds is dropped.
            }
            let shown = rec.borrow().fields.clone();
            self.ret_used = true;
            self.record_op(RecordOp::Get, &rec, &mut current.clone())?;
            rec.borrow_mut().fields = shown;
            return Ok(());
        }
        // The client reads the rows again from the row it stands on.
        st.borrow_mut().window = Vec::new();
        self.arrive(st)
    }
}
