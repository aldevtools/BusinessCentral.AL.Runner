//! The lookup of a fact vector in the save table (ADR-040 amendment 2026-11-13, part 1, point 2). Pure: no `Session`.
//!
//! A row matches when every fact it reads equals the vector's value. Rows that overlap nest (the table checks below), so the
//! matching rows form a chain and the most specific one answers. Before the run the run facts are not known: a row that
//! reads none of them and answers without a run answers then; otherwise the report runs when some row can still match, and
//! the lookup is repeated with the run facts. No row: [`Miss`], whose reason names every fact that keeps the save from a row.

use super::cell::{Cell, Evidence, K, Key};
use super::facts::{Describe, Fact, Facts};

/// Applies `$body` to each fact: `$f` the field, `$fact` its [`Fact`].
macro_rules! each_fact {
    ($m:ident) => {
        $m!(
            format: Format,
            target: Target,
            answer: Answer,
            kind: Kind,
            layout: Layout,
            decl: Decl,
            file: File,
            processing_only: ProcessingOnly,
            data_items: DataItems,
            extensions: Extensions,
            triggers: Triggers,
            custom: Custom,
            company_build: CompanyBuild,
            listed: Listed,
            selection: Selection,
            stored: Stored,
        )
    };
}

/// The value a key reads for `fact`, as words; `None` when it does not read it.
fn key_value(key: &Key, fact: Fact) -> Option<&'static str> {
    fn read<T: Describe>(k: &K<T>) -> Option<&'static str> {
        match k {
            K::Is(v) => Some(v.describe()),
            K::Unread => None,
        }
    }
    macro_rules! arms {
        ($($f:ident: $fact:ident),* $(,)?) => {
            match fact {
                $(Fact::$fact => read(&key.$f),)*
                Fact::Named => read(&key.named),
                Fact::Rows => read(&key.rows),
                Fact::DatasetRgba => read(&key.dataset_rgba),
            }
        };
    }
    each_fact!(arms)
}

/// Whether `key` admits the vector's value of `fact`: it does not read it, the values are equal, or the value is a run
/// fact not known yet.
fn admits(key: &Key, facts: &Facts, fact: Fact) -> bool {
    fn one<T: PartialEq>(k: &K<T>, v: &T) -> bool {
        matches!(k, K::Unread) || matches!(k, K::Is(x) if x == v)
    }
    fn opt<T: PartialEq>(k: &K<T>, v: &Option<T>) -> bool {
        v.as_ref().is_none_or(|v| one(k, v))
    }
    macro_rules! arms {
        ($($f:ident: $fact:ident),* $(,)?) => {
            match fact {
                $(Fact::$fact => one(&key.$f, &facts.$f),)*
                Fact::Named => opt(&key.named, &facts.named),
                Fact::Rows => opt(&key.rows, &facts.rows),
                Fact::DatasetRgba => opt(&key.dataset_rgba, &facts.dataset_rgba),
            }
        };
    }
    each_fact!(arms)
}

/// Whether the key reads `fact`.
pub(super) fn reads(key: &Key, fact: Fact) -> bool {
    key_value(key, fact).is_some()
}

fn matches(key: &Key, facts: &Facts) -> bool {
    Fact::ALL.iter().all(|&f| admits(key, facts, f))
}

/// Whether `a` reads every fact `b` reads, with the same value (`a` is at least as specific as `b`).
pub(super) fn refines(a: &Key, b: &Key) -> bool {
    Fact::ALL
        .iter()
        .all(|&f| key_value(b, f).is_none() || key_value(a, f) == key_value(b, f))
}

/// Whether some vector matches both keys.
#[cfg(test)]
pub(super) fn overlap(a: &Key, b: &Key) -> bool {
    Fact::ALL.iter().all(|&f| match (key_value(a, f), key_value(b, f)) {
        (Some(x), Some(y)) => x == y,
        _ => true,
    })
}

