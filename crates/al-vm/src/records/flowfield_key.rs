//! The rows of a record whose key starts with a FlowField whose value depends on the row ([`super::FLOWFIELD_KEY`]): ordered
//! by the FlowField's value, then by the rest of the key and the primary key, ascending. `SetAscending(FlowField, false)`
//! reverses the FlowField only (ties stay in primary key order), `Ascending(false)`, FindLast and `Next(-1)` walk the exact
//! reverse, and the row read carries the FlowField's value without CalcFields. Pinned by record/keys-sorting/
//! Y43SetAscendingFlowField.al (a sum with two `field()` conditions: Count, FindFirst, FindSet, FindLast and `Next(-1)`,
//! SetAscending, Ascending, GetAscending; corpus PR 966) and SetCurrentKeyFlowFieldOrder.al:85-180 (a count with one
//! condition, `Ascending(false)`, `SetCurrentKey(Cnt, Id)`). An exist is pinned by PwFlowFieldKeyRule.al:172; a Decimal average, min
//! and max and a lookup of a Text or Decimal field by record/flowfields/FfmOperations.al:930-1092. Other FlowField kinds, a
//! FlowField after another key field and a second FlowField in the key are refused.

use super::*;

/// The methods that run on a record marked with `why` ([`super::MEDIA_KEY`] or [`super::FLOWFIELD_KEY`]) besides the ones
/// that give it a key of its own again: those that do not depend on the key or the order of the rows, and on a FlowField
/// key the finds (`Find` with `-` or `+` only), the walk and the sort order (checked when the rows are read, [`Session::scan_flowfield_key`]).
pub(super) fn runs(method: RecordOp, why: &str, args: &[Value]) -> bool {
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
            )
            || why == FLOWFIELD_KEY
                && method == RecordOp::Find
                && matches!(args.first(), Some(Value::Text(w)) if w.as_str() == "-" || w.as_str() == "+"))
}

