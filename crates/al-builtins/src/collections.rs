//! List and Dictionary methods. The corpus pins the error of `List.Get` outside the list and of
//! `Dictionary.Get` of a missing key; the other out-of-range indexes and `Dictionary.Add` of an
//! existing key raise Unsupported.

use crate::describe::Describe;
use crate::native::{Ctx, NativeFn, Signal};
use crate::ops::compare;
use crate::value::Value;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::rc::Rc;

pub const REGISTRY: &[(&str, NativeFn)] = &[
    ("List.Add#1", |_, r, a| {
        with_list(r, |l| {
            l.push(a[0].clone());
            Ok(Value::Null)
        })
    }),
    ("List.AddRange#1", |_, r, a| {
        refuse_read_only(&r, "List.AddRange")?;
        with_list(r, |l| {
            l.extend(a.iter().cloned());
            Ok(Value::Null)
        })
    }),
    ("List.AddRange#2", |_, r, a| {
        refuse_read_only(&r, "List.AddRange")?;
        let items = list_items(&a[0])?;
        with_list(r, |l| {
            l.extend(items);
            Ok(Value::Null)
        })
    }),
    ("List.Contains#1", |_, r, a| {
        read_list(r, |l| Ok(Value::Boolean(position_checked(l, &a[0])?.is_some())))
    }),
    ("List.Count#1", |_, r, _| {
        read_list(r, |l| Ok(Value::Integer(l.len() as i32)))
    }),
    ("List.Get#1", |_, r, a| {
        let i = index(&a[0])?;
        read_list(r, |l| l.get(i).cloned().map_or_else(list_get_error, Ok))
    }),
    ("List.Get#2", |c, r, a| {
        // An index below 1 answers false like one past the end (collections/ListInt.al:Int_AddCountGet `get 0 ret`).
        let got = match &a[0] {
            Value::Integer(n) if *n < 1 && c.ret_used() => None,
            other => {
                let i = index(other)?;
                read_list(r, |l| Ok(l.get(i).cloned()))?
            }
        };
        match got {
            Some(v) => {
                a[1] = v;
                Ok(Value::Boolean(true))
            }
            None if c.ret_used() => Ok(Value::Boolean(false)),
            None => list_get_error(),
        }
    }),
    ("List.IndexOf#1", |_, r, a| {
        read_list(r, |l| {
            Ok(Value::Integer(position_checked(l, &a[0])?.map_or(0, |p| p as i32 + 1)))
        })
    }),
    ("List.LastIndexOf#1", |_, r, a| {
        if matches!(a[0], Value::Dictionary(_)) {
            return Err(Signal::Unsupported(
                "List.LastIndexOf of a Dictionary (not pinned)".into(),
            ));
        }
        read_list(r, |l| {
            Ok(Value::Integer(
                l.iter().rposition(|x| eq(x, &a[0])).map_or(0, |p| p as i32 + 1),
            ))
        })
    }),
    ("List.Insert#1", |_, r, a| {
        let i = index(&a[0])?;
        with_list(r, |l| {
            if i > l.len() {
                return out_of_range();
            }
            l.insert(i, a[1].clone());
            Ok(Value::Boolean(true))
        })
    }),
    ("List.Remove#1", |_, r, a| {
        let dictionary = matches!(a[0], Value::Dictionary(_));
        with_list(r, |l| {
            // A Dictionary is found by identity, any other one cannot be compared (CsCollections.al:DictionaryList_ContainsAndIndexOf).
            let at = if dictionary {
                position_checked(l, &a[0])?
            } else {
                position(l, &a[0])
            };
            Ok(Value::Boolean(match at {
                Some(p) => {
                    l.remove(p);
                    true
                }
                None => false,
            }))
        })
    }),
    // With the result used a bad index or range answers false and removes nothing (collections/ListInt.al:Int_Remove
    // `removeat 10 ret`, Int_RemoveRange `removerange 4,3`); the statement form raises.
    ("List.RemoveAt#1", |c, r, a| {
        if c.ret_used() {
            refuse_read_only(&r, "the result of a change")?;
        }
        let done = (|| {
            let i = index(&a[0])?;
            with_list(r, |l| {
                if i >= l.len() {
                    return out_of_range();
                }
                l.remove(i);
                Ok(Value::Boolean(true))
            })
        })();
        false_when_used(c.ret_used(), done)
    }),
    ("List.RemoveRange#1", |c, r, a| {
        if c.ret_used() {
            refuse_read_only(&r, "the result of a change")?;
        }
        let done = (|| {
            let i = index(&a[0])?;
            let n = count(&a[1])?;
            with_list(r, |l| {
                if i + n > l.len() {
                    return range_past_end();
                }
                l.drain(i..i + n);
                Ok(Value::Boolean(true))
            })
        })();
        false_when_used(c.ret_used(), done)
    }),
    ("List.Reverse#1", |_, r, _| {
        with_list(r, |l| {
            l.reverse();
            Ok(Value::Null)
        })
    }),
    ("List.Set#1", |_, r, a| {
        let i = index(&a[0])?;
        with_list(r, |l| match l.get_mut(i) {
            Some(slot) => {
                *slot = a[1].clone();
                Ok(Value::Boolean(true))
            }
            None => out_of_range(),
        })
    }),
    ("List.GetRange#1", |_, r, a| {
        let i = index(&a[0])?;
        let n = count(&a[1])?;
        read_list(r, |l| {
            if i + n > l.len() {
                return range_past_end();
            }
            Ok(Value::List(Rc::new(RefCell::new(l[i..i + n].to_vec().into()))))
        })
    }),
    ("Dictionary.Add#1", |c, r, a| {
        let ret_used = c.ret_used();
        with_dict(r, |d| {
            if live(d).any(|(_, (k, _))| eq(k, &a[0])) {
                return if ret_used {
                    Ok(Value::Boolean(false))
                } else {
                    error_code("An item with the same key has already been added.", "Argument")
                };
            }
            dict_insert(d, a[0].clone(), a[1].clone());
            Ok(Value::Boolean(true))
        })
    }),
    ("Dictionary.ContainsKey#1", |_, r, a| {
        with_dict(r, |d| Ok(Value::Boolean(live(d).any(|(_, (k, _))| eq(k, &a[0])))))
    }),
    ("Dictionary.Count#1", |_, r, _| {
        with_dict(r, |d| Ok(Value::Integer(live(d).count() as i32)))
    }),
    ("Dictionary.Get#1", |_, r, a| {
        with_dict(r, |d| {
            live(d)
                .find(|(_, (k, _))| eq(k, &a[0]))
                .map(|(_, (_, v))| v.clone())
                .map_or_else(key_not_found, Ok)
        })
    }),
    ("Dictionary.Get#2", |c, r, a| {
        let got = with_dict(r, |d| {
            Ok(live(d).find(|(_, (k, _))| eq(k, &a[0])).map(|(_, (_, v))| v.clone()))
        })?;
        match got {
            Some(v) => {
                a[1] = v;
                Ok(Value::Boolean(true))
            }
            None if c.ret_used() => Ok(Value::Boolean(false)),
            None => key_not_found(),
        }
    }),
    ("Dictionary.Keys#1", |_, r, _| {
        with_dict(r, |d| {
            Ok(Value::List(Rc::new(RefCell::new(
                live(d).map(|(_, (k, _))| k.clone()).collect(),
            ))))
        })
    }),
    ("Dictionary.Values#1", |_, r, _| {
        with_dict(r, |d| {
            Ok(Value::List(Rc::new(RefCell::new(
                live(d).map(|(_, (_, v))| v.clone()).collect(),
            ))))
        })
    }),
    ("Dictionary.Remove#1", |_, r, a| {
        with_dict(r, |d| {
            let found = live(d).find(|(_, (k, _))| eq(k, &a[0])).map(|(i, _)| i);
            Ok(Value::Boolean(match found {
                Some(p) => {
                    // The slot is freed, not closed up: a key added next takes the slot freed last (.NET Dictionary; collections/
                    // DictBasic.al:Dict_SetRemove, `keys after re-add` [a,b,c] after removing b and adding b again).
                    let seq = d.iter().filter_map(tombstone).max().unwrap_or(0) + 1;
                    d[p] = (Value::Null, Value::Integer(seq));
                    true
                }
                None => false,
            }))
        })
    }),
    ("Dictionary.Set#1", |_, r, a| {
        with_dict(r, |d| {
            match d.iter_mut().find(|(k, _)| !matches!(k, Value::Null) && eq(k, &a[0])) {
                Some((_, v)) => *v = a[1].clone(),
                None => dict_insert(d, a[0].clone(), a[1].clone()),
            }
            Ok(Value::Null)
        })
    }),
];

