//! Text methods on a call stack text that holds the untracked-frame mark (`CALL_STACK_MARK`, al-runner issue 7520).
//!
//! The mark stands for frames BC has and this runtime does not track, so the text after the first mark is unknown. A method
//! answers when its result depends only on the text before that mark, the known prefix, and every other result stays
//! refused. No new semantics: each answer is what the pinned method gives on the prefix, and a result that reaches the mark
//! keeps it, so every later read of it is held to the same rule (the same reasoning as the comparison of marked texts,
//! `ops::known_prefix_order`, and of JSON strings that hold a mark).
//!
//! What answers (P is the known prefix, the mark is `M`):
//! - `IndexOf`, `StrPos`, `Contains`, `StartsWith`: a needle found in P (the first match lies in P, as a match that reaches the
//!   mark starts later); `StartsWith` also answers false when the needle differs from P inside both.
//! - `Substring`, `CopyStr`, `DelStr`, `Remove`: a range that starts in P. A range that ends in P gives a text without the mark;
//!   one that reaches the mark gives the part of P with the mark kept (`DelStr` and `Remove` without a count delete to the end, so
//!   they give P up to the start, without the mark).
//! - `ToUpper`, `ToLower`, `UpperCase`, `LowerCase`, `ConvertStr`: the same mapping of P, then `M`.
//! - `Replace`: P with the matches in it replaced, then `M`, when no match could start in P and run on into the tail.
//! - `SelectStr`: an item that ends at a comma inside P.
//!
//! Everything else refuses with [`REASON`]: `StrLen`, `EndsWith`, `LastIndexOf`, `PadStr`, `Split` (the number of parts depends
//! on the tail), `Format` with a length, a range that reaches into the tail where BC may raise, an `IndexOf` that finds nothing
//! in P. The pinned behavior of the tail itself is a probe (docs/probes-needed.md).

use crate::native::{CALL_STACK_MARK, Ctx, Signal};
use crate::text::Txt;
use crate::value::Value;

type NResult = Result<Value, Signal>;
type Native = fn(&mut dyn Ctx, Option<&mut Value>, &mut [Value]) -> NResult;

/// The refusal for a result that depends on the part of a call stack text after its first untracked frame.
pub(crate) const REASON: &str = "a Text method whose result depends on the part of a call stack text after its first untracked frame (the frames BC has and this runtime does not track)";

/// What a method does with the known prefix.
#[derive(Clone, Copy)]
pub(crate) enum Kind {
    /// `IndexOf`, `StrPos`: a position found in the prefix.
    Find,
    Contains,
    StartsWith,
    /// A mapping of every character: the same mapping of the prefix, then the mark.
    Map,
    Replace,
    /// `Substring`, `CopyStr`: 1-based start, optional length.
    Range,
    /// `DelStr`, `Remove`: 1-based start, optional length; without a length the rest goes.
    Delete,
}

/// Where the text is: the receiver of a method or the first argument of a function.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum At {
    Receiver,
    Arg0,
}

/// The known prefix of a call stack text, `None` for any other value (the caller then goes on as before).
fn prefix_of(v: Option<&Value>) -> Option<Txt> {
    let (Value::Text(t) | Value::Code(t)) = v? else {
        return None;
    };
    if !t.has_marker() || t.has_lone() {
        return None;
    }
    let s = t.as_str();
    let end = s.find(CALL_STACK_MARK)?;
    let head = &s[..end];
    // A prefix that holds another kind of marker is not known text (the head ends before the first call stack mark).
    if head.contains(crate::native::TEXT_MARKS) {
        return None;
    }
    Some(head.to_string().into())
}

fn refuse() -> NResult {
    Err(Signal::unsupported(REASON.to_string()))
}

fn with_mark(v: NResult) -> NResult {
    match v {
        Ok(Value::Text(mut t)) => {
            t.push(CALL_STACK_MARK);
            Ok(Value::Text(t))
        }
        _ => refuse(),
    }
}

fn whole(a: &[Value], i: usize) -> Option<i64> {
    crate::textfns::int_or_decimal(a, i).ok()
}

/// Runs method `f` of `kind` on the known prefix when its text is a call stack text. `None` when the text is not one, and the
/// method goes on as before.
pub(crate) fn dispatch(
    kind: Kind,
    at: At,
    f: Native,
    ctx: &mut dyn Ctx,
    recv: &Option<&mut Value>,
    args: &mut [Value],
) -> Option<NResult> {
    let (head, offset) = match at {
        At::Receiver => (prefix_of(recv.as_deref())?, 0),
        At::Arg0 => (prefix_of(args.first())?, 1),
    };
    Some(run(kind, f, ctx, &head, offset, args))
}

