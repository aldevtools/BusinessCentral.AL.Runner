//! A tree repeater whose IndentationColumn is a page variable that OnAfterGetRecord sets (the Order Planning shape). The tree opens
//! collapsed: it reads every row of its view once (each row's level is the variable's value after OnAfterGetRecord ran for it), then
//! shows the rows without a parent, and Expand, Next and the other moves work on those levels as on a tree over an Integer field
//! (pages/lt-y72-page-probes/Y72TreeDefaultState.al header: `agr 10 .. agr 60;agr 10;agr 10;agcr 10`, the walk 10, 40, 60, Expand
//! on 10 adds 20 and 30). The levels are those of the rows the open read; a row it did not read is refused.

use super::*;

impl Session {
    /// The slot of the page variable a tree repeater indents by, when its IndentationColumn names an Integer variable of the page and
    /// the page declares OnAfterGetRecord (which sets it).
    pub(in crate::pages) fn tree_var_slot(&self, st: &St) -> Option<usize> {
        let name = self.tree_indentation(st)?;
        if !self.has_source(st) || !self.declares(st, "onaftergetrecord") {
            return None;
        }
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let globals = &engine.program.objects[object].globals;
        globals
            .iter()
            .position(|g| g.name.eq_ignore_ascii_case(name.trim().trim_matches('"')) && matches!(g.ty, RtType::Integer))
    }

    /// After OnAfterGetRecord of the row Rec holds: notes the row's level when the page is a tree over a page variable.
    pub(in crate::pages) fn note_tree_level(&mut self, st: &St) -> Result<(), Signal> {
        let Some(slot) = self.tree_var_slot(st) else {
            return Ok(());
        };
        let Some(inst) = st.borrow().instance.clone() else {
            return Ok(());
        };
        let Value::Integer(level) = inst.borrow().globals[slot] else {
            return Ok(());
        };
        let key = self.row_key(&self.page_rec(st))?;
        let mut s = st.borrow_mut();
        match s.tree_levels.iter_mut().find(|(k, _)| Self::same_key(k, &key)) {
            Some(entry) => entry.1 = level,
            None => s.tree_levels.push((key, level)),
        }
        Ok(())
    }

    /// The level of each row of Rec's view, the tree's own cut left out, in the page's order: from the Integer field at `level`, or
    /// from the levels the open read (`tree_levels`) for a tree over a page variable.
    pub(super) fn tree_rows_of(
        &mut self,
        st: &St,
        rec: &Rec,
        level: Option<usize>,
    ) -> Result<Vec<(Vec<Value>, i32)>, Signal> {
        if let Some(level) = level {
            return self.tree_rows(rec, level);
        }
        let all = Rc::new(RefCell::new(rec.borrow().clone()));
        all.borrow_mut().filters.retain(|f| f.group != TREE_GROUP);
        let mut rows = Vec::new();
        let table = self.table(&all)?;
        for fields in self.view_rows(&all, usize::MAX)? {
            let key = table.primary_key(&fields);
            let known = st
                .borrow()
                .tree_levels
                .iter()
                .find(|(k, _)| Self::same_key(k, &key))
                .map(|(_, l)| *l);
            let Some(lvl) = known else {
                return unsupported(
                    "a tree over a page variable with a row its open did not read (which level the row has is not pinned)",
                );
            };
            rows.push((key, lvl));
        }
        Ok(rows)
    }

    /// The filter that cuts Rec's view `rows` to the rows `shown`: an alternative of the values of one primary key field, the first
    /// whose values tell the shown rows from the others in the view.
    pub(super) fn tree_key_cut(
        &self,
        rec: &Rec,
        rows: &[(Vec<Value>, i32)],
        shown: &[Vec<Value>],
    ) -> Result<al_builtins::filter::FieldFilter, Signal> {
        let table = self.table(rec)?;
        let pk = &table.keys[0];
        let is_shown = |k: &Vec<Value>| shown.iter().any(|s| Self::same_key(s, k));
        for (i, field) in pk.iter().enumerate() {
            let values: Vec<&Value> = shown.iter().map(|k| &k[i]).collect();
            let tells = rows.iter().all(|(k, _)| {
                let hit = values
                    .iter()
                    .any(|v| Self::same_values(std::slice::from_ref(*v), std::slice::from_ref(&k[i])));
                hit == is_shown(k)
            });
            if tells {
                return Ok(al_builtins::filter::FieldFilter {
                    field: *field,
                    expr: al_builtins::filter::FilterExpr::Or(
                        values
                            .into_iter()
                            .map(|v| al_builtins::filter::FilterExpr::Eq(v.clone()))
                            .collect(),
                    ),
                    group: TREE_GROUP,
                });
            }
        }
        unsupported(
            "a tree over a table whose primary key has no field that tells the rows it shows from the others (not pinned)",
        )
    }

    /// After the open of a tree over a page variable read its view: the levels are known now, so the tree cuts its rows to those
    /// without a parent and the client's rows split into the ones it shows and the ones it holds hidden (`split_tree_window`).
    /// Only a view the client read whole is pinned.
    pub(in crate::pages) fn var_tree_cut_after_read(&mut self, st: &St) -> Result<(), Signal> {
        if self.tree_var_slot(st).is_none() {
            return Ok(());
        }
        let rec = self.page_rec(st);
        if rec.borrow().filters.iter().any(|f| f.group == TREE_GROUP) {
            return Ok(());
        }
        if !st.borrow().window_eof && st.borrow().window.len() >= Self::WINDOW {
            return unsupported(
                "a tree over a page variable whose view is longer than the rows its open reads (not pinned)",
            );
        }
        let rows = self.tree_rows_of(st, &rec, None)?;
        if rows.is_empty() {
            return Ok(());
        }
        let expanded = st.borrow().expanded.clone();
        let shown = Self::tree_visible(&rows, &expanded);
        if !rows.is_empty() {
            let cut = self.tree_key_cut(&rec, &rows, &shown)?;
            rec.borrow_mut().filters.push(cut);
        }
        st.borrow_mut().tree_shown = shown;
        self.split_tree_window(st, None);
        Ok(())
    }

    /// First on the first row a tree shows, on a tree page whose only row trigger is OnAfterGetCurrRecord, reads nothing
    /// (pages/lt-y72-page-probes/Y72TreeDefaultState.al TreeDefault_RowTriggerVariants_OnAfterGetCurrRecordOnly `first log` empty;
    /// with OnAfterGetRecord the client's rows answer it, `move_in_client_rows`).
    pub(in crate::pages) fn tree_first_on_first_row(&mut self, st: &St) -> Result<bool, Signal> {
        if self.tree_indentation(st).is_none()
            || self.declares(st, "onaftergetrecord")
            || !self.declares(st, "onaftergetcurrrecord")
            || ["onfindrecord", "onnextrecord"].iter().any(|t| self.declares(st, t))
            || st.borrow().cursor != Cursor::Row
            || st.borrow().draft
            || st.borrow().host.is_some()
        {
            return Ok(false);
        }
        let rec = self.page_rec(st);
        if !rec.borrow().filters.iter().any(|f| f.group == TREE_GROUP) {
            return Ok(false);
        }
        let key = self.row_key(&rec)?;
        let first = st.borrow().tree_shown.first().cloned();
        Ok(first.is_some_and(|f| Self::same_key(&f, &key)))
    }
}