impl Session {
    /// `SetAutoCalcFields` on a record whose key starts with a Lookup FlowField: the rows keep the FlowField's order and the
    /// auto-calculated fields carry their values (record/keys-sorting/KeyLookupFlowFieldFirst.al:351-352, a Boolean lookup first).
    /// Under a key that starts with another FlowField kind (sum, count, ...) it is not pinned and refuses.
    pub(super) fn auto_calc_on_lookup_key(&self, method: RecordOp, rec: &Rec, why: &str) -> bool {
        if why != FLOWFIELD_KEY || method != RecordOp::SetAutoCalcFields {
            return false;
        }
        let Ok(t) = self.table(rec) else {
            return false;
        };
        let schema = self.engine.program.objects[t.object as usize].table.as_ref();
        let r = rec.borrow();
        r.adhoc
            .as_ref()
            .and_then(|(_, wanted)| wanted.first())
            .and_then(|&p| schema.and_then(|s| s.fields.get(p)).and_then(|f| f.calc.as_ref()))
            .is_some_and(|c| c.method == al_ir::CalcMethod::Lookup)
    }

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
            && calc(first).is_some_and(|c| {
                use al_ir::CalcMethod as M;
                // An average, a min and a max of a Decimal field, and a lookup of a Text or a Decimal field (negated or not),
                // order by their value like a sum: record/flowfields/FfmOperations.al:930-985 (SetCurrentKey(FlowField)) and
                // :987-1092 (FlowField, then a stored field, with a filter on it, ascending and descending); al-runner issue
                // 7427. A lookup of a Boolean, an Integer or a Date field orders the same way, alone, before stored fields and
                // before the primary key field, with a filter on the stored fields and on the FlowField, a setup row missing
                // (the type's blank value) and after the looked-up rows changed (record/keys-sorting/KeyLookupFlowFieldFirst.al:
                // 205-213, :221-229, :253-261, :285-297, :321-322, :340-342, :413-414). A lookup of another type (Code, Option,
                // ...) is not pinned.
                // Not these: a lookup of a FlowField is no key at all (SetCurrentKey answers false, record/keys-sorting/
                // SetCurrentKeyFlowFieldOrder.al:236-262 `nestedFlowField`), and on a temporary record SetCurrentKey with a
                // lookup FlowField first does not take the key (`temporary_lookup_key`); both stay refused here.
                let plain_source = !rec.borrow().temporary
                    && c.field.is_some_and(|at| {
                        self.engine.tables[c.table]
                            .as_ref()
                            .is_some_and(|s| s.fields[at].stored)
                    });
                match c.method {
                    M::Sum | M::Count | M::Exist => true,
                    M::Average | M::Min | M::Max => plain_source && t.fields[first].ty == RtType::Decimal,
                    M::Lookup => {
                        plain_source
                            && matches!(
                                t.fields[first].ty,
                                RtType::Decimal | RtType::Text(_) | RtType::Boolean | RtType::Integer | RtType::Date
                            )
                    }
                }
            })
            && fields[1..].iter().all(|&p| calc(p).is_none() && t.fields[p].stored);
        if !pinned {
            return unsupported(format!(
                "the order of the rows by the FlowField {} of key {}: only a sum, count, exist, Decimal average, min or max, or a lookup of a Text, Decimal, Boolean, Integer or Date field first, then stored fields, is pinned",
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

    /// `SetCurrentKey` on a temporary record with a Lookup FlowField first: BC cannot sort by it. With the answer used it answers
    /// false and the key stays (record/keys-sorting/KeyLookupFlowFieldFirst.al:383-386, `SetCurrentKey` false, `CurrentKey` Id, the
    /// rows in primary key order); as a statement it raises "Sorting cannot be done" for the FlowField (SetCurrentKeyFlowFieldOrder.al:
    /// 319, a lookup and the primary key field). `None` on any other record or key (a second FlowField in the key, another
    /// FlowField kind first): the scan decides those.
    pub(super) fn temporary_lookup_key(
        &self,
        rec: &Rec,
        t: &TableDef,
        wanted: &[usize],
    ) -> Result<Option<Value>, Signal> {
        if !rec.borrow().temporary {
            return Ok(None);
        }
        let schema = self.engine.program.objects[t.object as usize].table.as_ref();
        let calc = |p: usize| schema.and_then(|s| s.fields.get(p)).and_then(|f| f.calc.as_ref());
        let lookup_first = wanted
            .first()
            .and_then(|&p| calc(p))
            .is_some_and(|c| c.method == al_ir::CalcMethod::Lookup);
        if !lookup_first || wanted[1..].iter().any(|&p| calc(p).is_some()) {
            return Ok(None);
        }
        // Only these type, sign and use pairs are pinned: a Boolean lookup with the answer used (KeyLookupFlowFieldFirst.al:383-386)
        // and a Text lookup as a statement (SetCurrentKeyFlowFieldOrder.al:316), neither negated.
        let negated = calc(wanted[0]).is_some_and(|c| c.negate);
        let pinned = !negated
            && match t.fields[wanted[0]].ty {
                RtType::Boolean => self.ret_used,
                RtType::Text(_) => !self.ret_used,
                _ => false,
            };
        if !pinned {
            return unsupported(format!(
                "SetCurrentKey on a temporary record with the Lookup FlowField {} first: only a Boolean lookup with the answer used and a Text lookup as a statement are pinned",
                t.fields[wanted[0]].name
            ));
        }
        if self.ret_used {
            return Ok(Some(Value::Boolean(false)));
        }
        Err(crate::transactions::al_error_code(
            &format!(
                "Sorting cannot be done based on the {} field(s) in the {} table.",
                t.fields[wanted[0]].name, t.name
            ),
            "DB:Ok",
        ))
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
            row[ff] = self.calc_value(&probe, ff)?;
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
