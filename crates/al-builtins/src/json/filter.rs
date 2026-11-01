//! `SelectToken` paths with a filter indexer `[?(@.name == x)]`, `!=`, `>` or `<=`: the forms
//! json/lt-db-json-leads/DbJPathLeads.al:52-85 and json/lt-dc-json-leads/DcJsonLeads.al:286-345 pin, and the `Y47_*` tests
//! of DbJPathLeads.al on corpus branch al-runtime-rs-staging-y47 (al-runner issue 6588, measured on real BC).

use super::{Handle, JsonNode, Scalar, Seg, error, tagged, with_node};
use crate::native::{Ctx, Signal};
use crate::value::Value;

/// The right side of a filter's comparison.
pub(super) enum Operand {
    /// A whole number: `@.salary==200` (DbJPathLeads.al:61), `@.id==1` (DcJsonLeads.al:294).
    Int(i64),
    /// A text in single quotes: `@.id=='John'` (DbJPathLeads.al:58), `@.name=='z'` (DcJsonLeads.al:310).
    Text(String),
}

/// The comparison of a filter.
#[derive(Clone, Copy, PartialEq)]
enum Op {
    Eq,
    /// `!=` (`Y47_ArrayFilter_Neighbours` `ne`).
    Ne,
    /// `>` (DbJPathLeads.al:62).
    Gt,
    /// `<=` (`Y47_ArrayFilter_Neighbours` `le`).
    Le,
}

/// One filter indexer: `@.<name> <op> <operand>`; only `==` compares a text.
pub(super) struct Filter {
    name: String,
    op: Op,
    rhs: Operand,
}

/// Why a filter indexer is not a [`Filter`].
pub(super) enum FilterErr {
    /// A path the platform cannot read: a text in double quotes, `@.id=="John"` (DbJPathLeads.al:57, :60, :67-69).
    Invalid,
    Refused(&'static str),
}

/// The filter of the indexer text `inner` (between `[` and `]`), which starts with `?`.
pub(super) fn parse(inner: &str) -> Result<Filter, FilterErr> {
    let refused = FilterErr::Refused;
    let Some(f) = inner.strip_prefix("?(").and_then(|s| s.strip_suffix(')')) else {
        return Err(refused("SelectToken filter form not pinned by the corpus"));
    };
    let ops = [("==", Op::Eq), ("!=", Op::Ne), ("<=", Op::Le), (">", Op::Gt)];
    let found: Vec<(&str, &str, Op)> = ops
        .iter()
        .filter_map(|(t, op)| f.split_once(t).map(|(l, r)| (l, r, *op)))
        .collect();
    let not_one = "SelectToken filter that is not one ==, !=, > or <= comparison (not pinned)";
    let [(lhs, rhs, op)] = found[..] else {
        return Err(refused(not_one));
    };
    if rhs.starts_with('=') || f.contains(['&', '|']) || (f.contains('<') && op != Op::Le) {
        return Err(refused(not_one));
    }
    let name = lhs.trim().strip_prefix("@.").unwrap_or("");
    if name.is_empty() || !name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_') {
        return Err(refused("SelectToken filter on something else than @.name (not pinned)"));
    }
    let rhs = rhs.trim();
    let operand = if let Ok(n) = rhs.parse::<i64>() {
        Operand::Int(n)
    } else if rhs.len() >= 2 && rhs.starts_with('"') && rhs.ends_with('"') {
        return Err(FilterErr::Invalid);
    } else if let Some(t) = rhs.strip_prefix('\'').and_then(|s| s.strip_suffix('\''))
        && !t.contains(['\'', '\\'])
        && op == Op::Eq
    {
        Operand::Text(t.to_string())
    } else {
        return Err(refused(
            "SelectToken filter against something else than a whole number or a quoted text (not pinned)",
        ));
    };
    Ok(Filter {
        name: name.to_string(),
        op,
        rhs: operand,
    })
}

