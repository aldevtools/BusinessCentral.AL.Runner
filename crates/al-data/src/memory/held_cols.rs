//! The columns a store notes on write so that a read can refuse a row it cannot answer for, at no cost until such a row
//! was stored: a value a platform row left unknown or a placeholder's size (`unknown_cols`), and a text outside ASCII (`accent_cols`), whose order or
//! `@` match may rest on letters no measurement covers.

use super::*;

/// Why a filter or an order may not compare the stored value `v`: a column a platform row left unknown, or a placeholder's size
/// (ADR-040 amendment 2026-11-12b, point 4: the size answers only a comparison with 0, which no filter or key order is).
fn unread(v: &Value) -> Option<String> {
    match v {
        Value::Unknown(why) => Some(why.to_string()),
        Value::PlaceholderSize(m, _) => Some(al_builtins::placeholder_output::size_refusal_text(
            m,
            "a record filter or sort on a field holding the value",
        )),
        _ => None,
    }
}

impl MemStore {
    /// A read that filters on, or orders by, a column a stored row holds unknown (a copy of a platform row) or holding a
    /// placeholder's size ([`unread`]) is refused: whether that row passes, or where it sorts, is not known. So is a filter whose
    /// value is a placeholder's size. Costs nothing until such a row was stored or such a size was made.
    pub(super) fn unknown_read(
        &self,
        cx: &AccessContext,
        t: &TableDef,
        filters: &[FieldFilter],
        order: &[usize],
    ) -> Result<(), StoreError> {
        if al_builtins::placeholder_output::sizes_made() {
            if let Some(m) = filters.iter().find_map(|f| f.expr.placeholder_size()) {
                return Err(StoreError::Refused(al_builtins::placeholder_output::size_refusal_text(
                    m,
                    "a record filter on the value",
                )));
            }
        }
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
            if let Some(why) = cols.iter().find_map(|&c| row.get(c).and_then(unread)) {
                return Err(StoreError::Refused(why));
            }
        }
        Ok(())
    }

    /// A read whose answer rests on an order or a match of letters no measurement covers is refused (al-runner issues 8195 and
    /// 8196, review of al-runner PR 8220): a row whose passing rests on such a filter answer
    /// ([`al_builtins::filter::row_passes_collation`]), unless a filter with a measured answer keeps it out; and, among the rows that
    /// pass, two neighbours in the order of `key` (`None`: a read in no order) whose order rests on `Æ` or `æ`
    /// ([`al_builtins::collation::ae_decides_order`]). The rows are looked at only when a filter that can refuse
    /// ([`al_builtins::filter::FilterExpr::collation_open`]) reads a column where a written row held `Æ` or `æ` (any letter outside
    /// ASCII, for an `@` filter) or holds a letter outside ASCII itself, or the key orders by a column where a written row held
    /// `Æ` or `æ` and no equality filter holds it to one text. Rows a backing or a virtual table
    /// serves are not noted. A store that is not the database (a temporary record's, a CRM connection's) holds no normal records.
    ///
    /// The answer is decided once per read: it is kept per table, key and filters ([`Verdict`]) and holds while the table is
    /// the same, or only Modifies changed it that wrote no column the read looks at ([`ModifyRun`]).
    pub(super) fn collation_read(
        &self,
        cx: &AccessContext,
        t: &TableDef,
        filters: &[FieldFilter],
        key: Option<usize>,
    ) -> Result<(), StoreError> {
        // Only `Æ` and `æ` make an order, a range or an equality open; other accents only an `@` filter.
        let held = self.ae_cols.get(&t.object).map_or(&[][..], Vec::as_slice);
        let accents = self.accent_cols.get(&t.object).map_or(&[][..], Vec::as_slice);
        let looked_at = |f: &FieldFilter| {
            (held.contains(&f.field) || f.expr.holds_non_ascii() || (accents.contains(&f.field) && f.expr.holds_at()))
                && f.expr.collation_open()
        };
        let checked = filters.iter().any(looked_at);
        let key_fields = |k: usize| t.keys[k].iter().chain(&t.keys[0]).copied();
        // Rows that pass an equality filter hold the same text in its field, so that field never decides their order.
        let one_text = |c: usize| {
            filters
                .iter()
                .any(|f| f.field == c && f.group != al_builtins::filter::OR_GROUP && f.expr.text_equality().is_some())
        };
        let ordered = key.is_some_and(|k| key_fields(k).any(|c| held.contains(&c) && !one_text(c)));
        if !checked && !ordered {
            return Ok(());
        }
        let slot = slot(cx, t);
        // Equalities alone, in no order that rests on these letters: only the rows that tie with an equality's text can be open.
        let ties: Option<Vec<(usize, &str)>> = filters
            .iter()
            .filter(|f| !ordered && looked_at(f))
            .map(|f| f.expr.tie_text().map(|x| (f.field, x)))
            .collect();
        if let Some(ties) = ties.filter(|_| !ordered) {
            return self
                .tie_read(cx, t, &slot, filters, &ties)
                .map_or(Ok(()), |why| Err(StoreError::Refused(why)));
        }
        let cols: Vec<usize> = filters
            .iter()
            .map(|f| f.field)
            .chain(key.into_iter().flat_map(key_fields))
            .collect();
        let refusal = match self.kept_verdict(&slot, key, filters, &cols) {
            Some(refusal) => refusal,
            None => {
                let Some(table) = self.table(cx, t) else {
                    return Ok(());
                };
                let order: Vec<usize> = match key {
                    Some(k) if ordered => key_fields(k).filter(|&c| held.contains(&c) && !one_text(c)).collect(),
                    _ => Vec::new(),
                };
                let refusal = self.collation_verdict(t, &table, filters, key, &order);
                self.keep_verdict(slot, key, filters, refusal.clone());
                refusal
            }
        };
        refusal.map_or(Ok(()), |why| Err(StoreError::Refused(why)))
    }

    /// The refusal of [`MemStore::collation_read`] for the rows of `table` as they are; `order`: the noted key fields that can
    /// decide the order of two rows that pass.
    fn collation_verdict(
        &self,
        t: &TableDef,
        table: &Table,
        filters: &[FieldFilter],
        key: Option<usize>,
        order: &[usize],
    ) -> Option<String> {
        let normal_record = self.holds_normal_records();
        let mut passing: Vec<&Row> = Vec::new();
        let mut holds_ae = false;
        for row in table.values() {
            #[cfg(test)]
            COLLATION_ROWS.with(|n| n.set(n.get() + 1));
            match al_builtins::filter::row_passes_collation(row, filters, normal_record) {
                Err(why) => return Some(why),
                Ok(false) => continue,
                Ok(true) => {}
            }
            if !order.is_empty() {
                holds_ae |= order.iter().any(
                    |&c| matches!(&row[c], Value::Text(s) | Value::Code(s) if al_builtins::collation::holds_ae(s)),
                );
                passing.push(row);
            }
        }
        let key = key.filter(|_| holds_ae && passing.len() >= 2)?;
        passing.sort_by(|a, b| cmp_rows(t, key, a, b));
        passing
            .windows(2)
            .find_map(|w| order_refusal(t, key, w[0], w[1], normal_record))
    }

    /// The refusal of [`MemStore::collation_read`] for a read whose open filters are the equalities `ties` (field, text): the rows
    /// whose text ties with one ([`al_builtins::collation::ae_fold`], found in [`MemStore::fold_index`]) are checked.
    fn tie_read(
        &self,
        cx: &AccessContext,
        t: &TableDef,
        slot: &Slot,
        filters: &[FieldFilter],
        ties: &[(usize, &str)],
    ) -> Option<String> {
        let table = self.table(cx, t)?;
        for &(c, x) in ties {
            let index = self.fold_index(slot, c, &table);
            for k in index.get(&al_builtins::collation::ae_fold(x)).into_iter().flatten() {
                let row = table.get(k)?;
                if let Err(why) = al_builtins::filter::row_passes_collation(row, filters, self.holds_normal_records()) {
                    return Some(why);
                }
            }
        }
        None
    }

    /// The rows of `table` whose text in column `c` holds `Æ`, `æ` or `ae` ([`al_builtins::collation::ae_equality_reachable`]), by
    /// that text folded ([`al_builtins::collation::ae_fold`]); kept like a verdict, for the column alone.
    fn fold_index(&self, slot: &Slot, c: usize, table: &Table) -> std::rc::Rc<FoldIndex> {
        let ck = (slot.clone(), c);
        if let Some((at, index)) = self.fold_indexes.borrow().get(&ck)
            && self.still_holds(slot, at, &[c])
        {
            return index.clone();
        }
        let mut index = FoldIndex::new();
        for (k, row) in table.iter() {
            #[cfg(test)]
            COLLATION_ROWS.with(|n| n.set(n.get() + 1));
            if let Value::Text(s) | Value::Code(s) = &row[c]
                && al_builtins::collation::ae_equality_reachable(s)
            {
                index
                    .entry(al_builtins::collation::ae_fold(s))
                    .or_default()
                    .push(k.clone());
            }
        }
        let index = std::rc::Rc::new(index);
        self.fold_indexes
            .borrow_mut()
            .insert(ck, (self.kept_at(slot), index.clone()));
        index
    }

    /// The version of `slot` now and the Modify run that ended there, to keep an answer computed now ([`KeptAt`]).
    fn kept_at(&self, slot: &Slot) -> KeptAt {
        let version = self.versions.get(slot).copied().unwrap_or(0);
        let run = self.modify_runs.get(slot).filter(|r| r.to == version).map(|r| r.id);
        KeptAt { version, run }
    }

    /// Whether an answer kept at `at` still holds for the rows of `slot` as far as the columns `cols` go: the table is at the same
    /// version, or only Modifies ran since (in the same run, or a run that started there) and none changed one of `cols`.
    fn still_holds(&self, slot: &Slot, at: &KeptAt, cols: &[usize]) -> bool {
        let now = self.versions.get(slot).copied().unwrap_or(0);
        at.version == now
            || self.modify_runs.get(slot).filter(|r| r.to == now).is_some_and(|r| {
                (at.version == r.from || at.run == Some(r.id))
                    && r.changed.iter().all(|(c, v)| *v <= at.version || !cols.contains(c))
            })
    }

    /// The verdict kept for a read of `slot` in `key` through `filters` that still holds: kept at the table's
    /// version now, or at a version since which only Modifies ran that changed none of `cols`.
    fn kept_verdict(
        &self,
        slot: &Slot,
        key: Option<usize>,
        filters: &[FieldFilter],
        cols: &[usize],
    ) -> Option<Option<String>> {
        let mut cache = self.collation_verdicts.borrow_mut();
        let kept = cache.get_mut(slot)?;
        let i = kept
            .iter()
            .position(|v| v.key == key && same_filters(&v.filters, filters) && self.still_holds(slot, &v.at, cols))?;
        kept[..=i].rotate_right(1);
        Some(kept[0].refusal.clone())
    }

    fn keep_verdict(&self, slot: Slot, key: Option<usize>, filters: &[FieldFilter], refusal: Option<String>) {
        let at = self.kept_at(&slot);
        let mut cache = self.collation_verdicts.borrow_mut();
        let kept = cache.entry(slot).or_default();
        kept.retain(|v| !(v.key == key && same_filters(&v.filters, filters)));
        kept.insert(
            0,
            Verdict {
                at,
                key,
                filters: filters.to_vec(),
                refusal,
            },
        );
        kept.truncate(VERDICTS_PER_TABLE);
    }

    /// Writes down a Modify of a table with noted columns (see [`ModifyRun`]): `before` is the table's version before it,
    /// `changed` the columns whose value it changed.
    pub(super) fn note_modify(&mut self, slot: &Slot, before: u64, changed: Vec<usize>) {
        let to = self.versions.get(slot).copied().unwrap_or(0);
        let run = self.modify_runs.entry(slot.clone()).or_default();
        if run.to != before {
            *run = ModifyRun {
                id: run.id + 1,
                from: before,
                to,
                changed: Vec::new(),
            };
        }
        run.to = to;
        for c in changed {
            match run.changed.iter_mut().find(|(i, _)| *i == c) {
                Some(e) => e.1 = to,
                None => run.changed.push((c, to)),
            }
        }
    }

    /// Whether this store holds the rows of normal records (the database), the only records the measurements of the letters
    /// `Æ` and `É` cover (al-runner issues 8195 and 8196).
    pub fn holds_normal_records(&self) -> bool {
        self.database
    }

    /// Writes down the columns of `row` that hold a value no read may compare (see `unknown_cols`, [`unread`]), a text outside ASCII (see
    /// `accent_cols`), and one with `Æ` or `æ` (see `ae_cols`).
    pub(super) fn note_held(&mut self, t: &TableDef, row: &Row) {
        self.note_held_in(t.object, row);
    }

    /// [`MemStore::note_held`] for a row of the table with object index `object`: for rows put in without a
    /// `TableDef` (`MemStore::with_committed_row_changes`).
    pub(super) fn note_held_in(&mut self, object: u32, row: &Row) {
        let note = |map: &mut std::collections::HashMap<u32, Vec<usize>>, i: usize| {
            let held = map.entry(object).or_default();
            if !held.contains(&i) {
                held.push(i);
            }
        };
        for (i, v) in row.iter().enumerate() {
            match v {
                _ if unread(v).is_some() => note(&mut self.unknown_cols, i),
                Value::Text(s) | Value::Code(s) if !s.is_ascii() => {
                    note(&mut self.accent_cols, i);
                    if al_builtins::collation::holds_ae(s) {
                        note(&mut self.ae_cols, i);
                    }
                }
                _ => {}
            }
        }
    }
}

