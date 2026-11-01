//! The rows of a record whose key starts with a FlowField whose value depends on the row ([`super::FLOWFIELD_KEY`]): ordered
//! by the FlowField's value, then by the rest of the key and the primary key, ascending. `SetAscending(FlowField, false)`
//! reverses the FlowField only (ties stay in primary key order), `Ascending(false)`, FindLast and `Next(-1)` walk the exact
//! reverse, and the row read carries the FlowField's value without CalcFields. Pinned by record/lt-set-current-key/
//! Y43SetAscendingFlowField.al (a sum with two `field()` conditions: Count, FindFirst, FindSet, FindLast and `Next(-1)`,
//! SetAscending, Ascending, GetAscending; corpus PR 966) and SetCurrentKeyFlowFieldOrder.al:85-180 (a count with one
//! condition, `Ascending(false)`, `SetCurrentKey(Cnt, Id)`). Other FlowField kinds, a FlowField after another key field
//! and a second FlowField in the key are refused.

use super::*;

/// The methods that run on a record marked with `why` ([`super::MEDIA_KEY`] or [`super::FLOWFIELD_KEY`]) besides the ones
/// that give it a key of its own again: those that do not depend on the key or the order of the rows, and on a FlowField
/// key the finds, the walk and the sort order (checked when the rows are read, [`Session::scan_flowfield_key`]).
pub(super) fn runs(method: RecordOp, why: &str) -> bool {
    let independent = matches!(
        method,
        RecordOp::CurrentKey
            | RecordOp::FieldNo
            | RecordOp::IsTemporary
            | RecordOp::TableName
            | RecordOp::TableCaption
            | RecordOp::Count
            | RecordOp::IsEmpty
            | RecordOp::SetRange
            | RecordOp::SetFilter
            | RecordOp::GetFilter
            | RecordOp::GetFilters
            | RecordOp::HasFilter
            | RecordOp::Init
    );
    independent
        || (why == FLOWFIELD_KEY
            && matches!(
                method,
                RecordOp::FindFirst
                    | RecordOp::FindLast
                    | RecordOp::FindSet
                    | RecordOp::Next
                    | RecordOp::Ascending
                    | RecordOp::SetAscending
                    | RecordOp::GetAscending
                    | RecordOp::GetView
                    | RecordOp::CalcFields
            ))
}

impl Session {
    /// The FlowField a scan of `rec` orders by, when its key starts with a row-dependent FlowField. Refused for a shape no
    /// test pins.
    fn flowfield_key_field(&self, rec: &Rec, t: &TableDef) -> Result<Option<usize>, Signal> {
        let key = rec.borrow().key;
        let Some(fields) = t.keys.get(key) else {
            return Ok(None);
        };
        let schema = self.engine.program.objects[t.object as usize].table.as_ref();
        let calc = |p: usize| schema.and_then(|s| s.fields.get(p)).and_then(|f| f.calc.as_ref());
        let Some(at) = fields.iter().position(|&p| calc(p).is_some()) else {
            return Ok(None);
        };
        let first = fields[0];
        let pinned = at == 0
            && calc(first).is_some_and(|c| matches!(c.method, al_ir::CalcMethod::Sum | al_ir::CalcMethod::Count))
            && fields[1..].iter().all(|&p| calc(p).is_none() && t.fields[p].stored);
        if !pinned {
            return unsupported(format!(
                "the order of the rows by the FlowField {} of key {}: only a sum or count FlowField first, then stored fields, is pinned",
                t.fields[fields[at]].name,
                fields
                    .iter()
                    .map(|&p| t.fields[p].name.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
            ));
        }
        Ok(Some(first))
    }

    /// [`Session::scan`] of a record whose key starts with a FlowField, `None` for any other key: every row of the filters,
    /// its FlowField calculated, sorted, then the first after `after` in the direction `forward`.
    pub(super) fn scan_flowfield_key(
        &mut self,
        rec: &Rec,
        t: &TableDef,
        forward: bool,
        after: &Option<Vec<Value>>,
    ) -> Result<Option<Option<Vec<Value>>>, Signal> {
        use std::cmp::Ordering;
        let Some(ff) = self.flowfield_key_field(rec, t)? else {
            return Ok(None);
        };
        let key = rec.borrow().key;
        let mut rows: Vec<(Vec<Value>, Vec<Value>)> = Vec::new();
        let mut cursor = None;
        while let Some(mut row) = self.scan_plain(rec, t, true, cursor.take())? {
            cursor = Some(position(t, key, &row));
            let probe = {
                let mut p = rec.borrow().clone();
                p.fields = row.clone();
                p.temporary = false;
                p.auto_calc.clear();
                Rc::new(RefCell::new(p))
            };
            let table = rec.borrow().table as usize;
            self.check_calc_field(table, ff)?;
            row[ff] = self.calc_value(&probe, ff, &mut Vec::new())?;
            rows.push((position(t, key, &row), row));
        }
        let mask = rec.borrow().field_descending;
        let order = |a: &[Value], b: &[Value]| -> Ordering {
            for (i, (x, y)) in a.iter().zip(b).enumerate() {
                let o = ops::compare(x, y).unwrap_or(Ordering::Equal);
                let o = if i < 32 && mask & (1 << i) != 0 { o.reverse() } else { o };
                if o != Ordering::Equal {
                    return o;
                }
            }
            Ordering::Equal
        };
        rows.sort_by(|a, b| order(&a.0, &b.0));
        let hit = if forward {
            rows.into_iter()
                .find(|(p, _)| after.as_deref().is_none_or(|a| order(p, a) == Ordering::Greater))
        } else {
            rows.into_iter()
                .rev()
                .find(|(p, _)| after.as_deref().is_none_or(|a| order(p, a) == Ordering::Less))
        };
        Ok(Some(hit.map(|(_, row)| row)))
    }
}