/// A freed slot of a dictionary: a null key and the sequence number of the removal (the free list of a .NET Dictionary is last in,
/// first out).
fn tombstone(e: &(Value, Value)) -> Option<i32> {
    match e {
        (Value::Null, Value::Integer(n)) => Some(*n),
        _ => None,
    }
}

/// The entries that are in the dictionary, with their slots.
fn live(d: &[(Value, Value)]) -> impl Iterator<Item = (usize, &(Value, Value))> {
    d.iter().enumerate().filter(|(_, e)| tombstone(e).is_none())
}

/// A new entry goes into the slot freed last, else after the last one.
fn dict_insert(d: &mut Vec<(Value, Value)>, k: Value, v: Value) {
    let free = d
        .iter()
        .enumerate()
        .filter_map(|(i, e)| tombstone(e).map(|n| (n, i)))
        .max()
        .map(|(_, i)| i);
    match free {
        Some(i) => d[i] = (k, v),
        None => d.push((k, v)),
    }
}

fn eq(a: &Value, b: &Value) -> bool {
    compare(a, b).ok() == Some(Ordering::Equal)
}

/// `Contains` and `IndexOf`: a list of interfaces does not support them, and two lists are the same list or cannot be compared
/// (collections/ListComplex.al:Interface_List `af33`, `af35`, Nested_ListOfList, Nested_ListOfListEquality).
fn position_checked(l: &[Value], v: &Value) -> Result<Option<usize>, Signal> {
    if matches!(v, Value::Codeunit(_)) {
        return error_code(
            "The function is not supported on lists of type 'Microsoft.Dynamics.Nav.Runtime.NavInterfaceHandle'.",
            "NotSupportedOperation",
        );
    }
    if let Value::List(want) = v {
        for (i, x) in l.iter().enumerate() {
            match x {
                Value::List(have) if Rc::ptr_eq(have, want) => return Ok(Some(i)),
                Value::List(_) => {
                    return error_code(
                        "Unable to compare operands of type NavList`1 with NavList`1.",
                        "InvalidComparison",
                    );
                }
                _ => {}
            }
        }
        return Ok(None);
    }
    // The same for a Dictionary: the same one is found, any other raises (collections/lt-cs-odd-leads/CsCollections.al:
    // DictionaryList_ContainsAndIndexOf, `contains second`, `contains equal contents`).
    if let Value::Dictionary(want) = v {
        for (i, x) in l.iter().enumerate() {
            match x {
                Value::Dictionary(have) if Rc::ptr_eq(have, want) => return Ok(Some(i)),
                Value::Dictionary(_) => {
                    return error_code(
                        "Unable to compare operands of type NavDictionary`2 with NavDictionary`2.",
                        "InvalidComparison",
                    );
                }
                _ => {}
            }
        }
        return Ok(None);
    }
    Ok(position(l, v))
}

