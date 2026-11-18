//! An Option handed to a `var` parameter whose declared member list differs from the variable's (al-runner issues 7994 and
//! 8291): which members the parameter reads while the call runs, and which the caller's variable has after it. Not pinned:
//! assumption `var-option-caller-members` (corpus PR 1432, tests/al-language/types/enum-option/OptionVarMemberLists.al and
//! OptionVarMemberlessEvaluate.al). The callers the probe does not measure are refused where the call is lowered (al-sema
//! `lower/var_option_arg.rs`).

use al_builtins::assumptions::{Assumption, assume};
use al_builtins::native::Signal;
use al_builtins::value::{RtType, Value};
use std::sync::Arc;

/// Whether `m` is the member list of an Option declared without members: `Option` and `Option ""` both read as one nameless
/// member (al-sema `ast::TypeRef::option_members`).
pub(crate) fn memberless(m: &[String]) -> bool {
    m.len() == 1 && m[0].is_empty()
}

/// The value a `var` parameter of type `ty` takes when it is handed `v`; `None`: the usual conversion.
/// - Declared without members, it keeps the members of the value handed in: Format, StrSubstNo and Evaluate in the callee
///   read the caller's members (OptionVarMemberLists.al MemberlessVar_CalleeFormat_CallerLocal, MemberlessVar_TwoHops,
///   Event_MemberlessPublisher, Event_MembersPublisher_MemberlessSubscriber; OptionVarMemberlessEvaluate.al).
/// - A subscriber's parameter with members, handed a value with another list: refused (not measured). A procedure's or a
///   publisher's parameter with members reads its own (OtherMembersVar_CalleeFormatAndAssign), the usual conversion.
pub(crate) fn var_binding(v: &Value, ty: &RtType, subscriber: bool) -> Option<Result<Value, Signal>> {
    let (Value::Option(_, has), RtType::Option(declared)) = (v, ty) else {
        return None;
    };
    if memberless(declared) && !memberless(has) {
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
/// members, the members it holds (a memberless caller stays memberless, OptionVarMemberLists.al MemberlessCaller_MembersVar;
/// a memberless `var` parameter handed on keeps its caller's, MemberlessVar_TwoHops). After an empty body, `Value := 1`,
/// a write to another list (`Value::C` gives Third, OtherMembersVar_CalleeFormatAndAssign), a TryFunction and an error after
/// the write (MemberlessVar_TryFunctionAssign, MemberlessVar_AssignThenError). `None`: `v` as it is.
pub(crate) fn written_back(v: &Value, declared: &RtType, current: impl FnOnce() -> Option<Value>) -> Option<Value> {
    let (Value::Option(o, has), RtType::Option(d)) = (v, declared) else {
        return None;
    };
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

/// The value an element of an array of memberless Options takes when `v` is stored into it while it holds `current` with
/// members: the element is one of a caller's array handed to a `var` array parameter and keeps the caller's members
/// (OptionVarMemberLists.al MemberlessVarArray_Callee `A[1] := 1`, OptionVarMemberlessEvaluate.al MemberlessVarArray_EvaluateResult).
/// `None`: the usual conversion.
pub(crate) fn element_store(v: &Value, el: &RtType, current: &Value) -> Option<Value> {
    match (v, el, current) {
        (Value::Option(o, _), RtType::Option(d), Value::Option(_, own)) if memberless(d) && !memberless(own) => {
            assume(Assumption::VarOptionCallerMembers);
            Some(Value::Option(*o, own.clone()))
        }
        _ => None,
    }
}