/// `SelectStr(n, T)`: the text is the second argument.
pub(crate) fn dispatch_select_str(f: Native, ctx: &mut dyn Ctx, args: &mut [Value]) -> Option<NResult> {
    let head = prefix_of(args.get(1))?;
    let n = whole(args, 0)?;
    // The item ends at a comma inside the prefix, so it is known.
    let commas = head.chars().filter(|c| *c == ',').count() as i64;
    if n < 1 || n > commas {
        return Some(refuse());
    }
    let mut call = [args[0].clone(), Value::Text(head)];
    Some(f(ctx, None, &mut call))
}

fn run(kind: Kind, f: Native, ctx: &mut dyn Ctx, head: &Txt, offset: usize, args: &[Value]) -> NResult {
    // `a` holds what follows the text; the text itself is replaced by the prefix.
    let rest: Vec<Value> = args[offset..].to_vec();
    let len = head.utf16_len() as i64;
    let call = |ctx: &mut dyn Ctx, rest: &[Value]| -> NResult {
        let mut v = Value::Text(head.clone());
        if offset == 0 {
            let mut a = rest.to_vec();
            f(ctx, Some(&mut v), &mut a)
        } else {
            let mut a = vec![v];
            a.extend_from_slice(rest);
            f(ctx, None, &mut a)
        }
    };
    let needle_empty = || match rest.first() {
        Some(Value::Text(t) | Value::Code(t)) => t.is_empty(),
        _ => false,
    };
    let needle = || match rest.first() {
        Some(Value::Text(t) | Value::Code(t)) if !t.has_marker() => Some(t.clone()),
        _ => None,
    };
    match kind {
        Kind::Find => match call(ctx, &rest) {
            Ok(Value::Integer(n)) if n > 0 || needle_empty() => Ok(Value::Integer(n)),
            _ => refuse(),
        },
        Kind::Contains => match call(ctx, &rest) {
            Ok(Value::Boolean(true)) => Ok(Value::Boolean(true)),
            _ => refuse(),
        },
        Kind::StartsWith => match call(ctx, &rest) {
            Ok(Value::Boolean(true)) => Ok(Value::Boolean(true)),
            // False is known when the needle and the prefix differ inside both; a needle that reaches past the prefix with the
            // prefix as its start (or an empty prefix) depends on the tail.
            Ok(Value::Boolean(false)) => match needle() {
                Some(n) if !head.is_empty() && !n.starts_with(head.as_str()) => Ok(Value::Boolean(false)),
                _ => refuse(),
            },
            _ => refuse(),
        },
        Kind::Map => {
            // A lower-case mapping of a final sigma depends on the letter after it.
            if head.ends_with('\u{3a3}') {
                return refuse();
            }
            with_mark(call(ctx, &rest))
        }
        Kind::Replace => {
            let Some(old) = needle().filter(|o| !o.is_empty()) else {
                return refuse();
            };
            // A match could start inside the prefix and continue into the tail.
            let straddles = old
                .char_indices()
                .skip(1)
                .any(|(i, _)| head.as_str().ends_with(&old[..i]));
            if straddles {
                return refuse();
            }
            with_mark(call(ctx, &rest))
        }
        Kind::Range => {
            let Some(start) = whole(&rest, 0) else {
                return refuse();
            };
            if start < 1 || start - 1 > len {
                return refuse();
            }
            match rest.get(1).map(|_| whole(&rest, 1)) {
                Some(None) => refuse(),
                Some(Some(n)) if n >= 0 && start - 1 + n <= len => call(ctx, &rest),
                // A length that reaches the tail: the text from the start of the range, and what follows is unknown.
                Some(Some(n)) if n >= 0 => with_mark(call(ctx, &rest[..1])),
                Some(Some(_)) => refuse(),
                None => with_mark(call(ctx, &rest)),
            }
        }
        Kind::Delete => {
            let Some(start) = whole(&rest, 0) else {
                return refuse();
            };
            if start < 1 || start - 1 > len {
                return refuse();
            }
            match rest.get(1).map(|_| whole(&rest, 1)) {
                Some(None) => refuse(),
                Some(Some(n)) if n >= 0 && start - 1 + n <= len => with_mark(call(ctx, &rest)),
                Some(Some(_)) => refuse(),
                // Without a count everything from the start goes, the tail with it.
                None => call(ctx, &rest),
            }
        }
    }
}