fn position(l: &[Value], v: &Value) -> Option<usize> {
    l.iter().position(|x| eq(x, v))
}

fn error_code<T>(message: &str, code: &str) -> Result<T, Signal> {
    Err(Signal::Error(Box::new(crate::native::AlError {
        message: message.into(),
        code: code.into(),
        call_stack: String::new(),
        ..Default::default()
    })))
}

const LIST_INVALID: &str = "An invalid argument was passed to a 'List' data type method.";

/// `List.Get` with an index past the end, in both forms (collections/TestListTrappableError.al:21-45).
fn list_get_error<T>() -> Result<T, Signal> {
    error_code(LIST_INVALID, "ArgumentOutOfRange")
}

/// `Dictionary.Get` of a missing key: pinned for the `Get(Key, var Value)` form
/// (collections/TestCollectionsDictionary.al:42-55); the value form is only pinned to raise
/// (collections/TestCollectionContracts.al:188), and is assumed to say the same (probes-needed).
fn key_not_found<T>() -> Result<T, Signal> {
    error_code("The given key was not present in the dictionary.", "KeyNotFound")
}

/// An index outside the list (session/TestProbeSessionMiscContracts.al:Dictionary_List_ErrorTexts: Get, RemoveAt, Set,
/// Insert and a range starting at 0 or past the count; code ArgumentOutOfRange).
fn out_of_range<T>() -> Result<T, Signal> {
    error_code(LIST_INVALID, "ArgumentOutOfRange")
}

