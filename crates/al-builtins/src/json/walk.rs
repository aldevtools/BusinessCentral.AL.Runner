//! `SelectToken` over a path without a filter: every step is followed over all its matches, then the path must end on exactly
//! one token (json/DbJPathLeads.al:465-873 `P6924_*`, `Union_OfKeys`, `LastArrayElement`).

use super::{Handle, JsonNode, Seg, descendants_named, error, tagged, with_node};
use crate::native::{Ctx, Signal};
use crate::value::Value;

fn children(h: &Handle) -> Vec<Handle> {
    with_node(h, |n| match n {
        JsonNode::Object(kids) => kids.iter().map(|(_, c)| c.clone()).collect(),
        JsonNode::Array(kids) => kids.clone(),
        _ => Vec::new(),
    })
}

/// `h` and everything below it, in document order.
fn below(h: &Handle, out: &mut Vec<Handle>) {
    for c in children(h) {
        out.push(c.clone());
        below(&c, out);
    }
}

/// The elements `[start:end:step]` picks from `len` (Python rules for a negative start; a negative end, a zero or a negative
/// step are not pinned).
fn slice_range(len: usize, start: Option<i64>, end: Option<i64>, step: Option<i64>) -> Result<Vec<usize>, Signal> {
    let unpinned =
        || Signal::Unsupported("SelectToken slice with a negative end or a step below 1 (not pinned)".into());
    if end.is_some_and(|e| e < 0) || step.is_some_and(|s| s < 1) {
        return Err(unpinned());
    }
    let n = i64::try_from(len).unwrap_or(i64::MAX);
    let from = match start {
        None => 0,
        Some(s) if s < 0 => (n + s).max(0),
        Some(s) => s.min(n),
    };
    let to = end.map_or(n, |e| e.min(n));
    let step = usize::try_from(step.unwrap_or(1)).unwrap_or(1);
    Ok((usize::try_from(from).unwrap_or(0)..usize::try_from(to).unwrap_or(0))
        .step_by(step)
        .collect())
}

/// The tokens one step reaches from `h`. `Err(None)` is a negative index (ArgumentOutOfRange).
fn step(h: &Handle, seg: &Seg) -> Result<Vec<Handle>, Option<Signal>> {
    Ok(match seg {
        Seg::Descent(name) => {
            let mut out = Vec::new();
            descendants_named(h, name, &mut out);
            out
        }
        Seg::DescentAny => {
            let mut out = Vec::new();
            below(h, &mut out);
            out
        }
        Seg::DescentIndex(i) => {
            let mut all = vec![h.clone()];
            below(h, &mut all);
            let i = usize::try_from(*i).map_err(|_| None)?;
            all.iter()
                .filter_map(|n| {
                    with_node(n, |n| match n {
                        JsonNode::Array(k) => k.get(i).cloned(),
                        _ => None,
                    })
                })
                .collect()
        }
        _ => {
            let r: Result<Vec<Handle>, Option<Signal>> = with_node(h, |node| match (node, seg) {
                (JsonNode::Object(kids), Seg::Key(k)) => {
                    Ok(kids.iter().filter(|(e, _)| e == k).map(|(_, c)| c.clone()).collect())
                }
                (JsonNode::Object(kids), Seg::Union(keys)) => Ok(keys
                    .iter()
                    .flat_map(|k| kids.iter().filter(move |(e, _)| e == k).map(|(_, c)| c.clone()))
                    .collect()),
                (JsonNode::Object(kids), Seg::ObjectWild) => {
                    if kids.len() < 2 {
                        return Err(Some(Signal::Unsupported(
                            "SelectToken `.*` on an object with fewer than two members (not pinned)".into(),
                        )));
                    }
                    Ok(kids.iter().map(|(_, c)| c.clone()).collect())
                }
                (JsonNode::Array(kids), Seg::Index(i)) => match usize::try_from(*i) {
                    Ok(i) => Ok(kids.get(i).cloned().into_iter().collect()),
                    Err(_) => Err(None),
                },
                (JsonNode::Array(kids), Seg::Wild) => Ok(kids.clone()),
                (JsonNode::Array(kids), Seg::Slice(a, z, s)) => Ok(slice_range(kids.len(), *a, *z, *s)
                    .map_err(Some)?
                    .into_iter()
                    .map(|i| kids[i].clone())
                    .collect()),
                _ => Ok(Vec::new()),
            });
            r?
        }
    })
}

pub(super) fn select(
    c: &mut dyn Ctx,
    root: Handle,
    segs: &[Seg],
    p: &str,
    used: bool,
    out: &mut Value,
) -> Result<Value, Signal> {
    let mut cur = vec![root];
    for seg in segs {
        let mut next = Vec::new();
        for h in &cur {
            match step(h, seg) {
                Ok(v) => next.extend(v),
                Err(Some(e)) => return Err(e),
                // A negative index with the result used: false and ArgumentOutOfRange as the last error (DbJPathLeads.al:154
                // `index_minus_1`); as a statement it is not pinned.
                Err(None) if used => {
                    c.set_last_error(crate::native::AlError {
                        message: "An invalid argument was passed to a 'Json' data type method.".into(),
                        code: "ArgumentOutOfRange".into(),
                        ..Default::default()
                    });
                    return Ok(Value::Boolean(false));
                }
                Err(None) => {
                    return Err(Signal::Unsupported(
                        "SelectToken with a negative index, result unused (not pinned)".into(),
                    ));
                }
            }
        }
        cur = next;
    }
    let loose = segs.iter().any(|s| matches!(s, Seg::DescentAny | Seg::DescentIndex(_)));
    match (cur.len(), used) {
        (1, _) => {
            *out = tagged("JsonToken", cur.pop().expect("one"));
            Ok(Value::Boolean(true))
        }
        (_, true) => Ok(Value::Boolean(false)),
        // `..*` and `..[n]` are pinned with the result used only.
        _ if loose => Err(Signal::Unsupported(
            "SelectToken `..*` or `..[n]` without one match, result unused (not pinned)".into(),
        )),
        (0, false) => Err(error(
            format!("The JSON path '{p}' does not match any tokens."),
            "InvalidJsonPathSelection",
        )),
        _ => Err(error(
            format!("The set of tokens matching the JSON path '{p}' contains more than one element."),
            "InvalidJsonPathSelection",
        )),
    }
}
