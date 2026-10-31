//! `System.Collections.Generic.Dictionary`2` (ADR-014 section 1). AL declares it without type arguments, an open generic type
//! the .NET host cannot create. Base Application's ODataUtility.GenerateFilterText creates one, fills it in GetNAVFilters
//! from Tenant Web Service Filter rows only, and walks it with `foreach` in CombineFiltersFromTables. With no filter rows
//! the dictionary stays empty, the walk finds nothing and the V3 and V4 filter texts are '' (tests/al-language-onprem-dotnet/
//! lt-qx-dotnet/QxDotNetDemoShapes.al:473, a query service; al-language-onprem/platform/PtWebServiceRows.al:326, a page
//! service; Microsoft's DemoTool, Create Web Services).
//!
//! Base Application's RequestPageParametersHelper.GetViewFromDynamicRequestPage makes one with the Int32 capacity
//! constructor, fills it with `ContainsKey`/`Add` (Integer keys, Text values) and walks it with `foreach` into a
//! `KeyValuePair`2` (ConvertFiltersToParameters). That shape is pinned by tests/al-language-onprem-dotnet/
//! lt-y17-dotnet-dictionary/Y17DotNetDictionary.al (walk order, `Count`, `Item`, `ContainsKey`, and the errors of a duplicate
//! `Add`, `Item` of a missing key and a negative capacity).
//!
//! Served: both constructors; on a dictionary made with a capacity `Add`, `ContainsKey`, `Count` and `Item` with Integer keys
//! and Text values; `foreach` in insertion order, and `Key`/`Value` of the pair it yields. Refused: entries in a dictionary
//! made without a capacity (no test fills one), keys other than Integer and values other than Text (how other boxed kinds
//! compare is not pinned), and every other member.
//!
//! State of a dictionary: `[Boolean(made with a capacity), key0, value0, key1, value1, ...]` in insertion order. State of
//! its enumerator: `[Integer(next index), pair0, pair1, ...]`. State of a pair: `[key, value]`.

use super::registry::{MemberKind, Pin, RegistryMember, RegistryType};
use crate::Session;
use al_builtins::dotnet_sigs::{GENERIC_DICTIONARY, GENERIC_DICTIONARY_ENUMERATOR, GENERIC_KEY_VALUE_PAIR};
use al_builtins::native::Signal;
use al_builtins::value::{DotNetKind, DotNetObject, Value};
use std::cell::RefCell;
use std::rc::Rc;

const PIN: &str = "tests/al-language-onprem-dotnet/lt-qx-dotnet/QxDotNetDemoShapes.al:473; \
    al-language-onprem/platform/PtWebServiceRows.al:326";

const FILL_PIN: &str = "tests/al-language-onprem-dotnet/lt-y17-dotnet-dictionary/Y17DotNetDictionary.al:56";

/// What BC names the closed type in the message of a failed member call (Y17DotNetDictionary.al:126, :147).
const CLOSED: &str = "System.Collections.Generic.Dictionary`2[System.Object,System.Object]";

fn unsupported<T>(why: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(why.into()))
}

fn new_dictionary(_: &mut Session, _: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    match args {
        [] => Ok(object(GENERIC_DICTIONARY, vec![Value::Boolean(false)])),
        // Y17DotNetDictionary.al:159 DnDict_Capacity_Negative.
        [Value::Integer(c)] if *c < 0 => al_builtins::native::raise(
            format!(
                "A call to {GENERIC_DICTIONARY} failed with this message: Specified argument was out of the range of valid values. (Parameter 'capacity')"
            ),
            "DotNetInvoke:ArgumentOutOfRange",
        ),
        // Y17DotNetDictionary.al:36 (capacity 3), :89 (capacity 0, empty).
        [Value::Integer(_)] => Ok(object(GENERIC_DICTIONARY, vec![Value::Boolean(true)])),
        _ => unsupported(format!(
            "DotNet new {GENERIC_DICTIONARY} with arguments other than an Integer capacity (not pinned)"
        )),
    }
}

fn object(type_name: &str, state: Vec<Value>) -> Value {
    Value::DotNet(Rc::new(DotNetObject::new(DotNetKind::Internal {
        type_name: type_name.into(),
        state: RefCell::new(state),
    })))
}

fn this_state<'a>(this: Option<&'a Rc<DotNetObject>>, type_name: &str) -> Result<&'a RefCell<Vec<Value>>, Signal> {
    match this.map(|o| &o.kind) {
        Some(DotNetKind::Internal { type_name: t, state }) if t == type_name => Ok(state),
        _ => unsupported(format!("DotNet {type_name} member on something the runtime did not make")),
    }
}

/// The entries of a dictionary made with a capacity; refused for one made without (no test fills one).
fn filled<'a>(this: Option<&'a Rc<DotNetObject>>, member: &str) -> Result<&'a RefCell<Vec<Value>>, Signal> {
    let state = this_state(this, GENERIC_DICTIONARY)?;
    if !matches!(state.borrow().first(), Some(Value::Boolean(true))) {
        return unsupported(format!(
            "DotNet {GENERIC_DICTIONARY}.{member} on a dictionary made without a capacity (only the capacity constructor is pinned with entries)"
        ));
    }
    Ok(state)
}

/// An Integer key: the only kind pinned (boxed Int32, Y17DotNetDictionary.al:36).
fn key_of(args: &[Value], member: &str) -> Result<i32, Signal> {
    match args.first() {
        Some(Value::Integer(k)) => Ok(*k),
        _ => unsupported(format!(
            "DotNet {GENERIC_DICTIONARY}.{member} with a key that is not an Integer (only boxed Int32 keys are pinned)"
        )),
    }
}