/// The refusal of an order of two rows next to each other in the order of `key`: the first key field where they differ is a text
/// whose order rests on `Æ` or `æ` ([`al_builtins::collation::ae_decides_order`]).
fn order_refusal(t: &TableDef, key: usize, a: &[Value], b: &[Value], normal_record: bool) -> Option<String> {
    let i = t.keys[key]
        .iter()
        .chain(&t.keys[0])
        .copied()
        .find(|&i| t.cmp_key_field(i, &a[i], &b[i]) != Ordering::Equal)?;
    let (Value::Text(x) | Value::Code(x), Value::Text(y) | Value::Code(y)) = (&a[i], &b[i]) else {
        return None;
    };
    let both_text = matches!((&a[i], &b[i]), (Value::Text(_), Value::Text(_)));
    al_builtins::collation::ae_decides_order(x, y, both_text, normal_record).then(|| {
        format!(
            "an order by the field {} of table {} between '{}' and '{}' (the letters Æ and æ are measured only in a Text field of a normal record, the capital Æ)",
            t.fields[i].name,
            t.name,
            x.as_str(),
            y.as_str()
        )
    })
}

/// Whether two filter lists are the same filters in the same order ([`FieldFilter::identical`]).
fn same_filters(a: &[FieldFilter], b: &[FieldFilter]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.identical(y))
}

