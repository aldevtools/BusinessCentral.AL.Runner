//! The columns a store notes on write so that a read can refuse a row it cannot answer for, at no cost until such a row
//! was stored: a value a platform row left unknown (`unknown_cols`), and a text outside ASCII (`accent_cols`), whose order or
//! `@` match may rest on letters no measurement covers.

use super::*;

impl MemStore {
    /// A read that filters on, or orders by, a column a stored row holds unknown (a copy of a platform row) is refused:
    /// whether that row passes, or where it sorts, is not known. Costs nothing until such a row was stored.
    pub(super) fn unknown_read(
        &self,
        cx: &AccessContext,
        t: &TableDef,
        filters: &[FieldFilter],
        order: &[usize],
    ) -> Result<(), StoreError> {
        if self.unknown_cols.is_empty() {
            return Ok(());
        }
        let Some(held) = self.unknown_cols.get(&t.object) else {
            return Ok(());
        };
        let cols: Vec<usize> = filters
            .iter()
            .map(|f| f.field)
            .chain(order.iter().copied())
            .filter(|c| held.contains(c))
            .collect();
        if cols.is_empty() {
            return Ok(());
        }
        let Some(table) = self.table(cx, t) else {
            return Ok(());
        };
        for row in table.values() {
            if let Some(why) = cols.iter().find_map(|&c| row.get(c).and_then(Value::unknown)) {
                return Err(StoreError::Refused(why.to_string()));
            }
        }
        Ok(())
    }

    /// A read whose answer rests on an order or a match of letters no measurement covers is refused (review of al-runner
    /// PR 8220): a filter of `filters` whose answer for a row is not measured (`FilterExpr::collation_refusal`), and, among two
    /// rows or more, an order by a column holding `Æ` or `æ` outside `collation::ae_order_measured`. The rows are checked
    /// only when a filtered or ordered column ever held a text outside ASCII, or a filter's value or pattern holds one.
    /// Rows a backing serves are not noted. A store that is not the database holds a temporary record's rows.
    pub(super) fn collation_read(
        &self,
        cx: &AccessContext,
        t: &TableDef,
        filters: &[FieldFilter],
        order: &[usize],
    ) -> Result<(), StoreError> {
        let held = self.accent_cols.get(&t.object).map_or(&[][..], Vec::as_slice);
        let checked: Vec<&FieldFilter> = filters
            .iter()
            .filter(|f| held.contains(&f.field) || f.expr.holds_non_ascii())
            .collect();
        let ordered: Vec<usize> = order.iter().copied().filter(|c| held.contains(c)).collect();
        if checked.is_empty() && ordered.is_empty() {
            return Ok(());
        }
        let Some(table) = self.table(cx, t) else {
            return Ok(());
        };
        // The order of one row is not observable.
        let ordered = if table.len() >= 2 { ordered } else { Vec::new() };
        let normal_record = self.database;
        for row in table.values() {
            if let Some(why) = checked
                .iter()
                .find_map(|f| f.expr.collation_refusal(&row[f.field], normal_record))
            {
                return Err(StoreError::Refused(why));
            }
            for &c in &ordered {
                if let Value::Text(s) | Value::Code(s) = &row[c] {
                    let text_value = matches!(row[c], Value::Text(_));
                    if !al_builtins::collation::ae_order_measured(s, text_value, normal_record) {
                        return Err(StoreError::Refused(format!(
                            "an order by the field {} of table {}, which holds '{}' (the letters Æ and æ are measured only in a Text field of a normal record, the capital Æ)",
                            t.fields[c].name,
                            t.name,
                            s.as_str()
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    /// Writes down the columns of `row` that hold an unknown value (see `unknown_cols`) or a text outside ASCII (see
    /// `accent_cols`).
    pub(super) fn note_held(&mut self, t: &TableDef, row: &Row) {
        for (i, v) in row.iter().enumerate() {
            let map = match v {
                _ if v.unknown().is_some() => &mut self.unknown_cols,
                Value::Text(s) | Value::Code(s) if !s.is_ascii() => &mut self.accent_cols,
                _ => continue,
            };
            let held = map.entry(t.object).or_default();
            if !held.contains(&i) {
                held.push(i);
            }
        }
    }
}