/// The rows that match, most specific last.
fn matching<'t>(table: &'t [Cell], facts: &Facts) -> Vec<&'t Cell> {
    let mut rows: Vec<&Cell> = table.iter().filter(|c| matches(&c.key, facts)).collect();
    rows.sort_by_key(|c| Fact::ALL.iter().filter(|&&f| reads(&c.key, f)).count());
    rows
}

/// What the lookup found before the run.
#[derive(Debug)]
pub(super) enum Before<'t> {
    /// A row answers without a run.
    Answer(&'t Cell),
    /// The report runs; the rows that can still match.
    Run(Vec<&'t Cell>),
    Miss(Miss),
}

/// The lookup before the run (the run facts of `facts` are `None`).
pub(super) fn before_run<'t>(table: &'t [Cell], facts: &Facts) -> Before<'t> {
    let rows = matching(table, facts);
    let plain = rows
        .iter()
        .rev()
        .find(|c| !Fact::ALL.iter().any(|f| f.after_run() && reads(&c.key, *f)));
    if let Some(c) = plain
        && c.answer.before_run()
    {
        return Before::Answer(c);
    }
    if rows.is_empty() {
        return Before::Miss(Miss::of(table, facts));
    }
    Before::Run(rows)
}

/// The assumption every row the run can end in marks, when they all mark the same one: the rows the lookup after the run
/// answers for each value of the run facts.
pub(super) fn one_mark_after_any_run(table: &[Cell], facts: &Facts) -> Option<al_builtins::assumptions::Assumption> {
    use super::facts::Rows;
    let mut marks = Vec::new();
    for rows in [Rows::Zero, Rows::Some] {
        for named in [false, true] {
            for dataset_rgba in [false, true] {
                let after = Facts {
                    rows: Some(rows),
                    named: Some(named),
                    dataset_rgba: Some(dataset_rgba),
                    ..*facts
                };
                if let Ok(c) = after_run(table, &after) {
                    marks.push(c.evidence.marks());
                }
            }
        }
    }
    let first = *marks.first()?;
    marks.iter().all(|m| *m == first).then_some(first).flatten()
}

/// The lookup after the run (every fact known).
pub(super) fn after_run<'t>(table: &'t [Cell], facts: &Facts) -> Result<&'t Cell, Miss> {
    matching(table, facts)
        .last()
        .copied()
        .ok_or_else(|| Miss::of(table, facts))
}

/// Whether some row of the table is about this save's kind of save at all: it admits the vector on every fact but the
/// layout selection, the stored row, the allow-list and the run facts. A miss of a save the table has no row about is the
/// save the ADR-040 allow-list may discard (`policy`).
pub(super) fn knows(table: &[Cell], facts: &Facts) -> bool {
    let skip = |f: Fact| f.after_run() || matches!(f, Fact::Selection | Fact::Stored | Fact::Listed);
    table
        .iter()
        .any(|c| Fact::ALL.iter().all(|&f| skip(f) || admits(&c.key, facts, f)))
}

/// Why the rgba replace is not measured on the render of `facts`: the facts that differ from the nearest rows whose rgba
/// column is measured, with their corpus lines.
pub(super) fn rgba_reason(table: &[Cell], facts: &Facts) -> String {
    let mut near: Vec<(Vec<String>, String)> = table
        .iter()
        .filter(|c| c.rgba)
        .map(|c| {
            let d = Fact::ALL
                .iter()
                .filter(|&&f| !admits(&c.key, facts, f))
                .map(|&f| {
                    format!(
                        "{} {} (row: {})",
                        f.name(),
                        facts.describe(f),
                        key_value(&c.key, f).unwrap_or("")
                    )
                })
                .collect();
            (d, c.evidence.cite())
        })
        .collect();
    near.sort_by_key(|(d, _)| d.len());
    let best = near.first().map_or(0, |(d, _)| d.len());
    let parts: Vec<String> = near
        .iter()
        .filter(|(d, _)| d.len() == best)
        .take(3)
        .map(|(d, cite)| format!("{} [{cite}]", d.join(", ")))
        .collect();
    format!("the nearest measured renders differ in: {}", parts.join("; "))
}

