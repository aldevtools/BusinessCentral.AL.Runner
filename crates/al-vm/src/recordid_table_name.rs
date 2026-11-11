//! The table name of the text `Evaluate(RecordId, Text)` reads, beyond the exact name (`Session::evaluate_record_id_text`):
//! the name in double quotes and a prefix of a table name (recordref-fieldref/N582EvalFieldRefUnused.al:143-169
//! WA_Evaluate_FieldRef_RecordId_Statement_TableNamePrefix, waldo run 2 of al-runner issue 6924, corpus master 176baa02).

use crate::Session;
use al_builtins::native::Signal;

/// `"<name>"` is read as `<name>` (N582EvalFieldRefUnused.al:151 `name in quotes`, :165-167: `"LT WA RecId Row": 2` reads
/// back `LT WA RecId Row: 2`).
pub(crate) fn unquoted(name: &str) -> &str {
    match name.strip_prefix('"').and_then(|n| n.strip_suffix('"')) {
        Some(n) if !n.contains('"') => n,
        _ => name,
    }
}

impl Session {
    /// The tables whose name starts with `name` (any case), for a text that names no table exactly. One such table is that
    /// table (N582EvalFieldRefUnused.al:147-148 `LT WA RecId R` with and without a key, :152-157). Several: BC took 97240
    /// "LT WA RecId Row" for `LT WA RecId` over 97241 "LT WA RecId Two" (:149, :158-160), the table with the lowest id and the
    /// first name alike; which rule it follows when the two differ is not pinned, and that case is refused. A text that is
    /// longer than every table name it starts with names no table (:150 `name followed by text`, :161-164).
    pub(crate) fn record_id_tables_by_prefix(&self, name: &str) -> Result<Vec<usize>, Signal> {
        if name.is_empty() {
            return Ok(Vec::new());
        }
        let heads = self.engine.program.objects.heads();
        let found: Vec<usize> = heads
            .iter()
            .enumerate()
            .filter(|(_, o)| {
                o.table.is_some()
                    && o.name.len() > name.len()
                    && o.name.get(..name.len()).is_some_and(|p| p.eq_ignore_ascii_case(name))
            })
            .map(|(i, _)| i)
            .collect();
        if found.len() < 2 {
            return Ok(found);
        }
        let by_id = found.iter().min_by_key(|i| heads[**i].id.unwrap_or(u32::MAX)).copied();
        let by_name = found
            .iter()
            .min_by_key(|i| heads[**i].name.to_ascii_lowercase())
            .copied();
        match (by_id, by_name) {
            (Some(a), Some(b)) if a == b => Ok(vec![a]),
            _ => Err(Signal::unsupported(format!(
                "Evaluate into a RecordId of `{name}`, a prefix of several table names whose lowest id is not the first name (not pinned)"
            ))),
        }
    }
}

/// The parts of the text inside the brackets of a RecordId text (what `Format(RecordId, 0, 9)` writes for a table in a
/// namespace): the namespace, if any, and the table name.
enum BracketedName<'a> {
    /// `NS.Name`, `NS."Name"` or `Name`.
    Name {
        namespace: Option<&'a str>,
        name: &'a str,
        quoted: bool,
    },
    /// A form BC reads as no table: the whole text in quotes around a name with a dot (`["LT.R7003.Data.R7003Row"]`,
    /// `["R7003.Dot"]`) or a quoted namespace before a quoted name (`["LT.R7003.Data"."R7003Row"]`)
    /// (language/namespaces/RecordIdFormat9Evaluate.al:282-283, :311, `No 0 []` on every version).
    NoTable,
}

fn split_bracketed(q: &str) -> Result<BracketedName<'_>, Signal> {
    let refuse = || {
        Signal::unsupported(format!(
            "Evaluate into a RecordId of the bracketed name `[{q}]` (not pinned)"
        ))
    };
    if let Some(rest) = q.strip_prefix('"') {
        let close = rest.find('"').ok_or_else(refuse)?;
        let (first, after) = (&rest[..close], &rest[close + 1..]);
        let quoted_name = after
            .strip_prefix(".\"")
            .and_then(|n| n.strip_suffix('"'))
            .is_some_and(|n| !n.contains('"'));
        return match after {
            "" if first.contains('.') => Ok(BracketedName::NoTable),
            _ if quoted_name => Ok(BracketedName::NoTable),
            _ => Err(refuse()),
        };
    }
    if let Some(body) = q.strip_suffix('"') {
        let open = body.find('"').ok_or_else(refuse)?;
        let namespace = body[..open].strip_suffix('.').ok_or_else(refuse)?;
        let name = &body[open + 1..];
        if namespace.is_empty() || namespace.contains('"') || name.contains('"') {
            return Err(refuse());
        }
        return Ok(BracketedName::Name {
            namespace: Some(namespace),
            name,
            quoted: true,
        });
    }
    if q.contains('"') {
        return Err(refuse());
    }
    Ok(match q.rsplit_once('.') {
        Some((namespace, name)) => BracketedName::Name {
            namespace: Some(namespace),
            name,
            quoted: false,
        },
        None => BracketedName::Name {
            namespace: None,
            name: q,
            quoted: false,
        },
    })
}