/// How many verdicts of [`MemStore::collation_read`] one table keeps, the one used last first.
const VERDICTS_PER_TABLE: usize = 8;

/// When an answer of [`MemStore::collation_read`] was kept: the table's version, and the Modify run that ended there.
#[derive(Clone)]
pub(super) struct KeptAt {
    version: u64,
    run: Option<u64>,
}

/// Per folded text, the primary keys of the rows that hold it ([`MemStore::fold_index`]).
pub(super) type FoldIndex = std::collections::HashMap<String, Vec<Key>>;

/// One answer of [`MemStore::collation_read`]: for a table as of `at`, read in `key` through `filters` (compared by [`FieldFilter::identical`]).
#[derive(Clone)]
pub(super) struct Verdict {
    at: KeptAt,
    key: Option<usize>,
    filters: Vec<FieldFilter>,
    refusal: Option<String>,
}

/// The Modifies of one table since its last other write: the table's version before the first (`from`) and after the last
/// (`to`), and per column the version after the last Modify that changed it. A rollback or any other write starts a new run
/// (`id`): a version inside an earlier run may name rows a rollback undid.
#[derive(Clone, Default)]
pub(super) struct ModifyRun {
    id: u64,
    from: u64,
    to: u64,
    changed: Vec<(usize, u64)>,
}

#[cfg(test)]
thread_local! {
    /// How many rows [`MemStore::collation_read`] looked at on this thread.
    pub(super) static COLLATION_ROWS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}