/// A range that reaches past the end of the list (code Argument).
fn range_past_end<T>() -> Result<T, Signal> {
    error_code(LIST_INVALID, "Argument")
}

/// `done`, or false for the list errors when the result is used.
fn false_when_used(ret_used: bool, done: Result<Value, Signal>) -> Result<Value, Signal> {
    match done {
        Err(Signal::Error(e)) if ret_used && e.message == LIST_INVALID => Ok(Value::Boolean(false)),
        other => other,
    }
}

/// AL list indexes are 1-based.
fn index(v: &Value) -> Result<usize, Signal> {
    match v {
        Value::Integer(i) if *i >= 1 => Ok(*i as usize - 1),
        _ => out_of_range(),
    }
}

fn count(v: &Value) -> Result<usize, Signal> {
    match v {
        Value::Integer(i) if *i >= 0 => Ok(*i as usize),
        _ => out_of_range(),
    }
}

fn list_items(v: &Value) -> Result<Vec<Value>, Signal> {
    match v {
        Value::List(l) => Ok(l.borrow().to_vec()),
        other => Err(Signal::Unsupported(format!("{} is not a list", other.describe()))),
    }
}

/// A read does not count as a change, so it may run inside a `foreach` over the same list: IndexOf (misc/lt-u-obsolete-pending/
/// UObsoletePending.al:35-36), Count, Get, Get(i, var), Contains, LastIndexOf and GetRange (collections/RefSemantics.al:566
/// PW_Foreach_ListReadsWhileIterating, :594 RW_Foreach_ReadAndAddThroughByValueParam). The changing methods raise "Collection
/// was modified" (collections/RefSemantics.al:343-408).
fn read_list<T>(r: Option<&mut Value>, f: impl FnOnce(&[Value]) -> Result<T, Signal>) -> Result<T, Signal> {
    match r {
        Some(Value::List(l)) => f(&l.borrow()),
        other => Err(Signal::Unsupported(format!("list method on {}", other.describe()))),
    }
}

fn with_list<T>(r: Option<&mut Value>, f: impl FnOnce(&mut Vec<Value>) -> Result<T, Signal>) -> Result<T, Signal> {
    match r {
        // CsCollections.al:EnumOrdinals_Mutations (Reverse, Add, RemoveAt, Insert, Set, Remove, RemoveRange), EnumNames_Mutations.
        Some(Value::List(l)) if l.borrow().read_only => {
            error_code("The requested operation is not supported.", "NotSupportedOperation")
        }
        Some(Value::List(l)) => f(&mut l.borrow_mut()),
        other => Err(Signal::Unsupported(format!("list method on {}", other.describe()))),
    }
}

/// A change to a read-only list that is not pinned is refused (`ListVec::read_only`).
fn refuse_read_only(r: &Option<&mut Value>, what: &str) -> Result<(), Signal> {
    match r {
        Some(Value::List(l)) if l.borrow().read_only => Err(Signal::Unsupported(format!(
            "{what} on the list of Enum.Names or Enum.Ordinals (not pinned)"
        ))),
        _ => Ok(()),
    }
}

fn with_dict<T>(
    r: Option<&mut Value>,
    f: impl FnOnce(&mut Vec<(Value, Value)>) -> Result<T, Signal>,
) -> Result<T, Signal> {
    match r {
        Some(Value::Dictionary(d)) => f(&mut d.borrow_mut()),
        other => Err(Signal::Unsupported(format!(
            "dictionary method on {}",
            other.describe()
        ))),
    }
}

#[allow(dead_code)]
fn _ctx(_: &dyn Ctx) {}
