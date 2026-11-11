//! The key an import gives the record of a table element without LinkTable below another table element that has a
//! SourceTableView: the view's sorting for the first row, the primary key after it (corpus branch al-runtime-rs-staging-r6924x,
//! corpus PR 1144, xmlports/shapes-properties/R6924xNestedView.al NestedView_Import_RowsOutsideTheView: `ins p1 f= k=Kind;ins p3
//! f= k=Entry;ins h1;ins p2 f= k=Entry`). The view filters nothing on import (the same test stores rows with Qty 0 and -1) and
//! GetFilters reads empty in OnBeforeInsertRecord.

use super::*;

impl Session {
    /// Before an XML import: the key of each such table element's SourceTableView sorting. A view with an ORDER is not pinned (left as it is without element triggers).
    pub(super) fn xp_import_view_keys(&mut self, inst: &Inst, schema: &Schema) -> Result<(), Signal> {
        if schema.xp().format != ir::XmlFormat::Xml {
            return Ok(());
        }
        for node in 0..schema.xp().nodes.len() {
            let Some(view) = Self::xp_import_view_of(schema, node) else {
                continue;
            };
            // With an ORDER: refused where a trigger of the port's elements could read the key or direction.
            let triggers = schema.xp().nodes.iter().any(|n| !n.triggers.is_empty());
            let ordered = crate::views::view_clauses(&view).order;
            if ordered && !triggers {
                continue;
            }
            if ordered {
                return unsupported(format!(
                    "XmlPort import of table element {} (no LinkTable, below another table element) whose SourceTableView has an ORDER (the key and direction its triggers see are not pinned)",
                    schema.node(node).name
                ));
            }
            let rec = self.xp_record(inst, schema.node(node))?;
            let t = self.table(&rec)?;
            let kept = rec.borrow().filters.clone();
            self.set_view_in(
                &rec,
                &t,
                &crate::views::property_view(&view),
                Some(XP_VIEW_GROUP),
                false,
                false,
                false,
            )?;
            let mut r = rec.borrow_mut();
            r.filters = kept;
            r.ascending = true;
            r.field_descending = 0;
        }
        Ok(())
    }

    /// After a row of such a table element is imported: the primary key ascending.
    pub(super) fn xp_import_view_after_row(&mut self, inst: &Inst, schema: &Schema, node: usize) -> Result<(), Signal> {
        if schema.xp().format != ir::XmlFormat::Xml || Self::xp_import_view_of(schema, node).is_none() {
            return Ok(());
        }
        let rec = self.xp_record(inst, schema.node(node))?;
        let mut r = rec.borrow_mut();
        r.key = 0;
        r.adhoc = None;
        r.ascending = true;
        r.field_descending = 0;
        Ok(())
    }

    /// The SourceTableView with a sorting of table element `node` without LinkTable right below another table element.
    fn xp_import_view_of(schema: &Schema, node: usize) -> Option<String> {
        let view = schema.node(node).source_view.as_deref()?;
        (unlinked_below_table(schema, node) && crate::views::view_clauses(view).sorting).then(|| view.to_string())
    }
}