fn position(state: &RefCell<Vec<Value>>, key: i32) -> Option<usize> {
    state
        .borrow()
        .iter()
        .skip(1)
        .step_by(2)
        .position(|k| matches!(k, Value::Integer(i) if *i == key))
}

fn add(_: &mut Session, this: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    let state = filled(this, "Add")?;
    let key = key_of(args, "Add")?;
    let value = match args {
        [_, Value::Text(t) | Value::Code(t)] => Value::Text(t.clone()),
        _ => {
            return unsupported(format!(
                "DotNet {GENERIC_DICTIONARY}.Add with a value that is not a Text (only Text values are pinned)"
            ));
        }
    };
    if position(state, key).is_some() {
        // Y17DotNetDictionary.al:126 DnDict_DuplicateAdd: the entries stay as they were.
        return al_builtins::native::raise(
            format!(
                "A call to {CLOSED}.Add failed with this message: An item with the same key has already been added. Key: {key}"
            ),
            "DotNetInvoke:Argument",
        );
    }
    let mut s = state.borrow_mut();
    s.push(Value::Integer(key));
    s.push(value);
    Ok(Value::Null)
}

fn contains_key(_: &mut Session, this: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    let state = filled(this, "ContainsKey")?;
    if args.len() != 1 {
        return unsupported(format!("DotNet {GENERIC_DICTIONARY}.ContainsKey takes one key"));
    }
    Ok(Value::Boolean(position(state, key_of(args, "ContainsKey")?).is_some()))
}

fn count(_: &mut Session, this: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    let state = filled(this, "Count")?;
    if !args.is_empty() {
        return unsupported(format!("DotNet {GENERIC_DICTIONARY}.Count takes no arguments"));
    }
    Ok(Value::Integer((state.borrow().len() / 2) as i32))
}

fn item(_: &mut Session, this: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    let state = filled(this, "Item")?;
    if args.len() != 1 {
        return unsupported(format!("DotNet {GENERIC_DICTIONARY}.Item: only the getter with one key is pinned"));
    }
    let key = key_of(args, "Item")?;
    match position(state, key) {
        Some(i) => Ok(state.borrow()[2 + 2 * i].clone()),
        // Y17DotNetDictionary.al:147 DnDict_Item_Missing.
        None => al_builtins::native::raise(
            format!("A call to {CLOSED}.Item failed with this message: The given key '{key}' was not present in the dictionary."),
            "DotNetInvoke:KeyNotFound",
        ),
    }
}

fn pair_part(this: Option<&Rc<DotNetObject>>, args: &[Value], at: usize, member: &str) -> Result<Value, Signal> {
    if !args.is_empty() {
        return unsupported(format!("DotNet {GENERIC_KEY_VALUE_PAIR}.{member} takes no arguments"));
    }
    Ok(this_state(this, GENERIC_KEY_VALUE_PAIR)?.borrow()[at].clone())
}

/// `Kvp.Key`: the Integer key, which StrSubstNo and Format show as a number (Y17DotNetDictionary.al:66-67).
fn pair_key(_: &mut Session, this: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    pair_part(this, args, 0, "Key")
}

fn pair_value(_: &mut Session, this: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    pair_part(this, args, 1, "Value")
}

/// The enumerator `foreach` steps over a dictionary the runtime made, if `o` is one: a snapshot of its pairs in insertion
/// order (Y17DotNetDictionary.al:64, `Table18=view18;Table36=view36;Table5=view5;`; the dictionary is not changed inside
/// the walk there).
pub(crate) fn foreach_start(o: &DotNetObject) -> Option<Value> {
    let DotNetKind::Internal { type_name, state } = &o.kind else {
        return None;
    };
    if type_name != GENERIC_DICTIONARY {
        return None;
    }
    let mut en = vec![Value::Integer(0)];
    for kv in state.borrow()[1..].chunks(2) {
        en.push(object(GENERIC_KEY_VALUE_PAIR, kv.to_vec()));
    }
    Some(object(GENERIC_DICTIONARY_ENUMERATOR, en))
}

/// The next pair of an enumerator of [`foreach_start`]: `None` if `o` is no such enumerator, `Some(None)` when the walk ends.
pub(crate) fn foreach_next(o: &DotNetObject) -> Option<Option<Value>> {
    let DotNetKind::Internal { type_name, state } = &o.kind else {
        return None;
    };
    if type_name != GENERIC_DICTIONARY_ENUMERATOR {
        return None;
    }
    let mut s = state.borrow_mut();
    let Value::Integer(at) = s[0] else { return Some(None) };
    let next = s.get(at as usize + 1).cloned();
    if next.is_some() {
        s[0] = Value::Integer(at + 1);
    }
    Some(next)
}

fn member(name: &str, kind: MemberKind, call: super::registry::MemberFn) -> RegistryMember {
    RegistryMember {
        name: name.into(),
        kind,
        pin: Pin::Corpus(FILL_PIN.into()),
        call,
        call_ref: None,
    }
}

pub fn types() -> Vec<RegistryType> {
    vec![RegistryType {
        clr_name: GENERIC_DICTIONARY.into(),
        members: vec![
            RegistryMember {
                name: "Dictionary".into(),
                kind: MemberKind::Construct,
                pin: Pin::Corpus(format!("{PIN}; {FILL_PIN}")),
                call: new_dictionary,
                call_ref: None,
            },
            member("Add", MemberKind::Call, add),
            member("ContainsKey", MemberKind::Call, contains_key),
            member("Count", MemberKind::Get, count),
            member("Item", MemberKind::Call, item),
        ],
    },
    RegistryType {
        clr_name: GENERIC_KEY_VALUE_PAIR.into(),
        members: vec![
            member("Key", MemberKind::Get, pair_key),
            member("Value", MemberKind::Get, pair_value),
        ],
    }]
}