/// One differing fact: the fact, the save's value, the row's value.
pub(crate) type Differs = (Fact, &'static str, &'static str);

/// A save no row answers: the facts that keep it from a row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Miss {
    /// Facts whose value no row of the save's format has: (fact, value, the measured values).
    pub never: Vec<(Fact, &'static str, Vec<&'static str>)>,
    /// The nearest rows: for each, the differing facts (fact, value, the row's value) and the row's corpus line.
    pub nearest: Vec<(Vec<Differs>, String)>,
}

impl Miss {
    fn of(table: &[Cell], facts: &Facts) -> Miss {
        let of_format: Vec<&Cell> = table.iter().filter(|c| admits(&c.key, facts, Fact::Format)).collect();
        let mut never = Vec::new();
        for &fact in Fact::ALL.iter().filter(|f| **f != Fact::Format && known(facts, **f)) {
            // No row of the format reads the value, and every row that matches the vector on all the other facts reads the
            // fact (a row that does not read it would take the save).
            let value = facts.describe(fact);
            let others = |c: &&&Cell| Fact::ALL.iter().all(|&g| g == fact || admits(&c.key, facts, g));
            let near: Vec<&&Cell> = of_format.iter().filter(others).collect();
            if of_format.iter().any(|c| key_value(&c.key, fact) == Some(value))
                || near.is_empty()
                || near.iter().any(|c| !reads(&c.key, fact))
            {
                continue;
            }
            let mut measured: Vec<&'static str> = of_format.iter().filter_map(|c| key_value(&c.key, fact)).collect();
            measured.sort_unstable();
            measured.dedup();
            never.push((fact, facts.describe(fact), measured));
        }
        let differing = |c: &Cell| -> Vec<Differs> {
            Fact::ALL
                .iter()
                .filter(|&&f| !admits(&c.key, facts, f))
                .map(|&f| (f, facts.describe(f), key_value(&c.key, f).unwrap_or("")))
                .collect()
        };
        // Measured rows only: a policy row is an owner decision, not a cell to probe next to.
        let mut nearest: Vec<_> = of_format
            .iter()
            .filter(|c| !matches!(c.evidence, Evidence::Policy(_)))
            .map(|c| (differing(c), c.evidence.cite(), c))
            .collect();
        // Fewest differing facts first; among those, the more specific rows (closer measured cells), then table order.
        nearest.sort_by_key(|(d, _, c)| {
            (
                d.len(),
                std::cmp::Reverse(Fact::ALL.iter().filter(|&&f| reads(&c.key, f)).count()),
            )
        });
        let nearest: Vec<_> = nearest.into_iter().map(|(d, cite, _)| (d, cite)).collect();
        Miss {
            never,
            nearest: nearest.into_iter().take(3).collect(),
        }
    }

    /// The refusal text: `what` names the save.
    pub(crate) fn reason(&self, what: &str) -> String {
        let mut out = format!("{what}: not measured on BC.");
        if !self.never.is_empty() {
            let parts: Vec<String> = self
                .never
                .iter()
                .map(|(f, v, m)| format!("{} = {v} (measured: {})", f.name(), m.join("; ")))
                .collect();
            out.push_str(&format!(" Never measured: {}.", parts.join(", ")));
        }
        if !self.nearest.is_empty() {
            let parts: Vec<String> = self
                .nearest
                .iter()
                .map(|(d, cite)| {
                    let facts: Vec<String> = d
                        .iter()
                        .map(|(f, v, r)| format!("{} {v} (row: {r})", f.name()))
                        .collect();
                    format!("{} [{cite}]", facts.join(", "))
                })
                .collect();
            out.push_str(&format!(" Nearest measured rows differ in: {}.", parts.join("; ")));
        }
        out
    }
}

fn known(facts: &Facts, fact: Fact) -> bool {
    match fact {
        Fact::Named => facts.named.is_some(),
        Fact::Rows => facts.rows.is_some(),
        Fact::DatasetRgba => facts.dataset_rgba.is_some(),
        _ => true,
    }
}