impl Session {
    /// The tables a bracketed table name `[q]` of a RecordId text names; empty when it names none (Evaluate answers false).
    /// Corpus language/namespaces/RecordIdFormat9Evaluate.al (al-runner issue 7326):
    /// - `[NS.Name]` and `[NS."Name"]` name the table `Name` of namespace `NS`, letter case as declared: the text
    ///   Format(RecordId, 0, 9) writes for own tables (:95, one word, `_`, `-`, `"."`, `/`, spaces) and Base Application
    ///   tables (:164, :207), and the name quoted by hand (:279-280); P69NsConsumers.al:214 (a name with spaces).
    /// - A namespace and name that name no table: another or a partial namespace (:309-310, :312), an unquoted name with a dot
    ///   (:281), a table without a namespace given one (:315), no such table (:353), and the whole text in lower case (:313).
    ///   Any other text that differs from a table only in letter case is not pinned and refused.
    /// - `[Name]` without a namespace names the table of that name from BC 28 and no table on BC 27, for a table in a
    ///   namespace and one without (:308, :314, `#if BC28PLUS` :328). Several tables of that name, or none, are not pinned.
    ///
    /// `evaluate` is false for a filter text (`record_id_filter_text`): only `[NS.Name]` with a space in the name is read
    /// there, as before; every other bracketed form is not pinned for a filter.
    pub(crate) fn record_id_tables_bracketed(&self, q: &str, evaluate: bool) -> Result<Vec<usize>, Signal> {
        let (namespace, name, quoted) = match split_bracketed(q)? {
            BracketedName::NoTable if evaluate => return Ok(Vec::new()),
            BracketedName::Name {
                namespace,
                name,
                quoted,
            } => (namespace, name, quoted),
            BracketedName::NoTable => (None, "", true),
        };
        if !evaluate && (quoted || namespace.is_none() || !name.contains(' ')) {
            return unsupported_bracketed_filter(q);
        }
        let heads = self.engine.program.objects.heads();
        let tables_where = |exact: bool| -> Vec<usize> {
            let same = |a: &str, b: &str| if exact { a == b } else { a.eq_ignore_ascii_case(b) };
            heads
                .iter()
                .enumerate()
                .filter(|(_, o)| {
                    o.table.is_some()
                        && same(&o.name, name)
                        && namespace.is_none_or(|ns| o.namespace.as_deref().is_some_and(|own| same(own, ns)))
                })
                .map(|(i, _)| i)
                .collect()
        };
        let exact = tables_where(true);
        if namespace.is_none() {
            return match exact.len() {
                1 if self.has(al_version::Gate::BracketedRecordIdNameWithoutNamespaceFrom28) => Ok(exact),
                1 => Ok(Vec::new()),
                _ => Err(Signal::unsupported(format!(
                    "Evaluate into a RecordId of `[{q}]`, a name without a namespace that {} tables have (not pinned)",
                    exact.len()
                ))),
            };
        }
        if !exact.is_empty() {
            return Ok(exact);
        }
        if !evaluate {
            return unsupported_bracketed_filter(q);
        }
        if tables_where(false).is_empty() || q == q.to_lowercase() {
            return Ok(Vec::new());
        }
        Err(Signal::unsupported(format!(
            "Evaluate into a RecordId of `[{q}]`, which names a table only in another letter case (not pinned)"
        )))
    }
}

fn unsupported_bracketed_filter(q: &str) -> Result<Vec<usize>, Signal> {
    Err(Signal::unsupported(format!(
        "SetFilter on a RecordId field of the bracketed name `[{q}]` (not pinned for a filter)"
    )))
}
