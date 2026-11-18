//! An Option handed to a `var` parameter whose declared member list differs from the variable's (al-runner issues 7994 and
//! 8291): which members the parameter reads while the call runs, and which the caller's variable has after it. Not pinned:
//! assumption `var-option-caller-members` (corpus PR 1432, tests/al-language/types/enum-option/OptionVarMemberLists.al and
//! OptionVarMemberlessEvaluate.al). The callers the probe does not measure are refused where the call is lowered (al-sema
//! `lower/var_option_arg.rs`); a member-name read after the routine stored into such a parameter is refused by its reader
//! (`al_builtins::option_members`).

use al_builtins::assumptions::{Assumption, assume};
use al_builtins::native::Signal;
use al_builtins::option_members::{is_stored, memberless, stored};
use al_builtins::value::{RtType, Value};
use std::sync::Arc;

/// The value a `var` parameter of type `ty` takes when it is handed `v`; `None`: the usual conversion.
/// - Declared without members, it keeps the members of the value handed in: Format, StrSubstNo and Evaluate in the callee
///   read the caller's members (OptionVarMemberLists.al:241, :364, :510, :531; OptionVarMemberlessEvaluate.al:77-244). A
///   value a routine stored into its own such parameter keeps its mark (`param_store`) into the next one.
/// - A subscriber's parameter with members, handed a value with another list: refused (not measured). A procedure's or a
///   publisher's parameter with members reads its own (OptionVarMemberLists.al:398), the usual conversion.
pub(crate) fn var_binding(v: &Value, ty: &RtType, subscriber: bool) -> Option<Result<Value, Signal>> {
    let (Value::Option(_, has), RtType::Option(declared)) = (v, ty) else {
        return None;
    };
    if memberless(declared) && (!memberless(has) || is_stored(has)) {
        assume(Assumption::VarOptionCallerMembers);
        return Some(Ok(v.clone()));
    }
    (subscriber && !memberless(declared) && has != declared).then(|| {
        Err(Signal::unsupported(
            "an event subscriber's `var` Option parameter with another member list than the value the publisher hands it (not measured, assumption var-option-caller-members)",
        ))
    })
}

/// The value a `var` argument's local or global variable of type `declared` takes back after the call, which left it `v`;
/// `current` is the variable's value now. The variable has its own members again: a declared list, or, declared without
/// members, the members it holds (a memberless caller stays memberless, OptionVarMemberLists.al:432; a memberless `var`
/// parameter handed on keeps its caller's, :364). After an empty body, `Value := 1` (:224), a write to another list
/// (`Value::C` gives Third, :398), a TryFunction and an error after the write (:552, :566). A value the callee stored into
/// its memberless parameter keeps its mark in a variable declared without members. An array of Options with members gets
/// its own members back in each element a callee stored into (`element_store`; :468). `None`: `v` as it is.
pub(crate) fn written_back(v: &Value, declared: &RtType, current: impl FnOnce() -> Option<Value>) -> Option<Value> {
    let (o, has, d) = match (v, declared) {
        (Value::Option(o, has), RtType::Option(d)) => (o, has, d),
        (Value::Array(a), RtType::Array(_, el)) => {
            if let RtType::Option(d) = el.as_ref() {
                own_members_back(&mut a.borrow_mut(), d);
            }
            return None;
        }
        _ => return None,
    };
    if memberless(d) && is_stored(has) {
        return None;
    }
    let own: Arc<[String]> = if memberless(d) {
        match current()? {
            Value::Option(_, m) => m,
            _ => return None,
        }
    } else {
        d.clone()
    };
    (own != *has).then(|| {
        assume(Assumption::VarOptionCallerMembers);
        Value::Option(*o, own)
    })
}

/// The elements a callee stored into (`element_store`) of an array whose elements are declared with members `d` get `d`.
fn own_members_back(elements: &mut [Value], d: &Arc<[String]>) {
    if memberless(d) {
        return;
    }
    for e in elements {
        if let Value::Option(o, m) = e
            && is_stored(m)
        {
            *e = Value::Option(*o, d.clone());
        }
    }
}

/// The value a `var` parameter slot declared `declared` takes when the routine stores `v` into it while it holds `current`
/// (`var_param`: the slot is a `var` parameter of the running routine). Declared without members and holding the caller's
/// members (or a value stored so before), a value with another list (`Value := 1`, `Value := OtherOption`) is marked
/// (`al_builtins::option_members::stored`): the caller's read after the call is measured (OptionVarMemberLists.al:224), a
/// member-name read before that is not, and refuses. A value with the list the slot holds (Evaluate,
/// OptionVarMemberlessEvaluate.al:244) is stored as it is.
pub(crate) fn param_store(v: Value, declared: &RtType, var_param: bool, current: &Value) -> Value {
    match (&v, declared, current) {
        (Value::Option(o, m), RtType::Option(d), Value::Option(_, own))
            if var_param && memberless(d) && (!memberless(own) || is_stored(own)) && !Arc::ptr_eq(m, own) =>
        {
            assume(Assumption::VarOptionCallerMembers);
            Value::Option(*o, stored())
        }
        _ => v,
    }
}