/// The elements of the array `h` that `f` keeps, in document order.
fn keep(h: &Handle, f: &Filter) -> Result<Vec<Handle>, Signal> {
    let refuse = |what: &str| Err(Signal::Unsupported(format!("SelectToken: {what} (not pinned)")));
    let Some(kids) = with_node(h, |n| match n {
        JsonNode::Array(kids) => Some(kids.clone()),
        _ => None,
    }) else {
        return refuse("a filter on a token that is not an array");
    };
    let mut out = Vec::new();
    for k in kids {
        let field = with_node(&k, |n| match n {
            JsonNode::Object(o) => Ok(o.iter().find(|(e, _)| *e == f.name).map(|(_, c)| c.clone())),
            _ => Err(()),
        });
        // An element without the name is left out, as SelectTokens leaves it out (json/lt-json-selecttokens/
        // SelectTokensList.al:265 `{"x":30}`).
        let field = match field {
            Ok(Some(c)) => c,
            Ok(None) => continue,
            Err(()) => return refuse("a filter over an element that is not an object"),
        };
        let hit = with_node(&field, |n| match (n, &f.rhs) {
            (JsonNode::Scalar(Scalar::Val(Value::Integer(v))), Operand::Int(r)) => Some(i64::from(*v).cmp(r)),
            (JsonNode::Scalar(Scalar::Val(Value::BigInteger(v))), Operand::Int(r)) => Some((*v).cmp(r)),
            (JsonNode::Scalar(Scalar::Val(Value::Text(v))), Operand::Text(r)) => Some(v.as_str().cmp(r.as_str())),
            // A text against a number matches nothing (`Y47_ArrayFilter_Neighbours` `str_vs_number`, `@.id=='2'`).
            (JsonNode::Scalar(Scalar::Val(Value::Integer(_) | Value::BigInteger(_))), Operand::Text(_)) => {
                Some(std::cmp::Ordering::Less)
            }
            _ => None,
        });
        let keep = match (hit, f.op) {
            (None, _) => return refuse("a filter that compares a value of another kind"),
            (Some(o), Op::Eq) => o.is_eq(),
            (Some(o), Op::Ne) => o.is_ne(),
            (Some(o), Op::Gt) => o.is_gt(),
            (Some(o), Op::Le) => o.is_le(),
        };
        if keep {
            out.push(k);
        }
    }
    Ok(out)
}

/// `SelectToken` over a path with a filter: every match of each step is followed, then the path must end on exactly one
/// token. One: true and the token (DcJsonLeads.al:294, :300). None or more than one: false when the result is used, with no
/// last error (DbJPathLeads.al:59, :62; DcJsonLeads.al:296, :298), else "The JSON path '<path>' does not match any tokens." or
/// "The set of tokens matching the JSON path '<path>' contains more than one element." (InvalidJsonPathSelection,
/// DcJsonLeads.al:302-305, :333-336).
pub(super) fn select(root: Handle, segs: &[Seg], p: &str, used: bool, out: &mut Value) -> Result<Value, Signal> {
    let mut cur = vec![root];
    for seg in segs {
        let mut next = Vec::new();
        for h in &cur {
            match seg {
                Seg::Filter(f) => next.extend(keep(h, f)?),
                Seg::Key(k) => next.extend(with_node(h, |n| match n {
                    JsonNode::Object(kids) => kids.iter().filter(|(e, _)| e == k).map(|(_, c)| c.clone()).collect(),
                    _ => Vec::new(),
                })),
                Seg::Index(i) => next.extend(with_node(h, |n| match n {
                    JsonNode::Array(kids) => usize::try_from(*i).ok().and_then(|i| kids.get(i).cloned()),
                    _ => None,
                })),
                _ => {
                    return Err(Signal::Unsupported(
                        "SelectToken with a filter and a wildcard, a slice or `..` (not pinned)".into(),
                    ));
                }
            }
        }
        cur = next;
    }
    match (cur.len(), used) {
        (1, _) => {
            *out = tagged("JsonToken", cur.pop().expect("one"));
            Ok(Value::Boolean(true))
        }
        (_, true) => Ok(Value::Boolean(false)),
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

/// A filter path the platform cannot read with the result used: false, and the last error "The requested operation cannot
/// be performed in this context." (InvalidOperation) without raising it (DbJPathLeads.al:67-69).
pub(super) fn invalid(c: &mut dyn Ctx) -> Value {
    c.set_last_error(crate::native::AlError {
        message: "The requested operation cannot be performed in this context.".into(),
        code: "InvalidOperation".into(),
        ..Default::default()
    });
    Value::Boolean(false)
}
