//! A ListPart whose repeater is a tree keeps the value typed on a row after the part's `CurrPage.Update(false)`; any other
//! part drops it. Pinned by pages/parts/R6889RealShape.al (objects 96130-96147, identical on BC 27.5, 28.5 and 29.0, al-runner
//! issue 6889): the tree parts of steps 2, 2b, 3 and 4 show the typed Type and store it (the Type SetValue writes the row, log
//! `M30:Resource/N30`, before the part reads its rows), steps 0, 1, 2a and 2c without a tree drop it (:1364 PG6889RS_Step2_,
//! :1438 _Step2b_TreeOnly, :1420 _Step2a_, :1456 _Step2c_). What the file does not split (ShowAsTree alone, another
//! IndentationColumn or TreeInitialState, a part without UpdatePropagation Both on its host) is refused.

use super::*;

impl Session {
    /// The row `st` (a part) holds as typed, when its tree keeps it across the part's update: returns the stored row (xRec) the
    /// save compares with. `None`: the part drops the typed row as before (no typed row, or no tree).
    pub(super) fn tree_part_keeps_typed_row(&self, st: &St, both: bool) -> Result<Option<Vec<Value>>, Signal> {
        if !st.borrow().dirty || st.borrow().draft {
            return Ok(None);
        }
        let engine = self.engine();
        let page = page_of(&engine, st);
        let Some(tree) = page.controls.iter().find(|c| {
            c.kind == ir::ControlKind::Repeater
                && matches!(c.prop("showastree"), Some(ir::PropVal::Lit(v)) if unquote(v).eq_ignore_ascii_case("true"))
        }) else {
            return Ok(None);
        };
        let lit = |name: &str| match tree.prop(name) {
            Some(ir::PropVal::Lit(v)) => Some(unquote(v)),
            _ => None,
        };
        let pinned = lit("indentationcolumn").is_some_and(|v| v.replace(' ', "").eq_ignore_ascii_case("Rec.Level"))
            && lit("treeinitialstate").is_some_and(|v| v.eq_ignore_ascii_case("expandall"));
        drop(engine);
        if !pinned || !both || !self.is_list(st) {
            return unsupported(
                "a tree part (ShowAsTree) other than IndentationColumn = Rec.Level and TreeInitialState = ExpandAll on a host part with UpdatePropagation Both, whose code ran CurrPage.Update after a typed value (whether it keeps the value is not pinned)",
            );
        }
        Ok(Some(self.page_xrec_fields(st)))
    }

    /// The tree part saves the row it holds as typed (`before` is the stored row), then reads its rows around it again: the view
    /// as the table has it now and the row's place in it.
    pub(super) fn save_typed_tree_row(
        &mut self,
        st: &St,
        before: Vec<Value>,
    ) -> Result<(Vec<Vec<Value>>, usize), Signal> {
        self.set_xrec_fields(st, before);
        st.borrow_mut().dirty = true;
        self.save_row(st)?;
        let rec = self.page_rec(st);
        let rows = self.view_rows(&rec, Self::WINDOW + 1)?;
        let table = self.table(&rec)?;
        let held = self.row_key(&rec)?;
        match rows.iter().position(|r| Self::same_key(&table.primary_key(r), &held)) {
            Some(at) if rows.len() <= Self::WINDOW => Ok((rows, at)),
            _ => unsupported("a tree part that saved its typed row and whose view no longer holds it or is longer than a window (not pinned)"),
        }
    }
}