/// The value an element of an array of memberless Options takes when `v` is stored into it while it holds `current` with
/// members (or a value stored so before), when the array is a `var` array parameter of the running routine (`var_param`):
/// the element is one of a caller's array. Evaluate's value, with the list the element holds, keeps it
/// (OptionVarMemberlessEvaluate.al:552); another value (`A[1] := 1`, OptionVarMemberLists.al:468) is marked as
/// `param_store` marks it, and the caller's array gets its members back (`written_back`). `None`: the usual conversion.
pub(crate) fn element_store(v: &Value, el: &RtType, var_param: bool, current: &Value) -> Option<Value> {
    match (v, el, current) {
        (Value::Option(o, m), RtType::Option(d), Value::Option(_, own))
            if var_param && memberless(d) && (!memberless(own) || is_stored(own)) =>
        {
            assume(Assumption::VarOptionCallerMembers);
            Some(Value::Option(
                *o,
                if Arc::ptr_eq(m, own) { own.clone() } else { stored() },
            ))
        }
        _ => None,
    }
}

impl crate::Session {
    /// What a store of `v` into an element of array `base` (elements of type `el`, now `current`) in frame `f` leaves there
    /// instead of the usual conversion: an element of a `var` array parameter of the running routine keeps its caller's
    /// members (`element_store`).
    pub(crate) fn option_element_store(
        &self,
        base: &al_ir::Place,
        v: &Value,
        el: &RtType,
        f: &crate::Frame,
        current: &Value,
    ) -> Option<Value> {
        let var_param = matches!(base, al_ir::Place::Local(s)
            if self.engine.program.objects[f.object].routines[f.routine].by_var.get(*s) == Some(&true));
        element_store(v, el, var_param, current)
    }

    /// The value a store of `v` into local `i` of the routine running in `f` leaves there: an Option goes through
    /// `param_store` (a memberless `var` Option parameter that holds its caller's members); any other value is stored as is.
    pub(crate) fn option_param_store(&self, i: usize, v: Value, f: &crate::Frame) -> Value {
        if !matches!(v, Value::Option(..)) {
            return v;
        }
        let r = &self.engine.program.objects[f.object].routines[f.routine];
        let var_param = r.by_var.get(i).copied().unwrap_or(false);
        param_store(v, &r.locals[i].ty, var_param, &f.locals[i])
    }

    /// The refusal of a call of event publisher `routine` of `object` that hands `args` (with `values`, from frame `f`) to a
    /// `var` Option parameter with members that a subscriber declares without members: the subscriber keeps the members
    /// (`var_binding`), measured for a `var` parameter that stands for a variable of the caller's `var` section
    /// (OptionVarMemberLists.al:531). Handed on from such a variable or from a `var` parameter it runs; from any other caller
    /// (a global, a record field, an array element, a parameter passed by value) the call refuses when such a subscriber exists.
    pub(crate) fn var_option_subscriber_refusal(
        &self,
        object: usize,
        routine: usize,
        args: &[al_ir::Arg],
        values: &[Value],
        f: &crate::Frame,
    ) -> Result<(), Signal> {
        let program = &self.engine.program;
        let r = &program.objects[object].routines[routine];
        let caller = &program.objects[f.object].routines[f.routine];
        let measured = |a: &al_ir::Arg| match a {
            al_ir::Arg::Ref(al_ir::Place::Local(s)) => {
                caller.by_var.get(*s) == Some(&true) || (*s >= caller.params && Some(*s) != caller.ret_local)
            }
            _ => false,
        };
        for (i, a) in args.iter().enumerate() {
            let (Some(Value::Option(_, has)), Some(slot)) = (values.get(i), r.locals.get(i)) else {
                continue;
            };
            let RtType::Option(declared) = &slot.ty else {
                continue;
            };
            let candidate = r.by_var.get(i) == Some(&true) && !memberless(declared) && !memberless(has);
            if candidate && !measured(a) && self.memberless_subscriber_param(object, routine, &slot.name) {
                return Err(Signal::unsupported(
                    "an event subscriber's `var` Option parameter declared without members, handed the publisher's Option with members from a caller the probe does not measure (assumption var-option-caller-members)",
                ));
            }
        }
        Ok(())
    }

    /// Whether a subscriber of event publisher `routine` of `object` declares its parameter `name` an Option without members.
    fn memberless_subscriber_param(&self, object: usize, routine: usize, name: &str) -> bool {
        let events = &self.engine.events;
        let Some(p) = events.publishers.get(&(object, routine)) else {
            return false;
        };
        let program = &self.engine.program;
        events
            .subscribers
            .get(&(p.object, p.event.clone()))
            .is_some_and(|subs| {
                subs.iter().any(|s| {
                    let r = &program.objects[s.object].routines[s.routine];
                    r.locals[..r.params.min(r.locals.len())].iter().any(|l| {
                        l.name.eq_ignore_ascii_case(name) && matches!(&l.ty, RtType::Option(m) if memberless(m))
                    })
                })
            })
    }
}
